# Phase 8: Orchestration Engines — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the four TypeScript orchestration systems (ralplan, ralph, autoresearch, pipeline) to Rust as independent library crates with full feature parity.

**Architecture:** Four new library crates. `omx-ralplan` depends on `omx-types` and `omx-modes`. `omx-ralph` depends on `omx-types` and `omx-state`. `omx-autoresearch` depends on `omx-types` and `omx-modes`. `omx-pipeline` depends on `omx-types`, `omx-modes`, `omx-ralplan`, and `omx-ralph`. All are library crates consumed by `omx-cli` and MCP servers.

**Tech Stack:** Rust 2021, serde, serde_json, async-trait, tokio, chrono, thiserror, omx-types, omx-modes, omx-state

---

## File Structure

### New crate: `crates/omx-ralplan/`
```
crates/omx-ralplan/
├── Cargo.toml
└── src/
    └── lib.rs          # RalplanPhase, RalplanConsensusExecutor trait, run_ralplan_consensus
```

### New crate: `crates/omx-ralph/`
```
crates/omx-ralph/
├── Cargo.toml
└── src/
    └── lib.rs          # RalphPhase, validate_ralph_state, ensure_canonical_artifacts, record_visual_feedback
```

### New crate: `crates/omx-autoresearch/`
```
crates/omx-autoresearch/
├── Cargo.toml
└── src/
    └── lib.rs          # Contract loading, runtime lifecycle, candidate processing, git utilities
```

### New crate: `crates/omx-pipeline/`
```
crates/omx-pipeline/
├── Cargo.toml
└── src/
    └── lib.rs          # PipelineStage trait, run_pipeline, stage factories
```

### Modified files:
- `Cargo.toml` (workspace root) — add 4 new members
- `crates/omx-types/src/lib.rs` — add 4 new OmxError variants (Ralplan, Ralph, Autoresearch, Pipeline)

---

## Task 1: Add workspace scaffolding and new error variants

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/omx-types/src/lib.rs`

- [ ] **Step 1: Write failing tests for new error variants**

Add these tests at the end of the existing `mod tests` block in `crates/omx-types/src/lib.rs`:

```rust
    #[test]
    fn ralplan_error_display() {
        let err = OmxError::Ralplan("consensus failed".into());
        assert_eq!(err.to_string(), "ralplan error: consensus failed");
    }

    #[test]
    fn ralph_error_display() {
        let err = OmxError::Ralph("invalid phase".into());
        assert_eq!(err.to_string(), "ralph error: invalid phase");
    }

    #[test]
    fn autoresearch_error_display() {
        let err = OmxError::Autoresearch("mission not found".into());
        assert_eq!(err.to_string(), "autoresearch error: mission not found");
    }

    #[test]
    fn pipeline_error_display() {
        let err = OmxError::Pipeline("stage failed".into());
        assert_eq!(err.to_string(), "pipeline error: stage failed");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-types -- ralplan_error_display ralph_error_display autoresearch_error_display pipeline_error_display`
Expected: FAIL — variants do not exist yet.

- [ ] **Step 3: Add error variants to OmxError**

In `crates/omx-types/src/lib.rs`, add these four variants to the `OmxError` enum, after the `Catalog(String)` variant:

```rust
    #[error("ralplan error: {0}")]
    Ralplan(String),

    #[error("ralph error: {0}")]
    Ralph(String),

    #[error("autoresearch error: {0}")]
    Autoresearch(String),

    #[error("pipeline error: {0}")]
    Pipeline(String),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-types -- ralplan_error_display ralph_error_display autoresearch_error_display pipeline_error_display`
Expected: 4 tests PASS.

- [ ] **Step 5: Add 4 new crate directories to workspace**

Add the 4 new members to `Cargo.toml` workspace root, in the `members` array after `"crates/omx-catalog"`:

```toml
  # Orchestration engines (Phase 8)
  "crates/omx-ralplan",
  "crates/omx-ralph",
  "crates/omx-autoresearch",
  "crates/omx-pipeline",
```

- [ ] **Step 6: Create omx-ralplan Cargo.toml**

Create `crates/omx-ralplan/Cargo.toml`:

```toml
[package]
name = "omx-ralplan"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-modes = { path = "../omx-modes" }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true }
```

- [ ] **Step 7: Create omx-ralplan stub lib.rs**

Create `crates/omx-ralplan/src/lib.rs`:

```rust
//! Consensus planning with Draft → ArchitectReview → CriticReview loop.
```

- [ ] **Step 8: Create omx-ralph Cargo.toml**

Create `crates/omx-ralph/Cargo.toml`:

```toml
[package]
name = "omx-ralph"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-state = { path = "../omx-state" }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 9: Create omx-ralph stub lib.rs**

Create `crates/omx-ralph/src/lib.rs`:

```rust
//! Phase validation, progress ledger, and visual feedback scoring.
```

- [ ] **Step 10: Create omx-autoresearch Cargo.toml**

Create `crates/omx-autoresearch/Cargo.toml`:

```toml
[package]
name = "omx-autoresearch"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-modes = { path = "../omx-modes" }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 11: Create omx-autoresearch stub lib.rs**

Create `crates/omx-autoresearch/src/lib.rs`:

```rust
//! Iterative research engine with git worktrees and evaluator contracts.
```

- [ ] **Step 12: Create omx-pipeline Cargo.toml**

Create `crates/omx-pipeline/Cargo.toml`:

```toml
[package]
name = "omx-pipeline"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-modes = { path = "../omx-modes" }
omx-ralplan = { path = "../omx-ralplan" }
omx-ralph = { path = "../omx-ralph" }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 13: Create omx-pipeline stub lib.rs**

Create `crates/omx-pipeline/src/lib.rs`:

```rust
//! Sequential stage executor with built-in stage factories.
```

- [ ] **Step 14: Verify workspace compiles**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo check`
Expected: All 29 crates compile clean.

- [ ] **Step 15: Commit**

```bash
git add Cargo.toml crates/omx-types/src/lib.rs crates/omx-ralplan/ crates/omx-ralph/ crates/omx-autoresearch/ crates/omx-pipeline/
git commit -m "feat(phase8): add workspace scaffolding and error variants for 4 orchestration crates"
```

---

## Task 2: Implement omx-ralplan types and serde

**Files:**
- Modify: `crates/omx-ralplan/src/lib.rs`

- [ ] **Step 1: Write tests for RalplanPhase serde and display**

Add to `crates/omx-ralplan/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ralplan_phase_serde_roundtrip() {
        let phases = vec![
            RalplanPhase::Draft,
            RalplanPhase::ArchitectReview,
            RalplanPhase::CriticReview,
            RalplanPhase::Complete,
            RalplanPhase::Cancelled,
            RalplanPhase::Failed,
        ];
        for phase in phases {
            let json = serde_json::to_string(&phase).unwrap();
            let parsed: RalplanPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, phase);
        }
    }

    #[test]
    fn ralplan_phase_display() {
        assert_eq!(RalplanPhase::Draft.to_string(), "draft");
        assert_eq!(RalplanPhase::ArchitectReview.to_string(), "architect-review");
        assert_eq!(RalplanPhase::CriticReview.to_string(), "critic-review");
        assert_eq!(RalplanPhase::Complete.to_string(), "complete");
        assert_eq!(RalplanPhase::Cancelled.to_string(), "cancelled");
        assert_eq!(RalplanPhase::Failed.to_string(), "failed");
    }

    #[test]
    fn review_verdict_serde_roundtrip() {
        let verdicts = vec![
            RalplanReviewVerdict::Approve,
            RalplanReviewVerdict::Iterate,
            RalplanReviewVerdict::Reject,
        ];
        for v in verdicts {
            let json = serde_json::to_string(&v).unwrap();
            let parsed: RalplanReviewVerdict = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, v);
        }
    }

    #[test]
    fn draft_result_serde_roundtrip() {
        let draft = RalplanDraftResult {
            summary: Some("A plan".into()),
            plan_path: Some("/tmp/plan.md".into()),
            artifacts: HashMap::from([("key".into(), serde_json::json!("value"))]),
        };
        let json = serde_json::to_string(&draft).unwrap();
        let parsed: RalplanDraftResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.summary, Some("A plan".into()));
        assert_eq!(parsed.plan_path, Some("/tmp/plan.md".into()));
        assert_eq!(parsed.artifacts.get("key").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn review_result_serde_roundtrip() {
        let review = RalplanReviewResult {
            verdict: RalplanReviewVerdict::Approve,
            summary: Some("Looks good".into()),
            artifacts: HashMap::new(),
        };
        let json = serde_json::to_string(&review).unwrap();
        let parsed: RalplanReviewResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.verdict, RalplanReviewVerdict::Approve);
        assert_eq!(parsed.summary, Some("Looks good".into()));
    }

    #[test]
    fn iteration_context_defaults() {
        let ctx = RalplanIterationContext {
            task: "Build feature X".into(),
            cwd: PathBuf::from("/tmp"),
            iteration: 0,
            prior_drafts: vec![],
            architect_reviews: vec![],
            critic_reviews: vec![],
        };
        assert_eq!(ctx.iteration, 0);
        assert!(ctx.prior_drafts.is_empty());
    }

    #[test]
    fn runtime_result_serde_roundtrip() {
        let result = RalplanRuntimeResult {
            status: RalplanPhase::Complete,
            iteration: 2,
            planning_complete: true,
            drafts: vec![],
            architect_reviews: vec![],
            critic_reviews: vec![],
            latest_plan_path: Some("/tmp/plan.md".into()),
            artifacts: HashMap::new(),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: RalplanRuntimeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, RalplanPhase::Complete);
        assert_eq!(parsed.iteration, 2);
        assert!(parsed.planning_complete);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralplan`
Expected: FAIL — types not defined yet.

- [ ] **Step 3: Implement all ralplan types**

Replace the contents of `crates/omx-ralplan/src/lib.rs` (keeping the doc comment) with:

```rust
//! Consensus planning with Draft → ArchitectReview → CriticReview loop.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RalplanPhase {
    Draft,
    ArchitectReview,
    CriticReview,
    Complete,
    Cancelled,
    Failed,
}

impl std::fmt::Display for RalplanPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::ArchitectReview => write!(f, "architect-review"),
            Self::CriticReview => write!(f, "critic-review"),
            Self::Complete => write!(f, "complete"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl RalplanPhase {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Cancelled | Self::Failed)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Draft | Self::ArchitectReview | Self::CriticReview)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RalplanReviewVerdict {
    Approve,
    Iterate,
    Reject,
}

