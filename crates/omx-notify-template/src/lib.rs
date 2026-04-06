use std::collections::HashMap;

use omx_types::HookEvent;
use serde::{Deserialize, Serialize};

/// Extract a string representation from a JSON value by key.
/// Returns the string value for strings, or the display representation
/// for numbers and bools. Returns empty string if missing or null.
fn json_str(value: &serde_json::Value, key: &str) -> String {
    match value.get(key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// All 21 known template variable names.
const KNOWN_VARS: &[&str] = &[
    "mode",
    "worker_id",
    "task_status",
    "branch",
    "commit_sha",
    "duration",
    "error",
    "session_id",
    "turn_count",
    "tokens_used",
    "quota_remaining",
    "job_id",
    "stage_name",
    "pipeline_name",
    "iteration",
    "worker_count",
    "completed_tasks",
    "pending_tasks",
    "team_name",
    "timestamp",
    "hostname",
];

/// Template context containing all 21 variables available for substitution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateContext {
    pub mode: String,
    pub worker_id: String,
    pub task_status: String,
    pub branch: String,
    pub commit_sha: String,
    pub duration: String,
    pub error: String,
    pub session_id: String,
    pub turn_count: String,
    pub tokens_used: String,
    pub quota_remaining: String,
    pub job_id: String,
    pub stage_name: String,
    pub pipeline_name: String,
    pub iteration: String,
    pub worker_count: String,
    pub completed_tasks: String,
    pub pending_tasks: String,
    pub team_name: String,
    pub timestamp: String,
    pub hostname: String,
}

impl TemplateContext {
    /// Build a `TemplateContext` from a `HookEvent`, extracting direct fields
    /// and pulling remaining values from the event's JSON `context` object.
    pub fn from_event(event: &HookEvent) -> Self {
        let ctx = &event.context;
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("HOST"))
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            mode: json_str(ctx, "mode"),
            worker_id: event
                .source
                .worker_id
                .clone()
                .unwrap_or_default(),
            task_status: json_str(ctx, "task_status"),
            branch: json_str(ctx, "branch"),
            commit_sha: json_str(ctx, "commit_sha"),
            duration: json_str(ctx, "duration"),
            error: json_str(ctx, "error"),
            session_id: event.session_id.clone().unwrap_or_default(),
            turn_count: json_str(ctx, "turn_count"),
            tokens_used: json_str(ctx, "tokens_used"),
            quota_remaining: json_str(ctx, "quota_remaining"),
            job_id: json_str(ctx, "job_id"),
            stage_name: json_str(ctx, "stage_name"),
            pipeline_name: json_str(ctx, "pipeline_name"),
            iteration: json_str(ctx, "iteration"),
            worker_count: json_str(ctx, "worker_count"),
            completed_tasks: json_str(ctx, "completed_tasks"),
            pending_tasks: json_str(ctx, "pending_tasks"),
            team_name: json_str(ctx, "team_name"),
            timestamp: event.timestamp.clone(),
            hostname,
        }
    }

    /// Return all 21 fields as a `HashMap<String, String>`.
    pub fn as_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("mode".to_string(), self.mode.clone());
        map.insert("worker_id".to_string(), self.worker_id.clone());
        map.insert("task_status".to_string(), self.task_status.clone());
        map.insert("branch".to_string(), self.branch.clone());
        map.insert("commit_sha".to_string(), self.commit_sha.clone());
        map.insert("duration".to_string(), self.duration.clone());
        map.insert("error".to_string(), self.error.clone());
        map.insert("session_id".to_string(), self.session_id.clone());
        map.insert("turn_count".to_string(), self.turn_count.clone());
        map.insert("tokens_used".to_string(), self.tokens_used.clone());
        map.insert("quota_remaining".to_string(), self.quota_remaining.clone());
        map.insert("job_id".to_string(), self.job_id.clone());
        map.insert("stage_name".to_string(), self.stage_name.clone());
        map.insert("pipeline_name".to_string(), self.pipeline_name.clone());
        map.insert("iteration".to_string(), self.iteration.clone());
        map.insert("worker_count".to_string(), self.worker_count.clone());
        map.insert("completed_tasks".to_string(), self.completed_tasks.clone());
        map.insert("pending_tasks".to_string(), self.pending_tasks.clone());
        map.insert("team_name".to_string(), self.team_name.clone());
        map.insert("timestamp".to_string(), self.timestamp.clone());
        map.insert("hostname".to_string(), self.hostname.clone());
        map
    }
}

