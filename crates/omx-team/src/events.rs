use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types of events that can occur within a team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamEventType {
    WorkerSpawned,
    WorkerDied,
    WorkerReassigned,
    TaskClaimed,
    TaskCompleted,
    TaskFailed,
    TaskReleased,
    DispatchSent,
    DispatchFailed,
    ApprovalDecision,
    HeartbeatExpired,
    MergeCompleted,
    MergeConflict,
    TeamStarted,
    TeamShutdown,
}

/// A single audit-log entry for a team event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamEvent {
    pub event_id: String,
    pub team_name: String,
    pub event_type: TeamEventType,
    pub worker: String,
    pub task_id: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl TeamEvent {
    /// Create a new event with a UUID-based event_id and created_at set to now.
    pub fn new(
        team_name: impl Into<String>,
        event_type: TeamEventType,
        worker: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            team_name: team_name.into(),
            event_type,
            worker: worker.into(),
            task_id: None,
            reason: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Builder method to attach a task ID.
    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// Builder method to attach a reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Returns the canonical path for a team's event log file.
pub fn event_log_path(state_dir: &Path, team_name: &str) -> PathBuf {
    state_dir.join("team").join(team_name).join("events.jsonl")
}

/// Append a single event as a JSON line to the log file, creating parent directories as needed.
pub fn append_event(log_path: &Path, event: &TeamEvent) -> Result<(), OmxError> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(OmxError::Io)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(OmxError::Io)?;
    let line = serde_json::to_string(event).map_err(OmxError::Json)?;
    writeln!(file, "{}", line).map_err(OmxError::Io)?;
    Ok(())
}

/// Read all events from the log file. Returns an empty vec if the file does not exist.
/// Malformed lines are silently skipped.
pub fn read_events(log_path: &Path) -> Result<Vec<TeamEvent>, OmxError> {
    if !log_path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(log_path).map_err(OmxError::Io)?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(OmxError::Io)?;
        if let Ok(event) = serde_json::from_str::<TeamEvent>(&line) {
            events.push(event);
        }
    }
    Ok(events)
}

/// Read events that appear after the event with the given `after_event_id`.
/// If the cursor event ID is not found, returns all events.
pub fn read_events_after(
    log_path: &Path,
    after_event_id: &str,
) -> Result<Vec<TeamEvent>, OmxError> {
    let all = read_events(log_path)?;
    if let Some(pos) = all.iter().position(|e| e.event_id == after_event_id) {
        Ok(all.into_iter().skip(pos + 1).collect())
    } else {
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

    #[test]
    fn append_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let log = event_log_path(tmp.path(), "alpha");

        let e1 =
            TeamEvent::new("alpha", TeamEventType::WorkerSpawned, "w-1").with_task_id("task-100");
        let e2 = TeamEvent::new("alpha", TeamEventType::TaskCompleted, "w-2")
            .with_reason("all checks passed");

        append_event(&log, &e1).unwrap();
        append_event(&log, &e2).unwrap();

        let events = read_events(&log).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, e1.event_id);
        assert_eq!(events[0].event_type, TeamEventType::WorkerSpawned);
        assert_eq!(events[0].task_id.as_deref(), Some("task-100"));
        assert_eq!(events[1].event_type, TeamEventType::TaskCompleted);
        assert_eq!(events[1].reason.as_deref(), Some("all checks passed"));
    }

    #[test]
    fn read_empty_and_nonexistent_log() {
        let tmp = TempDir::new().unwrap();
        // nonexistent file
        let log = tmp.path().join("no-such-file.jsonl");
        assert!(read_events(&log).unwrap().is_empty());

        // empty file
        let log2 = tmp.path().join("empty.jsonl");
        fs::File::create(&log2).unwrap();
        assert!(read_events(&log2).unwrap().is_empty());
    }

    #[test]
    fn read_events_after_cursor() {
        let tmp = TempDir::new().unwrap();
        let log = event_log_path(tmp.path(), "beta");

        let e1 = TeamEvent::new("beta", TeamEventType::TeamStarted, "leader");
        let e2 = TeamEvent::new("beta", TeamEventType::WorkerSpawned, "w-1");
        let e3 = TeamEvent::new("beta", TeamEventType::TaskClaimed, "w-1");
        let cursor_id = e1.event_id.clone();

        append_event(&log, &e1).unwrap();
        append_event(&log, &e2).unwrap();
        append_event(&log, &e3).unwrap();

        let after = read_events_after(&log, &cursor_id).unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].event_id, e2.event_id);
        assert_eq!(after[1].event_id, e3.event_id);
    }

    #[test]
    fn filter_events_by_worker() {
        let tmp = TempDir::new().unwrap();
        let log = event_log_path(tmp.path(), "gamma");

        let e1 = TeamEvent::new("gamma", TeamEventType::TaskClaimed, "w-1");
        let e2 = TeamEvent::new("gamma", TeamEventType::TaskCompleted, "w-2");
        let e3 = TeamEvent::new("gamma", TeamEventType::TaskFailed, "w-1");

        for e in [&e1, &e2, &e3] {
            append_event(&log, e).unwrap();
        }

        let events = read_events(&log).unwrap();
        let w1_events: Vec<_> = events.iter().filter(|e| e.worker == "w-1").collect();
        assert_eq!(w1_events.len(), 2);
        assert_eq!(w1_events[0].event_type, TeamEventType::TaskClaimed);
        assert_eq!(w1_events[1].event_type, TeamEventType::TaskFailed);
    }

    #[test]
    fn event_ids_are_unique() {
        let mut ids = HashSet::new();
        for _ in 0..100 {
            let e = TeamEvent::new("team", TeamEventType::WorkerSpawned, "w");
            assert!(ids.insert(e.event_id), "duplicate event_id generated");
        }
    }

    #[test]
    fn event_log_path_format() {
        let base = Path::new("/tmp/state");
        let p = event_log_path(base, "my-team");
        assert_eq!(p, PathBuf::from("/tmp/state/team/my-team/events.jsonl"));
    }
}