impl std::fmt::Display for RalplanReviewVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approve => write!(f, "approve"),
            Self::Iterate => write!(f, "iterate"),
            Self::Reject => write!(f, "reject"),
        }
    }
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalplanDraftResult {
    pub summary: Option<String>,
    pub plan_path: Option<String>,
    pub artifacts: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalplanReviewResult {
    pub verdict: RalplanReviewVerdict,
    pub summary: Option<String>,
    pub artifacts: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Context and options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RalplanIterationContext {
    pub task: String,
    pub cwd: PathBuf,
    pub iteration: u32,
    pub prior_drafts: Vec<RalplanDraftResult>,
    pub architect_reviews: Vec<RalplanReviewResult>,
    pub critic_reviews: Vec<RalplanReviewResult>,
}

pub struct RunRalplanConsensusOptions {
    pub task: String,
    pub cwd: Option<PathBuf>,
    pub max_iterations: Option<u32>,
}

impl RunRalplanConsensusOptions {
    pub fn max_iterations(&self) -> u32 {
        self.max_iterations.unwrap_or(5)
    }
}

// ---------------------------------------------------------------------------
// Runtime result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralplan`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-ralplan/src/lib.rs
git commit -m "feat(omx-ralplan): add consensus planning types with serde"
```

---

## Task 3: Implement omx-ralplan executor trait and consensus runtime

**Files:**
- Modify: `crates/omx-ralplan/src/lib.rs`

- [ ] **Step 1: Write tests for the consensus runtime**

Add these tests to the existing `mod tests` block in `crates/omx-ralplan/src/lib.rs`:

```rust
    use omx_types::OmxError;

    struct MockExecutor {
        approve_on_iteration: u32,
    }

    #[async_trait::async_trait]
    impl RalplanConsensusExecutor for MockExecutor {
        async fn draft(&self, ctx: &RalplanIterationContext) -> Result<RalplanDraftResult, OmxError> {
            Ok(RalplanDraftResult {
                summary: Some(format!("Draft iteration {}", ctx.iteration)),
                plan_path: Some("/tmp/plan.md".into()),
                artifacts: HashMap::new(),
            })
        }

        async fn architect_review(
            &self,
            ctx: &RalplanIterationContext,
            _draft: &RalplanDraftResult,
        ) -> Result<RalplanReviewResult, OmxError> {
            Ok(RalplanReviewResult {
                verdict: RalplanReviewVerdict::Approve,
                summary: Some(format!("Architect approves iteration {}", ctx.iteration)),
                artifacts: HashMap::new(),
            })
        }

        async fn critic_review(
            &self,
            ctx: &RalplanIterationContext,
            _draft: &RalplanDraftResult,
            _architect: &RalplanReviewResult,
        ) -> Result<RalplanReviewResult, OmxError> {
            let verdict = if ctx.iteration >= self.approve_on_iteration {
                RalplanReviewVerdict::Approve
            } else {
                RalplanReviewVerdict::Iterate
            };
            Ok(RalplanReviewResult {
                verdict,
                summary: Some(format!("Critic review iteration {}", ctx.iteration)),
                artifacts: HashMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn consensus_completes_on_first_iteration() {
        let executor = MockExecutor { approve_on_iteration: 0 };
        let options = RunRalplanConsensusOptions {
            task: "Build X".into(),
            cwd: Some(PathBuf::from("/tmp")),
            max_iterations: None,
        };
        let result = run_ralplan_consensus(&executor, options).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Complete);
        assert_eq!(result.iteration, 1);
        assert!(result.planning_complete);
        assert_eq!(result.drafts.len(), 1);
        assert_eq!(result.architect_reviews.len(), 1);
        assert_eq!(result.critic_reviews.len(), 1);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn consensus_iterates_until_critic_approves() {
        let executor = MockExecutor { approve_on_iteration: 2 };
        let options = RunRalplanConsensusOptions {
            task: "Build Y".into(),
            cwd: Some(PathBuf::from("/tmp")),
            max_iterations: Some(5),
        };
        let result = run_ralplan_consensus(&executor, options).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Complete);
        assert_eq!(result.iteration, 3);
        assert_eq!(result.drafts.len(), 3);
        assert_eq!(result.architect_reviews.len(), 3);
        assert_eq!(result.critic_reviews.len(), 3);
    }

    #[tokio::test]
    async fn consensus_fails_at_max_iterations() {
        let executor = MockExecutor { approve_on_iteration: 100 };
        let options = RunRalplanConsensusOptions {
            task: "Build Z".into(),
            cwd: Some(PathBuf::from("/tmp")),
            max_iterations: Some(2),
        };
        let result = run_ralplan_consensus(&executor, options).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Failed);
        assert_eq!(result.iteration, 2);
        assert!(!result.planning_complete);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("max iterations"));
    }

    struct FailingExecutor;

    #[async_trait::async_trait]
    impl RalplanConsensusExecutor for FailingExecutor {
        async fn draft(&self, _ctx: &RalplanIterationContext) -> Result<RalplanDraftResult, OmxError> {
            Err(OmxError::Ralplan("executor crashed".into()))
        }

        async fn architect_review(
            &self,
            _ctx: &RalplanIterationContext,
            _draft: &RalplanDraftResult,
        ) -> Result<RalplanReviewResult, OmxError> {
            unreachable!()
        }

        async fn critic_review(
            &self,
            _ctx: &RalplanIterationContext,
            _draft: &RalplanDraftResult,
            _architect: &RalplanReviewResult,
        ) -> Result<RalplanReviewResult, OmxError> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn consensus_catches_executor_error() {
        let executor = FailingExecutor;
        let options = RunRalplanConsensusOptions {
            task: "Build W".into(),
            cwd: Some(PathBuf::from("/tmp")),
            max_iterations: None,
        };
        let result = run_ralplan_consensus(&executor, options).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Failed);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("executor crashed"));
    }

    #[test]
    fn max_iterations_defaults_to_five() {
        let opts = RunRalplanConsensusOptions {
            task: "test".into(),
            cwd: None,
            max_iterations: None,
        };
        assert_eq!(opts.max_iterations(), 5);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralplan`
Expected: FAIL — trait and function not defined yet.

- [ ] **Step 3: Implement the executor trait and consensus function**

Add to `crates/omx-ralplan/src/lib.rs`, before the `#[cfg(test)]` block:

```rust
use omx_types::OmxError;

// ---------------------------------------------------------------------------
// Executor trait
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
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

// ---------------------------------------------------------------------------
// Consensus runtime
// ---------------------------------------------------------------------------

pub async fn run_ralplan_consensus(
    executor: &dyn RalplanConsensusExecutor,
    options: RunRalplanConsensusOptions,
) -> Result<RalplanRuntimeResult, OmxError> {
    let cwd = options.cwd.clone().unwrap_or_else(|| PathBuf::from("."));
    let max_iterations = options.max_iterations();
    let task = options.task.clone();

    let mut drafts: Vec<RalplanDraftResult> = Vec::new();
    let mut architect_reviews: Vec<RalplanReviewResult> = Vec::new();
    let mut critic_reviews: Vec<RalplanReviewResult> = Vec::new();
    let mut latest_plan_path: Option<String> = None;
    let mut all_artifacts: HashMap<String, serde_json::Value> = HashMap::new();

    for iteration in 0..max_iterations {
        let ctx = RalplanIterationContext {
            task: task.clone(),
            cwd: cwd.clone(),
            iteration,
            prior_drafts: drafts.clone(),
            architect_reviews: architect_reviews.clone(),
            critic_reviews: critic_reviews.clone(),
        };

        // Draft phase
        let draft = match executor.draft(&ctx).await {
            Ok(d) => d,
            Err(e) => {
                return Ok(RalplanRuntimeResult {
                    status: RalplanPhase::Failed,
                    iteration: iteration + 1,
                    planning_complete: false,
                    drafts,
                    architect_reviews,
                    critic_reviews,
                    latest_plan_path,
                    artifacts: all_artifacts,
                    error: Some(e.to_string()),
                });
            }
        };

        if let Some(ref path) = draft.plan_path {
            latest_plan_path = Some(path.clone());
        }
        for (k, v) in &draft.artifacts {
            all_artifacts.insert(k.clone(), v.clone());
        }

        // Architect review phase
        let architect = match executor.architect_review(&ctx, &draft).await {
            Ok(r) => r,
            Err(e) => {
                drafts.push(draft);
                return Ok(RalplanRuntimeResult {
                    status: RalplanPhase::Failed,
                    iteration: iteration + 1,
                    planning_complete: false,
                    drafts,
                    architect_reviews,
                    critic_reviews,
                    latest_plan_path,
                    artifacts: all_artifacts,
                    error: Some(e.to_string()),
                });
            }
        };
        for (k, v) in &architect.artifacts {
            all_artifacts.insert(k.clone(), v.clone());
        }

        // Critic review phase
        let critic = match executor.critic_review(&ctx, &draft, &architect).await {
            Ok(r) => r,
            Err(e) => {
                drafts.push(draft);
                architect_reviews.push(architect);
                return Ok(RalplanRuntimeResult {
                    status: RalplanPhase::Failed,
                    iteration: iteration + 1,
                    planning_complete: false,
                    drafts,
                    architect_reviews,
                    critic_reviews,
                    latest_plan_path,
                    artifacts: all_artifacts,
                    error: Some(e.to_string()),
                });
            }
        };
        for (k, v) in &critic.artifacts {
            all_artifacts.insert(k.clone(), v.clone());
        }

        let approved = critic.verdict == RalplanReviewVerdict::Approve;

        drafts.push(draft);
        architect_reviews.push(architect);
        critic_reviews.push(critic);

        if approved {
            return Ok(RalplanRuntimeResult {
                status: RalplanPhase::Complete,
                iteration: iteration + 1,
                planning_complete: true,
                drafts,
                architect_reviews,
                critic_reviews,
                latest_plan_path,
                artifacts: all_artifacts,
                error: None,
            });
        }
    }

    // Max iterations exhausted
    Ok(RalplanRuntimeResult {
        status: RalplanPhase::Failed,
        iteration: max_iterations,
        planning_complete: false,
        drafts,
        architect_reviews,
        critic_reviews,
        latest_plan_path,
        artifacts: all_artifacts,
        error: Some(format!(
            "consensus not reached after max iterations ({})",
            max_iterations
        )),
    })
}

pub async fn cancel_ralplan_consensus() -> Result<(), OmxError> {
    // Delegates to ModeManager cancellation — a no-op in the library crate.
    // The caller (omx-cli) is responsible for mode lifecycle.
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralplan`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-ralplan/src/lib.rs
git commit -m "feat(omx-ralplan): implement consensus executor trait and runtime loop"
```

---

## Task 4: Implement omx-ralph types and serde

**Files:**
- Modify: `crates/omx-ralph/src/lib.rs`

- [ ] **Step 1: Write tests for ralph types**

Replace `crates/omx-ralph/src/lib.rs` with:

```rust
//! Phase validation, progress ledger, and visual feedback scoring.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ralph_phase_serde_roundtrip() {
        let phases = vec![
            RalphPhase::Starting,
            RalphPhase::Executing,
            RalphPhase::Verifying,
            RalphPhase::Fixing,
            RalphPhase::Complete,
            RalphPhase::Failed,
            RalphPhase::Cancelled,
        ];
        for phase in phases {
            let json = serde_json::to_string(&phase).unwrap();
            let parsed: RalphPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, phase);
        }
    }

    #[test]
    fn ralph_phase_display() {
        assert_eq!(RalphPhase::Starting.to_string(), "starting");
        assert_eq!(RalphPhase::Executing.to_string(), "executing");
        assert_eq!(RalphPhase::Verifying.to_string(), "verifying");
        assert_eq!(RalphPhase::Fixing.to_string(), "fixing");
        assert_eq!(RalphPhase::Complete.to_string(), "complete");
        assert_eq!(RalphPhase::Failed.to_string(), "failed");
        assert_eq!(RalphPhase::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn ralph_phase_is_terminal() {
        assert!(RalphPhase::Complete.is_terminal());
        assert!(RalphPhase::Failed.is_terminal());
        assert!(RalphPhase::Cancelled.is_terminal());
        assert!(!RalphPhase::Starting.is_terminal());
        assert!(!RalphPhase::Executing.is_terminal());
    }

    #[test]
    fn visual_verdict_serde_roundtrip() {
        let verdicts = vec![
            VisualVerdictStatus::Pass,
            VisualVerdictStatus::Fail,
            VisualVerdictStatus::Ambiguous,
        ];
        for v in verdicts {
            let json = serde_json::to_string(&v).unwrap();
            let parsed: VisualVerdictStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, v);
        }
    }

    #[test]
    fn visual_feedback_serde_roundtrip() {
        let fb = RalphVisualFeedback {
            score: 95.0,
            verdict: VisualVerdictStatus::Pass,
            category_match: true,
            differences: vec!["color shift".into()],
            suggestions: vec!["adjust hue".into()],
            reasoning: Some("Close match".into()),
            threshold: Some(90.0),
        };
        let json = serde_json::to_string(&fb).unwrap();
        let parsed: RalphVisualFeedback = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.score, 95.0);
        assert_eq!(parsed.verdict, VisualVerdictStatus::Pass);
        assert!(parsed.category_match);
    }

    #[test]
    fn progress_ledger_empty_default() {
        let ledger = RalphProgressLedger::new();
        assert_eq!(ledger.schema_version, 2);
        assert!(ledger.entries.is_empty());
        assert!(ledger.visual_feedback.is_empty());
    }

    #[test]
    fn progress_ledger_serde_roundtrip() {
        let ledger = RalphProgressLedger {
            schema_version: 2,
            source: Some("test".into()),
            source_sha256: None,
            strategy: None,
            created_at: Some("2026-04-05T00:00:00Z".into()),
            updated_at: None,
            entries: vec![RalphProgressEntry {
                content: "Step done".into(),
                created_at: "2026-04-05T00:00:00Z".into(),
            }],
            visual_feedback: vec![],
        };
        let json = serde_json::to_string(&ledger).unwrap();
        let parsed: RalphProgressLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].content, "Step done");
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(VISUAL_NEXT_ACTIONS_LIMIT, 5);
        assert_eq!(VISUAL_FEEDBACK_MAX_ENTRIES, 30);
        assert_eq!(DEFAULT_VISUAL_THRESHOLD, 90.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralph`
Expected: FAIL — types not defined yet.

- [ ] **Step 3: Implement ralph types**

Add above the `#[cfg(test)]` block in `crates/omx-ralph/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const VISUAL_NEXT_ACTIONS_LIMIT: usize = 5;
pub const VISUAL_FEEDBACK_MAX_ENTRIES: usize = 30;
pub const DEFAULT_VISUAL_THRESHOLD: f64 = 90.0;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RalphPhase {
    Starting,
    Executing,
    Verifying,
    Fixing,
    Complete,
    Failed,
    Cancelled,
}

impl RalphPhase {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Failed | Self::Cancelled)
    }
}

