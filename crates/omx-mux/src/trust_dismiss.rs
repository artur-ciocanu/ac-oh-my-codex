/// String patterns that indicate a trust/permission prompt from Claude.
pub const TRUST_PATTERNS: &[&str] = &[
    "Do you trust",
    "trust this project",
    "Allow tool access",
    "Do you want to trust",
];

/// Returns `true` if `pane_output` contains any trust-prompt pattern (case-insensitive).
pub fn contains_trust_prompt(pane_output: &str) -> bool {
    let lower = pane_output.to_lowercase();
    TRUST_PATTERNS
        .iter()
        .any(|pattern| lower.contains(&pattern.to_lowercase()))
}

/// Returns the key sequence used to dismiss a trust prompt.
pub fn trust_dismiss_keys() -> &'static str {
    "y"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_trust_prompt_in_output() {
        let output = "Some preamble\nDo you trust this folder?\n> ";
        assert!(contains_trust_prompt(output));
    }

    #[test]
    fn no_trust_prompt_in_normal_output() {
        let output = "Compiling omx-mux v0.1.0\n    Finished dev [unoptimized] target(s)";
        assert!(!contains_trust_prompt(output));
    }

    #[test]
    fn detect_claude_trust_prompt_variant() {
        let output = "Do you want to trust the authors of this project?";
        assert!(contains_trust_prompt(output));
    }

    #[test]
    fn dismiss_keys_returns_y() {
        assert_eq!(trust_dismiss_keys(), "y");
    }

    #[test]
    fn detect_permission_prompt() {
        let output = "Allow tool access for /home/user/project?";
        assert!(contains_trust_prompt(output));
    }
}
