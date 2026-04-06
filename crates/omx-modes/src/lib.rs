//! Mode lifecycle management with exclusive conflict checking.

use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Autopilot,
    Autoresearch,
    DeepInterview,
    Ralph,
    Ultrawork,
    Team,
    Ultraqa,
    Ralplan,
}

impl Mode {
    pub const ALL: &'static [Mode] = &[
        Mode::Autopilot,
        Mode::Autoresearch,
        Mode::DeepInterview,
        Mode::Ralph,
        Mode::Ultrawork,
        Mode::Team,
        Mode::Ultraqa,
        Mode::Ralplan,
    ];

    pub fn parse_name(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "autopilot" => Some(Self::Autopilot),
            "autoresearch" => Some(Self::Autoresearch),
            "deep-interview" | "deep_interview" | "deepinterview" => Some(Self::DeepInterview),
            "ralph" => Some(Self::Ralph),
            "ultrawork" => Some(Self::Ultrawork),
            "team" => Some(Self::Team),
            "ultraqa" => Some(Self::Ultraqa),
            "ralplan" => Some(Self::Ralplan),
            _ => None,
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Autopilot => write!(f, "autopilot"),
            Self::Autoresearch => write!(f, "autoresearch"),
            Self::DeepInterview => write!(f, "deep-interview"),
            Self::Ralph => write!(f, "ralph"),
            Self::Ultrawork => write!(f, "ultrawork"),
            Self::Team => write!(f, "team"),
            Self::Ultraqa => write!(f, "ultraqa"),
            Self::Ralplan => write!(f, "ralplan"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HudPreset {
    Minimal,
    Standard,
    Verbose,
}

impl Default for HudPreset {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub agent_overrides: HashMap<String, String>,
    pub hud_preset: HudPreset,
    pub env_overrides: HashMap<String, String>,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            agent_overrides: HashMap::new(),
            hud_preset: HudPreset::Standard,
            env_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeState {
    pub active: Mode,
    pub activated_at: DateTime<Utc>,
    pub session_id: String,
    pub config: ModeConfig,
}

// ---------------------------------------------------------------------------
// Conflict matrix
// ---------------------------------------------------------------------------

/// Returns true if modes `a` and `b` can be active simultaneously.
/// Orchestration modes (Ralph, Team, Autoresearch, Ralplan) conflict with each other.
pub fn is_compatible(a: Mode, b: Mode) -> bool {
    if a == b {
        return true; // Same mode is always compatible with itself
    }
    let orchestration = [Mode::Ralph, Mode::Team, Mode::Autoresearch, Mode::Ralplan];
    let a_is_orch = orchestration.contains(&a);
    let b_is_orch = orchestration.contains(&b);
    // Two orchestration modes conflict
    !(a_is_orch && b_is_orch)
}

// ---------------------------------------------------------------------------
// Mode manager
// ---------------------------------------------------------------------------

pub struct ModeManager {
    current: Option<ModeState>,
}

impl ModeManager {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn current(&self) -> Option<&ModeState> {
        self.current.as_ref()
    }

    pub fn activate(
        &mut self,
        mode: Mode,
        session_id: String,
        config: ModeConfig,
    ) -> Result<&ModeState, OmxError> {
        if let Some(ref current) = self.current {
            if !is_compatible(current.active, mode) {
                return Err(OmxError::Mode(format!(
                    "cannot activate '{}': conflicts with active mode '{}'",
                    mode, current.active
                )));
            }
        }

        self.current = Some(ModeState {
            active: mode,
            activated_at: Utc::now(),
            session_id,
            config,
        });

        Ok(self.current.as_ref().unwrap())
    }

    pub fn deactivate(&mut self) -> Result<(), OmxError> {
        if self.current.is_none() {
            return Err(OmxError::Mode("no active mode to deactivate".into()));
        }
        self.current = None;
        Ok(())
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_serde_roundtrip() {
        let mode = Mode::Ralph;
        let json = serde_json::to_string(&mode).unwrap();
        let parsed: Mode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mode);
    }

    #[test]
    fn mode_display() {
        assert_eq!(Mode::Autopilot.to_string(), "autopilot");
        assert_eq!(Mode::DeepInterview.to_string(), "deep-interview");
        assert_eq!(Mode::Ralplan.to_string(), "ralplan");
    }

    #[test]
    fn mode_parse_name_parses_known_modes() {
        assert_eq!(Mode::parse_name("autopilot"), Some(Mode::Autopilot));
        assert_eq!(Mode::parse_name("Ralph"), Some(Mode::Ralph));
        assert_eq!(
            Mode::parse_name("deep-interview"),
            Some(Mode::DeepInterview)
        );
        assert_eq!(
            Mode::parse_name("deep_interview"),
            Some(Mode::DeepInterview)
        );
        assert_eq!(Mode::parse_name("unknown"), None);
    }

    #[test]
    fn mode_all_has_eight_modes() {
        assert_eq!(Mode::ALL.len(), 8);
    }

    #[test]
    fn same_mode_is_compatible() {
        assert!(is_compatible(Mode::Ralph, Mode::Ralph));
        assert!(is_compatible(Mode::Team, Mode::Team));
    }

    #[test]
    fn orchestration_modes_conflict() {
        assert!(!is_compatible(Mode::Ralph, Mode::Team));
        assert!(!is_compatible(Mode::Team, Mode::Autoresearch));
        assert!(!is_compatible(Mode::Ralplan, Mode::Ralph));
        assert!(!is_compatible(Mode::Autoresearch, Mode::Ralplan));
    }

    #[test]
    fn non_orchestration_modes_are_compatible_with_orchestration() {
        assert!(is_compatible(Mode::Autopilot, Mode::Team));
        assert!(is_compatible(Mode::Ultrawork, Mode::Ralph));
        assert!(is_compatible(Mode::DeepInterview, Mode::Ralplan));
        assert!(is_compatible(Mode::Ultraqa, Mode::Autoresearch));
    }

    #[test]
    fn non_orchestration_modes_are_compatible_with_each_other() {
        assert!(is_compatible(Mode::Autopilot, Mode::Ultrawork));
        assert!(is_compatible(Mode::DeepInterview, Mode::Ultraqa));
    }

    #[test]
    fn mode_manager_starts_with_no_active_mode() {
        let mgr = ModeManager::new();
        assert!(mgr.current().is_none());
    }

    #[test]
    fn mode_manager_activate_sets_current() {
        let mut mgr = ModeManager::new();
        let state = mgr
            .activate(Mode::Autopilot, "sess-1".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Autopilot);
        assert_eq!(state.session_id, "sess-1");
    }

    #[test]
    fn mode_manager_activate_conflicting_mode_fails() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Team, "sess-1".into(), ModeConfig::default())
            .unwrap();

        let result = mgr.activate(Mode::Ralph, "sess-1".into(), ModeConfig::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("conflicts"));
    }

    #[test]
    fn mode_manager_activate_compatible_mode_replaces_current() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Autopilot, "sess-1".into(), ModeConfig::default())
            .unwrap();

        let state = mgr
            .activate(Mode::Ultrawork, "sess-1".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Ultrawork);
    }

    #[test]
    fn mode_manager_deactivate_clears_current() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Ralph, "sess-1".into(), ModeConfig::default())
            .unwrap();

        mgr.deactivate().unwrap();
        assert!(mgr.current().is_none());
    }

    #[test]
    fn mode_manager_deactivate_when_none_returns_error() {
        let mut mgr = ModeManager::new();
        let result = mgr.deactivate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no active mode"));
    }

    #[test]
    fn mode_manager_activate_after_deactivate_works() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Team, "sess-1".into(), ModeConfig::default())
            .unwrap();
        mgr.deactivate().unwrap();

        let state = mgr
            .activate(Mode::Ralph, "sess-2".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Ralph);
        assert_eq!(state.session_id, "sess-2");
    }

    #[test]
    fn hud_preset_default_is_standard() {
        assert_eq!(HudPreset::default(), HudPreset::Standard);
    }

    #[test]
    fn mode_state_serde_roundtrip() {
        let state = ModeState {
            active: Mode::Ralph,
            activated_at: Utc::now(),
            session_id: "sess-test".into(),
            config: ModeConfig::default(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: ModeState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.active, Mode::Ralph);
        assert_eq!(parsed.session_id, "sess-test");
    }
}
