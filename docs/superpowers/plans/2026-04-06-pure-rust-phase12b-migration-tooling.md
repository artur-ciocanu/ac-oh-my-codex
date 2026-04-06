# Phase 12b: Migration Tooling — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `omx migrate` subcommand (config + session migration) and enhance `omx doctor` with grouped output, notification binary checks, config validation, hook checks, and TS migration hints.

**Architecture:** All new code lives in `crates/omx-cli/` — two new modules (`doctor.rs`, `migrate.rs`) plus clap subcommand additions in `main.rs`. No new crates. Config migration reads `.omx-config.json` and merges into `config.toml`. Session migration reads `rollout-*.jsonl` files and writes per-session directories. Doctor is extracted from inline code to a grouped-output module with 16 checks.

**Tech Stack:** Rust, clap, serde_json, toml, omx-config, omx-state, omx-session, tempfile (tests)

---

## File Structure

### New files

| File | Responsibility |
|------|---------------|
| `crates/omx-cli/src/doctor.rs` | Enhanced doctor with grouped output, 16 checks across 6 categories |
| `crates/omx-cli/src/migrate.rs` | Config migration (JSON→TOML), session migration (rollout JSONL→per-session dirs), dry-run/force |

### Modified files

| File | Change |
|------|--------|
| `crates/omx-cli/Cargo.toml` | Add `toml` workspace dependency |
| `crates/omx-cli/src/main.rs` | Add `mod doctor; mod migrate;`, add `Migrate` subcommand to clap enum, replace inline doctor with `doctor::run_doctor()`, add migrate handler |
| `tests/src/cli.rs` | Add 6 integration tests for migrate and enhanced doctor |

---

## Task 1: Extract and Enhance `omx doctor`

**Files:**
- Create: `crates/omx-cli/src/doctor.rs`
- Modify: `crates/omx-cli/src/main.rs:1,290-334`

- [ ] **Step 1: Create `doctor.rs` with types and all 16 checks**

Create `crates/omx-cli/src/doctor.rs`:

