use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, oneshot};

/// A lightweight LSP client that communicates via JSON-RPC over stdio.
pub struct LspClient {
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<serde_json::Value>>>>,
    next_id: AtomicI64,
    _child: Arc<Mutex<Child>>,
    #[allow(dead_code)]
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
        let root_uri = format!("file://{}", workspace_root.to_str().unwrap_or("."));
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {},
        });

        let response = client.request("initialize", init_params).await?;
        if response.get("error").is_some() {
            return Err(format!("LSP initialize failed: {}", response["error"]));
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
            stdin
                .flush()
                .await
                .map_err(|e| format!("Flush failed: {e}"))?;
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
        stdin
            .flush()
            .await
            .map_err(|e| format!("Flush failed: {e}"))?;
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
