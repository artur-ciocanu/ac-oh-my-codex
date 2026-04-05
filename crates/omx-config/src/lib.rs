use std::collections::HashMap;
use std::path::{Path, PathBuf};

use omx_types::OmxError;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Config structs (spec section 4.2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmxConfig {
    pub codex_home: PathBuf,
    pub models: ModelConfig,
    pub notifications: NotificationConfig,
    pub team: TeamDefaults,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub frontier: String,
    pub standard: String,
    pub spark: String,
    pub per_mode: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub discord: Option<DiscordConfig>,
    pub slack: Option<SlackConfig>,
    pub telegram: Option<TelegramConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDefaults {
    pub default_workers: u8,
    pub default_model: String,
    pub worktree_mode: String,
}

// ---------------------------------------------------------------------------
// Default impls
// ---------------------------------------------------------------------------

impl Default for OmxConfig {
    fn default() -> Self {
        Self {
            codex_home: default_codex_home(),
            models: ModelConfig::default(),
            notifications: NotificationConfig::default(),
            team: TeamDefaults::default(),
            env: HashMap::new(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            frontier: "o3".into(),
            standard: "o4-mini".into(),
            spark: "o4-mini".into(),
            per_mode: HashMap::new(),
        }
    }
}

impl Default for TeamDefaults {
    fn default() -> Self {
        Self {
            default_workers: 3,
            default_model: "o4-mini".into(),
            worktree_mode: "branch".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Internal deserialization structs for TOML config file
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TomlConfigFile {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub model_reasoning_effort: Option<String>,
    #[serde(default)]
    pub models: Option<TomlModelSection>,
    #[serde(default)]
    pub notifications: Option<NotificationConfig>,
    #[serde(default)]
    pub team: Option<TomlTeamSection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TomlModelSection {
    pub frontier: Option<String>,
    pub standard: Option<String>,
    pub spark: Option<String>,
    #[serde(default)]
    pub per_mode: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TomlTeamSection {
    pub default_workers: Option<u8>,
    pub default_model: Option<String>,
    pub worktree_mode: Option<String>,
}

// ---------------------------------------------------------------------------
// Internal deserialization struct for JSON config file
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct JsonConfigFile {
    #[serde(default)]
    pub codex_home: Option<String>,
    #[serde(default)]
    pub models: Option<TomlModelSection>,
    #[serde(default)]
    pub notifications: Option<NotificationConfig>,
    #[serde(default)]
    pub team: Option<TomlTeamSection>,
}

// ---------------------------------------------------------------------------
// ConfigLoader trait
// Resolution: CLI arg > env var > .omx-config.json > config.toml > defaults
// ---------------------------------------------------------------------------

pub trait ConfigLoader {
    fn load(codex_home: &Path, env: &HashMap<String, String>) -> Result<OmxConfig, OmxError>;
}

// ---------------------------------------------------------------------------
// Default implementation
// ---------------------------------------------------------------------------

pub struct DefaultConfigLoader;

impl ConfigLoader for DefaultConfigLoader {
    fn load(codex_home: &Path, env: &HashMap<String, String>) -> Result<OmxConfig, OmxError> {
        let mut config = OmxConfig {
            codex_home: codex_home.to_path_buf(),
            ..OmxConfig::default()
        };

        // Layer 1: config.toml (lowest precedence file source)
        let toml_path = codex_home.join("config.toml");
        if toml_path.exists() {
            let contents = std::fs::read_to_string(&toml_path)
                .map_err(|e| OmxError::Config(format!("failed to read {}: {e}", toml_path.display())))?;
            let toml_config: TomlConfigFile = toml::from_str(&contents)
                .map_err(|e| OmxError::Config(format!("invalid TOML in {}: {e}", toml_path.display())))?;
            apply_toml(&mut config, &toml_config);
        }

        // Layer 2: .omx-config.json (overrides TOML)
        let json_path = codex_home.join(".omx-config.json");
        if json_path.exists() {
            let contents = std::fs::read_to_string(&json_path)
                .map_err(|e| OmxError::Config(format!("failed to read {}: {e}", json_path.display())))?;
            let json_config: JsonConfigFile = serde_json::from_str(&contents)
                .map_err(|e| OmxError::Config(format!("invalid JSON in {}: {e}", json_path.display())))?;
            apply_json(&mut config, &json_config);
        }

        // Layer 3: environment variables (overrides file sources)
        apply_env(&mut config, env);

        // Store the resolved environment for downstream consumers
        config.env = env.clone();

        Ok(config)
    }
}

fn apply_toml(config: &mut OmxConfig, toml: &TomlConfigFile) {
    if let Some(ref models) = toml.models {
        if let Some(ref v) = models.frontier {
            config.models.frontier = v.clone();
        }
        if let Some(ref v) = models.standard {
            config.models.standard = v.clone();
        }
        if let Some(ref v) = models.spark {
            config.models.spark = v.clone();
        }
        for (k, v) in &models.per_mode {
            config.models.per_mode.insert(k.clone(), v.clone());
        }
    }
    if let Some(ref model) = toml.model {
        config.models.frontier = model.clone();
    }
    if let Some(ref notifications) = toml.notifications {
        config.notifications = notifications.clone();
    }
    if let Some(ref team) = toml.team {
        if let Some(v) = team.default_workers {
            config.team.default_workers = v;
        }
        if let Some(ref v) = team.default_model {
            config.team.default_model = v.clone();
        }
        if let Some(ref v) = team.worktree_mode {
            config.team.worktree_mode = v.clone();
        }
    }
}

fn apply_json(config: &mut OmxConfig, json: &JsonConfigFile) {
    if let Some(ref home) = json.codex_home {
        config.codex_home = PathBuf::from(home);
    }
    if let Some(ref models) = json.models {
        if let Some(ref v) = models.frontier {
            config.models.frontier = v.clone();
        }
        if let Some(ref v) = models.standard {
            config.models.standard = v.clone();
        }
        if let Some(ref v) = models.spark {
            config.models.spark = v.clone();
        }
        for (k, v) in &models.per_mode {
            config.models.per_mode.insert(k.clone(), v.clone());
        }
    }
    if let Some(ref notifications) = json.notifications {
        config.notifications = notifications.clone();
    }
    if let Some(ref team) = json.team {
        if let Some(v) = team.default_workers {
            config.team.default_workers = v;
        }
        if let Some(ref v) = team.default_model {
            config.team.default_model = v.clone();
        }
        if let Some(ref v) = team.worktree_mode {
            config.team.worktree_mode = v.clone();
        }
    }
}

fn apply_env(config: &mut OmxConfig, env: &HashMap<String, String>) {
    if let Some(v) = env.get("OMX_MODEL_FRONTIER") {
        config.models.frontier = v.clone();
    }
    if let Some(v) = env.get("OMX_MODEL_STANDARD") {
        config.models.standard = v.clone();
    }
    if let Some(v) = env.get("OMX_MODEL_SPARK") {
        config.models.spark = v.clone();
    }
    if let Some(v) = env.get("OMX_TEAM_WORKERS") {
        if let Ok(n) = v.parse::<u8>() {
            config.team.default_workers = n;
        }
    }
    if let Some(v) = env.get("OMX_TEAM_MODEL") {
        config.team.default_model = v.clone();
    }
    if let Some(v) = env.get("CODEX_HOME") {
        config.codex_home = PathBuf::from(v);
    }
}

/// Return the default codex home directory (~/.codex).
pub fn default_codex_home() -> PathBuf {
    dirs_next().join(".codex")
}

fn dirs_next() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_codex_home_uses_home_env() {
        let home = default_codex_home();
        assert!(home.to_str().unwrap().ends_with(".codex"));
    }

    #[test]
    fn default_config_has_sensible_values() {
        let config = OmxConfig::default();
        assert_eq!(config.models.frontier, "o3");
        assert_eq!(config.models.standard, "o4-mini");
        assert_eq!(config.team.default_workers, 3);
        assert_eq!(config.team.worktree_mode, "branch");
    }

    #[test]
    fn load_with_no_files_returns_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let env = HashMap::new();
        let config = DefaultConfigLoader::load(tmp.path(), &env).unwrap();
        assert_eq!(config.models.frontier, "o3");
        assert_eq!(config.team.default_workers, 3);
    }

    #[test]
    fn load_reads_config_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let toml_content = r#"
[models]
frontier = "claude-sonnet"
standard = "claude-haiku"

[team]
default_workers = 5
"#;
        fs::write(tmp.path().join("config.toml"), toml_content).unwrap();

        let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
        assert_eq!(config.models.frontier, "claude-sonnet");
        assert_eq!(config.models.standard, "claude-haiku");
        assert_eq!(config.models.spark, "o4-mini");
        assert_eq!(config.team.default_workers, 5);
    }

    #[test]
    fn load_reads_json_config() {
        let tmp = tempfile::tempdir().unwrap();
        let json_content = r#"{
            "models": { "frontier": "gpt-5" },
            "team": { "default_model": "gpt-4" }
        }"#;
        fs::write(tmp.path().join(".omx-config.json"), json_content).unwrap();

        let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
        assert_eq!(config.models.frontier, "gpt-5");
        assert_eq!(config.team.default_model, "gpt-4");
    }

    #[test]
    fn json_overrides_toml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("config.toml"),
            "[models]\nfrontier = \"from-toml\"\n",
        ).unwrap();
        fs::write(
            tmp.path().join(".omx-config.json"),
            r#"{"models": {"frontier": "from-json"}}"#,
        ).unwrap();

        let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
        assert_eq!(config.models.frontier, "from-json");
    }

    #[test]
    fn env_overrides_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("config.toml"),
            "[models]\nfrontier = \"from-toml\"\n",
        ).unwrap();

        let mut env = HashMap::new();
        env.insert("OMX_MODEL_FRONTIER".into(), "from-env".into());

        let config = DefaultConfigLoader::load(tmp.path(), &env).unwrap();
        assert_eq!(config.models.frontier, "from-env");
    }

    #[test]
    fn env_workers_parsed_as_u8() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env = HashMap::new();
        env.insert("OMX_TEAM_WORKERS".into(), "7".into());

        let config = DefaultConfigLoader::load(tmp.path(), &env).unwrap();
        assert_eq!(config.team.default_workers, 7);
    }

    #[test]
    fn invalid_toml_returns_config_error() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("config.toml"), "not valid [[[ toml").unwrap();

        let result = DefaultConfigLoader::load(tmp.path(), &HashMap::new());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid TOML"));
    }

    #[test]
    fn invalid_json_returns_config_error() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".omx-config.json"), "not json").unwrap();

        let result = DefaultConfigLoader::load(tmp.path(), &HashMap::new());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid JSON"));
    }

    #[test]
    fn toml_top_level_model_sets_frontier() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("config.toml"), "model = \"o3\"\n").unwrap();

        let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
        assert_eq!(config.models.frontier, "o3");
    }

    #[test]
    fn notification_config_roundtrips_through_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let toml_content = r#"
[notifications.discord]
webhook_url = "https://discord.com/api/webhooks/123/abc"

[notifications.slack]
webhook_url = "https://hooks.slack.com/services/T/B/X"
"#;
        fs::write(tmp.path().join("config.toml"), toml_content).unwrap();

        let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
        assert_eq!(
            config.notifications.discord.as_ref().unwrap().webhook_url,
            "https://discord.com/api/webhooks/123/abc"
        );
        assert_eq!(
            config.notifications.slack.as_ref().unwrap().webhook_url,
            "https://hooks.slack.com/services/T/B/X"
        );
        assert!(config.notifications.telegram.is_none());
    }

    #[test]
    fn codex_home_env_overrides_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut env = HashMap::new();
        env.insert("CODEX_HOME".into(), "/custom/home".into());

        let config = DefaultConfigLoader::load(tmp.path(), &env).unwrap();
        assert_eq!(config.codex_home, PathBuf::from("/custom/home"));
    }
}
