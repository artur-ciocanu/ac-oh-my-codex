//! Session tracking and history.

pub mod search;
pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Completed,
    Crashed,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Completed => write!(f, "completed"),
            Self::Crashed => write!(f, "crashed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub mode: String,
    pub turns: u32,
    pub tokens_used: u64,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub turn_number: u32,
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub content: String,
    pub tokens: u64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub session_id: String,
    pub turn_number: u32,
    pub snippet: String,
    pub score: f64,
}

pub use store::FileSessionStore;
