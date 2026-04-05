# Pure Rust Migration — Phase 1: Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all `todo!()` stubs in the four foundation crates (`omx-types`, `omx-config`, `omx-state`, `omx-mux`) with working implementations so that downstream crates (MCP servers, team runtime, CLI) can build on real functionality.

**Architecture:** Bottom-up implementation of the foundation layer. `omx-types` is already mostly complete from Phase 0 — just needs `Display` impls and phase ordering. `omx-config` implements the multi-source config precedence chain (CLI > env > JSON > TOML > defaults). `omx-state` implements atomic file I/O with fs2 locking. `omx-mux` is refactored to align its existing `MuxAdapter` trait with the spec's method-based trait (adding `create_window`, `kill_window`, `send_keys`) and adding `omx-types` integration.

**Tech Stack:** Rust 2021 edition, tokio 1.x, serde/serde_json, toml 0.8, fs2 0.4, thiserror 2.x, tracing, insta (snapshot tests)

**Spec:** `docs/superpowers/specs/2026-04-04-pure-rust-migration-design.md`

**Phase 0 baseline:** All crates exist with skeleton types, traits, and `todo!()` bodies. The workspace compiles.

**Milestone:** All four foundation crates have real implementations with passing tests. No `todo!()` remains in these crates.

---

## File Structure

### Modified files

```
crates/omx-types/src/lib.rs              # Add Display impls, phase ordering, Hash for enums
crates/omx-config/Cargo.toml             # Add dirs (home dir resolution)
crates/omx-config/src/lib.rs             # Implement DefaultConfigLoader, defaults, TOML/JSON parsing
crates/omx-state/src/lib.rs              # Implement FileStateStore with atomic I/O + fs2 locking
crates/omx-mux/Cargo.toml                # Add omx-types dependency
crates/omx-mux/src/types.rs              # Add create_window, kill_window, send_keys to MuxAdapter trait
crates/omx-mux/src/tmux.rs               # Implement new MuxAdapter methods
crates/omx-mux/src/lib.rs                # Re-export new items, update contract summary
```

---

## Task 1: Enhance omx-types with Display impls and phase ordering

**Files:**
- Modify: `crates/omx-types/src/lib.rs`

This task adds `Display` implementations for key enums (used in logging and HUD), implements phase ordering for `TeamPhase`, and adds missing trait derives that downstream crates will need.

- [ ] **Step 1: Add Display impl for CliProvider**

Add after the `from_label` method's closing brace:

```rust
impl std::fmt::Display for CliProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codex => write!(f, "codex"),
            Self::Claude => write!(f, "claude"),
        }
    }
}
```

- [ ] **Step 2: Add Display impls for TaskStatus, DispatchStatus, TeamPhase**

Add after each enum definition:

```rust
impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Blocked => write!(f, "blocked"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::fmt::Display for DispatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Notified => write!(f, "notified"),
            Self::Delivered => write!(f, "delivered"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::fmt::Display for TeamPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan => write!(f, "plan"),
            Self::Prd => write!(f, "prd"),
            Self::Exec => write!(f, "exec"),
            Self::Verify => write!(f, "verify"),
            Self::Fix => write!(f, "fix"),
        }
    }
}
```

- [ ] **Step 3: Add Display impl for HookEventName**

```rust
impl std::fmt::Display for HookEventName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::SessionIdle => "session_idle",
            Self::TurnComplete => "turn_complete",
            Self::Blocked => "blocked",
            Self::Finished => "finished",
            Self::Failed => "failed",
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::PrCreated => "pr_created",
            Self::TestStarted => "test_started",
            Self::TestFinished => "test_finished",
            Self::TestFailed => "test_failed",
        };
        write!(f, "{name}")
    }
}
```

- [ ] **Step 4: Implement phase ordering for TeamPhase**

Add after the `Display` impl for `TeamPhase`:

