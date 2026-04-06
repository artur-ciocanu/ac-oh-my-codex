//! Distributed file locks per worker to prevent double-dispatch.
//!
//! Uses `fs2` exclusive file locks so that only one process at a time
//! can act on behalf of a given worker name.

use fs2::FileExt;
use omx_types::OmxError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// An fs2-based exclusive file lock scoped to a single worker.
///
/// The lock is held for the lifetime of this struct and automatically
/// released on [`Drop`].
pub struct TeamFileLock {
    /// The open file handle whose advisory lock we hold.
    file: Option<fs::File>,
    /// Full path to the `.lock` file on disk.
    path: PathBuf,
}

impl TeamFileLock {
    /// Acquire an exclusive lock for `worker_name`, blocking until available.
    ///
    /// Creates `lock_dir` if it does not exist. The lock file will be
    /// `<lock_dir>/<worker_name>.lock`.
    pub fn acquire(lock_dir: &Path, worker_name: &str) -> Result<Self, OmxError> {
        let path = lock_dir.join(format!("{worker_name}.lock"));
        fs::create_dir_all(lock_dir)?;
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        file.lock_exclusive()?;
        Ok(Self {
            file: Some(file),
            path,
        })
    }

    /// Try to acquire the lock without blocking.
    ///
    /// Returns an error if the lock is already held by another process/thread.
    pub fn try_acquire(lock_dir: &Path, worker_name: &str) -> Result<Self, OmxError> {
        let path = lock_dir.join(format!("{worker_name}.lock"));
        fs::create_dir_all(lock_dir)?;
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        file.try_lock_exclusive().map_err(|e| {
            OmxError::Team(format!("lock already held for worker '{worker_name}': {e}"))
        })?;
        Ok(Self {
            file: Some(file),
            path,
        })
    }

    /// Return the path to the underlying lock file.
    pub fn lock_path(&self) -> &Path {
        &self.path
    }

    /// Explicitly release the lock and close the file handle.
    pub fn release(mut self) -> Result<(), OmxError> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<(), OmxError> {
        if let Some(file) = self.file.take() {
            file.unlock()?;
        }
        Ok(())
    }
}

impl Drop for TeamFileLock {
    fn drop(&mut self) {
        // Best-effort release; ignore errors during drop.
        let _ = self.release_inner();
    }
}

/// Returns `true` if the lock file at `lock_path` exists and is older than `max_age`.
pub fn is_stale_lock(lock_path: &Path, max_age: Duration) -> bool {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let Ok(elapsed) = modified.elapsed() else {
        return false;
    };
    elapsed > max_age
}

/// Remove the lock file at `lock_path` if it is older than `max_age`.
///
/// Returns `Ok(true)` if the file was removed, `Ok(false)` if it was not stale
/// (or did not exist), and `Err` on I/O failure during removal.
pub fn recover_stale_lock(lock_path: &Path, max_age: Duration) -> Result<bool, OmxError> {
    if !lock_path.exists() {
        return Ok(false);
    }
    if is_stale_lock(lock_path, max_age) {
        fs::remove_file(lock_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_and_release_works() {
        let dir = TempDir::new().unwrap();
        let lock = TeamFileLock::acquire(dir.path(), "worker-1").unwrap();
        assert!(lock.lock_path().exists());
        lock.release().unwrap();
    }

    #[test]
    fn try_acquire_fails_when_held() {
        let dir = TempDir::new().unwrap();
        let _lock = TeamFileLock::acquire(dir.path(), "worker-1").unwrap();
        let result = TeamFileLock::try_acquire(dir.path(), "worker-1");
        assert!(
            result.is_err(),
            "expected try_acquire to fail while lock is held"
        );
    }

    #[test]
    fn separate_workers_get_separate_locks() {
        let dir = TempDir::new().unwrap();
        let lock_a = TeamFileLock::acquire(dir.path(), "alpha").unwrap();
        let lock_b = TeamFileLock::acquire(dir.path(), "beta").unwrap();
        assert_ne!(lock_a.lock_path(), lock_b.lock_path());
        lock_a.release().unwrap();
        lock_b.release().unwrap();
    }

    #[test]
    fn lock_path_includes_worker_name() {
        let dir = TempDir::new().unwrap();
        let lock = TeamFileLock::acquire(dir.path(), "my-worker").unwrap();
        let path = lock.lock_path().to_owned();
        assert!(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("my-worker"),
            "lock path should contain worker name"
        );
        assert!(
            path.to_str().unwrap().ends_with("my-worker.lock"),
            "lock file should be named <worker>.lock"
        );
        lock.release().unwrap();
    }

    #[test]
    fn stale_lock_detection_by_age() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("old.lock");
        fs::write(&lock_path, b"").unwrap();

        // With a zero-duration max age the file is immediately stale.
        assert!(is_stale_lock(&lock_path, Duration::ZERO));

        // With a very large max age it should not be stale.
        assert!(!is_stale_lock(&lock_path, Duration::from_secs(3600)));
    }

    #[test]
    fn recover_stale_lock_only_removes_old_files() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("stale.lock");
        fs::write(&lock_path, b"").unwrap();

        // Should NOT remove a fresh file (huge max_age).
        let removed = recover_stale_lock(&lock_path, Duration::from_secs(3600)).unwrap();
        assert!(!removed, "should not remove a fresh lock");
        assert!(lock_path.exists());

        // Should remove when max_age is zero.
        let removed = recover_stale_lock(&lock_path, Duration::ZERO).unwrap();
        assert!(removed, "should remove a stale lock");
        assert!(!lock_path.exists());
    }

    #[test]
    fn drop_releases_the_lock() {
        let dir = TempDir::new().unwrap();
        {
            let _lock = TeamFileLock::acquire(dir.path(), "drop-test").unwrap();
            // lock is dropped here
        }
        // Re-acquiring should succeed after drop.
        let lock = TeamFileLock::try_acquire(dir.path(), "drop-test")
            .expect("should be able to re-acquire after drop");
        lock.release().unwrap();
    }

    #[test]
    fn recover_nonexistent_lock_returns_false() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("nonexistent.lock");
        let result = recover_stale_lock(&lock_path, Duration::ZERO).unwrap();
        assert!(!result);
    }
}
