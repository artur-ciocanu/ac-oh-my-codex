# OMX Pure Rust Migration — Phases 6-12 Design Specification

**Date:** 2026-04-05
**Status:** Draft
**Author:** Artur Ciocanu + Claude Opus 4.6
**Predecessor:** [Phase 0-5 Design](2026-04-04-pure-rust-migration-design.md)

---

## 1. Context

Phases 0-5 delivered the Rust foundation: 21 crates covering types, config, state, mux, hooks, MCP server skeletons, team basics, HUD, setup, CLI, explore, sparkshell, and notification stubs. All compile, all tests pass.

However, a comprehensive audit revealed:
- **TypeScript:** 412 files, 122,638 lines (91% of codebase)
- **Rust:** 62 files, 12,353 lines (9% of codebase)
- **MCP tools:** 14 Rust vs 30 TypeScript (16 missing)
- **Entirely missing systems:** modes, pipeline, Ralph, autoresearch, RALPLAN, agent definitions, catalog, session history
- **Major enhancement gaps** in team runtime, tmux, hooks, notifications, CLI, HUD, config

The upstream repo (Yeachan-Heo/oh-my-codex) confirms this: 414 TS files / 126,213 lines vs 33 Rust files / 6,774 lines. TypeScript is the application; Rust is the foundation layer.

This spec defines Phases 6-12 to achieve full feature parity and delete all TypeScript.

---

## 2. Approach

**Bottom-Up by Dependency Layer.** Each phase builds on the previous, with no circular dependencies. Each phase is independently testable.

Alternatives considered:
- **Feature-Vertical Slices:** Each phase delivers end-to-end user workflow. Rejected — too many internal dependencies between slices, infrastructure built piecemeal risks inconsistency.
- **Critical Path First:** Delete TS fastest by shipping MVP, backfill later. Rejected — ships feature regression (no modes/pipelines/Ralph), less pressure to complete post-deletion.

---

## 3. Phase Overview

| Phase | Name | New Crates | Crate Total |
|-------|------|-----------|-------------|
| 6 | Core Domain Models | omx-agents, omx-modes, omx-session, omx-catalog | 25 |
| 7 | MCP Feature Parity | — | 25 |
| 8 | Orchestration Engines | omx-pipeline, omx-ralph, omx-autoresearch, omx-ralplan | 29 |
| 9 | Team Runtime Hardening | — | 29 |
| 10 | Presentation Completion | — | 29 |
| 11 | Notification & Hooks | omx-notify-pushover, omx-notify-generic | 31 |
| 12 | Integration & Delete TS | — | 31 |

---

## 4. Phase 6: Core Domain Models

Domain primitives that every later phase depends on.

### 4.1 New crate: `omx-agents`

Port from `src/agents/definitions.ts` (32 agent type definitions).

**Types:**
```rust
pub struct AgentDef {
    pub name: String,
    pub posture: Posture,           // Autonomous | Supervised | Advisory
    pub model_class: ModelClass,    // Heavy | Medium | Light
    pub routing_role: RoutingRole,  // Leader | Worker | Critic | Planner | Architect
    pub system_prompt_ref: String,  // Path to prompt template
}

pub enum Posture { Autonomous, Supervised, Advisory }
pub enum ModelClass { Heavy, Medium, Light }
pub enum RoutingRole { Leader, Worker, Critic, Planner, Architect, General }

pub struct AgentRegistry { /* ... */ }
```

**Capabilities:**
- `AgentRegistry::lookup(name) -> Option<&AgentDef>`
- `AgentRegistry::filter_by_posture(Posture) -> Vec<&AgentDef>`
- `AgentRegistry::filter_by_model_class(ModelClass) -> Vec<&AgentDef>`
- `AgentRegistry::filter_by_role(RoutingRole) -> Vec<&AgentDef>`
- Validation: no duplicate names, all system_prompt_ref paths exist
- Load from TOML config (user overrides) merged with built-in defaults

