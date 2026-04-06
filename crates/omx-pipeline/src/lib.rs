//! Sequential stage executor with built-in stage factories.

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

#[async_trait::async_trait]
pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self, ctx: &StageContext) -> Result<StageResult, omx_types::OmxError>;
    fn can_skip(&self, _ctx: &StageContext) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Pipeline types
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

// ---------------------------------------------------------------------------
// State extension and descriptors
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_status_serde_roundtrip() {
        for status in [
            StageStatus::Completed,
            StageStatus::Failed,
            StageStatus::Skipped,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: StageStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn pipeline_status_serde_roundtrip() {
        for status in [
            PipelineStatus::Completed,
            PipelineStatus::Failed,
            PipelineStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: PipelineStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn stage_result_serde_roundtrip() {
        let sr = StageResult {
            status: StageStatus::Completed,
            artifacts: HashMap::from([("key".into(), serde_json::json!("value"))]),
            duration_ms: 42,
            error: None,
        };
        let json = serde_json::to_string(&sr).unwrap();
        let back: StageResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, StageStatus::Completed);
        assert_eq!(back.duration_ms, 42);
        assert!(back.error.is_none());
    }

    #[test]
    fn pipeline_result_serde_roundtrip() {
        let pr = PipelineResult {
            status: PipelineStatus::Completed,
            stage_results: HashMap::new(),
            duration_ms: 100,
            artifacts: HashMap::new(),
            error: None,
            failed_stage: None,
        };
        let json = serde_json::to_string(&pr).unwrap();
        let back: PipelineResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, PipelineStatus::Completed);
        assert_eq!(back.duration_ms, 100);
    }

    #[test]
    fn pipeline_mode_state_extension_serde_roundtrip() {
        let ext = PipelineModeStateExtension {
            pipeline_name: "autopilot".into(),
            pipeline_stages: vec!["ralplan".into(), "team-exec".into()],
            pipeline_stage_index: 1,
            pipeline_stage_results: HashMap::new(),
            pipeline_max_ralph_iterations: 10,
            pipeline_worker_count: 2,
            pipeline_agent_type: "executor".into(),
        };
        let json = serde_json::to_string(&ext).unwrap();
        let back: PipelineModeStateExtension = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pipeline_name, "autopilot");
        assert_eq!(back.pipeline_stages.len(), 2);
        assert_eq!(back.pipeline_stage_index, 1);
    }

    #[test]
    fn team_exec_descriptor_serde_roundtrip() {
        let desc = TeamExecDescriptor {
            task: "build app".into(),
            worker_count: 3,
            agent_type: "executor".into(),
            staffing_plan: Some("/tmp/plan.md".into()),
            use_worktrees: true,
            cwd: PathBuf::from("/tmp"),
            extra_env: None,
        };
        let json = serde_json::to_string(&desc).unwrap();
        let back: TeamExecDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.task, "build app");
        assert_eq!(back.worker_count, 3);
    }

    #[test]
    fn ralph_verify_descriptor_serde_roundtrip() {
        let desc = RalphVerifyDescriptor {
            task: "verify app".into(),
            max_iterations: 5,
            cwd: PathBuf::from("/tmp"),
            session_id: Some("sess-1".into()),
            execution_artifacts: HashMap::new(),
        };
        let json = serde_json::to_string(&desc).unwrap();
        let back: RalphVerifyDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.task, "verify app");
        assert_eq!(back.max_iterations, 5);
    }
}
