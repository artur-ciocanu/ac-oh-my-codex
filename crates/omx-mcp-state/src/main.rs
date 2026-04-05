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

#[derive(Clone)]
struct StateMcpServer {
    store: Arc<FileStateStore>,
}

impl std::fmt::Debug for StateMcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateMcpServer").finish_non_exhaustive()
    }
}

impl StateMcpServer {
    fn new(state_root: PathBuf) -> Self {
        Self {
            store: Arc::new(FileStateStore::new(state_root)),
        }
    }

    fn state_path(mode: &str, key: &str) -> PathBuf {
        Path::new("state").join(mode).join(format!("{key}.json"))
    }

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
