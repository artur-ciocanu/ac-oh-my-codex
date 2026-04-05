use omx_types::{TaskId, WorkerId};

/// Allocate ready tasks to available workers using round-robin.
pub fn allocate(ready_tasks: &[TaskId], available_workers: &[WorkerId]) -> Vec<(TaskId, WorkerId)> {
    if available_workers.is_empty() {
        return Vec::new();
    }

    ready_tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let worker = &available_workers[i % available_workers.len()];
            (task.clone(), worker.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_round_robin() {
        let tasks = vec![
            TaskId("t1".into()),
            TaskId("t2".into()),
            TaskId("t3".into()),
        ];
        let workers = vec![WorkerId("w1".into()), WorkerId("w2".into())];

        let assignments = allocate(&tasks, &workers);
        assert_eq!(assignments.len(), 3);
        assert_eq!(assignments[0], (TaskId("t1".into()), WorkerId("w1".into())));
        assert_eq!(assignments[1], (TaskId("t2".into()), WorkerId("w2".into())));
        assert_eq!(assignments[2], (TaskId("t3".into()), WorkerId("w1".into())));
    }

    #[test]
    fn allocate_no_workers_returns_empty() {
        let tasks = vec![TaskId("t1".into())];
        let assignments = allocate(&tasks, &[]);
        assert!(assignments.is_empty());
    }

    #[test]
    fn allocate_no_tasks_returns_empty() {
        let workers = vec![WorkerId("w1".into())];
        let assignments = allocate(&[], &workers);
        assert!(assignments.is_empty());
    }

    #[test]
    fn allocate_single_worker_gets_all_tasks() {
        let tasks = vec![TaskId("t1".into()), TaskId("t2".into())];
        let workers = vec![WorkerId("w1".into())];
        let assignments = allocate(&tasks, &workers);
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].1, WorkerId("w1".into()));
        assert_eq!(assignments[1].1, WorkerId("w1".into()));
    }
}