```rust
use std::path::Path;

// ---------------------------------------------------------------------------
// Check result types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum CheckStatus {
    Ok,
    Missing,
    Invalid(String),
    Warning(String),
    Info(String),
}

impl CheckStatus {
    fn label(&self) -> &str {
        match self {
            CheckStatus::Ok => "ok",
            CheckStatus::Missing => "MISSING",
            CheckStatus::Invalid(_) => "INVALID",
            CheckStatus::Warning(_) => "WARNING",
            CheckStatus::Info(_) => "info",
        }
    }

    fn is_failure(&self) -> bool {
        matches!(self, CheckStatus::Missing | CheckStatus::Invalid(_))
    }
}

#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
}

#[derive(Debug)]
pub struct CheckGroup {
    pub name: &'static str,
    pub checks: Vec<CheckResult>,
}

// ---------------------------------------------------------------------------
// Check helpers
// ---------------------------------------------------------------------------

fn check_binary_on_path(name: &str) -> CheckResult {
    let ok = std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    CheckResult {
        name: name.to_string(),
        status: if ok { CheckStatus::Ok } else { CheckStatus::Missing },
    }
}

fn check_binary_runs(name: &str, args: &[&str]) -> CheckResult {
    let ok = std::process::Command::new(name)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    CheckResult {
        name: name.to_string(),
        status: if ok { CheckStatus::Ok } else { CheckStatus::Missing },
    }
}

// ---------------------------------------------------------------------------
// Check groups
// ---------------------------------------------------------------------------

fn check_dependencies() -> CheckGroup {
    CheckGroup {
        name: "Dependencies",
        checks: vec![
            check_binary_runs("tmux", &["-V"]),
            check_binary_runs("codex", &["--version"]),
            check_binary_runs("claude", &["--version"]),
        ],
    }
}

fn check_mcp_servers() -> CheckGroup {
    CheckGroup {
        name: "MCP Servers",
        checks: vec![
            check_binary_on_path("omx-mcp-state"),
            check_binary_on_path("omx-mcp-memory"),
            check_binary_on_path("omx-mcp-code-intel"),
            check_binary_on_path("omx-mcp-trace"),
            check_binary_on_path("omx-mcp-team"),
        ],
    }
}

fn check_notification_binaries() -> CheckGroup {
    CheckGroup {
        name: "Notification Hooks",
        checks: vec![
            check_binary_on_path("omx-notify-discord"),
            check_binary_on_path("omx-notify-slack"),
            check_binary_on_path("omx-notify-telegram"),
            check_binary_on_path("omx-notify-pushover"),
            check_binary_on_path("omx-notify-generic"),
        ],
    }
}

fn check_configuration(codex_home: &Path) -> CheckGroup {
    let config_path = codex_home.join("config.toml");
    let exists = config_path.exists();

    let mut checks = vec![CheckResult {
        name: "config.toml exists".to_string(),
        status: if exists {
            CheckStatus::Ok
        } else {
            CheckStatus::Missing
        },
    }];

    if exists {
        let schema_check = match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<toml::Value>(&contents) {
                Ok(_) => CheckResult {
                    name: "config.toml schema valid".to_string(),
                    status: CheckStatus::Ok,
                },
                Err(e) => CheckResult {
                    name: "config.toml schema valid".to_string(),
                    status: CheckStatus::Invalid(e.to_string()),
                },
            },
            Err(e) => CheckResult {
                name: "config.toml schema valid".to_string(),
                status: CheckStatus::Invalid(format!("cannot read: {e}")),
            },
        };
        checks.push(schema_check);
    }

    CheckGroup {
        name: "Configuration",
        checks,
    }
}

fn check_hooks(codex_home: &Path) -> CheckGroup {
    let hooks_dir = codex_home.join(".omx").join("hooks");
    let check = if !hooks_dir.exists() {
        CheckResult {
            name: "hooks directory".to_string(),
            status: CheckStatus::Info("no hooks directory".to_string()),
        }
    } else {
        match std::fs::read_dir(&hooks_dir) {
            Ok(entries) => {
                let mut total = 0u32;
                let mut non_exec = 0u32;
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        total += 1;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if let Ok(meta) = path.metadata() {
                                if meta.permissions().mode() & 0o111 == 0 {
                                    non_exec += 1;
                                }
                            }
                        }
                    }
                }
                if total == 0 {
                    CheckResult {
                        name: "hooks".to_string(),
                        status: CheckStatus::Info("no hooks found".to_string()),
                    }
                } else if non_exec > 0 {
                    CheckResult {
                        name: format!("{total} hooks found"),
                        status: CheckStatus::Warning(format!(
                            "{non_exec} non-executable hook(s)"
                        )),
                    }
                } else {
                    CheckResult {
                        name: format!("{total} hooks found, all executable"),
                        status: CheckStatus::Ok,
                    }
                }
            }
            Err(e) => CheckResult {
                name: "hooks directory".to_string(),
                status: CheckStatus::Warning(format!("cannot read: {e}")),
            },
        }
    };

    CheckGroup {
        name: "Hooks",
        checks: vec![check],
    }
}

fn check_ts_migration(codex_home: &Path) -> CheckGroup {
    let mut checks = Vec::new();

    // Check for TS-era .omx-config.json
    let json_config = codex_home.join(".omx-config.json");
    if json_config.exists() {
        checks.push(CheckResult {
            name: ".omx-config.json found".to_string(),
            status: CheckStatus::Warning(
                "run `omx migrate config` to convert".to_string(),
            ),
        });
    }

    // Check for TS-era rollout files
    let sessions_dir = codex_home.join("sessions");
    if sessions_dir.exists() {
        let rollout_count = std::fs::read_dir(&sessions_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| {
                        let name = e.file_name();
                        let name = name.to_string_lossy();
                        name.starts_with("rollout-") && name.ends_with(".jsonl")
                    })
                    .count()
            })
            .unwrap_or(0);
        if rollout_count > 0 {
            checks.push(CheckResult {
                name: format!("rollout-*.jsonl found ({rollout_count} files)"),
                status: CheckStatus::Warning(
                    "run `omx migrate sessions` to convert".to_string(),
                ),
            });
        }
    }

    if checks.is_empty() {
        checks.push(CheckResult {
            name: "No TS-era files detected".to_string(),
            status: CheckStatus::Ok,
        });
    }

    CheckGroup {
        name: "TS Migration",
        checks,
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run all doctor checks and print grouped output.
/// Returns true if all checks passed (no Missing/Invalid).
pub fn run_doctor(codex_home: &Path) -> bool {
    println!("omx doctor — checking installation\n");

    let groups = vec![
        check_dependencies(),
        check_mcp_servers(),
        check_notification_binaries(),
        check_configuration(codex_home),
        check_hooks(codex_home),
        check_ts_migration(codex_home),
    ];

    let mut any_failure = false;

    for group in &groups {
        println!("  {}", group.name);
        for check in &group.checks {
            if check.status.is_failure() {
                any_failure = true;
            }
            match &check.status {
                CheckStatus::Ok => {
                    println!("    {:<7} {}", check.status.label(), check.name);
                }
                CheckStatus::Missing => {
                    println!("    {:<7} {}", check.status.label(), check.name);
                }
                CheckStatus::Invalid(reason) => {
                    println!(
                        "    {:<7} {} — {}",
                        check.status.label(),
                        check.name,
                        reason
                    );
                }
                CheckStatus::Warning(msg) => {
                    println!(
                        "    {:<7} {} — {}",
                        check.status.label(),
                        check.name,
                        msg
                    );
                }
                CheckStatus::Info(msg) => {
                    println!("    {:<7} {}", msg, check.name);
                }
            }
        }
        println!();
    }

    // Print help for common issues
    let deps = &groups[0];
    if deps
        .checks
        .iter()
        .any(|c| matches!(c.status, CheckStatus::Missing) && c.name == "tmux")
    {
        println!("  tmux is required. Install with: brew install tmux");
    }

    !any_failure
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn doctor_groups_checks_correctly() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("config.toml"), "[models]\nfrontier = \"o3\"\n").unwrap();
        // run_doctor prints to stdout; we just verify it doesn't panic
        // and returns a boolean
        let _result = run_doctor(dir.path());
    }

    #[test]
    fn doctor_detects_ts_era_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5"}}"#,
        )
        .unwrap();

        let group = check_ts_migration(dir.path());
        assert_eq!(group.name, "TS Migration");
        assert!(group.checks.iter().any(|c| c.name.contains(".omx-config.json")));
        assert!(group
            .checks
            .iter()
            .any(|c| matches!(&c.status, CheckStatus::Warning(msg) if msg.contains("migrate"))));
    }

    #[test]
    fn doctor_detects_rollout_files() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();
        std::fs::write(sessions_dir.join("rollout-abc.jsonl"), "{}").unwrap();
        std::fs::write(sessions_dir.join("rollout-def.jsonl"), "{}").unwrap();

        let group = check_ts_migration(dir.path());
        assert!(group
            .checks
            .iter()
            .any(|c| c.name.contains("rollout") && c.name.contains("2 files")));
    }

    #[test]
    fn doctor_no_ts_era_files() {
        let dir = TempDir::new().unwrap();
        let group = check_ts_migration(dir.path());
        assert!(group
            .checks
            .iter()
            .any(|c| c.name.contains("No TS-era") && matches!(c.status, CheckStatus::Ok)));
    }

    #[test]
    fn doctor_validates_config_schema() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("config.toml"), "not valid [[[ toml").unwrap();

        let group = check_configuration(dir.path());
        assert!(group
            .checks
            .iter()
            .any(|c| c.name.contains("schema") && matches!(&c.status, CheckStatus::Invalid(_))));
    }

    #[test]
    fn doctor_valid_config_passes_schema() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "[models]\nfrontier = \"o3\"\n",
        )
        .unwrap();

        let group = check_configuration(dir.path());
        let schema_check = group.checks.iter().find(|c| c.name.contains("schema"));
        assert!(schema_check.is_some());
        assert!(matches!(schema_check.unwrap().status, CheckStatus::Ok));
    }
}
```

