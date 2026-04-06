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
// Pipeline orchestrator
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

fn validate_config(config: &PipelineConfig) -> Result<(), omx_types::OmxError> {
    use std::collections::HashSet;

    if config.name.is_empty() {
        return Err(omx_types::OmxError::Pipeline(
            "pipeline name must not be empty".into(),
        ));
    }
    if config.task.is_empty() {
        return Err(omx_types::OmxError::Pipeline(
            "pipeline task must not be empty".into(),
        ));
    }
    if config.stages.is_empty() {
        return Err(omx_types::OmxError::Pipeline(
            "pipeline must have at least one stage".into(),
        ));
    }
    if let Some(max) = config.max_ralph_iterations {
        if max == 0 {
            return Err(omx_types::OmxError::Pipeline(
                "max_ralph_iterations must be positive".into(),
            ));
        }
    }
    if let Some(wc) = config.worker_count {
        if wc == 0 {
            return Err(omx_types::OmxError::Pipeline(
                "worker_count must be positive".into(),
            ));
        }
    }

    let mut seen = HashSet::new();
    for stage in &config.stages {
        if !seen.insert(stage.name().to_string()) {
            return Err(omx_types::OmxError::Pipeline(format!(
                "duplicate stage name: {}",
                stage.name()
            )));
        }
    }

    Ok(())
}

pub async fn run_pipeline(config: PipelineConfig) -> Result<PipelineResult, omx_types::OmxError> {
    validate_config(&config)?;

    let start = std::time::Instant::now();
    let cwd = config.cwd.clone().unwrap_or_else(|| PathBuf::from("."));
    let mut all_artifacts: HashMap<String, serde_json::Value> = HashMap::new();
    let mut stage_results: HashMap<String, StageResult> = HashMap::new();
    let mut previous_result: Option<StageResult> = None;

    for stage in &config.stages {
        let ctx = StageContext {
            task: config.task.clone(),
            artifacts: all_artifacts.clone(),
            previous_stage_result: previous_result.clone(),
            cwd: cwd.clone(),
            session_id: config.session_id.clone(),
        };

        if let Some(ref cb) = config.on_stage_transition {
            cb(stage.name(), &ctx);
        }

        if stage.can_skip(&ctx) {
            let result = StageResult {
                status: StageStatus::Skipped,
                artifacts: HashMap::new(),
                duration_ms: 0,
                error: None,
            };
            stage_results.insert(stage.name().to_string(), result.clone());
            previous_result = Some(result);
            continue;
        }

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

        // Merge artifacts keyed by stage name
        for (k, v) in &result.artifacts {
            all_artifacts.insert(format!("{}:{}", stage.name(), k), v.clone());
        }

        let is_failed = result.status == StageStatus::Failed;
        stage_results.insert(stage.name().to_string(), result.clone());
        previous_result = Some(result);

        if is_failed {
            return Ok(PipelineResult {
                status: PipelineStatus::Failed,
                stage_results,
                duration_ms: start.elapsed().as_millis() as u64,
                artifacts: all_artifacts,
                error: previous_result.as_ref().and_then(|r| r.error.clone()),
                failed_stage: Some(stage.name().to_string()),
            });
        }
    }

    Ok(PipelineResult {
        status: PipelineStatus::Completed,
        stage_results,
        duration_ms: start.elapsed().as_millis() as u64,
        artifacts: all_artifacts,
        error: None,
        failed_stage: None,
    })
}

pub fn can_resume_pipeline(state: &Option<PipelineModeStateExtension>) -> bool {
    match state {
        Some(ext) => ext.pipeline_stage_index < ext.pipeline_stages.len(),
        None => false,
    }
}

