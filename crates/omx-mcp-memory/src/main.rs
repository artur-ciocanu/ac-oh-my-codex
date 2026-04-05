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

#[derive(Clone)]
struct MemoryMcpServer {
    store: Arc<FileStateStore>,
}

impl std::fmt::Debug for MemoryMcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryMcpServer").finish()
    }
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
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}Z", dur.as_secs())
    }

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
                            let ts: u64 = entry
                                .updated_at
                                .trim_end_matches('Z')
                                .parse()
                                .unwrap_or(u64::MAX);
                            if ts < cutoff_secs
                                && self.store.delete(relative).await.is_ok()
                            {
                                pruned += 1;
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
