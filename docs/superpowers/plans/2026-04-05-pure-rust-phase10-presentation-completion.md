# Phase 10: Presentation Completion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the user-facing presentation layer by adding missing CLI commands, enhancing the HUD with live refresh and mode indicators, extending omx-config with feature flags / env / agents sections, and wiring omx-setup's prompt/skill copy + orphan detection.

**Architecture:** Bottom-up enhancements across 4 existing crates. `omx-config` gains new config sections first (other crates depend on it). `omx-hud` adds mode indicators, git context, presets, and live state refresh. `omx-setup` gains feature flags, orphan detection, and wired prompt/skill copy. `omx-cli` adds the 14 missing subcommands, the 3-phase launch sequence, and signal handler cleanup. Each task is independently testable.

**Tech Stack:** Rust, clap (CLI), ratatui/crossterm (HUD), toml/serde (config), tokio (async runtime)

---

## File Structure

### `omx-config` — Modified files

| File | Change |
|------|--------|
| `crates/omx-config/src/lib.rs` | Add `FeatureFlags`, `AgentsConfig`, env-per-mode section, pipeline config to `OmxConfig`; update TOML/JSON deserialization and `apply_*` functions |

### `omx-hud` — New and modified files

| File | Responsibility |
|------|---------------|
| `crates/omx-hud/src/lib.rs` | Split into module re-exports |
| `crates/omx-hud/src/state.rs` | `HudState` struct with new fields: mode, git context, token/quota metrics |
| `crates/omx-hud/src/mode_indicator.rs` | Mode-specific color and icon mapping for 8 modes |
| `crates/omx-hud/src/presets.rs` | `HudPreset` (Minimal/Standard/Verbose) layout configs |
| `crates/omx-hud/src/git_context.rs` | `GitContext` struct: branch, dirty, ahead/behind counts |
| `crates/omx-hud/src/renderer.rs` | `render_frame` refactored with preset-aware layout |
| `crates/omx-hud/src/runner.rs` | `run_hud` with live state refresh via file-watch polling |

### `omx-setup` — Modified files

| File | Change |
|------|--------|
| `crates/omx-setup/src/lib.rs` | Add `detect_orphaned_keys()`, wire `copy_prompts`/`copy_skills` with embedded asset list, feature-flag–aware config generation |

### `omx-cli` — New and modified files

| File | Responsibility |
|------|---------------|
| `crates/omx-cli/src/main.rs` | Add 14 new subcommands to `Commands` enum, 3-phase launch, signal cleanup |
| `crates/omx-cli/src/launch.rs` | 3-phase launch sequence: validate, inject AGENTS.md, start session |
| `crates/omx-cli/src/cleanup.rs` | Stale session/worktree/lock file cleanup logic |

---

## Task 1: Config — Feature Flags Section (`omx-config`)

**Files:**
- Modify: `crates/omx-config/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add to `crates/omx-config/src/lib.rs` tests module:

```rust
#[test]
fn feature_flags_default_all_false() {
    let flags = FeatureFlags::default();
    assert!(!flags.experimental_hud);
    assert!(!flags.experimental_pipeline);
    assert!(!flags.experimental_autoresearch);
}

#[test]
fn load_reads_feature_flags_from_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let toml_content = r#"
[features]
experimental_hud = true
experimental_pipeline = false
"#;
    fs::write(tmp.path().join("config.toml"), toml_content).unwrap();

    let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
    assert!(config.features.experimental_hud);
    assert!(!config.features.experimental_pipeline);
    assert!(!config.features.experimental_autoresearch);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-config feature_flags -- --nocapture 2>&1 | head -20`
Expected: FAIL — `FeatureFlags` not found

- [ ] **Step 3: Add `FeatureFlags` struct and wire into config**

Add the struct after the existing `TeamDefaults`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(default)]
    pub experimental_hud: bool,
    #[serde(default)]
    pub experimental_pipeline: bool,
    #[serde(default)]
    pub experimental_autoresearch: bool,
}
```

Add to `OmxConfig`:

```rust
pub features: FeatureFlags,
```

Add to `OmxConfig::default()`:

```rust
features: FeatureFlags::default(),
```

Add to `TomlConfigFile`:

```rust
#[serde(default)]
pub features: Option<FeatureFlags>,
```

Add to `JsonConfigFile`:

```rust
#[serde(default)]
pub features: Option<FeatureFlags>,
```

Add to `apply_toml`:

```rust
if let Some(ref features) = toml.features {
    config.features = features.clone();
}
```

Add to `apply_json`:

```rust
if let Some(ref features) = json.features {
    config.features = features.clone();
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-config -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-config/src/lib.rs
git commit -m "feat(omx-config): add feature flags section to OmxConfig"
```

---

## Task 2: Config — Agents Section (`omx-config`)

**Files:**
- Modify: `crates/omx-config/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn agents_config_default_is_empty() {
    let config = OmxConfig::default();
    assert!(config.agents.overrides.is_empty());
}

#[test]
fn load_reads_agents_section_from_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let toml_content = r#"
[agents.overrides.architect]
model = "o3"
description = "System design agent"

[agents.overrides.reviewer]
model = "o4-mini"
"#;
    fs::write(tmp.path().join("config.toml"), toml_content).unwrap();

    let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
    assert_eq!(config.agents.overrides.len(), 2);
    let architect = &config.agents.overrides["architect"];
    assert_eq!(architect.model.as_deref(), Some("o3"));
    assert_eq!(architect.description.as_deref(), Some("System design agent"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-config agents_config -- --nocapture 2>&1 | head -20`
Expected: FAIL — `agents` field not found on `OmxConfig`

