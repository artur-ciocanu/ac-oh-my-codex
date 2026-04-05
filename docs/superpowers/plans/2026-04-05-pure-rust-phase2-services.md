# Pure Rust Migration — Phase 2: Services Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all `todo!()` stubs in the 5 Phase 2 service crates (`omx-hooks`, `omx-mcp-state`, `omx-mcp-memory`, `omx-mcp-trace`, `omx-mcp-code-intel`) with working implementations so that MCP servers and hooks work end-to-end.

**Architecture:** All 4 MCP servers follow the same pattern: a struct holding an `Arc<FileStateStore>` initialized from `CODEX_HOME` env var (or default `~/.codex`), with tool methods that delegate to the state store. `omx-hooks` is a library that spawns hook executables as child processes, pipes `HookEvent` JSON to stdin, and collects results with timeout enforcement.

**Tech Stack:** Rust 2021 edition, tokio 1.x, rmcp (MCP SDK with stdio transport), omx-state `FileStateStore`, serde/serde_json, schemars, tracing, async-trait, tempfile (tests)

**Spec:** `docs/superpowers/specs/2026-04-04-pure-rust-migration-design.md` (sections 4.5, 5.1, 5.2)

**Phase 1 baseline:** Foundation crates (`omx-types`, `omx-config`, `omx-state`, `omx-mux`) are fully implemented with passing tests.

**Milestone:** All 5 Phase 2 crates have real implementations with passing tests. No `todo!()` remains in these crates.

---

## File Structure

### Modified files

```
crates/omx-hooks/src/lib.rs                  # Implement discover() and dispatch() with timeout
crates/omx-hooks/Cargo.toml                  # Add tempfile dev-dependency
crates/omx-mcp-state/src/main.rs             # Implement all 5 state tools using FileStateStore
crates/omx-mcp-memory/src/main.rs            # Implement all 5 memory/notepad tools
crates/omx-mcp-trace/src/main.rs             # Implement timeline + summary tools
crates/omx-mcp-trace/Cargo.toml              # Add chrono dependency
crates/omx-mcp-code-intel/src/main.rs        # Implement diagnostics + AST search tools
```

---

## Task 1: Implement omx-hooks discover and dispatch

**Files:**
- Modify: `crates/omx-hooks/src/lib.rs`
- Modify: `crates/omx-hooks/Cargo.toml`

This task replaces the two `todo!()` stubs in `ShellHookDispatcher` with working implementations. `discover()` scans a directory for executable files. `dispatch()` spawns each hook as a child process, writes `HookEvent` JSON to its stdin, reads stdout/stderr, and enforces a timeout.

- [ ] **Step 1: Add tempfile dev-dependency**

Add to `crates/omx-hooks/Cargo.toml`:

```toml
[dev-dependencies]
tokio = { workspace = true }
tempfile = "3"
```

(Replace the existing `[dev-dependencies]` section.)

- [ ] **Step 2: Implement the `discover` method**

In `crates/omx-hooks/src/lib.rs`, replace the `discover` method in the `impl HookDispatcher for ShellHookDispatcher` block:

```rust
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

            // Skip directories and hidden files
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
```

- [ ] **Step 3: Add the `is_executable` helper function**

Add after the `impl HookDispatcher for ShellHookDispatcher` block closing brace, before the `#[cfg(test)]` block:

```rust
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
```

- [ ] **Step 4: Implement the `dispatch` method**

Replace the `dispatch` method in the `impl HookDispatcher for ShellHookDispatcher` block:

```rust
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
```

- [ ] **Step 5: Add the `run_hook` async helper**

Add after the `is_executable` functions, before the `#[cfg(test)]` block:

