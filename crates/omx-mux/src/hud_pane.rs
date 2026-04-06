/// Tracks the state of a HUD (heads-up display) pane within a tmux session.
#[derive(Debug, Clone, Default)]
pub struct HudPaneState {
    pub pane_id: Option<String>,
    pub width: u16,
    pub height: u16,
}

impl HudPaneState {
    /// Create a new inactive HUD pane state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when the HUD pane is active (has a pane id).
    pub fn is_active(&self) -> bool {
        self.pane_id.is_some()
    }

    /// Mark the pane as active with the given pane identifier.
    pub fn activate(&mut self, pane_id: &str) {
        self.pane_id = Some(pane_id.to_owned());
    }

    /// Deactivate the pane: clear the identifier and reset dimensions.
    pub fn deactivate(&mut self) {
        self.pane_id = None;
        self.width = 0;
        self.height = 0;
    }

    /// Update the stored dimensions of the HUD pane.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }
}

/// Build the tmux argument list for creating a new HUD pane via `split-window`.
pub fn build_create_hud_args(session: &str, height_lines: u16) -> Vec<String> {
    vec![
        "split-window".into(),
        "-t".into(),
        session.into(),
        "-l".into(),
        height_lines.to_string(),
        "-d".into(),
        "-P".into(),
        "-F".into(),
        "#{session_name}:#{window_index}.#{pane_index}".into(),
    ]
}

/// Build the tmux argument list for resizing an existing HUD pane.
pub fn build_resize_hud_args(pane_id: &str, height_lines: u16) -> Vec<String> {
    vec![
        "resize-pane".into(),
        "-t".into(),
        pane_id.into(),
        "-y".into(),
        height_lines.to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_inactive() {
        let state = HudPaneState::new();
        assert!(!state.is_active());
        assert!(state.pane_id.is_none());
        assert_eq!(state.width, 0);
        assert_eq!(state.height, 0);
    }

    #[test]
    fn activate_and_deactivate() {
        let mut state = HudPaneState::new();

        state.activate("omx-dev:0.1");
        assert!(state.is_active());
        assert_eq!(state.pane_id.as_deref(), Some("omx-dev:0.1"));

        state.deactivate();
        assert!(!state.is_active());
        assert_eq!(state.width, 0);
        assert_eq!(state.height, 0);
    }

    #[test]
    fn resize_updates_dimensions() {
        let mut state = HudPaneState::new();
        state.resize(120, 24);
        assert_eq!(state.width, 120);
        assert_eq!(state.height, 24);
    }

    #[test]
    fn create_hud_args_correct() {
        let args = build_create_hud_args("omx-session", 10);
        assert_eq!(
            args,
            vec![
                "split-window",
                "-t",
                "omx-session",
                "-l",
                "10",
                "-d",
                "-P",
                "-F",
                "#{session_name}:#{window_index}.#{pane_index}",
            ]
        );
    }

    #[test]
    fn resize_hud_args_correct() {
        let args = build_resize_hud_args("omx-dev:0.1", 15);
        assert_eq!(args, vec!["resize-pane", "-t", "omx-dev:0.1", "-y", "15"]);
    }
}