**Dependencies:** omx-types, omx-config

### 4.2 New crate: `omx-modes`

Port from `src/modes/base.ts`. 8 canonical modes with lifecycle management.

**Types:**
```rust
pub enum Mode {
    Autopilot,
    Autoresearch,
    DeepInterview,
    Ralph,
    Ultrawork,
    Team,
    Ultraqa,
    Ralplan,
}

pub struct ModeState {
    pub active: Mode,
    pub activated_at: DateTime<Utc>,
    pub session_id: String,
    pub config: ModeConfig,
}

pub struct ModeConfig {
    pub agent_overrides: HashMap<String, String>,
    pub hud_preset: HudPreset,
    pub env_overrides: HashMap<String, String>,
}
```

**Capabilities:**
- `activate(mode) -> Result<ModeState>` — checks conflicts, deactivates current, activates new
- `deactivate() -> Result<()>`
- `is_compatible(a, b) -> bool` — exclusive conflict matrix
- Conflict rules: Ralph ↔ Team, Autoresearch ↔ Team, Ralplan ↔ Team (only one orchestration mode at a time)
- Session-scoped: mode state tied to session ID, persisted via omx-state

**Dependencies:** omx-types, omx-config, omx-state

### 4.3 New crate: `omx-session`

Port from `src/session-history/search.ts` and session management scattered across TS.

**Types:**
```rust
pub struct Session {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub mode: Mode,
    pub turns: u32,
    pub tokens_used: u64,
    pub status: SessionStatus, // Active | Completed | Crashed
}

pub struct SessionStore { /* ... */ }
```

**Capabilities:**
- Session ID generation (ULID-based for sortability)
- `SessionStore::create() -> Session`
- `SessionStore::get(id) -> Option<Session>`
- `SessionStore::list(filter) -> Vec<Session>`
- `SessionStore::search(query) -> Vec<SearchResult>` — full-text search over transcripts
- `SessionStore::resume(id) -> Result<Session>` — restore previous session
- Turn and token tracking — increment counters per interaction
- Transcript storage — append-only log of session turns
- Crash recovery — detect sessions with status Active but no heartbeat

**Dependencies:** omx-types, omx-state

### 4.4 New crate: `omx-catalog`

Port from `src/catalog/`. Skill and agent registry with discovery.

**Capabilities:**
- Scan filesystem for skill definitions (TOML/YAML files in configured directories)
- Validate skill definitions: required fields present, dependencies resolvable
- `CatalogRegistry::list_skills() -> Vec<SkillDef>`
- `CatalogRegistry::get_skill(name) -> Option<SkillDef>`
- `CatalogRegistry::list_agents() -> Vec<AgentDef>` (delegates to omx-agents)
- Dependency resolution: topological sort of skill dependencies

**Dependencies:** omx-types, omx-config, omx-agents

---

## 5. Phase 7: MCP Feature Parity

Complete all 5 MCP servers from 14 to 30 tools.

### 5.1 `omx-mcp-memory`: 5 → 9 tools

**New tools:**
| Tool | Description |
|------|-------------|
| `notepad_read` | Read notepad contents (priority or working section) |
| `notepad_write_priority` | Write to priority notepad (high-importance items) |
| `notepad_write_working` | Write to working notepad (scratch/WIP items) |
| `notepad_stats` | Return notepad size, entry count, last modified |

Notepad is a structured scratch area distinct from memory entries. Priority items surface in context; working items are background reference. Storage: `{memory_dir}/notepad/{priority,working}/` with atomic file I/O via fs2.

### 5.2 `omx-mcp-code-intel`: 2 → 9 tools

