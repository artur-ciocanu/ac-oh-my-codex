use std::path::{Path, PathBuf};

use async_trait::async_trait;
use omx_types::{HookEvent, OmxError};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Hook descriptor and result (spec section 4.5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDescriptor {
    pub name: String,
    pub path: PathBuf,
    pub executable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub hook: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// HookDispatcher trait (spec section 4.5)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait HookDispatcher: Send + Sync {
    async fn dispatch(&self, event: &HookEvent) -> Vec<HookResult>;
    fn discover(&self, hooks_dir: &Path) -> Result<Vec<HookDescriptor>, OmxError>;
}

// ---------------------------------------------------------------------------
// ShellHookDispatcher (skeleton)
// Contract: spawn executable, write HookEvent JSON to stdin,
//           read JSON from stdout, enforce timeout
// ---------------------------------------------------------------------------

pub struct ShellHookDispatcher {
    hooks: Vec<HookDescriptor>,
    timeout_ms: u64,
}

impl ShellHookDispatcher {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            hooks: Vec::new(),
            timeout_ms,
        }
    }

    pub fn with_hooks(mut self, hooks: Vec<HookDescriptor>) -> Self {
        self.hooks = hooks;
        self
    }

    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
}

#[async_trait]
impl HookDispatcher for ShellHookDispatcher {
    async fn dispatch(&self, event: &HookEvent) -> Vec<HookResult> {
        let event_json = match serde_json::to_string(event) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("failed to serialize HookEvent: {e}");
                return Vec::new();
            }
        };

        let mut results = Vec::new();

        for hook in &self.hooks {
            if !hook.executable {
                tracing::debug!("skipping non-executable hook: {}", hook.name);
                continue;
            }

            let start = std::time::Instant::now();
            let result = run_hook(&hook.path, &event_json, self.timeout_ms).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok((stdout, stderr)) => {
                    results.push(HookResult {
                        hook: hook.name.clone(),
                        success: true,
                        stdout,
                        stderr,
                        duration_ms,
                    });
                }
                Err(e) => {
                    results.push(HookResult {
                        hook: hook.name.clone(),
                        success: false,
                        stdout: String::new(),
                        stderr: e,
                        duration_ms,
                    });
                }
            }
        }

        results
    }

    fn discover(&self, hooks_dir: &Path) -> Result<Vec<HookDescriptor>, OmxError> {
        if !hooks_dir.exists() {
            return Ok(Vec::new());
        }

        let mut hooks = Vec::new();
        let entries = std::fs::read_dir(hooks_dir)
            .map_err(|e| OmxError::Hook(format!("failed to read hooks dir: {e}")))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| OmxError::Hook(format!("failed to read dir entry: {e}")))?;
            let path = entry.path();

            if path.is_dir() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }

            let executable = is_executable(&path);
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            hooks.push(HookDescriptor {
                name,
                path,
                executable,
            });
        }

        hooks.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(hooks)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e, "exe" | "bat" | "cmd"))
        .unwrap_or(false)
}

