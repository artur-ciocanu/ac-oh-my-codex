//! Consensus planning with Draft → ArchitectReview → CriticReview loop.

use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
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

impl fmt::Display for RalplanPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    /// Returns `true` for terminal phases: Complete, Cancelled, Failed.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Cancelled | Self::Failed)
    }

    /// Returns `true` for active phases: Draft, ArchitectReview, CriticReview.
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

impl fmt::Display for RalplanReviewVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[derive(Debug, Clone)]
pub struct RalplanIterationContext {
    pub task: String,
    pub cwd: PathBuf,
    pub iteration: u32,
    pub prior_drafts: Vec<RalplanDraftResult>,
    pub architect_reviews: Vec<RalplanReviewResult>,
    pub critic_reviews: Vec<RalplanReviewResult>,
}

// ---------------------------------------------------------------------------
// Options and runtime result
// ---------------------------------------------------------------------------

pub struct RunRalplanConsensusOptions {
    pub task: String,
    pub cwd: Option<PathBuf>,
    pub max_iterations: Option<u32>,
}

impl RunRalplanConsensusOptions {
    /// Returns the effective max iterations (default 5).
    pub fn effective_max_iterations(&self) -> u32 {
        self.max_iterations.unwrap_or(5)
    }
}

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

/// Runs the ralplan consensus loop: Draft → ArchitectReview → CriticReview.
///
/// Returns a [`RalplanRuntimeResult`] describing the final outcome.
pub async fn run_ralplan_consensus(
    executor: &dyn RalplanConsensusExecutor,
    options: RunRalplanConsensusOptions,
) -> Result<RalplanRuntimeResult, OmxError> {
    let max_iterations = options.effective_max_iterations();
    let cwd = options.cwd.unwrap_or_else(|| PathBuf::from("."));

    let mut drafts: Vec<RalplanDraftResult> = Vec::new();
    let mut architect_reviews: Vec<RalplanReviewResult> = Vec::new();
    let mut critic_reviews: Vec<RalplanReviewResult> = Vec::new();
    let mut all_artifacts: HashMap<String, serde_json::Value> = HashMap::new();
    let mut latest_plan_path: Option<String> = None;

    for iteration in 0..max_iterations {
        let ctx = RalplanIterationContext {
            task: options.task.clone(),
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
                    iteration,
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

        if let Some(ref p) = draft.plan_path {
            latest_plan_path = Some(p.clone());
        }
        for (k, v) in &draft.artifacts {
            all_artifacts.insert(k.clone(), v.clone());
        }
        drafts.push(draft.clone());

        // Architect review phase
        let architect = match executor.architect_review(&ctx, &draft).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(RalplanRuntimeResult {
                    status: RalplanPhase::Failed,
                    iteration,
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
        architect_reviews.push(architect.clone());

        // Critic review phase
        let critic = match executor.critic_review(&ctx, &draft, &architect).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(RalplanRuntimeResult {
                    status: RalplanPhase::Failed,
                    iteration,
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
        critic_reviews.push(critic.clone());

        // If critic approves, we're done
        if critic.verdict == RalplanReviewVerdict::Approve {
            return Ok(RalplanRuntimeResult {
                status: RalplanPhase::Complete,
                iteration,
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
        iteration: max_iterations - 1,
        planning_complete: false,
        drafts,
        architect_reviews,
        critic_reviews,
        latest_plan_path,
        artifacts: all_artifacts,
        error: Some(format!(
            "consensus not reached after {} iterations",
            max_iterations
        )),
    })
}

/// Cancel a running consensus loop. Currently a no-op — mode lifecycle is
/// handled by the caller.
pub async fn cancel_ralplan_consensus() -> Result<(), OmxError> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Phase tests --

    #[test]
    fn phase_serde_roundtrip() {
        for phase in [
            RalplanPhase::Draft,
            RalplanPhase::ArchitectReview,
            RalplanPhase::CriticReview,
            RalplanPhase::Complete,
            RalplanPhase::Cancelled,
            RalplanPhase::Failed,
        ] {
            let json = serde_json::to_string(&phase).unwrap();
            let back: RalplanPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(phase, back);
        }
    }

    #[test]
    fn phase_serde_kebab_case() {
        assert_eq!(
            serde_json::to_string(&RalplanPhase::ArchitectReview).unwrap(),
            "\"architect-review\""
        );
        assert_eq!(
            serde_json::to_string(&RalplanPhase::CriticReview).unwrap(),
            "\"critic-review\""
        );
    }

    #[test]
    fn phase_display() {
        assert_eq!(RalplanPhase::Draft.to_string(), "draft");
        assert_eq!(
            RalplanPhase::ArchitectReview.to_string(),
            "architect-review"
        );
        assert_eq!(RalplanPhase::CriticReview.to_string(), "critic-review");
        assert_eq!(RalplanPhase::Complete.to_string(), "complete");
        assert_eq!(RalplanPhase::Cancelled.to_string(), "cancelled");
        assert_eq!(RalplanPhase::Failed.to_string(), "failed");
    }

    #[test]
    fn phase_is_terminal() {
        assert!(RalplanPhase::Complete.is_terminal());
        assert!(RalplanPhase::Cancelled.is_terminal());
        assert!(RalplanPhase::Failed.is_terminal());
        assert!(!RalplanPhase::Draft.is_terminal());
        assert!(!RalplanPhase::ArchitectReview.is_terminal());
        assert!(!RalplanPhase::CriticReview.is_terminal());
    }

    #[test]
    fn phase_is_active() {
        assert!(RalplanPhase::Draft.is_active());
        assert!(RalplanPhase::ArchitectReview.is_active());
        assert!(RalplanPhase::CriticReview.is_active());
        assert!(!RalplanPhase::Complete.is_active());
        assert!(!RalplanPhase::Cancelled.is_active());
        assert!(!RalplanPhase::Failed.is_active());
    }

    // -- Verdict tests --

    #[test]
    fn verdict_serde_roundtrip() {
        for v in [
            RalplanReviewVerdict::Approve,
            RalplanReviewVerdict::Iterate,
            RalplanReviewVerdict::Reject,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: RalplanReviewVerdict = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn verdict_display() {
        assert_eq!(RalplanReviewVerdict::Approve.to_string(), "approve");
        assert_eq!(RalplanReviewVerdict::Iterate.to_string(), "iterate");
        assert_eq!(RalplanReviewVerdict::Reject.to_string(), "reject");
    }

    // -- DraftResult / ReviewResult serde --

    #[test]
    fn draft_result_serde_roundtrip() {
        let dr = RalplanDraftResult {
            summary: Some("a plan".into()),
            plan_path: Some("/tmp/plan.md".into()),
            artifacts: HashMap::from([("key".into(), serde_json::json!("value"))]),
        };
        let json = serde_json::to_string(&dr).unwrap();
        let back: RalplanDraftResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.summary.as_deref(), Some("a plan"));
        assert_eq!(back.plan_path.as_deref(), Some("/tmp/plan.md"));
        assert_eq!(
            back.artifacts.get("key").unwrap(),
            &serde_json::json!("value")
        );
    }

    #[test]
    fn review_result_serde_roundtrip() {
        let rr = RalplanReviewResult {
            verdict: RalplanReviewVerdict::Iterate,
            summary: Some("needs work".into()),
            artifacts: HashMap::new(),
        };
        let json = serde_json::to_string(&rr).unwrap();
        let back: RalplanReviewResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.verdict, RalplanReviewVerdict::Iterate);
        assert_eq!(back.summary.as_deref(), Some("needs work"));
    }

    // -- IterationContext --

    #[test]
    fn iteration_context_defaults() {
        let ctx = RalplanIterationContext {
            task: "build something".into(),
            cwd: PathBuf::from("/tmp"),
            iteration: 0,
            prior_drafts: Vec::new(),
            architect_reviews: Vec::new(),
            critic_reviews: Vec::new(),
        };
        assert_eq!(ctx.task, "build something");
        assert_eq!(ctx.iteration, 0);
        assert!(ctx.prior_drafts.is_empty());
        assert!(ctx.architect_reviews.is_empty());
        assert!(ctx.critic_reviews.is_empty());
    }

    // -- RuntimeResult serde --

    #[test]
    fn runtime_result_serde_roundtrip() {
        let rr = RalplanRuntimeResult {
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
        let json = serde_json::to_string(&rr).unwrap();
        let back: RalplanRuntimeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, RalplanPhase::Complete);
        assert_eq!(back.iteration, 2);
        assert!(back.planning_complete);
        assert_eq!(back.latest_plan_path.as_deref(), Some("/tmp/plan.md"));
        assert!(back.error.is_none());
    }

    // -- max_iterations default --

    #[test]
    fn max_iterations_defaults_to_5() {
        let opts = RunRalplanConsensusOptions {
            task: "test".into(),
            cwd: None,
            max_iterations: None,
        };
        assert_eq!(opts.effective_max_iterations(), 5);
    }

    // -- Mock executor helpers --

    /// Executor that approves on a given iteration.
    struct MockExecutor {
        approve_on_iteration: u32,
    }

    #[async_trait::async_trait]
    impl RalplanConsensusExecutor for MockExecutor {
        async fn draft(
            &self,
            ctx: &RalplanIterationContext,
        ) -> Result<RalplanDraftResult, OmxError> {
            Ok(RalplanDraftResult {
                summary: Some(format!("draft iteration {}", ctx.iteration)),
                plan_path: Some(format!("/tmp/plan-{}.md", ctx.iteration)),
                artifacts: HashMap::new(),
            })
        }

        async fn architect_review(
            &self,
            _ctx: &RalplanIterationContext,
            _draft: &RalplanDraftResult,
        ) -> Result<RalplanReviewResult, OmxError> {
            Ok(RalplanReviewResult {
                verdict: RalplanReviewVerdict::Approve,
                summary: Some("looks good".into()),
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
                summary: Some(format!("critic iteration {}", ctx.iteration)),
                artifacts: HashMap::new(),
            })
        }
    }

    /// Executor that always fails at draft.
    struct FailingExecutor;

    #[async_trait::async_trait]
    impl RalplanConsensusExecutor for FailingExecutor {
        async fn draft(
            &self,
            _ctx: &RalplanIterationContext,
        ) -> Result<RalplanDraftResult, OmxError> {
            Err(OmxError::State("draft exploded".into()))
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

    // -- Consensus runtime tests --

    #[tokio::test]
    async fn consensus_completes_on_first_iteration() {
        let executor = MockExecutor {
            approve_on_iteration: 0,
        };
        let opts = RunRalplanConsensusOptions {
            task: "test task".into(),
            cwd: None,
            max_iterations: None,
        };
        let result = run_ralplan_consensus(&executor, opts).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Complete);
        assert!(result.planning_complete);
        assert_eq!(result.iteration, 0);
        assert_eq!(result.drafts.len(), 1);
        assert_eq!(result.architect_reviews.len(), 1);
        assert_eq!(result.critic_reviews.len(), 1);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn consensus_iterates_until_critic_approves() {
        let executor = MockExecutor {
            approve_on_iteration: 2,
        };
        let opts = RunRalplanConsensusOptions {
            task: "test task".into(),
            cwd: None,
            max_iterations: None,
        };
        let result = run_ralplan_consensus(&executor, opts).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Complete);
        assert!(result.planning_complete);
        assert_eq!(result.iteration, 2);
        assert_eq!(result.drafts.len(), 3);
        assert_eq!(result.architect_reviews.len(), 3);
        assert_eq!(result.critic_reviews.len(), 3);
        assert_eq!(result.latest_plan_path.as_deref(), Some("/tmp/plan-2.md"));
    }

    #[tokio::test]
    async fn consensus_fails_at_max_iterations() {
        let executor = MockExecutor {
            approve_on_iteration: 100, // never approves within limit
        };
        let opts = RunRalplanConsensusOptions {
            task: "test task".into(),
            cwd: None,
            max_iterations: Some(3),
        };
        let result = run_ralplan_consensus(&executor, opts).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Failed);
        assert!(!result.planning_complete);
        assert_eq!(result.drafts.len(), 3);
        assert!(result
            .error
            .as_deref()
            .unwrap()
            .contains("consensus not reached"));
    }

    #[tokio::test]
    async fn consensus_catches_executor_error() {
        let executor = FailingExecutor;
        let opts = RunRalplanConsensusOptions {
            task: "test task".into(),
            cwd: None,
            max_iterations: None,
        };
        let result = run_ralplan_consensus(&executor, opts).await.unwrap();
        assert_eq!(result.status, RalplanPhase::Failed);
        assert!(!result.planning_complete);
        assert!(result.error.as_deref().unwrap().contains("draft exploded"));
        assert!(result.drafts.is_empty());
    }
}
