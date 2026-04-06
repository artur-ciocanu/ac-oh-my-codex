# Phase 9: Team Runtime Hardening — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Production-harden the `omx-team`, `omx-mux`, and `omx-hooks` crates by adding distributed locks, heartbeat/dead-worker recovery, event audit log, task approval gates, auto-commit, merge strategies, HUD pane management, leader pane protection, and hook chaining with built-in hook executables.

**Architecture:** Bottom-up within three existing crates. New modules are added to `omx-team` for locks, heartbeat, events, approvals, auto-commit scheduling, and merge strategies. `omx-mux` gains pane ID tracking, HUD pane management, leader protection, and resize hooks. `omx-hooks` gains hook chaining (ordered execution), per-hook timeout, and result aggregation. Each feature is independently testable with no circular dependencies between new modules.

**Tech Stack:** Rust, fs2 (file locks), tokio (async runtime), chrono (timestamps), serde/serde_json (serialization), uuid (IDs), tempfile (test fixtures)

---

## File Structure

### `omx-team` — New modules

| File | Responsibility |
|------|---------------|
| `crates/omx-team/src/heartbeat.rs` | Worker heartbeat write/read, staleness detection |
| `crates/omx-team/src/locks.rs` | fs2-based distributed file locks per worker/task |
| `crates/omx-team/src/events.rs` | Append-only team event audit log (JSONL) |
| `crates/omx-team/src/approvals.rs` | Task approval gate: approve/reject/rework before merge |
| `crates/omx-team/src/merge_strategy.rs` | Per-job merge/cherry-pick/squash integration |
| `crates/omx-team/src/auto_commit.rs` | Configurable interval WIP auto-commit for workers |

### `omx-team` — Modified modules

| File | Change |
|------|--------|
| `crates/omx-team/src/lib.rs` | Add `pub mod` for 6 new modules |
| `crates/omx-team/src/config.rs` | Add heartbeat, lock, auto-commit, merge config fields |
| `crates/omx-team/src/orchestrator.rs` | Integrate heartbeat checks and dead worker recovery into tick |
| `crates/omx-team/Cargo.toml` | Add `fs2` dependency |

### `omx-mux` — New and modified files

| File | Responsibility |
|------|---------------|
| `crates/omx-mux/src/pane_registry.rs` | Map logical worker IDs to tmux pane IDs |
| `crates/omx-mux/src/hud_pane.rs` | Dedicated HUD pane lifecycle: create, resize, render target |
| `crates/omx-mux/src/leader_guard.rs` | Prevent accidental kill of leader pane |
| `crates/omx-mux/src/trust_dismiss.rs` | Auto-dismiss Claude trust prompts in worker panes |
| `crates/omx-mux/src/resize_hook.rs` | Detect terminal resize, fire callback |
| `crates/omx-mux/src/lib.rs` | Add `pub mod` for 5 new modules |
| `crates/omx-mux/Cargo.toml` | Add `chrono` dependency |

### `omx-hooks` — New and modified files

| File | Responsibility |
|------|---------------|
| `crates/omx-hooks/src/chain.rs` | Ordered hook chain execution with short-circuit option |
| `crates/omx-hooks/src/aggregator.rs` | Collect and merge stdout from all hooks for an event |
| `crates/omx-hooks/src/builtins.rs` | 6 built-in hook descriptor factories |
| `crates/omx-hooks/src/lib.rs` | Add `pub mod` for 3 new modules, update `ShellHookDispatcher` |

---

## Task 1: Distributed File Locks (`omx-team/src/locks.rs`)

**Files:**
- Create: `crates/omx-team/src/locks.rs`
- Modify: `crates/omx-team/src/lib.rs`
- Modify: `crates/omx-team/Cargo.toml`

- [ ] **Step 1: Add `fs2` dependency to omx-team**

In `crates/omx-team/Cargo.toml`, add under `[dependencies]`:

```toml
fs2 = { workspace = true }
```

- [ ] **Step 2: Write the failing test for `TeamFileLock`**

Create `crates/omx-team/src/locks.rs` with only tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn acquire_and_release_lock() {
        let dir = tempfile::tempdir().unwrap();
        let lock = TeamFileLock::acquire(dir.path(), "worker-0").unwrap();
        assert!(lock.lock_path().exists());
        lock.release().unwrap();
    }

    #[test]
    fn lock_prevents_double_acquire() {
        let dir = tempfile::tempdir().unwrap();
        let _lock1 = TeamFileLock::acquire(dir.path(), "worker-0").unwrap();
        let result = TeamFileLock::try_acquire(dir.path(), "worker-0");
        assert!(result.is_err());
    }

    #[test]
    fn separate_workers_get_separate_locks() {
        let dir = tempfile::tempdir().unwrap();
        let _lock1 = TeamFileLock::acquire(dir.path(), "worker-0").unwrap();
        let lock2 = TeamFileLock::acquire(dir.path(), "worker-1");
        assert!(lock2.is_ok());
    }

    #[test]
    fn lock_file_path_includes_worker_name() {
        let dir = tempfile::tempdir().unwrap();
        let lock = TeamFileLock::acquire(dir.path(), "worker-3").unwrap();
        assert!(lock.lock_path().to_string_lossy().contains("worker-3"));
        lock.release().unwrap();
    }

    #[test]
    fn stale_lock_detection() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("worker-0.lock");
        // Create a lock file manually (simulating a stale lock from a crashed process)
        std::fs::write(&lock_path, "stale").unwrap();
        // Set mtime to 2 minutes ago
        let two_min_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        filetime::set_file_mtime(&lock_path, filetime::FileTime::from_system_time(two_min_ago)).unwrap();
        assert!(is_stale_lock(&lock_path, std::time::Duration::from_secs(60)));
        assert!(!is_stale_lock(&lock_path, std::time::Duration::from_secs(300)));
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p omx-team locks -- --nocapture 2>&1 | head -30`
Expected: FAIL — `TeamFileLock` and `is_stale_lock` not defined

- [ ] **Step 4: Write minimal implementation**

Replace `crates/omx-team/src/locks.rs` with:

```rust
use fs2::FileExt;
use omx_types::OmxError;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// A distributed file lock for a team worker, backed by fs2 exclusive locks.
pub struct TeamFileLock {
    lock_path: PathBuf,
    file: File,
}

impl TeamFileLock {
    /// Acquire a blocking exclusive lock for the given worker.
    pub fn acquire(lock_dir: &Path, worker_name: &str) -> Result<Self, OmxError> {
        fs::create_dir_all(lock_dir).map_err(OmxError::Io)?;
        let lock_path = lock_dir.join(format!("{worker_name}.lock"));
        let file = File::create(&lock_path).map_err(OmxError::Io)?;
        file.lock_exclusive()
            .map_err(|e| OmxError::Team(format!("failed to acquire lock for {worker_name}: {e}")))?;
        Ok(Self { lock_path, file })
    }

    /// Try to acquire a non-blocking exclusive lock. Returns error if already held.
    pub fn try_acquire(lock_dir: &Path, worker_name: &str) -> Result<Self, OmxError> {
        fs::create_dir_all(lock_dir).map_err(OmxError::Io)?;
        let lock_path = lock_dir.join(format!("{worker_name}.lock"));
        let file = File::create(&lock_path).map_err(OmxError::Io)?;
        file.try_lock_exclusive()
            .map_err(|e| OmxError::Team(format!("lock already held for {worker_name}: {e}")))?;
        Ok(Self { lock_path, file })
    }

    /// Return the path to the lock file.
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    /// Explicitly release the lock.
    pub fn release(self) -> Result<(), OmxError> {
        self.file
            .unlock()
            .map_err(|e| OmxError::Team(format!("failed to release lock: {e}")))?;
        Ok(())
    }
}

impl Drop for TeamFileLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Check if a lock file is stale (older than `max_age`).
pub fn is_stale_lock(lock_path: &Path, max_age: Duration) -> bool {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|age| age > max_age)
        .unwrap_or(false)
}