**New tools:**
| Tool | Description |
|------|-------------|
| `lsp_diagnostics` | Get diagnostics (errors/warnings) for a file from running LSP |
| `lsp_document_symbols` | List symbols in a file via LSP documentSymbol |
| `lsp_workspace_symbols` | Search symbols across workspace via LSP workspaceSymbol |
| `lsp_hover` | Get hover info (type, docs) for a position via LSP |
| `lsp_find_references` | Find all references to a symbol via LSP |
| `lsp_servers` | List running LSP servers and their status |
| `ast_grep_replace` | Structural search/replace using ast-grep CLI |

LSP tools communicate with running LSP servers via LSP protocol over stdio. The MCP server manages LSP client connections (start on first use, reuse across calls). ast-grep is invoked as a subprocess.

### 5.3 `omx-mcp-state`: storage model realignment

**Current (Rust):** flat key-value store.
**Target:** mode-scoped file-based storage.

All state operations gain a `mode` parameter:
- `state_read(mode, key)` — reads from `{state_dir}/{mode}/{key}`
- `state_write(mode, key, value)` — writes to `{state_dir}/{mode}/{key}`
- `state_list(mode)` — lists keys under `{state_dir}/{mode}/`
- `state_delete(mode, key)` — removes key
- `state_list_modes()` — lists all modes that have stored state

Backward compatibility: if `mode` is omitted, defaults to `"global"` scope.

### 5.4 `omx-mcp-trace`: enhanced

**Enhancements to existing 2 tools:**
- Filter parameters: `mode`, `time_range` (start/end), `severity` (info/warn/error)
- Mode event merging: correlate trace events with mode transitions from omx-modes
- Metrics integration: expose `turn_count`, `tokens_used`, `quota_remaining` per trace span

### 5.5 `omx-mcp-team`: enhanced

**Enhancements to existing 4 tools:**
- Job ID tracking: each `team_dispatch` returns a unique job ID; subsequent queries can filter by job
- Auto-nudge tool: `team_nudge(worker_id)` — send nudge to idle worker
- Event-driven wakeup: workers write completion events to trace; leader polls trace for wakeup
- Leader pane protection: `team_dispatch` refuses to target the leader's pane
- Exponential backoff: retry failed worker dispatches with configurable backoff

---

## 6. Phase 8: Orchestration Engines

Four new crates implementing workflow runtimes with zero Rust equivalent today.

### 6.1 New crate: `omx-pipeline`

Port from `src/pipeline/` (8 TS files).

**Types:**
```rust
pub struct Pipeline {
    pub name: String,
    pub stages: Vec<Stage>,
}

pub struct Stage {
    pub name: String,
    pub kind: StageKind,        // Ralplan | TeamExec | RalphVerify | Custom
    pub entry_conditions: Vec<Condition>,
    pub exit_conditions: Vec<Condition>,
    pub retry_policy: RetryPolicy,
}

pub struct StageExecution {
    pub stage: String,
    pub status: StageStatus,    // Pending | Running | Succeeded | Failed | Skipped
    pub artifacts: Vec<Artifact>,
    pub attempts: u32,
}

pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: Duration,
}

pub enum Condition {
    ArtifactExists(String),     // Named artifact must be present
    PreviousSucceeded,          // Previous stage must have succeeded
    Custom(String),             // Shell command returns 0
}

pub struct Artifact {
    pub name: String,
    pub path: PathBuf,          // File path to artifact content
    pub produced_by: String,    // Stage name that produced it
}
```

**Capabilities:**
- Stage sequencing: execute stages in order, pass artifacts between stages
- Built-in pipeline: RALPLAN → Team Exec → Ralph Verify
- Custom pipelines: user-defined via TOML (`[pipeline.my-workflow]` with stage list)
- State machine: Pending → Running → Succeeded/Failed, with retry on failure
- Inter-stage data: each stage produces `Vec<Artifact>` consumed by the next
- Abort: cancel pipeline, mark remaining stages as Skipped
- Pipeline state persisted via omx-state for resume after crash

### 6.2 New crate: `omx-ralph`

Port from `src/ralph/` — persistent multi-session workflows.

