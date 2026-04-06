#[cfg(test)]
mod tests {
    use crate::{mcp_cmd, TestConfig};
    use std::io::Write;

    /// Format a JSON-RPC request as a newline-terminated JSON string (no Content-Length framing).
    /// The rmcp transport reads raw JSON lines from stdin.
    fn jsonrpc_line(id: u64, method: &str, params: serde_json::Value) -> String {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        format!("{}\n", serde_json::to_string(&msg).unwrap())
    }

    /// Format a JSON-RPC notification (no id) as a newline-terminated JSON string.
    fn jsonrpc_notification_line(method: &str) -> String {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        format!("{}\n", serde_json::to_string(&msg).unwrap())
    }

    /// Send JSON-RPC messages (as raw JSON lines) to an MCP server process and read stdout.
    fn mcp_roundtrip(server: &str, messages: &[String]) -> String {
        let config = TestConfig::new();
        let mut child = mcp_cmd(server, &config)
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {server}: {e}"));

        if let Some(ref mut stdin) = child.stdin {
            for msg in messages {
                stdin.write_all(msg.as_bytes()).unwrap();
            }
        }
        drop(child.stdin.take());

        let output = child
            .wait_with_output()
            .expect("failed to wait for MCP server");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    // Smoke tests — send initialize, verify response

    #[test]
    fn mcp_state_smoke() {
        let init = jsonrpc_line(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }),
        );
        let response = mcp_roundtrip("omx-mcp-state", &[init]);
        assert!(
            response.contains("capabilities") || response.contains("result"),
            "state server should respond to initialize, got: {response}"
        );
    }

    #[test]
    fn mcp_memory_smoke() {
        let init = jsonrpc_line(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }),
        );
        let response = mcp_roundtrip("omx-mcp-memory", &[init]);
        assert!(
            response.contains("capabilities") || response.contains("result"),
            "memory server should respond to initialize, got: {response}"
        );
    }

    #[test]
    fn mcp_code_intel_smoke() {
        let init = jsonrpc_line(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }),
        );
        let response = mcp_roundtrip("omx-mcp-code-intel", &[init]);
        assert!(
            response.contains("capabilities") || response.contains("result"),
            "code-intel server should respond to initialize, got: {response}"
        );
    }

    #[test]
    fn mcp_trace_smoke() {
        let init = jsonrpc_line(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }),
        );
        let response = mcp_roundtrip("omx-mcp-trace", &[init]);
        assert!(
            response.contains("capabilities") || response.contains("result"),
            "trace server should respond to initialize, got: {response}"
        );
    }

    #[test]
    fn mcp_team_smoke() {
        let init = jsonrpc_line(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }),
        );
        let response = mcp_roundtrip("omx-mcp-team", &[init]);
        assert!(
            response.contains("capabilities") || response.contains("result"),
            "team server should respond to initialize, got: {response}"
        );
    }

    // Tool inventory tests

    fn init_and_list_tools(server: &str) -> String {
        let init = jsonrpc_line(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }),
        );
        let initialized = jsonrpc_notification_line("notifications/initialized");
        let list = jsonrpc_line(2, "tools/list", serde_json::json!({}));
        mcp_roundtrip(server, &[init, initialized, list])
    }

    #[test]
    fn mcp_state_tool_inventory() {
        let response = init_and_list_tools("omx-mcp-state");
        let expected_tools = [
            "state_read",
            "state_write",
            "state_clear",
            "state_list_active",
            "state_get_status",
            "state_delete",
            "state_list",
        ];
        for tool in expected_tools {
            assert!(
                response.contains(tool),
                "state server should register '{tool}', response: {response}"
            );
        }
    }

    #[test]
    fn mcp_memory_tool_inventory() {
        let response = init_and_list_tools("omx-mcp-memory");
        let expected_tools = [
            "project_memory_read",
            "project_memory_write",
            "project_memory_prune",
            "notepad_add_note",
            "notepad_add_directive",
            "notepad_read",
            "notepad_write_priority",
            "notepad_write_working",
            "notepad_stats",
        ];
        for tool in expected_tools {
            assert!(
                response.contains(tool),
                "memory server should register '{tool}', response: {response}"
            );
        }
    }

    #[test]
    fn mcp_code_intel_tool_inventory() {
        let response = init_and_list_tools("omx-mcp-code-intel");
        let expected_tools = [
            "diagnostics_typescript",
            "ast_pattern_search",
            "ast_grep_replace",
            "lsp_diagnostics",
            "lsp_document_symbols",
            "lsp_workspace_symbols",
            "lsp_hover",
            "lsp_find_references",
            "lsp_servers",
        ];
        for tool in expected_tools {
            assert!(
                response.contains(tool),
                "code-intel server should register '{tool}', response: {response}"
            );
        }
    }

    #[test]
    fn mcp_trace_tool_inventory() {
        let response = init_and_list_tools("omx-mcp-trace");
        let expected_tools = ["trace_timeline", "trace_summary"];
        for tool in expected_tools {
            assert!(
                response.contains(tool),
                "trace server should register '{tool}', response: {response}"
            );
        }
    }

    #[test]
    fn mcp_team_tool_inventory() {
        let response = init_and_list_tools("omx-mcp-team");
        let expected_tools = [
            "omx_run_team_start",
            "omx_run_team_status",
            "omx_run_team_wait",
            "omx_run_team_cleanup",
            "omx_run_team_nudge",
        ];
        for tool in expected_tools {
            assert!(
                response.contains(tool),
                "team server should register '{tool}', response: {response}"
            );
        }
    }
}