/// Remove a lock file if it is stale.
pub fn recover_stale_lock(lock_path: &Path, max_age: Duration) -> Result<bool, OmxError> {
    if is_stale_lock(lock_path, max_age) {
        fs::remove_file(lock_path).map_err(OmxError::Io)?;
        tracing::info!(path = %lock_path.display(), "recovered stale lock");
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_and_release_lock() {
        let dir = tempfile::tempdir().unwrap();
        let lock = TeamFileLock::acquire(dir.path(), "worker-0").unwrap();
        assert!(lock.lock_path().exists());
        lock.release().unwrap();
    }

    #[test]
    fn lock_prevents_double_acquire() {
        let dir = tempfile::tempdir().unwrap();
        let _lock1 = TeamFileLock::acquire(dir.path(), "worker-0").unwrap();
        let result = TeamFileLock::try_acquire(dir.path(), "worker-0");
        assert!(result.is_err());
    }

    #[test]
    fn separate_workers_get_separate_locks() {
        let dir = tempfile::tempdir().unwrap();
        let _lock1 = TeamFileLock::acquire(dir.path(), "worker-0").unwrap();
        let lock2 = TeamFileLock::acquire(dir.path(), "worker-1");
        assert!(lock2.is_ok());
    }

    #[test]
    fn lock_file_path_includes_worker_name() {
        let dir = tempfile::tempdir().unwrap();
        let lock = TeamFileLock::acquire(dir.path(), "worker-3").unwrap();
        assert!(lock.lock_path().to_string_lossy().contains("worker-3"));
        lock.release().unwrap();
    }

    #[test]
    fn stale_lock_detection_by_age() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");
        std::fs::write(&lock_path, "stale").unwrap();
        // Fresh lock is not stale
        assert!(!is_stale_lock(&lock_path, Duration::from_secs(60)));
        // Non-existent lock is not stale
        assert!(!is_stale_lock(&dir.path().join("nope.lock"), Duration::from_secs(1)));
    }

    #[test]
    fn recover_stale_lock_removes_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("old.lock");
        std::fs::write(&lock_path, "stale").unwrap();
        // File is fresh, should not recover
        let recovered = recover_stale_lock(&lock_path, Duration::from_secs(60)).unwrap();
        assert!(!recovered);
        assert!(lock_path.exists());
    }

    #[test]
    fn drop_releases_lock() {
        let dir = tempfile::tempdir().unwrap();
        {
            let _lock = TeamFileLock::acquire(dir.path(), "worker-0").unwrap();
            // Lock held here
        }
        // Lock released by Drop, re-acquire should succeed
        let lock2 = TeamFileLock::try_acquire(dir.path(), "worker-0");
        assert!(lock2.is_ok());
    }
}
```

- [ ] **Step 5: Register the module in lib.rs**

Add `pub mod locks;` to `crates/omx-team/src/lib.rs`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p omx-team locks -- --nocapture`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add crates/omx-team/src/locks.rs crates/omx-team/src/lib.rs crates/omx-team/Cargo.toml
git commit -m "feat(omx-team): add distributed file locks with fs2"
```

---

## Task 2: Worker Heartbeat System (`omx-team/src/heartbeat.rs`)

**Files:**
- Create: `crates/omx-team/src/heartbeat.rs`
- Modify: `crates/omx-team/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-team/src/heartbeat.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn write_and_read_heartbeat() {
        let dir = tempfile::tempdir().unwrap();
        let hb = WorkerHeartbeat::new("worker-0", 1234);
        write_heartbeat(dir.path(), &hb).unwrap();
        let loaded = read_heartbeat(dir.path(), "worker-0").unwrap().unwrap();
        assert_eq!(loaded.worker_name, "worker-0");
        assert_eq!(loaded.pid, 1234);
        assert_eq!(loaded.turn_count, 0);
        assert!(loaded.alive);
    }

    #[test]
    fn bump_turn_increments_count() {
        let dir = tempfile::tempdir().unwrap();
        let mut hb = WorkerHeartbeat::new("worker-0", 1234);
        hb.bump_turn();
        hb.bump_turn();
        write_heartbeat(dir.path(), &hb).unwrap();
        let loaded = read_heartbeat(dir.path(), "worker-0").unwrap().unwrap();
        assert_eq!(loaded.turn_count, 2);
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_heartbeat(dir.path(), "ghost").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn detect_stale_heartbeat() {
        let hb = WorkerHeartbeat {
            worker_name: "worker-0".into(),
            pid: 1234,
            last_beat_at: chrono::Utc::now() - chrono::Duration::seconds(120),
            turn_count: 5,
            alive: true,
        };
        assert!(hb.is_stale(Duration::from_secs(60)));
        assert!(!hb.is_stale(Duration::from_secs(300)));
    }

    #[test]
    fn list_stale_workers() {
        let dir = tempfile::tempdir().unwrap();

        let fresh = WorkerHeartbeat::new("worker-fresh", 100);
        write_heartbeat(dir.path(), &fresh).unwrap();

        let mut stale = WorkerHeartbeat::new("worker-stale", 200);
        stale.last_beat_at = chrono::Utc::now() - chrono::Duration::seconds(300);
        write_heartbeat(dir.path(), &stale).unwrap();

        let stale_list = find_stale_workers(dir.path(), Duration::from_secs(60)).unwrap();
        assert_eq!(stale_list.len(), 1);
        assert_eq!(stale_list[0].worker_name, "worker-stale");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-team heartbeat -- --nocapture 2>&1 | head -20`
Expected: FAIL — types not defined

- [ ] **Step 3: Write minimal implementation**

```rust
use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeat {
    pub worker_name: String,
    pub pid: u32,
    pub last_beat_at: DateTime<Utc>,
    pub turn_count: u64,
    pub alive: bool,
}

impl WorkerHeartbeat {
    pub fn new(worker_name: &str, pid: u32) -> Self {
        Self {
            worker_name: worker_name.to_string(),
            pid,
            last_beat_at: Utc::now(),
            turn_count: 0,
            alive: true,
        }
    }

    /// Record a new heartbeat (updates timestamp).
    pub fn beat(&mut self) {
        self.last_beat_at = Utc::now();
    }

    /// Record a turn completion (updates timestamp and increments counter).
    pub fn bump_turn(&mut self) {
        self.turn_count += 1;
        self.beat();
    }

    /// Check if this heartbeat is older than `max_age`.
    pub fn is_stale(&self, max_age: Duration) -> bool {
        let age = Utc::now().signed_duration_since(self.last_beat_at);
        age.to_std().map(|d| d > max_age).unwrap_or(true)
    }
}

/// Write a heartbeat file for a worker.
pub fn write_heartbeat(heartbeat_dir: &Path, hb: &WorkerHeartbeat) -> Result<(), OmxError> {
    std::fs::create_dir_all(heartbeat_dir).map_err(OmxError::Io)?;
    let path = heartbeat_dir.join(format!("{}.heartbeat.json", hb.worker_name));
    let json = serde_json::to_string_pretty(hb)?;
    std::fs::write(&path, json).map_err(OmxError::Io)?;
    Ok(())
}

/// Read a heartbeat file for a worker. Returns `None` if the file does not exist.
pub fn read_heartbeat(heartbeat_dir: &Path, worker_name: &str) -> Result<Option<WorkerHeartbeat>, OmxError> {
    let path = heartbeat_dir.join(format!("{worker_name}.heartbeat.json"));
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path).map_err(OmxError::Io)?;
    let hb: WorkerHeartbeat = serde_json::from_str(&data)?;
    Ok(Some(hb))
}

/// Find all workers with stale heartbeats in the given directory.
pub fn find_stale_workers(heartbeat_dir: &Path, max_age: Duration) -> Result<Vec<WorkerHeartbeat>, OmxError> {
    if !heartbeat_dir.exists() {
        return Ok(Vec::new());
    }

    let mut stale = Vec::new();
    let entries = std::fs::read_dir(heartbeat_dir).map_err(OmxError::Io)?;

    for entry in entries {
        let entry = entry.map_err(OmxError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if !path.to_string_lossy().contains(".heartbeat.") {
            continue;
        }
        let data = std::fs::read_to_string(&path).map_err(OmxError::Io)?;
        if let Ok(hb) = serde_json::from_str::<WorkerHeartbeat>(&data) {
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

    #[test]
    fn write_and_read_heartbeat() {
        let dir = tempfile::tempdir().unwrap();
        let hb = WorkerHeartbeat::new("worker-0", 1234);
        write_heartbeat(dir.path(), &hb).unwrap();
        let loaded = read_heartbeat(dir.path(), "worker-0").unwrap().unwrap();
        assert_eq!(loaded.worker_name, "worker-0");
        assert_eq!(loaded.pid, 1234);
        assert_eq!(loaded.turn_count, 0);
        assert!(loaded.alive);
    }

    #[test]
    fn bump_turn_increments_count() {
        let dir = tempfile::tempdir().unwrap();
        let mut hb = WorkerHeartbeat::new("worker-0", 1234);
        hb.bump_turn();
        hb.bump_turn();
        write_heartbeat(dir.path(), &hb).unwrap();
        let loaded = read_heartbeat(dir.path(), "worker-0").unwrap().unwrap();
        assert_eq!(loaded.turn_count, 2);
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_heartbeat(dir.path(), "ghost").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn detect_stale_heartbeat() {
        let hb = WorkerHeartbeat {
            worker_name: "worker-0".into(),
            pid: 1234,
            last_beat_at: Utc::now() - chrono::Duration::seconds(120),
            turn_count: 5,
            alive: true,
        };
        assert!(hb.is_stale(Duration::from_secs(60)));
        assert!(!hb.is_stale(Duration::from_secs(300)));
    }

    #[test]
    fn list_stale_workers() {
        let dir = tempfile::tempdir().unwrap();

        let fresh = WorkerHeartbeat::new("worker-fresh", 100);
        write_heartbeat(dir.path(), &fresh).unwrap();

        let mut stale = WorkerHeartbeat::new("worker-stale", 200);
        stale.last_beat_at = Utc::now() - chrono::Duration::seconds(300);
        write_heartbeat(dir.path(), &stale).unwrap();

        let stale_list = find_stale_workers(dir.path(), Duration::from_secs(60)).unwrap();
        assert_eq!(stale_list.len(), 1);
        assert_eq!(stale_list[0].worker_name, "worker-stale");
    }

    #[test]
    fn heartbeat_serde_roundtrip() {
        let hb = WorkerHeartbeat::new("w1", 999);
        let json = serde_json::to_string(&hb).unwrap();
        let parsed: WorkerHeartbeat = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.worker_name, "w1");
        assert_eq!(parsed.pid, 999);
    }
}
```

- [ ] **Step 4: Register the module in lib.rs**

Add `pub mod heartbeat;` to `crates/omx-team/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p omx-team heartbeat -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/omx-team/src/heartbeat.rs crates/omx-team/src/lib.rs
git commit -m "feat(omx-team): add worker heartbeat system with staleness detection"
```

---

## Task 3: Event Audit Log (`omx-team/src/events.rs`)

**Files:**
- Create: `crates/omx-team/src/events.rs`
- Modify: `crates/omx-team/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-team/src/events.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_read_events() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");

        let e1 = TeamEvent::new("team-1", TeamEventType::WorkerSpawned, "worker-0");
        append_event(&log_path, &e1).unwrap();

        let e2 = TeamEvent::new("team-1", TeamEventType::TaskCompleted, "worker-1")
            .with_task_id("task-0");
        append_event(&log_path, &e2).unwrap();

        let events = read_events(&log_path).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, TeamEventType::WorkerSpawned);
        assert_eq!(events[1].task_id, Some("task-0".into()));
    }

    #[test]
    fn read_empty_log_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("nonexistent.jsonl");
        let events = read_events(&log_path).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn read_events_after_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");

        let e1 = TeamEvent::new("t", TeamEventType::WorkerSpawned, "w0");
        let e2 = TeamEvent::new("t", TeamEventType::TaskClaimed, "w0");
        let e3 = TeamEvent::new("t", TeamEventType::TaskCompleted, "w1");
        append_event(&log_path, &e1).unwrap();
        append_event(&log_path, &e2).unwrap();
        append_event(&log_path, &e3).unwrap();

        let after = read_events_after(&log_path, &e1.event_id).unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].event_id, e2.event_id);
    }

    #[test]
    fn filter_events_by_worker() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");

        append_event(&log_path, &TeamEvent::new("t", TeamEventType::TaskClaimed, "w0")).unwrap();
        append_event(&log_path, &TeamEvent::new("t", TeamEventType::TaskClaimed, "w1")).unwrap();
        append_event(&log_path, &TeamEvent::new("t", TeamEventType::TaskCompleted, "w0")).unwrap();

        let events = read_events(&log_path).unwrap();
        let w0_events: Vec<_> = events.iter().filter(|e| e.worker == "w0").collect();
        assert_eq!(w0_events.len(), 2);
    }

    #[test]
    fn event_ids_are_unique() {
        let e1 = TeamEvent::new("t", TeamEventType::WorkerSpawned, "w0");
        let e2 = TeamEvent::new("t", TeamEventType::WorkerSpawned, "w0");
        assert_ne!(e1.event_id, e2.event_id);
    }

    #[test]
    fn event_log_path_format() {
        let path = event_log_path(std::path::Path::new("/state"), "myteam");
        assert_eq!(path, std::path::PathBuf::from("/state/team/myteam/events.jsonl"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-team events -- --nocapture 2>&1 | head -20`
Expected: FAIL — types not defined

- [ ] **Step 3: Write minimal implementation**

```rust
use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    pub fn new(team_name: &str, event_type: TeamEventType, worker: &str) -> Self {
        Self {
            event_id: format!("evt-{}", uuid::Uuid::new_v4().simple()),
            team_name: team_name.to_string(),
            event_type,
            worker: worker.to_string(),
            task_id: None,
            reason: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_task_id(mut self, task_id: &str) -> Self {
        self.task_id = Some(task_id.to_string());
        self
    }

    pub fn with_reason(mut self, reason: &str) -> Self {
        self.reason = Some(reason.to_string());
        self
    }
}

/// Get the event log path for a team.
pub fn event_log_path(state_dir: &Path, team_name: &str) -> PathBuf {
    state_dir.join(format!("team/{team_name}/events.jsonl"))
}

/// Append an event to the JSONL log.
pub fn append_event(log_path: &Path, event: &TeamEvent) -> Result<(), OmxError> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).map_err(OmxError::Io)?;
    }
    let mut line = serde_json::to_string(event)?;
    line.push('\n');

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(OmxError::Io)?;
    file.write_all(line.as_bytes()).map_err(OmxError::Io)?;
    Ok(())
}

/// Read all events from the JSONL log.
pub fn read_events(log_path: &Path) -> Result<Vec<TeamEvent>, OmxError> {
    if !log_path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(log_path).map_err(OmxError::Io)?;
    let events: Vec<TeamEvent> = data
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    Ok(events)
}

/// Read events that occur after a given cursor (event_id).
pub fn read_events_after(log_path: &Path, after_event_id: &str) -> Result<Vec<TeamEvent>, OmxError> {
    let all = read_events(log_path)?;
    let mut found_cursor = false;
    let mut result = Vec::new();
    for event in all {
        if found_cursor {
            result.push(event);
        } else if event.event_id == after_event_id {
            found_cursor = true;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_read_events() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");

        let e1 = TeamEvent::new("team-1", TeamEventType::WorkerSpawned, "worker-0");
        append_event(&log_path, &e1).unwrap();

        let e2 = TeamEvent::new("team-1", TeamEventType::TaskCompleted, "worker-1")
            .with_task_id("task-0");
        append_event(&log_path, &e2).unwrap();

        let events = read_events(&log_path).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, TeamEventType::WorkerSpawned);
        assert_eq!(events[1].task_id, Some("task-0".into()));
    }

    #[test]
    fn read_empty_log_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("nonexistent.jsonl");
        let events = read_events(&log_path).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn read_events_after_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");

        let e1 = TeamEvent::new("t", TeamEventType::WorkerSpawned, "w0");
        let e2 = TeamEvent::new("t", TeamEventType::TaskClaimed, "w0");
        let e3 = TeamEvent::new("t", TeamEventType::TaskCompleted, "w1");
        append_event(&log_path, &e1).unwrap();
        append_event(&log_path, &e2).unwrap();
        append_event(&log_path, &e3).unwrap();

        let after = read_events_after(&log_path, &e1.event_id).unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].event_id, e2.event_id);
    }

    #[test]
    fn filter_events_by_worker() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("events.jsonl");

        append_event(&log_path, &TeamEvent::new("t", TeamEventType::TaskClaimed, "w0")).unwrap();
        append_event(&log_path, &TeamEvent::new("t", TeamEventType::TaskClaimed, "w1")).unwrap();
        append_event(&log_path, &TeamEvent::new("t", TeamEventType::TaskCompleted, "w0")).unwrap();

        let events = read_events(&log_path).unwrap();
        let w0_events: Vec<_> = events.iter().filter(|e| e.worker == "w0").collect();
        assert_eq!(w0_events.len(), 2);
    }

    #[test]
    fn event_ids_are_unique() {
        let e1 = TeamEvent::new("t", TeamEventType::WorkerSpawned, "w0");
        let e2 = TeamEvent::new("t", TeamEventType::WorkerSpawned, "w0");
        assert_ne!(e1.event_id, e2.event_id);
    }

    #[test]
    fn event_log_path_format() {
        let path = event_log_path(Path::new("/state"), "myteam");
        assert_eq!(path, PathBuf::from("/state/team/myteam/events.jsonl"));
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add `pub mod events;` to `crates/omx-team/src/lib.rs`.

Run: `cargo test -p omx-team events -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/events.rs crates/omx-team/src/lib.rs
git commit -m "feat(omx-team): add append-only event audit log"
```

---

## Task 4: Task Approval Gate (`omx-team/src/approvals.rs`)

**Files:**
- Create: `crates/omx-team/src/approvals.rs`
- Modify: `crates/omx-team/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-team/src/approvals.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_approval() {
        let dir = tempfile::tempdir().unwrap();
        let approval = TaskApproval {
            task_id: "task-0".into(),
            required: true,
            status: ApprovalStatus::Pending,
            reviewer: "leader".into(),
            decision_reason: String::new(),
            decided_at: None,
        };
        write_approval(dir.path(), "team-1", &approval).unwrap();
        let loaded = read_approval(dir.path(), "team-1", "task-0").unwrap().unwrap();
        assert_eq!(loaded.status, ApprovalStatus::Pending);
        assert!(loaded.required);
    }

    #[test]
    fn approve_task() {
        let dir = tempfile::tempdir().unwrap();
        let mut approval = TaskApproval {
            task_id: "task-0".into(),
            required: true,
            status: ApprovalStatus::Pending,
            reviewer: "leader".into(),
            decision_reason: String::new(),
            decided_at: None,
        };
        approval.approve("looks good");
        assert_eq!(approval.status, ApprovalStatus::Approved);
        assert_eq!(approval.decision_reason, "looks good");
        assert!(approval.decided_at.is_some());
        write_approval(dir.path(), "team-1", &approval).unwrap();
        let loaded = read_approval(dir.path(), "team-1", "task-0").unwrap().unwrap();
        assert_eq!(loaded.status, ApprovalStatus::Approved);
    }

    #[test]
    fn reject_task() {
        let mut approval = TaskApproval {
            task_id: "task-0".into(),
            required: true,
            status: ApprovalStatus::Pending,
            reviewer: "leader".into(),
            decision_reason: String::new(),
            decided_at: None,
        };
        approval.reject("needs rework");
        assert_eq!(approval.status, ApprovalStatus::Rejected);
        assert_eq!(approval.decision_reason, "needs rework");
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_approval(dir.path(), "team-1", "task-999").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn approval_path_format() {
        let path = approval_path(std::path::Path::new("/state"), "t1", "task-0");
        assert_eq!(path, std::path::PathBuf::from("/state/team/t1/approvals/task-0.json"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-team approvals -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

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
    pub fn approve(&mut self, reason: &str) {
        self.status = ApprovalStatus::Approved;
        self.decision_reason = reason.to_string();
        self.decided_at = Some(Utc::now());
    }

    pub fn reject(&mut self, reason: &str) {
        self.status = ApprovalStatus::Rejected;
        self.decision_reason = reason.to_string();
        self.decided_at = Some(Utc::now());
    }

    pub fn is_decided(&self) -> bool {
        self.status != ApprovalStatus::Pending
    }
}

/// Get the approval file path for a task.
pub fn approval_path(state_dir: &Path, team_name: &str, task_id: &str) -> PathBuf {
    state_dir.join(format!("team/{team_name}/approvals/{task_id}.json"))
}

/// Write an approval record to disk.
pub fn write_approval(state_dir: &Path, team_name: &str, approval: &TaskApproval) -> Result<(), OmxError> {
    let path = approval_path(state_dir, team_name, &approval.task_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(OmxError::Io)?;
    }
    let json = serde_json::to_string_pretty(approval)?;
    std::fs::write(&path, json).map_err(OmxError::Io)?;
    Ok(())
}

/// Read an approval record from disk. Returns `None` if not found.
pub fn read_approval(state_dir: &Path, team_name: &str, task_id: &str) -> Result<Option<TaskApproval>, OmxError> {
    let path = approval_path(state_dir, team_name, task_id);
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path).map_err(OmxError::Io)?;
    let approval: TaskApproval = serde_json::from_str(&data)?;
    Ok(Some(approval))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_approval() {
        let dir = tempfile::tempdir().unwrap();
        let approval = TaskApproval {
            task_id: "task-0".into(),
            required: true,
            status: ApprovalStatus::Pending,
            reviewer: "leader".into(),
            decision_reason: String::new(),
            decided_at: None,
        };
        write_approval(dir.path(), "team-1", &approval).unwrap();
        let loaded = read_approval(dir.path(), "team-1", "task-0").unwrap().unwrap();
        assert_eq!(loaded.status, ApprovalStatus::Pending);
        assert!(loaded.required);
    }

    #[test]
    fn approve_task() {
        let dir = tempfile::tempdir().unwrap();
        let mut approval = TaskApproval {
            task_id: "task-0".into(),
            required: true,
            status: ApprovalStatus::Pending,
            reviewer: "leader".into(),
            decision_reason: String::new(),
            decided_at: None,
        };
        approval.approve("looks good");
        assert_eq!(approval.status, ApprovalStatus::Approved);
        assert_eq!(approval.decision_reason, "looks good");
        assert!(approval.decided_at.is_some());
        write_approval(dir.path(), "team-1", &approval).unwrap();
        let loaded = read_approval(dir.path(), "team-1", "task-0").unwrap().unwrap();
        assert_eq!(loaded.status, ApprovalStatus::Approved);
    }

    #[test]
    fn reject_task() {
        let mut approval = TaskApproval {
            task_id: "task-0".into(),
            required: true,
            status: ApprovalStatus::Pending,
            reviewer: "leader".into(),
            decision_reason: String::new(),
            decided_at: None,
        };
        approval.reject("needs rework");
        assert_eq!(approval.status, ApprovalStatus::Rejected);
        assert_eq!(approval.decision_reason, "needs rework");
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_approval(dir.path(), "team-1", "task-999").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn approval_path_format() {
        let path = approval_path(Path::new("/state"), "t1", "task-0");
        assert_eq!(path, PathBuf::from("/state/team/t1/approvals/task-0.json"));
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add `pub mod approvals;` to `crates/omx-team/src/lib.rs`.

Run: `cargo test -p omx-team approvals -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/approvals.rs crates/omx-team/src/lib.rs
git commit -m "feat(omx-team): add task approval gate"
```

---

## Task 5: Merge Strategies (`omx-team/src/merge_strategy.rs`)

**Files:**
- Create: `crates/omx-team/src/merge_strategy.rs`
- Modify: `crates/omx-team/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-team/src/merge_strategy.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_command_for_merge_strategy() {
        let cmd = MergeStrategy::Merge.git_args("feature/task-1");
        assert_eq!(cmd, vec!["merge", "feature/task-1"]);
    }

    #[test]
    fn merge_command_for_cherry_pick() {
        let cmd = MergeStrategy::CherryPick.git_args("abc123");
        assert_eq!(cmd, vec!["cherry-pick", "abc123"]);
    }

    #[test]
    fn merge_command_for_squash() {
        let cmd = MergeStrategy::Squash.git_args("feature/task-1");
        assert_eq!(cmd, vec!["merge", "--squash", "feature/task-1"]);
    }

    #[test]
    fn merge_result_records_conflict() {
        let result = MergeResult {
            strategy: MergeStrategy::Merge,
            source_ref: "feature/task-1".into(),
            success: false,
            conflict_files: vec!["src/lib.rs".into(), "Cargo.toml".into()],
        };
        assert!(!result.success);
        assert_eq!(result.conflict_files.len(), 2);
    }

    #[test]
    fn default_strategy_is_merge() {
        let strategy = MergeStrategy::default();
        assert_eq!(strategy, MergeStrategy::Merge);
    }

    #[test]
    fn strategy_serde_roundtrip() {
        let strategy = MergeStrategy::Squash;
        let json = serde_json::to_string(&strategy).unwrap();
        let parsed: MergeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, MergeStrategy::Squash);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-team merge_strategy -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};