```rust
impl TeamPhase {
    /// Return the numeric index of this phase in the lifecycle (0-based).
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Plan => 0,
            Self::Prd => 1,
            Self::Exec => 2,
            Self::Verify => 3,
            Self::Fix => 4,
        }
    }

    /// Return the next phase in the lifecycle, or `None` if this is the last.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Plan => Some(Self::Prd),
            Self::Prd => Some(Self::Exec),
            Self::Exec => Some(Self::Verify),
            Self::Verify => Some(Self::Fix),
            Self::Fix => None,
        }
    }

    /// All phases in lifecycle order.
    pub const ALL: &'static [TeamPhase] = &[
        Self::Plan,
        Self::Prd,
        Self::Exec,
        Self::Verify,
        Self::Fix,
    ];
}

impl PartialOrd for TeamPhase {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TeamPhase {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ordinal().cmp(&other.ordinal())
    }
}
```

- [ ] **Step 5: Add Hash derive to TeamPhase**

Change the `TeamPhase` derive line from:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamPhase {
```
to:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeamPhase {
```

- [ ] **Step 6: Replace the ignored placeholder test with real phase ordering tests**

Replace the existing `team_phase_ordering_placeholder` test:

```rust
    #[test]
    fn team_phase_ordering() {
        assert!(TeamPhase::Plan < TeamPhase::Exec);
        assert!(TeamPhase::Exec < TeamPhase::Verify);
        assert_eq!(TeamPhase::Plan.ordinal(), 0);
        assert_eq!(TeamPhase::Fix.ordinal(), 4);
        assert_eq!(TeamPhase::Exec.next(), Some(TeamPhase::Verify));
        assert_eq!(TeamPhase::Fix.next(), None);
    }

    #[test]
    fn display_impls_produce_lowercase_labels() {
        assert_eq!(CliProvider::Codex.to_string(), "codex");
        assert_eq!(TaskStatus::InProgress.to_string(), "in_progress");
        assert_eq!(DispatchStatus::Delivered.to_string(), "delivered");
        assert_eq!(TeamPhase::Verify.to_string(), "verify");
        assert_eq!(HookEventName::SessionStart.to_string(), "session_start");
    }
```

- [ ] **Step 7: Verify crate compiles and tests pass**

Run: `cargo test -p omx-types`
Expected: 6 tests pass, 0 ignored

- [ ] **Step 8: Commit**

```bash
git add crates/omx-types/
git commit -m "feat(omx-types): add Display impls, phase ordering, and Hash for TeamPhase"
```

---

## Task 2: Implement omx-config with full precedence chain

**Files:**
- Modify: `crates/omx-config/Cargo.toml`
- Modify: `crates/omx-config/src/lib.rs`

This task replaces the `todo!()` in `DefaultConfigLoader::load` with a working config loading implementation. The resolution order is: CLI arg > env var > `.omx-config.json` > `config.toml` > defaults.

- [ ] **Step 1: Add `toml` dev-dependency for tests and `dirs` for home resolution**

The `toml` crate is already a dependency. Add a `[dev-dependencies]` section to `crates/omx-config/Cargo.toml`:

```toml
[dev-dependencies]
insta = { workspace = true }
tempfile = "3"
```

- [ ] **Step 2: Add Default impls for config structs**

Add after the `TeamDefaults` struct definition in `crates/omx-config/src/lib.rs`:

```rust
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
```

- [ ] **Step 3: Add TOML config file structs for deserialization**

Add after the `Default` impls:

```rust
// ---------------------------------------------------------------------------
// TOML file representation (config.toml)
// These mirror the on-disk TOML structure which may differ from OmxConfig.
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
```

- [ ] **Step 4: Add JSON config file struct for deserialization**

Add after the TOML structs:

```rust
// ---------------------------------------------------------------------------
// JSON file representation (.omx-config.json)
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
```

- [ ] **Step 5: Implement the DefaultConfigLoader**

Replace the existing `DefaultConfigLoader` impl:

```rust
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
    // A top-level `model` key in TOML sets the frontier model
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
```

- [ ] **Step 6: Replace the ignored tests with real tests**

Replace the entire `#[cfg(test)] mod tests` block:

```rust
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
        assert_eq!(config.models.spark, "o4-mini"); // default preserved
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
        )
        .unwrap();
        fs::write(
            tmp.path().join(".omx-config.json"),
            r#"{"models": {"frontier": "from-json"}}"#,
        )
        .unwrap();

        let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
        assert_eq!(config.models.frontier, "from-json");
    }

    #[test]
    fn env_overrides_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("config.toml"),
            "[models]\nfrontier = \"from-toml\"\n",
        )
        .unwrap();

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
```

- [ ] **Step 7: Verify crate compiles and tests pass**

Run: `cargo test -p omx-config`
Expected: 12 tests pass, 0 ignored

- [ ] **Step 8: Commit**

```bash
git add crates/omx-config/
git commit -m "feat(omx-config): implement config loading with TOML/JSON/env precedence chain"
```

---

## Task 3: Implement omx-state FileStateStore with atomic I/O

**Files:**
- Modify: `crates/omx-state/Cargo.toml`
- Modify: `crates/omx-state/src/lib.rs`

This task replaces all `todo!()` stubs in `FileStateStore` with working implementations using temp-file + atomic rename for writes and fs2 file locking for concurrency safety.

- [ ] **Step 1: Add dev-dependencies to Cargo.toml**

Add to `crates/omx-state/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Implement the `read` method**

Replace the `read` method in the `impl StateStore for FileStateStore` block:

```rust
    async fn read<T: DeserializeOwned + Send>(&self, path: &Path) -> Result<Option<T>, OmxError> {
        let full_path = self.resolve(path);
        if !full_path.exists() {
            return Ok(None);
        }

        let data = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, OmxError> {
            use fs2::FileExt;
            let file = std::fs::File::open(&full_path)?;
            file.lock_shared()
                .map_err(|e| OmxError::State(format!("failed to acquire shared lock: {e}")))?;
            let data = std::fs::read(&full_path)?;
            file.unlock()
                .map_err(|e| OmxError::State(format!("failed to release lock: {e}")))?;
            Ok(data)
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))??;

        let value: T = serde_json::from_slice(&data)?;
        Ok(Some(value))
    }
