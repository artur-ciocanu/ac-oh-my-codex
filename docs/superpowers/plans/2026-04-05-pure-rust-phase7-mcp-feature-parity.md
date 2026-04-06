# Phase 7: MCP Feature Parity — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete all 5 MCP servers from 18 to 32 tools, adding notepad reads, LSP integration, ast-grep, enhanced trace filtering, and team nudge capabilities.

**Architecture:** Enhance 5 existing MCP binary crates. Each crate is a standalone Rust binary using `rmcp` for MCP protocol, `omx-state` for persistence, and `tokio` for async. The omx-mcp-code-intel crate gains a lightweight LSP client module (JSON-RPC over stdio) for 6 new LSP tools. All new tools follow the established `#[tool(description)]` + `#[tool(aggr)]` pattern.

**Tech Stack:** Rust 2021, rmcp, schemars, serde/serde_json, tokio, omx-state, omx-types, lsp-types (new dep)

---

## File Structure

### Modified: `crates/omx-mcp-state/src/main.rs`
Add 2 new tools: `state_delete`, `state_list` (per-mode key listing). Add 2 new param structs.

### Modified: `crates/omx-mcp-memory/src/main.rs`
Add 4 new tools: `notepad_read`, `notepad_write_priority`, `notepad_write_working`, `notepad_stats`. Add 4 new param structs. Restructure notepad storage from flat JSONL to section-based (`notepad/{priority,working}/entries.jsonl`).

### Modified: `crates/omx-mcp-code-intel/Cargo.toml`
Add `lsp-types`, `serde` dependencies.

### Modified: `crates/omx-mcp-code-intel/src/main.rs`
Add `mod lsp;` declaration. Add 7 new tools: `ast_grep_replace`, `lsp_diagnostics`, `lsp_document_symbols`, `lsp_workspace_symbols`, `lsp_hover`, `lsp_find_references`, `lsp_servers`. Convert from stateless `CodeIntelMcpServer` unit struct to stateful struct holding `LspClientManager`.

### New: `crates/omx-mcp-code-intel/src/lsp.rs`
Lightweight LSP client: JSON-RPC framing (Content-Length headers), request/response lifecycle, server process management, connection pooling by language.

### Modified: `crates/omx-mcp-trace/src/main.rs`
Enhance `trace_timeline` and `trace_summary` with filter params (`mode`, `time_range`, `severity`). Add metrics fields to summary output (`tokens_used`, `quota_remaining`).

### Modified: `crates/omx-mcp-team/src/main.rs`
Add 1 new tool: `omx_run_team_nudge`. Add job ID tracking to `omx_run_team_start` response.

### Modified: `Cargo.toml` (workspace root)
Add `lsp-types` to workspace dependencies.

---

## Task 1: omx-mcp-state — add state_delete and state_list tools

**Files:**
- Modify: `crates/omx-mcp-state/src/main.rs`

- [ ] **Step 1: Add StateDeleteParams and StateListParams structs**