pub async fn cancel_pipeline() -> Result<(), omx_types::OmxError> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Mock stages --

    struct PassStage {
        stage_name: String,
    }

    #[async_trait::async_trait]
    impl PipelineStage for PassStage {
        fn name(&self) -> &str {
            &self.stage_name
        }

        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, omx_types::OmxError> {
            let mut artifacts = HashMap::new();
            artifacts.insert(
                format!("{}-output", self.stage_name),
                serde_json::json!("ok"),
            );
            Ok(StageResult {
                status: StageStatus::Completed,
                artifacts,
                duration_ms: 1,
                error: None,
            })
        }
    }

    struct FailStage;

    #[async_trait::async_trait]
    impl PipelineStage for FailStage {
        fn name(&self) -> &str {
            "fail"
        }

        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, omx_types::OmxError> {
            Ok(StageResult {
                status: StageStatus::Failed,
                artifacts: HashMap::new(),
                duration_ms: 1,
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

        fn can_skip(&self, _ctx: &StageContext) -> bool {
            true
        }

        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, omx_types::OmxError> {
            unreachable!("skippable stage should not be run")
        }
    }

    struct ErrorStage;

    #[async_trait::async_trait]
    impl PipelineStage for ErrorStage {
        fn name(&self) -> &str {
            "error"
        }

        async fn run(&self, _ctx: &StageContext) -> Result<StageResult, omx_types::OmxError> {
            Err(omx_types::OmxError::Pipeline("stage panicked".into()))
        }
    }

    fn make_config(stages: Vec<Box<dyn PipelineStage>>) -> PipelineConfig {
        PipelineConfig {
            name: "test".into(),
            task: "do something".into(),
            stages,
            cwd: None,
            session_id: None,
            max_ralph_iterations: None,
            worker_count: None,
            agent_type: None,
            on_stage_transition: None,
        }
    }

    // -- Serde roundtrip tests --

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

    // -- Pipeline orchestrator tests --

    #[tokio::test]
    async fn pipeline_runs_all_stages() {
        let config = make_config(vec![
            Box::new(PassStage {
                stage_name: "stage-a".into(),
            }),
            Box::new(PassStage {
                stage_name: "stage-b".into(),
            }),
        ]);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Completed);
        assert_eq!(result.stage_results.len(), 2);
        assert!(result.error.is_none());
        assert!(result.failed_stage.is_none());
    }

    #[tokio::test]
    async fn pipeline_stops_on_failure() {
        let config = make_config(vec![
            Box::new(PassStage {
                stage_name: "pass".into(),
            }),
            Box::new(FailStage),
            Box::new(PassStage {
                stage_name: "never".into(),
            }),
        ]);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Failed);
        assert_eq!(result.failed_stage.as_deref(), Some("fail"));
        assert_eq!(result.stage_results.len(), 2);
        assert!(!result.stage_results.contains_key("never"));
    }

    #[tokio::test]
    async fn pipeline_skips_skippable_stages() {
        let config = make_config(vec![
            Box::new(SkippableStage),
            Box::new(PassStage {
                stage_name: "pass".into(),
            }),
        ]);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Completed);
        assert_eq!(
            result.stage_results.get("skippable").unwrap().status,
            StageStatus::Skipped
        );
    }

    #[tokio::test]
    async fn pipeline_catches_stage_errors() {
        let config = make_config(vec![Box::new(ErrorStage)]);
        let result = run_pipeline(config).await.unwrap();
        assert_eq!(result.status, PipelineStatus::Failed);
        assert_eq!(result.failed_stage.as_deref(), Some("error"));
        let sr = result.stage_results.get("error").unwrap();
        assert_eq!(sr.status, StageStatus::Failed);
        assert!(sr.error.as_deref().unwrap().contains("stage panicked"));
    }

    #[tokio::test]
    async fn pipeline_validates_empty_name() {
        let config = PipelineConfig {
            name: "".into(),
            task: "something".into(),
            stages: vec![Box::new(PassStage {
                stage_name: "a".into(),
            })],
            cwd: None,
            session_id: None,
            max_ralph_iterations: None,
            worker_count: None,
            agent_type: None,
            on_stage_transition: None,
        };
        let err = run_pipeline(config).await.unwrap_err();
        assert!(err.to_string().contains("name must not be empty"));
    }

    #[tokio::test]
    async fn pipeline_validates_no_stages() {
        let config = PipelineConfig {
            name: "test".into(),
            task: "something".into(),
            stages: vec![],
            cwd: None,
            session_id: None,
            max_ralph_iterations: None,
            worker_count: None,
            agent_type: None,
            on_stage_transition: None,
        };
        let err = run_pipeline(config).await.unwrap_err();
        assert!(err.to_string().contains("at least one stage"));
    }

    #[tokio::test]
    async fn pipeline_validates_duplicate_stage_names() {
        let config = make_config(vec![
            Box::new(PassStage {
                stage_name: "dup".into(),
            }),
            Box::new(PassStage {
                stage_name: "dup".into(),
            }),
        ]);
        let err = run_pipeline(config).await.unwrap_err();
        assert!(err.to_string().contains("duplicate stage name"));
    }

    #[tokio::test]
    async fn pipeline_accumulates_artifacts() {
        let config = make_config(vec![
            Box::new(PassStage {
                stage_name: "stage-a".into(),
            }),
            Box::new(PassStage {
                stage_name: "stage-b".into(),
            }),
        ]);
        let result = run_pipeline(config).await.unwrap();
        assert!(result.artifacts.contains_key("stage-a:stage-a-output"));
        assert!(result.artifacts.contains_key("stage-b:stage-b-output"));
    }
}
