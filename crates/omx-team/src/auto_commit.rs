use std::path::Path;
use std::process::Command;
use std::time::Duration;

use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};

/// Custom serde module to serialize Duration as u64 seconds.
mod duration_secs {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Configuration for automatic WIP commits by workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCommitConfig {
    #[serde(with = "duration_secs")]
    pub interval: Duration,
    pub enabled: bool,
    pub message_prefix: String,
}

impl Default for AutoCommitConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300),
            enabled: true,
            message_prefix: "wip".to_string(),
        }
    }
}

/// Determines whether an auto-commit should happen based on config and last commit time.
pub fn should_auto_commit(
    config: &AutoCommitConfig,
    last_commit_at: Option<DateTime<Utc>>,
) -> bool {
    if !config.enabled {
        return false;
    }

    match last_commit_at {
        None => true,
        Some(last) => {
            let elapsed = Utc::now().signed_duration_since(last);
            elapsed >= chrono::Duration::from_std(config.interval).unwrap_or(chrono::Duration::MAX)
        }
    }
}

/// Formats an auto-commit message.
pub fn auto_commit_message(prefix: &str, worker_name: &str, task_id: &str) -> String {
    format!("{prefix}: {worker_name} {task_id}")
}

/// Runs an auto-commit in the given working directory.
///
/// Stages all changes (`git add -A`), checks if there are staged changes,
/// and commits them. Returns the commit SHA if a commit was made, or `None`
/// if there was nothing to commit.
pub fn run_auto_commit(cwd: &Path, message: &str) -> Result<Option<String>, OmxError> {
    // git add -A
    let add_status = Command::new("git")
        .args(["add", "-A"])
        .current_dir(cwd)
        .output()
        .map_err(|e| OmxError::Team(format!("git add failed: {e}")))?;

    if !add_status.status.success() {
        let stderr = String::from_utf8_lossy(&add_status.stderr);
        return Err(OmxError::Team(format!("git add -A failed: {stderr}")));
    }

    // git diff --cached --quiet  (exit 0 = nothing staged, exit 1 = has changes)
    let diff_status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(cwd)
        .status()
        .map_err(|e| OmxError::Team(format!("git diff failed: {e}")))?;

    if diff_status.success() {
        // Nothing to commit
        return Ok(None);
    }

    // git commit
    let commit_output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(cwd)
        .output()
        .map_err(|e| OmxError::Team(format!("git commit failed: {e}")))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        return Err(OmxError::Team(format!("git commit failed: {stderr}")));
    }

    // git rev-parse HEAD
    let rev_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(cwd)
        .output()
        .map_err(|e| OmxError::Team(format!("git rev-parse failed: {e}")))?;

    let sha = String::from_utf8_lossy(&rev_output.stdout)
        .trim()
        .to_string();
    Ok(Some(sha))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn config_defaults() {
        let config = AutoCommitConfig::default();
        assert_eq!(config.interval, Duration::from_secs(300));
        assert!(config.enabled);
        assert_eq!(config.message_prefix, "wip");
    }

    #[test]
    fn should_commit_when_interval_elapsed() {
        let config = AutoCommitConfig {
            interval: Duration::from_secs(60),
            enabled: true,
            message_prefix: "wip".into(),
        };
        let past = Utc::now() - chrono::Duration::seconds(120);
        assert!(should_auto_commit(&config, Some(past)));
    }

    #[test]
    fn should_not_commit_when_recent() {
        let config = AutoCommitConfig {
            interval: Duration::from_secs(300),
            enabled: true,
            message_prefix: "wip".into(),
        };
        let recent = Utc::now() - chrono::Duration::seconds(10);
        assert!(!should_auto_commit(&config, Some(recent)));
    }

    #[test]
    fn should_commit_when_never_committed() {
        let config = AutoCommitConfig::default();
        assert!(should_auto_commit(&config, None));
    }

    #[test]
    fn should_not_commit_when_disabled() {
        let config = AutoCommitConfig {
            interval: Duration::from_secs(60),
            enabled: false,
            message_prefix: "wip".into(),
        };
        assert!(!should_auto_commit(&config, None));
    }

    #[test]
    fn format_auto_commit_message() {
        let msg = auto_commit_message("wip", "worker-1", "task-42");
        assert_eq!(msg, "wip: worker-1 task-42");
    }

    #[test]
    fn serde_roundtrip() {
        let config = AutoCommitConfig {
            interval: Duration::from_secs(120),
            enabled: false,
            message_prefix: "auto".into(),
        };
        let json = serde_json::to_string(&config).unwrap();
        // Verify interval is serialized as a plain u64
        assert!(json.contains("\"interval\":120"));
        let deserialized: AutoCommitConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.interval, Duration::from_secs(120));
        assert!(!deserialized.enabled);
        assert_eq!(deserialized.message_prefix, "auto");
    }
}
