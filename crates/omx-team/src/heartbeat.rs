use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};

/// Tracks liveness of a single worker via periodic heartbeats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeat {
    pub worker_name: String,
    pub pid: u32,
    pub last_beat_at: DateTime<Utc>,
    pub turn_count: u64,
    pub alive: bool,
}

impl WorkerHeartbeat {
    /// Create a fresh heartbeat for the given worker.
    pub fn new(worker_name: impl Into<String>, pid: u32) -> Self {
        Self {
            worker_name: worker_name.into(),
            pid,
            last_beat_at: Utc::now(),
            turn_count: 0,
            alive: true,
        }
    }

    /// Update the heartbeat timestamp to now.
    pub fn beat(&mut self) {
        self.last_beat_at = Utc::now();
    }

    /// Increment the turn counter and refresh the heartbeat.
    pub fn bump_turn(&mut self) {
        self.turn_count += 1;
        self.beat();
    }

    /// Returns `true` if the heartbeat is older than `max_age`.
    pub fn is_stale(&self, max_age: Duration) -> bool {
        let age = Utc::now() - self.last_beat_at;
        age > max_age
    }
}

/// Write a heartbeat to `{heartbeat_dir}/{worker_name}.heartbeat.json`.
pub fn write_heartbeat(heartbeat_dir: &Path, hb: &WorkerHeartbeat) -> Result<(), OmxError> {
    std::fs::create_dir_all(heartbeat_dir)?;
    let path = heartbeat_dir.join(format!("{}.heartbeat.json", hb.worker_name));
    let json = serde_json::to_string_pretty(hb)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Read a heartbeat file for the named worker. Returns `None` if the file does not exist.
pub fn read_heartbeat(
    heartbeat_dir: &Path,
    worker_name: &str,
) -> Result<Option<WorkerHeartbeat>, OmxError> {
    let path = heartbeat_dir.join(format!("{worker_name}.heartbeat.json"));
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)?;
    let hb: WorkerHeartbeat = serde_json::from_str(&data)?;
    Ok(Some(hb))
}

/// Scan `heartbeat_dir` for all heartbeat files and return those that are stale.
pub fn find_stale_workers(
    heartbeat_dir: &Path,
    max_age: Duration,
) -> Result<Vec<WorkerHeartbeat>, OmxError> {
    let mut stale = Vec::new();
    if !heartbeat_dir.exists() {
        return Ok(stale);
    }
    for entry in std::fs::read_dir(heartbeat_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".heartbeat.json"))
        {
            let data = std::fs::read_to_string(&path)?;
            let hb: WorkerHeartbeat = serde_json::from_str(&data)?;
            if hb.is_stale(max_age) {
                stale.push(hb);
            }
        }
    }
    Ok(stale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let hb = WorkerHeartbeat::new("alpha", 1234);
        write_heartbeat(dir.path(), &hb).unwrap();

        let loaded = read_heartbeat(dir.path(), "alpha").unwrap().unwrap();
        assert_eq!(loaded.worker_name, "alpha");
        assert_eq!(loaded.pid, 1234);
        assert_eq!(loaded.turn_count, 0);
        assert!(loaded.alive);
    }

    #[test]
    fn bump_turn_increments_count() {
        let mut hb = WorkerHeartbeat::new("beta", 42);
        assert_eq!(hb.turn_count, 0);
        hb.bump_turn();
        assert_eq!(hb.turn_count, 1);
        hb.bump_turn();
        assert_eq!(hb.turn_count, 2);
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let dir = TempDir::new().unwrap();
        let result = read_heartbeat(dir.path(), "ghost").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn detect_stale_heartbeat() {
        let mut hb = WorkerHeartbeat::new("stale-worker", 99);
        // Backdate the heartbeat by 10 seconds
        hb.last_beat_at = Utc::now() - Duration::seconds(10);
        assert!(hb.is_stale(Duration::seconds(5)));
        assert!(!hb.is_stale(Duration::seconds(30)));
    }

    #[test]
    fn find_stale_workers_filters_correctly() {
        let dir = TempDir::new().unwrap();

        let fresh = WorkerHeartbeat::new("fresh", 1);
        write_heartbeat(dir.path(), &fresh).unwrap();

        let mut old = WorkerHeartbeat::new("old", 2);
        old.last_beat_at = Utc::now() - Duration::seconds(60);
        write_heartbeat(dir.path(), &old).unwrap();

        let mut ancient = WorkerHeartbeat::new("ancient", 3);
        ancient.last_beat_at = Utc::now() - Duration::seconds(120);
        write_heartbeat(dir.path(), &ancient).unwrap();

        let stale = find_stale_workers(dir.path(), Duration::seconds(30)).unwrap();
        assert_eq!(stale.len(), 2);
        let names: Vec<&str> = stale.iter().map(|h| h.worker_name.as_str()).collect();
        assert!(names.contains(&"old"));
        assert!(names.contains(&"ancient"));
    }

    #[test]
    fn serde_roundtrip() {
        let hb = WorkerHeartbeat::new("serde-test", 555);
        let json = serde_json::to_string(&hb).unwrap();
        let parsed: WorkerHeartbeat = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.worker_name, "serde-test");
        assert_eq!(parsed.pid, 555);
        assert_eq!(parsed.turn_count, 0);
        assert!(parsed.alive);
    }
}