async fn run_hook(
    path: &Path,
    event_json: &str,
    timeout_ms: u64,
) -> Result<(String, String), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::process::Command;

    let mut child = Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {e}", path.display()))?;

    // Write event JSON to stdin in a background task
    if let Some(mut stdin) = child.stdin.take() {
        let payload = event_json.to_string();
        tokio::spawn(async move {
            let _ = stdin.write_all(payload.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }

    // Take stdout/stderr handles for reading in background tasks
    let mut stdout_handle = child.stdout.take();
    let mut stderr_handle = child.stderr.take();

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(ref mut r) = stdout_handle {
            let _ = r.read_to_end(&mut buf).await;
        }
        String::from_utf8_lossy(&buf).to_string()
    });

    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(ref mut r) = stderr_handle {
            let _ = r.read_to_end(&mut buf).await;
        }
        String::from_utf8_lossy(&buf).to_string()
    });

    // Wait for the child with timeout; child.wait() borrows &mut so select! works
    let timeout = tokio::time::Duration::from_millis(timeout_ms);
    tokio::select! {
        status = child.wait() => {
            let status = status.map_err(|e| format!("failed to wait for hook: {e}"))?;
            let stdout = stdout_task.await.unwrap_or_default();
            let stderr = stderr_task.await.unwrap_or_default();
            if status.success() {
                Ok((stdout, stderr))
            } else {
                Err(format!(
                    "hook exited with status {}: {}",
                    status,
                    stderr.trim()
                ))
            }
        }
        _ = tokio::time::sleep(timeout) => {
            let _ = child.start_kill();
            Err(format!("hook timed out after {timeout_ms}ms"))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_hook_dispatcher_stores_timeout() {
        let dispatcher = ShellHookDispatcher::new(5000);
        assert_eq!(dispatcher.timeout_ms(), 5000);
    }

    #[test]
    fn discover_finds_executable_files_in_hooks_dir() {
        let tmp = tempfile::tempdir().unwrap();

        let script_path = tmp.path().join("my-hook.sh");
        std::fs::write(&script_path, "#!/bin/sh\necho ok").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        std::fs::write(tmp.path().join("readme.txt"), "not a hook").unwrap();
        std::fs::write(tmp.path().join(".hidden"), "hidden").unwrap();

        let dispatcher = ShellHookDispatcher::new(5000);
        let hooks = dispatcher.discover(tmp.path()).unwrap();

        assert_eq!(hooks.len(), 2);
        let names: Vec<&str> = hooks.iter().map(|h| h.name.as_str()).collect();
        assert!(names.contains(&"my-hook"));
        assert!(names.contains(&"readme"));

        let my_hook = hooks.iter().find(|h| h.name == "my-hook").unwrap();
        #[cfg(unix)]
        assert!(my_hook.executable);
    }

    #[test]
    fn discover_returns_empty_for_missing_dir() {
        let dispatcher = ShellHookDispatcher::new(5000);
        let hooks = dispatcher.discover(Path::new("/nonexistent/hooks/dir")).unwrap();
        assert!(hooks.is_empty());
    }

    #[tokio::test]
    async fn dispatch_sends_hook_event_json_to_stdin() {
        let tmp = tempfile::tempdir().unwrap();

        let capture_path = tmp.path().join("captured.json");
        let script = format!(
            "#!/bin/sh\ncat > '{}'\necho '{{\"ok\":true}}'",
            capture_path.display()
        );
        let script_path = tmp.path().join("echo-hook.sh");
        std::fs::write(&script_path, &script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        let hooks = vec![HookDescriptor {
            name: "echo-hook".into(),
            path: script_path,
            executable: true,
        }];

        let dispatcher = ShellHookDispatcher::new(5000).with_hooks(hooks);

        let event = HookEvent {
            schema_version: "1".into(),
            event: omx_types::HookEventName::SessionStart,
            timestamp: "2026-04-05T00:00:00Z".into(),
            source: omx_types::HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({"test": true}),
            session_id: Some("test-session".into()),
        };

        let results = dispatcher.dispatch(&event).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].hook, "echo-hook");

        let captured = std::fs::read_to_string(&capture_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&captured).unwrap();
        assert_eq!(parsed["event"], "SessionStart");
    }

    #[tokio::test]
    async fn dispatch_enforces_timeout() {
        let tmp = tempfile::tempdir().unwrap();

        let script_path = tmp.path().join("slow-hook.sh");
        std::fs::write(&script_path, "#!/bin/sh\nsleep 60").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        let hooks = vec![HookDescriptor {
            name: "slow-hook".into(),
            path: script_path,
            executable: true,
        }];

        let dispatcher = ShellHookDispatcher::new(200).with_hooks(hooks);

        let event = HookEvent {
            schema_version: "1".into(),
            event: omx_types::HookEventName::SessionStart,
            timestamp: "2026-04-05T00:00:00Z".into(),
            source: omx_types::HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: None,
        };

        let results = dispatcher.dispatch(&event).await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].stderr.contains("timed out"));
    }

    #[tokio::test]
    async fn dispatch_skips_non_executable_hooks() {
        let hooks = vec![HookDescriptor {
            name: "non-exec".into(),
            path: PathBuf::from("/tmp/fake"),
            executable: false,
        }];

        let dispatcher = ShellHookDispatcher::new(5000).with_hooks(hooks);

        let event = HookEvent {
            schema_version: "1".into(),
            event: omx_types::HookEventName::SessionStart,
            timestamp: "2026-04-05T00:00:00Z".into(),
            source: omx_types::HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: None,
        };

        let results = dispatcher.dispatch(&event).await;
        assert!(results.is_empty());
    }
}
