use omx_types::TeamPhase;

/// Recommend roles for a given phase. Delegates to phase_controller logic.
pub fn recommend_roles(phase: &TeamPhase) -> Vec<String> {
    use crate::phase_controller::{DefaultPhaseController, PhaseController};
    DefaultPhaseController.recommend_roles(phase)
}

/// Route a task description to the best matching role from available roles.
/// Uses keyword-based intent matching.
pub fn route_task(description: &str, available_roles: &[String]) -> Option<String> {
    if available_roles.is_empty() {
        return None;
    }

    let desc_lower = description.to_lowercase();

    let role_keywords: &[(&str, &[&str])] = &[
        ("planner", &["plan", "design", "architect", "strategy"]),
        (
            "researcher",
            &["research", "investigate", "explore", "analyze"],
        ),
        (
            "executor",
            &[
                "implement",
                "build",
                "create",
                "write",
                "code",
                "add",
                "fix",
            ],
        ),
        ("reviewer", &["review", "check", "audit", "verify"]),
        ("tester", &["test", "validate", "qa", "quality"]),
        ("writer", &["document", "write docs", "readme", "spec"]),
        ("debugger", &["debug", "diagnose", "troubleshoot", "bisect"]),
    ];

    let mut best_role: Option<&String> = None;
    let mut best_score = 0usize;

    for role in available_roles {
        let role_lower = role.to_lowercase();
        if let Some((_, keywords)) = role_keywords.iter().find(|(r, _)| *r == role_lower) {
            let score = keywords
                .iter()
                .filter(|kw| desc_lower.contains(**kw))
                .count();
            if score > best_score {
                best_score = score;
                best_role = Some(role);
            }
        }
    }

    best_role.or(available_roles.first()).map(|r| r.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommend_roles_delegates_to_phase_controller() {
        let roles = recommend_roles(&TeamPhase::Exec);
        assert!(roles.contains(&"executor".to_string()));
    }

    #[test]
    fn route_task_matches_executor_for_implement() {
        let roles = vec!["planner".into(), "executor".into(), "reviewer".into()];
        let result = route_task("implement the login feature", &roles);
        assert_eq!(result, Some("executor".into()));
    }

    #[test]
    fn route_task_matches_planner_for_design() {
        let roles = vec!["planner".into(), "executor".into()];
        let result = route_task("design the API architecture", &roles);
        assert_eq!(result, Some("planner".into()));
    }

    #[test]
    fn route_task_falls_back_to_first_role() {
        let roles = vec!["specialist".into()];
        let result = route_task("do something unusual", &roles);
        assert_eq!(result, Some("specialist".into()));
    }

    #[test]
    fn route_task_empty_roles_returns_none() {
        let result = route_task("anything", &[]);
        assert_eq!(result, None);
    }

    #[test]
    fn route_task_multiple_keywords_picks_highest_score() {
        let roles = vec!["executor".into(), "tester".into()];
        let result = route_task("implement and build the module", &roles);
        assert_eq!(result, Some("executor".into()));
    }
}