- [ ] **Step 2: Add `toml` dependency to omx-cli Cargo.toml**

In `crates/omx-cli/Cargo.toml`, add after the `tracing-subscriber` line:

```toml
toml = { workspace = true }
```

- [ ] **Step 3: Wire doctor module in `main.rs`**

In `crates/omx-cli/src/main.rs`, add the module declaration at line 2 (after `mod cleanup;` and `mod launch;`):

```rust
mod doctor;
```

Then replace the entire `Some(Commands::Doctor) => { ... }` block (lines 290-334) with:

```rust
        Some(Commands::Doctor) => {
            let home = omx_config::default_codex_home();
            let all_ok = doctor::run_doctor(&home);
            if !all_ok {
                std::process::exit(1);
            }
        }
```

- [ ] **Step 4: Run doctor unit tests**

Run: `cargo test -p omx-cli doctor -- --nocapture`
Expected: All 5 doctor tests PASS

- [ ] **Step 5: Run full CLI build to verify compilation**

Run: `cargo build -p omx-cli 2>&1 | tail -5`
Expected: Clean build

- [ ] **Step 6: Commit**

```bash
git add crates/omx-cli/src/doctor.rs crates/omx-cli/src/main.rs crates/omx-cli/Cargo.toml
git commit -m "feat(omx-cli): extract doctor to module with grouped output and 16 checks"
```

---

## Task 2: Config Migration

**Files:**
- Create: `crates/omx-cli/src/migrate.rs`
- Modify: `crates/omx-cli/src/main.rs`

- [ ] **Step 1: Create `migrate.rs` with config migration logic and tests**

Create `crates/omx-cli/src/migrate.rs`:

```rust
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Config migration: .omx-config.json → config.toml
// ---------------------------------------------------------------------------

/// Represents a single field migration from JSON to TOML.
#[derive(Debug, Clone)]
pub struct MigratedField {
    pub toml_key: String,
    pub value: String,
    pub source: String,
    pub skipped: bool,
    pub reason: Option<String>,
}

/// Result of a config migration run.
#[derive(Debug)]
pub struct ConfigMigrationResult {
    pub fields: Vec<MigratedField>,
    pub source_path: String,
    pub target_path: String,
}

impl ConfigMigrationResult {
    pub fn migrated_count(&self) -> usize {
        self.fields.iter().filter(|f| !f.skipped).count()
    }

    pub fn skipped_count(&self) -> usize {
        self.fields.iter().filter(|f| f.skipped).count()
    }
}

/// Parse the TS-era .omx-config.json and extract fields to migrate.
///
/// The TS-era JSON has this shape:
/// ```json
/// {
///   "env": { "KEY": "value" },
///   "models": { "default": "gpt-5.4", "team": "gpt-5.4-mini" }
/// }
/// ```
///
/// Mapping:
/// - `models.default` → `models.frontier`
/// - `models.<mode>` → `models.per_mode.<mode>`
/// - `env.<KEY>` → `env.<KEY>`
fn extract_fields_from_json(json: &JsonValue) -> Vec<(String, String, String)> {
    let mut fields = Vec::new(); // (toml_key, value, source_description)

    if let Some(models) = json.get("models").and_then(|v| v.as_object()) {
        for (key, value) in models {
            if let Some(val_str) = value.as_str() {
                if key == "default" {
                    fields.push((
                        "models.frontier".to_string(),
                        val_str.to_string(),
                        "models.default".to_string(),
                    ));
                } else {
                    fields.push((
                        format!("models.per_mode.{key}"),
                        val_str.to_string(),
                        format!("models.{key}"),
                    ));
                }
            }
        }
    }

    if let Some(env) = json.get("env").and_then(|v| v.as_object()) {
        for (key, value) in env {
            if let Some(val_str) = value.as_str() {
                fields.push((
                    format!("env.{key}"),
                    val_str.to_string(),
                    format!("env.{key}"),
                ));
            }
        }
    }

    fields
}

/// Check if a TOML key already exists in the parsed TOML table.
fn toml_key_exists(toml: &toml::Value, dotted_key: &str) -> bool {
    let parts: Vec<&str> = dotted_key.split('.').collect();
    let mut current = toml;
    for part in &parts {
        match current.get(part) {
            Some(next) => current = next,
            None => return false,
        }
    }
    true
}

/// Set a dotted key in a TOML table. Creates intermediate tables as needed.
fn toml_set_key(toml: &mut toml::Value, dotted_key: &str, value: &str) {
    let parts: Vec<&str> = dotted_key.split('.').collect();
    let mut current = toml;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part — set the value
            current
                .as_table_mut()
                .unwrap()
                .insert(part.to_string(), toml::Value::String(value.to_string()));
        } else {
            // Intermediate part — ensure table exists
            if !current.as_table().unwrap().contains_key(*part) {
                current
                    .as_table_mut()
                    .unwrap()
                    .insert(part.to_string(), toml::Value::Table(toml::map::Map::new()));
            }
            current = current.get_mut(part).unwrap();
        }
    }
}

