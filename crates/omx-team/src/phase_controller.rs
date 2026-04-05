use omx_types::{TaskStatus, TeamPhase};

pub trait PhaseController: Send + Sync {
    fn infer_phase(&self, tasks: &[(TaskStatus, Option<TeamPhase>)]) -> TeamPhase;
    fn recommend_roles(&self, phase: &TeamPhase) -> Vec<String>;
}

pub struct DefaultPhaseController;

impl PhaseController for DefaultPhaseController {
    fn infer_phase(&self, tasks: &[(TaskStatus, Option<TeamPhase>)]) -> TeamPhase {
        if tasks.is_empty() {
            return TeamPhase::Plan;
        }

        // Any failed task → Fix phase
        if tasks.iter().any(|(s, _)| *s == TaskStatus::Failed) {
            return TeamPhase::Fix;
        }

        // All completed → Verify phase
        if tasks.iter().all(|(s, _)| *s == TaskStatus::Completed) {
            return TeamPhase::Verify;
        }

        // Any in-progress → use the phase annotation of in-progress tasks, default Exec
        if tasks.iter().any(|(s, _)| *s == TaskStatus::InProgress) {
            return tasks
                .iter()
                .find(|(s, _)| *s == TaskStatus::InProgress)
                .and_then(|(_, phase)| phase.clone())
                .unwrap_or(TeamPhase::Exec);
        }

        // All pending → Plan
        TeamPhase::Plan
    }

    fn recommend_roles(&self, phase: &TeamPhase) -> Vec<String> {
        match phase {
            TeamPhase::Plan => vec!["planner".into(), "researcher".into()],
            TeamPhase::Prd => vec!["writer".into(), "reviewer".into()],
            TeamPhase::Exec => vec!["executor".into(), "reviewer".into()],
            TeamPhase::Verify => vec!["tester".into(), "reviewer".into()],
            TeamPhase::Fix => vec!["executor".into(), "debugger".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_phase_all_pending_is_plan() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Pending, None),
            (TaskStatus::Pending, None),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Plan);
    }

    #[test]
    fn infer_phase_some_in_progress_is_exec() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Completed, Some(TeamPhase::Plan)),
            (TaskStatus::InProgress, Some(TeamPhase::Exec)),
            (TaskStatus::Pending, Some(TeamPhase::Exec)),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Exec);
    }

    #[test]
    fn infer_phase_all_completed_is_verify() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Completed, Some(TeamPhase::Exec)),
            (TaskStatus::Completed, Some(TeamPhase::Exec)),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Verify);
    }

    #[test]
    fn infer_phase_any_failed_is_fix() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Completed, Some(TeamPhase::Exec)),
            (TaskStatus::Failed, Some(TeamPhase::Exec)),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Fix);
    }

    #[test]
    fn recommend_roles_for_exec_phase() {
        let ctrl = DefaultPhaseController;
        let roles = ctrl.recommend_roles(&TeamPhase::Exec);
        assert!(roles.contains(&"executor".to_string()));
        assert!(roles.contains(&"reviewer".to_string()));
    }

    #[test]
    fn recommend_roles_for_plan_phase() {
        let ctrl = DefaultPhaseController;
        let roles = ctrl.recommend_roles(&TeamPhase::Plan);
        assert!(roles.contains(&"planner".to_string()));
        assert!(roles.contains(&"researcher".to_string()));
    }
}
