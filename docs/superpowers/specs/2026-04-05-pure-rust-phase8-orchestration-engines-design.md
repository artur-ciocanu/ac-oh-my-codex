# Phase 8: Orchestration Engines — Design Specification

**Date:** 2026-04-05
**Status:** Approved
**Author:** Artur Ciocanu + Claude Opus 4.6
**Predecessor:** Phase 7 (MCP Feature Parity)

---

## 1. Goal

Port the four TypeScript orchestration systems to Rust as independent library crates, achieving feature parity without legacy migration logic.

**TS sources being ported:**
- `src/ralplan/runtime.ts` (296 LoC) → `omx-ralplan`
- `src/ralph/contract.ts` + `persistence.ts` (456 LoC, minus migrations ≈ 250 LoC) → `omx-ralph`
- `src/autoresearch/contracts.ts` + `runtime.ts` (1561 LoC) → `omx-autoresearch`
- `src/pipeline/` (826 LoC) → `omx-pipeline`

**Excluded:** Legacy JSON PRD → markdown migration, legacy text progress → JSON ledger migration from ralph. The Rust system starts fresh with canonical formats only.

---

## 2. Architecture

Four independent library crates. Pipeline depends on ralplan and ralph for its built-in stage factories. Autoresearch is fully independent (invoked via CLI, not as a pipeline stage).

```
omx-types (existing)
    ↑
omx-modes (existing)        omx-state (existing)
    ↑                            ↑
├── omx-ralplan              omx-ralph
├── omx-autoresearch
│
omx-pipeline (depends on omx-ralplan, omx-ralph)
```

All crates are library crates (`lib.rs`), not binaries. They are consumed by `omx-cli` and MCP servers.

---

## 3. Crate: `omx-ralplan` — Consensus Planning

**Dependencies:** `omx-types`, `omx-modes`, `async-trait`, `serde`, `serde_json`, `chrono`

### 3.1 Types

```rust
/// Active and terminal phases for the consensus loop
pub enum RalplanPhase {
    Draft,
    ArchitectReview,
    CriticReview,
    Complete,
    Cancelled,
    Failed,
}

/// Review outcome from architect or critic
pub enum RalplanReviewVerdict {
    Approve,
    Iterate,
    Reject,
}

/// Output from the draft phase
pub struct RalplanDraftResult {
    pub summary: Option<String>,
    pub plan_path: Option<String>,
    pub artifacts: HashMap<String, serde_json::Value>,
}

/// Output from architect or critic review
pub struct RalplanReviewResult {
    pub verdict: RalplanReviewVerdict,
    pub summary: Option<String>,
    pub artifacts: HashMap<String, serde_json::Value>,
}

/// Context passed to each executor method, accumulates history
pub struct RalplanIterationContext {
    pub task: String,
    pub cwd: PathBuf,
    pub iteration: u32,
    pub prior_drafts: Vec<RalplanDraftResult>,
    pub architect_reviews: Vec<RalplanReviewResult>,
    pub critic_reviews: Vec<RalplanReviewResult>,
}

/// Options for starting a consensus run
pub struct RunRalplanConsensusOptions {
    pub task: String,
    pub cwd: Option<PathBuf>,
    pub max_iterations: Option<u32>,  // default 5
}

/// Final result of the consensus run
pub struct RalplanRuntimeResult {
    pub status: RalplanPhase,
    pub iteration: u32,
    pub planning_complete: bool,
    pub drafts: Vec<RalplanDraftResult>,
    pub architect_reviews: Vec<RalplanReviewResult>,
    pub critic_reviews: Vec<RalplanReviewResult>,
    pub latest_plan_path: Option<String>,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
}
```

### 3.2 Executor Trait

The caller provides agent implementations. The runtime only orchestrates the loop.

```rust
#[async_trait]
pub trait RalplanConsensusExecutor: Send + Sync {
    async fn draft(&self, ctx: &RalplanIterationContext) -> Result<RalplanDraftResult, OmxError>;
    async fn architect_review(
        &self,
        ctx: &RalplanIterationContext,
        draft: &RalplanDraftResult,
    ) -> Result<RalplanReviewResult, OmxError>;
    async fn critic_review(
        &self,
        ctx: &RalplanIterationContext,
        draft: &RalplanDraftResult,
        architect: &RalplanReviewResult,
    ) -> Result<RalplanReviewResult, OmxError>;
}
```

