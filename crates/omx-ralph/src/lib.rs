//! Phase validation, progress ledger, and visual feedback scoring.

use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
// Core functions
// ---------------------------------------------------------------------------

/// Normalize a raw phase string into a [`RalphPhase`].
///
/// Trims whitespace, lowercases, then matches against canonical names.
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
        other => Err(OmxError::Ralph(format!("unknown phase: {other}"))),
    }
}

/// Validate a candidate ralph state JSON value.
///
/// Checks phase validity, terminal-phase constraints, and iteration bounds.
pub fn validate_ralph_state(candidate: &serde_json::Value) -> RalphStateValidationResult {
    let active = candidate
        .get("active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Parse phase
    let phase = match candidate.get("current_phase").and_then(|v| v.as_str()) {
        Some(raw) => match normalize_ralph_phase(raw) {
            Ok(p) => Some(p),
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
                Some(RalphPhase::Starting)
            } else {
                None
            }
        }
    };

    // Terminal phases require active=false
    if let Some(ref p) = phase {
        if p.is_terminal() && active {
            return RalphStateValidationResult {
                ok: false,
                phase: Some(*p),
                warning: None,
                error: Some(format!("terminal phase '{}' requires active=false", p)),
            };
        }
    }

    // Validate iteration >= 0 (if present)
    if let Some(iter_val) = candidate.get("iteration") {
        if let Some(n) = iter_val.as_i64() {
            if n < 0 {
                return RalphStateValidationResult {
                    ok: false,
                    phase,
                    warning: None,
                    error: Some("iteration must be >= 0".into()),
                };
            }
        }
    }

    // Validate max_iterations > 0 (if present)
    if let Some(max_val) = candidate.get("max_iterations") {
        if let Some(n) = max_val.as_i64() {
            if n <= 0 {
                return RalphStateValidationResult {
                    ok: false,
                    phase,
                    warning: None,
                    error: Some("max_iterations must be > 0".into()),
                };
            }
        }
    }

    RalphStateValidationResult {
        ok: true,
        phase,
        warning: None,
        error: None,
    }
}

// ---------------------------------------------------------------------------
// Canonical artifacts and visual feedback
// ---------------------------------------------------------------------------

fn state_dir(cwd: &Path, session_id: Option<&str>) -> PathBuf {
    let base = cwd.join(".omx").join("state");
    match session_id {
        Some(sid) => base.join(sid),
        None => base,
    }
}

/// Ensure canonical artifact directories and progress ledger exist.
///
/// Creates `.omx/plans/` and the state directory. If `ralph-progress.json`
/// does not exist, writes an empty ledger. Idempotent.
pub async fn ensure_canonical_artifacts(
    cwd: &Path,
    session_id: Option<&str>,
) -> Result<RalphCanonicalArtifacts, OmxError> {
    let plans_dir = cwd.join(".omx").join("plans");
    let state = state_dir(cwd, session_id);

    tokio::fs::create_dir_all(&plans_dir).await?;
    tokio::fs::create_dir_all(&state).await?;

    let progress_path = state.join("ralph-progress.json");
    if !progress_path.exists() {
        let ledger = RalphProgressLedger::new();
        let json = serde_json::to_string_pretty(&ledger)?;
        tokio::fs::write(&progress_path, json).await?;
    }

    Ok(RalphCanonicalArtifacts {
        canonical_prd_path: Some(plans_dir),
        canonical_progress_path: progress_path,
    })
}

