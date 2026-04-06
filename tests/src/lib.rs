pub mod cli;
pub mod hooks;
pub mod mcp;
pub mod modes;
pub mod notifications;

use omx_types::{HookEvent, HookEventName, HookSource};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// TestConfig — isolated temp directory with valid OMX config
// ---------------------------------------------------------------------------

pub struct TestConfig {
    pub dir: tempfile::TempDir,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TestConfig {
    /// Create a minimal valid OMX config directory.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let config_toml = r#"
[models]
frontier = "test-model"
"#;
        std::fs::write(dir.path().join("config.toml"), config_toml).unwrap();
        // Create .omx subdirectory for state
        std::fs::create_dir_all(dir.path().join(".omx")).unwrap();
        Self { dir }
    }

    pub fn home_path(&self) -> &Path {
        self.dir.path()
    }

    pub fn config_path(&self) -> PathBuf {
        self.dir.path().join("config.toml")
    }
}

// ---------------------------------------------------------------------------
// Binary helpers
// ---------------------------------------------------------------------------

/// Build a Command for the `omx` CLI binary, pointing CODEX_HOME at the test config.
pub fn omx_cmd(config: &TestConfig) -> Command {
    let bin = cargo_bin("omx");
    let mut cmd = Command::new(bin);
    cmd.env("CODEX_HOME", config.home_path());
    cmd.env("HOME", config.home_path());
    cmd
}

/// Build a Command for an MCP server binary.
pub fn mcp_cmd(server: &str, config: &TestConfig) -> Command {
    let bin = cargo_bin(server);
    let mut cmd = Command::new(bin);
    cmd.env("CODEX_HOME", config.home_path());
    cmd.env("HOME", config.home_path());
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd
}

/// Build a Command for a notification hook binary.
pub fn notify_cmd(binary: &str) -> Command {
    let bin = cargo_bin(binary);
    let mut cmd = Command::new(bin);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd
}

/// Resolve a cargo binary path from the target directory.
fn cargo_bin(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // up from tests/ to workspace root
    path.push("target");
    path.push("debug");
    path.push(name);
    path
}

// ---------------------------------------------------------------------------
// JSON-RPC helpers
// ---------------------------------------------------------------------------

pub fn json_rpc_request(id: u64, method: &str, params: serde_json::Value) -> String {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    let body = serde_json::to_string(&msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

pub fn json_rpc_notification(method: &str, params: serde_json::Value) -> String {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    let body = serde_json::to_string(&msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

// ---------------------------------------------------------------------------
// HookEvent fixtures
// ---------------------------------------------------------------------------

pub fn session_start_event() -> HookEvent {
    HookEvent {
        schema_version: "1".into(),
        event: HookEventName::SessionStart,
        timestamp: "2026-04-05T10:00:00Z".into(),
        source: HookSource {
            component: "integration-test".into(),
            worker_id: None,
        },
        context: serde_json::json!({"mode": "autopilot"}),
        session_id: Some("test-sess-001".into()),
    }
}

pub fn failed_event(error: &str) -> HookEvent {
    HookEvent {
        schema_version: "1".into(),
        event: HookEventName::Failed,
        timestamp: "2026-04-05T10:00:00Z".into(),
        source: HookSource {
            component: "integration-test".into(),
            worker_id: Some("w-001".into()),
        },
        context: serde_json::json!({"mode": "team", "error": error}),
        session_id: Some("test-sess-002".into()),
    }
}

pub fn turn_complete_event(worker_id: &str) -> HookEvent {
    HookEvent {
        schema_version: "1".into(),
        event: HookEventName::TurnComplete,
        timestamp: "2026-04-05T10:00:00Z".into(),
        source: HookSource {
            component: "integration-test".into(),
            worker_id: Some(worker_id.into()),
        },
        context: serde_json::json!({"mode": "autopilot", "turn_count": "5"}),
        session_id: Some("test-sess-003".into()),
    }
}