```rust
async fn run_hook(
    path: &Path,
    event_json: &str,
    timeout_ms: u64,
) -> Result<(String, String), String> {
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;

    let mut child = Command::new(path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {e}", path.display()))?;

    // Write event JSON to stdin
    if let Some(mut stdin) = child.stdin.take() {
        let json = event_json.to_string();
        tokio::spawn(async move {
            let _ = stdin.write_all(json.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }

    // Wait with timeout
    let timeout = tokio::time::Duration::from_millis(timeout_ms);
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                Ok((stdout, stderr))
            } else {
                Err(format!(
                    "hook exited with status {}: {}",
                    output.status,
                    stderr.trim()
                ))
            }
        }
        Ok(Err(e)) => Err(format!("failed to wait for hook: {e}")),
        Err(_) => {
            // Timeout — kill the child
            let _ = child.kill().await;
            Err(format!("hook timed out after {timeout_ms}ms"))
        }
    }
}
```

- [ ] **Step 6: Replace the test stubs with real tests**

Replace the entire `#[cfg(test)] mod tests` block:

```rust
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

        // Create an executable script
        let script_path = tmp.path().join("my-hook.sh");
        std::fs::write(&script_path, "#!/bin/sh\necho ok").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        // Create a non-executable file
        std::fs::write(tmp.path().join("readme.txt"), "not a hook").unwrap();

        // Create a hidden file (should be skipped)
        std::fs::write(tmp.path().join(".hidden"), "hidden").unwrap();

        let dispatcher = ShellHookDispatcher::new(5000);
        let hooks = dispatcher.discover(tmp.path()).unwrap();

        assert_eq!(hooks.len(), 2); // my-hook.sh + readme.txt (hidden skipped)
        let names: Vec<&str> = hooks.iter().map(|h| h.name.as_str()).collect();
        assert!(names.contains(&"my-hook"));
        assert!(names.contains(&"readme"));

        // Check executable flag
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

        // Create a hook that echoes stdin to a capture file
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

        // Verify the hook received the event JSON
        let captured = std::fs::read_to_string(&capture_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&captured).unwrap();
        assert_eq!(parsed["event"], "SessionStart");
    }

    #[tokio::test]
    async fn dispatch_enforces_timeout() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a hook that sleeps forever
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

        // 200ms timeout
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
```

- [ ] **Step 7: Verify crate compiles and tests pass**

Run: `cargo test -p omx-hooks`
Expected: 6 tests pass, 0 ignored

- [ ] **Step 8: Commit**

```bash
git add crates/omx-hooks/
git commit -m "feat(omx-hooks): implement hook discovery and dispatch with timeout enforcement"
```

---

## Task 2: Implement omx-mcp-state with FileStateStore integration

**Files:**
- Modify: `crates/omx-mcp-state/src/main.rs`

This task replaces all 5 `todo!()` stubs in the state MCP server. The server holds an `Arc<FileStateStore>` and delegates all operations to it. State is organized as `state/{mode}/{key}.json` files.

- [ ] **Step 1: Rewrite the server with FileStateStore integration**

Replace the entire contents of `crates/omx-mcp-state/src/main.rs`:

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