- [ ] **Step 3: Add `AgentsConfig` and `AgentOverride` structs**

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentsConfig {
    #[serde(default)]
    pub overrides: HashMap<String, AgentOverride>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentOverride {
    pub model: Option<String>,
    pub description: Option<String>,
    pub tools: Option<Vec<String>>,
}
```

Add `pub agents: AgentsConfig` to `OmxConfig`, default it, add to `TomlConfigFile`/`JsonConfigFile` as `pub agents: Option<AgentsConfig>`, and wire `apply_toml`/`apply_json`:

```rust
if let Some(ref agents) = toml.agents {
    config.agents = agents.clone();
}
```

(Same pattern for `apply_json`.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-config -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-config/src/lib.rs
git commit -m "feat(omx-config): add agents override section to OmxConfig"
```

---

## Task 3: Config — Env-per-Mode Section (`omx-config`)

**Files:**
- Modify: `crates/omx-config/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn env_per_mode_default_is_empty() {
    let config = OmxConfig::default();
    assert!(config.env_per_mode.is_empty());
}

#[test]
fn load_reads_env_per_mode_from_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let toml_content = r#"
[env_per_mode.autopilot]
MAX_TURNS = "50"
VERBOSE = "true"

[env_per_mode.team]
MAX_TURNS = "100"
"#;
    fs::write(tmp.path().join("config.toml"), toml_content).unwrap();

    let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
    assert_eq!(config.env_per_mode.len(), 2);
    assert_eq!(config.env_per_mode["autopilot"]["MAX_TURNS"], "50");
    assert_eq!(config.env_per_mode["team"]["MAX_TURNS"], "100");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-config env_per_mode -- --nocapture 2>&1 | head -20`
Expected: FAIL — `env_per_mode` field not found

- [ ] **Step 3: Add `env_per_mode` field**

Add to `OmxConfig`:

```rust
pub env_per_mode: HashMap<String, HashMap<String, String>>,
```

Default it to `HashMap::new()`. Add to `TomlConfigFile`/`JsonConfigFile`:

```rust
#[serde(default)]
pub env_per_mode: Option<HashMap<String, HashMap<String, String>>>,
```

Wire in `apply_toml` and `apply_json`:

```rust
if let Some(ref env_per_mode) = toml.env_per_mode {
    config.env_per_mode = env_per_mode.clone();
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-config -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-config/src/lib.rs
git commit -m "feat(omx-config): add env-per-mode section to OmxConfig"
```

---

## Task 4: HUD — Mode Indicator (`omx-hud/src/mode_indicator.rs`)

**Files:**
- Create: `crates/omx-hud/src/mode_indicator.rs`
- Modify: `crates/omx-hud/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-hud/src/mode_indicator.rs`:

```rust
use ratatui::style::Color;

pub struct ModeIndicator {
    pub label: &'static str,
    pub icon: &'static str,
    pub color: Color,
}

pub fn indicator_for(mode_name: &str) -> ModeIndicator {
    match mode_name.to_lowercase().as_str() {
        "autopilot" => ModeIndicator { label: "AUTOPILOT", icon: "A", color: Color::Green },
        "autoresearch" => ModeIndicator { label: "AUTORESEARCH", icon: "R", color: Color::Blue },
        "deep-interview" | "deepinterview" => ModeIndicator { label: "INTERVIEW", icon: "I", color: Color::Magenta },
        "ralph" => ModeIndicator { label: "RALPH", icon: "P", color: Color::Yellow },
        "ultrawork" => ModeIndicator { label: "ULTRAWORK", icon: "U", color: Color::Cyan },
        "team" => ModeIndicator { label: "TEAM", icon: "T", color: Color::Red },
        "ultraqa" => ModeIndicator { label: "ULTRAQA", icon: "Q", color: Color::LightGreen },
        "ralplan" => ModeIndicator { label: "RALPLAN", icon: "L", color: Color::LightBlue },
        _ => ModeIndicator { label: "IDLE", icon: "-", color: Color::Gray },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_modes_return_distinct_indicators() {
        let modes = ["autopilot", "autoresearch", "deep-interview", "ralph", "ultrawork", "team", "ultraqa", "ralplan"];
        let indicators: Vec<_> = modes.iter().map(|m| indicator_for(m)).collect();
        // All labels are non-empty and distinct
        let labels: std::collections::HashSet<_> = indicators.iter().map(|i| i.label).collect();
        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn unknown_mode_returns_idle() {
        let ind = indicator_for("nonexistent");
        assert_eq!(ind.label, "IDLE");
    }

    #[test]
    fn case_insensitive_lookup() {
        let ind = indicator_for("Autopilot");
        assert_eq!(ind.label, "AUTOPILOT");
    }
}
```

- [ ] **Step 2: Add `pub mod mode_indicator;` to `crates/omx-hud/src/lib.rs`**

Add at the top of `lib.rs`:

```rust
pub mod mode_indicator;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omx-hud mode_indicator -- --nocapture`
Expected: All 3 tests PASS (implementation is already in the file)

- [ ] **Step 4: Commit**

```bash
git add crates/omx-hud/src/mode_indicator.rs crates/omx-hud/src/lib.rs
git commit -m "feat(omx-hud): add mode indicator with color and icon per mode"
```

---

## Task 5: HUD — Git Context (`omx-hud/src/git_context.rs`)

**Files:**
- Create: `crates/omx-hud/src/git_context.rs`
- Modify: `crates/omx-hud/src/lib.rs`

- [ ] **Step 1: Write the failing tests and implementation**

Create `crates/omx-hud/src/git_context.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Git repository context for HUD display.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitContext {
    pub branch: String,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
}

impl GitContext {
    /// Format as a compact status string for the HUD.
    pub fn status_line(&self) -> String {
        let dirty_marker = if self.dirty { "*" } else { "" };
        let sync = match (self.ahead, self.behind) {
            (0, 0) => String::new(),
            (a, 0) => format!(" +{a}"),
            (0, b) => format!(" -{b}"),
            (a, b) => format!(" +{a}/-{b}"),
        };
        format!("{}{}{}", self.branch, dirty_marker, sync)
    }
}

/// Build a `GitContext` by shelling out to `git`. Returns `None` if not in a git repo.
pub fn read_git_context() -> Option<GitContext> {
    let branch_output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !branch_output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();

    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    let dirty = !String::from_utf8_lossy(&status_output.stdout).trim().is_empty();

    let ahead_behind = std::process::Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<&str> = s.split('\t').collect();
                if parts.len() == 2 {
                    let ahead = parts[0].parse::<u32>().unwrap_or(0);
                    let behind = parts[1].parse::<u32>().unwrap_or(0);
                    return Some((ahead, behind));
                }
            }
            None
        })
        .unwrap_or((0, 0));

    Some(GitContext {
        branch,
        dirty,
        ahead: ahead_behind.0,
        behind: ahead_behind.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_line_clean_no_sync() {
        let ctx = GitContext { branch: "main".into(), dirty: false, ahead: 0, behind: 0 };
        assert_eq!(ctx.status_line(), "main");
    }

    #[test]
    fn status_line_dirty_with_ahead() {
        let ctx = GitContext { branch: "feat/x".into(), dirty: true, ahead: 3, behind: 0 };
        assert_eq!(ctx.status_line(), "feat/x* +3");
    }

    #[test]
    fn status_line_behind_only() {
        let ctx = GitContext { branch: "main".into(), dirty: false, ahead: 0, behind: 2 };
        assert_eq!(ctx.status_line(), "main -2");
    }

    #[test]
    fn status_line_ahead_and_behind() {
        let ctx = GitContext { branch: "dev".into(), dirty: true, ahead: 1, behind: 4 };
        assert_eq!(ctx.status_line(), "dev* +1/-4");
    }

    #[test]
    fn serde_roundtrip() {
        let ctx = GitContext { branch: "main".into(), dirty: true, ahead: 1, behind: 0 };
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: GitContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.branch, "main");
        assert!(parsed.dirty);
    }
}
```

- [ ] **Step 2: Add module declaration to `crates/omx-hud/src/lib.rs`**

```rust
pub mod git_context;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omx-hud git_context -- --nocapture`
Expected: All 5 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-hud/src/git_context.rs crates/omx-hud/src/lib.rs
git commit -m "feat(omx-hud): add git context (branch, dirty, ahead/behind) for HUD"
```

---

## Task 6: HUD — Presets (`omx-hud/src/presets.rs`)

**Files:**
- Create: `crates/omx-hud/src/presets.rs`
- Modify: `crates/omx-hud/src/lib.rs`

- [ ] **Step 1: Write the implementation and tests**

Create `crates/omx-hud/src/presets.rs`:

```rust
use serde::{Deserialize, Serialize};

/// HUD display density presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HudPreset {
    Minimal,
    Standard,
    Verbose,
}

