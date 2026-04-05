use std::path::{Path, PathBuf};
use std::sync::Arc;

use omx_state::FileStateStore;
use rmcp::{tool, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};

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

#[derive(Clone)]
struct TraceMcpServer {
    store: Arc<FileStateStore>,
}

impl std::fmt::Debug for TraceMcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceMcpServer").finish()
    }
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
}

#[rmcp::tool(tool_box)]
impl TraceMcpServer {
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