use omx_state::{FileStateStore, StateStore};
use rmcp::{tool, ServerHandler, ServiceExt};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StateReadParams {
    pub mode: String,
    pub key: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StateWriteParams {
    pub mode: String,
    pub key: String,
    pub value: serde_json::Value,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StateClearParams {
    pub mode: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StateListActiveParams {
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StateGetStatusParams {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
struct StateMcpServer {
    store: Arc<FileStateStore>,
}

impl StateMcpServer {
    fn new(state_root: PathBuf) -> Self {
        Self {
            store: Arc::new(FileStateStore::new(state_root)),
        }
    }

    /// Build the relative path for a mode/key pair: `state/{mode}/{key}.json`
    fn state_path(mode: &str, key: &str) -> PathBuf {
        Path::new("state").join(mode).join(format!("{key}.json"))
    }

    /// Build the relative dir path for a mode: `state/{mode}`
    fn mode_dir(mode: &str) -> PathBuf {
        Path::new("state").join(mode)
    }
}

#[rmcp::tool(tool_box)]
impl StateMcpServer {
    #[tool(description = "Read a state value by mode and key")]
    async fn state_read(&self, #[tool(aggr)] params: StateReadParams) -> String {
        let path = Self::state_path(&params.mode, &params.key);
        match self.store.read::<serde_json::Value>(&path).await {
            Ok(Some(value)) => serde_json::to_string_pretty(&value).unwrap_or_default(),
            Ok(None) => format!("No state found for mode={} key={}", params.mode, params.key),
            Err(e) => format!("Error reading state: {e}"),
        }
    }

    #[tool(description = "Write a state value by mode and key")]
    async fn state_write(&self, #[tool(aggr)] params: StateWriteParams) -> String {
        let path = Self::state_path(&params.mode, &params.key);
        match self.store.write(&path, &params.value).await {
            Ok(()) => format!("State written: mode={} key={}", params.mode, params.key),
            Err(e) => format!("Error writing state: {e}"),
        }
    }

    #[tool(description = "Clear all state for a mode")]
    async fn state_clear(&self, #[tool(aggr)] params: StateClearParams) -> String {
        let dir = Self::mode_dir(&params.mode);
        match self.store.list(&dir).await {
            Ok(entries) => {
                let mut deleted = 0;
                for entry in &entries {
                    // Convert absolute path back to relative for the store
                    if let Ok(relative) = entry.strip_prefix(self.store.root()) {
                        if let Err(e) = self.store.delete(relative).await {
                            return format!("Error clearing state: {e}");
                        }
                        deleted += 1;
                    }
                }
                format!("Cleared {deleted} entries for mode={}", params.mode)
            }
            Err(e) => format!("Error listing state: {e}"),
        }
    }

    #[tool(description = "List all active state modes")]
    async fn state_list_active(&self, #[tool(aggr)] _params: StateListActiveParams) -> String {
        let state_dir = Path::new("state");
        match self.store.list(state_dir).await {
            Ok(entries) => {
                let modes: Vec<String> = entries
                    .iter()
                    .filter(|p| p.is_dir())
                    .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
                    .collect();
                if modes.is_empty() {
                    "No active state modes".to_string()
                } else {
                    serde_json::to_string_pretty(&modes).unwrap_or_default()
                }
            }
            Err(e) => format!("Error listing modes: {e}"),
        }
    }

    #[tool(description = "Get overall state status")]
    async fn state_get_status(&self, #[tool(aggr)] _params: StateGetStatusParams) -> String {
        let state_dir = Path::new("state");
        match self.store.list(state_dir).await {
            Ok(entries) => {
                let mode_count = entries.iter().filter(|p| p.is_dir()).count();
                let status = serde_json::json!({
                    "state_root": self.store.root().display().to_string(),
                    "active_modes": mode_count,
                    "healthy": true,
                });
                serde_json::to_string_pretty(&status).unwrap_or_default()
            }
            Err(e) => {
                let status = serde_json::json!({
                    "state_root": self.store.root().display().to_string(),
                    "active_modes": 0,
                    "healthy": false,
                    "error": e.to_string(),
                });
                serde_json::to_string_pretty(&status).unwrap_or_default()
            }
        }
    }
}

#[rmcp::tool(tool_box)]
impl ServerHandler for StateMcpServer {}

fn resolve_state_root() -> PathBuf {
    std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".codex")
        })
        .join("state")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let state_root = resolve_state_root();
    let server = StateMcpServer::new(state_root);
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 2: Add `dirs` dependency to Cargo.toml**

Add `dirs` to the `[dependencies]` section of `crates/omx-mcp-state/Cargo.toml`:

```toml
dirs = "5"
```

- [ ] **Step 3: Verify crate compiles**

Run: `cargo build -p omx-mcp-state`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/omx-mcp-state/
git commit -m "feat(omx-mcp-state): implement state read/write/clear/list/status MCP tools"
```

---

## Task 3: Implement omx-mcp-memory with file-backed memory and notepad

**Files:**
- Modify: `crates/omx-mcp-memory/src/main.rs`

This task replaces all 5 `todo!()` stubs in the memory MCP server. Memory entries are stored as individual JSON files under `memory/{project}/{key}.json`. Notepad entries are appended as JSONL to `notepad/notes.jsonl` and `notepad/directives.jsonl`.

- [ ] **Step 1: Rewrite the server with FileStateStore integration**

Replace the entire contents of `crates/omx-mcp-memory/src/main.rs`:

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

use omx_state::{FileStateStore, StateStore};
use rmcp::{tool, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MemoryReadParams {
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MemoryWriteParams {
    pub project: Option<String>,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MemoryPruneParams {
    pub project: Option<String>,
    pub older_than_days: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotepadAddNoteParams {
    pub note: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotepadAddDirectiveParams {
    pub directive: String,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryEntry {
    key: String,
    value: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoteEntry {
    note: String,
    tags: Vec<String>,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirectiveEntry {
    directive: String,
    priority: String,
    created_at: String,
}

#[derive(Debug, Clone)]
struct MemoryMcpServer {
    store: Arc<FileStateStore>,
}

impl MemoryMcpServer {
    fn new(state_root: PathBuf) -> Self {
        Self {
            store: Arc::new(FileStateStore::new(state_root)),
        }
    }

    fn memory_path(project: &str, key: &str) -> PathBuf {
        Path::new("memory").join(project).join(format!("{key}.json"))
    }

    fn memory_dir(project: &str) -> PathBuf {
        Path::new("memory").join(project)
    }

    fn now_iso() -> String {
        // Use a simple UTC timestamp without pulling in chrono
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}Z", dur.as_secs())
    }
}

#[rmcp::tool(tool_box)]
impl MemoryMcpServer {
    #[tool(description = "Read project memory entries")]
    async fn project_memory_read(&self, #[tool(aggr)] params: MemoryReadParams) -> String {
        let project = params.project.as_deref().unwrap_or("default");
        let dir = Self::memory_dir(project);

        match self.store.list(&dir).await {
            Ok(entries) => {
                let mut memories = Vec::new();
                for entry_path in &entries {
                    if let Ok(relative) = entry_path.strip_prefix(self.store.root()) {
                        if let Ok(Some(entry)) =
                            self.store.read::<MemoryEntry>(relative).await
                        {
                            memories.push(entry);
                        }
                    }
                }
                if memories.is_empty() {
                    format!("No memory entries for project={project}")
                } else {
                    serde_json::to_string_pretty(&memories).unwrap_or_default()
                }
            }
            Err(e) => format!("Error reading memory: {e}"),
        }
    }

    #[tool(description = "Write a project memory entry")]
    async fn project_memory_write(&self, #[tool(aggr)] params: MemoryWriteParams) -> String {
        let project = params.project.as_deref().unwrap_or("default");
        let path = Self::memory_path(project, &params.key);

        let entry = MemoryEntry {
            key: params.key.clone(),
            value: params.value,
            updated_at: Self::now_iso(),
        };

        match self.store.write(&path, &entry).await {
            Ok(()) => format!("Memory written: project={project} key={}", params.key),
            Err(e) => format!("Error writing memory: {e}"),
        }
    }

    #[tool(description = "Prune old project memory entries")]
    async fn project_memory_prune(&self, #[tool(aggr)] params: MemoryPruneParams) -> String {
        let project = params.project.as_deref().unwrap_or("default");
        let older_than_days = params.older_than_days.unwrap_or(30);
        let dir = Self::memory_dir(project);

        let cutoff_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(older_than_days as u64 * 86400);

        match self.store.list(&dir).await {
            Ok(entries) => {
                let mut pruned = 0;
                for entry_path in &entries {
                    if let Ok(relative) = entry_path.strip_prefix(self.store.root()) {
                        if let Ok(Some(entry)) =
                            self.store.read::<MemoryEntry>(relative).await
                        {
                            // Parse the unix timestamp from updated_at
                            let ts: u64 = entry
                                .updated_at
                                .trim_end_matches('Z')
                                .parse()
                                .unwrap_or(u64::MAX);
                            if ts < cutoff_secs {
                                if self.store.delete(relative).await.is_ok() {
                                    pruned += 1;
                                }
                            }
                        }
                    }
                }
                format!("Pruned {pruned} entries older than {older_than_days} days for project={project}")
            }
            Err(e) => format!("Error pruning memory: {e}"),
        }
    }

    #[tool(description = "Add a note to the notepad")]
    async fn notepad_add_note(&self, #[tool(aggr)] params: NotepadAddNoteParams) -> String {
        let entry = NoteEntry {
            note: params.note,
            tags: params.tags.unwrap_or_default(),
            created_at: Self::now_iso(),
        };

        let path = Path::new("notepad/notes.jsonl");
        match self.store.append_jsonl(path, &entry).await {
            Ok(()) => "Note added".to_string(),
            Err(e) => format!("Error adding note: {e}"),
        }
    }

    #[tool(description = "Add a directive to the notepad")]
    async fn notepad_add_directive(
        &self,
        #[tool(aggr)] params: NotepadAddDirectiveParams,
    ) -> String {
        let entry = DirectiveEntry {
            directive: params.directive,
            priority: params.priority.unwrap_or_else(|| "normal".to_string()),
            created_at: Self::now_iso(),
        };

        let path = Path::new("notepad/directives.jsonl");
        match self.store.append_jsonl(path, &entry).await {
            Ok(()) => "Directive added".to_string(),
            Err(e) => format!("Error adding directive: {e}"),
        }
    }
}

#[rmcp::tool(tool_box)]
impl ServerHandler for MemoryMcpServer {}

fn resolve_state_root() -> PathBuf {
    std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".codex")
        })
        .join("state")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let state_root = resolve_state_root();
    let server = MemoryMcpServer::new(state_root);
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 2: Add `dirs` dependency to Cargo.toml**

Add `dirs` to the `[dependencies]` section of `crates/omx-mcp-memory/Cargo.toml`:

```toml
dirs = "5"
```

- [ ] **Step 3: Verify crate compiles**

Run: `cargo build -p omx-mcp-memory`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/omx-mcp-memory/
git commit -m "feat(omx-mcp-memory): implement memory read/write/prune and notepad tools"
```

---

## Task 4: Implement omx-mcp-trace with JSONL timeline and summary

**Files:**
- Modify: `crates/omx-mcp-trace/src/main.rs`

This task replaces both `todo!()` stubs in the trace MCP server. `trace_timeline` reads a JSONL timeline file and returns entries (with optional limit). `trace_summary` computes statistics from the timeline entries.

- [ ] **Step 1: Rewrite the server with FileStateStore integration**

Replace the entire contents of `crates/omx-mcp-trace/src/main.rs`:

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

use omx_state::{FileStateStore, StateStore};
use rmcp::{tool, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TraceTimelineParams {
    pub session_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TraceSummaryParams {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimelineEntry {
    #[serde(default)]
    turn: u32,
    #[serde(default)]
    event: String,
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    duration_ms: Option<u64>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
struct TraceMcpServer {
    store: Arc<FileStateStore>,
}

impl TraceMcpServer {
    fn new(state_root: PathBuf) -> Self {
        Self {
            store: Arc::new(FileStateStore::new(state_root)),
        }
    }

    fn timeline_path(session_id: &str) -> PathBuf {
        Path::new("trace")
            .join(session_id)
            .join("timeline.jsonl")
    }

    /// Read and parse JSONL timeline entries from disk.
    async fn read_timeline(&self, session_id: &str) -> Result<Vec<TimelineEntry>, String> {
        let path = Self::timeline_path(session_id);
        let full_path = self.store.resolve(&path);

        if !full_path.exists() {
            return Ok(Vec::new());
        }

        let contents = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| format!("failed to read timeline: {e}"))?;

        let entries: Vec<TimelineEntry> = contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(entries)
    }
}

#[rmcp::tool(tool_box)]
impl TraceMcpServer {
    #[tool(description = "Get turn-by-turn timeline for a session")]
    async fn trace_timeline(&self, #[tool(aggr)] params: TraceTimelineParams) -> String {
        let session_id = params.session_id.as_deref().unwrap_or("current");

        match self.read_timeline(session_id).await {
            Ok(entries) => {
                if entries.is_empty() {
                    return format!("No timeline entries for session={session_id}");
                }
                let limited: Vec<&TimelineEntry> = if let Some(limit) = params.limit {
                    entries.iter().rev().take(limit as usize).collect()
                } else {
                    entries.iter().collect()
                };
                serde_json::to_string_pretty(&limited).unwrap_or_default()
            }
            Err(e) => format!("Error reading timeline: {e}"),
        }
    }

    #[tool(description = "Get summary statistics for a session")]
    async fn trace_summary(&self, #[tool(aggr)] params: TraceSummaryParams) -> String {
        let session_id = params.session_id.as_deref().unwrap_or("current");

        match self.read_timeline(session_id).await {
            Ok(entries) => {
                if entries.is_empty() {
                    return format!("No timeline data for session={session_id}");
                }

                let total_turns = entries.len();
                let total_duration_ms: u64 = entries
                    .iter()
                    .filter_map(|e| e.duration_ms)
                    .sum();
                let avg_duration_ms = if total_turns > 0 {
                    total_duration_ms / total_turns as u64
                } else {
                    0
                };

                // Count events by type
                let mut event_counts = std::collections::HashMap::new();
                for entry in &entries {
                    *event_counts.entry(entry.event.clone()).or_insert(0u32) += 1;
                }

                let summary = serde_json::json!({
                    "session_id": session_id,
                    "total_turns": total_turns,
                    "total_duration_ms": total_duration_ms,
                    "avg_duration_ms": avg_duration_ms,
                    "event_counts": event_counts,
                    "first_timestamp": entries.first().map(|e| &e.timestamp),
                    "last_timestamp": entries.last().map(|e| &e.timestamp),
                });

                serde_json::to_string_pretty(&summary).unwrap_or_default()
            }
            Err(e) => format!("Error computing summary: {e}"),
        }
    }
}

#[rmcp::tool(tool_box)]
impl ServerHandler for TraceMcpServer {}

fn resolve_state_root() -> PathBuf {
    std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".codex")
        })
        .join("state")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let state_root = resolve_state_root();
    let server = TraceMcpServer::new(state_root);
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 2: Add `dirs` dependency to Cargo.toml**

Add `dirs` to the `[dependencies]` section of `crates/omx-mcp-trace/Cargo.toml`:

```toml
dirs = "5"
```

- [ ] **Step 3: Verify crate compiles**

Run: `cargo build -p omx-mcp-trace`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/omx-mcp-trace/
git commit -m "feat(omx-mcp-trace): implement timeline and summary MCP tools"
```

---

## Task 5: Implement omx-mcp-code-intel with tsc diagnostics and grep-based search

**Files:**
- Modify: `crates/omx-mcp-code-intel/src/main.rs`

This task replaces both `todo!()` stubs. `diagnostics_typescript` shells out to `npx tsc --noEmit` and parses the output. `ast_pattern_search` uses `rg` (ripgrep) for pattern matching as a pragmatic first implementation (tree-sitter integration can come later).

- [ ] **Step 1: Rewrite the server with process-based implementations**

Replace the entire contents of `crates/omx-mcp-code-intel/src/main.rs`:

```rust
use std::path::PathBuf;

use rmcp::{tool, ServerHandler, ServiceExt};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DiagnosticsParams {
    pub workspace_root: Option<String>,
    pub file_patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AstPatternSearchParams {
    pub pattern: String,
    pub language: Option<String>,
    pub workspace_root: Option<String>,
}

#[derive(Debug, Clone)]
struct CodeIntelMcpServer;

#[rmcp::tool(tool_box)]
impl CodeIntelMcpServer {
    #[tool(description = "Get TypeScript/JavaScript diagnostics for workspace files")]
    async fn diagnostics_typescript(&self, #[tool(aggr)] params: DiagnosticsParams) -> String {
        let workspace = params
            .workspace_root
            .as_deref()
            .unwrap_or(".");

        let mut args = vec!["tsc", "--noEmit", "--pretty", "false"];

        // If file patterns specified, pass them as arguments
        let patterns = params.file_patterns.unwrap_or_default();
        let pattern_refs: Vec<&str> = patterns.iter().map(|s| s.as_str()).collect();
        if !pattern_refs.is_empty() {
            // tsc doesn't support file patterns directly, so we run on the whole project
            // and filter output later
        }

        let output = match tokio::process::Command::new("npx")
            .args(&args)
            .current_dir(workspace)
            .output()
            .await
        {
            Ok(output) => output,
            Err(e) => {
                return format!("Failed to run tsc: {e}. Is Node.js/npx installed?");
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() && stdout.trim().is_empty() {
            return "No TypeScript diagnostics found (clean build)".to_string();
        }

        // Filter output by file patterns if specified
        let result = if !pattern_refs.is_empty() {
            stdout
                .lines()
                .filter(|line| {
                    pattern_refs
                        .iter()
                        .any(|pat| line.contains(pat))
                })
                .collect::<Vec<&str>>()
                .join("\n")
        } else {
            stdout.to_string()
        };

        if result.trim().is_empty() && !stderr.trim().is_empty() {
            format!("tsc stderr: {stderr}")
        } else {
            result
        }
    }

    #[tool(description = "Search code using AST patterns")]
    async fn ast_pattern_search(&self, #[tool(aggr)] params: AstPatternSearchParams) -> String {
        let workspace = params
            .workspace_root
            .as_deref()
            .unwrap_or(".");

        // Map language to ripgrep type filter
        let type_flag: Option<&str> = params.language.as_deref().and_then(|lang| match lang {
            "typescript" | "ts" => Some("ts"),
            "javascript" | "js" => Some("js"),
            "rust" | "rs" => Some("rust"),
            "python" | "py" => Some("py"),
            "go" => Some("go"),
            "java" => Some("java"),
            "c" => Some("c"),
            "cpp" | "c++" => Some("cpp"),
            _ => None,
        });

        let mut cmd = tokio::process::Command::new("rg");
        cmd.arg("--json")
            .arg("--max-count=50")
            .arg(&params.pattern);

        if let Some(t) = type_flag {
            cmd.arg("--type").arg(t);
        }

        cmd.current_dir(workspace);

        let output = match cmd.output().await {
            Ok(output) => output,
            Err(e) => {
                return format!("Failed to run rg (ripgrep): {e}. Is ripgrep installed?");
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.trim().is_empty() {
            return format!("No matches found for pattern: {}", params.pattern);
        }

        // Parse rg JSON output into a simpler format
        let mut results = Vec::new();
        for line in stdout.lines() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                if val["type"] == "match" {
                    let data = &val["data"];
                    let path = data["path"]["text"].as_str().unwrap_or("");
                    let line_number = data["line_number"].as_u64().unwrap_or(0);
                    let text = data["lines"]["text"].as_str().unwrap_or("").trim();
                    results.push(serde_json::json!({
                        "file": path,
                        "line": line_number,
                        "text": text,
                    }));
                }
            }
        }

        if results.is_empty() {
            format!("No matches found for pattern: {}", params.pattern)
        } else {
            serde_json::to_string_pretty(&results).unwrap_or_default()
        }
    }
}

#[rmcp::tool(tool_box)]
impl ServerHandler for CodeIntelMcpServer {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let service = CodeIntelMcpServer
        .serve(rmcp::transport::io::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 2: Verify crate compiles**

Run: `cargo build -p omx-mcp-code-intel`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add crates/omx-mcp-code-intel/
git commit -m "feat(omx-mcp-code-intel): implement tsc diagnostics and ripgrep-based pattern search"
```

---

## Task 6: Workspace verification and final check

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace`
Expected: all crates compile with no errors

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test --workspace`
Expected: all tests pass, 0 ignored

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Verify no todo!() remains in Phase 2 crates**

Run: `rg 'todo!\(' crates/omx-hooks/ crates/omx-mcp-state/ crates/omx-mcp-memory/ crates/omx-mcp-trace/ crates/omx-mcp-code-intel/`
Expected: no matches

- [ ] **Step 5: Fix any issues found in steps 1-4**

Address any compilation errors, test failures, clippy warnings, or remaining `todo!()` stubs.

- [ ] **Step 6: Final commit (if any fixes were needed)**

```bash
git add -A
git commit -m "chore: Phase 2 cleanup — fix warnings and remaining issues"
```