### 3.3 Core Function

`pub async fn run_ralplan_consensus(executor, options) -> Result<RalplanRuntimeResult, OmxError>`

Behavior (matching TS):
1. Check no active ralplan mode exists.
2. Start `ralplan` mode via ModeManager.
3. Loop up to `max_iterations`:
   - Draft phase: call `executor.draft()` with full history context.
   - Architect review phase: call `executor.architect_review()` with current draft.
   - Critic review phase: call `executor.critic_review()` with draft + architect review.
   - If critic approves: check planning artifacts completeness, mark complete, return.
   - If max iterations reached without approval: mark failed.
   - Otherwise: increment iteration, loop.
4. Persist state after every phase transition via ModeManager.
5. Build review_history for state persistence.
6. On error: mark failed with error message.

`pub async fn cancel_ralplan_consensus(cwd) -> Result<(), OmxError>` — delegates to mode cancellation.

---

## 4. Crate: `omx-ralph` — Phase Validation & Persistence

**Dependencies:** `omx-types`, `omx-state`, `serde`, `serde_json`, `chrono`, `tokio`

### 4.1 Types

```rust
/// Ralph lifecycle phases
pub enum RalphPhase {
    Starting,
    Executing,
    Verifying,
    Fixing,
    Complete,
    Failed,
    Cancelled,
}

pub struct RalphStateValidationResult {
    pub ok: bool,
    pub phase: Option<RalphPhase>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

/// Visual feedback from screenshot comparison
pub struct RalphVisualFeedback {
    pub score: f64,
    pub verdict: VisualVerdictStatus,
    pub category_match: bool,
    pub differences: Vec<String>,
    pub suggestions: Vec<String>,
    pub reasoning: Option<String>,
    pub threshold: Option<f64>,
}

pub enum VisualVerdictStatus {
    Pass,
    Fail,
    Ambiguous,
}

/// Progress tracking ledger
pub struct RalphProgressLedger {
    pub schema_version: u32,  // always 2
    pub source: Option<String>,
    pub source_sha256: Option<String>,
    pub strategy: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub entries: Vec<RalphProgressEntry>,
    pub visual_feedback: Vec<RalphVisualFeedback>,
}

pub struct RalphProgressEntry {
    pub content: String,
    pub created_at: String,
}

pub struct RalphCanonicalArtifacts {
    pub canonical_prd_path: Option<PathBuf>,
    pub canonical_progress_path: PathBuf,
}
```

### 4.2 Constants

```rust
pub const VISUAL_NEXT_ACTIONS_LIMIT: usize = 5;
pub const VISUAL_FEEDBACK_MAX_ENTRIES: usize = 30;
pub const DEFAULT_VISUAL_THRESHOLD: f64 = 90.0;
```

### 4.3 Core Functions

- `pub fn normalize_ralph_phase(raw: &str) -> Result<RalphPhase, OmxError>` — validates phase string. No legacy aliases (fresh Rust system).

- `pub fn validate_ralph_state(candidate: &serde_json::Value) -> RalphStateValidationResult` — validates:
  - `current_phase` is a valid RalphPhase.
  - Auto-fills defaults when `active=true`: iteration=0, max_iterations=50, current_phase=Starting, started_at=now.
  - Iteration is integer >= 0, max_iterations is integer > 0.
  - Terminal phases (Complete/Failed/Cancelled) require `active=false` and auto-fill `completed_at`.
  - Validates ISO timestamp format for started_at/completed_at.

- `pub async fn ensure_canonical_artifacts(cwd: &Path, session_id: Option<&str>) -> Result<RalphCanonicalArtifacts, OmxError>` — creates `.omx/plans/` directory, ensures canonical progress ledger file exists. Returns artifact paths.

- `pub async fn record_visual_feedback(cwd: &Path, feedback: RalphVisualFeedback, session_id: Option<&str>) -> Result<(), OmxError>` — appends to progress ledger, calculates pass/fail against threshold (default 90), extracts next_actions from suggestions+differences (capped at `VISUAL_NEXT_ACTIONS_LIMIT`), keeps last `VISUAL_FEEDBACK_MAX_ENTRIES` entries.

