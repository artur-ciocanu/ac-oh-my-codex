use std::collections::HashSet;

use omx_types::{LeaseToken, OmxError, TaskId, TaskStatus, WorkerId};

/// Return tasks that are Pending and whose dependencies are all Completed.
pub fn ready_tasks(tasks: &[(TaskId, TaskStatus, Vec<TaskId>)]) -> Vec<TaskId> {
    let completed: HashSet<&TaskId> = tasks
        .iter()
        .filter(|(_, status, _)| *status == TaskStatus::Completed)
        .map(|(id, _, _)| id)
        .collect();

    tasks
        .iter()
        .filter(|(_, status, deps)| {
            *status == TaskStatus::Pending && deps.iter().all(|dep| completed.contains(dep))
        })
        .map(|(id, _, _)| id.clone())
        .collect()
}

/// Claim a task for a worker. Task must be Pending.
pub fn claim(
    task: &TaskId,
    worker: &WorkerId,
    current_status: &TaskStatus,
) -> Result<LeaseToken, OmxError> {
    if *current_status != TaskStatus::Pending {
        return Err(OmxError::Team(format!(
            "task {} is {:?}, not Pending — cannot claim",
            task.0, current_status
        )));
    }

    let token = format!(
        "lease-{}-{}-{}",
        task.0,
        worker.0,
        uuid::Uuid::new_v4().simple()
    );
    Ok(LeaseToken(token))
}

/// Transition a task to a new status. Token must be non-empty.
pub fn transition(
    task: &TaskId,
    token: &LeaseToken,
    new_status: TaskStatus,
    _result: Option<String>,
) -> Result<(), OmxError> {
    if token.0.is_empty() {
        return Err(OmxError::Team(format!(
            "empty lease token for task {}",
            task.0
        )));
    }

    tracing::info!(task = %task.0, new_status = ?new_status, "task transition");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_tasks_returns_pending_with_no_deps() {
        let tasks = vec![
            (TaskId("t1".into()), TaskStatus::Pending, vec![]),
            (
                TaskId("t2".into()),
                TaskStatus::Pending,
                vec![TaskId("t1".into())],
            ),
            (TaskId("t3".into()), TaskStatus::Completed, vec![]),
        ];
        let ready = ready_tasks(&tasks);
        assert_eq!(ready, vec![TaskId("t1".into())]);
    }

    #[test]
    fn ready_tasks_unblocks_when_deps_completed() {
        let tasks = vec![
            (TaskId("t1".into()), TaskStatus::Completed, vec![]),
            (
                TaskId("t2".into()),
                TaskStatus::Pending,
                vec![TaskId("t1".into())],
            ),
        ];
        let ready = ready_tasks(&tasks);
        assert_eq!(ready, vec![TaskId("t2".into())]);
    }

    #[test]
    fn ready_tasks_skips_in_progress() {
        let tasks = vec![(TaskId("t1".into()), TaskStatus::InProgress, vec![])];
        let ready = ready_tasks(&tasks);
        assert!(ready.is_empty());
    }

    #[test]
    fn claim_generates_lease_token() {
        let token = claim(
            &TaskId("t1".into()),
            &WorkerId("w1".into()),
            &TaskStatus::Pending,
        )
        .unwrap();
        assert!(!token.0.is_empty());
    }

    #[test]
    fn claim_rejects_non_pending_tasks() {
        let result = claim(
            &TaskId("t1".into()),
            &WorkerId("w1".into()),
            &TaskStatus::InProgress,
        );
        assert!(result.is_err());
    }

    #[test]
    fn transition_succeeds_with_valid_token() {
        let token = LeaseToken("valid-token".into());
        let result = transition(
            &TaskId("t1".into()),
            &token,
            TaskStatus::Completed,
            Some("done".into()),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn transition_rejects_empty_token() {
        let token = LeaseToken("".into());
        let result = transition(&TaskId("t1".into()), &token, TaskStatus::Completed, None);
        assert!(result.is_err());
    }
}