/// Run the config migration.
///
/// `codex_home` is the root directory (e.g., `~/.codex`).
/// `dry_run` — if true, don't write anything.
/// `force` — if true, overwrite existing TOML values.
pub fn migrate_config(
    codex_home: &Path,
    dry_run: bool,
    force: bool,
) -> Result<ConfigMigrationResult, String> {
    let json_path = codex_home.join(".omx-config.json");
    let toml_path = codex_home.join("config.toml");

    if !json_path.exists() {
        return Ok(ConfigMigrationResult {
            fields: Vec::new(),
            source_path: json_path.display().to_string(),
            target_path: toml_path.display().to_string(),
        });
    }

    // Read and parse JSON
    let json_str = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("Failed to read {}: {e}", json_path.display()))?;
    let json: JsonValue = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse {}: {e}", json_path.display()))?;

    // Read existing TOML (or start with empty table)
    let mut toml_value: toml::Value = if toml_path.exists() {
        let toml_str = std::fs::read_to_string(&toml_path)
            .map_err(|e| format!("Failed to read {}: {e}", toml_path.display()))?;
        toml::from_str(&toml_str)
            .map_err(|e| format!("Failed to parse {}: {e}", toml_path.display()))?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    // Extract fields and apply
    let extracted = extract_fields_from_json(&json);
    let mut fields = Vec::new();

    for (toml_key, value, source) in &extracted {
        let exists = toml_key_exists(&toml_value, toml_key);
        if exists && !force {
            fields.push(MigratedField {
                toml_key: toml_key.clone(),
                value: value.clone(),
                source: source.clone(),
                skipped: true,
                reason: Some("already exists in config.toml".to_string()),
            });
        } else {
            toml_set_key(&mut toml_value, toml_key, value);
            fields.push(MigratedField {
                toml_key: toml_key.clone(),
                value: value.clone(),
                source: source.clone(),
                skipped: false,
                reason: None,
            });
        }
    }

    // Write TOML (unless dry-run)
    if !dry_run && fields.iter().any(|f| !f.skipped) {
        let toml_str = toml::to_string_pretty(&toml_value)
            .map_err(|e| format!("Failed to serialize TOML: {e}"))?;
        std::fs::write(&toml_path, toml_str)
            .map_err(|e| format!("Failed to write {}: {e}", toml_path.display()))?;

        // Back up the original JSON
        let backup_path = codex_home.join(".omx-config.json.bak");
        std::fs::copy(&json_path, &backup_path)
            .map_err(|e| format!("Failed to back up JSON: {e}"))?;
    }

    Ok(ConfigMigrationResult {
        fields,
        source_path: json_path.display().to_string(),
        target_path: toml_path.display().to_string(),
    })
}

/// Print config migration results.
pub fn print_config_result(result: &ConfigMigrationResult, dry_run: bool) {
    if dry_run {
        println!("omx migrate config --dry-run\n");
    } else {
        println!("omx migrate config\n");
    }

    if result.fields.is_empty() {
        println!("  No TS-era config found, nothing to migrate");
        return;
    }

    println!("  Source: {}", result.source_path);
    println!("  Target: {}\n", result.target_path);

    for field in &result.fields {
        if field.skipped {
            println!(
                "  SKIP {} = {:?} ({})",
                field.toml_key,
                field.value,
                field.reason.as_deref().unwrap_or("skipped")
            );
        } else {
            println!(
                "  {} = {:?} (from {})",
                field.toml_key, field.value, field.source
            );
        }
    }

    println!(
        "\n  Migrated {} fields ({} skipped)",
        result.migrated_count(),
        result.skipped_count()
    );
}

// ---------------------------------------------------------------------------
// Session migration: rollout-*.jsonl → per-session dirs
// ---------------------------------------------------------------------------

/// Result of migrating a single session.
#[derive(Debug)]
pub struct SessionMigrationEntry {
    pub session_id: String,
    pub turns: u32,
    pub skipped: bool,
    pub failed: bool,
    pub reason: Option<String>,
}

/// Result of migrating all sessions.
#[derive(Debug)]
pub struct SessionMigrationResult {
    pub rollout_files: usize,
    pub entries: Vec<SessionMigrationEntry>,
}

impl SessionMigrationResult {
    pub fn migrated_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.skipped && !e.failed)
            .count()
    }

    pub fn skipped_count(&self) -> usize {
        self.entries.iter().filter(|e| e.skipped).count()
    }

    pub fn failed_count(&self) -> usize {
        self.entries.iter().filter(|e| e.failed).count()
    }
}

/// Parsed data from a single session found in rollout files.
#[derive(Debug)]
struct ParsedSession {
    id: String,
    timestamp: Option<String>,
    mode: String,
    turns: Vec<ParsedTurn>,
}

#[derive(Debug)]
struct ParsedTurn {
    role: String,
    content: String,
    timestamp: Option<String>,
}

/// Scan rollout-*.jsonl files and group records by session.
fn parse_rollout_files(
    sessions_dir: &Path,
) -> Result<(usize, Vec<ParsedSession>), String> {
    if !sessions_dir.exists() {
        return Ok((0, Vec::new()));
    }

    let mut rollout_files: Vec<std::path::PathBuf> = std::fs::read_dir(sessions_dir)
        .map_err(|e| format!("Failed to read sessions dir: {e}"))?
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("rollout-") && name.ends_with(".jsonl")
        })
        .map(|e| e.path())
        .collect();
    rollout_files.sort();

    let file_count = rollout_files.len();
    let mut sessions_map: HashMap<String, ParsedSession> = HashMap::new();

    for path in &rollout_files {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

        // Derive fallback session ID from filename: rollout-<id>.jsonl → <id>
        let fallback_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .strip_prefix("rollout-")
            .unwrap_or("unknown")
            .to_string();

        let mut current_session_id = fallback_id.clone();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parsed: JsonValue = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue, // Skip malformed lines
            };

            let record_type = parsed
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            if record_type == "session_meta" {
                if let Some(payload) = parsed.get("payload") {
                    if let Some(id) = payload.get("id").and_then(|v| v.as_str()) {
                        current_session_id = id.to_string();
                    }
                    let timestamp = payload
                        .get("timestamp")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let mode = payload
                        .get("agent_role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    sessions_map
                        .entry(current_session_id.clone())
                        .or_insert_with(|| ParsedSession {
                            id: current_session_id.clone(),
                            timestamp,
                            mode,
                            turns: Vec::new(),
                        });
                }
            } else if record_type == "response_item" {
                if let Some(payload) = parsed.get("payload") {
                    let payload_type = payload
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    if payload_type == "message" {
                        let role = payload
                            .get("role")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        // Extract text content from content array
                        let content = extract_text_content(payload.get("content"));

                        if !content.is_empty() {
                            let session = sessions_map
                                .entry(current_session_id.clone())
                                .or_insert_with(|| ParsedSession {
                                    id: current_session_id.clone(),
                                    timestamp: None,
                                    mode: "unknown".to_string(),
                                    turns: Vec::new(),
                                });

                            session.turns.push(ParsedTurn {
                                role,
                                content,
                                timestamp: None,
                            });
                        }
                    }
                }
            }
        }
    }

    let sessions: Vec<ParsedSession> = sessions_map.into_values().collect();
    Ok((file_count, sessions))
}