/// Render a template string by substituting `{{variable}}` placeholders and
/// evaluating `{{#if variable}}content{{/if}}` conditionals.
///
/// Conditionals are processed first: content inside `{{#if var}}...{{/if}}`
/// is included only when `var` is non-empty. Then variable substitution runs.
/// Unknown variables are left as-is.
pub fn render(template: &str, ctx: &TemplateContext) -> String {
    let map = ctx.as_map();

    // Phase 1: process conditionals
    let mut result = template.to_string();
    // Repeatedly resolve conditionals (handles non-nested cases)
    loop {
        let Some(start) = result.find("{{#if ") else {
            break;
        };
        let Some(name_end) = result[start + 6..].find("}}") else {
            break;
        };
        let var_name = result[start + 6..start + 6 + name_end].trim();
        let end_tag = "{{/if}}";
        let Some(end_pos) = result[start..].find(end_tag) else {
            break;
        };
        let content_start = start + 6 + name_end + 2; // after `}}`
        let content_end = start + end_pos;
        let content = &result[content_start..content_end];

        let is_non_empty = map
            .get(var_name)
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        let replacement = if is_non_empty {
            content.to_string()
        } else {
            String::new()
        };

        result = format!(
            "{}{}{}",
            &result[..start],
            replacement,
            &result[start + end_pos + end_tag.len()..]
        );
    }

    // Phase 2: substitute variables
    for (key, value) in &map {
        let placeholder = format!("{{{{{key}}}}}");
        result = result.replace(&placeholder, value);
    }

    result
}

