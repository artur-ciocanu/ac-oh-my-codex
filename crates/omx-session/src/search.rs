//! Full-text search over session transcripts.

use crate::{SearchResult, SessionTurn};

/// Search turns for a query string (case-insensitive substring match).
/// Returns matching turns ranked by simple relevance (exact match > partial).
pub fn search_turns(session_id: &str, turns: &[SessionTurn], query: &str) -> Vec<SearchResult> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for turn in turns {
        let content_lower = turn.content.to_lowercase();
        if content_lower.contains(&query_lower) {
            let count = content_lower.matches(&query_lower).count();
            let score = count as f64 / content_lower.len().max(1) as f64;
            let snippet = make_snippet(&turn.content, query, 80);
            results.push(SearchResult {
                session_id: session_id.to_string(),
                turn_number: turn.turn_number,
                snippet,
                score,
            });
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

fn make_snippet(content: &str, query: &str, max_len: usize) -> String {
    let lower = content.to_lowercase();
    let query_lower = query.to_lowercase();
    if let Some(pos) = lower.find(&query_lower) {
        let start = pos.saturating_sub(max_len / 2);
        let end = (pos + query.len() + max_len / 2).min(content.len());
        let slice = &content[start..end];
        if start > 0 || end < content.len() {
            format!("...{}...", slice.trim())
        } else {
            slice.to_string()
        }
    } else {
        content.chars().take(max_len).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_turn(num: u32, content: &str) -> SessionTurn {
        SessionTurn {
            turn_number: num,
            timestamp: Utc::now(),
            role: "assistant".into(),
            content: content.into(),
            tokens: 10,
        }
    }

    #[test]
    fn search_finds_matching_turns() {
        let turns = vec![
            make_turn(1, "Hello world"),
            make_turn(2, "Fix the bug in auth module"),
            make_turn(3, "The authentication is broken"),
        ];
        let results = search_turns("sess-1", &turns, "auth");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_is_case_insensitive() {
        let turns = vec![make_turn(1, "ERROR: connection refused")];
        let results = search_turns("sess-1", &turns, "error");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let turns = vec![make_turn(1, "Hello world")];
        let results = search_turns("sess-1", &turns, "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn search_returns_sorted_by_score() {
        let turns = vec![
            make_turn(1, "auth auth auth is critical"),
            make_turn(2, "the auth module handles authentication"),
        ];
        let results = search_turns("sess-1", &turns, "auth");
        assert_eq!(results.len(), 2);
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn snippet_includes_context_around_match() {
        let snippet = make_snippet(
            "The quick brown fox jumps over the lazy dog near the authentication module",
            "authentication",
            80,
        );
        assert!(snippet.contains("authentication"));
    }

    #[test]
    fn snippet_short_content_returns_full_string() {
        let snippet = make_snippet("auth error", "auth", 80);
        assert_eq!(snippet, "auth error");
    }
}