---

## 5. Crate: `omx-autoresearch` — Iterative Research Engine

**Dependencies:** `omx-types`, `omx-modes`, `serde`, `serde_json`, `chrono`, `tokio` (process, fs)

### 5.1 Contract Types

```rust
pub struct AutoresearchMissionContract {
    pub mission_dir: PathBuf,
    pub repo_root: PathBuf,
    pub mission_file: PathBuf,
    pub sandbox_file: PathBuf,
    pub mission_relative_dir: String,
    pub mission_content: String,
    pub sandbox_content: String,
    pub sandbox: ParsedSandboxContract,
    pub mission_slug: String,
}

pub struct ParsedSandboxContract {
    pub frontmatter: HashMap<String, String>,
    pub evaluator: AutoresearchEvaluatorContract,
    pub body: String,
}

pub struct AutoresearchEvaluatorContract {
    pub command: String,
    pub format: String,  // always "json"
    pub keep_policy: AutoresearchKeepPolicy,
}

pub enum AutoresearchKeepPolicy {
    ScoreImprovement,
    PassOnly,
}

/// Parsed output from evaluator command
pub struct AutoresearchEvaluatorResult {
    pub pass: bool,
    pub score: Option<f64>,
}
```

### 5.2 Candidate & Evaluation Types

```rust
pub enum AutoresearchCandidateStatus { Candidate, Noop, Abort, Interrupted }
pub enum AutoresearchDecisionStatus { Baseline, Keep, Discard, Ambiguous, Noop, Abort, Interrupted, Error }
pub enum AutoresearchRunStatus { Running, Stopped, Completed, Failed }

pub struct AutoresearchCandidateArtifact {
    pub status: AutoresearchCandidateStatus,
    pub candidate_commit: Option<String>,
    pub base_commit: Option<String>,
    pub description: Option<String>,
    pub notes: Vec<String>,
    pub created_at: String,
}

pub struct AutoresearchEvaluationRecord {
    pub command: String,
    pub ran_at: String,
    pub status: String,
    pub pass: Option<bool>,
    pub score: Option<f64>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub parse_error: Option<String>,
}

pub struct AutoresearchLedgerEntry {
    pub iteration: u32,
    pub kind: String,
    pub decision: AutoresearchDecisionStatus,
    pub decision_reason: String,
    pub candidate_status: AutoresearchCandidateStatus,
    pub base_commit: Option<String>,
    pub candidate_commit: Option<String>,
    pub kept_commit: Option<String>,
    pub keep_policy: String,
    pub evaluator: Option<AutoresearchEvaluationRecord>,
    pub created_at: String,
    pub notes: Vec<String>,
    pub description: Option<String>,
}
```

### 5.3 Run Manifest & Prepared Runtime

```rust
pub struct AutoresearchRunManifest {
    pub schema_version: u32,
    pub run_id: String,
    pub run_tag: String,
    pub run_dir: PathBuf,
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub mission_slug: String,
    pub status: AutoresearchRunStatus,
    pub iteration: u32,
    pub baseline_pass: Option<bool>,
    pub baseline_score: Option<f64>,
    pub last_kept_commit: Option<String>,
    pub last_kept_score: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct PreparedAutoresearchRuntime {
    pub run_id: String,
    pub run_tag: String,
    pub run_dir: PathBuf,
    pub instructions_file: PathBuf,
    pub manifest_file: PathBuf,
    pub ledger_file: PathBuf,
    pub latest_evaluator_file: PathBuf,
    pub results_file: PathBuf,
    pub state_file: PathBuf,
    pub candidate_file: PathBuf,
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub task_description: String,
}
```

### 5.4 Core Functions

**Contract loading:**
- `pub fn slugify_mission_name(value: &str) -> String` — lowercase, alphanumeric+hyphens, max 48 chars.
- `pub fn parse_sandbox_contract(content: &str) -> Result<ParsedSandboxContract, OmxError>` — hand-rolled YAML frontmatter parser (matching TS). Validates evaluator block has `command` and `format=json`. Supports optional `keep_policy`.
- `pub fn parse_evaluator_result(raw: &str) -> Result<AutoresearchEvaluatorResult, OmxError>` — parses JSON, requires `pass: bool`, optional `score: f64`.
- `pub async fn load_mission_contract(mission_dir: &Path) -> Result<AutoresearchMissionContract, OmxError>` — resolves path, verifies inside git repo, requires mission.md + sandbox.md, parses sandbox contract, computes slug.

