use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// CLI provider abstraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CliProvider {
    Codex,
    Claude,
}

impl CliProvider {
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_lowercase().as_str() {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }
}

impl std::fmt::Display for CliProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codex => write!(f, "codex"),
            Self::Claude => write!(f, "claude"),
        }
    }
}

// ---------------------------------------------------------------------------
// Team primitives
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseToken(pub String);

// ---------------------------------------------------------------------------
// Task lifecycle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Blocked,
    InProgress,
    Completed,
    Failed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Blocked => write!(f, "blocked"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Dispatch lifecycle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchStatus {
    Pending,
    Notified,
    Delivered,
    Failed,
}

impl std::fmt::Display for DispatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Notified => write!(f, "notified"),
            Self::Delivered => write!(f, "delivered"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Team phases
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeamPhase {
    Plan,
    Prd,
    Exec,
    Verify,
    Fix,
}

impl std::fmt::Display for TeamPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan => write!(f, "plan"),
            Self::Prd => write!(f, "prd"),
            Self::Exec => write!(f, "exec"),
            Self::Verify => write!(f, "verify"),
            Self::Fix => write!(f, "fix"),
        }
    }
}

impl TeamPhase {
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Plan => 0,
            Self::Prd => 1,
            Self::Exec => 2,
            Self::Verify => 3,
            Self::Fix => 4,
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Plan => Some(Self::Prd),
            Self::Prd => Some(Self::Exec),
            Self::Exec => Some(Self::Verify),
            Self::Verify => Some(Self::Fix),
            Self::Fix => None,
        }
    }

    pub const ALL: &'static [TeamPhase] = &[
        Self::Plan, Self::Prd, Self::Exec, Self::Verify, Self::Fix,
    ];
}

impl PartialOrd for TeamPhase {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TeamPhase {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ordinal().cmp(&other.ordinal())
    }
}

// ---------------------------------------------------------------------------
// Hook event contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub schema_version: String,
    pub event: HookEventName,
    pub timestamp: String,
    pub source: HookSource,
    pub context: serde_json::Value,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HookEventName {
    SessionStart,
    SessionEnd,
    SessionIdle,
    TurnComplete,
    Blocked,
    Finished,
    Failed,
    PreToolUse,
    PostToolUse,
    PrCreated,
    TestStarted,
    TestFinished,
    TestFailed,
}

impl std::fmt::Display for HookEventName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::SessionIdle => "session_idle",
            Self::TurnComplete => "turn_complete",
            Self::Blocked => "blocked",
            Self::Finished => "finished",
            Self::Failed => "failed",
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::PrCreated => "pr_created",
            Self::TestStarted => "test_started",
            Self::TestFinished => "test_finished",
            Self::TestFailed => "test_failed",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookSource {
    pub component: String,
    pub worker_id: Option<String>,
}

/// Format a HookEvent into a human-readable notification message.
/// Used by all notification hooks (Discord, Slack, Telegram).
pub fn format_hook_message(event: &HookEvent) -> String {
    let mut parts = vec![
        format!("**[OMX]** `{}`", event.event),
        format!("Source: `{}`", event.source.component),
    ];

    if let Some(ref wid) = event.source.worker_id {
        parts.push(format!("Worker: `{wid}`"));
    }

    if let Some(ref sid) = event.session_id {
        parts.push(format!("Session: `{sid}`"));
    }

    if event.context != serde_json::json!({}) {
        if let Ok(ctx) = serde_json::to_string_pretty(&event.context) {
            parts.push(format!("Context:\n```json\n{ctx}\n```"));
        }
    }

    parts.push(format!("Time: {}", event.timestamp));

    parts.join("\n")
}

// ---------------------------------------------------------------------------
// Unified error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum OmxError {
    #[error("config error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tmux error: {0}")]
    Tmux(String),

    #[error("state error: {0}")]
    State(String),

    #[error("team error: {0}")]
    Team(String),

    #[error("hook error: {0}")]
    Hook(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("mode error: {0}")]
    Mode(String),

    #[error("session error: {0}")]
    Session(String),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("catalog error: {0}")]
    Catalog(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_provider_from_label_parses_known_providers() {
        assert_eq!(CliProvider::from_label("codex"), Some(CliProvider::Codex));
        assert_eq!(CliProvider::from_label("Claude"), Some(CliProvider::Claude));
        assert_eq!(CliProvider::from_label("gemini"), None);
    }

    #[test]
    fn task_status_serde_roundtrip() {
        let status = TaskStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn hook_event_serde_roundtrip() {
        let event = HookEvent {
            schema_version: "1".into(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-04T00:00:00Z".into(),
            source: HookSource {
                component: "omx-cli".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: Some("sess-1".into()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event, event.event);
    }

    #[test]
    fn omx_error_display() {
        let err = OmxError::Config("missing field".into());
        assert_eq!(err.to_string(), "config error: missing field");
    }

    #[test]
    fn team_phase_ordering() {
        assert!(TeamPhase::Plan < TeamPhase::Exec);
        assert!(TeamPhase::Exec < TeamPhase::Verify);
        assert_eq!(TeamPhase::Plan.ordinal(), 0);
        assert_eq!(TeamPhase::Fix.ordinal(), 4);
        assert_eq!(TeamPhase::Exec.next(), Some(TeamPhase::Verify));
        assert_eq!(TeamPhase::Fix.next(), None);
    }

    #[test]
    fn team_phase_all_is_sorted_by_ordinal() {
        for window in TeamPhase::ALL.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn display_impls_produce_lowercase_labels() {
        assert_eq!(CliProvider::Codex.to_string(), "codex");
        assert_eq!(TaskStatus::InProgress.to_string(), "in_progress");
        assert_eq!(DispatchStatus::Delivered.to_string(), "delivered");
        assert_eq!(TeamPhase::Verify.to_string(), "verify");
        assert_eq!(HookEventName::SessionStart.to_string(), "session_start");
    }

    #[test]
    fn format_hook_message_includes_event_and_source() {
        let event = HookEvent {
            schema_version: "1".into(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "omx-cli".into(),
                worker_id: Some("w-001".into()),
            },
            context: serde_json::json!({"task": "build frontend"}),
            session_id: Some("sess-abc".into()),
        };

        let msg = format_hook_message(&event);
        assert!(msg.contains("session_start"), "should contain event name");
        assert!(msg.contains("omx-cli"), "should contain source component");
        assert!(msg.contains("w-001"), "should contain worker id");
        assert!(msg.contains("sess-abc"), "should contain session id");
    }

    #[test]
    fn format_hook_message_handles_missing_optional_fields() {
        let event = HookEvent {
            schema_version: "1".into(),
            event: HookEventName::Failed,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "omx-team".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: None,
        };

        let msg = format_hook_message(&event);
        assert!(msg.contains("failed"), "should contain event name");
        assert!(msg.contains("omx-team"), "should contain source component");
        assert!(!msg.contains("worker:"), "should not contain worker label when None");
    }

    #[test]
    fn mode_error_display() {
        let err = OmxError::Mode("conflict".into());
        assert_eq!(err.to_string(), "mode error: conflict");
    }

    #[test]
    fn session_error_display() {
        let err = OmxError::Session("not found".into());
        assert_eq!(err.to_string(), "session error: not found");
    }

    #[test]
    fn agent_error_display() {
        let err = OmxError::Agent("duplicate name".into());
        assert_eq!(err.to_string(), "agent error: duplicate name");
    }

    #[test]
    fn catalog_error_display() {
        let err = OmxError::Catalog("missing field".into());
        assert_eq!(err.to_string(), "catalog error: missing field");
    }
}