/// Record visual feedback into the progress ledger.
///
/// Reads the existing ledger (or creates an empty one on parse failure),
/// appends the feedback, caps at [`VISUAL_FEEDBACK_MAX_ENTRIES`], updates
/// the `updated_at` timestamp, and writes back.
pub async fn record_visual_feedback(
    cwd: &Path,
    feedback: RalphVisualFeedback,
    session_id: Option<&str>,
) -> Result<(), OmxError> {
    let state = state_dir(cwd, session_id);
    let progress_path = state.join("ralph-progress.json");

    let mut ledger = if progress_path.exists() {
        let data = tokio::fs::read_to_string(&progress_path).await?;
        serde_json::from_str::<RalphProgressLedger>(&data)
            .unwrap_or_else(|_| RalphProgressLedger::new())
    } else {
        tokio::fs::create_dir_all(&state).await?;
        RalphProgressLedger::new()
    };

    ledger.visual_feedback.push(feedback);

    // Cap at max entries, keeping the most recent
    if ledger.visual_feedback.len() > VISUAL_FEEDBACK_MAX_ENTRIES {
        let excess = ledger.visual_feedback.len() - VISUAL_FEEDBACK_MAX_ENTRIES;
        ledger.visual_feedback.drain(..excess);
    }

    ledger.updated_at = Some(chrono::Utc::now().to_rfc3339());

    let json = serde_json::to_string_pretty(&ledger)?;
    tokio::fs::write(&progress_path, json).await?;

    Ok(())
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

    // -- normalize_ralph_phase --

    #[test]
    fn normalize_valid_phases() {
        assert_eq!(
            normalize_ralph_phase("starting").unwrap(),
            RalphPhase::Starting
        );
        assert_eq!(
            normalize_ralph_phase("executing").unwrap(),
            RalphPhase::Executing
        );
        assert_eq!(
            normalize_ralph_phase("verifying").unwrap(),
            RalphPhase::Verifying
        );
        assert_eq!(normalize_ralph_phase("fixing").unwrap(), RalphPhase::Fixing);
        assert_eq!(
            normalize_ralph_phase("complete").unwrap(),
            RalphPhase::Complete
        );
        assert_eq!(normalize_ralph_phase("failed").unwrap(), RalphPhase::Failed);
        assert_eq!(
            normalize_ralph_phase("cancelled").unwrap(),
            RalphPhase::Cancelled
        );
    }

    #[test]
    fn normalize_case_insensitive() {
        assert_eq!(
            normalize_ralph_phase("STARTING").unwrap(),
            RalphPhase::Starting
        );
        assert_eq!(
            normalize_ralph_phase("Executing").unwrap(),
            RalphPhase::Executing
        );
        assert_eq!(
            normalize_ralph_phase("COMPLETE").unwrap(),
            RalphPhase::Complete
        );
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert_eq!(
            normalize_ralph_phase("  starting  ").unwrap(),
            RalphPhase::Starting
        );
        assert_eq!(
            normalize_ralph_phase("\texecuting\n").unwrap(),
            RalphPhase::Executing
        );
    }

    #[test]
    fn normalize_rejects_empty() {
        assert!(normalize_ralph_phase("").is_err());
        assert!(normalize_ralph_phase("   ").is_err());
    }

    #[test]
    fn normalize_rejects_unknown() {
        let err = normalize_ralph_phase("bogus").unwrap_err();
        assert!(err.to_string().contains("unknown phase"));
    }

    // -- validate_ralph_state --

    #[test]
    fn validate_valid_active_state() {
        let state = serde_json::json!({
            "active": true,
            "current_phase": "executing",
            "iteration": 1,
            "max_iterations": 5,
        });
        let result = validate_ralph_state(&state);
        assert!(result.ok);
        assert_eq!(result.phase, Some(RalphPhase::Executing));
        assert!(result.error.is_none());
    }

    #[test]
    fn validate_auto_fills_defaults_when_active() {
        let state = serde_json::json!({ "active": true });
        let result = validate_ralph_state(&state);
        assert!(result.ok);
        assert_eq!(result.phase, Some(RalphPhase::Starting));
    }

    #[test]
    fn validate_terminal_requires_inactive() {
        let state = serde_json::json!({
            "active": true,
            "current_phase": "complete",
        });
        let result = validate_ralph_state(&state);
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap().contains("terminal phase"));
    }

    #[test]
    fn validate_rejects_negative_iteration() {
        let state = serde_json::json!({
            "active": true,
            "current_phase": "executing",
            "iteration": -1,
        });
        let result = validate_ralph_state(&state);
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap().contains("iteration"));
    }

    #[test]
    fn validate_rejects_zero_max_iterations() {
        let state = serde_json::json!({
            "active": true,
            "current_phase": "executing",
            "max_iterations": 0,
        });
        let result = validate_ralph_state(&state);
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap().contains("max_iterations"));
    }

    #[test]
    fn validate_rejects_invalid_phase() {
        let state = serde_json::json!({
            "active": true,
            "current_phase": "bogus",
        });
        let result = validate_ralph_state(&state);
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap().contains("unknown phase"));
    }

    // -- ensure_canonical_artifacts --

    #[tokio::test]
    async fn ensure_canonical_artifacts_creates_dirs_and_ledger() {
        let tmp = tempfile::tempdir().unwrap();
        let arts = ensure_canonical_artifacts(tmp.path(), None).await.unwrap();

        assert!(tmp.path().join(".omx/plans").is_dir());
        assert!(tmp.path().join(".omx/state").is_dir());
        assert!(arts.canonical_progress_path.exists());

        let data = tokio::fs::read_to_string(&arts.canonical_progress_path)
            .await
            .unwrap();
        let ledger: RalphProgressLedger = serde_json::from_str(&data).unwrap();
        assert_eq!(ledger.schema_version, 2);
        assert!(ledger.entries.is_empty());
    }

    #[tokio::test]
    async fn ensure_canonical_artifacts_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let arts1 = ensure_canonical_artifacts(tmp.path(), None).await.unwrap();
        let arts2 = ensure_canonical_artifacts(tmp.path(), None).await.unwrap();
        assert_eq!(arts1.canonical_progress_path, arts2.canonical_progress_path);
    }

    #[tokio::test]
    async fn ensure_canonical_artifacts_with_session_id() {
        let tmp = tempfile::tempdir().unwrap();
        let arts = ensure_canonical_artifacts(tmp.path(), Some("sess-42"))
            .await
            .unwrap();
        assert!(tmp.path().join(".omx/state/sess-42").is_dir());
        assert!(arts
            .canonical_progress_path
            .ends_with("sess-42/ralph-progress.json"));
    }

    // -- record_visual_feedback --

    #[tokio::test]
    async fn record_visual_feedback_appends_to_ledger() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_canonical_artifacts(tmp.path(), None).await.unwrap();

        let fb = RalphVisualFeedback {
            score: 92.0,
            verdict: VisualVerdictStatus::Pass,
            category_match: true,
            differences: vec![],
            suggestions: vec![],
            reasoning: None,
            threshold: None,
        };
        record_visual_feedback(tmp.path(), fb, None).await.unwrap();

        let data = tokio::fs::read_to_string(tmp.path().join(".omx/state/ralph-progress.json"))
            .await
            .unwrap();
        let ledger: RalphProgressLedger = serde_json::from_str(&data).unwrap();
        assert_eq!(ledger.visual_feedback.len(), 1);
        assert!((ledger.visual_feedback[0].score - 92.0).abs() < f64::EPSILON);
        assert!(ledger.updated_at.is_some());
    }

    #[tokio::test]
    async fn record_visual_feedback_caps_at_max_entries() {
        let tmp = tempfile::tempdir().unwrap();
        ensure_canonical_artifacts(tmp.path(), None).await.unwrap();

        // Write 35 entries
        for i in 0..35 {
            let fb = RalphVisualFeedback {
                score: i as f64,
                verdict: VisualVerdictStatus::Pass,
                category_match: true,
                differences: vec![],
                suggestions: vec![],
                reasoning: None,
                threshold: None,
            };
            record_visual_feedback(tmp.path(), fb, None).await.unwrap();
        }

        let data = tokio::fs::read_to_string(tmp.path().join(".omx/state/ralph-progress.json"))
            .await
            .unwrap();
        let ledger: RalphProgressLedger = serde_json::from_str(&data).unwrap();
        assert_eq!(ledger.visual_feedback.len(), VISUAL_FEEDBACK_MAX_ENTRIES);
        // The oldest entries (0..5) should have been dropped, first remaining = 5.0
        assert!((ledger.visual_feedback[0].score - 5.0).abs() < f64::EPSILON);
    }
}