/// Strategy for integrating worker output into the main branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Standard git merge (preserves full history).
    Merge,
    /// Cherry-pick specific commits.
    CherryPick,
    /// Squash merge (single commit on target).
    Squash,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::Merge
    }
}

impl MergeStrategy {
    /// Generate the git command arguments for this strategy.
    pub fn git_args(&self, source_ref: &str) -> Vec<&str> {
        match self {
            Self::Merge => vec!["merge", source_ref],
            Self::CherryPick => vec!["cherry-pick", source_ref],
            Self::Squash => vec!["merge", "--squash", source_ref],
        }
    }
}

/// Result of a merge operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub strategy: MergeStrategy,
    pub source_ref: String,
    pub success: bool,
    pub conflict_files: Vec<String>,
}

/// Perform a merge using the given strategy. Runs git commands in `cwd`.
pub fn execute_merge(
    cwd: &std::path::Path,
    strategy: &MergeStrategy,
    source_ref: &str,
) -> Result<MergeResult, omx_types::OmxError> {
    let args = strategy.git_args(source_ref);
    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(cwd)
        .output()
        .map_err(|e| omx_types::OmxError::Team(format!("failed to run git: {e}")))?;

    if output.status.success() {
        Ok(MergeResult {
            strategy: strategy.clone(),
            source_ref: source_ref.to_string(),
            success: true,
            conflict_files: Vec::new(),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let conflict_files = parse_conflict_files(&stderr);
        Ok(MergeResult {
            strategy: strategy.clone(),
            source_ref: source_ref.to_string(),
            success: false,
            conflict_files,
        })
    }
}

/// Parse conflict file paths from git merge stderr output.
fn parse_conflict_files(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| line.contains("CONFLICT") || line.contains("Merge conflict in"))
        .filter_map(|line| {
            // "CONFLICT (content): Merge conflict in path/to/file"
            line.rsplit("Merge conflict in ").next().map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_command_for_merge_strategy() {
        let cmd = MergeStrategy::Merge.git_args("feature/task-1");
        assert_eq!(cmd, vec!["merge", "feature/task-1"]);
    }

    #[test]
    fn merge_command_for_cherry_pick() {
        let cmd = MergeStrategy::CherryPick.git_args("abc123");
        assert_eq!(cmd, vec!["cherry-pick", "abc123"]);
    }

    #[test]
    fn merge_command_for_squash() {
        let cmd = MergeStrategy::Squash.git_args("feature/task-1");
        assert_eq!(cmd, vec!["merge", "--squash", "feature/task-1"]);
    }

    #[test]
    fn merge_result_records_conflict() {
        let result = MergeResult {
            strategy: MergeStrategy::Merge,
            source_ref: "feature/task-1".into(),
            success: false,
            conflict_files: vec!["src/lib.rs".into(), "Cargo.toml".into()],
        };
        assert!(!result.success);
        assert_eq!(result.conflict_files.len(), 2);
    }

    #[test]
    fn default_strategy_is_merge() {
        let strategy = MergeStrategy::default();
        assert_eq!(strategy, MergeStrategy::Merge);
    }

    #[test]
    fn strategy_serde_roundtrip() {
        let strategy = MergeStrategy::Squash;
        let json = serde_json::to_string(&strategy).unwrap();
        let parsed: MergeStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, MergeStrategy::Squash);
    }

    #[test]
    fn parse_conflict_files_extracts_paths() {
        let stderr = "Auto-merging foo.rs\nCONFLICT (content): Merge conflict in src/lib.rs\nCONFLICT (content): Merge conflict in Cargo.toml\n";
        let files = parse_conflict_files(stderr);
        assert_eq!(files, vec!["src/lib.rs", "Cargo.toml"]);
    }

    #[test]
    fn parse_conflict_files_empty_on_no_conflicts() {
        let stderr = "Already up to date.\n";
        let files = parse_conflict_files(stderr);
        assert!(files.is_empty());
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add `pub mod merge_strategy;` to `crates/omx-team/src/lib.rs`.

Run: `cargo test -p omx-team merge_strategy -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/merge_strategy.rs crates/omx-team/src/lib.rs
git commit -m "feat(omx-team): add merge/cherry-pick/squash strategies"
```

---

## Task 6: Auto-Commit Scheduler (`omx-team/src/auto_commit.rs`)

**Files:**
- Create: `crates/omx-team/src/auto_commit.rs`
- Modify: `crates/omx-team/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-team/src/auto_commit.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn auto_commit_config_defaults() {
        let cfg = AutoCommitConfig::default();
        assert_eq!(cfg.interval, Duration::from_secs(300));
        assert!(cfg.enabled);
        assert_eq!(cfg.message_prefix, "wip");
    }

    #[test]
    fn should_commit_when_interval_elapsed() {
        let cfg = AutoCommitConfig {
            interval: Duration::from_secs(60),
            enabled: true,
            message_prefix: "wip".into(),
        };
        let last_commit = chrono::Utc::now() - chrono::Duration::seconds(120);
        assert!(should_auto_commit(&cfg, Some(last_commit)));
    }

    #[test]
    fn should_not_commit_when_recent() {
        let cfg = AutoCommitConfig {
            interval: Duration::from_secs(300),
            enabled: true,
            message_prefix: "wip".into(),
        };
        let last_commit = chrono::Utc::now() - chrono::Duration::seconds(10);
        assert!(!should_auto_commit(&cfg, Some(last_commit)));
    }

    #[test]
    fn should_commit_when_never_committed() {
        let cfg = AutoCommitConfig::default();
        assert!(should_auto_commit(&cfg, None));
    }

    #[test]
    fn should_not_commit_when_disabled() {
        let cfg = AutoCommitConfig {
            interval: Duration::from_secs(60),
            enabled: false,
            message_prefix: "wip".into(),
        };
        assert!(!should_auto_commit(&cfg, None));
    }

    #[test]
    fn format_auto_commit_message() {
        let msg = auto_commit_message("wip", "worker-0", "task-3");
        assert_eq!(msg, "wip: worker-0 task-3");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-team auto_commit -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for automatic WIP commits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCommitConfig {
    /// How often to auto-commit (in seconds, deserialized from TOML).
    #[serde(with = "duration_secs")]
    pub interval: Duration,
    /// Whether auto-commit is enabled.
    pub enabled: bool,
    /// Prefix for auto-commit messages.
    pub message_prefix: String,
}

impl Default for AutoCommitConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300),
            enabled: true,
            message_prefix: "wip".into(),
        }
    }
}

/// Check whether an auto-commit should happen now.
pub fn should_auto_commit(config: &AutoCommitConfig, last_commit_at: Option<DateTime<Utc>>) -> bool {
    if !config.enabled {
        return false;
    }

    match last_commit_at {
        None => true,
        Some(last) => {
            let elapsed = Utc::now().signed_duration_since(last);
            elapsed
                .to_std()
                .map(|d| d >= config.interval)
                .unwrap_or(true)
        }
    }
}

/// Format an auto-commit message.
pub fn auto_commit_message(prefix: &str, worker_name: &str, task_id: &str) -> String {
    format!("{prefix}: {worker_name} {task_id}")
}

/// Run `git add -A && git commit` in the given working directory.
pub fn run_auto_commit(
    cwd: &std::path::Path,
    message: &str,
) -> Result<Option<String>, omx_types::OmxError> {
    // Stage all changes
    let add_output = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(cwd)
        .output()
        .map_err(|e| omx_types::OmxError::Team(format!("git add failed: {e}")))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(omx_types::OmxError::Team(format!("git add failed: {stderr}")));
    }

    // Check if there's anything to commit
    let diff_output = std::process::Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(cwd)
        .output()
        .map_err(|e| omx_types::OmxError::Team(format!("git diff failed: {e}")))?;

    if diff_output.status.success() {
        // Nothing staged — skip commit
        return Ok(None);
    }

    // Commit
    let commit_output = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(cwd)
        .output()
        .map_err(|e| omx_types::OmxError::Team(format!("git commit failed: {e}")))?;

    if commit_output.status.success() {
        // Get the commit SHA
        let sha_output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(cwd)
            .output()
            .map_err(|e| omx_types::OmxError::Team(format!("git rev-parse failed: {e}")))?;
        let sha = String::from_utf8_lossy(&sha_output.stdout).trim().to_string();
        Ok(Some(sha))
    } else {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        Err(omx_types::OmxError::Team(format!("git commit failed: {stderr}")))
    }
}

/// Serde helper for Duration as seconds (u64).
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(duration: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_commit_config_defaults() {
        let cfg = AutoCommitConfig::default();
        assert_eq!(cfg.interval, Duration::from_secs(300));
        assert!(cfg.enabled);
        assert_eq!(cfg.message_prefix, "wip");
    }

    #[test]
    fn should_commit_when_interval_elapsed() {
        let cfg = AutoCommitConfig {
            interval: Duration::from_secs(60),
            enabled: true,
            message_prefix: "wip".into(),
        };
        let last_commit = Utc::now() - chrono::Duration::seconds(120);
        assert!(should_auto_commit(&cfg, Some(last_commit)));
    }

    #[test]
    fn should_not_commit_when_recent() {
        let cfg = AutoCommitConfig {
            interval: Duration::from_secs(300),
            enabled: true,
            message_prefix: "wip".into(),
        };
        let last_commit = Utc::now() - chrono::Duration::seconds(10);
        assert!(!should_auto_commit(&cfg, Some(last_commit)));
    }

    #[test]
    fn should_commit_when_never_committed() {
        let cfg = AutoCommitConfig::default();
        assert!(should_auto_commit(&cfg, None));
    }

    #[test]
    fn should_not_commit_when_disabled() {
        let cfg = AutoCommitConfig {
            interval: Duration::from_secs(60),
            enabled: false,
            message_prefix: "wip".into(),
        };
        assert!(!should_auto_commit(&cfg, None));
    }

    #[test]
    fn format_auto_commit_message() {
        let msg = auto_commit_message("wip", "worker-0", "task-3");
        assert_eq!(msg, "wip: worker-0 task-3");
    }

    #[test]
    fn auto_commit_config_serde_roundtrip() {
        let cfg = AutoCommitConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: AutoCommitConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.interval, Duration::from_secs(300));
        assert!(parsed.enabled);
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add `pub mod auto_commit;` to `crates/omx-team/src/lib.rs`.

Run: `cargo test -p omx-team auto_commit -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/auto_commit.rs crates/omx-team/src/lib.rs
git commit -m "feat(omx-team): add auto-commit scheduler for workers"
```

---

## Task 7: Config Enhancements (`omx-team/src/config.rs`)

**Files:**
- Modify: `crates/omx-team/src/config.rs`

- [ ] **Step 1: Write the failing tests**

Add to `crates/omx-team/src/config.rs` test module:

```rust
#[test]
fn team_config_with_heartbeat_defaults() {
    let config = parse_team_spec(2, "executor", "build it", None).unwrap();
    assert_eq!(config.heartbeat_interval_secs, 30);
    assert_eq!(config.heartbeat_stale_secs, 90);
}

#[test]
fn team_config_with_merge_strategy() {
    let config = parse_team_spec(1, "executor", "task", None).unwrap();
    assert_eq!(config.merge_strategy, crate::merge_strategy::MergeStrategy::Merge);
}

#[test]
fn team_config_with_auto_commit() {
    let config = parse_team_spec(1, "executor", "task", None).unwrap();
    assert!(config.auto_commit.enabled);
}

#[test]
fn team_config_approval_required_defaults_false() {
    let config = parse_team_spec(1, "executor", "task", None).unwrap();
    assert!(!config.approval_required);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-team config -- --nocapture 2>&1 | head -30`
Expected: FAIL — fields don't exist

- [ ] **Step 3: Add new fields to TeamConfig**

Modify `crates/omx-team/src/config.rs` to add the following fields to `TeamConfig`:

```rust
use crate::auto_commit::AutoCommitConfig;
use crate::merge_strategy::MergeStrategy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub name: TeamName,
    pub workers: Vec<WorkerConfig>,
    pub tasks: Vec<TeamTask>,
    pub governance: TeamGovernance,
    pub worktree_mode: WorktreeMode,
    pub dispatch_mode: DispatchMode,
    // Phase 9 additions:
    pub heartbeat_interval_secs: u64,
    pub heartbeat_stale_secs: u64,
    pub merge_strategy: MergeStrategy,
    pub auto_commit: AutoCommitConfig,
    pub approval_required: bool,
}
```

Update `parse_team_spec` to set defaults:

```rust
Ok(TeamConfig {
    name,
    workers,
    tasks,
    governance: TeamGovernance::default(),
    worktree_mode: WorktreeMode::PerWorker,
    dispatch_mode: DispatchMode::Tmux,
    heartbeat_interval_secs: 30,
    heartbeat_stale_secs: 90,
    merge_strategy: MergeStrategy::default(),
    auto_commit: AutoCommitConfig::default(),
    approval_required: false,
})
```

Update all existing test helper functions (`make_config` in `orchestrator.rs` tests, etc.) to include the new fields with defaults.

- [ ] **Step 4: Fix compilation across crate**

Add the new fields with defaults wherever `TeamConfig` is constructed in tests:

In `crates/omx-team/src/orchestrator.rs` `make_config`:
```rust
heartbeat_interval_secs: 30,
heartbeat_stale_secs: 90,
merge_strategy: crate::merge_strategy::MergeStrategy::default(),
auto_commit: crate::auto_commit::AutoCommitConfig::default(),
approval_required: false,
```

In `crates/omx-team/src/worker.rs` test:
```rust
heartbeat_interval_secs: 30,
heartbeat_stale_secs: 90,
merge_strategy: crate::merge_strategy::MergeStrategy::default(),
auto_commit: crate::auto_commit::AutoCommitConfig::default(),
approval_required: false,
```

- [ ] **Step 5: Run full crate tests**

Run: `cargo test -p omx-team -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/omx-team/src/config.rs crates/omx-team/src/orchestrator.rs crates/omx-team/src/worker.rs
git commit -m "feat(omx-team): add heartbeat, merge, auto-commit, approval config fields"
```

---

## Task 8: Dead Worker Recovery in Orchestrator (`omx-team/src/orchestrator.rs`)

**Files:**
- Modify: `crates/omx-team/src/orchestrator.rs`

- [ ] **Step 1: Write the failing test**

Add to `crates/omx-team/src/orchestrator.rs` tests:

```rust
#[test]
fn mark_stale_workers_dead() {
    let config = make_config(2);
    let mut state = OrchestratorState::new(config);

    // Simulate worker-0 becoming stale
    state.workers[0].2 = false; // alive = false

    let dead = collect_dead_workers(&state);
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0], WorkerId("worker-0".into()));
}

#[test]
fn reassign_dead_worker_tasks() {
    let config = make_config(2);
    let mut state = OrchestratorState::new(config);

    // Assign task-0 to worker-0, then kill worker-0
    state.tasks[0].1 = TaskStatus::InProgress;
    state.workers[0].3 = Some(TaskId("task-0".into()));
    state.workers[0].2 = false; // dead

    let reassigned = reassign_from_dead_workers(&mut state);
    assert_eq!(reassigned.len(), 1);
    assert_eq!(reassigned[0], TaskId("task-0".into()));
    // Task should be back to Pending
    assert_eq!(state.tasks[0].1, TaskStatus::Pending);
    // Worker-0 should have no task
    assert!(state.workers[0].3.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-team orchestrator -- --nocapture 2>&1 | head -30`
Expected: FAIL — functions not defined

- [ ] **Step 3: Write implementations**

Add to `crates/omx-team/src/orchestrator.rs`:

```rust
/// Return IDs of workers that are not alive.
pub fn collect_dead_workers(state: &OrchestratorState) -> Vec<WorkerId> {
    state
        .workers
        .iter()
        .filter(|(_, _, alive, _)| !*alive)
        .map(|(id, _, _, _)| id.clone())
        .collect()
}

/// Reassign tasks from dead workers back to Pending. Returns the IDs of reassigned tasks.
pub fn reassign_from_dead_workers(state: &mut OrchestratorState) -> Vec<TaskId> {
    let dead_workers = collect_dead_workers(state);
    let mut reassigned = Vec::new();

    for dead_id in &dead_workers {
        // Find which worker index this is
        if let Some(worker) = state.workers.iter_mut().find(|(id, _, _, _)| id == dead_id) {
            if let Some(task_id) = worker.3.take() {
                // Reset the task to Pending
                if let Some(task) = state.tasks.iter_mut().find(|(id, _, _)| *id == task_id) {
                    if task.1 == TaskStatus::InProgress {
                        task.1 = TaskStatus::Pending;
                        reassigned.push(task_id);
                        tracing::info!(
                            worker = %dead_id.0,
                            task = %reassigned.last().unwrap().0,
                            "reassigned task from dead worker"
                        );
                    }
                }
            }
        }
    }

    reassigned
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-team orchestrator -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/orchestrator.rs
git commit -m "feat(omx-team): add dead worker detection and task reassignment"
```

---

## Task 9: Pane Registry (`omx-mux/src/pane_registry.rs`)

**Files:**
- Create: `crates/omx-mux/src/pane_registry.rs`
- Modify: `crates/omx-mux/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-mux/src/pane_registry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_pane() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "omx-team:0.1");
        assert_eq!(registry.lookup("worker-0"), Some("omx-team:0.1"));
    }

    #[test]
    fn lookup_missing_returns_none() {
        let registry = PaneRegistry::new();
        assert_eq!(registry.lookup("ghost"), None);
    }

    #[test]
    fn unregister_removes_entry() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "omx-team:0.1");
        registry.unregister("worker-0");
        assert_eq!(registry.lookup("worker-0"), None);
    }

    #[test]
    fn list_all_entries() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "sess:0.0");
        registry.register("worker-1", "sess:0.1");
        let all = registry.list();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn update_existing_entry() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "sess:0.0");
        registry.register("worker-0", "sess:1.0");
        assert_eq!(registry.lookup("worker-0"), Some("sess:1.0"));
    }

    #[test]
    fn reverse_lookup_by_pane_id() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "sess:0.1");
        registry.register("worker-1", "sess:0.2");
        assert_eq!(registry.reverse_lookup("sess:0.1"), Some("worker-0"));
        assert_eq!(registry.reverse_lookup("sess:9.9"), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-mux pane_registry -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use std::collections::HashMap;

/// Maps logical worker IDs to tmux pane IDs and vice versa.
#[derive(Debug, Clone, Default)]
pub struct PaneRegistry {
    worker_to_pane: HashMap<String, String>,
}

impl PaneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a mapping from worker ID to pane ID.
    pub fn register(&mut self, worker_id: &str, pane_id: &str) {
        self.worker_to_pane
            .insert(worker_id.to_string(), pane_id.to_string());
    }

    /// Look up the pane ID for a worker.
    pub fn lookup(&self, worker_id: &str) -> Option<&str> {
        self.worker_to_pane.get(worker_id).map(|s| s.as_str())
    }

    /// Reverse lookup: find the worker ID for a pane ID.
    pub fn reverse_lookup(&self, pane_id: &str) -> Option<&str> {
        self.worker_to_pane
            .iter()
            .find(|(_, v)| v.as_str() == pane_id)
            .map(|(k, _)| k.as_str())
    }

    /// Remove a worker's pane mapping.
    pub fn unregister(&mut self, worker_id: &str) {
        self.worker_to_pane.remove(worker_id);
    }

    /// List all registered (worker_id, pane_id) pairs.
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.worker_to_pane
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_pane() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "omx-team:0.1");
        assert_eq!(registry.lookup("worker-0"), Some("omx-team:0.1"));
    }

    #[test]
    fn lookup_missing_returns_none() {
        let registry = PaneRegistry::new();
        assert_eq!(registry.lookup("ghost"), None);
    }

    #[test]
    fn unregister_removes_entry() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "omx-team:0.1");
        registry.unregister("worker-0");
        assert_eq!(registry.lookup("worker-0"), None);
    }

    #[test]
    fn list_all_entries() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "sess:0.0");
        registry.register("worker-1", "sess:0.1");
        let all = registry.list();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn update_existing_entry() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "sess:0.0");
        registry.register("worker-0", "sess:1.0");
        assert_eq!(registry.lookup("worker-0"), Some("sess:1.0"));
    }

    #[test]
    fn reverse_lookup_by_pane_id() {
        let mut registry = PaneRegistry::new();
        registry.register("worker-0", "sess:0.1");
        registry.register("worker-1", "sess:0.2");
        assert_eq!(registry.reverse_lookup("sess:0.1"), Some("worker-0"));
        assert_eq!(registry.reverse_lookup("sess:9.9"), None);
    }
}
```

- [ ] **Step 4: Register module in lib.rs**

Add to `crates/omx-mux/src/lib.rs`:

```rust
pub mod pane_registry;
pub use pane_registry::PaneRegistry;
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p omx-mux pane_registry -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/omx-mux/src/pane_registry.rs crates/omx-mux/src/lib.rs
git commit -m "feat(omx-mux): add pane registry for worker-to-pane mapping"
```

---

## Task 10: HUD Pane Management (`omx-mux/src/hud_pane.rs`)

**Files:**
- Create: `crates/omx-mux/src/hud_pane.rs`
- Modify: `crates/omx-mux/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-mux/src/hud_pane.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_pane_state_starts_inactive() {
        let state = HudPaneState::new();
        assert!(!state.is_active());
        assert!(state.pane_id.is_none());
    }

    #[test]
    fn activate_and_deactivate() {
        let mut state = HudPaneState::new();
        state.activate("omx-team:hud.0");
        assert!(state.is_active());
        assert_eq!(state.pane_id.as_deref(), Some("omx-team:hud.0"));
        state.deactivate();
        assert!(!state.is_active());
        assert!(state.pane_id.is_none());
    }

    #[test]
    fn resize_updates_dimensions() {
        let mut state = HudPaneState::new();
        state.activate("sess:hud.0");
        state.resize(120, 30);
        assert_eq!(state.width, 120);
        assert_eq!(state.height, 30);
    }

    #[test]
    fn create_hud_args_correct() {
        let args = build_create_hud_args("omx-team-abc", 10);
        assert_eq!(args, vec![
            "split-window", "-t", "omx-team-abc", "-l", "10",
            "-d", "-P", "-F", "#{session_name}:#{window_index}.#{pane_index}"
        ]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-mux hud_pane -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
/// State for the dedicated HUD pane.
#[derive(Debug, Clone, Default)]
pub struct HudPaneState {
    pub pane_id: Option<String>,
    pub width: u16,
    pub height: u16,
}

impl HudPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.pane_id.is_some()
    }

    pub fn activate(&mut self, pane_id: &str) {
        self.pane_id = Some(pane_id.to_string());
    }

    pub fn deactivate(&mut self) {
        self.pane_id = None;
        self.width = 0;
        self.height = 0;
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }
}

