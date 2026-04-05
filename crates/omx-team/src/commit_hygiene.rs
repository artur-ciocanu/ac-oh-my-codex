use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRecord {
    pub sha: String,
    pub worker: String,
    pub message: String,
    pub timestamp: String,
}

/// Record a worker commit to the hygiene ledger (JSONL file).
pub fn record_commit(ledger_path: &Path, worker: &str, sha: &str, message: &str) -> Result<(), OmxError> {
    let record = CommitRecord {
        sha: sha.to_string(),
        worker: worker.to_string(),
        message: message.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut line = serde_json::to_string(&record)?;
    line.push('\n');

    // Ensure parent directory exists
    if let Some(parent) = ledger_path.parent() {
        std::fs::create_dir_all(parent).map_err(OmxError::Io)?;
    }

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path)
        .map_err(OmxError::Io)?;
    file.write_all(line.as_bytes()).map_err(OmxError::Io)?;

    tracing::info!(worker = %worker, sha = %sha, "recorded commit");
    Ok(())
}

/// Read all commit records from the ledger.
pub fn read_ledger(ledger_path: &Path) -> Result<Vec<CommitRecord>, OmxError> {
    if !ledger_path.exists() {
        return Ok(Vec::new());
    }

    let data = std::fs::read_to_string(ledger_path).map_err(OmxError::Io)?;

    let records: Vec<CommitRecord> = data
        .lines()
        .filter(|line| !line.is_empty())
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(records)
}

/// Get the ledger file path for a team.
pub fn ledger_path(state_dir: &Path, team_name: &str) -> PathBuf {
    state_dir.join(format!("team/{team_name}/commit-ledger.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_read_commit_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("ledger.jsonl");

        record_commit(&ledger, "worker-0", "abc123", "feat: add feature").unwrap();
        record_commit(&ledger, "worker-1", "def456", "fix: bug").unwrap();

        let records = read_ledger(&ledger).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sha, "abc123");
        assert_eq!(records[0].worker, "worker-0");
        assert_eq!(records[1].sha, "def456");
        assert_eq!(records[1].message, "fix: bug");
    }

    #[test]
    fn read_empty_ledger_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("nonexistent.jsonl");
        let records = read_ledger(&ledger).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn ledger_path_format() {
        let path = ledger_path(Path::new("/home/.omx/state"), "myteam");
        assert_eq!(
            path,
            PathBuf::from("/home/.omx/state/team/myteam/commit-ledger.jsonl")
        );
    }

    #[test]
    fn commit_record_serde_roundtrip() {
        let record = CommitRecord {
            sha: "abc123".into(),
            worker: "w1".into(),
            message: "test".into(),
            timestamp: "2026-04-05T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let parsed: CommitRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sha, "abc123");
    }
}