**Types:**
```rust
pub struct RalphSession {
    pub id: String,
    pub plan: String,           // The original plan being executed
    pub iterations: Vec<RalphIteration>,
    pub status: RalphStatus,    // Planning | Executing | Verifying | Reworking | Complete | Failed
}

pub struct RalphIteration {
    pub index: u32,
    pub team_job_id: Option<String>,
    pub verification: Option<VerificationResult>,
    pub rework_instructions: Option<String>,
}

pub struct VerificationResult {
    pub passed: bool,
    pub findings: Vec<String>,
}
```

**Capabilities:**
- Persistent sessions: survive across CLI invocations, stored via omx-state with `ralph` namespace
- Execution cycle: dispatch plan to team → wait for completion → verify output
- Verification loop: Ralph reviews team output against plan, produces findings
- Rework cycle: if verification fails, generate corrective instructions, re-dispatch to team
- Max iterations: configurable limit to prevent infinite rework loops
- Resume: `ralph resume <session-id>` picks up where it left off
- Integration: pipeline invokes Ralph as a stage (`StageKind::RalphVerify`)

### 6.3 New crate: `omx-autoresearch`

Port from `src/autoresearch/runtime.ts` (1,314 lines).

**Types:**
```rust
pub struct ResearchSession {
    pub id: String,
    pub query: String,
    pub iterations: Vec<ResearchIteration>,
    pub max_iterations: u32,
    pub status: ResearchStatus, // Iterating | Satisfied | MaxReached | Failed
}

pub struct ResearchIteration {
    pub index: u32,
    pub worktree_path: PathBuf,
    pub findings: String,
    pub evaluation: EvaluationResult,
    pub refined_query: Option<String>,
}

pub struct EvaluationResult {
    pub satisfied: bool,
    pub score: f64,
    pub reasoning: String,
}
```

**Capabilities:**
- Iterative research with git worktrees: each iteration gets an isolated worktree via omx-mux
- Evaluation: user-defined success conditions checked after each iteration
- Query refinement: if evaluation unsatisfied, refine query based on findings
- Worktree lifecycle: create on iteration start, merge findings or discard on completion
- Configurable: max iterations, evaluation criteria, worktree base path
- Session persistence via omx-state for resume

### 6.4 New crate: `omx-ralplan`

Port from `src/ralplan/runtime.ts` — consensus planning.

**Types:**
```rust
pub struct ConsensusSession {
    pub id: String,
    pub objective: String,
    pub rounds: Vec<ConsensusRound>,
    pub max_rounds: u32,
    pub status: ConsensusStatus, // InProgress | Agreed | Deadlocked | MaxRounds
}

pub struct ConsensusRound {
    pub index: u32,
    pub plan: String,           // Planner output
    pub review: String,         // Architect review
    pub critique: String,       // Critic findings
    pub consensus: bool,        // All three agree?
}
```

**Capabilities:**
- Three-role model: Planner generates plan, Architect validates feasibility, Critic finds gaps
- Each role is an agent (resolved from omx-agents by RoutingRole)
- Consensus loop: iterate until all three roles approve or max rounds reached
- Deadlock handling: if max rounds reached without consensus, surface disagreements to user
- Output: approved plan document that feeds into pipeline as first stage artifact
- Session persistence via omx-state for resume

---

## 7. Phase 9: Team Runtime Hardening

Production-hardening the existing omx-team, omx-mux, and omx-hooks crates.

### 7.1 `omx-team` enhancements

| Feature | Description |
|---------|-------------|
| Distributed locks | fs2-based lock per worker to prevent double-dispatch |
| Heartbeat system | Workers write periodic heartbeat files; leader detects stale workers |
| Dead worker recovery | Leader reassigns tasks from workers whose heartbeat expired |
| Auto-commit | Workers commit WIP at configurable intervals |
| Merge strategies | Per-job: merge, cherry-pick, or squash when integrating worker output |
| Cross-worker rebase | Handle conflicts when multiple workers touch overlapping files |
| Event audit log | Append-only log of all team events (dispatch, completion, failure, reassignment) |
| Task approval gate | Leader reviews worker output before merge (approve/reject/rework) |
| Multi-CLI support | Multiple CLI instances can observe the same team session |