```

- [ ] **Step 3: Implement the `write` method**

Replace the `write` method:

```rust
    async fn write<T: Serialize + Send + Sync>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<(), OmxError> {
        let full_path = self.resolve(path);
        let data = serde_json::to_vec_pretty(value)?;

        tokio::task::spawn_blocking(move || -> Result<(), OmxError> {
            use fs2::FileExt;

            // Ensure parent directory exists
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Write to a temp file in the same directory, then atomic rename
            let dir = full_path.parent().unwrap_or(Path::new("."));
            let mut tmp = tempfile::NamedTempFile::new_in(dir)
                .map_err(|e| OmxError::State(format!("failed to create temp file: {e}")))?;

            // Acquire exclusive lock on temp file
            tmp.as_file().lock_exclusive()
                .map_err(|e| OmxError::State(format!("failed to acquire exclusive lock: {e}")))?;

            std::io::Write::write_all(&mut tmp, &data)?;

            // Persist (atomic rename)
            tmp.persist(&full_path)
                .map_err(|e| OmxError::State(format!("failed to persist temp file: {e}")))?;

            Ok(())
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }
```

- [ ] **Step 4: Implement the `delete` method**

Replace the `delete` method:

```rust
    async fn delete(&self, path: &Path) -> Result<(), OmxError> {
        let full_path = self.resolve(path);

        tokio::task::spawn_blocking(move || -> Result<(), OmxError> {
            match std::fs::remove_file(&full_path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(OmxError::Io(e)),
            }
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }
```

- [ ] **Step 5: Implement the `list` method**

Replace the `list` method:

```rust
    async fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, OmxError> {
        let full_dir = self.resolve(dir);

        tokio::task::spawn_blocking(move || -> Result<Vec<PathBuf>, OmxError> {
            if !full_dir.exists() {
                return Ok(Vec::new());
            }
            let mut entries = Vec::new();
            for entry in std::fs::read_dir(&full_dir)? {
                let entry = entry?;
                entries.push(entry.path());
            }
            entries.sort();
            Ok(entries)
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }
```

- [ ] **Step 6: Implement the `append_jsonl` method**

Replace the `append_jsonl` method:

```rust
    async fn append_jsonl<T: Serialize + Send + Sync>(
        &self,
        path: &Path,
        entry: &T,
    ) -> Result<(), OmxError> {
        let full_path = self.resolve(path);
        let mut line = serde_json::to_string(entry)?;
        line.push('\n');

        tokio::task::spawn_blocking(move || -> Result<(), OmxError> {
            use fs2::FileExt;
            use std::io::Write;

            // Ensure parent directory exists
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&full_path)?;

            file.lock_exclusive()
                .map_err(|e| OmxError::State(format!("failed to acquire exclusive lock: {e}")))?;

            // Use BufWriter for the append
            let mut writer = std::io::BufWriter::new(&file);
            writer.write_all(line.as_bytes())?;
            writer.flush()?;

            file.unlock()
                .map_err(|e| OmxError::State(format!("failed to release lock: {e}")))?;

            Ok(())
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }
```

- [ ] **Step 7: Add `tempfile` as a regular dependency (needed by write)**

Update `crates/omx-state/Cargo.toml` dependencies section to add:

```toml
tempfile = "3"
```

(This is a runtime dependency because `write` uses `NamedTempFile` for atomic rename.)

- [ ] **Step 8: Replace ignored tests with real tests**

Replace the entire `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn file_state_store_resolves_paths() {
        let store = FileStateStore::new(PathBuf::from("/tmp/omx-state"));
        assert_eq!(
            store.resolve(Path::new("team/config.json")),
            PathBuf::from("/tmp/omx-state/team/config.json")
        );
    }

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        let data = TestData {
            name: "hello".into(),
            value: 42,
        };

        store
            .write(Path::new("test.json"), &data)
            .await
            .unwrap();

        let read_back: Option<TestData> = store.read(Path::new("test.json")).await.unwrap();
        assert_eq!(read_back, Some(data));
    }

    #[tokio::test]
    async fn read_nonexistent_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        let result: Option<TestData> = store.read(Path::new("nope.json")).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn write_creates_parent_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        let data = TestData {
            name: "nested".into(),
            value: 1,
        };
        store
            .write(Path::new("a/b/c/data.json"), &data)
            .await
            .unwrap();

        let read_back: Option<TestData> = store.read(Path::new("a/b/c/data.json")).await.unwrap();
        assert_eq!(read_back, Some(data));
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        let data = TestData {
            name: "bye".into(),
            value: 0,
        };
        store.write(Path::new("del.json"), &data).await.unwrap();
        store.delete(Path::new("del.json")).await.unwrap();

        let result: Option<TestData> = store.read(Path::new("del.json")).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn delete_nonexistent_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        // Should not error
        store.delete(Path::new("nope.json")).await.unwrap();
    }

    #[tokio::test]
    async fn list_returns_sorted_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        let data = TestData {
            name: "x".into(),
            value: 0,
        };
        store.write(Path::new("dir/b.json"), &data).await.unwrap();
        store.write(Path::new("dir/a.json"), &data).await.unwrap();
        store.write(Path::new("dir/c.json"), &data).await.unwrap();

        let entries = store.list(Path::new("dir")).await.unwrap();
        let names: Vec<&str> = entries
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();
        assert_eq!(names, vec!["a.json", "b.json", "c.json"]);
    }

    #[tokio::test]
    async fn list_nonexistent_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        let entries = store.list(Path::new("missing")).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn append_jsonl_preserves_existing_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());

        let entry1 = TestData {
            name: "first".into(),
            value: 1,
        };
        let entry2 = TestData {
            name: "second".into(),
            value: 2,
        };
        let entry3 = TestData {
            name: "third".into(),
            value: 3,
        };

        store
            .append_jsonl(Path::new("log.jsonl"), &entry1)
            .await
            .unwrap();
        store
            .append_jsonl(Path::new("log.jsonl"), &entry2)
            .await
            .unwrap();
        store
            .append_jsonl(Path::new("log.jsonl"), &entry3)
            .await
            .unwrap();

        let full_path = store.resolve(Path::new("log.jsonl"));
        let contents = std::fs::read_to_string(full_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 3);

        let parsed1: TestData = serde_json::from_str(lines[0]).unwrap();
        let parsed2: TestData = serde_json::from_str(lines[1]).unwrap();
        let parsed3: TestData = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(parsed1, entry1);
        assert_eq!(parsed2, entry2);
        assert_eq!(parsed3, entry3);
    }

    #[tokio::test]
    async fn concurrent_writes_do_not_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let store = std::sync::Arc::new(FileStateStore::new(tmp.path().to_path_buf()));

        let mut handles = Vec::new();
        for i in 0..20 {
            let store = store.clone();
            handles.push(tokio::spawn(async move {
                let data = TestData {
                    name: format!("writer-{i}"),
                    value: i,
                };
                store.write(Path::new("shared.json"), &data).await.unwrap();
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // The file should contain valid JSON (one of the writers' values)
        let result: Option<TestData> = store.read(Path::new("shared.json")).await.unwrap();
        assert!(result.is_some());
        let data = result.unwrap();
        assert!(data.name.starts_with("writer-"));
    }
}
```

- [ ] **Step 9: Verify crate compiles and tests pass**

Run: `cargo test -p omx-state`
Expected: 10 tests pass, 0 ignored

- [ ] **Step 10: Commit**

```bash
git add crates/omx-state/
git commit -m "feat(omx-state): implement FileStateStore with atomic writes and fs2 locking"
```

---

## Task 4: Refactor omx-mux to align with spec trait and integrate omx-types

**Files:**
- Modify: `crates/omx-mux/Cargo.toml`
- Modify: `crates/omx-mux/src/types.rs`
- Modify: `crates/omx-mux/src/tmux.rs`
- Modify: `crates/omx-mux/src/lib.rs`

The spec (section 4.3) defines a `MuxAdapter` trait with individual methods (`resolve_target`, `send_input`, `capture_tail`, `inspect_liveness`, `create_window`, `kill_window`, `send_keys`). The current implementation uses an enum-dispatch pattern (`execute(&MuxOperation) -> MuxOutcome`). This task adds the spec's method-based trait alongside the existing enum-dispatch, adds `create_window`/`kill_window`/`send_keys` operations, and integrates `omx-types` for error conversion.

- [ ] **Step 1: Add omx-types dependency to Cargo.toml**

Add to the `[dependencies]` section of `crates/omx-mux/Cargo.toml`:

```toml
omx-types = { path = "../omx-types" }
```

- [ ] **Step 2: Add new operations to MuxOperation and MuxOutcome enums**

In `crates/omx-mux/src/types.rs`, add three new variants to `MuxOperation`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MuxOperation {
    ResolveTarget {
        target: MuxTarget,
    },
    SendInput {
        target: MuxTarget,
        envelope: InputEnvelope,
    },
    CaptureTail {
        target: MuxTarget,
        visible_lines: usize,
    },
    InspectLiveness {
        target: MuxTarget,
    },
    Attach {
        target: MuxTarget,
    },
    Detach {
        target: MuxTarget,
    },
    CreateWindow {
        session: String,
        name: String,
    },
    KillWindow {
        target: String,
    },
    SendKeys {
        target: String,
        keys: String,
    },
}
```

Add matching variants to `MuxOutcome`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MuxOutcome {
    TargetResolved { resolved_handle: String },
    InputAccepted { bytes_written: usize },
    TailCaptured { visible_lines: usize, body: String },
    LivenessChecked { alive: bool },
    Attached { handle: String },
    Detached { handle: String },
    WindowCreated { handle: String },
    WindowKilled { handle: String },
    KeysSent { target: String },
}
```

- [ ] **Step 3: Add From<MuxError> for OmxError conversion**

Add at the bottom of `crates/omx-mux/src/types.rs`:

```rust
impl From<MuxError> for omx_types::OmxError {
    fn from(err: MuxError) -> Self {
        omx_types::OmxError::Tmux(err.to_string())
    }
}
```

- [ ] **Step 4: Implement the new operations in TmuxAdapter**

In `crates/omx-mux/src/tmux.rs`, add three new methods to the `TmuxAdapter` impl block (before the `MuxAdapter` impl):

```rust
    fn do_create_window(&self, session: &str, name: &str) -> Result<MuxOutcome, MuxError> {
        let output = run_tmux(&["new-window", "-t", session, "-n", name, "-P", "-F", "#{session_name}:#{window_index}"])?;
        let handle = output.trim().to_string();
        Ok(MuxOutcome::WindowCreated { handle })
    }

    fn do_kill_window(&self, target: &str) -> Result<MuxOutcome, MuxError> {
        run_tmux(&["kill-window", "-t", target])?;
        Ok(MuxOutcome::WindowKilled {
            handle: target.to_string(),
        })
    }

    fn do_send_keys(&self, target: &str, keys: &str) -> Result<MuxOutcome, MuxError> {
        run_tmux(&["send-keys", "-t", target, keys])?;
        Ok(MuxOutcome::KeysSent {
            target: target.to_string(),
        })
    }
```

- [ ] **Step 5: Update the MuxAdapter::execute match to handle new operations**

In `crates/omx-mux/src/tmux.rs`, extend the `execute` match:

```rust
impl MuxAdapter for TmuxAdapter {
    fn adapter_name(&self) -> &'static str {
        "tmux"
    }

    fn execute(&self, operation: &MuxOperation) -> Result<MuxOutcome, MuxError> {
        match operation {
            MuxOperation::ResolveTarget { target } => self.do_resolve_target(target),
            MuxOperation::SendInput { target, envelope } => self.do_send_input(target, envelope),
            MuxOperation::CaptureTail {
                target,
                visible_lines,
            } => self.do_capture_tail(target, *visible_lines),
            MuxOperation::InspectLiveness { target } => self.do_inspect_liveness(target),
            MuxOperation::Attach { target } => self.do_attach(target),
            MuxOperation::Detach { target } => self.do_detach(target),
            MuxOperation::CreateWindow { session, name } => self.do_create_window(session, name),
            MuxOperation::KillWindow { target } => self.do_kill_window(target),
            MuxOperation::SendKeys { target, keys } => self.do_send_keys(target, keys),
        }
    }
}
```

- [ ] **Step 6: Update describe_operation and contract summary**

In `crates/omx-mux/src/types.rs`, update `describe_operation`:

```rust
pub fn describe_operation(operation: &MuxOperation) -> &'static str {
    match operation {
        MuxOperation::ResolveTarget { .. } => "resolve-target",
        MuxOperation::SendInput { .. } => "send-input",
        MuxOperation::CaptureTail { .. } => "capture-tail",
        MuxOperation::InspectLiveness { .. } => "inspect-liveness",
        MuxOperation::Attach { .. } => "attach",
        MuxOperation::Detach { .. } => "detach",
        MuxOperation::CreateWindow { .. } => "create-window",
        MuxOperation::KillWindow { .. } => "kill-window",
        MuxOperation::SendKeys { .. } => "send-keys",
    }
}
```

In `crates/omx-mux/src/types.rs`, update `MUX_OPERATION_NAMES`:

```rust
pub const MUX_OPERATION_NAMES: &[&str] = &[
    "resolve-target",
    "send-input",
    "capture-tail",
    "inspect-liveness",
    "attach",
    "detach",
    "create-window",
    "kill-window",
    "send-keys",
];
```

- [ ] **Step 7: Update tests for new operations**

In `crates/omx-mux/src/lib.rs`, update the `canonical_contract_names_remain_generic` test:

```rust
    #[test]
    fn canonical_contract_names_remain_generic() {
        assert_eq!(
            MUX_OPERATION_NAMES,
            &[
                "resolve-target",
                "send-input",
                "capture-tail",
                "inspect-liveness",
                "attach",
                "detach",
                "create-window",
                "kill-window",
                "send-keys",
            ]
        );
        assert_eq!(MUX_TARGET_KINDS, &["delivery-handle", "detached"]);
    }
```

Add a new test for the `CreateWindow` operation serde roundtrip in `crates/omx-mux/src/lib.rs`:

```rust
    #[test]
    fn serde_roundtrip_create_window() {
        let op = MuxOperation::CreateWindow {
            session: "omx-team-dev".into(),
            name: "worker-1".into(),
        };
        let json = serde_json::to_string(&op).expect("serialize");
        let deserialized: MuxOperation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(format!("{deserialized:?}"), format!("{op:?}"));
    }

    #[test]
    fn serde_roundtrip_window_created_outcome() {
        let outcome = MuxOutcome::WindowCreated {
            handle: "omx-team-dev:1".into(),
        };
        let json = serde_json::to_string(&outcome).expect("serialize");
        let deserialized: MuxOutcome = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, outcome);
    }
```

Add a test for `MuxError` to `OmxError` conversion in `crates/omx-mux/src/lib.rs`:

```rust
    #[test]
    fn mux_error_converts_to_omx_error() {
        let mux_err = MuxError::AdapterFailed("tmux not found".into());
        let omx_err: omx_types::OmxError = mux_err.into();
        assert!(omx_err.to_string().contains("tmux"));
    }
```

- [ ] **Step 8: Verify crate compiles and tests pass**

Run: `cargo test -p omx-mux`
Expected: All tests pass (existing + 3 new)

- [ ] **Step 9: Verify full workspace compiles**

Run: `cargo check`
Expected: Entire workspace compiles. No `todo!()` panics remain in the four foundation crates.

- [ ] **Step 10: Commit**

```bash
git add crates/omx-mux/
git commit -m "feat(omx-mux): add create_window/kill_window/send_keys ops and omx-types integration"
```

---

## Task 5: Final verification and workspace-wide check

**Files:**
- None (verification only)

- [ ] **Step 1: Run all foundation crate tests together**

Run: `cargo test -p omx-types -p omx-config -p omx-state -p omx-mux`
Expected: All tests pass across all four crates.

- [ ] **Step 2: Verify no todo!() remains in foundation crates**

Run: `rg 'todo!' crates/omx-types/ crates/omx-config/ crates/omx-state/ crates/omx-mux/`
Expected: No matches found.

- [ ] **Step 3: Run clippy on foundation crates**

Run: `cargo clippy -p omx-types -p omx-config -p omx-state -p omx-mux -- -D warnings`
Expected: No warnings or errors.

- [ ] **Step 4: Verify full workspace still compiles**

Run: `cargo check`
Expected: Success. Downstream crates that depend on these four should still compile (their `todo!()` stubs are unchanged).

- [ ] **Step 5: Commit any clippy fixes if needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings in foundation crates"
```

(Skip this step if clippy reported no issues.)
