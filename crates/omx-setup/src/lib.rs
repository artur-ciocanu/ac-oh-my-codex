use omx_config::OmxConfig;
use omx_types::OmxError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupScope {
    User,
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
}

pub trait SetupGenerator: Send + Sync {
    fn generate_config_toml(
        &self,
        config: &OmxConfig,
        scope: SetupScope,
    ) -> Result<String, OmxError>;

    fn generate_agents_md(&self, config: &OmxConfig) -> Result<String, OmxError>;

    fn generate_agent_tomls(
        &self,
        agents: &[AgentDefinition],
    ) -> Result<Vec<(String, String)>, OmxError>;

    fn sync_mcp_servers(&self, config: &OmxConfig, scope: SetupScope) -> Result<(), OmxError>;

    fn copy_prompts(&self, scope: SetupScope) -> Result<(), OmxError>;

    fn copy_skills(&self, scope: SetupScope) -> Result<(), OmxError>;
}

pub struct DefaultSetupGenerator;

impl SetupGenerator for DefaultSetupGenerator {
    fn generate_config_toml(
        &self,
        config: &OmxConfig,
        _scope: SetupScope,
    ) -> Result<String, OmxError> {
        let mut out = String::new();
        out.push_str("# OMX:START — managed by omx setup, do not edit\n\n");
        out.push_str(&format!("model = \"{}\"\n\n", config.models.frontier));
        let servers = [
            ("omx_state", "omx-mcp-state"),
            ("omx_memory", "omx-mcp-memory"),
            ("omx_code_intel", "omx-mcp-code-intel"),
            ("omx_trace", "omx-mcp-trace"),
            ("omx_team", "omx-mcp-team"),
        ];
        for (name, binary) in servers {
            out.push_str(&format!("[mcp_servers.{name}]\n"));
            out.push_str(&format!("command = \"{binary}\"\n"));
            out.push_str("args = []\n");
            out.push_str("enabled = true\n");
            out.push_str("startup_timeout_sec = 5\n\n");
        }
        out.push_str("# OMX:END\n");
        Ok(out)
    }

    fn generate_agents_md(&self, _config: &OmxConfig) -> Result<String, OmxError> {
        todo!("Phase 4: generate AGENTS.md with all agent definitions")
    }

    fn generate_agent_tomls(
        &self,
        _agents: &[AgentDefinition],
    ) -> Result<Vec<(String, String)>, OmxError> {
        todo!("Phase 4: generate per-agent .toml files")
    }

    fn sync_mcp_servers(&self, _config: &OmxConfig, _scope: SetupScope) -> Result<(), OmxError> {
        todo!("Phase 4: register Rust MCP server binaries in config.toml")
    }

    fn copy_prompts(&self, _scope: SetupScope) -> Result<(), OmxError> {
        todo!("Phase 4: write embedded prompts to disk")
    }

    fn copy_skills(&self, _scope: SetupScope) -> Result<(), OmxError> {
        todo!("Phase 4: write embedded skills to disk")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_scope_serde_roundtrip() {
        let scope = SetupScope::Project;
        let json = serde_json::to_string(&scope).unwrap();
        let parsed: SetupScope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, scope);
    }

    #[test]
    fn generate_config_toml_has_markers_and_mcp_entries() {
        let gen = DefaultSetupGenerator;
        let config = OmxConfig::default();
        let result = gen.generate_config_toml(&config, SetupScope::User).unwrap();
        assert!(result.contains("# OMX:START"), "must have start marker");
        assert!(result.contains("# OMX:END"), "must have end marker");
        assert!(
            result.contains("[mcp_servers.omx_state]"),
            "must register omx-mcp-state"
        );
        assert!(
            result.contains("[mcp_servers.omx_memory]"),
            "must register omx-mcp-memory"
        );
        assert!(
            result.contains("[mcp_servers.omx_code_intel]"),
            "must register omx-mcp-code-intel"
        );
        assert!(
            result.contains("[mcp_servers.omx_trace]"),
            "must register omx-mcp-trace"
        );
        assert!(
            result.contains("[mcp_servers.omx_team]"),
            "must register omx-mcp-team"
        );
        assert!(
            result.contains("command = \"omx-mcp-state\""),
            "must use binary name"
        );
    }

    #[ignore]
    #[test]
    fn generate_agents_md_placeholder() {
        // Phase 4: test AGENTS.md generation
    }
}