**Runtime lifecycle:**
- `pub fn build_run_tag() -> String` — timestamp-based tag like `20260405T123456Z`.
- `pub async fn prepare_autoresearch_runtime(contract, project_root, worktree_path, options?) -> Result<PreparedAutoresearchRuntime, OmxError>` — asserts no active run (lock check), sets up git info/exclude, asserts worktree clean, creates run directory under `.omx/logs/autoresearch/<runId>/`, initializes all run files (manifest, ledger, results TSV, candidate placeholder, instructions), starts autoresearch mode, runs baseline evaluator, updates manifest with baseline results.
- `pub async fn resume_autoresearch_runtime(project_root, run_id) -> Result<PreparedAutoresearchRuntime, OmxError>` — loads manifest, verifies still running and worktree exists, re-asserts lock, re-initializes mode state.
- `pub async fn stop_autoresearch_runtime(project_root) -> Result<(), OmxError>` — stops active run.
- `pub async fn finalize_run_state(project_root, run_id, updates) -> Result<(), OmxError>` — marks run as stopped/failed/completed.

**Iteration loop:**
- `pub async fn run_evaluator(contract, worktree_path, ledger_file?, latest_evaluator_file?) -> Result<AutoresearchEvaluationRecord, OmxError>` — spawns evaluator command via `tokio::process::Command` in the worktree, parses JSON output. Writes to ledger and evaluator files if paths provided.
- `pub fn decide_outcome(manifest, candidate, evaluation) -> (AutoresearchDecisionStatus, String)` — pure decision logic: abort→stop, noop→log, interrupted→inspect, error→discard, fail→discard, pass_only+pass→keep, score_improvement+improved→keep, else→discard, missing scores→ambiguous.
- `pub async fn process_candidate(contract, manifest, project_root) -> Result<AutoresearchLedgerEntry, OmxError>` — increments iteration, reads candidate.json, validates commits (base matches last_kept, candidate resolves and matches HEAD), handles non-candidate statuses, runs evaluator, calls decide_outcome, keeps or resets worktree (`git reset --hard`), records to ledger + results TSV, regenerates instructions, updates manifest + ModeState.
- `pub fn build_instructions(contract, context) -> String` — generates markdown instruction file for the worker agent.

**Git utilities (direct command spawning):**
- `pub async fn assert_reset_safe_worktree(worktree_path) -> Result<(), OmxError>` — `git status --porcelain`, allows only known runtime files.
- `pub async fn count_trailing_noops(ledger_file) -> Result<u32, OmxError>` — counts consecutive noop entries at end of ledger.
- Internal: `git_rev_parse`, `git_reset_hard`, `git_add`, `git_commit`, `git_status_porcelain` — thin wrappers around `tokio::process::Command::new("git")`.

### 5.5 File Layout Per Run

```
.omx/logs/autoresearch/<runId>/
    manifest.json
    iteration-ledger.json
    latest-evaluator-result.json
    candidate.json
    bootstrap-instructions.md
<worktree>/
    results.tsv
```

Active run lock: `.omx/state/autoresearch-state.json`

---

## 6. Crate: `omx-pipeline` — Sequential Stage Executor

**Dependencies:** `omx-types`, `omx-modes`, `omx-ralplan`, `omx-ralph`, `async-trait`, `serde`, `serde_json`, `chrono`, `tokio`

### 6.1 Stage Interface

```rust
#[async_trait]
pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self, ctx: &StageContext) -> Result<StageResult, OmxError>;
    fn can_skip(&self, _ctx: &StageContext) -> bool { false }
}

pub struct StageContext {
    pub task: String,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub previous_stage_result: Option<StageResult>,
    pub cwd: PathBuf,
    pub session_id: Option<String>,
}

pub struct StageResult {
    pub status: StageStatus,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub enum StageStatus { Completed, Failed, Skipped }
```