Add after the existing `StateGetStatusParams` struct in `crates/omx-mcp-state/src/main.rs`:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StateDeleteParams {
    /// Mode scope (e.g. "autopilot", "ralph")
    pub mode: String,
    /// Key to delete
    pub key: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StateListParams {
    /// Mode scope — list all keys under this mode
    pub mode: String,
    pub session_id: Option<String>,
}
```

- [ ] **Step 2: Add state_delete tool**

Add inside the `#[rmcp::tool(tool_box)] impl StateMcpServer` block, after the `state_get_status` method:

```rust
    #[tool(description = "Delete a single state key within a mode")]
    async fn state_delete(&self, #[tool(aggr)] params: StateDeleteParams) -> String {
        let path = Self::state_path(&params.mode, &params.key);
        match self.store.delete(&path).await {
            Ok(()) => format!("Deleted: mode={} key={}", params.mode, params.key),
            Err(e) => format!("Error deleting state: {e}"),
        }
    }
```

- [ ] **Step 3: Add state_list tool**

Add after the `state_delete` method:

```rust
    #[tool(description = "List all keys stored under a specific mode")]
    async fn state_list(&self, #[tool(aggr)] params: StateListParams) -> String {
        let dir = Self::mode_dir(&params.mode);
        match self.store.list(&dir).await {
            Ok(entries) => {
                let keys: Vec<String> = entries
                    .iter()
                    .filter_map(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .and_then(|n| n.strip_suffix(".json"))
                            .map(String::from)
                    })
                    .collect();
                if keys.is_empty() {
                    format!("No keys found for mode={}", params.mode)
                } else {
                    serde_json::to_string_pretty(&keys).unwrap_or_default()
                }
            }
            Err(e) => format!("Error listing keys: {e}"),
        }
    }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p omx-mcp-state`
Expected: success

- [ ] **Step 5: Commit**

```bash
git add crates/omx-mcp-state/src/main.rs
git commit -m "feat(omx-mcp-state): add state_delete and state_list tools"
```

---

## Task 2: omx-mcp-memory — add 4 notepad tools

**Files:**
- Modify: `crates/omx-mcp-memory/src/main.rs`

- [ ] **Step 1: Add NotepadReadParams, NotepadWritePriorityParams, NotepadWriteWorkingParams, NotepadStatsParams structs**

Add after the existing `NotepadAddDirectiveParams` struct:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotepadReadParams {
    /// Section to read: "priority" or "working"
    pub section: String,
    /// Maximum number of entries to return (default: all)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotepadWritePriorityParams {
    /// Content to write to priority notepad
    pub content: String,
    /// Optional tags for categorization
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotepadWriteWorkingParams {
    /// Content to write to working notepad
    pub content: String,
    /// Optional tags for categorization
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NotepadStatsParams {
    /// Optional section filter: "priority", "working", or omit for both
    pub section: Option<String>,
}
```

- [ ] **Step 2: Add a helper to read JSONL entries**

Add this helper method to the `impl MemoryMcpServer` block (the non-tool one):

```rust
    fn notepad_path(section: &str) -> PathBuf {
        Path::new("notepad").join(section).join("entries.jsonl")
    }

    async fn read_notepad_entries(&self, section: &str) -> Result<Vec<NoteEntry>, String> {
        let path = Self::notepad_path(section);
        let full_path = self.store.resolve(&path);

        if !full_path.exists() {
            return Ok(Vec::new());
        }

        let contents = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| format!("failed to read notepad: {e}"))?;

        let entries: Vec<NoteEntry> = contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        Ok(entries)
    }
```

- [ ] **Step 3: Add notepad_read tool**

Add inside the `#[rmcp::tool(tool_box)] impl MemoryMcpServer` block, after the `notepad_add_directive` method:

```rust
    #[tool(description = "Read notepad contents from a section (priority or working)")]
    async fn notepad_read(&self, #[tool(aggr)] params: NotepadReadParams) -> String {
        let section = params.section.as_str();
        if section != "priority" && section != "working" {
            return "Error: section must be \"priority\" or \"working\"".to_string();
        }

        match self.read_notepad_entries(section).await {
            Ok(entries) => {
                if entries.is_empty() {
                    return format!("No entries in {section} notepad");
                }
                let limited: Vec<&NoteEntry> = if let Some(limit) = params.limit {
                    entries.iter().rev().take(limit as usize).collect()
                } else {
                    entries.iter().collect()
                };
                serde_json::to_string_pretty(&limited).unwrap_or_default()
            }
            Err(e) => format!("Error reading notepad: {e}"),
        }
    }
```

- [ ] **Step 4: Add notepad_write_priority tool**

```rust
    #[tool(description = "Write to priority notepad (high-importance items that surface in context)")]
    async fn notepad_write_priority(
        &self,
        #[tool(aggr)] params: NotepadWritePriorityParams,
    ) -> String {
        let entry = NoteEntry {
            note: params.content,
            tags: params.tags.unwrap_or_default(),
            created_at: Self::now_iso(),
        };

        let path = Self::notepad_path("priority");
        match self.store.append_jsonl(&path, &entry).await {
            Ok(()) => "Priority note added".to_string(),
            Err(e) => format!("Error writing priority note: {e}"),
        }
    }
```

- [ ] **Step 5: Add notepad_write_working tool**

```rust
    #[tool(description = "Write to working notepad (scratch/WIP items for background reference)")]
    async fn notepad_write_working(
        &self,
        #[tool(aggr)] params: NotepadWriteWorkingParams,
    ) -> String {
        let entry = NoteEntry {
            note: params.content,
            tags: params.tags.unwrap_or_default(),
            created_at: Self::now_iso(),
        };

        let path = Self::notepad_path("working");
        match self.store.append_jsonl(&path, &entry).await {
            Ok(()) => "Working note added".to_string(),
            Err(e) => format!("Error writing working note: {e}"),
        }
    }
```

- [ ] **Step 6: Add notepad_stats tool**

```rust
    #[tool(description = "Get notepad statistics: entry count, size, last modified")]
    async fn notepad_stats(&self, #[tool(aggr)] params: NotepadStatsParams) -> String {
        let sections: Vec<&str> = match params.section.as_deref() {
            Some("priority") => vec!["priority"],
            Some("working") => vec!["working"],
            _ => vec!["priority", "working"],
        };

        let mut stats = serde_json::Map::new();

        for section in sections {
            let entries = match self.read_notepad_entries(section).await {
                Ok(e) => e,
                Err(e) => {
                    stats.insert(
                        section.to_string(),
                        serde_json::json!({"error": e}),
                    );
                    continue;
                }
            };

            let path = Self::notepad_path(section);
            let full_path = self.store.resolve(&path);
            let file_size = tokio::fs::metadata(&full_path)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            let last_modified = entries.last().map(|e| e.created_at.clone());

            stats.insert(
                section.to_string(),
                serde_json::json!({
                    "entry_count": entries.len(),
                    "file_size_bytes": file_size,
                    "last_modified": last_modified,
                }),
            );
        }

        serde_json::to_string_pretty(&serde_json::Value::Object(stats)).unwrap_or_default()
    }
```

- [ ] **Step 7: Verify compilation**

Run: `cargo build -p omx-mcp-memory`
Expected: success

- [ ] **Step 8: Commit**

```bash
git add crates/omx-mcp-memory/src/main.rs
git commit -m "feat(omx-mcp-memory): add notepad_read, notepad_write_priority, notepad_write_working, notepad_stats tools"
```

---

## Task 3: omx-mcp-trace — enhanced filtering and metrics

**Files:**
- Modify: `crates/omx-mcp-trace/src/main.rs`

- [ ] **Step 1: Update TraceTimelineParams with filter fields**

Replace the existing `TraceTimelineParams` struct:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TraceTimelineParams {
    pub session_id: Option<String>,
    pub limit: Option<u32>,
    /// Filter by mode name (e.g. "autopilot", "ralph")
    pub mode: Option<String>,
    /// Filter events at or after this ISO timestamp
    pub time_start: Option<String>,
    /// Filter events at or before this ISO timestamp
    pub time_end: Option<String>,
    /// Filter by severity: "info", "warn", "error"
    pub severity: Option<String>,
}
```

- [ ] **Step 2: Update TraceSummaryParams with filter fields**

Replace the existing `TraceSummaryParams` struct:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TraceSummaryParams {
    pub session_id: Option<String>,
    /// Filter by mode name
    pub mode: Option<String>,
    /// Filter events at or after this ISO timestamp
    pub time_start: Option<String>,
    /// Filter events at or before this ISO timestamp
    pub time_end: Option<String>,
}
```

- [ ] **Step 3: Add a filter_entries helper to TraceMcpServer**

Add this method to the non-tool `impl TraceMcpServer` block:

```rust
    fn filter_entries(
        entries: Vec<TimelineEntry>,
        mode: Option<&str>,
        time_start: Option<&str>,
        time_end: Option<&str>,
        severity: Option<&str>,
    ) -> Vec<TimelineEntry> {
        entries
            .into_iter()
            .filter(|e| {
                if let Some(m) = mode {
                    let entry_mode = e.extra.get("mode").and_then(|v| v.as_str()).unwrap_or("");
                    if entry_mode != m {
                        return false;
                    }
                }
                if let Some(start) = time_start {
                    if e.timestamp.as_str() < start {
                        return false;
                    }
                }
                if let Some(end) = time_end {
                    if e.timestamp.as_str() > end {
                        return false;
                    }
                }
                if let Some(sev) = severity {
                    let entry_sev = e.extra.get("severity").and_then(|v| v.as_str()).unwrap_or("info");
                    if entry_sev != sev {
                        return false;
                    }
                }
                true
            })
            .collect()
    }
```

- [ ] **Step 4: Update trace_timeline tool to use filters**

Replace the existing `trace_timeline` method body:

```rust
    #[tool(description = "Get turn-by-turn timeline for a session with optional filters")]
    async fn trace_timeline(&self, #[tool(aggr)] params: TraceTimelineParams) -> String {
        let session_id = params.session_id.as_deref().unwrap_or("current");

        match self.read_timeline(session_id).await {
            Ok(entries) => {
                if entries.is_empty() {
                    return format!("No timeline entries for session={session_id}");
                }
                let filtered = Self::filter_entries(
                    entries,
                    params.mode.as_deref(),
                    params.time_start.as_deref(),
                    params.time_end.as_deref(),
                    params.severity.as_deref(),
                );
                let limited: Vec<&TimelineEntry> = if let Some(limit) = params.limit {
                    filtered.iter().rev().take(limit as usize).collect()
                } else {
                    filtered.iter().collect()
                };
                if limited.is_empty() {
                    "No entries match the given filters".to_string()
                } else {
                    serde_json::to_string_pretty(&limited).unwrap_or_default()
                }
            }
            Err(e) => format!("Error reading timeline: {e}"),
        }
    }
```

- [ ] **Step 5: Update trace_summary tool with filters and metrics**

Replace the existing `trace_summary` method body:

```rust
    #[tool(description = "Get summary statistics for a session with optional filters")]
    async fn trace_summary(&self, #[tool(aggr)] params: TraceSummaryParams) -> String {
        let session_id = params.session_id.as_deref().unwrap_or("current");

        match self.read_timeline(session_id).await {
            Ok(entries) => {
                if entries.is_empty() {
                    return format!("No timeline data for session={session_id}");
                }

                let filtered = Self::filter_entries(
                    entries,
                    params.mode.as_deref(),
                    params.time_start.as_deref(),
                    params.time_end.as_deref(),
                    None,
                );

                if filtered.is_empty() {
                    return "No entries match the given filters".to_string();
                }

                let total_turns = filtered.len();
                let total_duration_ms: u64 = filtered
                    .iter()
                    .filter_map(|e| e.duration_ms)
                    .sum();
                let avg_duration_ms = if total_turns > 0 {
                    total_duration_ms / total_turns as u64
                } else {
                    0
                };

                let mut event_counts = std::collections::HashMap::new();
                for entry in &filtered {
                    *event_counts.entry(entry.event.clone()).or_insert(0u32) += 1;
                }

                let tokens_used: u64 = filtered
                    .iter()
                    .filter_map(|e| e.extra.get("tokens_used").and_then(|v| v.as_u64()))
                    .sum();
                let quota_remaining = filtered
                    .last()
                    .and_then(|e| e.extra.get("quota_remaining").and_then(|v| v.as_u64()));

                let summary = serde_json::json!({
                    "session_id": session_id,
                    "total_turns": total_turns,
                    "total_duration_ms": total_duration_ms,
                    "avg_duration_ms": avg_duration_ms,
                    "event_counts": event_counts,
                    "first_timestamp": filtered.first().map(|e| &e.timestamp),
                    "last_timestamp": filtered.last().map(|e| &e.timestamp),
                    "tokens_used": tokens_used,
                    "quota_remaining": quota_remaining,
                });

                serde_json::to_string_pretty(&summary).unwrap_or_default()
            }
            Err(e) => format!("Error computing summary: {e}"),
        }
    }
```

- [ ] **Step 6: Verify compilation**

Run: `cargo build -p omx-mcp-trace`
Expected: success

- [ ] **Step 7: Commit**

```bash
git add crates/omx-mcp-trace/src/main.rs
git commit -m "feat(omx-mcp-trace): add mode/time_range/severity filters and token metrics to trace tools"
```

---

## Task 4: omx-mcp-team — add team_nudge and job ID tracking

**Files:**
- Modify: `crates/omx-mcp-team/src/main.rs`

- [ ] **Step 1: Add TeamNudgeParams struct and uuid dependency**

Add at the top of `crates/omx-mcp-team/src/main.rs`, after the existing use statements:

```rust
use uuid::Uuid;
```

Add after the existing `TeamCleanupParams` struct:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TeamNudgeParams {
    /// Team name
    pub team_name: String,
    /// Worker ID to nudge
    pub worker_id: String,
    /// Optional message to send with the nudge
    pub message: Option<String>,
}
```

- [ ] **Step 2: Add uuid dependency to Cargo.toml**

In `crates/omx-mcp-team/Cargo.toml`, add to `[dependencies]`:

```toml
uuid = { workspace = true }
```

- [ ] **Step 3: Update omx_run_team_start to return job_id**

Replace the `omx_run_team_start` method body:

```rust
    #[tool(description = "Start a new team run with N workers of a given role")]
    async fn omx_run_team_start(&self, #[tool(aggr)] params: TeamStartParams) -> String {
        let config = match parse_team_spec(params.workers, &params.role, &params.task, params.model)
        {
            Ok(c) => c,
            Err(e) => return format!("{{\"error\": \"{e}\"}}"),
        };

        let team_name = config.name.0.clone();
        let job_id = Uuid::new_v4().to_string();

        let mut runtime = self.runtime.lock().await;
        match runtime.start(config).await {
            Ok(()) => {
                serde_json::json!({
                    "status": "started",
                    "team_name": team_name,
                    "job_id": job_id
                })
                .to_string()
            }
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }
```

- [ ] **Step 4: Add omx_run_team_nudge tool**

Add after the `omx_run_team_cleanup` method:

```rust
    #[tool(description = "Send a nudge to an idle worker to resume work")]
    async fn omx_run_team_nudge(&self, #[tool(aggr)] params: TeamNudgeParams) -> String {
        let worker_id = omx_types::WorkerId(params.worker_id.clone());
        let message = params
            .message
            .unwrap_or_else(|| "Nudge: please check your inbox and continue working.".to_string());

        let runtime = self.runtime.lock().await;
        let leader_id = omx_types::WorkerId("leader".to_string());
        match runtime.send_message(&leader_id, &worker_id, &message).await {
            Ok(()) => {
                serde_json::json!({
                    "status": "nudged",
                    "worker_id": params.worker_id,
                    "message": message
                })
                .to_string()
            }
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p omx-mcp-team`
Expected: success

- [ ] **Step 6: Commit**

```bash
git add crates/omx-mcp-team/src/main.rs crates/omx-mcp-team/Cargo.toml
git commit -m "feat(omx-mcp-team): add team_nudge tool and job_id tracking in team_start"
```

---

## Task 5: omx-mcp-code-intel — add ast_grep_replace tool

**Files:**
- Modify: `crates/omx-mcp-code-intel/src/main.rs`

- [ ] **Step 1: Add AstGrepReplaceParams struct**

Add after the existing `AstPatternSearchParams` struct:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AstGrepReplaceParams {
    /// ast-grep pattern to match
    pub pattern: String,
    /// Replacement pattern
    pub replacement: String,
    /// Language: ts, js, rust, py, go, java, c, cpp
    pub language: String,
    /// Workspace root (default: ".")
    pub workspace_root: Option<String>,
    /// Dry run — show changes without applying (default: true)
    pub dry_run: Option<bool>,
}
```

- [ ] **Step 2: Add ast_grep_replace tool**

Add inside the `#[rmcp::tool(tool_box)] impl CodeIntelMcpServer` block, after the `ast_pattern_search` method:

```rust
    #[tool(description = "Structural search and replace using ast-grep")]
    async fn ast_grep_replace(&self, #[tool(aggr)] params: AstGrepReplaceParams) -> String {
        let workspace = params.workspace_root.as_deref().unwrap_or(".");
        let dry_run = params.dry_run.unwrap_or(true);

        let mut cmd = tokio::process::Command::new("sg");

        if dry_run {
            cmd.arg("scan");
        } else {
            cmd.arg("scan").arg("--rewrite").arg(&params.replacement);
        }

        cmd.arg("--pattern").arg(&params.pattern);
        cmd.arg("--lang").arg(&params.language);
        cmd.arg("--json");
        cmd.current_dir(workspace);

        let output = match cmd.output().await {
            Ok(output) => output,
            Err(e) => {
                return format!(
                    "Failed to run ast-grep (sg): {e}. Install via: cargo install ast-grep"
                );
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if stdout.trim().is_empty() {
            if !stderr.trim().is_empty() {
                return format!("ast-grep error: {stderr}");
            }
            return format!(
                "No matches found for pattern: {} (lang: {})",
                params.pattern, params.language
            );
        }

        if dry_run {
            format!("Dry run results:\n{stdout}")
        } else {
            format!("Applied replacements:\n{stdout}")
        }
    }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build -p omx-mcp-code-intel`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add crates/omx-mcp-code-intel/src/main.rs
git commit -m "feat(omx-mcp-code-intel): add ast_grep_replace tool for structural search/replace"
```

---

## Task 6: omx-mcp-code-intel — LSP client infrastructure

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/omx-mcp-code-intel/Cargo.toml`
- Create: `crates/omx-mcp-code-intel/src/lsp.rs`
- Modify: `crates/omx-mcp-code-intel/src/main.rs`

- [ ] **Step 1: Add lsp-types workspace dependency**

In the root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
lsp-types = "0.97"
```

- [ ] **Step 2: Update omx-mcp-code-intel Cargo.toml**

Replace `crates/omx-mcp-code-intel/Cargo.toml` dependencies section:

```toml
[package]
name = "omx-mcp-code-intel"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-config = { path = "../omx-config" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
rmcp = { workspace = true }
schemars = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
lsp-types = { workspace = true }
```

- [ ] **Step 3: Create lsp.rs with LspClient**

Create `crates/omx-mcp-code-intel/src/lsp.rs`:

```rust
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, oneshot};

/// A lightweight LSP client that communicates via JSON-RPC over stdio.
pub struct LspClient {
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<serde_json::Value>>>>,
    next_id: AtomicI64,
    _child: Arc<Mutex<Child>>,
    pub language: String,
    pub server_cmd: String,
}

impl LspClient {
    /// Spawn an LSP server process and initialize the protocol.
    pub async fn start(
        language: &str,
        server_cmd: &str,
        server_args: &[&str],
        workspace_root: &Path,
    ) -> Result<Self, String> {
        let mut child = Command::new(server_cmd)
            .args(server_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .current_dir(workspace_root)
            .spawn()
            .map_err(|e| format!("Failed to start {server_cmd}: {e}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout".to_string())?;

        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<serde_json::Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Spawn reader task to dispatch responses
        let pending_clone = pending.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                // Read Content-Length header
                let mut header_line = String::new();
                if reader.read_line(&mut header_line).await.unwrap_or(0) == 0 {
                    break;
                }
                let content_length: usize = header_line
                    .trim()
                    .strip_prefix("Content-Length: ")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                if content_length == 0 {
                    continue;
                }

                // Read blank line separator
                let mut blank = String::new();
                let _ = reader.read_line(&mut blank).await;

                // Read body
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).await.is_err() {
                    break;
                }

                let msg: serde_json::Value = match serde_json::from_slice(&body) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Dispatch response by ID
                if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
                    let mut map = pending_clone.lock().await;
                    if let Some(tx) = map.remove(&id) {
                        let _ = tx.send(msg);
                    }
                }
            }
        });

        let client = Self {
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            next_id: AtomicI64::new(1),
            _child: Arc::new(Mutex::new(child)),
            language: language.to_string(),
            server_cmd: server_cmd.to_string(),
        };

        // Send initialize request
        let root_uri = format!(
            "file://{}",
            workspace_root.to_str().unwrap_or(".")
        );
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {},
        });

        let response = client.request("initialize", init_params).await?;
        if response.get("error").is_some() {
            return Err(format!(
                "LSP initialize failed: {}",
                response["error"]
            ));
        }

        // Send initialized notification
        client.notify("initialized", serde_json::json!({})).await?;

        Ok(client)
    }

    /// Send a JSON-RPC request and wait for the response.
    pub async fn request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let body = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(frame.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to LSP stdin: {e}"))?;
            stdin.flush().await.map_err(|e| format!("Flush failed: {e}"))?;
        }

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| "LSP request timed out after 30s".to_string())?
            .map_err(|_| "LSP response channel closed".to_string())
    }

    /// Send a JSON-RPC notification (no response expected).
    pub async fn notify(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), String> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let body = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(frame.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to LSP stdin: {e}"))?;
        stdin.flush().await.map_err(|e| format!("Flush failed: {e}"))?;
        Ok(())
    }
}

