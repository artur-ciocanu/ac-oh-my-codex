use ratatui::style::Color;

pub struct ModeIndicator {
    pub label: &'static str,
    pub icon: &'static str,
    pub color: Color,
}

pub fn indicator_for(mode_name: &str) -> ModeIndicator {
    match mode_name.to_lowercase().as_str() {
        "autopilot" => ModeIndicator {
            label: "AUTOPILOT",
            icon: "A",
            color: Color::Green,
        },
        "autoresearch" => ModeIndicator {
            label: "AUTORESEARCH",
            icon: "R",
            color: Color::Blue,
        },
        "deep-interview" | "deepinterview" => ModeIndicator {
            label: "INTERVIEW",
            icon: "I",
            color: Color::Magenta,
        },
        "ralph" => ModeIndicator {
            label: "RALPH",
            icon: "P",
            color: Color::Yellow,
        },
        "ultrawork" => ModeIndicator {
            label: "ULTRAWORK",
            icon: "U",
            color: Color::Cyan,
        },
        "team" => ModeIndicator {
            label: "TEAM",
            icon: "T",
            color: Color::Red,
        },
        "ultraqa" => ModeIndicator {
            label: "ULTRAQA",
            icon: "Q",
            color: Color::LightGreen,
        },
        "ralplan" => ModeIndicator {
            label: "RALPLAN",
            icon: "L",
            color: Color::LightBlue,
        },
        _ => ModeIndicator {
            label: "IDLE",
            icon: "-",
            color: Color::Gray,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_modes_return_distinct_indicators() {
        let modes = [
            "autopilot",
            "autoresearch",
            "deep-interview",
            "ralph",
            "ultrawork",
            "team",
            "ultraqa",
            "ralplan",
        ];
        let indicators: Vec<_> = modes.iter().map(|m| indicator_for(m)).collect();
        let labels: std::collections::HashSet<_> = indicators.iter().map(|i| i.label).collect();
        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn unknown_mode_returns_idle() {
        let ind = indicator_for("nonexistent");
        assert_eq!(ind.label, "IDLE");
    }

    #[test]
    fn case_insensitive_lookup() {
        let ind = indicator_for("Autopilot");
        assert_eq!(ind.label, "AUTOPILOT");
    }
}