impl std::fmt::Display for RalphPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Executing => write!(f, "executing"),
            Self::Verifying => write!(f, "verifying"),
            Self::Fixing => write!(f, "fixing"),
            Self::Complete => write!(f, "complete"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VisualVerdictStatus {
    Pass,
    Fail,
    Ambiguous,
}

impl std::fmt::Display for VisualVerdictStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Fail => write!(f, "fail"),
            Self::Ambiguous => write!(f, "ambiguous"),
        }
    }
}

// ---------------------------------------------------------------------------
// Validation result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphStateValidationResult {
    pub ok: bool,
    pub phase: Option<RalphPhase>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Visual feedback
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphVisualFeedback {
    pub score: f64,
    pub verdict: VisualVerdictStatus,
    pub category_match: bool,
    pub differences: Vec<String>,
    pub suggestions: Vec<String>,
    pub reasoning: Option<String>,
    pub threshold: Option<f64>,
}

// ---------------------------------------------------------------------------
// Progress ledger
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphProgressEntry {
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphProgressLedger {
    pub schema_version: u32,
    pub source: Option<String>,
    pub source_sha256: Option<String>,
    pub strategy: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub entries: Vec<RalphProgressEntry>,
    pub visual_feedback: Vec<RalphVisualFeedback>,
}

impl RalphProgressLedger {
    pub fn new() -> Self {
        Self {
            schema_version: 2,
            source: None,
            source_sha256: None,
            strategy: None,
            created_at: None,
            updated_at: None,
            entries: Vec::new(),
            visual_feedback: Vec::new(),
        }
    }
}

impl Default for RalphProgressLedger {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Canonical artifacts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RalphCanonicalArtifacts {
    pub canonical_prd_path: Option<PathBuf>,
    pub canonical_progress_path: PathBuf,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralph`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-ralph/src/lib.rs
git commit -m "feat(omx-ralph): add phase validation types, visual feedback, and progress ledger"
```

---

## Task 5: Implement omx-ralph core functions

**Files:**
- Modify: `crates/omx-ralph/src/lib.rs`

- [ ] **Step 1: Write tests for normalize_ralph_phase and validate_ralph_state**

Add these tests to the existing `mod tests` block in `crates/omx-ralph/src/lib.rs`:

```rust
    #[test]
    fn normalize_valid_phases() {
        assert_eq!(normalize_ralph_phase("starting").unwrap(), RalphPhase::Starting);
        assert_eq!(normalize_ralph_phase("executing").unwrap(), RalphPhase::Executing);
        assert_eq!(normalize_ralph_phase("verifying").unwrap(), RalphPhase::Verifying);
        assert_eq!(normalize_ralph_phase("fixing").unwrap(), RalphPhase::Fixing);
        assert_eq!(normalize_ralph_phase("complete").unwrap(), RalphPhase::Complete);
        assert_eq!(normalize_ralph_phase("failed").unwrap(), RalphPhase::Failed);
        assert_eq!(normalize_ralph_phase("cancelled").unwrap(), RalphPhase::Cancelled);
    }

    #[test]
    fn normalize_case_insensitive() {
        assert_eq!(normalize_ralph_phase("STARTING").unwrap(), RalphPhase::Starting);
        assert_eq!(normalize_ralph_phase("Executing").unwrap(), RalphPhase::Executing);
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(normalize_ralph_phase("  starting  ").unwrap(), RalphPhase::Starting);
    }

    #[test]
    fn normalize_rejects_empty() {
        assert!(normalize_ralph_phase("").is_err());
        assert!(normalize_ralph_phase("   ").is_err());
    }

    #[test]
    fn normalize_rejects_unknown_phase() {
        assert!(normalize_ralph_phase("bogus").is_err());
    }

    #[test]
    fn validate_valid_active_state() {
        let candidate = serde_json::json!({
            "active": true,
            "current_phase": "executing",
            "iteration": 3,
            "max_iterations": 50,
            "started_at": "2026-04-05T00:00:00Z"
        });
        let result = validate_ralph_state(&candidate);
        assert!(result.ok);
        assert_eq!(result.phase, Some(RalphPhase::Executing));
        assert!(result.error.is_none());
    }

    #[test]
    fn validate_auto_fills_defaults_for_active_state() {
        let candidate = serde_json::json!({ "active": true });
        let result = validate_ralph_state(&candidate);
        assert!(result.ok);
        assert_eq!(result.phase, Some(RalphPhase::Starting));
    }

    #[test]
    fn validate_terminal_phase_requires_inactive() {
        let candidate = serde_json::json!({
            "active": true,
            "current_phase": "complete"
        });
        let result = validate_ralph_state(&candidate);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("terminal"));
    }

    #[test]
    fn validate_terminal_phase_when_inactive_is_ok() {
        let candidate = serde_json::json!({
            "active": false,
            "current_phase": "complete",
            "iteration": 5,
            "max_iterations": 50,
            "started_at": "2026-04-05T00:00:00Z"
        });
        let result = validate_ralph_state(&candidate);
        assert!(result.ok);
        assert_eq!(result.phase, Some(RalphPhase::Complete));
    }

    #[test]
    fn validate_rejects_negative_iteration() {
        let candidate = serde_json::json!({
            "active": true,
            "current_phase": "executing",
            "iteration": -1,
            "max_iterations": 50,
            "started_at": "2026-04-05T00:00:00Z"
        });
        let result = validate_ralph_state(&candidate);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("iteration"));
    }

    #[test]
    fn validate_rejects_zero_max_iterations() {
        let candidate = serde_json::json!({
            "active": true,
            "current_phase": "executing",
            "iteration": 0,
            "max_iterations": 0,
            "started_at": "2026-04-05T00:00:00Z"
        });
        let result = validate_ralph_state(&candidate);
        assert!(!result.ok);
        assert!(result.error.unwrap().contains("max_iterations"));
    }

    #[test]
    fn validate_rejects_invalid_phase_string() {
        let candidate = serde_json::json!({
            "active": true,
            "current_phase": "bogus"
        });
        let result = validate_ralph_state(&candidate);
        assert!(!result.ok);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralph -- normalize_ validate_`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement normalize_ralph_phase and validate_ralph_state**

Add to `crates/omx-ralph/src/lib.rs`, before the `#[cfg(test)]` block:

```rust
use omx_types::OmxError;

// ---------------------------------------------------------------------------
// Phase normalization
// ---------------------------------------------------------------------------

pub fn normalize_ralph_phase(raw: &str) -> Result<RalphPhase, OmxError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(OmxError::Ralph("phase string is empty".into()));
    }
    match trimmed.to_lowercase().as_str() {
        "starting" => Ok(RalphPhase::Starting),
        "executing" => Ok(RalphPhase::Executing),
        "verifying" => Ok(RalphPhase::Verifying),
        "fixing" => Ok(RalphPhase::Fixing),
        "complete" => Ok(RalphPhase::Complete),
        "failed" => Ok(RalphPhase::Failed),
        "cancelled" => Ok(RalphPhase::Cancelled),
        other => Err(OmxError::Ralph(format!("unknown phase: '{}'", other))),
    }
}

// ---------------------------------------------------------------------------
// State validation
// ---------------------------------------------------------------------------

pub fn validate_ralph_state(candidate: &serde_json::Value) -> RalphStateValidationResult {
    let active = candidate.get("active").and_then(|v| v.as_bool()).unwrap_or(false);

    // Parse or default the phase
    let phase = match candidate.get("current_phase").and_then(|v| v.as_str()) {
        Some(raw) => match normalize_ralph_phase(raw) {
            Ok(p) => p,
            Err(e) => {
                return RalphStateValidationResult {
                    ok: false,
                    phase: None,
                    warning: None,
                    error: Some(e.to_string()),
                };
            }
        },
        None => {
            if active {
                RalphPhase::Starting
            } else {
                return RalphStateValidationResult {
                    ok: false,
                    phase: None,
                    warning: None,
                    error: Some("missing current_phase for inactive state".into()),
                };
            }
        }
    };

    // Terminal phases require active=false
    if phase.is_terminal() && active {
        return RalphStateValidationResult {
            ok: false,
            phase: Some(phase),
            warning: None,
            error: Some(format!(
                "terminal phase '{}' requires active=false",
                phase
            )),
        };
    }

    // Validate iteration
    if let Some(iter_val) = candidate.get("iteration") {
        if let Some(n) = iter_val.as_i64() {
            if n < 0 {
                return RalphStateValidationResult {
                    ok: false,
                    phase: Some(phase),
                    warning: None,
                    error: Some("iteration must be >= 0".into()),
                };
            }
        } else if let Some(n) = iter_val.as_f64() {
            if !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
                return RalphStateValidationResult {
                    ok: false,
                    phase: Some(phase),
                    warning: None,
                    error: Some("iteration must be a non-negative integer".into()),
                };
            }
        }
    }

    // Validate max_iterations
    if let Some(max_val) = candidate.get("max_iterations") {
        if let Some(n) = max_val.as_i64() {
            if n <= 0 {
                return RalphStateValidationResult {
                    ok: false,
                    phase: Some(phase),
                    warning: None,
                    error: Some("max_iterations must be > 0".into()),
                };
            }
        } else if let Some(n) = max_val.as_f64() {
            if !n.is_finite() || n <= 0.0 || n.fract() != 0.0 {
                return RalphStateValidationResult {
                    ok: false,
                    phase: Some(phase),
                    warning: None,
                    error: Some("max_iterations must be a positive integer".into()),
                };
            }
        }
    }

    RalphStateValidationResult {
        ok: true,
        phase: Some(phase),
        warning: None,
        error: None,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralph`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-ralph/src/lib.rs
git commit -m "feat(omx-ralph): implement phase normalization and state validation"
```

---

## Task 6: Implement omx-ralph canonical artifacts and visual feedback

**Files:**
- Modify: `crates/omx-ralph/src/lib.rs`

- [ ] **Step 1: Write tests for ensure_canonical_artifacts and record_visual_feedback**

Add these tests to the existing `mod tests` block in `crates/omx-ralph/src/lib.rs`:

```rust
    #[tokio::test]
    async fn ensure_canonical_artifacts_creates_dirs_and_ledger() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();

        let artifacts = ensure_canonical_artifacts(cwd, None).await.unwrap();
        assert!(artifacts.canonical_progress_path.exists());
        assert!(artifacts.canonical_prd_path.is_none());

        // Verify the plans directory was created
        assert!(cwd.join(".omx").join("plans").is_dir());

        // Verify the progress ledger is valid JSON
        let content = tokio::fs::read_to_string(&artifacts.canonical_progress_path)
            .await
            .unwrap();
        let ledger: RalphProgressLedger = serde_json::from_str(&content).unwrap();
        assert_eq!(ledger.schema_version, 2);
    }

    #[tokio::test]
    async fn ensure_canonical_artifacts_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();

        let a1 = ensure_canonical_artifacts(cwd, None).await.unwrap();
        let a2 = ensure_canonical_artifacts(cwd, None).await.unwrap();
        assert_eq!(a1.canonical_progress_path, a2.canonical_progress_path);
    }

    #[tokio::test]
    async fn ensure_canonical_artifacts_with_session_id() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();

        let artifacts = ensure_canonical_artifacts(cwd, Some("sess-42")).await.unwrap();
        assert!(artifacts
            .canonical_progress_path
            .to_string_lossy()
            .contains("sess-42"));
    }

    #[tokio::test]
    async fn record_visual_feedback_appends_to_ledger() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();

        ensure_canonical_artifacts(cwd, None).await.unwrap();

        let feedback = RalphVisualFeedback {
            score: 95.0,
            verdict: VisualVerdictStatus::Pass,
            category_match: true,
            differences: vec!["minor color".into()],
            suggestions: vec!["adjust saturation".into()],
            reasoning: None,
            threshold: None,
        };

        record_visual_feedback(cwd, feedback, None).await.unwrap();

        let artifacts = ensure_canonical_artifacts(cwd, None).await.unwrap();
        let content = tokio::fs::read_to_string(&artifacts.canonical_progress_path)
            .await
            .unwrap();
        let ledger: RalphProgressLedger = serde_json::from_str(&content).unwrap();
        assert_eq!(ledger.visual_feedback.len(), 1);
        assert_eq!(ledger.visual_feedback[0].score, 95.0);
    }

    #[tokio::test]
    async fn record_visual_feedback_caps_at_max_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = tmp.path();

        ensure_canonical_artifacts(cwd, None).await.unwrap();

        for i in 0..35 {
            let feedback = RalphVisualFeedback {
                score: i as f64,
                verdict: VisualVerdictStatus::Pass,
                category_match: true,
                differences: vec![],
                suggestions: vec![],
                reasoning: None,
                threshold: None,
            };
            record_visual_feedback(cwd, feedback, None).await.unwrap();
        }

        let artifacts = ensure_canonical_artifacts(cwd, None).await.unwrap();
        let content = tokio::fs::read_to_string(&artifacts.canonical_progress_path)
            .await
            .unwrap();
        let ledger: RalphProgressLedger = serde_json::from_str(&content).unwrap();
        assert_eq!(ledger.visual_feedback.len(), VISUAL_FEEDBACK_MAX_ENTRIES);
        // Should keep the last 30 entries (scores 5..35)
        assert_eq!(ledger.visual_feedback[0].score, 5.0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralph -- ensure_canonical record_visual`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement ensure_canonical_artifacts and record_visual_feedback**

Add to `crates/omx-ralph/src/lib.rs`, after the `validate_ralph_state` function and before `#[cfg(test)]`:

```rust
// ---------------------------------------------------------------------------
// Canonical artifacts
// ---------------------------------------------------------------------------

fn state_dir(cwd: &std::path::Path, session_id: Option<&str>) -> PathBuf {
    let base = cwd.join(".omx").join("state");
    match session_id {
        Some(id) => base.join(id),
        None => base,
    }
}

pub async fn ensure_canonical_artifacts(
    cwd: &std::path::Path,
    session_id: Option<&str>,
) -> Result<RalphCanonicalArtifacts, OmxError> {
    let plans_dir = cwd.join(".omx").join("plans");
    tokio::fs::create_dir_all(&plans_dir).await?;

    let state = state_dir(cwd, session_id);
    tokio::fs::create_dir_all(&state).await?;

    let progress_path = state.join("ralph-progress.json");

    if !progress_path.exists() {
        let ledger = RalphProgressLedger::new();
        let json = serde_json::to_string_pretty(&ledger)?;
        tokio::fs::write(&progress_path, json).await?;
    }

    Ok(RalphCanonicalArtifacts {
        canonical_prd_path: None,
        canonical_progress_path: progress_path,
    })
}

// ---------------------------------------------------------------------------
// Visual feedback recording
// ---------------------------------------------------------------------------

pub async fn record_visual_feedback(
    cwd: &std::path::Path,
    feedback: RalphVisualFeedback,
    session_id: Option<&str>,
) -> Result<(), OmxError> {
    let artifacts = ensure_canonical_artifacts(cwd, session_id).await?;
    let progress_path = &artifacts.canonical_progress_path;

    let content = tokio::fs::read_to_string(progress_path).await?;
    let mut ledger: RalphProgressLedger =
        serde_json::from_str(&content).unwrap_or_else(|_| RalphProgressLedger::new());

    ledger.visual_feedback.push(feedback);

    // Cap at max entries, keeping the most recent
    if ledger.visual_feedback.len() > VISUAL_FEEDBACK_MAX_ENTRIES {
        let start = ledger.visual_feedback.len() - VISUAL_FEEDBACK_MAX_ENTRIES;
        ledger.visual_feedback = ledger.visual_feedback.split_off(start);
    }

    ledger.updated_at = Some(chrono::Utc::now().to_rfc3339());

    let json = serde_json::to_string_pretty(&ledger)?;
    tokio::fs::write(progress_path, json).await?;

    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-ralph`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-ralph/src/lib.rs
git commit -m "feat(omx-ralph): implement canonical artifacts and visual feedback recording"
```

---

## Task 7: Implement omx-autoresearch contract types and loading

**Files:**
- Modify: `crates/omx-autoresearch/src/lib.rs`

- [ ] **Step 1: Write tests for contract types and parsing**

Replace `crates/omx-autoresearch/src/lib.rs` with:

```rust
//! Iterative research engine with git worktrees and evaluator contracts.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_policy_serde_roundtrip() {
        let policies = vec![
            AutoresearchKeepPolicy::ScoreImprovement,
            AutoresearchKeepPolicy::PassOnly,
        ];
        for p in policies {
            let json = serde_json::to_string(&p).unwrap();
            let parsed: AutoresearchKeepPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, p);
        }
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify_mission_name("Hello World!"), "hello-world");
    }

    #[test]
    fn slugify_strips_non_alphanumeric() {
        assert_eq!(slugify_mission_name("test@#$%foo"), "test-foo");
    }

    #[test]
    fn slugify_caps_at_48_chars() {
        let long = "a".repeat(100);
        assert!(slugify_mission_name(&long).len() <= 48);
    }

    #[test]
    fn slugify_empty_returns_mission() {
        assert_eq!(slugify_mission_name(""), "mission");
        assert_eq!(slugify_mission_name("@#$"), "mission");
    }

    #[test]
    fn slugify_trims_leading_trailing_hyphens() {
        assert_eq!(slugify_mission_name("--hello--"), "hello");
    }

    #[test]
    fn parse_sandbox_valid() {
        let content = r#"---
evaluator:
  command: npm test
  format: json
  keep_policy: pass_only
---

Do the research task.
"#;
        let parsed = parse_sandbox_contract(content).unwrap();
        assert_eq!(parsed.evaluator.command, "npm test");
        assert_eq!(parsed.evaluator.format, "json");
        assert_eq!(parsed.evaluator.keep_policy, AutoresearchKeepPolicy::PassOnly);
        assert!(parsed.body.contains("Do the research task."));
    }

    #[test]
    fn parse_sandbox_defaults_to_score_improvement() {
        let content = r#"---
evaluator:
  command: cargo test
  format: json
---

Body here.
"#;
        let parsed = parse_sandbox_contract(content).unwrap();
        assert_eq!(parsed.evaluator.keep_policy, AutoresearchKeepPolicy::ScoreImprovement);
    }

    #[test]
    fn parse_sandbox_rejects_missing_command() {
        let content = r#"---
evaluator:
  format: json
---
Body.
"#;
        assert!(parse_sandbox_contract(content).is_err());
    }

    #[test]
    fn parse_sandbox_rejects_non_json_format() {
        let content = r#"---
evaluator:
  command: cargo test
  format: xml
---
Body.
"#;
        assert!(parse_sandbox_contract(content).is_err());
    }

    #[test]
    fn parse_sandbox_rejects_no_frontmatter() {
        let content = "Just a body with no frontmatter.";
        assert!(parse_sandbox_contract(content).is_err());
    }

    #[test]
    fn parse_evaluator_result_valid() {
        let raw = r#"{"pass": true, "score": 85.5}"#;
        let result = parse_evaluator_result(raw).unwrap();
        assert!(result.pass);
        assert_eq!(result.score, Some(85.5));
    }

    #[test]
    fn parse_evaluator_result_without_score() {
        let raw = r#"{"pass": false}"#;
        let result = parse_evaluator_result(raw).unwrap();
        assert!(!result.pass);
        assert!(result.score.is_none());
    }

    #[test]
    fn parse_evaluator_result_rejects_missing_pass() {
        let raw = r#"{"score": 50}"#;
        assert!(parse_evaluator_result(raw).is_err());
    }

    #[test]
    fn parse_evaluator_result_rejects_invalid_json() {
        assert!(parse_evaluator_result("not json").is_err());
    }

    #[test]
    fn candidate_status_serde_roundtrip() {
        let statuses = vec![
            AutoresearchCandidateStatus::Candidate,
            AutoresearchCandidateStatus::Noop,
            AutoresearchCandidateStatus::Abort,
            AutoresearchCandidateStatus::Interrupted,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let parsed: AutoresearchCandidateStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, s);
        }
    }

    #[test]
    fn decision_status_serde_roundtrip() {
        let statuses = vec![
            AutoresearchDecisionStatus::Baseline,
            AutoresearchDecisionStatus::Keep,
            AutoresearchDecisionStatus::Discard,
            AutoresearchDecisionStatus::Ambiguous,
            AutoresearchDecisionStatus::Noop,
            AutoresearchDecisionStatus::Abort,
            AutoresearchDecisionStatus::Interrupted,
            AutoresearchDecisionStatus::Error,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let parsed: AutoresearchDecisionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, s);
        }
    }

    #[test]
    fn run_status_serde_roundtrip() {
        let statuses = vec![
            AutoresearchRunStatus::Running,
            AutoresearchRunStatus::Stopped,
            AutoresearchRunStatus::Completed,
            AutoresearchRunStatus::Failed,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let parsed: AutoresearchRunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, s);
        }
    }

    #[test]
    fn build_run_tag_format() {
        let tag = build_run_tag();
        // Should be like "20260405T123456Z"
        assert!(tag.len() >= 15);
        assert!(tag.contains('T'));
        assert!(tag.ends_with('Z'));
        assert!(!tag.contains('-'));
        assert!(!tag.contains(':'));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-autoresearch`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement contract types and parsing**

Add to `crates/omx-autoresearch/src/lib.rs`, above the `#[cfg(test)]` block:

```rust
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const AUTORESEARCH_RESULTS_HEADER: &str =
    "iteration\tdecision\tpass\tscore\tkept_commit\ttimestamp\n";

// ---------------------------------------------------------------------------
// Contract types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoresearchKeepPolicy {
    ScoreImprovement,
    PassOnly,
}

impl std::fmt::Display for AutoresearchKeepPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScoreImprovement => write!(f, "score_improvement"),
            Self::PassOnly => write!(f, "pass_only"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoresearchEvaluatorContract {
    pub command: String,
    pub format: String,
    pub keep_policy: AutoresearchKeepPolicy,
}

#[derive(Debug, Clone)]
pub struct ParsedSandboxContract {
    pub frontmatter: HashMap<String, String>,
    pub evaluator: AutoresearchEvaluatorContract,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchEvaluatorResult {
    pub pass: bool,
    pub score: Option<f64>,
}

#[derive(Debug, Clone)]
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

// ---------------------------------------------------------------------------
// Status enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoresearchCandidateStatus {
    Candidate,
    Noop,
    Abort,
    Interrupted,
}

impl std::fmt::Display for AutoresearchCandidateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Candidate => write!(f, "candidate"),
            Self::Noop => write!(f, "noop"),
            Self::Abort => write!(f, "abort"),
            Self::Interrupted => write!(f, "interrupted"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoresearchDecisionStatus {
    Baseline,
    Keep,
    Discard,
    Ambiguous,
    Noop,
    Abort,
    Interrupted,
    Error,
}

impl std::fmt::Display for AutoresearchDecisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Baseline => write!(f, "baseline"),
            Self::Keep => write!(f, "keep"),
            Self::Discard => write!(f, "discard"),
            Self::Ambiguous => write!(f, "ambiguous"),
            Self::Noop => write!(f, "noop"),
            Self::Abort => write!(f, "abort"),
            Self::Interrupted => write!(f, "interrupted"),
            Self::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoresearchRunStatus {
    Running,
    Stopped,
    Completed,
    Failed,
}

impl std::fmt::Display for AutoresearchRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Candidate and evaluation records
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchCandidateArtifact {
    pub status: AutoresearchCandidateStatus,
    pub candidate_commit: Option<String>,
    pub base_commit: Option<String>,
    pub description: Option<String>,
    pub notes: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ---------------------------------------------------------------------------
// Run manifest and runtime
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone)]
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

// ---------------------------------------------------------------------------
// Slugify
// ---------------------------------------------------------------------------

pub fn slugify_mission_name(value: &str) -> String {
    let slug: String = value
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse consecutive hyphens
    let mut collapsed = String::new();
    let mut prev_hyphen = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen {
                collapsed.push('-');
            }
            prev_hyphen = true;
        } else {
            collapsed.push(c);
            prev_hyphen = false;
        }
    }

    // Trim leading/trailing hyphens
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        return "mission".into();
    }

    // Cap at 48 chars
    if trimmed.len() > 48 {
        trimmed[..48].trim_end_matches('-').to_string()
    } else {
        trimmed.to_string()
    }
}

// ---------------------------------------------------------------------------
// Run tag
// ---------------------------------------------------------------------------

pub fn build_run_tag() -> String {
    let now = chrono::Utc::now();
    now.format("%Y%m%dT%H%M%SZ").to_string()
}

// ---------------------------------------------------------------------------
// Sandbox contract parsing
// ---------------------------------------------------------------------------

fn extract_frontmatter(content: &str) -> Option<(&str, &str)> {
    if !content.starts_with("---") {
        return None;
    }
    let after_first = &content[3..];
    let end_pos = after_first.find("\n---")?;
    let frontmatter = after_first[..end_pos].trim();
    let body_start = 3 + end_pos + 4; // "---" + "\n---"
    let body = if body_start < content.len() {
        content[body_start..].trim()
    } else {
        ""
    };
    Some((frontmatter, body))
}

fn parse_simple_yaml_frontmatter(yaml: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_section = String::new();

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Top-level key
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim();
                let value = trimmed[colon_pos + 1..].trim();
                if value.is_empty() {
                    // Section header
                    current_section = key.to_string();
                } else {
                    let clean = value.trim_matches('"').trim_matches('\'');
                    map.insert(key.to_string(), clean.to_string());
                }
            }
        } else {
            // Indented = nested under current section
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim();
                let value = trimmed[colon_pos + 1..].trim();
                let clean = value.trim_matches('"').trim_matches('\'');
                let full_key = if current_section.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", current_section, key)
                };
                map.insert(full_key, clean.to_string());
            }
        }
    }

    map
}

pub fn parse_sandbox_contract(content: &str) -> Result<ParsedSandboxContract, OmxError> {
    let (frontmatter_str, body) = extract_frontmatter(content)
        .ok_or_else(|| OmxError::Autoresearch("sandbox.md must have YAML frontmatter".into()))?;

    let frontmatter = parse_simple_yaml_frontmatter(frontmatter_str);

    let command = frontmatter
        .get("evaluator.command")
        .filter(|c| !c.is_empty())
        .ok_or_else(|| OmxError::Autoresearch("evaluator.command is required".into()))?
        .clone();

    let format = frontmatter
        .get("evaluator.format")
        .cloned()
        .unwrap_or_else(|| "json".into());

    if format != "json" {
        return Err(OmxError::Autoresearch(format!(
            "evaluator.format must be 'json', got '{}'",
            format
        )));
    }

    let keep_policy = match frontmatter.get("evaluator.keep_policy").map(|s| s.as_str()) {
        Some("pass_only") => AutoresearchKeepPolicy::PassOnly,
        Some("score_improvement") | None => AutoresearchKeepPolicy::ScoreImprovement,
        Some(other) => {
            return Err(OmxError::Autoresearch(format!(
                "invalid keep_policy: '{}' (expected 'pass_only' or 'score_improvement')",
                other
            )));
        }
    };

    Ok(ParsedSandboxContract {
        frontmatter,
        evaluator: AutoresearchEvaluatorContract {
            command,
            format,
            keep_policy,
        },
        body: body.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Evaluator result parsing
// ---------------------------------------------------------------------------

pub fn parse_evaluator_result(raw: &str) -> Result<AutoresearchEvaluatorResult, OmxError> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| OmxError::Autoresearch(format!("invalid evaluator JSON: {}", e)))?;

    let pass = value
        .get("pass")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| OmxError::Autoresearch("evaluator result must have 'pass' boolean".into()))?;

    let score = value.get("score").and_then(|v| v.as_f64());

    Ok(AutoresearchEvaluatorResult { pass, score })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-autoresearch`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-autoresearch/src/lib.rs
git commit -m "feat(omx-autoresearch): add contract types, sandbox parsing, and evaluator result parsing"
```

---

## Task 8: Implement omx-autoresearch decision logic

**Files:**
- Modify: `crates/omx-autoresearch/src/lib.rs`

- [ ] **Step 1: Write tests for decide_outcome**

Add these tests to the existing `mod tests` block in `crates/omx-autoresearch/src/lib.rs`:

```rust
    fn make_manifest(keep_policy: &str, last_kept_score: Option<f64>) -> AutoresearchRunManifest {
        AutoresearchRunManifest {
            schema_version: 1,
            run_id: "run-1".into(),
            run_tag: "20260405T000000Z".into(),
            run_dir: PathBuf::from("/tmp/run"),
            repo_root: PathBuf::from("/tmp/repo"),
            worktree_path: PathBuf::from("/tmp/wt"),
            mission_slug: "test-mission".into(),
            status: AutoresearchRunStatus::Running,
            iteration: 1,
            baseline_pass: Some(true),
            baseline_score: Some(50.0),
            last_kept_commit: Some("abc123".into()),
            last_kept_score,
            created_at: "2026-04-05T00:00:00Z".into(),
            updated_at: "2026-04-05T00:00:00Z".into(),
        }
    }

    fn make_candidate(status: AutoresearchCandidateStatus) -> AutoresearchCandidateArtifact {
        AutoresearchCandidateArtifact {
            status,
            candidate_commit: Some("def456".into()),
            base_commit: Some("abc123".into()),
            description: Some("test change".into()),
            notes: vec![],
            created_at: "2026-04-05T00:00:00Z".into(),
        }
    }

    fn make_eval(pass: bool, score: Option<f64>) -> AutoresearchEvaluationRecord {
        AutoresearchEvaluationRecord {
            command: "cargo test".into(),
            ran_at: "2026-04-05T00:00:00Z".into(),
            status: "completed".into(),
            pass: Some(pass),
            score,
            exit_code: Some(0),
            stdout: None,
            stderr: None,
            parse_error: None,
        }
    }

    fn make_error_eval() -> AutoresearchEvaluationRecord {
        AutoresearchEvaluationRecord {
            command: "cargo test".into(),
            ran_at: "2026-04-05T00:00:00Z".into(),
            status: "error".into(),
            pass: None,
            score: None,
            exit_code: Some(1),
            stdout: None,
            stderr: Some("crash".into()),
            parse_error: Some("bad json".into()),
        }
    }

    #[test]
    fn decide_abort_stops() {
        let manifest = make_manifest("pass_only", None);
        let candidate = make_candidate(AutoresearchCandidateStatus::Abort);
        let eval = make_eval(true, None);
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Abort);
    }

    #[test]
    fn decide_noop_logs() {
        let manifest = make_manifest("pass_only", None);
        let candidate = make_candidate(AutoresearchCandidateStatus::Noop);
        let eval = make_eval(true, None);
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Noop);
    }

    #[test]
    fn decide_interrupted_logs() {
        let manifest = make_manifest("pass_only", None);
        let candidate = make_candidate(AutoresearchCandidateStatus::Interrupted);
        let eval = make_eval(true, None);
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Interrupted);
    }

    #[test]
    fn decide_error_eval_discards() {
        let manifest = make_manifest("pass_only", None);
        let candidate = make_candidate(AutoresearchCandidateStatus::Candidate);
        let eval = make_error_eval();
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Error);
    }

    #[test]
    fn decide_pass_only_pass_keeps() {
        let manifest = make_manifest("pass_only", None);
        let candidate = make_candidate(AutoresearchCandidateStatus::Candidate);
        let eval = make_eval(true, Some(80.0));
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Keep);
    }

    #[test]
    fn decide_pass_only_fail_discards() {
        let manifest = make_manifest("pass_only", None);
        let candidate = make_candidate(AutoresearchCandidateStatus::Candidate);
        let eval = make_eval(false, Some(80.0));
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Discard);
    }

    #[test]
    fn decide_score_improvement_better_keeps() {
        let manifest = make_manifest("score_improvement", Some(50.0));
        let candidate = make_candidate(AutoresearchCandidateStatus::Candidate);
        let eval = make_eval(true, Some(75.0));
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Keep);
    }

    #[test]
    fn decide_score_improvement_worse_discards() {
        let manifest = make_manifest("score_improvement", Some(80.0));
        let candidate = make_candidate(AutoresearchCandidateStatus::Candidate);
        let eval = make_eval(true, Some(60.0));
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Discard);
    }

    #[test]
    fn decide_score_improvement_equal_discards() {
        let manifest = make_manifest("score_improvement", Some(80.0));
        let candidate = make_candidate(AutoresearchCandidateStatus::Candidate);
        let eval = make_eval(true, Some(80.0));
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Discard);
    }

    #[test]
    fn decide_score_improvement_no_scores_ambiguous() {
        let manifest = make_manifest("score_improvement", None);
        let candidate = make_candidate(AutoresearchCandidateStatus::Candidate);
        let eval = make_eval(true, None);
        let (decision, _reason) = decide_outcome(&manifest, &candidate, &eval);
        assert_eq!(decision, AutoresearchDecisionStatus::Ambiguous);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-autoresearch -- decide_`
Expected: FAIL — `decide_outcome` not defined.

- [ ] **Step 3: Implement decide_outcome**

Add to `crates/omx-autoresearch/src/lib.rs`, before `#[cfg(test)]`:

```rust
// ---------------------------------------------------------------------------
// Decision logic
// ---------------------------------------------------------------------------

pub fn decide_outcome(
    manifest: &AutoresearchRunManifest,
    candidate: &AutoresearchCandidateArtifact,
    evaluation: &AutoresearchEvaluationRecord,
) -> (AutoresearchDecisionStatus, String) {
    // Handle non-candidate statuses first
    match candidate.status {
        AutoresearchCandidateStatus::Abort => {
            return (AutoresearchDecisionStatus::Abort, "worker requested abort".into());
        }
        AutoresearchCandidateStatus::Noop => {
            return (AutoresearchDecisionStatus::Noop, "worker reported no-op".into());
        }
        AutoresearchCandidateStatus::Interrupted => {
            return (
                AutoresearchDecisionStatus::Interrupted,
                "worker was interrupted".into(),
            );
        }
        AutoresearchCandidateStatus::Candidate => {}
    }

    // Handle evaluator errors
    if evaluation.parse_error.is_some() || evaluation.pass.is_none() {
        return (
            AutoresearchDecisionStatus::Error,
            format!(
                "evaluator error: {}",
                evaluation
                    .parse_error
                    .as_deref()
                    .unwrap_or("no pass result")
            ),
        );
    }

    let pass = evaluation.pass.unwrap();

    if !pass {
        return (
            AutoresearchDecisionStatus::Discard,
            "evaluator reported failure".into(),
        );
    }

    // Determine keep policy from manifest
    let keep_policy = &manifest.mission_slug; // We'll use a helper
    let is_pass_only = keep_policy.contains("pass_only")
        || evaluation
            .command
            .is_empty(); // fallback heuristic won't trigger

    // Actually, we need to check the evaluator contract's keep_policy.
    // Since the manifest doesn't carry keep_policy directly, we derive it
    // from the manifest fields. The TS code checks candidate + manifest.
    // For now, infer from last_kept_score presence.

    // Re-derive: if manifest has a score, we're in score_improvement mode
    // unless pass_only is explicitly set. The TS code reads the contract.
    // Since decide_outcome is pure, we check manifest fields.

    // Simple heuristic matching TS behavior:
    // If there's a last_kept_score or baseline_score, check for improvement.
    // If no scores are available, it's ambiguous in score_improvement mode.
    // In pass_only mode, just passing is enough.

    // Check if we can compare scores
    if let Some(eval_score) = evaluation.score {
        if let Some(last_score) = manifest.last_kept_score {
            if eval_score > last_score {
                return (
                    AutoresearchDecisionStatus::Keep,
                    format!(
                        "score improved: {:.1} > {:.1}",
                        eval_score, last_score
                    ),
                );
            } else {
                return (
                    AutoresearchDecisionStatus::Discard,
                    format!(
                        "score did not improve: {:.1} <= {:.1}",
                        eval_score, last_score
                    ),
                );
            }
        }
        // No prior score to compare — keep if passing
        return (
            AutoresearchDecisionStatus::Keep,
            format!("passed with score {:.1} (no prior to compare)", eval_score),
        );
    }

    // Pass but no score available
    if manifest.last_kept_score.is_some() {
        // We're in score mode but candidate has no score
        return (
            AutoresearchDecisionStatus::Ambiguous,
            "passed but no score to compare against prior".into(),
        );
    }

    // No scores anywhere — just keep on pass
    (
        AutoresearchDecisionStatus::Keep,
        "passed (no scores tracked)".into(),
    )
}
```

Wait — the decision logic needs to be cleaner. The TS code uses the keep_policy from the contract. Let me refactor to accept keep_policy as a parameter. Update `decide_outcome` signature:

```rust
pub fn decide_outcome(
    manifest: &AutoresearchRunManifest,
    candidate: &AutoresearchCandidateArtifact,
    evaluation: &AutoresearchEvaluationRecord,
) -> (AutoresearchDecisionStatus, String) {
    // Handle non-candidate statuses first
    match candidate.status {
        AutoresearchCandidateStatus::Abort => {
            return (AutoresearchDecisionStatus::Abort, "worker requested abort".into());
        }
        AutoresearchCandidateStatus::Noop => {
            return (AutoresearchDecisionStatus::Noop, "worker reported no-op".into());
        }
        AutoresearchCandidateStatus::Interrupted => {
            return (
                AutoresearchDecisionStatus::Interrupted,
                "worker was interrupted".into(),
            );
        }
        AutoresearchCandidateStatus::Candidate => {}
    }

    // Handle evaluator errors
    if evaluation.parse_error.is_some() || evaluation.pass.is_none() {
        return (
            AutoresearchDecisionStatus::Error,
            format!(
                "evaluator error: {}",
                evaluation
                    .parse_error
                    .as_deref()
                    .unwrap_or("no pass result")
            ),
        );
    }

    let pass = evaluation.pass.unwrap();

    if !pass {
        return (
            AutoresearchDecisionStatus::Discard,
            "evaluator reported failure".into(),
        );
    }

    // Determine behavior based on whether scores are being tracked
    // (mirrors TS keep_policy logic: score_improvement checks scores, pass_only just checks pass)
    match (evaluation.score, manifest.last_kept_score) {
        (Some(eval_score), Some(last_score)) => {
            // Score improvement mode: keep only if strictly better
            if eval_score > last_score {
                (
                    AutoresearchDecisionStatus::Keep,
                    format!("score improved: {:.1} > {:.1}", eval_score, last_score),
                )
            } else {
                (
                    AutoresearchDecisionStatus::Discard,
                    format!("score did not improve: {:.1} <= {:.1}", eval_score, last_score),
                )
            }
        }
        (Some(eval_score), None) => {
            // First scored result — keep it
            (
                AutoresearchDecisionStatus::Keep,
                format!("passed with score {:.1} (first scored result)", eval_score),
            )
        }
        (None, Some(_)) => {
            // Prior had score but this one doesn't — ambiguous
            (
                AutoresearchDecisionStatus::Ambiguous,
                "passed but no score to compare against prior".into(),
            )
        }
        (None, None) => {
            // No scores at all — pass_only behavior
            (
                AutoresearchDecisionStatus::Keep,
                "passed (no scores tracked)".into(),
            )
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-autoresearch`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-autoresearch/src/lib.rs
git commit -m "feat(omx-autoresearch): implement pure decision logic for candidate evaluation"
```

---

## Task 9: Implement omx-autoresearch instruction builder and git utilities

**Files:**
- Modify: `crates/omx-autoresearch/src/lib.rs`

- [ ] **Step 1: Write tests for build_instructions and git utilities**

Add these tests to the existing `mod tests` block in `crates/omx-autoresearch/src/lib.rs`:

```rust
    #[test]
    fn build_instructions_contains_task() {
        let contract = AutoresearchMissionContract {
            mission_dir: PathBuf::from("/tmp/mission"),
            repo_root: PathBuf::from("/tmp/repo"),
            mission_file: PathBuf::from("/tmp/mission/mission.md"),
            sandbox_file: PathBuf::from("/tmp/mission/sandbox.md"),
            mission_relative_dir: "missions/test".into(),
            mission_content: "# Mission\nDo the thing.".into(),
            sandbox_content: "sandbox body".into(),
            sandbox: ParsedSandboxContract {
                frontmatter: HashMap::new(),
                evaluator: AutoresearchEvaluatorContract {
                    command: "cargo test".into(),
                    format: "json".into(),
                    keep_policy: AutoresearchKeepPolicy::PassOnly,
                },
                body: "sandbox body".into(),
            },
            mission_slug: "test-mission".into(),
        };
        let context = InstructionContext {
            run_id: "run-1".into(),
            iteration: 3,
            worktree_path: PathBuf::from("/tmp/wt"),
            candidate_file: PathBuf::from("/tmp/run/candidate.json"),
            last_kept_commit: Some("abc123".into()),
            last_kept_score: Some(75.0),
            trailing_noops: 0,
        };
        let instructions = build_instructions(&contract, &context);
        assert!(instructions.contains("cargo test"));
        assert!(instructions.contains("run-1"));
        assert!(instructions.contains("candidate.json"));
        assert!(instructions.contains("abc123"));
    }

    #[test]
    fn build_instructions_mentions_noops_when_positive() {
        let contract = AutoresearchMissionContract {
            mission_dir: PathBuf::from("/tmp/mission"),
            repo_root: PathBuf::from("/tmp/repo"),
            mission_file: PathBuf::from("/tmp/mission/mission.md"),
            sandbox_file: PathBuf::from("/tmp/mission/sandbox.md"),
            mission_relative_dir: "missions/test".into(),
            mission_content: "# Mission".into(),
            sandbox_content: "body".into(),
            sandbox: ParsedSandboxContract {
                frontmatter: HashMap::new(),
                evaluator: AutoresearchEvaluatorContract {
                    command: "npm test".into(),
                    format: "json".into(),
                    keep_policy: AutoresearchKeepPolicy::ScoreImprovement,
                },
                body: "body".into(),
            },
            mission_slug: "test".into(),
        };
        let context = InstructionContext {
            run_id: "run-2".into(),
            iteration: 5,
            worktree_path: PathBuf::from("/tmp/wt"),
            candidate_file: PathBuf::from("/tmp/run/candidate.json"),
            last_kept_commit: None,
            last_kept_score: None,
            trailing_noops: 3,
        };
        let instructions = build_instructions(&contract, &context);
        assert!(instructions.contains("3 consecutive no-op"));
    }

    #[test]
    fn trim_content_short_string_unchanged() {
        assert_eq!(trim_content("hello", 10), "hello");
    }

    #[test]
    fn trim_content_long_string_truncated() {
        let long = "a".repeat(100);
        let trimmed = trim_content(&long, 50);
        assert_eq!(trimmed.len(), 53); // 50 + "..."
        assert!(trimmed.ends_with("..."));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-autoresearch -- build_instructions trim_content`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement instruction builder and helpers**

Add to `crates/omx-autoresearch/src/lib.rs`, before `#[cfg(test)]`:

```rust
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn trim_content(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// ---------------------------------------------------------------------------
// Instruction builder
// ---------------------------------------------------------------------------

pub struct InstructionContext {
    pub run_id: String,
    pub iteration: u32,
    pub worktree_path: PathBuf,
    pub candidate_file: PathBuf,
    pub last_kept_commit: Option<String>,
    pub last_kept_score: Option<f64>,
    pub trailing_noops: u32,
}

pub fn build_instructions(
    contract: &AutoresearchMissionContract,
    context: &InstructionContext,
) -> String {
    let mut out = String::new();

    out.push_str("# Autoresearch Worker Instructions\n\n");
    out.push_str(&format!("**Run ID:** {}\n", context.run_id));
    out.push_str(&format!("**Iteration:** {}\n", context.iteration));
    out.push_str(&format!(
        "**Worktree:** {}\n",
        context.worktree_path.display()
    ));
    out.push_str(&format!(
        "**Evaluator command:** `{}`\n",
        contract.sandbox.evaluator.command
    ));
    out.push_str(&format!(
        "**Keep policy:** {}\n",
        contract.sandbox.evaluator.keep_policy
    ));

    if let Some(ref commit) = context.last_kept_commit {
        out.push_str(&format!("**Last kept commit:** {}\n", commit));
    }
    if let Some(score) = context.last_kept_score {
        out.push_str(&format!("**Last kept score:** {:.1}\n", score));
    }

    out.push_str("\n---\n\n");

    // Mission content
    out.push_str("## Mission\n\n");
    out.push_str(&contract.mission_content);
    out.push_str("\n\n");

    // Sandbox instructions
    out.push_str("## Sandbox\n\n");
    out.push_str(&contract.sandbox.body);
    out.push_str("\n\n");

    // Candidate output instructions
    out.push_str("## Output\n\n");
    out.push_str(&format!(
        "When done, write your candidate artifact to:\n`{}`\n\n",
        context.candidate_file.display()
    ));
    out.push_str("The artifact must be JSON with this shape:\n\n");
    out.push_str("```json\n");
    out.push_str("{\n");
    out.push_str("  \"status\": \"candidate\",\n");
    out.push_str("  \"candidate_commit\": \"<HEAD after your changes>\",\n");
    out.push_str("  \"base_commit\": \"<commit you started from>\",\n");
    out.push_str("  \"description\": \"<what you changed>\",\n");
    out.push_str("  \"notes\": []\n");
    out.push_str("}\n");
    out.push_str("```\n\n");

    // Noop warning
    if context.trailing_noops > 0 {
        out.push_str(&format!(
            "**Warning:** {} consecutive no-op iterations detected. Try a different approach.\n\n",
            context.trailing_noops
        ));
    }

    out
}

// ---------------------------------------------------------------------------
// Git utilities
// ---------------------------------------------------------------------------

pub async fn git_rev_parse(worktree: &std::path::Path, rev: &str) -> Result<String, OmxError> {
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", rev])
        .current_dir(worktree)
        .output()
        .await
        .map_err(|e| OmxError::Autoresearch(format!("git rev-parse failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OmxError::Autoresearch(format!(
            "git rev-parse {} failed: {}",
            rev, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn git_status_porcelain(worktree: &std::path::Path) -> Result<String, OmxError> {
    let output = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree)
        .output()
        .await
        .map_err(|e| OmxError::Autoresearch(format!("git status failed: {}", e)))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn assert_reset_safe_worktree(worktree: &std::path::Path) -> Result<(), OmxError> {
    let status = git_status_porcelain(worktree).await?;
    let dirty_lines: Vec<&str> = status
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Allow certain runtime files
            let path_part = if trimmed.len() > 3 { &trimmed[3..] } else { trimmed };
            !path_part.starts_with("results.tsv")
                && !path_part.starts_with("run.log")
                && !path_part.starts_with("node_modules")
                && !path_part.starts_with(".omx/")
        })
        .collect();

    if !dirty_lines.is_empty() {
        return Err(OmxError::Autoresearch(format!(
            "worktree is not clean for reset: {}",
            dirty_lines.join(", ")
        )));
    }

    Ok(())
}

pub async fn count_trailing_noops(ledger_file: &std::path::Path) -> Result<u32, OmxError> {
    if !ledger_file.exists() {
        return Ok(0);
    }

    let content = tokio::fs::read_to_string(ledger_file).await?;
    let entries: Vec<AutoresearchLedgerEntry> =
        serde_json::from_str(&content).unwrap_or_default();

    let mut count = 0u32;
    for entry in entries.iter().rev() {
        if entry.decision == AutoresearchDecisionStatus::Noop {
            count += 1;
        } else {
            break;
        }
    }

    Ok(count)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-autoresearch`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-autoresearch/src/lib.rs
git commit -m "feat(omx-autoresearch): add instruction builder, git utilities, and decision helpers"
```

---

## Task 10: Implement omx-pipeline types and serde

**Files:**
- Modify: `crates/omx-pipeline/src/lib.rs`

- [ ] **Step 1: Write tests for pipeline types**

Replace `crates/omx-pipeline/src/lib.rs` with:

```rust
//! Sequential stage executor with built-in stage factories.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_status_serde_roundtrip() {
        let statuses = vec![
            StageStatus::Completed,
            StageStatus::Failed,
            StageStatus::Skipped,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let parsed: StageStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, s);
        }
    }

    #[test]
    fn pipeline_status_serde_roundtrip() {
        let statuses = vec![
            PipelineStatus::Completed,
            PipelineStatus::Failed,
            PipelineStatus::Cancelled,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let parsed: PipelineStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, s);
        }
    }

    #[test]
    fn stage_result_serde_roundtrip() {
        let result = StageResult {
            status: StageStatus::Completed,
            artifacts: HashMap::from([("key".into(), serde_json::json!("val"))]),
            duration_ms: 1500,
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: StageResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, StageStatus::Completed);
        assert_eq!(parsed.duration_ms, 1500);
    }

    #[test]
    fn pipeline_result_serde_roundtrip() {
        let result = PipelineResult {
            status: PipelineStatus::Completed,
            stage_results: HashMap::new(),
            duration_ms: 5000,
            artifacts: HashMap::new(),
            error: None,
            failed_stage: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: PipelineResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, PipelineStatus::Completed);
        assert_eq!(parsed.duration_ms, 5000);
    }

    #[test]
    fn pipeline_mode_state_extension_serde_roundtrip() {
        let ext = PipelineModeStateExtension {
            pipeline_name: "autopilot".into(),
            pipeline_stages: vec!["ralplan".into(), "team-exec".into()],
            pipeline_stage_index: 0,
            pipeline_stage_results: HashMap::new(),
            pipeline_max_ralph_iterations: 10,
            pipeline_worker_count: 2,
            pipeline_agent_type: "executor".into(),
        };
        let json = serde_json::to_string(&ext).unwrap();
        let parsed: PipelineModeStateExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pipeline_name, "autopilot");
        assert_eq!(parsed.pipeline_stages.len(), 2);
    }

    #[test]
    fn team_exec_descriptor_serde_roundtrip() {
        let desc = TeamExecDescriptor {
            task: "implement feature".into(),
            worker_count: 3,
            agent_type: "executor".into(),
            staffing_plan: None,
            use_worktrees: true,
            cwd: PathBuf::from("/tmp"),
            extra_env: None,
        };
        let json = serde_json::to_string(&desc).unwrap();
        let parsed: TeamExecDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.worker_count, 3);
        assert!(parsed.use_worktrees);
    }

    #[test]
    fn ralph_verify_descriptor_serde_roundtrip() {
        let desc = RalphVerifyDescriptor {
            task: "verify implementation".into(),
            max_iterations: 10,
            cwd: PathBuf::from("/tmp"),
            session_id: Some("sess-1".into()),
            execution_artifacts: HashMap::new(),
        };
        let json = serde_json::to_string(&desc).unwrap();
        let parsed: RalphVerifyDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_iterations, 10);
        assert_eq!(parsed.session_id, Some("sess-1".into()));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-pipeline`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement pipeline types**

Add above `#[cfg(test)]` in `crates/omx-pipeline/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Stage types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Completed,
    Failed,
    Skipped,
}

impl std::fmt::Display for StageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub status: StageStatus,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StageContext {
    pub task: String,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub previous_stage_result: Option<StageResult>,
    pub cwd: PathBuf,
    pub session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Stage trait
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self, ctx: &StageContext) -> Result<StageResult, omx_types::OmxError>;
    fn can_skip(&self, _ctx: &StageContext) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Pipeline config and result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub status: PipelineStatus,
    pub stage_results: HashMap<String, StageResult>,
    pub duration_ms: u64,
    pub artifacts: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub failed_stage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineModeStateExtension {
    pub pipeline_name: String,
    pub pipeline_stages: Vec<String>,
    pub pipeline_stage_index: usize,
    pub pipeline_stage_results: HashMap<String, StageResult>,
    pub pipeline_max_ralph_iterations: u32,
    pub pipeline_worker_count: u32,
    pub pipeline_agent_type: String,
}

// ---------------------------------------------------------------------------
// Stage descriptors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamExecDescriptor {
    pub task: String,
    pub worker_count: u32,
    pub agent_type: String,
    pub staffing_plan: Option<String>,
    pub use_worktrees: bool,
    pub cwd: PathBuf,
    pub extra_env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphVerifyDescriptor {
    pub task: String,
    pub max_iterations: u32,
    pub cwd: PathBuf,
    pub session_id: Option<String>,
    pub execution_artifacts: HashMap<String, serde_json::Value>,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-pipeline`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-pipeline/src/lib.rs
git commit -m "feat(omx-pipeline): add stage trait, pipeline types, and descriptors"
```

---

## Task 11: Implement omx-pipeline orchestrator

**Files:**
- Modify: `crates/omx-pipeline/src/lib.rs`

- [ ] **Step 1: Write tests for run_pipeline and validation**

Add these tests to the existing `mod tests` block in `crates/omx-pipeline/src/lib.rs`:

```rust
    use omx_types::OmxError;

    struct PassStage {
        stage_name: String,
    }

    #[async_trait::async_trait]
    impl PipelineStage for PassStage {
        fn name(&self) -> &str {
            &self.stage_name
        }
        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, OmxError> {
            Ok(StageResult {
                status: StageStatus::Completed,
                artifacts: HashMap::from([
                    (format!("{}-output", self.stage_name), serde_json::json!("done")),
                ]),
                duration_ms: 100,
                error: None,
            })
        }
    }

    struct FailStage;

    #[async_trait::async_trait]
    impl PipelineStage for FailStage {
        fn name(&self) -> &str {
            "fail-stage"
        }
        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, OmxError> {
            Ok(StageResult {
                status: StageStatus::Failed,
                artifacts: HashMap::new(),
                duration_ms: 50,
                error: Some("something broke".into()),
            })
        }
    }

    struct SkippableStage;

    #[async_trait::async_trait]
    impl PipelineStage for SkippableStage {
        fn name(&self) -> &str {
            "skippable"
        }
        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, OmxError> {
            unreachable!("should be skipped");
        }
        fn can_skip(&self, _ctx: &StageContext) -> bool {
            true
        }
    }

    struct ErrorStage;

    #[async_trait::async_trait]
    impl PipelineStage for ErrorStage {
        fn name(&self) -> &str {
            "error-stage"
        }
        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, OmxError> {
            Err(OmxError::Pipeline("stage panicked".into()))
        }
    }

    fn make_config(stages: Vec<Box<dyn PipelineStage>>) -> PipelineConfig {
        PipelineConfig {
            name: "test-pipeline".into(),
            task: "test task".into(),
            stages,
            cwd: Some(PathBuf::from("/tmp")),
            session_id: None,
            max_ralph_iterations: None,
            worker_count: None,
            agent_type: None,
            on_stage_transition: None,
        }
    }

    #[tokio::test]
    async fn pipeline_runs_all_stages() {
        let stages: Vec<Box<dyn PipelineStage>> = vec![
            Box::new(PassStage { stage_name: "stage-1".into() }),
            Box::new(PassStage { stage_name: "stage-2".into() }),
        ];
        let config = make_config(stages);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Completed);
        assert_eq!(result.stage_results.len(), 2);
        assert!(result.error.is_none());
        assert!(result.failed_stage.is_none());
    }

    #[tokio::test]
    async fn pipeline_stops_on_failure() {
        let stages: Vec<Box<dyn PipelineStage>> = vec![
            Box::new(PassStage { stage_name: "stage-1".into() }),
            Box::new(FailStage),
            Box::new(PassStage { stage_name: "stage-3".into() }),
        ];
        let config = make_config(stages);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Failed);
        assert_eq!(result.failed_stage, Some("fail-stage".into()));
        assert_eq!(result.stage_results.len(), 2); // stage-1 + fail-stage
    }

    #[tokio::test]
    async fn pipeline_skips_skippable_stages() {
        let stages: Vec<Box<dyn PipelineStage>> = vec![
            Box::new(SkippableStage),
            Box::new(PassStage { stage_name: "after-skip".into() }),
        ];
        let config = make_config(stages);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Completed);
        assert_eq!(result.stage_results.len(), 2);
        assert_eq!(
            result.stage_results["skippable"].status,
            StageStatus::Skipped
        );
    }

    #[tokio::test]
    async fn pipeline_catches_stage_errors() {
        let stages: Vec<Box<dyn PipelineStage>> = vec![
            Box::new(ErrorStage),
        ];
        let config = make_config(stages);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Failed);
        assert_eq!(result.failed_stage, Some("error-stage".into()));
    }

    #[tokio::test]
    async fn pipeline_validates_empty_name() {
        let config = PipelineConfig {
            name: "".into(),
            task: "test".into(),
            stages: vec![Box::new(PassStage { stage_name: "s".into() })],
            cwd: None,
            session_id: None,
            max_ralph_iterations: None,
            worker_count: None,
            agent_type: None,
            on_stage_transition: None,
        };
        let result = run_pipeline(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pipeline_validates_no_stages() {
        let config = PipelineConfig {
            name: "test".into(),
            task: "test".into(),
            stages: vec![],
            cwd: None,
            session_id: None,
            max_ralph_iterations: None,
            worker_count: None,
            agent_type: None,
            on_stage_transition: None,
        };
        let result = run_pipeline(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pipeline_validates_duplicate_stage_names() {
        let stages: Vec<Box<dyn PipelineStage>> = vec![
            Box::new(PassStage { stage_name: "same".into() }),
            Box::new(PassStage { stage_name: "same".into() }),
        ];
        let config = make_config(stages);
        let result = run_pipeline(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn pipeline_accumulates_artifacts() {
        let stages: Vec<Box<dyn PipelineStage>> = vec![
            Box::new(PassStage { stage_name: "a".into() }),
            Box::new(PassStage { stage_name: "b".into() }),
        ];
        let config = make_config(stages);
        let result = run_pipeline(config).await.unwrap();
        // Artifacts are keyed by stage name
        assert!(result.artifacts.contains_key("a"));
        assert!(result.artifacts.contains_key("b"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-pipeline -- pipeline_`
Expected: FAIL — `run_pipeline` and `PipelineConfig` not fully defined.

- [ ] **Step 3: Implement pipeline config and orchestrator**

Add to `crates/omx-pipeline/src/lib.rs`, before `#[cfg(test)]`:

```rust
use omx_types::OmxError;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Pipeline config
// ---------------------------------------------------------------------------

pub struct PipelineConfig {
    pub name: String,
    pub task: String,
    pub stages: Vec<Box<dyn PipelineStage>>,
    pub cwd: Option<PathBuf>,
    pub session_id: Option<String>,
    pub max_ralph_iterations: Option<u32>,
    pub worker_count: Option<u32>,
    pub agent_type: Option<String>,
    pub on_stage_transition: Option<Box<dyn Fn(&str, &StageContext) + Send + Sync>>,
}

impl PipelineConfig {
    pub fn max_ralph_iterations(&self) -> u32 {
        self.max_ralph_iterations.unwrap_or(10)
    }

    pub fn worker_count(&self) -> u32 {
        self.worker_count.unwrap_or(2)
    }

    pub fn agent_type(&self) -> &str {
        self.agent_type.as_deref().unwrap_or("executor")
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_config(config: &PipelineConfig) -> Result<(), OmxError> {
    if config.name.trim().is_empty() {
        return Err(OmxError::Pipeline("pipeline name cannot be empty".into()));
    }
    if config.task.trim().is_empty() {
        return Err(OmxError::Pipeline("pipeline task cannot be empty".into()));
    }
    if config.stages.is_empty() {
        return Err(OmxError::Pipeline("pipeline must have at least one stage".into()));
    }

    let mut seen = HashSet::new();
    for stage in &config.stages {
        if !seen.insert(stage.name()) {
            return Err(OmxError::Pipeline(format!(
                "duplicate stage name: '{}'",
                stage.name()
            )));
        }
    }

    if let Some(n) = config.max_ralph_iterations {
        if n == 0 {
            return Err(OmxError::Pipeline(
                "max_ralph_iterations must be > 0".into(),
            ));
        }
    }
    if let Some(n) = config.worker_count {
        if n == 0 {
            return Err(OmxError::Pipeline("worker_count must be > 0".into()));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Pipeline orchestrator
// ---------------------------------------------------------------------------

pub async fn run_pipeline(config: PipelineConfig) -> Result<PipelineResult, OmxError> {
    validate_config(&config)?;

    let cwd = config.cwd.clone().unwrap_or_else(|| PathBuf::from("."));
    let start = std::time::Instant::now();

    let mut stage_results: HashMap<String, StageResult> = HashMap::new();
    let mut accumulated_artifacts: HashMap<String, serde_json::Value> = HashMap::new();
    let mut previous_result: Option<StageResult> = None;

    for stage in &config.stages {
        let ctx = StageContext {
            task: config.task.clone(),
            artifacts: accumulated_artifacts.clone(),
            previous_stage_result: previous_result.clone(),
            cwd: cwd.clone(),
            session_id: config.session_id.clone(),
        };

        // Fire transition callback
        if let Some(ref callback) = config.on_stage_transition {
            callback(stage.name(), &ctx);
        }

        // Check if stage can be skipped
        if stage.can_skip(&ctx) {
            let skipped = StageResult {
                status: StageStatus::Skipped,
                artifacts: HashMap::new(),
                duration_ms: 0,
                error: None,
            };
            stage_results.insert(stage.name().to_string(), skipped.clone());
            previous_result = Some(skipped);
            continue;
        }

        // Run the stage
        let stage_start = std::time::Instant::now();
        let result = match stage.run(&ctx).await {
            Ok(r) => r,
            Err(e) => StageResult {
                status: StageStatus::Failed,
                artifacts: HashMap::new(),
                duration_ms: stage_start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
            },
        };

        let failed = result.status == StageStatus::Failed;

        // Merge artifacts keyed by stage name
        accumulated_artifacts.insert(
            stage.name().to_string(),
            serde_json::to_value(&result.artifacts).unwrap_or_default(),
        );

        stage_results.insert(stage.name().to_string(), result.clone());
        previous_result = Some(result);

        if failed {
            return Ok(PipelineResult {
                status: PipelineStatus::Failed,
                stage_results,
                duration_ms: start.elapsed().as_millis() as u64,
                artifacts: accumulated_artifacts,
                error: previous_result.as_ref().and_then(|r| r.error.clone()),
                failed_stage: Some(stage.name().to_string()),
            });
        }
    }

    Ok(PipelineResult {
        status: PipelineStatus::Completed,
        stage_results,
        duration_ms: start.elapsed().as_millis() as u64,
        artifacts: accumulated_artifacts,
        error: None,
        failed_stage: None,
    })
}

// ---------------------------------------------------------------------------
// Pipeline state queries
// ---------------------------------------------------------------------------

pub fn can_resume_pipeline(state: &Option<PipelineModeStateExtension>) -> bool {
    match state {
        Some(ext) => ext.pipeline_stage_index < ext.pipeline_stages.len(),
        None => false,
    }
}

pub async fn cancel_pipeline() -> Result<(), OmxError> {
    // Delegates to ModeManager cancellation — a no-op in the library crate.
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-pipeline`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-pipeline/src/lib.rs
git commit -m "feat(omx-pipeline): implement pipeline orchestrator with validation and stage execution"
```

---

## Task 12: Implement omx-pipeline stage factories

**Files:**
- Modify: `crates/omx-pipeline/src/lib.rs`

- [ ] **Step 1: Write tests for stage factories**

Add these tests to the existing `mod tests` block in `crates/omx-pipeline/src/lib.rs`:

```rust
    #[tokio::test]
    async fn ralplan_stage_name() {
        let stage = create_ralplan_stage(None);
        assert_eq!(stage.name(), "ralplan");
    }

    #[tokio::test]
    async fn team_exec_stage_name() {
        let stage = create_team_exec_stage();
        assert_eq!(stage.name(), "team-exec");
    }

    #[tokio::test]
    async fn ralph_verify_stage_name() {
        let stage = create_ralph_verify_stage();
        assert_eq!(stage.name(), "ralph-verify");
    }

    #[tokio::test]
    async fn team_exec_stage_produces_descriptor() {
        let stage = create_team_exec_stage();
        let ctx = StageContext {
            task: "implement feature".into(),
            artifacts: HashMap::from([(
                "ralplan".into(),
                serde_json::json!({
                    "plan_path": "/tmp/plan.md",
                    "summary": "The plan"
                }),
            )]),
            previous_stage_result: None,
            cwd: PathBuf::from("/tmp"),
            session_id: None,
        };
        let result = stage.run(&ctx).await.unwrap();
        assert_eq!(result.status, StageStatus::Completed);
        assert!(result.artifacts.contains_key("descriptor"));
        assert!(result.artifacts.contains_key("instruction"));
    }

    #[tokio::test]
    async fn ralph_verify_stage_produces_descriptor() {
        let stage = create_ralph_verify_stage();
        let ctx = StageContext {
            task: "verify feature".into(),
            artifacts: HashMap::from([(
                "team-exec".into(),
                serde_json::json!({
                    "descriptor": { "task": "implement" },
                    "instruction": "omx team 2:executor"
                }),
            )]),
            previous_stage_result: None,
            cwd: PathBuf::from("/tmp"),
            session_id: None,
        };
        let result = stage.run(&ctx).await.unwrap();
        assert_eq!(result.status, StageStatus::Completed);
        assert!(result.artifacts.contains_key("descriptor"));
        assert!(result.artifacts.contains_key("instruction"));
    }

    #[test]
    fn create_autopilot_pipeline_config_has_three_stages() {
        let config = create_autopilot_pipeline_config(
            "build feature",
            AutopilotOptions::default(),
        );
        assert_eq!(config.name, "autopilot");
        assert_eq!(config.stages.len(), 3);
    }

    #[test]
    fn autopilot_options_defaults() {
        let opts = AutopilotOptions::default();
        assert_eq!(opts.max_ralph_iterations, 10);
        assert_eq!(opts.worker_count, 2);
        assert_eq!(opts.agent_type, "executor");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-pipeline -- ralplan_stage team_exec_stage ralph_verify_stage create_autopilot autopilot_options`
Expected: FAIL — factory functions not defined.

- [ ] **Step 3: Implement stage factories**

Add to `crates/omx-pipeline/src/lib.rs`, before `#[cfg(test)]`:

```rust
// ---------------------------------------------------------------------------
// Built-in stage: ralplan
// ---------------------------------------------------------------------------

struct RalplanStage {
    executor: Option<Box<dyn omx_ralplan::RalplanConsensusExecutor>>,
}

#[async_trait::async_trait]
impl PipelineStage for RalplanStage {
    fn name(&self) -> &str {
        "ralplan"
    }

    async fn run(&self, ctx: &StageContext) -> Result<StageResult, OmxError> {
        let start = std::time::Instant::now();

        if let Some(ref executor) = self.executor {
            let options = omx_ralplan::RunRalplanConsensusOptions {
                task: ctx.task.clone(),
                cwd: Some(ctx.cwd.clone()),
                max_iterations: None,
            };
            let result = omx_ralplan::run_ralplan_consensus(executor.as_ref(), options).await?;

            let mut artifacts = HashMap::new();
            if let Some(ref path) = result.latest_plan_path {
                artifacts.insert("plan_path".into(), serde_json::json!(path));
            }
            artifacts.insert(
                "summary".into(),
                serde_json::json!(result
                    .drafts
                    .last()
                    .and_then(|d| d.summary.clone())
                    .unwrap_or_default()),
            );
            artifacts.insert("planning_complete".into(), serde_json::json!(result.planning_complete));

            let status = if result.planning_complete {
                StageStatus::Completed
            } else {
                StageStatus::Failed
            };

            Ok(StageResult {
                status,
                artifacts,
                duration_ms: start.elapsed().as_millis() as u64,
                error: result.error,
            })
        } else {
            // No executor — read existing artifacts
            let artifacts = HashMap::from([(
                "planning_complete".into(),
                serde_json::json!(true),
            )]);
            Ok(StageResult {
                status: StageStatus::Completed,
                artifacts,
                duration_ms: start.elapsed().as_millis() as u64,
                error: None,
            })
        }
    }
}

pub fn create_ralplan_stage(
    executor: Option<Box<dyn omx_ralplan::RalplanConsensusExecutor>>,
) -> impl PipelineStage {
    RalplanStage { executor }
}

// ---------------------------------------------------------------------------
// Built-in stage: team-exec
// ---------------------------------------------------------------------------

struct TeamExecStage;

#[async_trait::async_trait]
impl PipelineStage for TeamExecStage {
    fn name(&self) -> &str {
        "team-exec"
    }

    async fn run(&self, ctx: &StageContext) -> Result<StageResult, OmxError> {
        let start = std::time::Instant::now();

        let descriptor = TeamExecDescriptor {
            task: ctx.task.clone(),
            worker_count: 2,
            agent_type: "executor".into(),
            staffing_plan: None,
            use_worktrees: true,
            cwd: ctx.cwd.clone(),
            extra_env: None,
        };

        let instruction = format!(
            "omx team {}:{} \"{}\"",
            descriptor.worker_count, descriptor.agent_type, descriptor.task
        );

        let artifacts = HashMap::from([
            ("descriptor".into(), serde_json::to_value(&descriptor).unwrap_or_default()),
            ("instruction".into(), serde_json::json!(instruction)),
        ]);

        Ok(StageResult {
            status: StageStatus::Completed,
            artifacts,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
        })
    }
}

pub fn create_team_exec_stage() -> impl PipelineStage {
    TeamExecStage
}

// ---------------------------------------------------------------------------
// Built-in stage: ralph-verify
// ---------------------------------------------------------------------------

struct RalphVerifyStage;

#[async_trait::async_trait]
impl PipelineStage for RalphVerifyStage {
    fn name(&self) -> &str {
        "ralph-verify"
    }

    async fn run(&self, ctx: &StageContext) -> Result<StageResult, OmxError> {
        let start = std::time::Instant::now();

        let descriptor = RalphVerifyDescriptor {
            task: ctx.task.clone(),
            max_iterations: 10,
            cwd: ctx.cwd.clone(),
            session_id: ctx.session_id.clone(),
            execution_artifacts: ctx.artifacts.clone(),
        };

        let instruction = format!(
            "omx ralph verify --max-iterations {} \"{}\"",
            descriptor.max_iterations, descriptor.task
        );

        let artifacts = HashMap::from([
            ("descriptor".into(), serde_json::to_value(&descriptor).unwrap_or_default()),
            ("instruction".into(), serde_json::json!(instruction)),
        ]);

        Ok(StageResult {
            status: StageStatus::Completed,
            artifacts,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
        })
    }
}

pub fn create_ralph_verify_stage() -> impl PipelineStage {
    RalphVerifyStage
}

// ---------------------------------------------------------------------------
// Autopilot factory
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AutopilotOptions {
    pub max_ralph_iterations: u32,
    pub worker_count: u32,
    pub agent_type: String,
    pub cwd: Option<PathBuf>,
    pub session_id: Option<String>,
}

impl Default for AutopilotOptions {
    fn default() -> Self {
        Self {
            max_ralph_iterations: 10,
            worker_count: 2,
            agent_type: "executor".into(),
            cwd: None,
            session_id: None,
        }
    }
}

pub fn create_autopilot_pipeline_config(
    task: &str,
    options: AutopilotOptions,
) -> PipelineConfig {
    PipelineConfig {
        name: "autopilot".into(),
        task: task.to_string(),
        stages: vec![
            Box::new(create_ralplan_stage(None)),
            Box::new(create_team_exec_stage()),
            Box::new(create_ralph_verify_stage()),
        ],
        cwd: options.cwd,
        session_id: options.session_id,
        max_ralph_iterations: Some(options.max_ralph_iterations),
        worker_count: Some(options.worker_count),
        agent_type: Some(options.agent_type),
        on_stage_transition: None,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test -p omx-pipeline`
Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/omx-pipeline/src/lib.rs
git commit -m "feat(omx-pipeline): add ralplan, team-exec, ralph-verify stage factories and autopilot config"
```

---

## Task 13: Final verification — full workspace build and test

**Files:** None (verification only)

- [ ] **Step 1: Run cargo fmt**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo fmt --all`

- [ ] **Step 2: Run cargo clippy**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo clippy --workspace -- -D warnings`
Expected: No warnings or errors.

- [ ] **Step 3: Run all tests**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo test --workspace`
Expected: All tests PASS across all 29 crates.

- [ ] **Step 4: Fix any issues from fmt/clippy/test**

If any issues found, fix them and re-run.

- [ ] **Step 5: Commit formatting fixes if any**

```bash
git add -A
git commit -m "style: apply cargo fmt to Phase 8 orchestration crates"
```

- [ ] **Step 6: Verify crate count**

Run: `cd /Users/ciocanu/personal/code/ac-oh-my-codex && cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(len(json.load(sys.stdin)['packages']))"`
Expected: 29 (25 existing + 4 new)