/// Extract text from a content field (may be string, array of objects with "text" keys, etc.)
fn extract_text_content(content: Option<&JsonValue>) -> String {
    match content {
        Some(JsonValue::String(s)) => s.clone(),
        Some(JsonValue::Array(arr)) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    parts.push(text);
                } else if let Some(s) = item.as_str() {
                    parts.push(s);
                }
            }
            parts.join("\n")
        }
        _ => String::new(),
    }
}

/// Run session migration.
pub fn migrate_sessions(
    codex_home: &Path,
    dry_run: bool,
    force: bool,
) -> Result<SessionMigrationResult, String> {
    let ts_sessions_dir = codex_home.join("sessions");
    let rust_sessions_dir = codex_home.join(".omx").join("sessions");

    let (file_count, sessions) = parse_rollout_files(&ts_sessions_dir)?;

    if file_count == 0 {
        return Ok(SessionMigrationResult {
            rollout_files: 0,
            entries: Vec::new(),
        });
    }

    let mut entries = Vec::new();

    for session in &sessions {
        let session_dir = rust_sessions_dir.join(&session.id);

        // Check if already exists
        if session_dir.exists() && !force {
            entries.push(SessionMigrationEntry {
                session_id: session.id.clone(),
                turns: session.turns.len() as u32,
                skipped: true,
                failed: false,
                reason: Some("already exists".to_string()),
            });
            continue;
        }

        if dry_run {
            entries.push(SessionMigrationEntry {
                session_id: session.id.clone(),
                turns: session.turns.len() as u32,
                skipped: false,
                failed: false,
                reason: None,
            });
            continue;
        }

        // Write meta.json and transcript.jsonl
        match write_rust_session(&rust_sessions_dir, session) {
            Ok(()) => {
                entries.push(SessionMigrationEntry {
                    session_id: session.id.clone(),
                    turns: session.turns.len() as u32,
                    skipped: false,
                    failed: false,
                    reason: None,
                });
            }
            Err(e) => {
                entries.push(SessionMigrationEntry {
                    session_id: session.id.clone(),
                    turns: session.turns.len() as u32,
                    skipped: false,
                    failed: true,
                    reason: Some(e),
                });
            }
        }
    }

    Ok(SessionMigrationResult {
        rollout_files: file_count,
        entries,
    })
}

/// Write a parsed session to the Rust-era per-session directory.
fn write_rust_session(
    sessions_root: &Path,
    session: &ParsedSession,
) -> Result<(), String> {
    let session_dir = sessions_root.join(&session.id);
    std::fs::create_dir_all(&session_dir)
        .map_err(|e| format!("Failed to create session dir: {e}"))?;

    // Write meta.json
    let started_at = session
        .timestamp
        .as_deref()
        .unwrap_or("1970-01-01T00:00:00Z");
    let meta = serde_json::json!({
        "id": session.id,
        "started_at": started_at,
        "ended_at": null,
        "mode": session.mode,
        "turns": session.turns.len(),
        "tokens_used": 0,
        "status": "completed"
    });
    let meta_str = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Failed to serialize meta: {e}"))?;
    std::fs::write(session_dir.join("meta.json"), meta_str)
        .map_err(|e| format!("Failed to write meta.json: {e}"))?;

    // Write transcript.jsonl
    let mut transcript = String::new();
    for (i, turn) in session.turns.iter().enumerate() {
        let timestamp = turn
            .timestamp
            .as_deref()
            .or(session.timestamp.as_deref())
            .unwrap_or("1970-01-01T00:00:00Z");
        let turn_json = serde_json::json!({
            "turn_number": i + 1,
            "timestamp": timestamp,
            "role": turn.role,
            "content": turn.content,
            "tokens": 0
        });
        let line = serde_json::to_string(&turn_json)
            .map_err(|e| format!("Failed to serialize turn: {e}"))?;
        transcript.push_str(&line);
        transcript.push('\n');
    }
    if !transcript.is_empty() {
        std::fs::write(session_dir.join("transcript.jsonl"), transcript)
            .map_err(|e| format!("Failed to write transcript.jsonl: {e}"))?;
    }

    Ok(())
}