/// Manages LSP client connections by language, reusing existing connections.
pub struct LspClientManager {
    clients: Mutex<HashMap<String, Arc<LspClient>>>,
}

impl LspClientManager {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create an LSP client for the given language.
    pub async fn get_client(
        &self,
        language: &str,
        workspace_root: &Path,
    ) -> Result<Arc<LspClient>, String> {
        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get(language) {
            return Ok(client.clone());
        }

        let (cmd, args): (&str, Vec<&str>) = match language {
            "typescript" | "ts" | "javascript" | "js" => {
                ("typescript-language-server", vec!["--stdio"])
            }
            "rust" | "rs" => ("rust-analyzer", vec![]),
            "python" | "py" => ("pylsp", vec![]),
            "go" => ("gopls", vec!["serve"]),
            "java" => ("jdtls", vec![]),
            _ => {
                return Err(format!(
                    "No LSP server configured for language: {language}"
                ))
            }
        };

        let client = LspClient::start(language, cmd, &args, workspace_root).await?;
        let client = Arc::new(client);
        clients.insert(language.to_string(), client.clone());
        Ok(client)
    }

    /// List all active LSP connections.
    pub async fn list_servers(&self) -> Vec<(String, String)> {
        let clients = self.clients.lock().await;
        clients
            .iter()
            .map(|(lang, c)| (lang.clone(), c.server_cmd.clone()))
            .collect()
    }
}
```

- [ ] **Step 4: Add mod lsp to main.rs and convert server to stateful struct**

At the top of `crates/omx-mcp-code-intel/src/main.rs`, replace the module-level code up through the `CodeIntelMcpServer` struct definition:

```rust
use std::sync::Arc;

