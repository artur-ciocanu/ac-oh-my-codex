use serde::{Deserialize, Serialize};

/// A built-in hook descriptor shipped with OMX.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinHook {
    pub name: String,
    pub description: String,
    pub events: Vec<String>,
}

/// Returns all 6 built-in hook descriptors.
pub fn all_builtin_hooks() -> Vec<BuiltinHook> {
    vec![
        BuiltinHook {
            name: "notify-fallback-watcher".into(),
            description: "Monitors notification delivery failures, retries or escalates".into(),
            events: vec!["session-start".into(), "session-end".into()],
        },
        BuiltinHook {
            name: "notify-hook-auto-nudge".into(),
            description: "Detects idle workers and sends nudge notifications".into(),
            events: vec!["session-idle".into(), "turn-complete".into()],
        },
        BuiltinHook {
            name: "notify-hook-team-leader-nudge".into(),
            description: "Nudges team leader when workers await review".into(),
            events: vec!["turn-complete".into()],
        },
        BuiltinHook {
            name: "notify-hook-team-dispatch".into(),
            description: "Fires notification when team dispatches work".into(),
            events: vec!["turn-complete".into()],
        },
        BuiltinHook {
            name: "notify-hook-worker-idle".into(),
            description: "Fires notification when worker goes idle too long".into(),
            events: vec!["session-idle".into()],
        },
        BuiltinHook {
            name: "notify-hook-tmux-heal".into(),
            description: "Detects and repairs broken tmux sessions".into(),
            events: vec!["session-start".into(), "turn-complete".into()],
        },
    ]
}

/// Look up a built-in hook by name.
pub fn lookup_builtin(name: &str) -> Option<BuiltinHook> {
    all_builtin_hooks().into_iter().find(|h| h.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_builtins_have_unique_names() {
        let hooks = all_builtin_hooks();
        let names: HashSet<&str> = hooks.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names.len(), hooks.len());
    }

    #[test]
    fn count_is_six() {
        assert_eq!(all_builtin_hooks().len(), 6);
    }

    #[test]
    fn fallback_watcher_description_matches() {
        let hook = lookup_builtin("notify-fallback-watcher").unwrap();
        assert_eq!(
            hook.description,
            "Monitors notification delivery failures, retries or escalates"
        );
    }

    #[test]
    fn tmux_heal_exists() {
        assert!(lookup_builtin("notify-hook-tmux-heal").is_some());
    }

    #[test]
    fn lookup_by_name_works() {
        let hook = lookup_builtin("notify-hook-auto-nudge").unwrap();
        assert_eq!(hook.name, "notify-hook-auto-nudge");
        assert_eq!(
            hook.description,
            "Detects idle workers and sends nudge notifications"
        );
        assert_eq!(hook.events, vec!["session-idle", "turn-complete"]);
    }

    #[test]
    fn lookup_nonexistent_returns_none() {
        assert!(lookup_builtin("does-not-exist").is_none());
    }
}
