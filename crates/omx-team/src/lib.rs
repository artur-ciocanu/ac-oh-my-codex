pub mod allocation;
pub mod approvals;
pub mod auto_commit;
pub mod commit_hygiene;
pub mod config;
pub mod dispatch;
pub mod events;
pub mod heartbeat;
pub mod locks;
pub mod mailbox;
pub mod merge_strategy;
pub mod orchestrator;
pub mod phase_controller;
pub mod role_router;
pub mod scaling;
pub mod task_queue;
pub mod tmux_session;
pub mod worker;
pub mod worktree;

use async_trait::async_trait;
use omx_types::{LeaseToken, OmxError, TaskId, TaskStatus, WorkerId};
use serde::{Deserialize, Serialize};

use crate::config::TeamConfig;
use crate::orchestrator::OrchestratorState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSnapshot {
    pub team_name: String,
    pub phase: omx_types::TeamPhase,
    pub workers: Vec<WorkerSnapshot>,
    pub tasks: Vec<TaskSnapshot>,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSnapshot {
    pub id: WorkerId,
    pub role: String,
    pub alive: bool,
    pub current_task: Option<TaskId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub status: TaskStatus,
    pub assigned_to: Option<WorkerId>,
    pub result: Option<String>,
}

#[async_trait]
pub trait TeamRuntime: Send + Sync {
    async fn start(&mut self, config: TeamConfig) -> Result<(), OmxError>;
    async fn monitor(&self) -> Result<TeamSnapshot, OmxError>;
    async fn send_message(
        &self,
        from: &WorkerId,
        to: &WorkerId,
        body: &str,
    ) -> Result<(), OmxError>;
    async fn broadcast(&self, from: &WorkerId, body: &str) -> Result<(), OmxError>;
    async fn claim_task(&self, worker: &WorkerId, task: &TaskId) -> Result<LeaseToken, OmxError>;
    async fn transition_task(
        &self,
        task: &TaskId,
        token: &LeaseToken,
        status: TaskStatus,
        result: Option<String>,
    ) -> Result<(), OmxError>;
    async fn shutdown(&mut self) -> Result<(), OmxError>;
}

/// Default team runtime that wires together all modules.
#[derive(Default)]
pub struct DefaultTeamRuntime {
    state: Option<OrchestratorState>,
}

impl DefaultTeamRuntime {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TeamRuntime for DefaultTeamRuntime {
    async fn start(&mut self, config: TeamConfig) -> Result<(), OmxError> {
        tracing::info!(team = %config.name.0, workers = config.workers.len(), "starting team");

        // Create tmux session
        let session = tmux_session::create_team_session(&config.name.0)?;
        tracing::info!(session = %session, "created team session");

        // Initialize orchestrator state
        self.state = Some(OrchestratorState::new(config));

        Ok(())
    }

    async fn monitor(&self) -> Result<TeamSnapshot, OmxError> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| OmxError::Team("team not started".into()))?;

        Ok(orchestrator::tick(state))
    }

    async fn send_message(
        &self,
        from: &WorkerId,
        to: &WorkerId,
        body: &str,
    ) -> Result<(), OmxError> {
        let _msg = mailbox::create_message(from, to, body)?;
        tracing::info!(from = %from.0, to = %to.0, "message queued");
        Ok(())
    }

    async fn broadcast(&self, from: &WorkerId, body: &str) -> Result<(), OmxError> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| OmxError::Team("team not started".into()))?;

        for (worker_id, _, _, _) in &state.workers {
            if *worker_id != *from {
                let _msg = mailbox::create_message(from, worker_id, body)?;
            }
        }

        tracing::info!(from = %from.0, "broadcast sent");
        Ok(())
    }

    async fn claim_task(&self, worker: &WorkerId, task: &TaskId) -> Result<LeaseToken, OmxError> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| OmxError::Team("team not started".into()))?;

        let (_, status, _) = state
            .tasks
            .iter()
            .find(|(id, _, _)| id == task)
            .ok_or_else(|| OmxError::Team(format!("task {} not found", task.0)))?;

        task_queue::claim(task, worker, status)
    }

    async fn transition_task(
        &self,
        task: &TaskId,
        token: &LeaseToken,
        status: TaskStatus,
        result: Option<String>,
    ) -> Result<(), OmxError> {
        task_queue::transition(task, token, status, result)
    }

    async fn shutdown(&mut self) -> Result<(), OmxError> {
        if let Some(state) = &self.state {
            let team_name = &state.config.name.0;
            tracing::info!(team = %team_name, "shutting down team");

            if let Err(e) = tmux_session::kill_team_session(team_name) {
                tracing::warn!(error = %e, "failed to kill team session");
            }
        }

        self.state = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_snapshot_serde_roundtrip() {
        let snapshot = TeamSnapshot {
            team_name: "test-team".into(),
            phase: omx_types::TeamPhase::Exec,
            workers: vec![],
            tasks: vec![],
            uptime_seconds: 42,
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: TeamSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.team_name, "test-team");
    }

    #[test]
    fn default_team_runtime_starts_uninitialized() {
        let runtime = DefaultTeamRuntime::new();
        assert!(runtime.state.is_none());
    }
}