### 6.2 Pipeline Config & Result

```rust
pub struct PipelineConfig {
    pub name: String,
    pub task: String,
    pub stages: Vec<Box<dyn PipelineStage>>,
    pub cwd: Option<PathBuf>,
    pub session_id: Option<String>,
    pub max_ralph_iterations: Option<u32>,  // default 10
    pub worker_count: Option<u32>,          // default 2
    pub agent_type: Option<String>,         // default "executor"
    pub on_stage_transition: Option<Box<dyn Fn(&str, &StageContext) + Send + Sync>>,
}

pub struct PipelineResult {
    pub status: PipelineStatus,
    pub stage_results: HashMap<String, StageResult>,
    pub duration_ms: u64,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub failed_stage: Option<String>,
}

pub enum PipelineStatus { Completed, Failed, Cancelled }

pub struct PipelineModeStateExtension {
    pub pipeline_name: String,
    pub pipeline_stages: Vec<String>,
    pub pipeline_stage_index: usize,
    pub pipeline_stage_results: HashMap<String, StageResult>,
    pub pipeline_max_ralph_iterations: u32,
    pub pipeline_worker_count: u32,
    pub pipeline_agent_type: String,
}
```

### 6.3 Stage Descriptors

```rust
pub struct TeamExecDescriptor {
    pub task: String,
    pub worker_count: u32,
    pub agent_type: String,
    pub staffing_plan: Option<String>,
    pub use_worktrees: bool,
    pub cwd: PathBuf,
    pub extra_env: Option<HashMap<String, String>>,
}

pub struct RalphVerifyDescriptor {
    pub task: String,
    pub max_iterations: u32,
    pub cwd: PathBuf,
    pub session_id: Option<String>,
    pub execution_artifacts: HashMap<String, serde_json::Value>,
}
```

### 6.4 Core Functions

- `pub async fn run_pipeline(config: PipelineConfig) -> Result<PipelineResult, OmxError>` — validates config (non-empty name/task, unique stage names, at least one stage, positive integers for iterations/workers), starts autopilot mode, persists PipelineModeStateExtension, iterates stages sequentially: builds StageContext with accumulated artifacts, fires on_stage_transition callback, checks can_skip, runs stage, merges artifacts keyed by stage name, persists stage result to ModeState, short-circuits on failure (marks mode inactive, returns Failed). On all-complete: marks mode inactive with phase complete, returns Completed.

- `pub async fn can_resume_pipeline(cwd: &Path) -> bool` — reads ModeState, returns true if active and not complete/failed.

- `pub async fn read_pipeline_state(cwd: &Path) -> Option<PipelineModeStateExtension>` — reads extension fields from ModeState.

- `pub async fn cancel_pipeline(cwd: &Path) -> Result<(), OmxError>` — delegates to cancel_mode for autopilot.

### 6.5 Built-in Stage Factories

- `pub fn create_ralplan_stage(executor: Option<Box<dyn RalplanConsensusExecutor>>) -> impl PipelineStage` — can_skip returns true if planning artifacts already exist; run calls `run_ralplan_consensus` if executor provided, otherwise reads existing artifacts and returns them.

- `pub fn create_team_exec_stage() -> impl PipelineStage` — extracts ralplan artifacts from context, builds plan context string, produces TeamExecDescriptor and instruction string as artifacts.

- `pub fn create_ralph_verify_stage() -> impl PipelineStage` — extracts team-exec artifacts from context, produces RalphVerifyDescriptor and instruction string as artifacts.

- `pub fn create_autopilot_pipeline_config(task: &str, options: AutopilotOptions) -> PipelineConfig` — factory for the canonical 3-stage sequence (ralplan → team-exec → ralph-verify) with defaults: max_ralph_iterations=10, worker_count=2, agent_type="executor".

---

## 7. Estimated Sizes

| Crate | TS LoC | Rust LoC (est.) |
|-------|--------|-----------------|
| omx-ralplan | 296 | ~350 |
| omx-ralph | ~250 | ~300 |
| omx-autoresearch | 1561 | ~1200 |
| omx-pipeline | 826 | ~700 |
| **Total** | **~2933** | **~2550** |