### 7.2 `omx-mux` tmux enhancements

| Feature | Description |
|---------|-------------|
| HUD pane management | Dedicated pane for HUD display, resize-aware |
| Pane ID tracking | Map logical worker IDs to tmux pane IDs |
| Leader pane protection | Prevent accidental kill of leader pane |
| Trust prompt dismissal | Auto-dismiss Claude trust prompts in worker panes |
| Resize hooks | Re-render HUD when terminal resizes |
| Multi-transport dispatch | Send commands to panes via multiple methods (send-keys, pipe-pane) |

### 7.3 `omx-hooks` enhancements

| Feature | Description |
|---------|-------------|
| Hook chaining | Multiple hooks per event, executed in defined order |
| Hook timeout | Kill hooks that exceed configured duration |
| Hook result aggregation | Collect stdout from all hooks for an event |

**Built-in hook executables** (ported from TS):
1. `notify-fallback-watcher` — monitors for notification delivery failures, retries or escalates
2. `notify-hook-auto-nudge` — detects idle workers, sends nudge notifications
3. `notify-hook-team-leader-nudge` — nudges team leader when workers await review
4. `notify-hook-team-dispatch` — fires notification when team dispatches work
5. `notify-hook-worker-idle` — fires notification when worker goes idle too long
6. `notify-hook-tmux-heal` — detects and repairs broken tmux sessions

---

## 8. Phase 10: Presentation Completion

Everything the user directly interacts with.

### 8.1 `omx-cli` missing commands

| Command | Description |
|---------|-------------|
| `exec` | Run a single agent task (non-team) |
| `agents` | List available agent definitions |
| `agents-init` | Scaffold agent config files |
| `uninstall` | Remove OMX configuration and artifacts |
| `cleanup` | Remove stale sessions, worktrees, lock files |
| `session` | List/inspect session history |
| `resume` | Restore a previous session |
| `ralph` | Start/resume Ralph persistent workflow |
| `autoresearch` | Start autoresearch loop |
| `ralplan` | Start consensus planning session |
| `pipeline` | Run a named pipeline |
| `tmux-hook` | Invoked by tmux hooks (resize, pane close) |
| `status` | Show current mode, active team, session metrics |
| `reasoning` | Display/configure model reasoning settings |

### 8.2 Default launch policy

3-phase launch sequence:
1. **Validate config** — load and validate TOML, check required fields, warn on deprecations
2. **Inject AGENTS.md overlay** — merge OMX agent instructions into project's AGENTS.md (non-destructive, marker-delimited section)
3. **Start session** — create session, activate requested mode, register signal handlers for cleanup

Signal handler cleanup: on SIGINT/SIGTERM/panic, deactivate mode, write session status, remove lock files.

### 8.3 `omx-hud` enhancements

| Feature | Description |
|---------|-------------|
| 8 mode indicators | Show active mode with mode-specific color and icon |
| 3 presets | Minimal, standard, verbose (configurable per mode) |
| Git context | Current branch, dirty state, ahead/behind counts |
| Session metrics | Turn count, token usage, quota remaining |
| Team status | Worker count, completed/pending tasks (in team mode) |

### 8.4 `omx-setup` / `omx-config` enhancements

| Feature | Description |
|---------|-------------|
| Feature flags section | Toggle experimental features in config |
| Env section | Environment variable overrides per mode |
| Agents section | Custom agent definition overrides |
| TUI config wizard | Interactive setup via ratatui |
| Shared MCP registry | Central config for which MCP servers to launch |
| Orphaned section cleanup | Detect and warn about stale config keys after upgrades |

---

## 9. Phase 11: Notification & Hook Completion

### 9.1 New crate: `omx-notify-pushover`

