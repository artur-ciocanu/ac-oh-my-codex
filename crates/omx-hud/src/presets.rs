use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

pub fn preset_layout(preset: HudPreset) -> PresetLayout {
    match preset {
        HudPreset::Minimal => PresetLayout {
            header_height: 3,
            stats_height: 3,
            show_git: false,
            show_tokens: false,
        },
        HudPreset::Standard => PresetLayout {
            header_height: 3,
            stats_height: 5,
            show_git: true,
            show_tokens: false,
        },
        HudPreset::Verbose => PresetLayout {
            header_height: 3,
            stats_height: 7,
            show_git: true,
            show_tokens: true,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetLayout {
    pub header_height: u16,
    pub stats_height: u16,
    pub show_git: bool,
    pub show_tokens: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preset_is_standard() {
        assert_eq!(HudPreset::default(), HudPreset::Standard);
    }

    #[test]
    fn minimal_hides_git_and_tokens() {
        let layout = preset_layout(HudPreset::Minimal);
        assert!(!layout.show_git);
        assert!(!layout.show_tokens);
    }

    #[test]
    fn verbose_shows_everything() {
        let layout = preset_layout(HudPreset::Verbose);
        assert!(layout.show_git);
        assert!(layout.show_tokens);
    }

    #[test]
    fn standard_shows_git_not_tokens() {
        let layout = preset_layout(HudPreset::Standard);
        assert!(layout.show_git);
        assert!(!layout.show_tokens);
    }

    #[test]
    fn serde_roundtrip() {
        let preset = HudPreset::Verbose;
        let json = serde_json::to_string(&preset).unwrap();
        let parsed: HudPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, preset);
    }
}
