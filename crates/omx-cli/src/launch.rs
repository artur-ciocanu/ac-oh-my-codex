use omx_config::OmxConfig;
use std::path::{Path, PathBuf};

/// Phase 1: Validate config — load and validate TOML, check required fields.
pub fn validate_config(config: &OmxConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    if config.models.frontier.is_empty() {
        warnings.push("models.frontier is empty — defaulting to 'o3'".into());
    }
    if config.team.default_workers == 0 {
        warnings.push("team.default_workers is 0 — no workers will be spawned".into());
    }
    warnings
}

/// Phase 2: Inject AGENTS.md overlay — merge OMX agent instructions into project's AGENTS.md.
pub fn inject_agents_overlay(
    agents_md_path: &Path,
    overlay_content: &str,
) -> Result<(), std::io::Error> {
    let existing = if agents_md_path.exists() {
        std::fs::read_to_string(agents_md_path)?
    } else {
        String::new()
    };

    let start_marker = "<!-- OMX:START -->";
    let end_marker = "<!-- OMX:END -->";
    let section = format!("{start_marker}\n{overlay_content}\n{end_marker}");

    let new_content = if existing.contains(start_marker) && existing.contains(end_marker) {
        let start = existing.find(start_marker).unwrap();
        let end = existing.find(end_marker).unwrap() + end_marker.len();
        let end = if existing[end..].starts_with('\n') {
            end + 1
        } else {
            end
        };
        format!("{}{}\n{}", &existing[..start], section, &existing[end..])
    } else if existing.is_empty() {
        section
    } else {
        format!("{}\n\n{}\n", existing.trim_end(), section)
    };

    if let Some(parent) = agents_md_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(agents_md_path, new_content)
}

/// Phase 3 helper: determine the lock file path for the current session.
pub fn session_lock_path(codex_home: &Path) -> PathBuf {
    codex_home.join(".omx").join("session.lock")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_config_warns_on_empty_frontier() {
        let mut config = OmxConfig::default();
        config.models.frontier = String::new();
        let warnings = validate_config(&config);
        assert!(warnings.iter().any(|w| w.contains("frontier")));
    }

    #[test]
    fn validate_config_warns_on_zero_workers() {
        let mut config = OmxConfig::default();
        config.team.default_workers = 0;
        let warnings = validate_config(&config);
        assert!(warnings.iter().any(|w| w.contains("workers")));
    }

    #[test]
    fn validate_config_no_warnings_for_defaults() {
        let config = OmxConfig::default();
        let warnings = validate_config(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn inject_agents_overlay_creates_new_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("AGENTS.md");
        inject_agents_overlay(&path, "## OMX Agents\nHello").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<!-- OMX:START -->"));
        assert!(content.contains("## OMX Agents"));
        assert!(content.contains("<!-- OMX:END -->"));
    }

    #[test]
    fn inject_agents_overlay_replaces_existing_section() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(
            &path,
            "# My Agents\n\n<!-- OMX:START -->\nold\n<!-- OMX:END -->\n\n# Custom",
        )
        .unwrap();
        inject_agents_overlay(&path, "new content").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("new content"));
        assert!(!content.contains("old"));
        assert!(content.contains("# My Agents"));
        assert!(content.contains("# Custom"));
    }

    #[test]
    fn inject_agents_overlay_appends_to_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "# Existing content\n").unwrap();
        inject_agents_overlay(&path, "injected").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Existing content"));
        assert!(content.contains("injected"));
        assert!(content.contains("<!-- OMX:START -->"));
    }

    #[test]
    fn session_lock_path_correct() {
        let path = session_lock_path(Path::new("/home/user/.codex"));
        assert_eq!(path, PathBuf::from("/home/user/.codex/.omx/session.lock"));
    }
}