impl Default for HudPreset {
    fn default() -> Self {
        Self::Standard
    }
}

/// Layout constraints for each preset — returns (header_height, stats_height, show_git, show_tokens).
pub fn preset_layout(preset: HudPreset) -> PresetLayout {
    match preset {
        HudPreset::Minimal => PresetLayout {
            header_height: 3,
            stats_height: 3,
            show_git: false,
            show_tokens: false,
        },
        HudPreset::Standard => PresetLayout {
            header_height: 3,
            stats_height: 5,
            show_git: true,
            show_tokens: false,
        },
        HudPreset::Verbose => PresetLayout {
            header_height: 3,
            stats_height: 7,
            show_git: true,
            show_tokens: true,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetLayout {
    pub header_height: u16,
    pub stats_height: u16,
    pub show_git: bool,
    pub show_tokens: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_is_standard() {
        assert_eq!(HudPreset::default(), HudPreset::Standard);
    }

    #[test]
    fn minimal_hides_git_and_tokens() {
        let layout = preset_layout(HudPreset::Minimal);
        assert!(!layout.show_git);
        assert!(!layout.show_tokens);
    }

    #[test]
    fn verbose_shows_everything() {
        let layout = preset_layout(HudPreset::Verbose);
        assert!(layout.show_git);
        assert!(layout.show_tokens);
    }

    #[test]
    fn standard_shows_git_not_tokens() {
        let layout = preset_layout(HudPreset::Standard);
        assert!(layout.show_git);
        assert!(!layout.show_tokens);
    }

    #[test]
    fn serde_roundtrip() {
        let preset = HudPreset::Verbose;
        let json = serde_json::to_string(&preset).unwrap();
        let parsed: HudPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, preset);
    }
}
```

- [ ] **Step 2: Add module declaration to `crates/omx-hud/src/lib.rs`**

```rust
pub mod presets;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omx-hud presets -- --nocapture`
Expected: All 5 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-hud/src/presets.rs crates/omx-hud/src/lib.rs
git commit -m "feat(omx-hud): add HUD presets (minimal/standard/verbose)"
```

---

## Task 7: HUD — Enhanced State with Mode, Git, Tokens (`omx-hud`)

**Files:**
- Modify: `crates/omx-hud/src/lib.rs`

This task extends the existing `HudState` struct with new fields for mode, git, and session metrics.

- [ ] **Step 1: Write the failing tests**

Add to the test module in `crates/omx-hud/src/lib.rs`:

```rust
#[test]
fn hud_state_with_mode_and_git() {
    let state = HudState {
        session_id: Some("sess-1".into()),
        provider: Some("codex".into()),
        model: Some("o3".into()),
        team_phase: Some(TeamPhase::Exec),
        worker_count: 3,
        pending_tasks: 5,
        completed_tasks: 2,
        uptime_seconds: 120,
        active_mode: Some("autopilot".into()),
        git_branch: Some("main".into()),
        git_dirty: false,
        turn_count: 42,
        tokens_used: 15000,
        preset: presets::HudPreset::Standard,
    };
    assert_eq!(state.active_mode.as_deref(), Some("autopilot"));
    assert_eq!(state.turn_count, 42);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-hud hud_state_with_mode -- --nocapture 2>&1 | head -20`
Expected: FAIL — `active_mode` field not found

- [ ] **Step 3: Add new fields to `HudState`**

Add these fields to the existing `HudState` struct:

```rust
pub active_mode: Option<String>,
pub git_branch: Option<String>,
pub git_dirty: bool,
pub turn_count: u64,
pub tokens_used: u64,
pub preset: crate::presets::HudPreset,
```

Update the `Default` derive — since `HudPreset` already implements `Default`, the derive should work. But `HudState` uses `#[derive(Default)]`, so `bool` and `u64` default to `false`/`0` and `Option` to `None`, which is correct.

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-hud -- --nocapture`
Expected: All tests PASS. Existing `render_frame_shows_header_and_stats` test needs updating — add the new fields with default values to that test's `HudState` construction:

```rust
active_mode: None,
git_branch: None,
git_dirty: false,
turn_count: 0,
tokens_used: 0,
preset: crate::presets::HudPreset::Standard,
```

Also update the `hud_state_serde_roundtrip` test similarly.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-hud/src/lib.rs
git commit -m "feat(omx-hud): extend HudState with mode, git, token, and preset fields"
```

---

## Task 8: HUD — Enhanced Renderer with Mode and Git Display (`omx-hud`)

**Files:**
- Modify: `crates/omx-hud/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn render_frame_shows_mode_indicator() {
    use ratatui::{backend::TestBackend, Terminal};

    let state = HudState {
        session_id: Some("sess-abc".into()),
        provider: Some("codex".into()),
        model: Some("o3".into()),
        team_phase: Some(TeamPhase::Exec),
        worker_count: 3,
        pending_tasks: 5,
        completed_tasks: 2,
        uptime_seconds: 120,
        active_mode: Some("autopilot".into()),
        git_branch: Some("main".into()),
        git_dirty: true,
        turn_count: 42,
        tokens_used: 15000,
        preset: presets::HudPreset::Standard,
    };

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| {
        render_frame(&state, frame, frame.area());
    }).unwrap();

    let buf = terminal.backend().buffer().clone();
    let text = (0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>())
        .collect::<Vec<_>>().join("\n");

    assert!(text.contains("AUTOPILOT"), "should show mode indicator");
    assert!(text.contains("main"), "should show git branch");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-hud render_frame_shows_mode -- --nocapture 2>&1 | head -30`
Expected: FAIL — "AUTOPILOT" not found in output

- [ ] **Step 3: Update `render_frame` to display mode and git context**

Replace the existing `render_frame` function in `crates/omx-hud/src/lib.rs`:

```rust
pub fn render_frame(state: &HudState, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    use ratatui::{
        layout::{Constraint, Layout},
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph},
    };

    let layout = presets::preset_layout(state.preset);

    let chunks = Layout::vertical([
        Constraint::Length(layout.header_height),
        Constraint::Length(layout.stats_height),
        Constraint::Min(0),
    ])
    .split(area);

    // Header with mode indicator
    let provider = state.provider.as_deref().unwrap_or("—");
    let model = state.model.as_deref().unwrap_or("—");
    let session = state.session_id.as_deref().unwrap_or("—");

    let mode_span = if let Some(ref mode) = state.active_mode {
        let ind = mode_indicator::indicator_for(mode);
        Span::styled(
            format!(" {} ", ind.label),
            Style::default().fg(Color::Black).bg(ind.color),
        )
    } else {
        Span::styled(" IDLE ", Style::default().fg(Color::Black).bg(Color::Gray))
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" OMX ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::raw(" "),
        mode_span,
        Span::raw(format!("  {provider} / {model}  session: {session}")),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    // Stats
    let phase_str = state
        .team_phase
        .as_ref()
        .map(|p| format!("{p:?}"))
        .unwrap_or_else(|| "Idle".into());
    let uptime_min = state.uptime_seconds / 60;
    let uptime_sec = state.uptime_seconds % 60;

    let mut lines = vec![
        Line::from(format!("Phase: {}  Workers: {}", phase_str, state.worker_count)),
        Line::from(format!("Tasks: {} pending / {} completed", state.pending_tasks, state.completed_tasks)),
    ];

    if layout.show_git {
        let branch = state.git_branch.as_deref().unwrap_or("—");
        let dirty = if state.git_dirty { "*" } else { "" };
        lines.push(Line::from(format!("Git: {branch}{dirty}")));
    }

    if layout.show_tokens {
        lines.push(Line::from(format!("Turns: {}  Tokens: {}", state.turn_count, state.tokens_used)));
    }

    lines.push(Line::from(format!("Uptime: {uptime_min}m {uptime_sec}s")));

    let stats = Paragraph::new(lines)
        .block(Block::default().title(" Status ").borders(Borders::ALL));
    frame.render_widget(stats, chunks[1]);
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-hud -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-hud/src/lib.rs
git commit -m "feat(omx-hud): render mode indicator and git context in HUD frame"
```

---

## Task 9: HUD — Live State Refresh (`omx-hud`)

**Files:**
- Modify: `crates/omx-hud/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn refresh_state_updates_git_context() {
    let mut state = HudState::default();
    assert!(state.git_branch.is_none());
    refresh_state(&mut state);
    // After refresh, git_branch should be populated if we're in a git repo
    // (tests run from within the repo, so this should be Some)
    assert!(state.git_branch.is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-hud refresh_state -- --nocapture 2>&1 | head -20`
Expected: FAIL — `refresh_state` not found

- [ ] **Step 3: Add `refresh_state` function**

Add to `crates/omx-hud/src/lib.rs`:

```rust
/// Refresh HudState with live data (git context).
pub fn refresh_state(state: &mut HudState) {
    if let Some(ctx) = git_context::read_git_context() {
        state.git_branch = Some(ctx.branch);
        state.git_dirty = ctx.dirty;
    }
}
```

- [ ] **Step 4: Update `run_hud` to call `refresh_state` each tick**

Replace the `run_hud` loop body to call refresh:

```rust
pub async fn run_hud(initial_state: HudState) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = ratatui::backend::CrosstermBackend::new(stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;
    let mut state = initial_state;

    loop {
        refresh_state(&mut state);

        terminal.draw(|frame| {
            render_frame(&state, frame, frame.area());
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p omx-hud -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/omx-hud/src/lib.rs
git commit -m "feat(omx-hud): add live state refresh with git context polling"
```

---

## Task 10: Setup — Orphaned Config Key Detection (`omx-setup`)

**Files:**
- Modify: `crates/omx-setup/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add to `crates/omx-setup/src/lib.rs` test module:

```rust
#[test]
fn detect_orphaned_keys_finds_unknown_sections() {
    let toml_content = r#"
model = "o3"

[models]
frontier = "o3"

[unknown_section]
key = "value"

[notifications.discord]
webhook_url = "https://example.com"

[also_unknown]
x = 1
"#;
    let orphans = detect_orphaned_keys(toml_content);
    assert_eq!(orphans.len(), 2);
    assert!(orphans.contains(&"unknown_section".to_string()));
    assert!(orphans.contains(&"also_unknown".to_string()));
}

#[test]
fn detect_orphaned_keys_no_false_positives() {
    let toml_content = r#"
model = "o3"

[models]
frontier = "o3"

[notifications.discord]
webhook_url = "https://example.com"

[team]
default_workers = 3

[features]
experimental_hud = true

[agents.overrides.architect]
model = "o3"

[env_per_mode.autopilot]
MAX_TURNS = "50"
"#;
    let orphans = detect_orphaned_keys(toml_content);
    assert!(orphans.is_empty(), "known sections should not be flagged: {orphans:?}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-setup detect_orphaned -- --nocapture 2>&1 | head -20`
Expected: FAIL — `detect_orphaned_keys` not found

- [ ] **Step 3: Implement `detect_orphaned_keys`**

Add to `crates/omx-setup/src/lib.rs`:

```rust
/// Known top-level config sections that are valid in config.toml.
const KNOWN_SECTIONS: &[&str] = &[
    "model",
    "model_reasoning_effort",
    "models",
    "notifications",
    "team",
    "features",
    "agents",
    "env_per_mode",
    "mcp_servers",
];

/// Scan TOML content for top-level keys that are not in the known set.
/// Returns the names of unknown sections.
pub fn detect_orphaned_keys(toml_content: &str) -> Vec<String> {
    let table: toml::Table = match toml::from_str(toml_content) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    table
        .keys()
        .filter(|key| !KNOWN_SECTIONS.contains(&key.as_str()))
        .cloned()
        .collect()
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-setup -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-setup/src/lib.rs
git commit -m "feat(omx-setup): add orphaned config key detection"
```

---

## Task 11: Setup — Wire `copy_prompts` and `copy_skills` (`omx-setup`)

**Files:**
- Modify: `crates/omx-setup/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn copy_prompts_creates_prompts_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    let gen = DefaultSetupGenerator;
    gen.copy_prompts(SetupScope::User).unwrap();
    let prompts_dir = dir.path().join(".codex").join("prompts");
    assert!(prompts_dir.exists(), "prompts dir should be created");
}

#[test]
fn copy_skills_creates_skills_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    let gen = DefaultSetupGenerator;
    gen.copy_skills(SetupScope::User).unwrap();
    let skills_dir = dir.path().join(".codex").join("skills");
    assert!(skills_dir.exists(), "skills dir should be created");
}

#[test]
fn embedded_prompts_list_is_not_empty() {
    let prompts = embedded_prompt_names();
    assert!(!prompts.is_empty(), "should have at least one embedded prompt");
}

#[test]
fn embedded_skills_list_is_not_empty() {
    let skills = embedded_skill_names();
    assert!(!skills.is_empty(), "should have at least one embedded skill");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-setup copy_prompts_creates -- --nocapture 2>&1 | head -20`
Expected: FAIL — prompts dir not created (stub impl)

- [ ] **Step 3: Implement embedded asset lists and wire `copy_prompts`/`copy_skills`**

Add to `crates/omx-setup/src/lib.rs`:

```rust
/// Names of embedded prompt templates shipped with OMX.
pub fn embedded_prompt_names() -> Vec<&'static str> {
    vec![
        "executor",
        "planner",
        "verifier",
        "architect",
        "reviewer",
    ]
}

/// Names of embedded skill definitions shipped with OMX.
pub fn embedded_skill_names() -> Vec<&'static str> {
    vec![
        "explore",
        "sparkshell",
        "deep-interview",
    ]
}
```

Update `copy_prompts` in `DefaultSetupGenerator`:

```rust
fn copy_prompts(&self, scope: SetupScope) -> Result<(), OmxError> {
    let base = match scope {
        SetupScope::User => omx_config::default_codex_home().join("prompts"),
        SetupScope::Project => std::env::current_dir()
            .map_err(OmxError::Io)?
            .join(".omx")
            .join("prompts"),
    };
    std::fs::create_dir_all(&base).map_err(OmxError::Io)?;
    for name in embedded_prompt_names() {
        let path = base.join(format!("{name}.md"));
        if !path.exists() {
            std::fs::write(&path, format!("# {name}\n\n> Placeholder prompt for {name} agent.\n"))
                .map_err(OmxError::Io)?;
        }
    }
    tracing::info!("copy_prompts: wrote {} prompts to {}", embedded_prompt_names().len(), base.display());
    Ok(())
}
```

Update `copy_skills` similarly:

```rust
fn copy_skills(&self, scope: SetupScope) -> Result<(), OmxError> {
    let base = match scope {
        SetupScope::User => omx_config::default_codex_home().join("skills"),
        SetupScope::Project => std::env::current_dir()
            .map_err(OmxError::Io)?
            .join(".omx")
            .join("skills"),
    };
    std::fs::create_dir_all(&base).map_err(OmxError::Io)?;
    for name in embedded_skill_names() {
        let path = base.join(format!("{name}.md"));
        if !path.exists() {
            std::fs::write(&path, format!("# {name}\n\n> Placeholder skill definition for {name}.\n"))
                .map_err(OmxError::Io)?;
        }
    }
    tracing::info!("copy_skills: wrote {} skills to {}", embedded_skill_names().len(), base.display());
    Ok(())
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-setup -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-setup/src/lib.rs
git commit -m "feat(omx-setup): wire copy_prompts and copy_skills with embedded asset lists"
```

---

## Task 12: CLI — Add Missing Subcommands to `Commands` Enum (`omx-cli`)

**Files:**
- Modify: `crates/omx-cli/src/main.rs`

This task adds the 14 missing subcommand variants to the `Commands` enum and `Cargo.toml` deps. The handler bodies are stubs that print a message — wiring comes in later tasks.

- [ ] **Step 1: Add new dependencies to `crates/omx-cli/Cargo.toml`**

Add these to `[dependencies]`:

```toml
omx-session = { path = "../omx-session" }
omx-modes = { path = "../omx-modes" }
omx-pipeline = { path = "../omx-pipeline" }
```

- [ ] **Step 2: Add new variants to `Commands` enum**

Add these variants after the existing `HookApi` variant in `crates/omx-cli/src/main.rs`:

```rust
/// Run a single agent task (non-team)
Exec {
    /// Agent name
    #[arg(long)]
    agent: String,
    /// Task prompt
    prompt: String,
},

/// List available agent definitions
Agents,

/// Scaffold agent config files
AgentsInit,

/// Remove OMX configuration and artifacts
Uninstall,

/// Remove stale sessions, worktrees, lock files
Cleanup,

/// List/inspect session history
Session {
    #[command(subcommand)]
    action: Option<SessionAction>,
},

/// Restore a previous session
Resume {
    /// Session ID to resume
    session_id: String,
},

/// Start/resume Ralph persistent workflow
Ralph {
    /// Task description
    prompt: Option<String>,
},

/// Start autoresearch loop
Autoresearch {
    /// Research topic
    prompt: String,
},

/// Start consensus planning session
Ralplan {
    /// Planning topic
    prompt: String,
},

/// Run a named pipeline
Pipeline {
    /// Pipeline name
    name: String,
},

/// Invoked by tmux hooks (resize, pane close)
TmuxHook {
    /// Hook event name
    event: String,
    /// Target pane
    #[arg(long)]
    target: Option<String>,
},

/// Show current mode, active team, session metrics
Status,

/// Display/configure model reasoning settings
Reasoning {
    /// Set reasoning effort (low, medium, high)
    #[arg(long)]
    effort: Option<String>,
},
```

Add the `SessionAction` enum:

```rust
#[derive(Debug, Subcommand)]
enum SessionAction {
    /// List recent sessions
    List,
    /// Inspect a specific session
    Show { session_id: String },
}
```

- [ ] **Step 3: Add stub match arms**

Add to the main `match cli.command` block:

```rust
Some(Commands::Exec { agent, prompt }) => {
    println!("exec: agent={agent}, prompt={prompt}");
    println!("(not yet wired)");
}
Some(Commands::Agents) => {
    println!("Available agents:");
    println!("  (agent listing not yet wired)");
}
Some(Commands::AgentsInit) => {
    println!("Scaffolding agent config files...");
    println!("  (not yet wired)");
}
Some(Commands::Uninstall) => {
    println!("Uninstalling OMX...");
    println!("  (not yet wired — would remove ~/.codex/.omx)");
}
Some(Commands::Cleanup) => {
    println!("Cleaning up stale sessions, worktrees, lock files...");
    println!("  (not yet wired)");
}
Some(Commands::Session { action }) => {
    match action {
        Some(SessionAction::List) | None => {
            println!("Recent sessions:");
            println!("  (session listing not yet wired)");
        }
        Some(SessionAction::Show { session_id }) => {
            println!("Session: {session_id}");
            println!("  (session detail not yet wired)");
        }
    }
}
Some(Commands::Resume { session_id }) => {
    println!("Resuming session {session_id}...");
    println!("  (not yet wired)");
}
Some(Commands::Ralph { prompt }) => {
    let desc = prompt.as_deref().unwrap_or("(interactive)");
    println!("Starting Ralph workflow: {desc}");
    println!("  (not yet wired)");
}
Some(Commands::Autoresearch { prompt }) => {
    println!("Starting autoresearch: {prompt}");
    println!("  (not yet wired)");
}
Some(Commands::Ralplan { prompt }) => {
    println!("Starting consensus planning: {prompt}");
    println!("  (not yet wired)");
}
Some(Commands::Pipeline { name }) => {
    println!("Running pipeline: {name}");
    println!("  (not yet wired)");
}
Some(Commands::TmuxHook { event, target }) => {
    let tgt = target.as_deref().unwrap_or("(none)");
    println!("tmux-hook: event={event}, target={tgt}");
}
Some(Commands::Status) => {
    let home = omx_config::default_codex_home();
    let env: std::collections::HashMap<String, String> = std::env::vars().collect();
    let config = omx_config::DefaultConfigLoader::load(&home, &env)
        .unwrap_or_else(|_| omx_config::OmxConfig::default());
    println!("OMX Status");
    println!("  Model: {}", config.models.frontier);
    println!("  Mode: (idle)");
    println!("  Session: (none active)");
}
Some(Commands::Reasoning { effort }) => {
    match effort {
        Some(e) => println!("Reasoning effort set to: {e}"),
        None => println!("Current reasoning effort: (default)"),
    }
}
```

- [ ] **Step 4: Run build**

Run: `cargo build -p omx-cli 2>&1 | tail -10`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add crates/omx-cli/Cargo.toml crates/omx-cli/src/main.rs
git commit -m "feat(omx-cli): add 14 missing subcommand stubs (exec, agents, cleanup, status, etc.)"
```

---

## Task 13: CLI — Launch Sequence Module (`omx-cli/src/launch.rs`)

**Files:**
- Create: `crates/omx-cli/src/launch.rs`
- Modify: `crates/omx-cli/src/main.rs`

- [ ] **Step 1: Create `crates/omx-cli/src/launch.rs`**

```rust
use omx_config::OmxConfig;
use std::path::{Path, PathBuf};

/// Phase 1: Validate config — load and validate TOML, check required fields.
pub fn validate_config(config: &OmxConfig) -> Vec<String> {
    let mut warnings = Vec::new();

    if config.models.frontier.is_empty() {
        warnings.push("models.frontier is empty — defaulting to 'o3'".into());
    }

    if config.team.default_workers == 0 {
        warnings.push("team.default_workers is 0 — no workers will be spawned".into());
    }

    warnings
}

/// Phase 2: Inject AGENTS.md overlay — merge OMX agent instructions into project's AGENTS.md.
/// Uses OMX:START/OMX:END markers for non-destructive replacement.
pub fn inject_agents_overlay(agents_md_path: &Path, overlay_content: &str) -> Result<(), std::io::Error> {
    let existing = if agents_md_path.exists() {
        std::fs::read_to_string(agents_md_path)?
    } else {
        String::new()
    };

    let start_marker = "<!-- OMX:START -->";
    let end_marker = "<!-- OMX:END -->";
    let section = format!("{start_marker}\n{overlay_content}\n{end_marker}");

    let new_content = if existing.contains(start_marker) && existing.contains(end_marker) {
        let start = existing.find(start_marker).unwrap();
        let end = existing.find(end_marker).unwrap() + end_marker.len();
        let end = if existing[end..].starts_with('\n') { end + 1 } else { end };
        format!("{}{}\n{}", &existing[..start], section, &existing[end..])
    } else if existing.is_empty() {
        section
    } else {
        format!("{}\n\n{}\n", existing.trim_end(), section)
    };

    if let Some(parent) = agents_md_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(agents_md_path, new_content)
}

/// Phase 3 helper: determine the lock file path for the current session.
pub fn session_lock_path(codex_home: &Path) -> PathBuf {
    codex_home.join(".omx").join("session.lock")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_config_warns_on_empty_frontier() {
        let mut config = OmxConfig::default();
        config.models.frontier = String::new();
        let warnings = validate_config(&config);
        assert!(warnings.iter().any(|w| w.contains("frontier")));
    }

    #[test]
    fn validate_config_warns_on_zero_workers() {
        let mut config = OmxConfig::default();
        config.team.default_workers = 0;
        let warnings = validate_config(&config);
        assert!(warnings.iter().any(|w| w.contains("workers")));
    }

    #[test]
    fn validate_config_no_warnings_for_defaults() {
        let config = OmxConfig::default();
        let warnings = validate_config(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn inject_agents_overlay_creates_new_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("AGENTS.md");
        inject_agents_overlay(&path, "## OMX Agents\nHello").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<!-- OMX:START -->"));
        assert!(content.contains("## OMX Agents"));
        assert!(content.contains("<!-- OMX:END -->"));
    }

    #[test]
    fn inject_agents_overlay_replaces_existing_section() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "# My Agents\n\n<!-- OMX:START -->\nold\n<!-- OMX:END -->\n\n# Custom").unwrap();
        inject_agents_overlay(&path, "new content").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("new content"));
        assert!(!content.contains("old"));
        assert!(content.contains("# My Agents"));
        assert!(content.contains("# Custom"));
    }

    #[test]
    fn inject_agents_overlay_appends_to_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "# Existing content\n").unwrap();
        inject_agents_overlay(&path, "injected").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Existing content"));
        assert!(content.contains("injected"));
        assert!(content.contains("<!-- OMX:START -->"));
    }

    #[test]
    fn session_lock_path_correct() {
        let path = session_lock_path(Path::new("/home/user/.codex"));
        assert_eq!(path, PathBuf::from("/home/user/.codex/.omx/session.lock"));
    }
}
```

- [ ] **Step 2: Convert `omx-cli` from single-file binary to module structure**

Add `mod launch;` to the top of `crates/omx-cli/src/main.rs`:

```rust
mod launch;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omx-cli -- --nocapture`
Expected: All 6 launch tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-cli/src/launch.rs crates/omx-cli/src/main.rs
git commit -m "feat(omx-cli): add 3-phase launch sequence module with AGENTS.md injection"
```