/// Print session migration results.
pub fn print_sessions_result(result: &SessionMigrationResult, dry_run: bool) {
    if dry_run {
        println!("omx migrate sessions --dry-run\n");
    } else {
        println!("omx migrate sessions\n");
    }

    if result.rollout_files == 0 {
        println!("  No rollout-*.jsonl files found, nothing to migrate");
        return;
    }

    println!(
        "  Found: {} rollout files, {} sessions\n",
        result.rollout_files,
        result.entries.len()
    );

    for entry in &result.entries {
        if entry.skipped {
            println!(
                "    - {}  ({})",
                entry.session_id,
                entry.reason.as_deref().unwrap_or("skipped")
            );
        } else if entry.failed {
            println!(
                "    x {}  (FAILED: {})",
                entry.session_id,
                entry.reason.as_deref().unwrap_or("unknown error")
            );
        } else {
            println!("    + {}  ({} turns)", entry.session_id, entry.turns);
        }
    }

    println!(
        "\n  Migrated {} sessions ({} skipped, {} failed)",
        result.migrated_count(),
        result.skipped_count(),
        result.failed_count()
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- Config migration tests ---

    #[test]
    fn config_migration_maps_default_to_frontier() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4"}}"#,
        )
        .unwrap();

        let result = migrate_config(dir.path(), false, false).unwrap();
        assert_eq!(result.migrated_count(), 1);
        assert_eq!(result.fields[0].toml_key, "models.frontier");
        assert_eq!(result.fields[0].value, "gpt-5.4");

        // Verify TOML was written
        let toml_str = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(toml_str.contains("gpt-5.4"));
    }

    #[test]
    fn config_migration_maps_mode_to_per_mode() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"models": {"team": "gpt-5.4-mini", "autoresearch": "o3"}}"#,
        )
        .unwrap();

        let result = migrate_config(dir.path(), false, false).unwrap();
        assert_eq!(result.migrated_count(), 2);
        assert!(result
            .fields
            .iter()
            .any(|f| f.toml_key == "models.per_mode.team" && f.value == "gpt-5.4-mini"));
        assert!(result
            .fields
            .iter()
            .any(|f| f.toml_key == "models.per_mode.autoresearch" && f.value == "o3"));
    }

    #[test]
    fn config_migration_preserves_existing_toml() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "[models]\nfrontier = \"existing-model\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4"}}"#,
        )
        .unwrap();

        let result = migrate_config(dir.path(), false, false).unwrap();
        assert_eq!(result.migrated_count(), 0);
        assert_eq!(result.skipped_count(), 1);
        assert!(result.fields[0].skipped);

        // Verify existing value is preserved
        let toml_str = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(toml_str.contains("existing-model"));
    }

    #[test]
    fn config_migration_force_overwrites() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "[models]\nfrontier = \"existing-model\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4"}}"#,
        )
        .unwrap();

        let result = migrate_config(dir.path(), false, true).unwrap();
        assert_eq!(result.migrated_count(), 1);
        assert_eq!(result.skipped_count(), 0);

        let toml_str = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(toml_str.contains("gpt-5.4"));
    }

    #[test]
    fn config_migration_no_json_file() {
        let dir = TempDir::new().unwrap();
        let result = migrate_config(dir.path(), false, false).unwrap();
        assert!(result.fields.is_empty());
    }

    #[test]
    fn config_migration_env_vars() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"env": {"MY_KEY": "my_value", "OTHER": "val2"}}"#,
        )
        .unwrap();

        let result = migrate_config(dir.path(), false, false).unwrap();
        assert_eq!(result.migrated_count(), 2);
        assert!(result
            .fields
            .iter()
            .any(|f| f.toml_key == "env.MY_KEY" && f.value == "my_value"));
    }

    #[test]
    fn config_migration_dry_run_does_not_write() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4"}}"#,
        )
        .unwrap();

        let result = migrate_config(dir.path(), true, false).unwrap();
        assert_eq!(result.migrated_count(), 1);

        // config.toml should NOT exist
        assert!(!dir.path().join("config.toml").exists());
    }

    #[test]
    fn config_migration_creates_backup() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4"}}"#,
        )
        .unwrap();

        migrate_config(dir.path(), false, false).unwrap();
        assert!(dir.path().join(".omx-config.json.bak").exists());
    }

    // --- Session migration tests ---

    #[test]
    fn session_migration_parses_rollout_jsonl() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();

        let rollout = r#"{"type":"session_meta","payload":{"id":"sess-001","timestamp":"2026-04-01T10:00:00Z","cwd":"/tmp","agent_role":"autopilot"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"hello world"}}
{"type":"response_item","payload":{"type":"message","role":"assistant","content":"hi there"}}
"#;
        std::fs::write(sessions_dir.join("rollout-001.jsonl"), rollout).unwrap();

        let (file_count, sessions) = parse_rollout_files(&sessions_dir).unwrap();
        assert_eq!(file_count, 1);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "sess-001");
        assert_eq!(sessions[0].mode, "autopilot");
        assert_eq!(sessions[0].turns.len(), 2);
    }

    #[test]
    fn session_migration_builds_meta_json() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();
        std::fs::create_dir_all(dir.path().join(".omx").join("sessions")).unwrap();

        let rollout = r#"{"type":"session_meta","payload":{"id":"sess-002","timestamp":"2026-04-01T10:00:00Z","agent_role":"team"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"do thing"}}
"#;
        std::fs::write(sessions_dir.join("rollout-002.jsonl"), rollout).unwrap();

        let result = migrate_sessions(dir.path(), false, false).unwrap();
        assert_eq!(result.migrated_count(), 1);

        let meta_path = dir
            .path()
            .join(".omx")
            .join("sessions")
            .join("sess-002")
            .join("meta.json");
        assert!(meta_path.exists());

        let meta_str = std::fs::read_to_string(&meta_path).unwrap();
        let meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap();
        assert_eq!(meta["id"], "sess-002");
        assert_eq!(meta["mode"], "team");
        assert_eq!(meta["turns"], 1);
        assert_eq!(meta["status"], "completed");
    }

    #[test]
    fn session_migration_builds_transcript() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();
        std::fs::create_dir_all(dir.path().join(".omx").join("sessions")).unwrap();

        let rollout = r#"{"type":"session_meta","payload":{"id":"sess-003","timestamp":"2026-04-01T10:00:00Z","agent_role":"autopilot"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"first"}}
{"type":"response_item","payload":{"type":"message","role":"assistant","content":"second"}}
"#;
        std::fs::write(sessions_dir.join("rollout-003.jsonl"), rollout).unwrap();

        migrate_sessions(dir.path(), false, false).unwrap();

        let transcript_path = dir
            .path()
            .join(".omx")
            .join("sessions")
            .join("sess-003")
            .join("transcript.jsonl");
        assert!(transcript_path.exists());

        let contents = std::fs::read_to_string(&transcript_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);

        let turn1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(turn1["turn_number"], 1);
        assert_eq!(turn1["role"], "user");
        assert_eq!(turn1["content"], "first");

        let turn2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(turn2["turn_number"], 2);
        assert_eq!(turn2["role"], "assistant");
        assert_eq!(turn2["content"], "second");
    }

    #[test]
    fn session_migration_skips_existing() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();

        // Create existing Rust-era session
        let existing_dir = dir.path().join(".omx").join("sessions").join("sess-004");
        std::fs::create_dir_all(&existing_dir).unwrap();
        std::fs::write(existing_dir.join("meta.json"), "{}").unwrap();

        let rollout = r#"{"type":"session_meta","payload":{"id":"sess-004","timestamp":"2026-04-01T10:00:00Z","agent_role":"ralph"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"hello"}}
