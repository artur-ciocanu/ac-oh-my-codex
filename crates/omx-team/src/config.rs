use omx_types::{OmxError, TeamName, TeamPhase};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub name: TeamName,
    pub workers: Vec<WorkerConfig>,
    pub tasks: Vec<TeamTask>,
    pub governance: TeamGovernance,
    pub worktree_mode: WorktreeMode,
    pub dispatch_mode: DispatchMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub role: String,
    pub model: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask {
    pub description: String,
    pub depends_on: Vec<String>,
    pub phase: Option<TeamPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamGovernance {
    pub nested_teams_allowed: bool,
    pub auto_scale: bool,
    pub max_workers: u8,
}

impl Default for TeamGovernance {
    fn default() -> Self {
        Self {
            nested_teams_allowed: false,
            auto_scale: false,
            max_workers: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeMode {
    Shared,
    PerWorker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchMode {
    Tmux,
}

pub fn parse_team_spec(
    worker_count: u32,
    role: &str,
    task_description: &str,
    model: Option<String>,
) -> Result<TeamConfig, OmxError> {
    if worker_count == 0 {
        return Err(OmxError::Team("worker count must be at least 1".into()));
    }

    let team_id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
    let name = TeamName(format!("omx-team-{team_id}"));

    let workers = (0..worker_count)
        .map(|_| WorkerConfig {
            role: role.to_string(),
            model: model.clone(),
            provider: None,
        })
        .collect();

    let tasks = vec![TeamTask {
        description: task_description.to_string(),
        depends_on: vec![],
        phase: None,
    }];

    Ok(TeamConfig {
        name,
        workers,
        tasks,
        governance: TeamGovernance::default(),
        worktree_mode: WorktreeMode::PerWorker,
        dispatch_mode: DispatchMode::Tmux,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_defaults_disable_nesting() {
        let gov = TeamGovernance::default();
        assert!(!gov.nested_teams_allowed);
        assert!(!gov.auto_scale);
        assert_eq!(gov.max_workers, 10);
    }

    #[test]
    fn parse_team_spec_3_executors() {
        let config = parse_team_spec(3, "executor", "build the feature", None).unwrap();
        assert!(config.name.0.starts_with("omx-team-"));
        assert_eq!(config.workers.len(), 3);
        assert_eq!(config.workers[0].role, "executor");
        assert_eq!(config.tasks.len(), 1);
        assert_eq!(config.tasks[0].description, "build the feature");
        assert!(!config.governance.nested_teams_allowed);
        assert_eq!(config.worktree_mode, WorktreeMode::PerWorker);
        assert_eq!(config.dispatch_mode, DispatchMode::Tmux);
    }

    #[test]
    fn parse_team_spec_with_model_override() {
        let config = parse_team_spec(2, "planner", "design API", Some("o3".into())).unwrap();
        assert_eq!(config.workers.len(), 2);
        assert_eq!(config.workers[0].model, Some("o3".into()));
    }

    #[test]
    fn parse_team_spec_zero_workers_errors() {
        let result = parse_team_spec(0, "executor", "task", None);
        assert!(result.is_err());
    }
}
