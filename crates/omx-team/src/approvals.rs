use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};

/// Status of a task approval decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

/// A task approval gate for leader review before merge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskApproval {
    pub task_id: String,
    pub required: bool,
    pub status: ApprovalStatus,
    pub reviewer: String,
    pub decision_reason: String,
    pub decided_at: Option<DateTime<Utc>>,
}

impl TaskApproval {
    /// Approve the task, recording the reason and timestamp.
    pub fn approve(&mut self, reason: &str) {
        self.status = ApprovalStatus::Approved;
        self.decision_reason = reason.to_string();
        self.decided_at = Some(Utc::now());
    }

    /// Reject the task, recording the reason and timestamp.
    pub fn reject(&mut self, reason: &str) {
        self.status = ApprovalStatus::Rejected;
        self.decision_reason = reason.to_string();
        self.decided_at = Some(Utc::now());
    }

    /// Returns true if the approval has been decided (not Pending).
    pub fn is_decided(&self) -> bool {
        self.status != ApprovalStatus::Pending
    }
}

/// Build the filesystem path where an approval JSON file is stored.
pub fn approval_path(state_dir: &Path, team_name: &str, task_id: &str) -> PathBuf {
    state_dir
        .join("team")
        .join(team_name)
        .join("approvals")
        .join(format!("{}.json", task_id))
}

/// Persist a `TaskApproval` to disk as JSON.
pub fn write_approval(
    state_dir: &Path,
    team_name: &str,
    approval: &TaskApproval,
) -> Result<(), OmxError> {
    let path = approval_path(state_dir, team_name, &approval.task_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(approval)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Read a `TaskApproval` from disk. Returns `None` if the file does not exist.
pub fn read_approval(
    state_dir: &Path,
    team_name: &str,
    task_id: &str,
) -> Result<Option<TaskApproval>, OmxError> {
    let path = approval_path(state_dir, team_name, task_id);
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)?;
    let approval: TaskApproval = serde_json::from_str(&data)?;
    Ok(Some(approval))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_approval(task_id: &str) -> TaskApproval {
        TaskApproval {
            task_id: task_id.to_string(),
            required: true,
            status: ApprovalStatus::Pending,
            reviewer: "leader-1".to_string(),
            decision_reason: String::new(),
            decided_at: None,
        }
    }

    #[test]
    fn write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mut approval = make_approval("task-42");
        approval.approve("looks good");

        write_approval(tmp.path(), "alpha", &approval).unwrap();
        let loaded = read_approval(tmp.path(), "alpha", "task-42")
            .unwrap()
            .expect("should exist");

        assert_eq!(loaded.task_id, "task-42");
        assert_eq!(loaded.status, ApprovalStatus::Approved);
        assert_eq!(loaded.decision_reason, "looks good");
        assert!(loaded.decided_at.is_some());
    }

    #[test]
    fn approve_sets_status_reason_and_timestamp() {
        let mut approval = make_approval("task-1");
        assert_eq!(approval.status, ApprovalStatus::Pending);
        assert!(!approval.is_decided());

        approval.approve("LGTM");

        assert_eq!(approval.status, ApprovalStatus::Approved);
        assert_eq!(approval.decision_reason, "LGTM");
        assert!(approval.decided_at.is_some());
        assert!(approval.is_decided());
    }

    #[test]
    fn reject_sets_status_and_reason() {
        let mut approval = make_approval("task-2");

        approval.reject("needs rework");

        assert_eq!(approval.status, ApprovalStatus::Rejected);
        assert_eq!(approval.decision_reason, "needs rework");
        assert!(approval.decided_at.is_some());
        assert!(approval.is_decided());
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let tmp = TempDir::new().unwrap();
        let result = read_approval(tmp.path(), "beta", "no-such-task").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn approval_path_format() {
        let p = approval_path(Path::new("/state"), "my-team", "t-99");
        assert_eq!(p, PathBuf::from("/state/team/my-team/approvals/t-99.json"));
    }
}