"#;
        std::fs::write(sessions_dir.join("rollout-004.jsonl"), rollout).unwrap();

        let result = migrate_sessions(dir.path(), false, false).unwrap();
        assert_eq!(result.skipped_count(), 1);
        assert_eq!(result.migrated_count(), 0);
    }

    #[test]
    fn session_migration_skips_malformed_lines() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();
        std::fs::create_dir_all(dir.path().join(".omx").join("sessions")).unwrap();

        let rollout = r#"{"type":"session_meta","payload":{"id":"sess-005","timestamp":"2026-04-01T10:00:00Z","agent_role":"autopilot"}}
this is not valid json at all
{"type":"response_item","payload":{"type":"message","role":"user","content":"works fine"}}
"#;
        std::fs::write(sessions_dir.join("rollout-005.jsonl"), rollout).unwrap();

        let result = migrate_sessions(dir.path(), false, false).unwrap();
        assert_eq!(result.migrated_count(), 1);
        assert_eq!(result.entries[0].turns, 1); // Only the valid response_item
    }

    #[test]
    fn session_migration_no_rollout_files() {
        let dir = TempDir::new().unwrap();
        let result = migrate_sessions(dir.path(), false, false).unwrap();
        assert_eq!(result.rollout_files, 0);
        assert!(result.entries.is_empty());
    }
}
```

- [ ] **Step 2: Run migrate unit tests**

Run: `cargo test -p omx-cli migrate -- --nocapture`
Expected: All 13 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/omx-cli/src/migrate.rs
git commit -m "feat(omx-cli): add config and session migration logic with tests"
```

---

## Task 3: Wire `omx migrate` Subcommand in `main.rs`

**Files:**
- Modify: `crates/omx-cli/src/main.rs`

- [ ] **Step 1: Add module declaration**

In `crates/omx-cli/src/main.rs`, after line 2 (`mod launch;`), add:

```rust
mod migrate;
```

- [ ] **Step 2: Add Migrate subcommand to clap enum**

In the `Commands` enum, after the `Reasoning` variant (around line 131), add:

```rust
    /// Migrate TS-era config and session data to Rust format
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },
```

- [ ] **Step 3: Add MigrateAction enum**

After the `HookApiAction` enum (around line 215), add:

```rust
#[derive(Debug, Subcommand)]
enum MigrateAction {
    /// Convert .omx-config.json to config.toml
    Config {
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing config.toml values
        #[arg(long)]
        force: bool,
    },
    /// Convert rollout-*.jsonl to per-session directories
    Sessions {
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing sessions
        #[arg(long)]
        force: bool,
    },
    /// Run both config and sessions migration
    All {
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing data
        #[arg(long)]
        force: bool,
    },
}
```

- [ ] **Step 4: Add migrate handler in main match block**

In the main `match cli.command` block, before the closing `Ok(())`, add:

```rust
        Some(Commands::Migrate { action }) => {
            let home = omx_config::default_codex_home();
            match action {
                MigrateAction::Config { dry_run, force } => {
                    match migrate::migrate_config(&home, dry_run, force) {
                        Ok(result) => migrate::print_config_result(&result, dry_run),
                        Err(e) => {
                            eprintln!("Config migration failed: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                MigrateAction::Sessions { dry_run, force } => {
                    match migrate::migrate_sessions(&home, dry_run, force) {
                        Ok(result) => migrate::print_sessions_result(&result, dry_run),
                        Err(e) => {
                            eprintln!("Session migration failed: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                MigrateAction::All { dry_run, force } => {
                    match migrate::migrate_config(&home, dry_run, force) {
                        Ok(result) => migrate::print_config_result(&result, dry_run),
                        Err(e) => {
                            eprintln!("Config migration failed: {e}");
                            std::process::exit(1);
                        }
                    }
                    println!();
                    match migrate::migrate_sessions(&home, dry_run, force) {
                        Ok(result) => migrate::print_sessions_result(&result, dry_run),
                        Err(e) => {
                            eprintln!("Session migration failed: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
```

- [ ] **Step 5: Build and verify**

Run: `cargo build -p omx-cli 2>&1 | tail -5`
Expected: Clean build

- [ ] **Step 6: Run all CLI tests**

Run: `cargo test -p omx-cli -- --nocapture`
Expected: All tests PASS (doctor + migrate + cleanup + launch)

- [ ] **Step 7: Commit**

```bash
git add crates/omx-cli/src/main.rs
git commit -m "feat(omx-cli): wire omx migrate subcommand with config, sessions, all actions"
```

---

## Task 4: Integration Tests for Migrate and Doctor

**Files:**
- Modify: `tests/src/cli.rs`

- [ ] **Step 1: Add integration tests for migrate and doctor**

In `tests/src/cli.rs`, append these tests inside the `mod tests` block, before the closing `}`:

