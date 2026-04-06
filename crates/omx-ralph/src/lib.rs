//! Phase validation, progress ledger, and visual feedback scoring.

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

impl RalphPhase {
    /// Returns `true` for terminal phases: Complete, Failed, Cancelled.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Failed | Self::Cancelled)
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
// Result / feedback / ledger types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RalphStateValidationResult {
    pub ok: bool,
    pub phase: Option<RalphPhase>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

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

#[derive(Debug, Clone)]
pub struct RalphCanonicalArtifacts {
    pub canonical_prd_path: Option<PathBuf>,
    pub canonical_progress_path: PathBuf,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Constants --

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(VISUAL_NEXT_ACTIONS_LIMIT, 5);
        assert_eq!(VISUAL_FEEDBACK_MAX_ENTRIES, 30);
        assert!((DEFAULT_VISUAL_THRESHOLD - 90.0).abs() < f64::EPSILON);
    }

    // -- Phase tests --

    #[test]
    fn phase_serde_roundtrip() {
        for phase in [
            RalphPhase::Starting,
            RalphPhase::Executing,
            RalphPhase::Verifying,
            RalphPhase::Fixing,
            RalphPhase::Complete,
            RalphPhase::Failed,
            RalphPhase::Cancelled,
        ] {
            let json = serde_json::to_string(&phase).unwrap();
            let back: RalphPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(phase, back);
        }
    }

    #[test]
    fn phase_display() {
        assert_eq!(RalphPhase::Starting.to_string(), "starting");
        assert_eq!(RalphPhase::Executing.to_string(), "executing");
        assert_eq!(RalphPhase::Verifying.to_string(), "verifying");
        assert_eq!(RalphPhase::Fixing.to_string(), "fixing");
        assert_eq!(RalphPhase::Complete.to_string(), "complete");
        assert_eq!(RalphPhase::Failed.to_string(), "failed");
        assert_eq!(RalphPhase::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn phase_is_terminal() {
        assert!(RalphPhase::Complete.is_terminal());
        assert!(RalphPhase::Failed.is_terminal());
        assert!(RalphPhase::Cancelled.is_terminal());
        assert!(!RalphPhase::Starting.is_terminal());
        assert!(!RalphPhase::Executing.is_terminal());
        assert!(!RalphPhase::Verifying.is_terminal());
        assert!(!RalphPhase::Fixing.is_terminal());
    }

    // -- VisualVerdictStatus tests --

    #[test]
    fn visual_verdict_serde_roundtrip() {
        for v in [
            VisualVerdictStatus::Pass,
            VisualVerdictStatus::Fail,
            VisualVerdictStatus::Ambiguous,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: VisualVerdictStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    // -- Visual feedback serde --

    #[test]
    fn visual_feedback_serde_roundtrip() {
        let fb = RalphVisualFeedback {
            score: 95.5,
            verdict: VisualVerdictStatus::Pass,
            category_match: true,
            differences: vec!["color shift".into()],
            suggestions: vec!["adjust hue".into()],
            reasoning: Some("close match".into()),
            threshold: Some(90.0),
        };
        let json = serde_json::to_string(&fb).unwrap();
        let back: RalphVisualFeedback = serde_json::from_str(&json).unwrap();
        assert!((back.score - 95.5).abs() < f64::EPSILON);
        assert_eq!(back.verdict, VisualVerdictStatus::Pass);
        assert!(back.category_match);
        assert_eq!(back.differences.len(), 1);
        assert_eq!(back.suggestions.len(), 1);
        assert_eq!(back.reasoning.as_deref(), Some("close match"));
        assert!((back.threshold.unwrap() - 90.0).abs() < f64::EPSILON);
    }

    // -- Progress ledger --

    #[test]
    fn progress_ledger_empty_default() {
        let ledger = RalphProgressLedger::default();
        assert_eq!(ledger.schema_version, 2);
        assert!(ledger.source.is_none());
        assert!(ledger.source_sha256.is_none());
        assert!(ledger.strategy.is_none());
        assert!(ledger.created_at.is_none());
        assert!(ledger.updated_at.is_none());
        assert!(ledger.entries.is_empty());
        assert!(ledger.visual_feedback.is_empty());
    }

    #[test]
    fn progress_ledger_serde_roundtrip() {
        let ledger = RalphProgressLedger {
            schema_version: 2,
            source: Some("test".into()),
            source_sha256: Some("abc123".into()),
            strategy: Some("full".into()),
            created_at: Some("2026-04-05T00:00:00Z".into()),
            updated_at: Some("2026-04-05T01:00:00Z".into()),
            entries: vec![RalphProgressEntry {
                content: "step 1 done".into(),
                created_at: "2026-04-05T00:30:00Z".into(),
            }],
            visual_feedback: vec![RalphVisualFeedback {
                score: 80.0,
                verdict: VisualVerdictStatus::Fail,
                category_match: false,
                differences: vec!["layout".into()],
                suggestions: vec!["realign".into()],
                reasoning: None,
                threshold: None,
            }],
        };
        let json = serde_json::to_string(&ledger).unwrap();
        let back: RalphProgressLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, 2);
        assert_eq!(back.source.as_deref(), Some("test"));
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.visual_feedback.len(), 1);
    }
}