use rmcp::{tool, ServerHandler, ServiceExt};
use serde::Deserialize;

mod lsp;

use lsp::LspClientManager;

// ... (keep existing param structs) ...

#[derive(Clone)]
struct CodeIntelMcpServer {
    lsp_manager: Arc<LspClientManager>,
}

impl std::fmt::Debug for CodeIntelMcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeIntelMcpServer").finish()
    }
}
```

Then update the `main` function:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let server = CodeIntelMcpServer {
        lsp_manager: Arc::new(LspClientManager::new()),
    };
    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo build -p omx-mcp-code-intel`
Expected: success

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/omx-mcp-code-intel/Cargo.toml crates/omx-mcp-code-intel/src/lsp.rs crates/omx-mcp-code-intel/src/main.rs
git commit -m "feat(omx-mcp-code-intel): add LSP client infrastructure with JSON-RPC over stdio"
```

---

## Task 7: omx-mcp-code-intel — add 6 LSP tools

**Files:**
- Modify: `crates/omx-mcp-code-intel/src/main.rs`

- [ ] **Step 1: Add LSP param structs**

Add after the existing `AstGrepReplaceParams` struct:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LspDiagnosticsParams {
    /// File path to get diagnostics for
    pub file_path: String,
    /// Language of the file (ts, rust, py, go, java)
    pub language: String,
    /// Workspace root (default: ".")
    pub workspace_root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LspDocumentSymbolsParams {
    /// File path to get symbols for
    pub file_path: String,
    /// Language of the file
    pub language: String,
    /// Workspace root (default: ".")
    pub workspace_root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LspWorkspaceSymbolsParams {
    /// Symbol query string
    pub query: String,
    /// Language of the workspace
    pub language: String,
    /// Workspace root (default: ".")
    pub workspace_root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LspHoverParams {
    /// File path
    pub file_path: String,
    /// Line number (0-based)
    pub line: u32,
    /// Column number (0-based)
    pub character: u32,
    /// Language of the file
    pub language: String,
    /// Workspace root (default: ".")
    pub workspace_root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LspFindReferencesParams {
    /// File path containing the symbol
    pub file_path: String,
    /// Line number (0-based)
    pub line: u32,
    /// Column number (0-based)
    pub character: u32,
    /// Language of the file
    pub language: String,
    /// Workspace root (default: ".")
    pub workspace_root: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LspServersParams {}
```