---

## Task 14: CLI — Cleanup Module (`omx-cli/src/cleanup.rs`)

**Files:**
- Create: `crates/omx-cli/src/cleanup.rs`
- Modify: `crates/omx-cli/src/main.rs`

- [ ] **Step 1: Create `crates/omx-cli/src/cleanup.rs`**

```rust
use std::path::Path;

/// Result of a cleanup operation.
#[derive(Debug, Default)]
pub struct CleanupReport {
    pub stale_locks_removed: u32,
    pub stale_sessions_removed: u32,
    pub stale_worktrees_removed: u32,
}

impl CleanupReport {
    pub fn total(&self) -> u32 {
        self.stale_locks_removed + self.stale_sessions_removed + self.stale_worktrees_removed
    }
}

/// Scan `omx_dir` for stale lock files (*.lock) and remove them.
pub fn remove_stale_locks(omx_dir: &Path) -> Result<u32, std::io::Error> {
    let mut count = 0;
    if !omx_dir.exists() {
        return Ok(0);
    }
    for entry in std::fs::read_dir(omx_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("lock") {
            std::fs::remove_file(&path)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Scan `sessions_dir` for session directories older than `max_age_days`.
pub fn remove_stale_sessions(sessions_dir: &Path, max_age_days: u64) -> Result<u32, std::io::Error> {
    let mut count = 0;
    if !sessions_dir.exists() {
        return Ok(0);
    }
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_days * 86400))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    for entry in std::fs::read_dir(sessions_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    std::fs::remove_dir_all(entry.path())?;
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

/// Run full cleanup: locks, sessions, worktrees.
pub fn run_cleanup(codex_home: &Path) -> Result<CleanupReport, std::io::Error> {
    let omx_dir = codex_home.join(".omx");
    let sessions_dir = omx_dir.join("sessions");

    let stale_locks_removed = remove_stale_locks(&omx_dir)?;
    let stale_sessions_removed = remove_stale_sessions(&sessions_dir, 30)?;

    Ok(CleanupReport {
        stale_locks_removed,
        stale_sessions_removed,
        stale_worktrees_removed: 0, // worktree cleanup deferred to git integration
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn remove_stale_locks_removes_lock_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("session.lock"), "").unwrap();
        std::fs::write(dir.path().join("worker.lock"), "").unwrap();
        std::fs::write(dir.path().join("data.json"), "{}").unwrap();

        let count = remove_stale_locks(dir.path()).unwrap();
        assert_eq!(count, 2);
        assert!(!dir.path().join("session.lock").exists());
        assert!(!dir.path().join("worker.lock").exists());
        assert!(dir.path().join("data.json").exists()); // not a lock
    }

    #[test]
    fn remove_stale_locks_nonexistent_dir() {
        let count = remove_stale_locks(Path::new("/nonexistent/path")).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn remove_stale_sessions_removes_old_dirs() {
        let dir = TempDir::new().unwrap();
        let old_session = dir.path().join("sess-old");
        std::fs::create_dir(&old_session).unwrap();
        // Set modification time to 60 days ago by creating a file and using filetime
        // For simplicity in tests, we test with max_age_days=0 to catch everything
        let count = remove_stale_sessions(dir.path(), 0).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn cleanup_report_total() {
        let report = CleanupReport {
            stale_locks_removed: 2,
            stale_sessions_removed: 1,
            stale_worktrees_removed: 0,
        };
        assert_eq!(report.total(), 3);
    }

    #[test]
    fn run_cleanup_on_empty_dir() {
        let dir = TempDir::new().unwrap();
        let report = run_cleanup(dir.path()).unwrap();
        assert_eq!(report.total(), 0);
    }
}
```

