use crate::types::MuxError;

/// Prevents accidental kill of the leader pane in a multiplexer session.
#[derive(Debug, Clone)]
pub struct LeaderGuard {
    leader_pane_id: Option<String>,
}

impl LeaderGuard {
    /// Create a guard that protects the given pane from being killed.
    pub fn new(leader_pane_id: &str) -> Self {
        Self {
            leader_pane_id: Some(leader_pane_id.to_owned()),
        }
    }

    /// Create a guard that protects nothing.
    pub fn none() -> Self {
        Self {
            leader_pane_id: None,
        }
    }

    /// Returns `true` if `pane_id` is the protected leader pane.
    pub fn is_protected(&self, pane_id: &str) -> bool {
        self.leader_pane_id.as_deref() == Some(pane_id)
    }

    /// Returns `Err(MuxError::InvalidTarget)` when `target` matches the leader pane.
    pub fn check_kill_target(&self, target: &str) -> Result<(), MuxError> {
        if self.is_protected(target) {
            Err(MuxError::InvalidTarget(format!(
                "cannot kill leader pane: {target}"
            )))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_blocks_leader_pane_kill() {
        let guard = LeaderGuard::new("sess:0.0");
        assert!(guard.is_protected("sess:0.0"));
        assert!(!guard.is_protected("sess:0.1"));
    }

    #[test]
    fn check_kill_target_errors_for_leader() {
        let guard = LeaderGuard::new("sess:0.0");
        let result = guard.check_kill_target("sess:0.0");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MuxError::InvalidTarget(_)));
        assert!(err.to_string().contains("leader pane"));
    }

    #[test]
    fn check_kill_target_ok_for_workers() {
        let guard = LeaderGuard::new("sess:0.0");
        assert!(guard.check_kill_target("sess:0.1").is_ok());
        assert!(guard.check_kill_target("sess:1.0").is_ok());
    }

    #[test]
    fn guard_with_none_allows_all() {
        let guard = LeaderGuard::none();
        assert!(!guard.is_protected("sess:0.0"));
        assert!(guard.check_kill_target("sess:0.0").is_ok());
        assert!(guard.check_kill_target("anything").is_ok());
    }
}
