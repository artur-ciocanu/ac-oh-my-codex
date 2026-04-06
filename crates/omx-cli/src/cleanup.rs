use std::path::Path;

#[derive(Debug, Default)]
pub struct CleanupReport {
    pub stale_locks_removed: u32,
    pub stale_sessions_removed: u32,
    pub stale_worktrees_removed: u32,
}

impl CleanupReport {
    pub fn total(&self) -> u32 {
        self.stale_locks_removed + self.stale_sessions_removed + self.stale_worktrees_removed
    }
}

pub fn remove_stale_locks(omx_dir: &Path) -> Result<u32, std::io::Error> {
    let mut count = 0;
    if !omx_dir.exists() {
        return Ok(0);
    }
    for entry in std::fs::read_dir(omx_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("lock") {
            std::fs::remove_file(&path)?;
            count += 1;
        }
    }
    Ok(count)
}

pub fn remove_stale_sessions(
    sessions_dir: &Path,
    max_age_days: u64,
) -> Result<u32, std::io::Error> {
    let mut count = 0;
    if !sessions_dir.exists() {
        return Ok(0);
    }
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_days * 86400))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    for entry in std::fs::read_dir(sessions_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    std::fs::remove_dir_all(entry.path())?;
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

pub fn run_cleanup(codex_home: &Path) -> Result<CleanupReport, std::io::Error> {
    let omx_dir = codex_home.join(".omx");
    let sessions_dir = omx_dir.join("sessions");

    let stale_locks_removed = remove_stale_locks(&omx_dir)?;
    let stale_sessions_removed = remove_stale_sessions(&sessions_dir, 30)?;

    Ok(CleanupReport {
        stale_locks_removed,
        stale_sessions_removed,
        stale_worktrees_removed: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn remove_stale_locks_removes_lock_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("session.lock"), "").unwrap();
        std::fs::write(dir.path().join("worker.lock"), "").unwrap();
        std::fs::write(dir.path().join("data.json"), "{}").unwrap();

        let count = remove_stale_locks(dir.path()).unwrap();
        assert_eq!(count, 2);
        assert!(!dir.path().join("session.lock").exists());
        assert!(!dir.path().join("worker.lock").exists());
        assert!(dir.path().join("data.json").exists());
    }

    #[test]
    fn remove_stale_locks_nonexistent_dir() {
        let count = remove_stale_locks(Path::new("/nonexistent/path")).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn remove_stale_sessions_removes_old_dirs() {
        let dir = TempDir::new().unwrap();
        let old_session = dir.path().join("sess-old");
        std::fs::create_dir(&old_session).unwrap();
        let count = remove_stale_sessions(dir.path(), 0).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn cleanup_report_total() {
        let report = CleanupReport {
            stale_locks_removed: 2,
            stale_sessions_removed: 1,
            stale_worktrees_removed: 0,
        };
        assert_eq!(report.total(), 3);
    }

    #[test]
    fn run_cleanup_on_empty_dir() {
        let dir = TempDir::new().unwrap();
        let report = run_cleanup(dir.path()).unwrap();
        assert_eq!(report.total(), 0);
    }
}