- [ ] **Step 2: Add a helper to build file URI**

Add a helper function before the tool impl block:

```rust
fn file_uri(workspace: &str, file_path: &str) -> String {
    let full = std::path::Path::new(workspace).join(file_path);
    let abs = full
        .canonicalize()
        .unwrap_or(full);
    format!("file://{}", abs.display())
}
```

- [ ] **Step 3: Add lsp_diagnostics tool**

Add inside the `#[rmcp::tool(tool_box)] impl CodeIntelMcpServer` block:

```rust
    #[tool(description = "Get diagnostics (errors/warnings) for a file from a running LSP server")]
    async fn lsp_diagnostics(&self, #[tool(aggr)] params: LspDiagnosticsParams) -> String {
        let workspace = params.workspace_root.as_deref().unwrap_or(".");
        let client = match self
            .lsp_manager
            .get_client(&params.language, std::path::Path::new(workspace))
            .await
        {
            Ok(c) => c,
            Err(e) => return format!("LSP error: {e}"),
        };

        let uri = file_uri(workspace, &params.file_path);

        // Open the document to trigger diagnostics
        let contents = match tokio::fs::read_to_string(
            std::path::Path::new(workspace).join(&params.file_path),
        )
        .await
        {
            Ok(c) => c,
            Err(e) => return format!("Failed to read file: {e}"),
        };

        let open_params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": params.language,
                "version": 1,
                "text": contents,
            }
        });

        if let Err(e) = client.notify("textDocument/didOpen", open_params).await {
            return format!("LSP error: {e}");
        }

        // Give the server a moment to compute diagnostics
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Use textDocument/diagnostic if supported, otherwise return status
        let diag_params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        match client
            .request("textDocument/diagnostic", diag_params)
            .await
        {
            Ok(response) => {
                if let Some(result) = response.get("result") {
                    serde_json::to_string_pretty(result).unwrap_or_default()
                } else if let Some(error) = response.get("error") {
                    format!("LSP returned error: {error}")
                } else {
                    "No diagnostics returned".to_string()
                }
            }
            Err(e) => format!("LSP request failed: {e}"),
        }
    }
```

