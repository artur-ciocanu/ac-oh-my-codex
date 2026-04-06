use std::path::Path;
use std::process::Command;

use omx_types::OmxError;
use serde::{Deserialize, Serialize};

/// Per-job merge/cherry-pick/squash integration strategies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    Merge,
    CherryPick,
    Squash,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::Merge
    }
}

impl MergeStrategy {
    /// Returns the git CLI arguments for this merge strategy.
    pub fn git_args<'a>(&self, source_ref: &'a str) -> Vec<&'a str> {
        match self {
            Self::Merge => vec!["merge", source_ref],
            Self::CherryPick => vec!["cherry-pick", source_ref],
            Self::Squash => vec!["merge", "--squash", source_ref],
        }
    }
}

/// Result of executing a merge strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub strategy: MergeStrategy,
    pub source_ref: String,
    pub success: bool,
    pub conflict_files: Vec<String>,
}

/// Execute a merge using the given strategy in the specified working directory.
pub fn execute_merge(
    cwd: &Path,
    strategy: &MergeStrategy,
    source_ref: &str,
) -> Result<MergeResult, OmxError> {
    let args = strategy.git_args(source_ref);

    let output = Command::new("git")
        .args(&args)
        .current_dir(cwd)
        .output()
        .map_err(|e| OmxError::Team(format!("failed to run git: {e}")))?;

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let conflict_files = if success {
        vec![]
    } else {
        parse_conflict_files(&stderr)
    };

    Ok(MergeResult {
        strategy: strategy.clone(),
        source_ref: source_ref.to_string(),
        success,
        conflict_files,
    })
}

/// Extract conflict file paths from git stderr output.
/// Looks for lines matching "CONFLICT ... Merge conflict in <path>".
fn parse_conflict_files(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(idx) = trimmed.find("Merge conflict in ") {
                let path = trimmed[idx + "Merge conflict in ".len()..].trim();
                if !path.is_empty() {
                    return Some(path.to_string());
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_args_merge() {
        let strategy = MergeStrategy::Merge;
        assert_eq!(
            strategy.git_args("feature/foo"),
            vec!["merge", "feature/foo"]
        );
    }

    #[test]
    fn git_args_cherry_pick() {
        let strategy = MergeStrategy::CherryPick;
        assert_eq!(strategy.git_args("abc123"), vec!["cherry-pick", "abc123"]);
    }

    #[test]
    fn git_args_squash() {
        let strategy = MergeStrategy::Squash;
        assert_eq!(
            strategy.git_args("feature/bar"),
            vec!["merge", "--squash", "feature/bar"]
        );
    }

    #[test]
    fn default_is_merge() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::Merge);
    }

    #[test]
    fn merge_result_records_conflicts() {
        let result = MergeResult {
            strategy: MergeStrategy::Merge,
            source_ref: "feature/x".into(),
            success: false,
            conflict_files: vec!["src/main.rs".into(), "Cargo.toml".into()],
        };
        assert!(!result.success);
        assert_eq!(result.conflict_files.len(), 2);
        assert_eq!(result.conflict_files[0], "src/main.rs");
        assert_eq!(result.conflict_files[1], "Cargo.toml");
    }

    #[test]
    fn serde_roundtrip_strategy() {
        for strategy in [
            MergeStrategy::Merge,
            MergeStrategy::CherryPick,
            MergeStrategy::Squash,
        ] {
            let json = serde_json::to_string(&strategy).unwrap();
            let parsed: MergeStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, strategy);
        }
    }

    #[test]
    fn serde_roundtrip_merge_result() {
        let result = MergeResult {
            strategy: MergeStrategy::Squash,
            source_ref: "origin/main".into(),
            success: true,
            conflict_files: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: MergeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.strategy, MergeStrategy::Squash);
        assert_eq!(parsed.source_ref, "origin/main");
        assert!(parsed.success);
        assert!(parsed.conflict_files.is_empty());
    }

    #[test]
    fn parse_conflict_files_extracts_paths() {
        let stderr = "\
Auto-merging src/lib.rs
CONFLICT (content): Merge conflict in src/lib.rs
Auto-merging README.md
CONFLICT (content): Merge conflict in README.md
Automatic merge failed; fix conflicts and then commit the result.";
        let files = parse_conflict_files(stderr);
        assert_eq!(files, vec!["src/lib.rs", "README.md"]);
    }

    #[test]
    fn parse_conflict_files_empty_on_no_conflicts() {
        let stderr = "Already up to date.\n";
        let files = parse_conflict_files(stderr);
        assert!(files.is_empty());
    }

    #[test]
    fn parse_conflict_files_empty_on_empty_input() {
        let files = parse_conflict_files("");
        assert!(files.is_empty());
    }
}