```rust
    // ----- Migrate tests -----

    #[test]
    fn cli_migrate_config_dry_run() {
        let config = TestConfig::new();
        // Create a TS-era .omx-config.json
        std::fs::write(
            config.home_path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4", "team": "gpt-5.4-mini"}, "env": {"MY_VAR": "hello"}}"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "config", "--dry-run"])
            .output()
            .expect("failed to run omx migrate config --dry-run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("models.frontier") && stdout.contains("gpt-5.4"),
            "dry-run should show field mapping, got: {stdout}"
        );
        assert!(
            stdout.contains("dry-run"),
            "should indicate dry-run mode, got: {stdout}"
        );
        // Verify config.toml was NOT created
        assert!(
            !config.home_path().join("config.toml.migrated").exists(),
            "dry-run should not write files"
        );
    }

    #[test]
    fn cli_migrate_config_writes() {
        let config = TestConfig::new();
        std::fs::write(
            config.home_path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4"}}"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "config"])
            .output()
            .expect("failed to run omx migrate config");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Migrated"),
            "should report migration, got: {stdout}"
        );

        // Verify config.toml has the migrated value
        let toml_str =
            std::fs::read_to_string(config.home_path().join("config.toml")).unwrap();
        assert!(
            toml_str.contains("gpt-5.4"),
            "config.toml should contain migrated model, got: {toml_str}"
        );
    }

    #[test]
    fn cli_migrate_sessions_dry_run() {
        let config = TestConfig::new();
        let sessions_dir = config.home_path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();
        std::fs::write(
            sessions_dir.join("rollout-test.jsonl"),
            r#"{"type":"session_meta","payload":{"id":"s1","timestamp":"2026-04-01T10:00:00Z","agent_role":"autopilot"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"hello"}}
"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "sessions", "--dry-run"])
            .output()
            .expect("failed to run omx migrate sessions --dry-run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1 rollout") || stdout.contains("Found"),
            "should report rollout files found, got: {stdout}"
        );
        assert!(
            stdout.contains("dry-run"),
            "should indicate dry-run, got: {stdout}"
        );
    }

    #[test]
    fn cli_migrate_sessions_writes() {
        let config = TestConfig::new();
        let sessions_dir = config.home_path().join("sessions");
        std::fs::create_dir(&sessions_dir).unwrap();
        std::fs::write(
            sessions_dir.join("rollout-test.jsonl"),
            r#"{"type":"session_meta","payload":{"id":"s2","timestamp":"2026-04-01T10:00:00Z","agent_role":"team"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"world"}}
"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "sessions"])
            .output()
            .expect("failed to run omx migrate sessions");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Migrated"),
            "should report migration, got: {stdout}"
        );

        // Verify session directory was created
        let meta_path = config
            .home_path()
            .join(".omx")
            .join("sessions")
            .join("s2")
            .join("meta.json");
        assert!(meta_path.exists(), "session meta.json should exist");
    }

    // ----- Enhanced doctor tests -----

    #[test]
    fn cli_doctor_grouped_output() {
        let config = TestConfig::new();
        std::fs::write(
            config.home_path().join("config.toml"),
            "[models]\nfrontier = \"o3\"\n",
        )
        .unwrap();

        let output = omx_cmd(&config)
            .arg("doctor")
            .output()
            .expect("failed to run omx doctor");
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Check for grouped section headers
        assert!(
            stdout.contains("Dependencies"),
            "should have Dependencies section, got: {stdout}"
        );
        assert!(
            stdout.contains("MCP Servers"),
            "should have MCP Servers section, got: {stdout}"
        );
        assert!(
            stdout.contains("Notification Hooks"),
            "should have Notification Hooks section, got: {stdout}"
        );
        assert!(
            stdout.contains("Configuration"),
            "should have Configuration section, got: {stdout}"
        );
    }

    #[test]
    fn cli_doctor_detects_ts_era() {
        let config = TestConfig::new();
        std::fs::write(
            config.home_path().join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5"}}"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .arg("doctor")
            .output()
            .expect("failed to run omx doctor");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("TS Migration"),
            "should have TS Migration section, got: {stdout}"
        );
        assert!(
            stdout.contains(".omx-config.json") && stdout.contains("migrate"),
            "should suggest migration, got: {stdout}"
        );
    }
```

- [ ] **Step 2: Build integration test crate**

Run: `cargo build -p omx-integration-tests 2>&1 | tail -5`
Expected: Clean build

- [ ] **Step 3: Run new integration tests**

Run: `cargo test -p omx-integration-tests cli_migrate -- --nocapture`
Expected: All 4 migrate tests PASS

Run: `cargo test -p omx-integration-tests cli_doctor_grouped -- --nocapture`
Expected: PASS

Run: `cargo test -p omx-integration-tests cli_doctor_detects -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add tests/src/cli.rs
git commit -m "test(integration): add migrate and enhanced doctor integration tests"
```

---

## Task 5: Full Suite Verification

**Files:** None (verification only)

- [ ] **Step 1: Build entire workspace**

Run: `cargo build --workspace 2>&1 | tail -5`
Expected: Clean build, no errors

- [ ] **Step 2: Run all omx-cli tests**

Run: `cargo test -p omx-cli 2>&1 | tail -30`
Expected: All tests pass (doctor + migrate + cleanup + launch)

- [ ] **Step 3: Run all integration tests**

Run: `cargo test -p omx-integration-tests 2>&1 | tail -40`
Expected: All tests pass, including new migrate and doctor tests

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p omx-cli -- -D warnings 2>&1 | tail -10`
Expected: No warnings

- [ ] **Step 5: Run format check**

Run: `cargo fmt --all -- --check`
Expected: No issues (fix with `cargo fmt --all` if needed)

- [ ] **Step 6: Fix any issues, then commit if formatting was needed**

```bash
cargo fmt --all
git add crates/omx-cli/ tests/
git commit -m "style: apply cargo fmt to phase 12b migration tooling"
```

- [ ] **Step 7: Full workspace test**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: All workspace tests pass