- [ ] **Step 4: Add lsp_document_symbols tool**

```rust
    #[tool(description = "List symbols in a file via LSP documentSymbol")]
    async fn lsp_document_symbols(
        &self,
        #[tool(aggr)] params: LspDocumentSymbolsParams,
    ) -> String {
        let workspace = params.workspace_root.as_deref().unwrap_or(".");
        let client = match self
            .lsp_manager
            .get_client(&params.language, std::path::Path::new(workspace))
            .await
        {
            Ok(c) => c,
            Err(e) => return format!("LSP error: {e}"),
        };

        let uri = file_uri(workspace, &params.file_path);
        let req_params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        match client
            .request("textDocument/documentSymbol", req_params)
            .await
        {
            Ok(response) => {
                if let Some(result) = response.get("result") {
                    serde_json::to_string_pretty(result).unwrap_or_default()
                } else if let Some(error) = response.get("error") {
                    format!("LSP error: {error}")
                } else {
                    "No symbols found".to_string()
                }
            }
            Err(e) => format!("LSP request failed: {e}"),
        }
    }
```

- [ ] **Step 5: Add lsp_workspace_symbols tool**

```rust
    #[tool(description = "Search symbols across the workspace via LSP")]
    async fn lsp_workspace_symbols(
        &self,
        #[tool(aggr)] params: LspWorkspaceSymbolsParams,
    ) -> String {
        let workspace = params.workspace_root.as_deref().unwrap_or(".");
        let client = match self
            .lsp_manager
            .get_client(&params.language, std::path::Path::new(workspace))
            .await
        {
            Ok(c) => c,
            Err(e) => return format!("LSP error: {e}"),
        };

        let req_params = serde_json::json!({
            "query": params.query
        });

        match client.request("workspace/symbol", req_params).await {
            Ok(response) => {
                if let Some(result) = response.get("result") {
                    serde_json::to_string_pretty(result).unwrap_or_default()
                } else if let Some(error) = response.get("error") {
                    format!("LSP error: {error}")
                } else {
                    "No symbols found".to_string()
                }
            }
            Err(e) => format!("LSP request failed: {e}"),
        }
    }
```

