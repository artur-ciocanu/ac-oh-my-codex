//! Iterative research engine with git worktrees and evaluator contracts.

use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const AUTORESEARCH_RESULTS_HEADER: &str =
    "iteration\tdecision\tpass\tscore\tkept_commit\ttimestamp\n";

// ---------------------------------------------------------------------------
// Keep policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoresearchKeepPolicy {
    ScoreImprovement,
    PassOnly,
}

impl std::fmt::Display for AutoresearchKeepPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScoreImprovement => write!(f, "score_improvement"),
            Self::PassOnly => write!(f, "pass_only"),
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluator contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AutoresearchEvaluatorContract {
    pub command: String,
    pub format: String,
    pub keep_policy: AutoresearchKeepPolicy,
}

// ---------------------------------------------------------------------------
// Parsed sandbox contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ParsedSandboxContract {
    pub frontmatter: HashMap<String, String>,
    pub evaluator: AutoresearchEvaluatorContract,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Evaluator result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchEvaluatorResult {
    pub pass: bool,
    pub score: Option<f64>,
}

// ---------------------------------------------------------------------------
// Mission contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AutoresearchMissionContract {
    pub mission_dir: PathBuf,
    pub repo_root: PathBuf,
    pub mission_file: PathBuf,
    pub sandbox_file: PathBuf,
    pub mission_relative_dir: String,
    pub mission_content: String,
    pub sandbox_content: String,
    pub sandbox: ParsedSandboxContract,
    pub mission_slug: String,
}

// ---------------------------------------------------------------------------
// Status enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoresearchCandidateStatus {
    Candidate,
    Noop,
    Abort,
    Interrupted,
}

impl std::fmt::Display for AutoresearchCandidateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Candidate => write!(f, "candidate"),
            Self::Noop => write!(f, "noop"),
            Self::Abort => write!(f, "abort"),
            Self::Interrupted => write!(f, "interrupted"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoresearchDecisionStatus {
    Baseline,
    Keep,
    Discard,
    Ambiguous,
    Noop,
    Abort,
    Interrupted,
    Error,
}

impl std::fmt::Display for AutoresearchDecisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Baseline => write!(f, "baseline"),
            Self::Keep => write!(f, "keep"),
            Self::Discard => write!(f, "discard"),
            Self::Ambiguous => write!(f, "ambiguous"),
            Self::Noop => write!(f, "noop"),
            Self::Abort => write!(f, "abort"),
            Self::Interrupted => write!(f, "interrupted"),
            Self::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoresearchRunStatus {
    Running,
    Stopped,
    Completed,
    Failed,
}

impl std::fmt::Display for AutoresearchRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Record types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchCandidateArtifact {
    pub status: AutoresearchCandidateStatus,
    pub candidate_commit: Option<String>,
    pub base_commit: Option<String>,
    pub description: Option<String>,
    pub notes: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchEvaluationRecord {
    pub command: String,
    pub ran_at: String,
    pub status: String,
    pub pass: Option<bool>,
    pub score: Option<f64>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchLedgerEntry {
    pub iteration: u32,
    pub kind: String,
    pub decision: AutoresearchDecisionStatus,
    pub decision_reason: String,
    pub candidate_status: AutoresearchCandidateStatus,
    pub base_commit: Option<String>,
    pub candidate_commit: Option<String>,
    pub kept_commit: Option<String>,
    pub keep_policy: String,
    pub evaluator: Option<AutoresearchEvaluationRecord>,
    pub created_at: String,
    pub notes: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoresearchRunManifest {
    pub schema_version: u32,
    pub run_id: String,
    pub run_tag: String,
    pub run_dir: PathBuf,
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub mission_slug: String,
    pub status: AutoresearchRunStatus,
    pub iteration: u32,
    pub baseline_pass: Option<bool>,
    pub baseline_score: Option<f64>,
    pub last_kept_commit: Option<String>,
    pub last_kept_score: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct PreparedAutoresearchRuntime {
    pub run_id: String,
    pub run_tag: String,
    pub run_dir: PathBuf,
    pub instructions_file: PathBuf,
    pub manifest_file: PathBuf,
    pub ledger_file: PathBuf,
    pub latest_evaluator_file: PathBuf,
    pub results_file: PathBuf,
    pub state_file: PathBuf,
    pub candidate_file: PathBuf,
    pub repo_root: PathBuf,
    pub worktree_path: PathBuf,
    pub task_description: String,
}

// ---------------------------------------------------------------------------
// Slugify
// ---------------------------------------------------------------------------

pub fn slugify_mission_name(value: &str) -> String {
    let lower = value.to_lowercase();
    let mut slug = String::with_capacity(lower.len());
    let mut prev_hyphen = false;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        return "mission".to_string();
    }
    let capped = if trimmed.len() > 48 {
        &trimmed[..48]
    } else {
        trimmed
    };
    capped.trim_end_matches('-').to_string()
}

// ---------------------------------------------------------------------------
// Run tag
// ---------------------------------------------------------------------------

pub fn build_run_tag() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

// ---------------------------------------------------------------------------
// Frontmatter parsing
// ---------------------------------------------------------------------------

fn extract_frontmatter(content: &str) -> Option<(&str, &str)> {
    if !content.starts_with("---\n") {
        return None;
    }
    let after_first = &content[4..];
    let end_pos = after_first.find("\n---\n")?;
    let frontmatter = &after_first[..end_pos];
    let body = &after_first[end_pos + 5..];
    Some((frontmatter, body))
}

fn parse_simple_yaml_frontmatter(yaml: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_section: Option<String> = None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check if this is a section header (key with no value, followed by indented keys)
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim();
                let value = trimmed[colon_pos + 1..].trim();
                if value.is_empty() {
                    current_section = Some(key.to_string());
                } else {
                    current_section = None;
                    let clean_value = value.trim_matches('"').trim_matches('\'');
                    map.insert(key.to_string(), clean_value.to_string());
                }
            }
        } else if let Some(ref section) = current_section {
            // Nested key
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim();
                let value = trimmed[colon_pos + 1..].trim();
                let clean_value = value.trim_matches('"').trim_matches('\'');
                let full_key = format!("{}.{}", section, key);
                map.insert(full_key, clean_value.to_string());
            }
        }
    }

    map
}