- [ ] **Step 2: Add `mod cleanup;` to `crates/omx-cli/src/main.rs`**

```rust
mod cleanup;
```

- [ ] **Step 3: Wire the `Cleanup` command handler**

Replace the `Cleanup` stub in main.rs:

```rust
Some(Commands::Cleanup) => {
    let home = omx_config::default_codex_home();
    match cleanup::run_cleanup(&home) {
        Ok(report) => {
            println!("Cleanup complete:");
            println!("  {} lock files removed", report.stale_locks_removed);
            println!("  {} stale sessions removed", report.stale_sessions_removed);
            println!("  {} stale worktrees removed", report.stale_worktrees_removed);
        }
        Err(e) => eprintln!("Cleanup failed: {e}"),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-cli -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-cli/src/cleanup.rs crates/omx-cli/src/main.rs
git commit -m "feat(omx-cli): add cleanup module for stale locks and sessions"
```

---

## Task 15: CLI — Wire Launch Sequence into Default Command (`omx-cli`)

**Files:**
- Modify: `crates/omx-cli/src/main.rs`

- [ ] **Step 1: Update the `None` (default) command handler**

Replace the `None =>` match arm in `main.rs` with:

```rust
None => {
    let home = omx_config::default_codex_home();
    let env: std::collections::HashMap<String, String> = std::env::vars().collect();
    let config = omx_config::DefaultConfigLoader::load(&home, &env)
        .unwrap_or_else(|_| omx_config::OmxConfig::default());

    // Phase 1: Validate config
    let warnings = launch::validate_config(&config);
    for w in &warnings {
        eprintln!("warning: {w}");
    }

    // Phase 2: Inject AGENTS.md overlay
    let agents_path = home.join("AGENTS.md");
    let gen = omx_setup::DefaultSetupGenerator;
    use omx_setup::SetupGenerator;
    if let Ok(overlay) = gen.generate_agents_md(&config) {
        if let Err(e) = launch::inject_agents_overlay(&agents_path, &overlay) {
            eprintln!("warning: failed to inject AGENTS.md: {e}");
        }
    }

    // Phase 3: Start session — launch provider
    let provider = cli.provider.as_deref().unwrap_or("codex");
    let model = cli.model.as_deref().unwrap_or(&config.models.frontier);
    let bin = match provider {
        "codex" => "codex",
        "claude" => "claude",
        other => {
            eprintln!("Unknown provider: {other}. Supported: codex, claude");
            std::process::exit(1);
        }
    };
    let args: Vec<String> = vec!["--model".into(), model.into()];
    tracing::info!("Launching {bin} with model {model}");
    let status = std::process::Command::new(bin)
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
```

- [ ] **Step 2: Run build**

Run: `cargo build -p omx-cli 2>&1 | tail -5`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add crates/omx-cli/src/main.rs
git commit -m "feat(omx-cli): wire 3-phase launch sequence into default command"
```

---

## Task 16: Full Workspace Build Verification

**Files:** None (verification only)

- [ ] **Step 1: Full workspace build**

Run: `cargo build --workspace 2>&1 | tail -5`
Expected: Clean build, no errors

- [ ] **Step 2: Full workspace tests**

Run: `cargo test --workspace 2>&1 | tail -40`
Expected: All tests pass, 0 failures

- [ ] **Step 3: Clippy**

Run: `cargo clippy --workspace -- -D warnings 2>&1 | tail -10`
Expected: No warnings

- [ ] **Step 4: Format check**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues
