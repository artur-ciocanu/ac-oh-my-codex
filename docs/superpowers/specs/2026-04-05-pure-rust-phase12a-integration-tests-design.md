# Phase 12a: Integration Test Suite — Design Specification

**Date:** 2026-04-05
**Status:** Draft
**Author:** Artur Ciocanu + Claude Opus 4.6
**Predecessor:** [Phases 6-12 Design](2026-04-05-pure-rust-phases6-12-design.md)

---

## 1. Context

Phases 0–11 delivered 32 Rust crates (31 from the original plan + `omx-notify-template`) covering the full OMX feature set: types, config, state, mux, hooks, MCP servers, team runtime, HUD, setup, CLI, explore, sparkshell, notifications, modes, sessions, agents, catalog, pipeline, ralph, autoresearch, and ralplan.

All crates compile, all unit tests pass. However, there is no cross-crate integration test infrastructure. Before we can delete the TypeScript codebase (Phase 12c) and release v1.0.0 (Phase 12d), we need confidence that the Rust crates work together correctly end-to-end.

### Current State

- **TypeScript:** 412 files, 122,638 lines (still present, functioning)
- **Rust:** 96 files, 22,854 lines across 32 crates
- **CLI commands:** ~25 subcommands defined; ~15 fully wired, ~10 print "(not yet wired)" stubs
- **MCP servers:** 5 binaries, 14+ tools registered
- **Notification hooks:** 5 platform binaries (Discord, Slack, Telegram, Pushover, Generic)
- **Integration tests:** None

---

## 2. Goals

1. Validate that all CLI subcommands produce expected output (real or stub)
2. Verify all 5 MCP servers launch, respond to JSON-RPC, and register expected tools
3. Confirm all 5 notification platform binaries accept HookEvent stdin and produce valid HTTP requests
4. Test the hook-to-notification dispatch chain end-to-end
5. Exercise the mode lifecycle API (activate, deactivate, conflict detection, state scoping)
6. Establish a test infrastructure that Phase 12b–d can build upon

### Non-Goals

- Wiring stub CLI commands to their crates (separate work)
- Testing real tmux sessions or provider CLIs in the default test run
- Duplicating individual tool logic already covered by unit tests

---

## 3. Architecture

### 3.1 Test Crate Location

A single workspace-root test crate:

```
tests/
├── Cargo.toml
└── src/
    ├── lib.rs              # Shared test helpers, fixtures, temp config builder
    ├── cli.rs              # CLI end-to-end tests
    ├── mcp.rs              # MCP server smoke + tool inventory
    ├── notifications.rs    # Mock HTTP + all 5 platforms
    ├── hooks.rs            # Event → hook chain → notification dispatch
    └── modes.rs            # Mode lifecycle, conflict matrix, state scoping
```

Rationale: A root-level crate keeps integration tests clearly separate from unit tests, can depend on any combination of crates, and avoids polluting individual crate `tests/` dirs with cross-cutting scenarios.

### 3.2 Workspace Registration

Add `"tests"` to the `[workspace] members` array in the root `Cargo.toml`.

---

## 4. Test Categories

### 4.1 CLI End-to-End (`cli.rs`)

Spawn the `omx` binary via `std::process::Command` (or `assert_cmd`) with a temporary config directory. Each test creates an isolated `tempdir` with a valid `config.toml` and sets `HOME`/`CODEX_HOME` accordingly.

**Wired commands — assert real behavior:**

| Command | Assertion |
|---------|-----------|
| `omx version` | stdout contains version string |
| `omx doctor` | stdout contains "ok" or "MISSING" for each dependency |
| `omx setup --scope user` | Creates AGENTS.md, syncs prompts |
| `omx status` | stdout contains "OMX Status", model name |
| `omx cancel` | stdout contains "Cancelling" |
| `omx cleanup` | stdout contains cleanup report |
| `omx hooks status` | stdout contains hook count or "No hooks found" |
| `omx hooks validate` | stdout contains valid/invalid counts |
| `omx reasoning` | stdout contains "reasoning effort" |
| `omx reasoning --effort high` | stdout contains "set to: high" |
| `omx hook-api state-write` | Writes state, verify via state-read |
| `omx hook-api state-read` | Reads state written above |
| `omx hook-api session-read` | Returns JSON (empty or session data) |

**Stub commands — assert placeholder output:**

| Command | Assertion |
|---------|-----------|
| `omx exec --agent test "prompt"` | stdout contains "not yet wired" |
| `omx agents` | stdout contains "not yet wired" |
| `omx agents-init` | stdout contains "not yet wired" |
| `omx uninstall` | stdout contains "not yet wired" |
| `omx session` | stdout contains "not yet wired" or session listing |
| `omx resume <id>` | stdout contains "not yet wired" |
| `omx ralph` | stdout contains "not yet wired" |
| `omx autoresearch "q"` | stdout contains "not yet wired" |
| `omx ralplan "q"` | stdout contains "not yet wired" |
| `omx pipeline "name"` | stdout contains "not yet wired" or pipeline output |

