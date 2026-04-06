# Phase 12b: Migration Tooling — Design Specification

**Date:** 2026-04-06
**Status:** Draft
**Author:** Artur Ciocanu + Claude Opus 4.6
**Predecessor:** [Phase 12a Integration Tests](2026-04-05-pure-rust-phase12a-integration-tests-design.md)

---

## 1. Context

Phase 12a delivered a 50-test integration test suite covering CLI commands, MCP servers, notifications, hooks, and mode lifecycle. Before we can delete the TypeScript codebase (Phase 12c) and release v1.0.0 (Phase 12d), users need a migration path from the TS-era configuration and session data to the Rust-era formats.

### Current State

- **TypeScript:** 412 files, ~123K lines still present in `src/`
- **Rust:** 32 crates, 96 files, ~23K lines across `crates/`
- **TS-era config:** `.omx-config.json` (JSON, mode-based model mapping)
- **Rust-era config:** `config.toml` (TOML, tiered model config with `frontier`/`standard`/`spark`)
- **TS-era sessions:** `~/.codex/sessions/rollout-*.jsonl` (rolling files, multiple sessions per file)
- **Rust-era sessions:** `~/.codex/.omx/sessions/{id}/meta.json` + `transcript.jsonl` (per-session dirs)
- **`omx doctor`:** 9 checks (tmux, codex, claude, 5 MCP binaries, config.toml existence)

---

## 2. Goals

1. Provide an `omx migrate` command that converts TS-era config and session data to Rust-era formats
2. Enhance `omx doctor` from 9 to 16 checks with grouped output, notification binary checks, config schema validation, hook validation, and TS migration hints
3. Extract doctor and migration logic into separate modules to keep `main.rs` as a thin router
4. Add integration tests for the new commands

### Non-Goals

- Wiring the 11 stub CLI commands to their crates (separate effort)
- Deleting TypeScript files (Phase 12c)
- Release tooling (Phase 12d)
- Backward compatibility with TS-era runtime — this is a one-way migration

---

## 3. Architecture

### 3.1 Approach

**Migration logic inside `omx-cli` + enhanced `omx doctor`.** No new crate — migration is temporary code that will be removed after users have transitioned. Keeping it in the CLI crate makes it visible and easy to remove.

Alternatives considered:
- **Separate `omx-migrate` crate:** Rejected — adds a whole crate for throwaway code
- **Migration inside `omx setup`:** Rejected — mixing setup concerns with migration, harder to test/debug

### 3.2 File Structure

| File | Responsibility |
|------|---------------|
| `crates/omx-cli/src/main.rs` | Add `Migrate` subcommand to clap enum, delegate to modules |
| `crates/omx-cli/src/migrate.rs` | Config migration, session migration, dry-run/force handling |
| `crates/omx-cli/src/doctor.rs` | Extracted and enhanced doctor logic with grouped output |

### 3.3 Dependencies

No new crate dependencies. `omx-cli` already depends on `omx-config`, `omx-state`, `omx-types`, `serde_json`, and `toml`.

---

## 4. `omx migrate` Subcommand

### 4.1 Command Surface

```
omx migrate
├── omx migrate config       # Convert TS-era .omx-config.json to config.toml
├── omx migrate sessions     # Convert TS-era rollout-*.jsonl to per-session dirs
└── omx migrate all          # Run both config + sessions
```

All subcommands support:
- `--dry-run` — show what would change without writing
- `--force` — overwrite existing Rust-era files

### 4.2 Config Migration

**Input:** `{codex_home}/.omx-config.json`

```json
{
  "env": { "KEY": "value" },
  "models": { "default": "gpt-5.4", "team": "gpt-5.4-mini", "autoresearch": "o3" }
}
```

**Output:** Merged into `{codex_home}/config.toml`

**Field mapping:**

| TS-era (JSON) | Rust-era (TOML) |
|----------------|-----------------|
| `models.default` | `models.frontier` |
| `models.{mode}` (any other key) | `models.per_mode.{mode}` |
| `env.{KEY}` | `env.{KEY}` |