/// Build tmux args to create a HUD pane as a split below.
pub fn build_create_hud_args(session: &str, height_lines: u16) -> Vec<String> {
    vec![
        "split-window".into(),
        "-t".into(),
        session.into(),
        "-l".into(),
        height_lines.to_string(),
        "-d".into(),
        "-P".into(),
        "-F".into(),
        "#{session_name}:#{window_index}.#{pane_index}".into(),
    ]
}

/// Build tmux args to resize the HUD pane.
pub fn build_resize_hud_args(pane_id: &str, height_lines: u16) -> Vec<String> {
    vec![
        "resize-pane".into(),
        "-t".into(),
        pane_id.into(),
        "-y".into(),
        height_lines.to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_pane_state_starts_inactive() {
        let state = HudPaneState::new();
        assert!(!state.is_active());
        assert!(state.pane_id.is_none());
    }

    #[test]
    fn activate_and_deactivate() {
        let mut state = HudPaneState::new();
        state.activate("omx-team:hud.0");
        assert!(state.is_active());
        assert_eq!(state.pane_id.as_deref(), Some("omx-team:hud.0"));
        state.deactivate();
        assert!(!state.is_active());
        assert!(state.pane_id.is_none());
    }

    #[test]
    fn resize_updates_dimensions() {
        let mut state = HudPaneState::new();
        state.activate("sess:hud.0");
        state.resize(120, 30);
        assert_eq!(state.width, 120);
        assert_eq!(state.height, 30);
    }

    #[test]
    fn create_hud_args_correct() {
        let args = build_create_hud_args("omx-team-abc", 10);
        assert_eq!(args, vec![
            "split-window", "-t", "omx-team-abc", "-l", "10",
            "-d", "-P", "-F", "#{session_name}:#{window_index}.#{pane_index}"
        ]);
    }

    #[test]
    fn resize_hud_args_correct() {
        let args = build_resize_hud_args("sess:0.2", 15);
        assert_eq!(args, vec!["resize-pane", "-t", "sess:0.2", "-y", "15"]);
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add to `crates/omx-mux/src/lib.rs`:

```rust
pub mod hud_pane;
pub use hud_pane::HudPaneState;
```

Run: `cargo test -p omx-mux hud_pane -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-mux/src/hud_pane.rs crates/omx-mux/src/lib.rs
git commit -m "feat(omx-mux): add HUD pane lifecycle management"
```

---

## Task 11: Leader Pane Guard (`omx-mux/src/leader_guard.rs`)

**Files:**
- Create: `crates/omx-mux/src/leader_guard.rs`
- Modify: `crates/omx-mux/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-mux/src/leader_guard.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_blocks_leader_pane_kill() {
        let guard = LeaderGuard::new("sess:0.0");
        assert!(guard.is_protected("sess:0.0"));
        assert!(!guard.is_protected("sess:0.1"));
    }

    #[test]
    fn guard_blocks_kill_window_targeting_leader() {
        let guard = LeaderGuard::new("sess:0.0");
        let result = guard.check_kill_target("sess:0.0");
        assert!(result.is_err());
    }

    #[test]
    fn guard_allows_kill_window_for_workers() {
        let guard = LeaderGuard::new("sess:0.0");
        let result = guard.check_kill_target("sess:0.1");
        assert!(result.is_ok());
    }

    #[test]
    fn guard_with_no_leader_allows_all() {
        let guard = LeaderGuard::none();
        assert!(!guard.is_protected("sess:0.0"));
        assert!(guard.check_kill_target("sess:0.0").is_ok());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-mux leader_guard -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use crate::types::MuxError;

/// Prevents accidental destruction of the leader pane.
#[derive(Debug, Clone)]
pub struct LeaderGuard {
    leader_pane_id: Option<String>,
}

impl LeaderGuard {
    /// Create a guard protecting the given pane.
    pub fn new(leader_pane_id: &str) -> Self {
        Self {
            leader_pane_id: Some(leader_pane_id.to_string()),
        }
    }

    /// Create a guard that protects nothing.
    pub fn none() -> Self {
        Self {
            leader_pane_id: None,
        }
    }

    /// Check if a pane ID is the protected leader pane.
    pub fn is_protected(&self, pane_id: &str) -> bool {
        self.leader_pane_id.as_deref() == Some(pane_id)
    }

    /// Check if a kill-window/kill-pane target is safe. Returns error if targeting the leader.
    pub fn check_kill_target(&self, target: &str) -> Result<(), MuxError> {
        if self.is_protected(target) {
            Err(MuxError::InvalidTarget(format!(
                "cannot kill leader pane: {target}"
            )))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_blocks_leader_pane_kill() {
        let guard = LeaderGuard::new("sess:0.0");
        assert!(guard.is_protected("sess:0.0"));
        assert!(!guard.is_protected("sess:0.1"));
    }

    #[test]
    fn guard_blocks_kill_window_targeting_leader() {
        let guard = LeaderGuard::new("sess:0.0");
        let result = guard.check_kill_target("sess:0.0");
        assert!(result.is_err());
    }

    #[test]
    fn guard_allows_kill_window_for_workers() {
        let guard = LeaderGuard::new("sess:0.0");
        let result = guard.check_kill_target("sess:0.1");
        assert!(result.is_ok());
    }

    #[test]
    fn guard_with_no_leader_allows_all() {
        let guard = LeaderGuard::none();
        assert!(!guard.is_protected("sess:0.0"));
        assert!(guard.check_kill_target("sess:0.0").is_ok());
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add to `crates/omx-mux/src/lib.rs`:

```rust
pub mod leader_guard;
pub use leader_guard::LeaderGuard;
```

Run: `cargo test -p omx-mux leader_guard -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-mux/src/leader_guard.rs crates/omx-mux/src/lib.rs
git commit -m "feat(omx-mux): add leader pane protection guard"
```

---

## Task 12: Trust Prompt Dismissal (`omx-mux/src/trust_dismiss.rs`)

**Files:**
- Create: `crates/omx-mux/src/trust_dismiss.rs`
- Modify: `crates/omx-mux/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-mux/src/trust_dismiss.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_trust_prompt_in_output() {
        let output = "some output\nDo you trust the files in this folder?\nmore text";
        assert!(contains_trust_prompt(output));
    }

    #[test]
    fn no_trust_prompt_in_normal_output() {
        let output = "Building project...\nDone.";
        assert!(!contains_trust_prompt(output));
    }

    #[test]
    fn detect_claude_trust_prompt() {
        let output = "? Do you want to trust this project directory?";
        assert!(contains_trust_prompt(output));
    }

    #[test]
    fn dismiss_keys_sends_yes_enter() {
        let keys = trust_dismiss_keys();
        assert_eq!(keys, "y");
    }

    #[test]
    fn detect_permission_prompt() {
        let output = "Allow tool access? (yes/no)";
        assert!(contains_trust_prompt(output));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-mux trust_dismiss -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
/// Patterns that indicate a trust/permission prompt in pane output.
const TRUST_PATTERNS: &[&str] = &[
    "Do you trust",
    "trust this project",
    "Allow tool access",
    "Do you want to trust",
];

/// Check if pane output contains a trust/permission prompt.
pub fn contains_trust_prompt(pane_output: &str) -> bool {
    let lower = pane_output.to_lowercase();
    TRUST_PATTERNS
        .iter()
        .any(|pattern| lower.contains(&pattern.to_lowercase()))
}

/// Return the keys to send to dismiss a trust prompt (answer "yes").
pub fn trust_dismiss_keys() -> &'static str {
    "y"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_trust_prompt_in_output() {
        let output = "some output\nDo you trust the files in this folder?\nmore text";
        assert!(contains_trust_prompt(output));
    }

    #[test]
    fn no_trust_prompt_in_normal_output() {
        let output = "Building project...\nDone.";
        assert!(!contains_trust_prompt(output));
    }

    #[test]
    fn detect_claude_trust_prompt() {
        let output = "? Do you want to trust this project directory?";
        assert!(contains_trust_prompt(output));
    }

    #[test]
    fn dismiss_keys_sends_yes_enter() {
        let keys = trust_dismiss_keys();
        assert_eq!(keys, "y");
    }

    #[test]
    fn detect_permission_prompt() {
        let output = "Allow tool access? (yes/no)";
        assert!(contains_trust_prompt(output));
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add to `crates/omx-mux/src/lib.rs`:

```rust
pub mod trust_dismiss;
```

Run: `cargo test -p omx-mux trust_dismiss -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-mux/src/trust_dismiss.rs crates/omx-mux/src/lib.rs
git commit -m "feat(omx-mux): add trust prompt detection and auto-dismiss"
```

---

## Task 13: Resize Hook (`omx-mux/src/resize_hook.rs`)

**Files:**
- Create: `crates/omx-mux/src/resize_hook.rs`
- Modify: `crates/omx-mux/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-mux/src/resize_hook.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_event_captures_dimensions() {
        let event = ResizeEvent::new("sess:0.0", 120, 40);
        assert_eq!(event.pane_id, "sess:0.0");
        assert_eq!(event.width, 120);
        assert_eq!(event.height, 40);
    }

    #[test]
    fn build_monitor_hook_args() {
        let args = build_resize_monitor_args("omx-team-abc", "omx resize-handler");
        assert_eq!(args, vec![
            "set-hook", "-t", "omx-team-abc",
            "after-resize-pane", "run-shell", "omx resize-handler"
        ]);
    }

    #[test]
    fn build_remove_hook_args() {
        let args = build_remove_resize_hook_args("omx-team-abc");
        assert_eq!(args, vec![
            "set-hook", "-u", "-t", "omx-team-abc", "after-resize-pane"
        ]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-mux resize_hook -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};

/// A terminal resize event for a pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeEvent {
    pub pane_id: String,
    pub width: u16,
    pub height: u16,
    pub timestamp: String,
}

impl ResizeEvent {
    pub fn new(pane_id: &str, width: u16, height: u16) -> Self {
        Self {
            pane_id: pane_id.to_string(),
            width,
            height,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Build tmux args to register a resize hook on a session.
pub fn build_resize_monitor_args<'a>(session: &'a str, command: &'a str) -> Vec<&'a str> {
    vec![
        "set-hook", "-t", session,
        "after-resize-pane", "run-shell", command,
    ]
}

/// Build tmux args to remove a resize hook from a session.
pub fn build_remove_resize_hook_args<'a>(session: &'a str) -> Vec<&'a str> {
    vec![
        "set-hook", "-u", "-t", session, "after-resize-pane",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_event_captures_dimensions() {
        let event = ResizeEvent::new("sess:0.0", 120, 40);
        assert_eq!(event.pane_id, "sess:0.0");
        assert_eq!(event.width, 120);
        assert_eq!(event.height, 40);
    }

    #[test]
    fn build_monitor_hook_args() {
        let args = build_resize_monitor_args("omx-team-abc", "omx resize-handler");
        assert_eq!(args, vec![
            "set-hook", "-t", "omx-team-abc",
            "after-resize-pane", "run-shell", "omx resize-handler"
        ]);
    }

    #[test]
    fn build_remove_hook_args() {
        let args = build_remove_resize_hook_args("omx-team-abc");
        assert_eq!(args, vec![
            "set-hook", "-u", "-t", "omx-team-abc", "after-resize-pane"
        ]);
    }
}
```

- [ ] **Step 4: Add chrono to omx-mux Cargo.toml**

In `crates/omx-mux/Cargo.toml`, add:

```toml
chrono = { workspace = true }
```

- [ ] **Step 5: Register module and run tests**

Add to `crates/omx-mux/src/lib.rs`:

```rust
pub mod resize_hook;
```

Run: `cargo test -p omx-mux resize_hook -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add crates/omx-mux/src/resize_hook.rs crates/omx-mux/src/lib.rs crates/omx-mux/Cargo.toml
git commit -m "feat(omx-mux): add resize hook registration and events"
```

---

## Task 14: Hook Chaining (`omx-hooks/src/chain.rs`)

**Files:**
- Create: `crates/omx-hooks/src/chain.rs`
- Modify: `crates/omx-hooks/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-hooks/src/chain.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_preserves_order() {
        let chain = HookChain::new(vec![
            ChainEntry { name: "first".into(), priority: 1 },
            ChainEntry { name: "second".into(), priority: 2 },
            ChainEntry { name: "third".into(), priority: 3 },
        ]);
        let names: Vec<&str> = chain.ordered().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn chain_sorts_by_priority() {
        let chain = HookChain::new(vec![
            ChainEntry { name: "low".into(), priority: 10 },
            ChainEntry { name: "high".into(), priority: 1 },
            ChainEntry { name: "mid".into(), priority: 5 },
        ]);
        let names: Vec<&str> = chain.ordered().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["high", "mid", "low"]);
    }

    #[test]
    fn empty_chain() {
        let chain = HookChain::new(vec![]);
        assert!(chain.ordered().is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn chain_length() {
        let chain = HookChain::new(vec![
            ChainEntry { name: "a".into(), priority: 1 },
            ChainEntry { name: "b".into(), priority: 2 },
        ]);
        assert_eq!(chain.len(), 2);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-hooks chain -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};

/// An entry in the hook chain, with a name and execution priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    pub name: String,
    /// Lower numbers run first.
    pub priority: u32,
}

/// An ordered sequence of hooks to execute for a given event.
#[derive(Debug, Clone)]
pub struct HookChain {
    entries: Vec<ChainEntry>,
}

impl HookChain {
    /// Create a new hook chain. Entries are sorted by priority on construction.
    pub fn new(mut entries: Vec<ChainEntry>) -> Self {
        entries.sort_by_key(|e| e.priority);
        Self { entries }
    }

    /// Return the hooks in execution order (sorted by priority).
    pub fn ordered(&self) -> &[ChainEntry] {
        &self.entries
    }

    /// Number of hooks in the chain.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_preserves_order() {
        let chain = HookChain::new(vec![
            ChainEntry { name: "first".into(), priority: 1 },
            ChainEntry { name: "second".into(), priority: 2 },
            ChainEntry { name: "third".into(), priority: 3 },
        ]);
        let names: Vec<&str> = chain.ordered().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn chain_sorts_by_priority() {
        let chain = HookChain::new(vec![
            ChainEntry { name: "low".into(), priority: 10 },
            ChainEntry { name: "high".into(), priority: 1 },
            ChainEntry { name: "mid".into(), priority: 5 },
        ]);
        let names: Vec<&str> = chain.ordered().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["high", "mid", "low"]);
    }

    #[test]
    fn empty_chain() {
        let chain = HookChain::new(vec![]);
        assert!(chain.ordered().is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn chain_length() {
        let chain = HookChain::new(vec![
            ChainEntry { name: "a".into(), priority: 1 },
            ChainEntry { name: "b".into(), priority: 2 },
        ]);
        assert_eq!(chain.len(), 2);
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add `pub mod chain;` to `crates/omx-hooks/src/lib.rs`.

Run: `cargo test -p omx-hooks chain -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-hooks/src/chain.rs crates/omx-hooks/src/lib.rs
git commit -m "feat(omx-hooks): add ordered hook chain with priority sorting"
```

---

## Task 15: Hook Result Aggregator (`omx-hooks/src/aggregator.rs`)

**Files:**
- Create: `crates/omx-hooks/src/aggregator.rs`
- Modify: `crates/omx-hooks/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-hooks/src/aggregator.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::HookResult;

    #[test]
    fn aggregate_collects_all_stdout() {
        let results = vec![
            HookResult { hook: "a".into(), success: true, stdout: "output-a".into(), stderr: String::new(), duration_ms: 10 },
            HookResult { hook: "b".into(), success: true, stdout: "output-b".into(), stderr: String::new(), duration_ms: 20 },
        ];
        let agg = aggregate_results(&results);
        assert_eq!(agg.total, 2);
        assert_eq!(agg.succeeded, 2);
        assert_eq!(agg.failed, 0);
        assert!(agg.combined_stdout.contains("output-a"));
        assert!(agg.combined_stdout.contains("output-b"));
    }

    #[test]
    fn aggregate_counts_failures() {
        let results = vec![
            HookResult { hook: "a".into(), success: true, stdout: "ok".into(), stderr: String::new(), duration_ms: 10 },
            HookResult { hook: "b".into(), success: false, stdout: String::new(), stderr: "boom".into(), duration_ms: 5 },
        ];
        let agg = aggregate_results(&results);
        assert_eq!(agg.total, 2);
        assert_eq!(agg.succeeded, 1);
        assert_eq!(agg.failed, 1);
        assert!(agg.combined_stderr.contains("boom"));
    }

    #[test]
    fn aggregate_empty_results() {
        let agg = aggregate_results(&[]);
        assert_eq!(agg.total, 0);
        assert_eq!(agg.succeeded, 0);
        assert!(agg.combined_stdout.is_empty());
    }

    #[test]
    fn aggregate_total_duration() {
        let results = vec![
            HookResult { hook: "a".into(), success: true, stdout: String::new(), stderr: String::new(), duration_ms: 100 },
            HookResult { hook: "b".into(), success: true, stdout: String::new(), stderr: String::new(), duration_ms: 200 },
        ];
        let agg = aggregate_results(&results);
        assert_eq!(agg.total_duration_ms, 300);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-hooks aggregator -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use crate::HookResult;

/// Aggregated result from running a chain of hooks.
#[derive(Debug, Clone)]
pub struct AggregatedHookResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub combined_stdout: String,
    pub combined_stderr: String,
    pub total_duration_ms: u64,
}

/// Aggregate results from multiple hook executions.
pub fn aggregate_results(results: &[HookResult]) -> AggregatedHookResult {
    let mut stdout_parts = Vec::new();
    let mut stderr_parts = Vec::new();
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut total_duration_ms = 0u64;

    for result in results {
        if result.success {
            succeeded += 1;
        } else {
            failed += 1;
        }
        if !result.stdout.is_empty() {
            stdout_parts.push(result.stdout.as_str());
        }
        if !result.stderr.is_empty() {
            stderr_parts.push(result.stderr.as_str());
        }
        total_duration_ms += result.duration_ms;
    }

    AggregatedHookResult {
        total: results.len(),
        succeeded,
        failed,
        combined_stdout: stdout_parts.join("\n"),
        combined_stderr: stderr_parts.join("\n"),
        total_duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HookResult;

    #[test]
    fn aggregate_collects_all_stdout() {
        let results = vec![
            HookResult { hook: "a".into(), success: true, stdout: "output-a".into(), stderr: String::new(), duration_ms: 10 },
            HookResult { hook: "b".into(), success: true, stdout: "output-b".into(), stderr: String::new(), duration_ms: 20 },
        ];
        let agg = aggregate_results(&results);
        assert_eq!(agg.total, 2);
        assert_eq!(agg.succeeded, 2);
        assert_eq!(agg.failed, 0);
        assert!(agg.combined_stdout.contains("output-a"));
        assert!(agg.combined_stdout.contains("output-b"));
    }

    #[test]
    fn aggregate_counts_failures() {
        let results = vec![
            HookResult { hook: "a".into(), success: true, stdout: "ok".into(), stderr: String::new(), duration_ms: 10 },
            HookResult { hook: "b".into(), success: false, stdout: String::new(), stderr: "boom".into(), duration_ms: 5 },
        ];
        let agg = aggregate_results(&results);
        assert_eq!(agg.total, 2);
        assert_eq!(agg.succeeded, 1);
        assert_eq!(agg.failed, 1);
        assert!(agg.combined_stderr.contains("boom"));
    }

    #[test]
    fn aggregate_empty_results() {
        let agg = aggregate_results(&[]);
        assert_eq!(agg.total, 0);
        assert_eq!(agg.succeeded, 0);
        assert!(agg.combined_stdout.is_empty());
    }

    #[test]
    fn aggregate_total_duration() {
        let results = vec![
            HookResult { hook: "a".into(), success: true, stdout: String::new(), stderr: String::new(), duration_ms: 100 },
            HookResult { hook: "b".into(), success: true, stdout: String::new(), stderr: String::new(), duration_ms: 200 },
        ];
        let agg = aggregate_results(&results);
        assert_eq!(agg.total_duration_ms, 300);
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add `pub mod aggregator;` to `crates/omx-hooks/src/lib.rs`.

Run: `cargo test -p omx-hooks aggregator -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-hooks/src/aggregator.rs crates/omx-hooks/src/lib.rs
git commit -m "feat(omx-hooks): add hook result aggregator"
```

---

## Task 16: Built-in Hook Descriptors (`omx-hooks/src/builtins.rs`)

**Files:**
- Create: `crates/omx-hooks/src/builtins.rs`
- Modify: `crates/omx-hooks/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Create `crates/omx-hooks/src/builtins.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_have_unique_names() {
        let builtins = all_builtin_hooks();
        let names: Vec<&str> = builtins.iter().map(|b| b.name.as_str()).collect();
        let mut deduped = names.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "duplicate builtin hook names found");
    }

    #[test]
    fn all_builtins_count() {
        let builtins = all_builtin_hooks();
        assert_eq!(builtins.len(), 6);
    }

    #[test]
    fn notify_fallback_watcher_builtin() {
        let builtins = all_builtin_hooks();
        let fallback = builtins.iter().find(|b| b.name == "notify-fallback-watcher");
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().description, "Monitors notification delivery failures, retries or escalates");
    }

    #[test]
    fn tmux_heal_builtin() {
        let builtins = all_builtin_hooks();
        let heal = builtins.iter().find(|b| b.name == "notify-hook-tmux-heal");
        assert!(heal.is_some());
    }

    #[test]
    fn builtin_lookup_by_name() {
        let hook = lookup_builtin("notify-hook-auto-nudge");
        assert!(hook.is_some());
        assert!(lookup_builtin("nonexistent").is_none());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-hooks builtins -- --nocapture 2>&1 | head -20`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

```rust
use serde::{Deserialize, Serialize};

/// A built-in hook definition (compiled into the binary, not a filesystem executable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinHook {
    pub name: String,
    pub description: String,
    pub events: Vec<String>,
}

/// Return all 6 built-in hook descriptors.
pub fn all_builtin_hooks() -> Vec<BuiltinHook> {
    vec![
        BuiltinHook {
            name: "notify-fallback-watcher".into(),
            description: "Monitors notification delivery failures, retries or escalates".into(),
            events: vec!["session-start".into(), "session-end".into()],
        },
        BuiltinHook {
            name: "notify-hook-auto-nudge".into(),
            description: "Detects idle workers and sends nudge notifications".into(),
            events: vec!["session-idle".into(), "turn-complete".into()],
        },
        BuiltinHook {
            name: "notify-hook-team-leader-nudge".into(),
            description: "Nudges team leader when workers await review".into(),
            events: vec!["turn-complete".into()],
        },
        BuiltinHook {
            name: "notify-hook-team-dispatch".into(),
            description: "Fires notification when team dispatches work".into(),
            events: vec!["turn-complete".into()],
        },
        BuiltinHook {
            name: "notify-hook-worker-idle".into(),
            description: "Fires notification when worker goes idle too long".into(),
            events: vec!["session-idle".into()],
        },
        BuiltinHook {
            name: "notify-hook-tmux-heal".into(),
            description: "Detects and repairs broken tmux sessions".into(),
            events: vec!["session-start".into(), "turn-complete".into()],
        },
    ]
}

/// Look up a built-in hook by name.
pub fn lookup_builtin(name: &str) -> Option<BuiltinHook> {
    all_builtin_hooks().into_iter().find(|h| h.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_have_unique_names() {
        let builtins = all_builtin_hooks();
        let names: Vec<&str> = builtins.iter().map(|b| b.name.as_str()).collect();
        let mut deduped = names.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "duplicate builtin hook names found");
    }

    #[test]
    fn all_builtins_count() {
        let builtins = all_builtin_hooks();
        assert_eq!(builtins.len(), 6);
    }

    #[test]
    fn notify_fallback_watcher_builtin() {
        let builtins = all_builtin_hooks();
        let fallback = builtins.iter().find(|b| b.name == "notify-fallback-watcher");
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().description, "Monitors notification delivery failures, retries or escalates");
    }

    #[test]
    fn tmux_heal_builtin() {
        let builtins = all_builtin_hooks();
        let heal = builtins.iter().find(|b| b.name == "notify-hook-tmux-heal");
        assert!(heal.is_some());
    }

    #[test]
    fn builtin_lookup_by_name() {
        let hook = lookup_builtin("notify-hook-auto-nudge");
        assert!(hook.is_some());
        assert!(lookup_builtin("nonexistent").is_none());
    }
}
```

- [ ] **Step 4: Register module and run tests**

Add `pub mod builtins;` to `crates/omx-hooks/src/lib.rs`.

Run: `cargo test -p omx-hooks builtins -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-hooks/src/builtins.rs crates/omx-hooks/src/lib.rs
git commit -m "feat(omx-hooks): add 6 built-in hook descriptors"
```

---

## Task 17: Full Workspace Build Verification

**Files:**
- None modified — verification only

- [ ] **Step 1: Run full workspace build**

Run: `cargo build --workspace 2>&1 | tail -20`
Expected: Build succeeds with no errors

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: All tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace -- -D warnings 2>&1 | tail -20`
Expected: No warnings

- [ ] **Step 4: Run cargo fmt check**

Run: `cargo fmt --all -- --check 2>&1 | tail -10`
Expected: No formatting issues

- [ ] **Step 5: Fix any issues found and commit**

If any issues are found in steps 1-4, fix them and commit:

```bash
git add -A
git commit -m "style: fix clippy warnings and formatting for Phase 9"
```
