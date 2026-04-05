use std::path::{Path, PathBuf};

use omx_types::OmxError;
use serde::de::DeserializeOwned;
use serde::Serialize;

// ---------------------------------------------------------------------------
// StateStore trait (spec section 4.4)
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
pub trait StateStore: Send + Sync {
    async fn read<T: DeserializeOwned + Send>(&self, path: &Path) -> Result<Option<T>, OmxError>;
    async fn write<T: Serialize + Send + Sync>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<(), OmxError>;
    async fn delete(&self, path: &Path) -> Result<(), OmxError>;
    async fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, OmxError>;
    async fn append_jsonl<T: Serialize + Send + Sync>(
        &self,
        path: &Path,
        entry: &T,
    ) -> Result<(), OmxError>;
}

// ---------------------------------------------------------------------------
// FileStateStore (atomic rename + fs2 locking)
// ---------------------------------------------------------------------------

pub struct FileStateStore {
    root: PathBuf,
}

impl FileStateStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative path against the store root.
    pub fn resolve(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }
}

#[async_trait::async_trait]
impl StateStore for FileStateStore {
    async fn read<T: DeserializeOwned + Send>(&self, path: &Path) -> Result<Option<T>, OmxError> {
        let full_path = self.resolve(path);
        if !full_path.exists() {
            return Ok(None);
        }

        let data = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, OmxError> {
            use fs2::FileExt;
            let file = std::fs::File::open(&full_path)?;
            file.lock_shared()
                .map_err(|e| OmxError::State(format!("failed to acquire shared lock: {e}")))?;
            let data = std::fs::read(&full_path)?;
            file.unlock()
                .map_err(|e| OmxError::State(format!("failed to release lock: {e}")))?;
            Ok(data)
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))??;

