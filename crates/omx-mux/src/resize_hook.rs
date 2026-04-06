use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeEvent {
    pub pane_id: String,
    pub width: u16,
    pub height: u16,
    pub timestamp: String,
}

impl ResizeEvent {
    pub fn new(pane_id: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            pane_id: pane_id.into(),
            width,
            height,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub fn build_resize_monitor_args<'a>(session: &'a str, command: &'a str) -> Vec<&'a str> {
    vec![
        "set-hook",
        "-t",
        session,
        "after-resize-pane",
        "run-shell",
        command,
    ]
}

pub fn build_remove_resize_hook_args(session: &str) -> Vec<&str> {
    vec!["set-hook", "-u", "-t", session, "after-resize-pane"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_event_captures_dimensions() {
        let event = ResizeEvent::new("%1", 120, 40);
        assert_eq!(event.pane_id, "%1");
        assert_eq!(event.width, 120);
        assert_eq!(event.height, 40);
        assert!(!event.timestamp.is_empty());
    }

    #[test]
    fn build_resize_monitor_args_correct() {
        let args = build_resize_monitor_args("my-session", "echo resized");
        assert_eq!(
            args,
            vec![
                "set-hook",
                "-t",
                "my-session",
                "after-resize-pane",
                "run-shell",
                "echo resized",
            ]
        );
    }

    #[test]
    fn build_remove_resize_hook_args_correct() {
        let args = build_remove_resize_hook_args("my-session");
        assert_eq!(
            args,
            vec!["set-hook", "-u", "-t", "my-session", "after-resize-pane"]
        );
    }
}
