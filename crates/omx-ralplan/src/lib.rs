//! Consensus planning with Draft → ArchitectReview → CriticReview loop.

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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn max_iterations_defaults_to_5() {
        let opts = RunRalplanConsensusOptions {
            task: "test".into(),
            cwd: None,
            max_iterations: None,
        };
        assert_eq!(opts.effective_max_iterations(), 5);
    }
}
