use omx_types::{TaskId, TaskStatus, WorkerId};
use std::time::Instant;

use crate::config::TeamConfig;
use crate::mailbox::MailboxMessage;
use crate::phase_controller::{DefaultPhaseController, PhaseController};
use crate::{TaskSnapshot, TeamSnapshot, WorkerSnapshot};

/// Mutable team state tracked by the orchestrator.
pub struct OrchestratorState {
    pub config: TeamConfig,
    pub start_time: Instant,
    pub tasks: Vec<(TaskId, TaskStatus, Vec<TaskId>)>,
    pub workers: Vec<(WorkerId, String, bool, Option<TaskId>)>,
    pub messages: Vec<MailboxMessage>,
}

impl OrchestratorState {
    pub fn new(config: TeamConfig) -> Self {
        let tasks: Vec<(TaskId, TaskStatus, Vec<TaskId>)> = config
            .tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let id = TaskId(format!("task-{i}"));
                let deps = t.depends_on.iter().map(|d| TaskId(d.clone())).collect();
                (id, TaskStatus::Pending, deps)
            })
            .collect();

        let workers: Vec<(WorkerId, String, bool, Option<TaskId>)> = config
            .workers
            .iter()
            .enumerate()
            .map(|(i, w)| (WorkerId(format!("worker-{i}")), w.role.clone(), true, None))
            .collect();

        Self {
            config,
            start_time: Instant::now(),
            tasks,
            workers,
            messages: Vec::new(),
        }
    }
}

/// Perform one monitor tick: snapshot state, infer phase.
pub fn tick(state: &OrchestratorState) -> TeamSnapshot {
    let phase_controller = DefaultPhaseController;

    let phase_input: Vec<_> = state
        .tasks
        .iter()
        .enumerate()
        .map(|(i, (_, status, _))| {
            let phase = state.config.tasks.get(i).and_then(|t| t.phase.clone());
            (status.clone(), phase)
        })
        .collect();

    let phase = phase_controller.infer_phase(&phase_input);

    let workers: Vec<WorkerSnapshot> = state
        .workers
        .iter()
        .map(|(id, role, alive, current_task)| WorkerSnapshot {
            id: id.clone(),
            role: role.clone(),
            alive: *alive,
            current_task: current_task.clone(),
        })
        .collect();

    let tasks: Vec<TaskSnapshot> = state
        .tasks
        .iter()
        .map(|(id, status, _)| {
            let assigned_to = state
                .workers
                .iter()
                .find(|(_, _, _, ct)| ct.as_ref() == Some(id))
                .map(|(wid, _, _, _)| wid.clone());
            TaskSnapshot {
                id: id.clone(),
                status: status.clone(),
                assigned_to,
                result: None,
            }
        })
        .collect();

    let snapshot = TeamSnapshot {
        team_name: state.config.name.0.clone(),
        phase,
        workers,
        tasks,
        uptime_seconds: state.start_time.elapsed().as_secs(),
    };

    tracing::debug!(
        phase = ?snapshot.phase,
        uptime = snapshot.uptime_seconds,
        "tick complete"
    );

    snapshot
}

/// Check if the team is done (all tasks completed or failed).
pub fn is_done(state: &OrchestratorState) -> bool {
    state
        .tasks
        .iter()
        .all(|(_, status, _)| *status == TaskStatus::Completed || *status == TaskStatus::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use omx_types::{TeamName, TeamPhase};

    fn make_config(task_count: usize) -> TeamConfig {
        TeamConfig {
            name: TeamName("test".into()),
            workers: vec![WorkerConfig {
                role: "executor".into(),
                model: None,
                provider: None,
            }],
            tasks: (0..task_count)
                .map(|i| TeamTask {
                    description: format!("task {i}"),
                    depends_on: vec![],
                    phase: None,
                })
                .collect(),
            governance: TeamGovernance::default(),
            worktree_mode: WorktreeMode::Shared,
            dispatch_mode: DispatchMode::Tmux,
            heartbeat_interval_secs: 30,
            heartbeat_stale_secs: 90,
            merge_strategy: crate::merge_strategy::MergeStrategy::default(),
            auto_commit: crate::auto_commit::AutoCommitConfig::default(),
            approval_required: false,
        }
    }

    #[test]
    fn orchestrator_state_initializes_tasks_and_workers() {
        let config = make_config(3);
        let state = OrchestratorState::new(config);
        assert_eq!(state.tasks.len(), 3);
        assert_eq!(state.workers.len(), 1);
        assert_eq!(state.tasks[0].0, TaskId("task-0".into()));
        assert_eq!(state.tasks[0].1, TaskStatus::Pending);
    }

    #[test]
    fn tick_returns_snapshot_with_plan_phase_when_all_pending() {
        let config = make_config(2);
        let state = OrchestratorState::new(config);
        let snapshot = tick(&state);
        assert_eq!(snapshot.phase, TeamPhase::Plan);
        assert_eq!(snapshot.tasks.len(), 2);
        assert_eq!(snapshot.workers.len(), 1);
    }

    #[test]
    fn is_done_when_all_completed() {
        let config = make_config(2);
        let mut state = OrchestratorState::new(config);
        state.tasks[0].1 = TaskStatus::Completed;
        state.tasks[1].1 = TaskStatus::Completed;
        assert!(is_done(&state));
    }

    #[test]
    fn is_done_false_when_pending_tasks_remain() {
        let config = make_config(2);
        let mut state = OrchestratorState::new(config);
        state.tasks[0].1 = TaskStatus::Completed;
        assert!(!is_done(&state));
    }
}