- Pushover API integration via reqwest
- Priority mapping: OMX severity → Pushover priority (-2 to 2)
- Device targeting: send to specific user devices

### 9.2 New crate: `omx-notify-generic`

- Generic webhook: POST JSON payload to user-configured URL
- Covers email gateways, custom integrations, any HTTP endpoint
- Configurable: headers, auth token, payload template

### 9.3 Existing notify crate enhancements (discord, slack, telegram)

| Feature | Description |
|---------|-------------|
| Mention support | @user, @role, @channel per platform syntax |
| Reply correlation | Thread replies (Slack/Discord), reply-to (Telegram) |
| Rich formatting | Platform-native embeds/blocks instead of plain text |

### 9.4 Template system

Shared across all notify crates:

**21 template variables:**
`{{mode}}`, `{{worker_id}}`, `{{task_status}}`, `{{branch}}`, `{{commit_sha}}`, `{{duration}}`, `{{error}}`, `{{session_id}}`, `{{turn_count}}`, `{{tokens_used}}`, `{{quota_remaining}}`, `{{job_id}}`, `{{stage_name}}`, `{{pipeline_name}}`, `{{iteration}}`, `{{worker_count}}`, `{{completed_tasks}}`, `{{pending_tasks}}`, `{{team_name}}`, `{{timestamp}}`, `{{hostname}}`

**Features:**
- Conditionals: `{{#if error}}Error: {{error}}{{/if}}`
- Per-event config: different template, channel, mention rules per event type
- Template validation at config load time

### 9.5 Shell hook → notification flow

1. Event occurs (team dispatch, worker idle, etc.)
2. omx-hooks dispatcher fires matching hook executables
3. Hook produces HookEvent JSON on stdout
4. Notification dispatcher routes event to configured platforms
5. Platform-specific notify crate formats message using templates and sends

User-defined hooks follow the same pattern: any executable that reads JSON stdin and writes HookEvent JSON stdout integrates with the notification system.

---

## 10. Phase 12: Integration, Migration & Delete TypeScript

### 10.1 Integration test suite

| Test category | Coverage |
|---------------|----------|
| CLI end-to-end | Every command exercised against real config/state files |
| Team workflow | Leader spawns workers → complete tasks → merge → verify |
| Pipeline | Full RALPLAN → Team Exec → Ralph Verify cycle |
| MCP tools | All 30 tools exercised via stdio transport |
| Notifications | Mock HTTP endpoints verify all 5 platforms |
| Hook chains | Event fires → hooks execute → notifications dispatch |
| Mode lifecycle | Activate → state scoping → conflict detection → deactivate |

### 10.2 Migration tooling

- **Config migration:** convert TS-era TOML configs to new schema if structure changed
- **Session migration:** convert existing session history files to Rust-era format
- **`omx doctor`:** validation command — checks config, MCP servers, hooks, tmux, permissions, reports issues

### 10.3 Delete TypeScript

- Remove all `src/` TypeScript files (412 files, ~123K lines)
- Remove `package.json`, `tsconfig.json`, `node_modules`, `bun.lockb`, npm/bun tooling
- Remove TS-specific CI/CD steps
- Update Cargo.toml workspace — all 31 crates build via single `cargo build`
- Single binary output: `omx` binary embeds everything

### 10.4 Release

- Tag v1.0.0
- Update README: installation is `cargo install omx` or download binary
- Changelog covering full migration

---

## 11. Dependency Graph