- [ ] **Step 6: Add lsp_hover tool**

```rust
    #[tool(description = "Get hover info (type, docs) for a position via LSP")]
    async fn lsp_hover(&self, #[tool(aggr)] params: LspHoverParams) -> String {
        let workspace = params.workspace_root.as_deref().unwrap_or(".");
        let client = match self
            .lsp_manager
            .get_client(&params.language, std::path::Path::new(workspace))
            .await
        {
            Ok(c) => c,
            Err(e) => return format!("LSP error: {e}"),
        };

        let uri = file_uri(workspace, &params.file_path);
        let req_params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": {
                "line": params.line,
                "character": params.character
            }
        });

        match client.request("textDocument/hover", req_params).await {
            Ok(response) => {
                if let Some(result) = response.get("result") {
                    if result.is_null() {
                        "No hover info at this position".to_string()
                    } else {
                        serde_json::to_string_pretty(result).unwrap_or_default()
                    }
                } else if let Some(error) = response.get("error") {
                    format!("LSP error: {error}")
                } else {
                    "No hover info available".to_string()
                }
            }
            Err(e) => format!("LSP request failed: {e}"),
        }
    }
```

- [ ] **Step 7: Add lsp_find_references tool**

```rust
    #[tool(description = "Find all references to a symbol at a position via LSP")]
    async fn lsp_find_references(
        &self,
        #[tool(aggr)] params: LspFindReferencesParams,
    ) -> String {
        let workspace = params.workspace_root.as_deref().unwrap_or(".");
        let client = match self
            .lsp_manager
            .get_client(&params.language, std::path::Path::new(workspace))
            .await
        {
            Ok(c) => c,
            Err(e) => return format!("LSP error: {e}"),
        };

        let uri = file_uri(workspace, &params.file_path);
        let req_params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": {
                "line": params.line,
                "character": params.character
            },
            "context": {
                "includeDeclaration": true
            }
        });

        match client
            .request("textDocument/references", req_params)
            .await
        {
            Ok(response) => {
                if let Some(result) = response.get("result") {
                    if result.is_null() || (result.is_array() && result.as_array().unwrap().is_empty()) {
                        "No references found".to_string()
                    } else {
                        serde_json::to_string_pretty(result).unwrap_or_default()
                    }
                } else if let Some(error) = response.get("error") {
                    format!("LSP error: {error}")
                } else {
                    "No references found".to_string()
                }
            }
            Err(e) => format!("LSP request failed: {e}"),
        }
    }
```

