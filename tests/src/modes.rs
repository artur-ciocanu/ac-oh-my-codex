#[cfg(test)]
mod tests {
    use omx_modes::{is_compatible, Mode, ModeConfig, ModeManager};

    #[test]
    fn activate_mode_returns_state_with_correct_mode() {
        let mut mgr = ModeManager::new();
        let state = mgr
            .activate(Mode::Autopilot, "sess-int-1".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Autopilot);
        assert_eq!(state.session_id, "sess-int-1");
    }

    #[test]
    fn deactivate_clears_active_mode() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Autopilot, "sess-int-2".into(), ModeConfig::default())
            .unwrap();
        mgr.deactivate().unwrap();
        assert!(mgr.current().is_none());
    }

    #[test]
    fn conflict_ralph_vs_team() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Ralph, "sess-int-3".into(), ModeConfig::default())
            .unwrap();
        let result = mgr.activate(Mode::Team, "sess-int-3".into(), ModeConfig::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("conflicts"), "expected conflict error, got: {err_msg}");
    }

    #[test]
    fn conflict_autoresearch_vs_team() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Autoresearch, "sess-int-4".into(), ModeConfig::default())
            .unwrap();
        let result = mgr.activate(Mode::Team, "sess-int-4".into(), ModeConfig::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("conflicts"), "expected conflict error, got: {err_msg}");
    }

    #[test]
    fn conflict_ralplan_vs_team() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Ralplan, "sess-int-5".into(), ModeConfig::default())
            .unwrap();
        let result = mgr.activate(Mode::Team, "sess-int-5".into(), ModeConfig::default());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("conflicts"), "expected conflict error, got: {err_msg}");
    }

    #[test]
    fn non_conflicting_modes_coexist() {
        assert!(is_compatible(Mode::Autopilot, Mode::Ultrawork));
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Autopilot, "sess-int-6".into(), ModeConfig::default())
            .unwrap();
        let state = mgr
            .activate(Mode::Ultrawork, "sess-int-6".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Ultrawork);
    }
}