**Tmux-dependent commands (marked `#[ignore]`):**

| Command | Assertion |
|---------|-----------|
| `omx team start 2:executor "task"` | Exits successfully, stdout contains "Team started" |
| `omx explore --prompt "test"` | Launches omx-explore binary |
| `omx sparkshell ls` | Launches omx-sparkshell binary |
| `omx hud` | Launches HUD (may need `--no-interactive` flag or immediate exit) |

**Edge cases:**

| Test | Assertion |
|------|-----------|
| No args (default launch) | Attempts to launch provider, exits with error if provider not found |
| Invalid team spec | stderr contains error, non-zero exit |
| Invalid provider | stderr contains "Unknown provider" |
| `omx hook-api state-read` with path traversal | stderr contains "Invalid", non-zero exit |

**Expected count:** ~25 tests

### 4.2 MCP Server Smoke + Tool Inventory (`mcp.rs`)

For each of the 5 MCP server binaries (`omx-mcp-state`, `omx-mcp-memory`, `omx-mcp-code-intel`, `omx-mcp-trace`, `omx-mcp-team`):

**Smoke test (1 per server = 5 tests):**
1. Spawn the binary via `std::process::Command` with stdin/stdout piped
2. Send JSON-RPC `initialize` request
3. Assert valid JSON-RPC response with `capabilities`
4. Send one representative tool call (e.g., `state_list_modes` for state server)
5. Assert valid JSON-RPC response (success or expected error)
6. Send `shutdown` notification
7. Assert process exits cleanly

**Tool inventory test (1 per server = 5 tests):**
1. Spawn the binary, send `initialize`
2. Send `tools/list` JSON-RPC request
3. Assert response contains the expected tool names:

| Server | Expected Tools |
|--------|---------------|
| `omx-mcp-state` | `state_read`, `state_write`, `state_list`, `state_delete`, `state_list_modes` |
| `omx-mcp-memory` | `memory_create`, `memory_read`, `memory_list`, `memory_search`, `memory_delete`, `notepad_read`, `notepad_write_priority`, `notepad_write_working`, `notepad_stats` |
| `omx-mcp-code-intel` | `code_search`, `code_definition`, `lsp_diagnostics`, `lsp_document_symbols`, `lsp_workspace_symbols`, `lsp_hover`, `lsp_find_references`, `lsp_servers`, `ast_grep_replace` |
| `omx-mcp-trace` | `trace_read`, `trace_query` |
| `omx-mcp-team` | `team_dispatch`, `team_status`, `team_result`, `team_cancel`, `team_nudge` |

Note: The exact tool names will be verified against what the servers actually register. If a server registers fewer tools than the spec target, the test documents the current state.

**Expected count:** ~10 tests

### 4.3 Notifications (`notifications.rs`)

Use `wiremock` to stand up a mock HTTP server on localhost. For each notification binary:

1. Start `wiremock::MockServer`
2. Register a mock that accepts POST and returns 200/204
3. Construct a `HookEvent` JSON payload
4. Set platform-specific env vars pointing at the mock URL
5. Pipe the HookEvent to the binary's stdin via `std::process::Command`
6. Assert: mock received exactly 1 request
7. Assert: request body matches platform format

| Binary | Env Var | Body Assertion |
|--------|---------|---------------|
| `omx-notify-discord` | `OMX_DISCORD_WEBHOOK_URL` | JSON with `embeds` array, `color` field |
| `omx-notify-slack` | `OMX_SLACK_WEBHOOK_URL` | JSON with `blocks` array, header block |
| `omx-notify-telegram` | `OMX_TELEGRAM_BOT_TOKEN` + `OMX_TELEGRAM_CHAT_ID` | JSON with `text` (HTML), `parse_mode: "HTML"` |
| `omx-notify-pushover` | `OMX_PUSHOVER_USER_KEY` + `OMX_PUSHOVER_APP_TOKEN` | Form-encoded with `token`, `user`, `message`, `priority` |
| `omx-notify-generic` | `OMX_GENERIC_WEBHOOK_URL` | JSON with `event`, `message`, `timestamp`, `context` |

**Expected count:** 5 tests

### 4.4 Hook Chains (`hooks.rs`)

Tests the full dispatch chain using the library APIs directly (no binary spawning except for hook executables).

