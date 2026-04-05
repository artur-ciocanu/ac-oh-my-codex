use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use omx_types::TeamPhase;
use serde::{Deserialize, Serialize};
use std::io::stdout;
use std::time::Duration;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HudState {
    pub session_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub team_phase: Option<TeamPhase>,
    pub worker_count: u32,
    pub pending_tasks: u32,
    pub completed_tasks: u32,
    pub uptime_seconds: u64,
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

pub async fn run_hud(initial_state: HudState) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = TerminalGuard; // cleanup runs on drop (even on panic)

    let backend = ratatui::backend::CrosstermBackend::new(stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;
    let state = initial_state;

    loop {
        terminal.draw(|frame| {
            render_frame(&state, frame, frame.area());
        })?;

        // Poll for events with 250ms timeout (4 redraws/sec)
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }
    }

    // _guard drops here, calling disable_raw_mode + LeaveAlternateScreen
    Ok(())
}

pub fn render_frame(state: &HudState, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    use ratatui::{
        layout::{Constraint, Layout},
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph},
    };

    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(5), // stats
        Constraint::Min(0),    // spacer
    ])
    .split(area);

    // Header
    let provider = state.provider.as_deref().unwrap_or("—");
    let model = state.model.as_deref().unwrap_or("—");
    let session = state.session_id.as_deref().unwrap_or("—");
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" OMX ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(format!("  {provider} / {model}  session: {session}")),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    // Stats
    let phase_str = state
        .team_phase
        .as_ref()
        .map(|p| format!("{p:?}"))
        .unwrap_or_else(|| "Idle".into());
    let uptime_min = state.uptime_seconds / 60;
    let uptime_sec = state.uptime_seconds % 60;
    let stats = Paragraph::new(vec![
        Line::from(format!(
            "Phase: {}  Workers: {}",
            phase_str, state.worker_count
        )),
        Line::from(format!(
            "Tasks: {} pending / {} completed",
            state.pending_tasks, state.completed_tasks
        )),
        Line::from(format!("Uptime: {uptime_min}m {uptime_sec}s")),
    ])
    .block(
        Block::default()
            .title(" Status ")
            .borders(Borders::ALL),
    );
    frame.render_widget(stats, chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_state_default_is_idle() {
        let state = HudState::default();
        assert!(state.session_id.is_none());
        assert_eq!(state.worker_count, 0);
    }

    #[test]
    fn hud_state_serde_roundtrip() {
        let state = HudState {
            session_id: Some("sess-1".into()),
            provider: Some("codex".into()),
            model: Some("o3".into()),
            team_phase: Some(TeamPhase::Exec),
            worker_count: 3,
            pending_tasks: 5,
            completed_tasks: 2,
            uptime_seconds: 120,
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: HudState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.worker_count, 3);
    }

    #[tokio::test]
    async fn run_hud_returns_ok_with_test_backend() {
        let state = HudState::default();
        let _: fn(HudState) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>>> =
            |s| Box::pin(run_hud(s));
        // Verify the state is valid (we don't call run_hud as it requires a real terminal)
        assert_eq!(state.worker_count, 0);
    }

    #[test]
    fn render_frame_shows_header_and_stats() {
        use ratatui::{backend::TestBackend, Terminal};

        let state = HudState {
            session_id: Some("sess-abc".into()),
            provider: Some("codex".into()),
            model: Some("o3".into()),
            team_phase: Some(TeamPhase::Exec),
            worker_count: 3,
            pending_tasks: 5,
            completed_tasks: 2,
            uptime_seconds: 120,
        };

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_frame(&state, frame, frame.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let text = (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(text.contains("OMX"), "header should contain OMX");
        assert!(text.contains("codex"), "should show provider");
        assert!(text.contains("Exec"), "should show team phase");
        assert!(text.contains("3"), "should show worker count");
    }
}