        let value: T = serde_json::from_slice(&data)?;
        Ok(Some(value))
    }

    async fn write<T: Serialize + Send + Sync>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<(), OmxError> {
        let full_path = self.resolve(path);
        let data = serde_json::to_vec_pretty(value)?;

        tokio::task::spawn_blocking(move || -> Result<(), OmxError> {
            use fs2::FileExt;

            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let dir = full_path.parent().unwrap_or(Path::new("."));
            let mut tmp = tempfile::NamedTempFile::new_in(dir)
                .map_err(|e| OmxError::State(format!("failed to create temp file: {e}")))?;

            tmp.as_file().lock_exclusive()
                .map_err(|e| OmxError::State(format!("failed to acquire exclusive lock: {e}")))?;

            std::io::Write::write_all(&mut tmp, &data)?;

            tmp.persist(&full_path)
                .map_err(|e| OmxError::State(format!("failed to persist temp file: {e}")))?;

            Ok(())
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }

    async fn delete(&self, path: &Path) -> Result<(), OmxError> {
        let full_path = self.resolve(path);

        tokio::task::spawn_blocking(move || -> Result<(), OmxError> {
            match std::fs::remove_file(&full_path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(OmxError::Io(e)),
            }
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }

    async fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, OmxError> {
        let full_dir = self.resolve(dir);

        tokio::task::spawn_blocking(move || -> Result<Vec<PathBuf>, OmxError> {
            if !full_dir.exists() {
                return Ok(Vec::new());
            }
            let mut entries = Vec::new();
            for entry in std::fs::read_dir(&full_dir)? {
                let entry = entry?;
                entries.push(entry.path());
            }
            entries.sort();
            Ok(entries)
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }

    async fn append_jsonl<T: Serialize + Send + Sync>(
        &self,
        path: &Path,
        entry: &T,
    ) -> Result<(), OmxError> {
        let full_path = self.resolve(path);
        let mut line = serde_json::to_string(entry)?;
        line.push('\n');

        tokio::task::spawn_blocking(move || -> Result<(), OmxError> {
            use fs2::FileExt;
            use std::io::Write;

            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&full_path)?;

            file.lock_exclusive()
                .map_err(|e| OmxError::State(format!("failed to acquire exclusive lock: {e}")))?;

            let mut writer = std::io::BufWriter::new(&file);
            writer.write_all(line.as_bytes())?;
            writer.flush()?;

            file.unlock()
                .map_err(|e| OmxError::State(format!("failed to release lock: {e}")))?;

            Ok(())
        })
        .await
        .map_err(|e| OmxError::State(format!("blocking task failed: {e}")))?
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn file_state_store_resolves_paths() {
        let store = FileStateStore::new(PathBuf::from("/tmp/omx-state"));
        assert_eq!(
            store.resolve(Path::new("team/config.json")),
            PathBuf::from("/tmp/omx-state/team/config.json")
        );
    }

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        let data = TestData { name: "hello".into(), value: 42 };
        store.write(Path::new("test.json"), &data).await.unwrap();
        let read_back: Option<TestData> = store.read(Path::new("test.json")).await.unwrap();
        assert_eq!(read_back, Some(data));
    }

    #[tokio::test]
    async fn read_nonexistent_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        let result: Option<TestData> = store.read(Path::new("nope.json")).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn write_creates_parent_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        let data = TestData { name: "nested".into(), value: 1 };
        store.write(Path::new("a/b/c/data.json"), &data).await.unwrap();
        let read_back: Option<TestData> = store.read(Path::new("a/b/c/data.json")).await.unwrap();
        assert_eq!(read_back, Some(data));
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        let data = TestData { name: "bye".into(), value: 0 };
        store.write(Path::new("del.json"), &data).await.unwrap();
        store.delete(Path::new("del.json")).await.unwrap();
        let result: Option<TestData> = store.read(Path::new("del.json")).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn delete_nonexistent_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        store.delete(Path::new("nope.json")).await.unwrap();
    }

    #[tokio::test]
    async fn list_returns_sorted_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        let data = TestData { name: "x".into(), value: 0 };
        store.write(Path::new("dir/b.json"), &data).await.unwrap();
        store.write(Path::new("dir/a.json"), &data).await.unwrap();
        store.write(Path::new("dir/c.json"), &data).await.unwrap();
        let entries = store.list(Path::new("dir")).await.unwrap();
        let names: Vec<&str> = entries.iter().filter_map(|p| p.file_name().and_then(|n| n.to_str())).collect();
        assert_eq!(names, vec!["a.json", "b.json", "c.json"]);
    }

    #[tokio::test]
    async fn list_nonexistent_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        let entries = store.list(Path::new("missing")).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn append_jsonl_preserves_existing_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path().to_path_buf());
        let entry1 = TestData { name: "first".into(), value: 1 };
        let entry2 = TestData { name: "second".into(), value: 2 };
        let entry3 = TestData { name: "third".into(), value: 3 };
        store.append_jsonl(Path::new("log.jsonl"), &entry1).await.unwrap();
        store.append_jsonl(Path::new("log.jsonl"), &entry2).await.unwrap();
        store.append_jsonl(Path::new("log.jsonl"), &entry3).await.unwrap();
        let full_path = store.resolve(Path::new("log.jsonl"));
        let contents = std::fs::read_to_string(full_path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 3);
        let parsed1: TestData = serde_json::from_str(lines[0]).unwrap();
        let parsed2: TestData = serde_json::from_str(lines[1]).unwrap();
        let parsed3: TestData = serde_json::from_str(lines[2]).unwrap();
        assert_eq!(parsed1, entry1);
        assert_eq!(parsed2, entry2);
        assert_eq!(parsed3, entry3);
    }

    #[tokio::test]
    async fn concurrent_writes_do_not_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let store = std::sync::Arc::new(FileStateStore::new(tmp.path().to_path_buf()));
        let mut handles = Vec::new();
        for i in 0..20 {
            let store = store.clone();
            handles.push(tokio::spawn(async move {
                let data = TestData { name: format!("writer-{i}"), value: i };
                store.write(Path::new("shared.json"), &data).await.unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        let result: Option<TestData> = store.read(Path::new("shared.json")).await.unwrap();
        assert!(result.is_some());
        let data = result.unwrap();
        assert!(data.name.starts_with("writer-"));
    }
}