/// Validate a template and return names of unknown variables.
/// Skips `{{#if ...}}` and `{{/if}}` tags.
pub fn validate_template(template: &str) -> Vec<String> {
    let mut unknown = Vec::new();
    let mut pos = 0;
    let bytes = template.as_bytes();

    while pos < template.len() {
        if let Some(start) = template[pos..].find("{{") {
            let abs_start = pos + start;
            let after_braces = abs_start + 2;

            // Skip {{#if ...}} and {{/if}}
            if template[after_braces..].starts_with("#if ")
                || template[after_braces..].starts_with("/if}}")
            {
                pos = after_braces;
                continue;
            }

            if let Some(end) = template[after_braces..].find("}}") {
                let var_name = template[after_braces..after_braces + end].trim();
                if !KNOWN_VARS.contains(&var_name) {
                    unknown.push(var_name.to_string());
                }
                pos = after_braces + end + 2;
            } else {
                pos = after_braces;
            }
        } else {
            break;
        }
    }

    // Suppress unused variable warning for bytes (kept for potential future use)
    let _ = bytes;

    unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{HookEvent, HookEventName, HookSource};
    use serde_json::json;

    fn rich_event() -> HookEvent {
        HookEvent {
            schema_version: "1".to_string(),
            event: HookEventName::TurnComplete,
            timestamp: "2026-04-05T12:00:00Z".to_string(),
            source: HookSource {
                component: "orchestrator".to_string(),
                worker_id: Some("worker-42".to_string()),
            },
            context: json!({
                "mode": "swarm",
                "task_status": "running",
                "branch": "feat/template",
                "commit_sha": "abc123",
                "duration": "42s",
                "error": "",
                "turn_count": 5,
                "tokens_used": 1500,
                "quota_remaining": 8500,
                "job_id": "job-99",
                "stage_name": "build",
                "pipeline_name": "ci-main",
                "iteration": 3,
                "worker_count": 4,
                "completed_tasks": 7,
                "pending_tasks": 2,
                "team_name": "alpha"
            }),
            session_id: Some("sess-abc".to_string()),
        }
    }

    fn minimal_event() -> HookEvent {
        HookEvent {
            schema_version: "1".to_string(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-05T00:00:00Z".to_string(),
            source: HookSource {
                component: "cli".to_string(),
                worker_id: None,
            },
            context: json!({}),
            session_id: None,
        }
    }

    #[test]
    fn from_event_extracts_all_fields() {
        let ctx = TemplateContext::from_event(&rich_event());
        assert_eq!(ctx.mode, "swarm");
        assert_eq!(ctx.worker_id, "worker-42");
        assert_eq!(ctx.task_status, "running");
        assert_eq!(ctx.branch, "feat/template");
        assert_eq!(ctx.commit_sha, "abc123");
        assert_eq!(ctx.duration, "42s");
        assert_eq!(ctx.error, "");
        assert_eq!(ctx.session_id, "sess-abc");
        assert_eq!(ctx.turn_count, "5");
        assert_eq!(ctx.tokens_used, "1500");
        assert_eq!(ctx.quota_remaining, "8500");
        assert_eq!(ctx.job_id, "job-99");
        assert_eq!(ctx.stage_name, "build");
        assert_eq!(ctx.pipeline_name, "ci-main");
        assert_eq!(ctx.iteration, "3");
        assert_eq!(ctx.worker_count, "4");
        assert_eq!(ctx.completed_tasks, "7");
        assert_eq!(ctx.pending_tasks, "2");
        assert_eq!(ctx.team_name, "alpha");
        assert_eq!(ctx.timestamp, "2026-04-05T12:00:00Z");
        assert!(!ctx.hostname.is_empty());
    }

    #[test]
    fn from_event_defaults_missing_fields() {
        let ctx = TemplateContext::from_event(&minimal_event());
        assert_eq!(ctx.worker_id, "");
        assert_eq!(ctx.session_id, "");
        assert_eq!(ctx.mode, "");
        assert_eq!(ctx.branch, "");
        assert_eq!(ctx.turn_count, "");
    }

    #[test]
    fn as_map_has_21_entries() {
        let ctx = TemplateContext::default();
        let map = ctx.as_map();
        assert_eq!(map.len(), 21);
    }

    #[test]
    fn render_substitutes_variables() {
        let ctx = TemplateContext {
            mode: "swarm".to_string(),
            worker_id: "w1".to_string(),
            ..Default::default()
        };
        let result = render("Mode: {{mode}}, Worker: {{worker_id}}", &ctx);
        assert_eq!(result, "Mode: swarm, Worker: w1");
    }

    #[test]
    fn render_conditional_shown_when_non_empty() {
        let ctx = TemplateContext {
            error: "boom".to_string(),
            ..Default::default()
        };
        let result = render("{{#if error}}Error: {{error}}{{/if}}", &ctx);
        assert_eq!(result, "Error: boom");
    }

    #[test]
    fn render_conditional_hidden_when_empty() {
        let ctx = TemplateContext::default();
        let result = render("start{{#if error}}Error: {{error}}{{/if}}end", &ctx);
        assert_eq!(result, "startend");
    }

    #[test]
    fn render_mixed_conditionals_and_vars() {
        let ctx = TemplateContext {
            mode: "solo".to_string(),
            error: "fail".to_string(),
            branch: String::new(),
            ..Default::default()
        };
        let tpl = "[{{mode}}]{{#if error}} ERR={{error}}{{/if}}{{#if branch}} BR={{branch}}{{/if}}";
        let result = render(tpl, &ctx);
        assert_eq!(result, "[solo] ERR=fail");
    }

    #[test]
    fn render_unknown_var_left_as_is() {
        let ctx = TemplateContext::default();
        let result = render("Hello {{unknown_var}}", &ctx);
        assert_eq!(result, "Hello {{unknown_var}}");
    }

    #[test]
    fn validate_template_finds_unknown_vars() {
        let unknown = validate_template("{{mode}} {{bogus}} {{also_bad}}");
        assert_eq!(unknown, vec!["bogus", "also_bad"]);
    }

    #[test]
    fn validate_template_ignores_conditionals() {
        let unknown = validate_template("{{#if error}}{{error}}{{/if}} {{weird}}");
        assert_eq!(unknown, vec!["weird"]);
    }

    #[test]
    fn validate_template_all_known_vars_pass() {
        let tpl = KNOWN_VARS
            .iter()
            .map(|v| format!("{{{{{v}}}}}"))
            .collect::<Vec<_>>()
            .join(" ");
        let unknown = validate_template(&tpl);
        assert!(unknown.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let ctx = TemplateContext::from_event(&rich_event());
        let json = serde_json::to_string(&ctx).unwrap();
        let ctx2: TemplateContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.mode, ctx2.mode);
        assert_eq!(ctx.worker_id, ctx2.worker_id);
        assert_eq!(ctx.timestamp, ctx2.timestamp);
    }
}