**Rules:**
- If `config.toml` already has a value for a field, skip it (don't overwrite) unless `--force`
- If `.omx-config.json` doesn't exist, print "No TS-era config found, nothing to migrate" and exit 0
- `--dry-run` prints the would-be changes without writing
- Preserves existing `config.toml` content — only adds/fills missing fields
- Uses OMX marker comments (`# OMX:MIGRATED:START` / `# OMX:MIGRATED:END`) around migrated sections so they're identifiable

**Output format:**
```
omx migrate config

  Source: ~/.codex/.omx-config.json
  Target: ~/.codex/config.toml

  models.frontier = "gpt-5.4"           (from models.default)
  models.per_mode.team = "gpt-5.4-mini" (from models.team)
  models.per_mode.autoresearch = "o3"   (from models.autoresearch)
  env.KEY = "value"                      (from env.KEY)

  Migrated 4 fields (0 skipped, 0 conflicts)
```

### 4.3 Session Migration

**Input:** `{codex_home}/sessions/rollout-*.jsonl`

Each rollout file contains interleaved JSONL records from multiple sessions. Record types include `session_meta`, `event_msg`, and `response_item`.

**Output:** `{codex_home}/.omx/sessions/{id}/meta.json` + `transcript.jsonl`

**Steps per rollout file:**
1. Read file line by line (streaming, not loading entire file)
2. Parse each JSON line, extract `sessionId` field
3. Group lines by session ID
4. For each session group:
   - Extract `session_meta` record → build `meta.json`:
     - `id`: original session ID
     - `started_at`: from `timestamp` field (or file mtime as fallback)
     - `mode`: from `agent_role` field if present, default `"unknown"`
     - `turns`: count of `response_item` records
     - `tokens_used`: 0 (not tracked in TS-era format)
     - `status`: `Completed` (if session has data, assume it completed)
   - Extract `response_item` records → build `transcript.jsonl` turns:
     - `turn_number`: sequential index
     - `timestamp`: from record timestamp or parent session timestamp
     - `role`: from `item.role` or inferred from record type
     - `content`: from `item.content` (text extracted from nested payload)
     - `tokens`: 0 (not tracked per-turn in TS-era format)

**Rules:**
- Skip sessions that already exist in Rust-era directory (unless `--force`)
- `--dry-run` prints count of sessions found, would-be-created, would-be-skipped
- Malformed JSONL lines are logged and skipped (don't abort the whole migration)
- Report summary at end

**Output format:**
```
omx migrate sessions

  Scanning: ~/.codex/sessions/rollout-*.jsonl
  Found: 3 rollout files, 47 sessions

  Migrating sessions...
    ✓ sess-abc123  (12 turns)
    ✓ sess-def456  (8 turns)
    - sess-ghi789  (already exists, skipped)
    ...

  Migrated 45 sessions (2 skipped, 0 failed)
```

### 4.4 `omx migrate all`

Runs config migration first, then session migration. Reports both summaries. Passes through `--dry-run` and `--force` flags to both.

---

## 5. Enhanced `omx doctor`

### 5.1 Current Checks (9)

tmux, codex, claude, 5 MCP binaries, config.toml existence

### 5.2 New Checks (7 additional → 16 total)

| Category | Check | Pass | Fail |
|----------|-------|------|------|
| **Notification binaries** | `omx-notify-discord` on PATH | `ok` | `MISSING` |
| | `omx-notify-slack` on PATH | `ok` | `MISSING` |
| | `omx-notify-telegram` on PATH | `ok` | `MISSING` |
| | `omx-notify-pushover` on PATH | `ok` | `MISSING` |
| | `omx-notify-generic` on PATH | `ok` | `MISSING` |
| **Config validation** | `config.toml` parses as valid `OmxConfig` | `ok` | `INVALID: {reason}` |
| **Hooks** | `.omx/hooks/` — hooks found and executable | `ok` / `no hooks` | `WARNING: non-executable hooks` |

### 5.3 Output Format (grouped)

```
omx doctor — checking installation

  Dependencies
    ok      tmux
    ok      codex
    MISSING claude

  MCP Servers
    ok      omx-mcp-state
    ok      omx-mcp-memory
    ok      omx-mcp-code-intel
    ok      omx-mcp-trace
    ok      omx-mcp-team

  Notification Hooks
    ok      omx-notify-discord
    ok      omx-notify-slack
    MISSING omx-notify-telegram
    ok      omx-notify-pushover
    ok      omx-notify-generic

  Configuration
    ok      config.toml exists
    ok      config.toml schema valid

  Hooks
    ok      3 hooks found, all executable

  TS Migration
    ok      No TS-era config files detected
```

If TS-era files are detected:
```
  TS Migration
    WARNING .omx-config.json found — run `omx migrate config`
    WARNING rollout-*.jsonl found (12 files) — run `omx migrate sessions`
```

### 5.4 Exit Code

- `0` if all checks pass (warnings don't affect exit code)
- `1` if any `MISSING` or `INVALID` check fails

### 5.5 Module Extraction

The current doctor logic (~45 lines in `main.rs`) is extracted to `doctor.rs` as a `run_doctor()` function. This keeps `main.rs` as a thin clap router. The grouped output uses a `CheckGroup` struct:

```rust
struct CheckGroup {
    name: &'static str,
    checks: Vec<CheckResult>,
}

struct CheckResult {
    name: String,
    status: CheckStatus,
}

enum CheckStatus {
    Ok,
    Missing,
    Invalid(String),
    Warning(String),
    Info(String),
}
```

---

## 6. Testing

### 6.1 Unit Tests (in `migrate.rs` and `doctor.rs`)

| Test | What It Exercises |
|------|-------------------|
| `config_migration_maps_default_to_frontier` | JSON `models.default` → TOML `models.frontier` |
| `config_migration_maps_mode_to_per_mode` | JSON `models.team` → TOML `models.per_mode.team` |
| `config_migration_preserves_existing_toml` | Existing TOML values not overwritten without `--force` |
| `config_migration_force_overwrites` | `--force` replaces existing values |
| `config_migration_no_json_file` | Missing `.omx-config.json` → clean exit with message |
| `config_migration_env_vars` | JSON `env` → TOML `env` section |
| `session_migration_parses_rollout_jsonl` | Correctly groups lines by sessionId |
| `session_migration_builds_meta_json` | Extracted meta matches expected structure |
| `session_migration_builds_transcript` | Extracted turns are sequential with correct content |
| `session_migration_skips_existing` | Existing Rust-era sessions not overwritten |
| `session_migration_skips_malformed_lines` | Bad JSON lines logged and skipped |
| `doctor_groups_checks_correctly` | Output contains section headers |
| `doctor_detects_ts_era_config` | `.omx-config.json` present → WARNING |
| `doctor_validates_config_schema` | Invalid TOML → INVALID status |

### 6.2 Integration Tests (added to `tests/src/cli.rs`)

| Test | What It Exercises |
|------|-------------------|
| `cli_migrate_config_dry_run` | Create fixture `.omx-config.json`, run `omx migrate config --dry-run`, verify output shows fields without writing |
| `cli_migrate_sessions_dry_run` | Create fixture rollout JSONL, run `omx migrate sessions --dry-run`, verify session count reported |
| `cli_migrate_config_writes` | Run `omx migrate config`, verify `config.toml` contains migrated values |
| `cli_migrate_sessions_writes` | Run `omx migrate sessions`, verify per-session dirs created |
| `cli_doctor_grouped_output` | Run `omx doctor`, verify grouped section headers present |
| `cli_doctor_detects_ts_era` | Create `.omx-config.json` in test dir, run `omx doctor`, verify migration hint |

---

## 7. Open Questions

| # | Question | Default |
|---|----------|---------|
| 1 | Should `omx migrate config` back up the original `.omx-config.json` before migrating? | Yes — rename to `.omx-config.json.bak` |
| 2 | Should `omx migrate sessions` handle very large rollout files (>100MB) with streaming or load into memory? | Streaming — read line by line, never load entire file |
| 3 | Should `omx doctor` check notification binary versions or just existence? | Just existence — version checking is overkill for v1.0.0 |