// ---------------------------------------------------------------------------
// Sandbox contract parsing
// ---------------------------------------------------------------------------

pub fn parse_sandbox_contract(content: &str) -> Result<ParsedSandboxContract, OmxError> {
    let (fm_str, body) = extract_frontmatter(content)
        .ok_or_else(|| OmxError::Autoresearch("sandbox file has no frontmatter".into()))?;

    let frontmatter = parse_simple_yaml_frontmatter(fm_str);

    let command = frontmatter
        .get("evaluator.command")
        .cloned()
        .ok_or_else(|| OmxError::Autoresearch("evaluator.command is required".into()))?;

    let format = frontmatter
        .get("evaluator.format")
        .cloned()
        .unwrap_or_else(|| "json".to_string());

    if format != "json" {
        return Err(OmxError::Autoresearch(format!(
            "unsupported evaluator format: {}, only json is supported",
            format
        )));
    }

    let keep_policy = match frontmatter.get("evaluator.keep_policy").map(|s| s.as_str()) {
        Some("pass_only") => AutoresearchKeepPolicy::PassOnly,
        Some("score_improvement") | None => AutoresearchKeepPolicy::ScoreImprovement,
        Some(other) => {
            return Err(OmxError::Autoresearch(format!(
                "unknown keep_policy: {}",
                other
            )))
        }
    };

    Ok(ParsedSandboxContract {
        frontmatter,
        evaluator: AutoresearchEvaluatorContract {
            command,
            format,
            keep_policy,
        },
        body: body.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Evaluator result parsing
// ---------------------------------------------------------------------------

pub fn parse_evaluator_result(raw: &str) -> Result<AutoresearchEvaluatorResult, OmxError> {
    let val: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| OmxError::Autoresearch(format!("invalid evaluator JSON: {}", e)))?;

    let pass = val
        .get("pass")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| OmxError::Autoresearch("evaluator result missing boolean 'pass'".into()))?;

    let score = val.get("score").and_then(|v| v.as_f64());

    Ok(AutoresearchEvaluatorResult { pass, score })
}

// ---------------------------------------------------------------------------
// Decision logic
// ---------------------------------------------------------------------------

pub fn decide_outcome(
    manifest: &AutoresearchRunManifest,
    candidate: &AutoresearchCandidateArtifact,
    evaluation: &AutoresearchEvaluationRecord,
) -> (AutoresearchDecisionStatus, String) {
    // Check candidate status first
    match candidate.status {
        AutoresearchCandidateStatus::Abort => {
            return (
                AutoresearchDecisionStatus::Abort,
                "candidate aborted".into(),
            )
        }
        AutoresearchCandidateStatus::Noop => {
            return (
                AutoresearchDecisionStatus::Noop,
                "candidate produced no changes".into(),
            )
        }
        AutoresearchCandidateStatus::Interrupted => {
            return (
                AutoresearchDecisionStatus::Interrupted,
                "candidate was interrupted".into(),
            )
        }
        AutoresearchCandidateStatus::Candidate => {}
    }

    // Check for evaluation errors
    if evaluation.parse_error.is_some() {
        return (
            AutoresearchDecisionStatus::Error,
            "evaluator output could not be parsed".into(),
        );
    }

    let pass = match evaluation.pass {
        Some(p) => p,
        None => {
            return (
                AutoresearchDecisionStatus::Error,
                "evaluator did not return pass status".into(),
            )
        }
    };

    if !pass {
        return (
            AutoresearchDecisionStatus::Discard,
            "evaluator reported failure".into(),
        );
    }

    // pass=true from here
    match (evaluation.score, manifest.last_kept_score) {
        (Some(eval_score), Some(kept_score)) => {
            if eval_score > kept_score {
                (
                    AutoresearchDecisionStatus::Keep,
                    format!("score improved: {} -> {}", kept_score, eval_score),
                )
            } else {
                (
                    AutoresearchDecisionStatus::Discard,
                    format!(
                        "score did not improve: {} vs kept {}",
                        eval_score, kept_score
                    ),
                )
            }
        }
        (Some(eval_score), None) => (
            AutoresearchDecisionStatus::Keep,
            format!("first scored iteration: {}", eval_score),
        ),
        (None, Some(_)) => (
            AutoresearchDecisionStatus::Ambiguous,
            "pass but no score to compare against kept score".into(),
        ),
        (None, None) => (
            AutoresearchDecisionStatus::Keep,
            "pass with no scores tracked".into(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Instruction builder
// ---------------------------------------------------------------------------

pub fn trim_content(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut result = s[..max_len].to_string();
        result.push_str("...");
        result
    }
}

#[derive(Debug, Clone)]
pub struct InstructionContext {
    pub run_id: String,
    pub iteration: u32,
    pub worktree_path: PathBuf,
    pub candidate_file: PathBuf,
    pub last_kept_commit: Option<String>,
    pub last_kept_score: Option<f64>,
    pub trailing_noops: u32,
}

pub fn build_instructions(
    contract: &AutoresearchMissionContract,
    context: &InstructionContext,
) -> String {
    let mut out = String::new();

    out.push_str("# Autoresearch Instructions\n\n");
    out.push_str(&format!("**Run ID:** {}\n", context.run_id));
    out.push_str(&format!("**Iteration:** {}\n", context.iteration));
    out.push_str(&format!(
        "**Worktree:** {}\n",
        context.worktree_path.display()
    ));
    out.push_str(&format!(
        "**Evaluator command:** {}\n",
        contract.sandbox.evaluator.command
    ));
    out.push_str(&format!(
        "**Keep policy:** {}\n",
        contract.sandbox.evaluator.keep_policy
    ));

    if let Some(ref commit) = context.last_kept_commit {
        out.push_str(&format!("**Last kept commit:** {}\n", commit));
    }
    if let Some(score) = context.last_kept_score {
        out.push_str(&format!("**Last kept score:** {}\n", score));
    }

    out.push_str("\n## Mission\n\n");
    out.push_str(&contract.mission_content);
    out.push_str("\n\n## Sandbox\n\n");
    out.push_str(&contract.sandbox.body);

    out.push_str("\n\n## Candidate Output\n\n");
    out.push_str(&format!(
        "Write your candidate artifact as JSON to: `{}`\n",
        context.candidate_file.display()
    ));

    if context.trailing_noops > 0 {
        out.push_str(&format!(
            "\n**WARNING:** {} consecutive noop iterations detected. \
             Please make substantive changes to produce a meaningful candidate.\n",
            context.trailing_noops
        ));
    }

    out
}

// ---------------------------------------------------------------------------
// Git utilities
// ---------------------------------------------------------------------------

pub async fn git_rev_parse(worktree: &Path, rev: &str) -> Result<String, OmxError> {
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", rev])
        .current_dir(worktree)
        .output()
        .await
        .map_err(|e| OmxError::Autoresearch(format!("git rev-parse failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OmxError::Autoresearch(format!(
            "git rev-parse {} failed: {}",
            rev, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn git_status_porcelain(worktree: &Path) -> Result<String, OmxError> {
    let output = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree)
        .output()
        .await
        .map_err(|e| OmxError::Autoresearch(format!("git status failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(OmxError::Autoresearch(format!(
            "git status failed: {}",
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn assert_reset_safe_worktree(worktree: &Path) -> Result<(), OmxError> {
    let status = git_status_porcelain(worktree).await?;
    let allowed_patterns = ["results.tsv", "run.log", "node_modules", ".omx/"];

    for line in status.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // The file path starts after the two-character status prefix and a space
        let file_part = if line.len() > 3 { &line[3..] } else { line };
        let is_allowed = allowed_patterns
            .iter()
            .any(|pat| file_part.starts_with(pat) || file_part.contains(pat));
        if !is_allowed {
            return Err(OmxError::Autoresearch(format!(
                "worktree has uncommitted changes that are not safe to reset: {}",
                file_part
            )));
        }
    }

    Ok(())
}

pub async fn count_trailing_noops(ledger_file: &Path) -> Result<u32, OmxError> {
    let content = match tokio::fs::read_to_string(ledger_file).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => {
            return Err(OmxError::Autoresearch(format!(
                "failed to read ledger: {}",
                e
            )))
        }
    };

    let entries: Vec<AutoresearchLedgerEntry> = serde_json::from_str(&content)
        .map_err(|e| OmxError::Autoresearch(format!("failed to parse ledger: {}", e)))?;

    let mut count = 0u32;
    for entry in entries.iter().rev() {
        if entry.decision == AutoresearchDecisionStatus::Noop {
            count += 1;
        } else {
            break;
        }
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Task 7: Contract types and loading --

    #[test]
    fn keep_policy_serde_roundtrip() {
        let policy = AutoresearchKeepPolicy::ScoreImprovement;
        let json = serde_json::to_string(&policy).unwrap();
        assert_eq!(json, "\"score_improvement\"");
        let back: AutoresearchKeepPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back, policy);

        let policy2 = AutoresearchKeepPolicy::PassOnly;
        let json2 = serde_json::to_string(&policy2).unwrap();
        assert_eq!(json2, "\"pass_only\"");
        let back2: AutoresearchKeepPolicy = serde_json::from_str(&json2).unwrap();
        assert_eq!(back2, policy2);
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify_mission_name("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_strips_non_alphanumeric() {
        assert_eq!(slugify_mission_name("foo@bar#baz!qux"), "foo-bar-baz-qux");
    }

    #[test]
    fn slugify_caps_at_48() {
        let long = "a".repeat(100);
        let result = slugify_mission_name(&long);
        assert!(result.len() <= 48);
    }

    #[test]
    fn slugify_empty_returns_mission() {
        assert_eq!(slugify_mission_name(""), "mission");
        assert_eq!(slugify_mission_name("---"), "mission");
    }

    #[test]
    fn slugify_trims_hyphens() {
        assert_eq!(slugify_mission_name("--hello--"), "hello");
    }

    #[test]
    fn parse_sandbox_valid() {
        let content = "---\nevaluator:\n  command: cargo test\n  format: json\n---\nBody here\n";
        let contract = parse_sandbox_contract(content).unwrap();
        assert_eq!(contract.evaluator.command, "cargo test");
        assert_eq!(contract.evaluator.format, "json");
        assert_eq!(
            contract.evaluator.keep_policy,
            AutoresearchKeepPolicy::ScoreImprovement
        );
        assert_eq!(contract.body, "Body here\n");
    }

    #[test]
    fn parse_sandbox_defaults_to_score_improvement() {
        let content = "---\nevaluator:\n  command: ./test.sh\n  format: json\n---\nbody\n";
        let contract = parse_sandbox_contract(content).unwrap();
        assert_eq!(
            contract.evaluator.keep_policy,
            AutoresearchKeepPolicy::ScoreImprovement
        );
    }

    #[test]
    fn parse_sandbox_rejects_missing_command() {
        let content = "---\nevaluator:\n  format: json\n---\nbody\n";
        let err = parse_sandbox_contract(content).unwrap_err();
        assert!(err.to_string().contains("command"));
    }

    #[test]
    fn parse_sandbox_rejects_non_json_format() {
        let content = "---\nevaluator:\n  command: test\n  format: yaml\n---\nbody\n";
        let err = parse_sandbox_contract(content).unwrap_err();
        assert!(err.to_string().contains("json"));
    }

    #[test]
    fn parse_sandbox_rejects_no_frontmatter() {
        let content = "No frontmatter here\n";
        let err = parse_sandbox_contract(content).unwrap_err();
        assert!(err.to_string().contains("frontmatter"));
    }

    #[test]
    fn parse_evaluator_result_valid() {
        let raw = r#"{"pass": true, "score": 0.95}"#;
        let result = parse_evaluator_result(raw).unwrap();
        assert!(result.pass);
        assert_eq!(result.score, Some(0.95));
    }

    #[test]
    fn parse_evaluator_result_without_score() {
        let raw = r#"{"pass": false}"#;
        let result = parse_evaluator_result(raw).unwrap();
        assert!(!result.pass);
        assert_eq!(result.score, None);
    }

    #[test]
    fn parse_evaluator_result_rejects_missing_pass() {
        let raw = r#"{"score": 0.5}"#;
        let err = parse_evaluator_result(raw).unwrap_err();
        assert!(err.to_string().contains("pass"));
    }

    #[test]
    fn parse_evaluator_result_rejects_invalid_json() {
        let raw = "not json";
        let err = parse_evaluator_result(raw).unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn candidate_status_serde_roundtrip() {
        for status in [
            AutoresearchCandidateStatus::Candidate,
            AutoresearchCandidateStatus::Noop,
            AutoresearchCandidateStatus::Abort,
            AutoresearchCandidateStatus::Interrupted,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: AutoresearchCandidateStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn decision_status_serde_roundtrip() {
        for status in [
            AutoresearchDecisionStatus::Baseline,
            AutoresearchDecisionStatus::Keep,
            AutoresearchDecisionStatus::Discard,
            AutoresearchDecisionStatus::Ambiguous,
            AutoresearchDecisionStatus::Noop,
            AutoresearchDecisionStatus::Abort,
            AutoresearchDecisionStatus::Interrupted,
            AutoresearchDecisionStatus::Error,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: AutoresearchDecisionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn run_status_serde_roundtrip() {
        for status in [
            AutoresearchRunStatus::Running,
            AutoresearchRunStatus::Stopped,
            AutoresearchRunStatus::Completed,
            AutoresearchRunStatus::Failed,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: AutoresearchRunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn build_run_tag_format() {
        let tag = build_run_tag();
        // Format: 20260405T123456Z
        assert_eq!(tag.len(), 16);
        assert!(tag.contains('T'));
        assert!(tag.ends_with('Z'));
    }

    // -- Task 8: Decision logic --

    fn make_manifest(last_kept_score: Option<f64>) -> AutoresearchRunManifest {
        AutoresearchRunManifest {
            schema_version: 1,
            run_id: "test".into(),
            run_tag: "20260405T000000Z".into(),
            run_dir: PathBuf::from("/tmp/run"),
            repo_root: PathBuf::from("/tmp/repo"),
            worktree_path: PathBuf::from("/tmp/wt"),
            mission_slug: "test-mission".into(),
            status: AutoresearchRunStatus::Running,
            iteration: 1,
            baseline_pass: Some(true),
            baseline_score: Some(0.5),
            last_kept_commit: Some("abc123".into()),
            last_kept_score,
            created_at: "2026-04-05T00:00:00Z".into(),
            updated_at: "2026-04-05T00:00:00Z".into(),
        }
    }

    fn make_candidate(status: AutoresearchCandidateStatus) -> AutoresearchCandidateArtifact {
        AutoresearchCandidateArtifact {
            status,
            candidate_commit: Some("def456".into()),
            base_commit: Some("abc123".into()),
            description: Some("test candidate".into()),
            notes: vec![],
            created_at: "2026-04-05T00:00:00Z".into(),
        }
    }

    fn make_eval(
        pass: Option<bool>,
        score: Option<f64>,
        parse_error: Option<String>,
    ) -> AutoresearchEvaluationRecord {
        AutoresearchEvaluationRecord {
            command: "cargo test".into(),
            ran_at: "2026-04-05T00:00:00Z".into(),
            status: "completed".into(),
            pass,
            score,
            exit_code: Some(0),
            stdout: None,
            stderr: None,
            parse_error,
        }
    }

    #[test]
    fn decide_outcome_abort() {
        let m = make_manifest(Some(0.5));
        let c = make_candidate(AutoresearchCandidateStatus::Abort);
        let e = make_eval(Some(true), Some(0.8), None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Abort);
    }

    #[test]
    fn decide_outcome_noop() {
        let m = make_manifest(Some(0.5));
        let c = make_candidate(AutoresearchCandidateStatus::Noop);
        let e = make_eval(Some(true), Some(0.8), None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Noop);
    }

    #[test]
    fn decide_outcome_interrupted() {
        let m = make_manifest(Some(0.5));
        let c = make_candidate(AutoresearchCandidateStatus::Interrupted);
        let e = make_eval(Some(true), Some(0.8), None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Interrupted);
    }

    #[test]
    fn decide_outcome_error_eval() {
        let m = make_manifest(Some(0.5));
        let c = make_candidate(AutoresearchCandidateStatus::Candidate);
        let e = make_eval(None, None, Some("bad json".into()));
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Error);
    }

    #[test]
    fn decide_outcome_pass_only_keeps() {
        let m = make_manifest(None);
        let c = make_candidate(AutoresearchCandidateStatus::Candidate);
        let e = make_eval(Some(true), None, None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Keep);
    }

    #[test]
    fn decide_outcome_pass_only_fail_discards() {
        let m = make_manifest(None);
        let c = make_candidate(AutoresearchCandidateStatus::Candidate);
        let e = make_eval(Some(false), None, None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Discard);
    }

    #[test]
    fn decide_outcome_score_improvement_better_keeps() {
        let m = make_manifest(Some(0.5));
        let c = make_candidate(AutoresearchCandidateStatus::Candidate);
        let e = make_eval(Some(true), Some(0.8), None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Keep);
    }

    #[test]
    fn decide_outcome_score_improvement_worse_discards() {
        let m = make_manifest(Some(0.8));
        let c = make_candidate(AutoresearchCandidateStatus::Candidate);
        let e = make_eval(Some(true), Some(0.5), None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Discard);
    }

    #[test]
    fn decide_outcome_equal_discards() {
        let m = make_manifest(Some(0.5));
        let c = make_candidate(AutoresearchCandidateStatus::Candidate);
        let e = make_eval(Some(true), Some(0.5), None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Discard);
    }

    #[test]
    fn decide_outcome_no_scores_ambiguous() {
        let m = make_manifest(Some(0.5));
        let c = make_candidate(AutoresearchCandidateStatus::Candidate);
        let e = make_eval(Some(true), None, None);
        let (status, _) = decide_outcome(&m, &c, &e);
        assert_eq!(status, AutoresearchDecisionStatus::Ambiguous);
    }

    // -- Task 9: Instruction builder and git utilities --

    #[test]
    fn trim_content_short_unchanged() {
        assert_eq!(trim_content("hello", 10), "hello");
    }

    #[test]
    fn trim_content_long_truncated() {
        assert_eq!(trim_content("hello world", 5), "hello...");
    }

    #[test]
    fn build_instructions_contains_task_info() {
        let contract = AutoresearchMissionContract {
            mission_dir: PathBuf::from("/tmp/mission"),
            repo_root: PathBuf::from("/tmp/repo"),
            mission_file: PathBuf::from("/tmp/mission/mission.md"),
            sandbox_file: PathBuf::from("/tmp/mission/sandbox.md"),
            mission_relative_dir: "mission".into(),
            mission_content: "Do the thing".into(),
            sandbox_content: "sandbox raw".into(),
            sandbox: ParsedSandboxContract {
                frontmatter: HashMap::new(),
                evaluator: AutoresearchEvaluatorContract {
                    command: "cargo test".into(),
                    format: "json".into(),
                    keep_policy: AutoresearchKeepPolicy::ScoreImprovement,
                },
                body: "sandbox body".into(),
            },
            mission_slug: "test-mission".into(),
        };

        let context = InstructionContext {
            run_id: "run-123".into(),
            iteration: 5,
            worktree_path: PathBuf::from("/tmp/wt"),
            candidate_file: PathBuf::from("/tmp/candidate.json"),
            last_kept_commit: Some("abc123".into()),
            last_kept_score: Some(0.75),
            trailing_noops: 0,
        };

        let instructions = build_instructions(&contract, &context);
        assert!(instructions.contains("run-123"));
        assert!(instructions.contains("5"));
        assert!(instructions.contains("cargo test"));
        assert!(instructions.contains("score_improvement"));
        assert!(instructions.contains("abc123"));
        assert!(instructions.contains("0.75"));
        assert!(instructions.contains("Do the thing"));
        assert!(instructions.contains("sandbox body"));
        assert!(!instructions.contains("noop"));
    }

    #[test]
    fn build_instructions_mentions_noops_when_positive() {
        let contract = AutoresearchMissionContract {
            mission_dir: PathBuf::from("/tmp/mission"),
            repo_root: PathBuf::from("/tmp/repo"),
            mission_file: PathBuf::from("/tmp/mission/mission.md"),
            sandbox_file: PathBuf::from("/tmp/mission/sandbox.md"),
            mission_relative_dir: "mission".into(),
            mission_content: "Do the thing".into(),
            sandbox_content: "sandbox raw".into(),
            sandbox: ParsedSandboxContract {
                frontmatter: HashMap::new(),
                evaluator: AutoresearchEvaluatorContract {
                    command: "cargo test".into(),
                    format: "json".into(),
                    keep_policy: AutoresearchKeepPolicy::ScoreImprovement,
                },
                body: "sandbox body".into(),
            },
            mission_slug: "test-mission".into(),
        };

        let context = InstructionContext {
            run_id: "run-123".into(),
            iteration: 5,
            worktree_path: PathBuf::from("/tmp/wt"),
            candidate_file: PathBuf::from("/tmp/candidate.json"),
            last_kept_commit: None,
            last_kept_score: None,
            trailing_noops: 3,
        };

        let instructions = build_instructions(&contract, &context);
        assert!(instructions.contains("3 consecutive noop"));
    }

    #[tokio::test]
    async fn count_trailing_noops_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("ledger.json");
        tokio::fs::write(&ledger, "[]").await.unwrap();
        let count = count_trailing_noops(&ledger).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn count_trailing_noops_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("nonexistent.json");
        let count = count_trailing_noops(&ledger).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn count_trailing_noops_with_noops() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("ledger.json");
        let entries = serde_json::to_string(&vec![
            AutoresearchLedgerEntry {
                iteration: 1,
                kind: "iteration".into(),
                decision: AutoresearchDecisionStatus::Keep,
                decision_reason: "good".into(),
                candidate_status: AutoresearchCandidateStatus::Candidate,
                base_commit: None,
                candidate_commit: None,
                kept_commit: None,
                keep_policy: "score_improvement".into(),
                evaluator: None,
                created_at: "2026-04-05T00:00:00Z".into(),
                notes: vec![],
                description: None,
            },
            AutoresearchLedgerEntry {
                iteration: 2,
                kind: "iteration".into(),
                decision: AutoresearchDecisionStatus::Noop,
                decision_reason: "no changes".into(),
                candidate_status: AutoresearchCandidateStatus::Noop,
                base_commit: None,
                candidate_commit: None,
                kept_commit: None,
                keep_policy: "score_improvement".into(),
                evaluator: None,
                created_at: "2026-04-05T00:00:00Z".into(),
                notes: vec![],
                description: None,
            },
            AutoresearchLedgerEntry {
                iteration: 3,
                kind: "iteration".into(),
                decision: AutoresearchDecisionStatus::Noop,
                decision_reason: "no changes".into(),
                candidate_status: AutoresearchCandidateStatus::Noop,
                base_commit: None,
                candidate_commit: None,
                kept_commit: None,
                keep_policy: "score_improvement".into(),
                evaluator: None,
                created_at: "2026-04-05T00:00:00Z".into(),
                notes: vec![],
                description: None,
            },
        ])
        .unwrap();
        tokio::fs::write(&ledger, entries).await.unwrap();
        let count = count_trailing_noops(&ledger).await.unwrap();
        assert_eq!(count, 2);
    }
}