```
Phase 6: Core Domain Models
  └── omx-agents, omx-modes, omx-session, omx-catalog
       ↓
Phase 7: MCP Feature Parity
  └── All 5 MCP servers enhanced (depends on omx-modes, omx-agents)
       ↓
Phase 8: Orchestration Engines
  └── omx-pipeline, omx-ralph, omx-autoresearch, omx-ralplan
  └── (depends on Phase 6 + 7 + existing omx-team, omx-mux)
       ↓
Phase 9: Team Runtime Hardening
  └── omx-team, omx-mux, omx-hooks enhanced
  └── (depends on Phase 6 agents, Phase 8 pipeline/ralph)
       ↓
Phase 10: Presentation Completion
  └── omx-cli, omx-hud, omx-setup, omx-config enhanced
  └── (depends on all previous phases — CLI is top-level entry)
       ↓
Phase 11: Notification & Hook Completion
  └── omx-notify-pushover, omx-notify-generic, template system
  └── (depends on Phase 9 hooks, Phase 6 modes/sessions)
       ↓
Phase 12: Integration & Delete TS
  └── Integration tests, migration tooling, delete TypeScript, v1.0.0
```

---

## 12. Crate Inventory (Final State)

| # | Crate | Layer | Status after Phase 12 |
|---|-------|-------|-----------------------|
| 1 | omx-types | Foundation | Complete (Phase 1) |
| 2 | omx-config | Foundation | Enhanced (Phase 10) |
| 3 | omx-state | Foundation | Complete (Phase 1) |
| 4 | omx-mux | Foundation | Enhanced (Phase 9) |
| 5 | omx-agents | Domain | New (Phase 6) |
| 6 | omx-modes | Domain | New (Phase 6) |
| 7 | omx-session | Domain | New (Phase 6) |
| 8 | omx-catalog | Domain | New (Phase 6) |
| 9 | omx-runtime-core | Runtime | Complete (Phase 0) |
| 10 | omx-hooks | Runtime | Enhanced (Phase 9) |
| 11 | omx-runtime | Runtime | Complete (Phase 0) |
| 12 | omx-mcp-state | Service | Enhanced (Phase 7) |
| 13 | omx-mcp-memory | Service | Enhanced (Phase 7) |
| 14 | omx-mcp-code-intel | Service | Enhanced (Phase 7) |
| 15 | omx-mcp-trace | Service | Enhanced (Phase 7) |
| 16 | omx-mcp-team | Service | Enhanced (Phase 7) |
| 17 | omx-pipeline | Orchestration | New (Phase 8) |
| 18 | omx-ralph | Orchestration | New (Phase 8) |
| 19 | omx-autoresearch | Orchestration | New (Phase 8) |
| 20 | omx-ralplan | Orchestration | New (Phase 8) |
| 21 | omx-team | Team | Enhanced (Phase 9) |
| 22 | omx-hud | Presentation | Enhanced (Phase 10) |
| 23 | omx-setup | Presentation | Enhanced (Phase 10) |
| 24 | omx-cli | Presentation | Enhanced (Phase 10) |
| 25 | omx-sparkshell | Standalone | Complete (Phase 0) |
| 26 | omx-explore | Standalone | Complete (Phase 0) |
| 27 | omx-notify-discord | Notification | Enhanced (Phase 11) |
| 28 | omx-notify-slack | Notification | Enhanced (Phase 11) |
| 29 | omx-notify-telegram | Notification | Enhanced (Phase 11) |
| 30 | omx-notify-pushover | Notification | New (Phase 11) |
| 31 | omx-notify-generic | Notification | New (Phase 11) |

---

## 13. Open Questions

| # | Question | Impact | Default if unresolved |
|---|----------|--------|-----------------------|
| 1 | Should `omx-catalog` support hot-reload of skill definitions? | Phase 6 scope | No — reload on CLI restart |
| 2 | Should LSP client connections in code-intel be persistent or per-request? | Phase 7 performance | Persistent with idle timeout |
| 3 | Should Ralph verification use a dedicated agent or reuse the Critic role? | Phase 8 design | Dedicated agent — verification ≠ critique |
| 4 | Should `omx doctor` be a CLI subcommand or a separate binary? | Phase 12 distribution | Subcommand of `omx` |
| 5 | Gemini CLI support — add as a third LLM CLI target? | Post-v1.0.0 | Defer to v1.1.0 |
