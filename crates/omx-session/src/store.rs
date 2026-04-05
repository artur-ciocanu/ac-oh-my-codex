//! File-based session store implementation.

use crate::{Session, SessionStatus, SessionTurn};
use chrono::Utc;
use omx_types::OmxError;
use std::path::PathBuf;
use tokio::fs;

pub struct FileSessionStore {
    root: PathBuf,
}

impl FileSessionStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    fn session_dir(&self, id: &str) -> PathBuf {
        self.sessions_dir().join(id)
    }

    fn session_meta_path(&self, id: &str) -> PathBuf {
        self.session_dir(id).join("meta.json")
    }

    fn transcript_path(&self, id: &str) -> PathBuf {
        self.session_dir(id).join("transcript.jsonl")
    }

    pub async fn create(&self, mode: &str) -> Result<Session, OmxError> {
        let id = ulid::Ulid::new().to_string().to_lowercase();
        let session = Session {
            id: id.clone(),
            started_at: Utc::now(),
            ended_at: None,
            mode: mode.to_string(),
            turns: 0,
            tokens_used: 0,
            status: SessionStatus::Active,
        };

        let dir = self.session_dir(&id);
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| OmxError::Session(format!("failed to create session dir: {e}")))?;

        let json = serde_json::to_string_pretty(&session)
            .map_err(|e| OmxError::Session(format!("failed to serialize session: {e}")))?;
        fs::write(self.session_meta_path(&id), json)
            .await
            .map_err(|e| OmxError::Session(format!("failed to write session meta: {e}")))?;

        Ok(session)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Session>, OmxError> {
        let path = self.session_meta_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&path)
            .await
            .map_err(|e| OmxError::Session(format!("failed to read session: {e}")))?;
        let session: Session = serde_json::from_str(&contents)
            .map_err(|e| OmxError::Session(format!("failed to parse session: {e}")))?;
        Ok(Some(session))
    }

    pub async fn update(&self, session: &Session) -> Result<(), OmxError> {
        let json = serde_json::to_string_pretty(session)
            .map_err(|e| OmxError::Session(format!("failed to serialize session: {e}")))?;
        fs::write(self.session_meta_path(&session.id), json)
            .await
            .map_err(|e| OmxError::Session(format!("failed to write session: {e}")))?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Session>, OmxError> {
        let dir = self.sessions_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&dir)
            .await
            .map_err(|e| OmxError::Session(format!("failed to read sessions dir: {e}")))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| OmxError::Session(format!("failed to read entry: {e}")))?
        {
            let meta_path = entry.path().join("meta.json");
            if meta_path.exists() {
                let contents = fs::read_to_string(&meta_path)
                    .await
                    .map_err(|e| OmxError::Session(format!("failed to read session meta: {e}")))?;
                if let Ok(session) = serde_json::from_str::<Session>(&contents) {
                    sessions.push(session);
                }
            }
        }
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(sessions)
    }

    pub async fn append_turn(&self, session_id: &str, turn: &SessionTurn) -> Result<(), OmxError> {
        let path = self.transcript_path(session_id);
        let mut line = serde_json::to_string(turn)
            .map_err(|e| OmxError::Session(format!("failed to serialize turn: {e}")))?;
        line.push('\n');

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| OmxError::Session(format!("failed to open transcript: {e}")))?;
        file.write_all(line.as_bytes())
            .await
            .map_err(|e| OmxError::Session(format!("failed to write turn: {e}")))?;

        Ok(())
    }

    pub async fn read_transcript(&self, session_id: &str) -> Result<Vec<SessionTurn>, OmxError> {
        let path = self.transcript_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path)
            .await
            .map_err(|e| OmxError::Session(format!("failed to read transcript: {e}")))?;
        let mut turns = Vec::new();
        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let turn: SessionTurn = serde_json::from_str(line)
                .map_err(|e| OmxError::Session(format!("failed to parse turn: {e}")))?;
            turns.push(turn);
        }
        Ok(turns)
    }

    pub async fn complete(&self, session_id: &str) -> Result<(), OmxError> {
        let mut session = self
            .get(session_id)
            .await?
            .ok_or_else(|| OmxError::Session(format!("session not found: {session_id}")))?;
        session.status = SessionStatus::Completed;
        session.ended_at = Some(Utc::now());
        self.update(&session).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn create_session_generates_ulid_and_writes_meta() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        let session = store.create("autopilot").await.unwrap();
        assert!(!session.id.is_empty());
        assert_eq!(session.mode, "autopilot");
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.turns, 0);
        assert_eq!(session.tokens_used, 0);
    }

    #[tokio::test]
    async fn get_returns_created_session() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        let created = store.create("ralph").await.unwrap();
        let fetched = store.get(&created.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().mode, "ralph");
    }

    #[tokio::test]
    async fn get_returns_none_for_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        assert!(store.get("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_returns_sessions_sorted_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        let s1 = store.create("mode1").await.unwrap();
        let s2 = store.create("mode2").await.unwrap();
        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, s2.id);
        assert_eq!(all[1].id, s1.id);
    }

    #[tokio::test]
    async fn list_returns_empty_when_no_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        assert!(store.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn append_and_read_transcript() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        let session = store.create("team").await.unwrap();

        let turn1 = SessionTurn {
            turn_number: 1,
            timestamp: Utc::now(),
            role: "user".into(),
            content: "hello".into(),
            tokens: 5,
        };
        let turn2 = SessionTurn {
            turn_number: 2,
            timestamp: Utc::now(),
            role: "assistant".into(),
            content: "hi there".into(),
            tokens: 10,
        };

        store.append_turn(&session.id, &turn1).await.unwrap();
        store.append_turn(&session.id, &turn2).await.unwrap();

        let turns = store.read_transcript(&session.id).await.unwrap();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].content, "hello");
        assert_eq!(turns[1].content, "hi there");
    }

    #[tokio::test]
    async fn read_transcript_returns_empty_for_new_session() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        let session = store.create("autopilot").await.unwrap();
        let turns = store.read_transcript(&session.id).await.unwrap();
        assert!(turns.is_empty());
    }

    #[tokio::test]
    async fn complete_marks_session_completed() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        let session = store.create("ralph").await.unwrap();
        store.complete(&session.id).await.unwrap();
        let fetched = store.get(&session.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, SessionStatus::Completed);
        assert!(fetched.ended_at.is_some());
    }

    #[tokio::test]
    async fn update_persists_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        let mut session = store.create("team").await.unwrap();
        session.turns = 42;
        session.tokens_used = 5000;
        store.update(&session).await.unwrap();
        let fetched = store.get(&session.id).await.unwrap().unwrap();
        assert_eq!(fetched.turns, 42);
        assert_eq!(fetched.tokens_used, 5000);
    }
}