| Test | What It Exercises |
|------|-------------------|
| `hook_discovery_finds_executable_scripts` | Create temp dir with executable scripts, `ShellHookDispatcher::discover` returns them |
| `hook_dispatch_executes_matching_hooks` | Create a temp hook script that echoes JSON, dispatch a HookEvent, verify HookResult |
| `notification_router_routes_to_configured_platforms` | Create `NotificationConfig` with Discord + Slack, call `route()`, assert both enabled |
| `notification_router_empty_config_routes_nowhere` | Default `NotificationConfig`, call `route()`, assert no platforms enabled |

**Expected count:** 4 tests

### 4.5 Mode Lifecycle (`modes.rs`)

Tests the `omx-modes` crate API directly.

| Test | What It Exercises |
|------|-------------------|
| `activate_mode_returns_state` | `activate(Autopilot)` succeeds, returns `ModeState` with correct mode |
| `deactivate_clears_active_mode` | Activate then deactivate, verify no active mode |
| `conflict_ralph_vs_team` | Activate Ralph, attempt Team → should fail or report conflict |
| `conflict_autoresearch_vs_team` | Activate Autoresearch, attempt Team → should fail |
| `conflict_ralplan_vs_team` | Activate Ralplan, attempt Team → should fail |
| `non_conflicting_modes_coexist` | Activate Autopilot, verify no conflict with Ultrawork |

**Expected count:** 6 tests

---

## 5. Shared Test Helpers (`lib.rs`)

### 5.1 `TestConfig`

Builder for creating temporary OMX config directories:

```rust
struct TestConfig {
    dir: tempfile::TempDir,
}

impl TestConfig {
    fn new() -> Self;                           // minimal valid config
    fn with_notifications(discord, slack, ...) -> Self;  // notification config
    fn config_path(&self) -> &Path;             // path to config.toml
    fn home_path(&self) -> &Path;               // path to temp home dir
}
```

### 5.2 `OmxBinary`

Helper for spawning OMX binaries with the test config:

```rust
fn omx_cmd(config: &TestConfig) -> Command;           // omx binary with env set
fn mcp_cmd(server: &str, config: &TestConfig) -> Command;  // MCP server binary
fn notify_cmd(platform: &str) -> Command;              // notification binary
```

### 5.3 `JsonRpc`

Helper for constructing JSON-RPC messages:

```rust
fn json_rpc_request(id: u64, method: &str, params: Value) -> String;
fn parse_json_rpc_response(output: &str) -> Value;
```

### 5.4 `HookEventFixture`

Pre-built HookEvent payloads for common test scenarios:

```rust
fn session_start_event() -> HookEvent;
fn failed_event(error: &str) -> HookEvent;
fn turn_complete_event(worker_id: &str) -> HookEvent;
```

---

## 6. Dependencies

```toml
[package]
name = "omx-integration-tests"
version.workspace = true
edition.workspace = true

[dependencies]
# OMX crates for API-level tests
omx-types = { path = "../crates/omx-types" }
omx-config = { path = "../crates/omx-config" }
omx-state = { path = "../crates/omx-state" }
omx-modes = { path = "../crates/omx-modes" }
omx-hooks = { path = "../crates/omx-hooks" }
omx-notify-template = { path = "../crates/omx-notify-template" }

# Test infrastructure
wiremock = "0.6"
tempfile = { workspace = true }
serde_json = { workspace = true }
serde = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
```

---

## 7. Test Execution

```bash
# Run all integration tests (excludes #[ignore])
cargo test -p omx-integration-tests

# Run only tmux-dependent tests
cargo test -p omx-integration-tests -- --ignored

# Run a specific category
cargo test -p omx-integration-tests cli
cargo test -p omx-integration-tests mcp
cargo test -p omx-integration-tests notifications
cargo test -p omx-integration-tests hooks
cargo test -p omx-integration-tests modes

# Run everything including ignored
cargo test -p omx-integration-tests -- --include-ignored
```

---

## 8. Expected Test Count Summary

| Module | Tests | `#[ignore]` |
|--------|-------|-------------|
| `cli.rs` | ~25 | ~4 (tmux-dependent) |
| `mcp.rs` | ~10 | 0 |
| `notifications.rs` | 5 | 0 |
| `hooks.rs` | 4 | 0 |
| `modes.rs` | 6 | 0 |
| **Total** | **~50** | **~4** |

---

## 9. Open Questions

| # | Question | Default |
|---|----------|---------|
| 1 | Should MCP smoke tests use the debug or release binary? | Debug — faster to build during development |
| 2 | Should notification tests verify exact payload structure or just presence of key fields? | Key fields — exact structure is covered by unit tests |
| 3 | Should we add `assert_cmd` as a dependency or use raw `std::process::Command`? | Raw `Command` — fewer dependencies, sufficient for our needs |