- [ ] **Step 8: Add lsp_servers tool**

```rust
    #[tool(description = "List running LSP servers and their status")]
    async fn lsp_servers(&self, #[tool(aggr)] _params: LspServersParams) -> String {
        let servers = self.lsp_manager.list_servers().await;
        if servers.is_empty() {
            return "No LSP servers running. Servers start on first use.".to_string();
        }
        let entries: Vec<serde_json::Value> = servers
            .into_iter()
            .map(|(lang, cmd)| {
                serde_json::json!({
                    "language": lang,
                    "command": cmd,
                    "status": "running"
                })
            })
            .collect();
        serde_json::to_string_pretty(&entries).unwrap_or_default()
    }
```

- [ ] **Step 9: Verify compilation**

Run: `cargo build -p omx-mcp-code-intel`
Expected: success

- [ ] **Step 10: Commit**

```bash
git add crates/omx-mcp-code-intel/src/main.rs
git commit -m "feat(omx-mcp-code-intel): add 6 LSP tools — diagnostics, symbols, hover, references, servers"
```

---

## Task 8: Final verification — build all MCP crates and run clippy

**Files:** none (verification only)

- [ ] **Step 1: Build all MCP crates**

Run: `cargo build -p omx-mcp-state -p omx-mcp-memory -p omx-mcp-code-intel -p omx-mcp-trace -p omx-mcp-team`
Expected: all compile successfully

- [ ] **Step 2: Run clippy on all MCP crates**

Run: `cargo clippy -p omx-mcp-state -p omx-mcp-memory -p omx-mcp-code-intel -p omx-mcp-trace -p omx-mcp-team -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Fix any clippy warnings**

Apply fixes as needed.

- [ ] **Step 4: Run full workspace build**

Run: `cargo build`
Expected: full workspace compiles

- [ ] **Step 5: Run full workspace tests**

Run: `cargo test`
Expected: all existing tests pass

- [ ] **Step 6: Commit any clippy fixes**

```bash
git add -A
git commit -m "style: fix clippy warnings in Phase 7 MCP crates"
```
