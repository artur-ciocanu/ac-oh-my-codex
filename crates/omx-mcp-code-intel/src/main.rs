use std::sync::Arc;

use rmcp::{tool, ServerHandler, ServiceExt};
use serde::Deserialize;

mod lsp;
use lsp::LspClientManager;

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

fn file_uri(workspace: &str, file_path: &str) -> String {
    let full = std::path::Path::new(workspace).join(file_path);
    let abs = full.canonicalize().unwrap_or(full);
    format!("file://{}", abs.display())
}

#[derive(Clone)]
struct CodeIntelMcpServer {
    lsp_manager: Arc<LspClientManager>,
}

impl std::fmt::Debug for CodeIntelMcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeIntelMcpServer").finish()
    }
}

#[rmcp::tool(tool_box)]
impl CodeIntelMcpServer {
    #[tool(description = "Get TypeScript/JavaScript diagnostics for workspace files")]
    async fn diagnostics_typescript(&self, #[tool(aggr)] params: DiagnosticsParams) -> String {
        let workspace = params.workspace_root.as_deref().unwrap_or(".");

        let args = vec!["tsc", "--noEmit", "--pretty", "false"];

        let patterns = params.file_patterns.unwrap_or_default();

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

        let result = if !patterns.is_empty() {
            stdout
                .lines()
                .filter(|line| patterns.iter().any(|pat| line.contains(pat.as_str())))
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
        let workspace = params.workspace_root.as_deref().unwrap_or(".");

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
        cmd.arg("--json").arg("--max-count=50").arg(&params.pattern);

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

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let diag_params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        match client.request("textDocument/diagnostic", diag_params).await {
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

    #[tool(description = "List symbols in a file via LSP documentSymbol")]
    async fn lsp_document_symbols(&self, #[tool(aggr)] params: LspDocumentSymbolsParams) -> String {
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

    #[tool(description = "Find all references to a symbol at a position via LSP")]
    async fn lsp_find_references(&self, #[tool(aggr)] params: LspFindReferencesParams) -> String {
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

        match client.request("textDocument/references", req_params).await {
            Ok(response) => {
                if let Some(result) = response.get("result") {
                    if result.is_null()
                        || (result.is_array() && result.as_array().unwrap().is_empty())
                    {
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
}

#[rmcp::tool(tool_box)]
impl ServerHandler for CodeIntelMcpServer {}

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
