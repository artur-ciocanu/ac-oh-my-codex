# Phase 11: Notification & Hook Completion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the notification system by adding Pushover and generic webhook crates, a shared template engine, rich formatting (embeds/blocks/mentions) for existing platforms, and wiring the hook-to-notification dispatch flow.

**Architecture:** Bottom-up — first add a shared `omx-notify-template` library crate with Handlebars-style template rendering and 21 context variables. Then add `omx-notify-pushover` and `omx-notify-generic` binary crates following the existing stdin-JSON/stdout-JSON hook pattern. Then enhance the 3 existing notify binaries with rich formatting, mentions, and reply correlation. Finally, extend `omx-config` with Pushover/Generic config sections and add a `NotificationRouter` to `omx-hooks` that routes HookEvents to configured platforms.

**Tech Stack:** Rust, reqwest (HTTP), serde/serde_json, tokio, omx-types (HookEvent), omx-config (NotificationConfig)

---

## File Structure

### New crate: `omx-notify-template` — Library

| File | Responsibility |
|------|---------------|
| `crates/omx-notify-template/Cargo.toml` | Crate manifest |
| `crates/omx-notify-template/src/lib.rs` | Template engine: `TemplateContext`, `render()`, conditional blocks, variable substitution |

### New crate: `omx-notify-pushover` — Binary

| File | Responsibility |
|------|---------------|
| `crates/omx-notify-pushover/Cargo.toml` | Crate manifest |
| `crates/omx-notify-pushover/src/main.rs` | Pushover API integration: read HookEvent stdin, render template, POST to Pushover, write HookResult stdout |

### New crate: `omx-notify-generic` — Binary

| File | Responsibility |
|------|---------------|
| `crates/omx-notify-generic/Cargo.toml` | Crate manifest |
| `crates/omx-notify-generic/src/main.rs` | Generic webhook: read HookEvent stdin, render template, POST JSON to user URL, write HookResult stdout |

### Modified: `omx-notify-discord`

| File | Change |
|------|--------|
| `crates/omx-notify-discord/Cargo.toml` | Add omx-notify-template dependency |
| `crates/omx-notify-discord/src/main.rs` | Replace plain text with Discord embed, add mention support |

### Modified: `omx-notify-slack`

| File | Change |
|------|--------|
| `crates/omx-notify-slack/Cargo.toml` | Add omx-notify-template dependency |
| `crates/omx-notify-slack/src/main.rs` | Replace plain text with Slack Block Kit, add mention support |

### Modified: `omx-notify-telegram`

| File | Change |
|------|--------|
| `crates/omx-notify-telegram/Cargo.toml` | Add omx-notify-template dependency |
| `crates/omx-notify-telegram/src/main.rs` | Add HTML formatting, reply-to support |

### Modified: `omx-config`

| File | Change |
|------|--------|
| `crates/omx-config/src/lib.rs` | Add `PushoverConfig`, `GenericWebhookConfig` to `NotificationConfig`; add per-event template config |

### Modified: `omx-hooks`

| File | Change |
|------|--------|
| `crates/omx-hooks/src/router.rs` | New: `NotificationRouter` that maps events to configured notification platforms |
| `crates/omx-hooks/src/lib.rs` | Add `pub mod router;` |

### Modified: Workspace `Cargo.toml`

| File | Change |
|------|--------|
| `Cargo.toml` | Add `omx-notify-template`, `omx-notify-pushover`, `omx-notify-generic` to workspace members |

---

## Task 1: Workspace — Register New Crates

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add new crate members to workspace**

Add these 3 entries to the `[workspace] members` array, in the "Notification hooks" section after `"crates/omx-notify-telegram"`:

```toml
  "crates/omx-notify-template",
  "crates/omx-notify-pushover",
  "crates/omx-notify-generic",
```

- [ ] **Step 2: Create crate directory stubs**

Run:
```bash
mkdir -p crates/omx-notify-template/src
mkdir -p crates/omx-notify-pushover/src
mkdir -p crates/omx-notify-generic/src
```

- [ ] **Step 3: Create `crates/omx-notify-template/Cargo.toml`**

```toml
[package]
name = "omx-notify-template"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Shared notification template engine for OMX"

[dependencies]
omx-types = { path = "../omx-types" }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 4: Create `crates/omx-notify-template/src/lib.rs` placeholder**

```rust
// Template engine — implemented in Task 2
```

- [ ] **Step 5: Create `crates/omx-notify-pushover/Cargo.toml`**

```toml
[package]
name = "omx-notify-pushover"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Pushover notification hook for OMX"

[[bin]]
name = "omx-notify-pushover"
path = "src/main.rs"

[dependencies]
omx-types = { path = "../omx-types" }
omx-notify-template = { path = "../omx-notify-template" }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 6: Create `crates/omx-notify-pushover/src/main.rs` placeholder**

```rust
fn main() {
    println!("omx-notify-pushover placeholder");
}
```

- [ ] **Step 7: Create `crates/omx-notify-generic/Cargo.toml`**

```toml
[package]
name = "omx-notify-generic"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Generic webhook notification hook for OMX"

[[bin]]
name = "omx-notify-generic"
path = "src/main.rs"

[dependencies]
omx-types = { path = "../omx-types" }
omx-notify-template = { path = "../omx-notify-template" }
serde = { workspace = true }
serde_json = { workspace = true }
reqwest = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 8: Create `crates/omx-notify-generic/src/main.rs` placeholder**

```rust
fn main() {
    println!("omx-notify-generic placeholder");
}
```

- [ ] **Step 9: Run build to verify workspace compiles**

Run: `cargo build --workspace 2>&1 | tail -5`
Expected: Clean build, no errors

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock crates/omx-notify-template crates/omx-notify-pushover crates/omx-notify-generic
git commit -m "chore: register omx-notify-template, omx-notify-pushover, omx-notify-generic in workspace"
```

---

## Task 2: Template Engine — `omx-notify-template`

**Files:**
- Modify: `crates/omx-notify-template/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Replace the placeholder in `crates/omx-notify-template/src/lib.rs` with:

```rust
use omx_types::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// All 21 template variables extracted from a HookEvent + runtime context.
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
    /// Build a TemplateContext from a HookEvent, extracting known fields from `context` JSON.
    pub fn from_event(event: &HookEvent) -> Self {
        let ctx = &event.context;
        Self {
            mode: json_str(ctx, "mode"),
            worker_id: event.source.worker_id.clone().unwrap_or_default(),
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
            hostname: hostname(),
        }
    }

    /// Convert to a HashMap for template variable lookup.
    pub fn as_map(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("mode".into(), self.mode.clone());
        m.insert("worker_id".into(), self.worker_id.clone());
        m.insert("task_status".into(), self.task_status.clone());
        m.insert("branch".into(), self.branch.clone());
        m.insert("commit_sha".into(), self.commit_sha.clone());
        m.insert("duration".into(), self.duration.clone());
        m.insert("error".into(), self.error.clone());
        m.insert("session_id".into(), self.session_id.clone());
        m.insert("turn_count".into(), self.turn_count.clone());
        m.insert("tokens_used".into(), self.tokens_used.clone());
        m.insert("quota_remaining".into(), self.quota_remaining.clone());
        m.insert("job_id".into(), self.job_id.clone());
        m.insert("stage_name".into(), self.stage_name.clone());
        m.insert("pipeline_name".into(), self.pipeline_name.clone());
        m.insert("iteration".into(), self.iteration.clone());
        m.insert("worker_count".into(), self.worker_count.clone());
        m.insert("completed_tasks".into(), self.completed_tasks.clone());
        m.insert("pending_tasks".into(), self.pending_tasks.clone());
        m.insert("team_name".into(), self.team_name.clone());
        m.insert("timestamp".into(), self.timestamp.clone());
        m.insert("hostname".into(), self.hostname.clone());
        m
    }
}

fn json_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "unknown".into())
}

/// Render a template string, substituting `{{variable}}` with values from context.
/// Supports conditionals: `{{#if variable}}content{{/if}}`
pub fn render(template: &str, ctx: &TemplateContext) -> String {
    let vars = ctx.as_map();
    let mut output = template.to_string();

    // Process conditionals first: {{#if var}}content{{/if}}
    loop {
        let Some(start) = output.find("{{#if ") else {
            break;
        };
        let Some(cond_end) = output[start..].find("}}") else {
            break;
        };
        let cond_end = start + cond_end;
        let var_name = output[start + 6..cond_end].trim();

        let Some(endif) = output[cond_end..].find("{{/if}}") else {
            break;
        };
        let endif = cond_end + endif;

        let var_value = vars.get(var_name).map(|s| s.as_str()).unwrap_or("");
        let body = &output[cond_end + 2..endif];

        let replacement = if var_value.is_empty() {
            String::new()
        } else {
            body.to_string()
        };

        output = format!("{}{}{}", &output[..start], replacement, &output[endif + 7..]);
    }

    // Then substitute variables: {{variable}}
    for (key, value) in &vars {
        let placeholder = format!("{{{{{}}}}}", key);
        output = output.replace(&placeholder, value);
    }

    output
}

/// Validate a template string — returns variable names that are not in the known 21.
pub fn validate_template(template: &str) -> Vec<String> {
    let known: std::collections::HashSet<&str> = [
        "mode", "worker_id", "task_status", "branch", "commit_sha",
        "duration", "error", "session_id", "turn_count", "tokens_used",
        "quota_remaining", "job_id", "stage_name", "pipeline_name",
        "iteration", "worker_count", "completed_tasks", "pending_tasks",
        "team_name", "timestamp", "hostname",
    ].into_iter().collect();

    let mut unknown = Vec::new();
    let mut remaining = template;
    while let Some(start) = remaining.find("{{") {
        // Skip conditional tags
        if remaining[start..].starts_with("{{#if ")
            || remaining[start..].starts_with("{{/if}}")
        {
            remaining = &remaining[start + 2..];
            continue;
        }
        if let Some(end) = remaining[start + 2..].find("}}") {
            let var = remaining[start + 2..start + 2 + end].trim();
            if !known.contains(var) && !unknown.contains(&var.to_string()) {
                unknown.push(var.to_string());
            }
            remaining = &remaining[start + 2 + end + 2..];
        } else {
            break;
        }
    }
    unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{HookEventName, HookSource};

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "omx-team".into(),
                worker_id: Some("w-001".into()),
            },
            context: serde_json::json!({
                "mode": "autopilot",
                "branch": "feat/notify",
                "error": "",
                "turn_count": "42",
                "tokens_used": "15000"
            }),
            session_id: Some("sess-abc".into()),
        }
    }

    #[test]
    fn from_event_extracts_all_fields() {
        let ctx = TemplateContext::from_event(&test_event());
        assert_eq!(ctx.mode, "autopilot");
        assert_eq!(ctx.worker_id, "w-001");
        assert_eq!(ctx.branch, "feat/notify");
        assert_eq!(ctx.session_id, "sess-abc");
        assert_eq!(ctx.turn_count, "42");
        assert_eq!(ctx.tokens_used, "15000");
        assert_eq!(ctx.timestamp, "2026-04-05T10:00:00Z");
    }

    #[test]
    fn from_event_defaults_missing_fields() {
        let event = HookEvent {
            schema_version: "1".into(),
            event: HookEventName::Failed,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: None,
        };
        let ctx = TemplateContext::from_event(&event);
        assert_eq!(ctx.mode, "");
        assert_eq!(ctx.worker_id, "");
        assert_eq!(ctx.session_id, "");
    }

    #[test]
    fn as_map_has_21_entries() {
        let ctx = TemplateContext::default();
        assert_eq!(ctx.as_map().len(), 21);
    }

    #[test]
    fn render_substitutes_variables() {
        let ctx = TemplateContext::from_event(&test_event());
        let result = render("Mode: {{mode}}, Branch: {{branch}}", &ctx);
        assert_eq!(result, "Mode: autopilot, Branch: feat/notify");
    }

    #[test]
    fn render_conditional_shown_when_non_empty() {
        let ctx = TemplateContext::from_event(&test_event());
        let result = render("{{#if branch}}on {{branch}}{{/if}}", &ctx);
        assert_eq!(result, "on feat/notify");
    }

    #[test]
    fn render_conditional_hidden_when_empty() {
        let ctx = TemplateContext::from_event(&test_event());
        let result = render("{{#if error}}Error: {{error}}{{/if}}", &ctx);
        assert_eq!(result, "");
    }

    #[test]
    fn render_mixed_conditionals_and_vars() {
        let ctx = TemplateContext::from_event(&test_event());
        let tmpl = "[{{mode}}] Turn {{turn_count}}{{#if error}} ERROR: {{error}}{{/if}}";
        let result = render(tmpl, &ctx);
        assert_eq!(result, "[autopilot] Turn 42");
    }

    #[test]
    fn render_unknown_var_left_as_is() {
        let ctx = TemplateContext::default();
        let result = render("Hello {{unknown_var}}", &ctx);
        assert_eq!(result, "Hello {{unknown_var}}");
    }

    #[test]
    fn validate_template_finds_unknown_vars() {
        let unknown = validate_template("{{mode}} {{bad_var}} {{branch}} {{also_bad}}");
        assert_eq!(unknown, vec!["bad_var".to_string(), "also_bad".to_string()]);
    }

    #[test]
    fn validate_template_ignores_conditionals() {
        let unknown = validate_template("{{#if mode}}yes{{/if}} {{branch}}");
        assert!(unknown.is_empty());
    }

    #[test]
    fn validate_template_all_known_vars_pass() {
        let tmpl = "{{mode}} {{worker_id}} {{task_status}} {{branch}} {{commit_sha}} {{duration}} {{error}} {{session_id}} {{turn_count}} {{tokens_used}} {{quota_remaining}} {{job_id}} {{stage_name}} {{pipeline_name}} {{iteration}} {{worker_count}} {{completed_tasks}} {{pending_tasks}} {{team_name}} {{timestamp}} {{hostname}}";
        let unknown = validate_template(tmpl);
        assert!(unknown.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let ctx = TemplateContext::from_event(&test_event());
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: TemplateContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mode, ctx.mode);
        assert_eq!(parsed.worker_id, ctx.worker_id);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omx-notify-template -- --nocapture`
Expected: All 11 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/omx-notify-template/src/lib.rs
git commit -m "feat(omx-notify-template): add template engine with 21 variables and conditionals"
```

---

## Task 3: Config — Add Pushover and Generic Webhook Config Sections

**Files:**
- Modify: `crates/omx-config/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add to the test module in `crates/omx-config/src/lib.rs`:

```rust
#[test]
fn load_reads_pushover_config_from_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let toml_content = r#"
[notifications.pushover]
user_key = "uQiRzpo4DXghDmr9QzzfQu27cmVRsG"
app_token = "azGDORePK8gMaC0QOYAMyEEuzJnyUi"
device = "iphone"
"#;
    fs::write(tmp.path().join("config.toml"), toml_content).unwrap();
    let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
    let pushover = config.notifications.pushover.as_ref().unwrap();
    assert_eq!(pushover.user_key, "uQiRzpo4DXghDmr9QzzfQu27cmVRsG");
    assert_eq!(pushover.app_token, "azGDORePK8gMaC0QOYAMyEEuzJnyUi");
    assert_eq!(pushover.device.as_deref(), Some("iphone"));
}

#[test]
fn load_reads_generic_webhook_config_from_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let toml_content = r#"
[notifications.generic]
url = "https://example.com/webhook"
auth_token = "Bearer secret123"

[notifications.generic.headers]
X-Custom = "value"
"#;
    fs::write(tmp.path().join("config.toml"), toml_content).unwrap();
    let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
    let generic = config.notifications.generic.as_ref().unwrap();
    assert_eq!(generic.url, "https://example.com/webhook");
    assert_eq!(generic.auth_token.as_deref(), Some("Bearer secret123"));
    assert_eq!(generic.headers.get("X-Custom").unwrap(), "value");
}

#[test]
fn notification_mentions_config() {
    let tmp = tempfile::tempdir().unwrap();
    let toml_content = r#"
[notifications.discord]
webhook_url = "https://discord.com/api/webhooks/123/abc"
mention = "<@&12345>"

[notifications.slack]
webhook_url = "https://hooks.slack.com/services/T/B/X"
mention = "<@U12345>"
"#;
    fs::write(tmp.path().join("config.toml"), toml_content).unwrap();
    let config = DefaultConfigLoader::load(tmp.path(), &HashMap::new()).unwrap();
    assert_eq!(config.notifications.discord.as_ref().unwrap().mention.as_deref(), Some("<@&12345>"));
    assert_eq!(config.notifications.slack.as_ref().unwrap().mention.as_deref(), Some("<@U12345>"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omx-config pushover -- --nocapture 2>&1 | head -20`
Expected: FAIL — `pushover` field not found on `NotificationConfig`

- [ ] **Step 3: Add config structs**

Add after the existing `TelegramConfig` struct in `crates/omx-config/src/lib.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushoverConfig {
    pub user_key: String,
    pub app_token: String,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericWebhookConfig {
    pub url: String,
    pub auth_token: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}
```

Add `mention` field to the existing `DiscordConfig`, `SlackConfig`, and `TelegramConfig`:

```rust
// DiscordConfig — add field:
pub mention: Option<String>,

// SlackConfig — add field:
pub mention: Option<String>,

// TelegramConfig — add field:
pub mention: Option<String>,
```

Add to `NotificationConfig`:

```rust
pub pushover: Option<PushoverConfig>,
pub generic: Option<GenericWebhookConfig>,
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-config -- --nocapture`
Expected: All tests PASS (existing tests still pass since `mention`, `pushover`, `generic` are `Option` and default to `None`)

- [ ] **Step 5: Commit**

```bash
git add crates/omx-config/src/lib.rs
git commit -m "feat(omx-config): add pushover, generic webhook, and mention config sections"
```

---

## Task 4: Pushover Notification Hook — `omx-notify-pushover`

**Files:**
- Modify: `crates/omx-notify-pushover/src/main.rs`

- [ ] **Step 1: Write the implementation with tests**

Replace the placeholder in `crates/omx-notify-pushover/src/main.rs`:

```rust
use omx_notify_template::{render, TemplateContext};
use omx_types::{format_hook_message, HookEvent};
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str = "[OMX] {{mode}} — {{timestamp}}{{#if error}}\nError: {{error}}{{/if}}{{#if worker_id}}\nWorker: {{worker_id}}{{/if}}";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let event: HookEvent = serde_json::from_str(&input)?;
    let (success, stdout, stderr) = send_pushover_notification(&event).await;

    let result = serde_json::json!({
        "hook": "omx-notify-pushover",
        "success": success,
        "stdout": stdout,
        "stderr": stderr,
        "duration_ms": 0
    });

    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

async fn send_pushover_notification(event: &HookEvent) -> (bool, String, String) {
    let user_key = match std::env::var("OMX_PUSHOVER_USER_KEY") {
        Ok(k) => k,
        Err(_) => return (false, String::new(), "OMX_PUSHOVER_USER_KEY not set".into()),
    };

    let app_token = match std::env::var("OMX_PUSHOVER_APP_TOKEN") {
        Ok(t) => t,
        Err(_) => return (false, String::new(), "OMX_PUSHOVER_APP_TOKEN not set".into()),
    };

    let device = std::env::var("OMX_PUSHOVER_DEVICE").ok();
    let template = std::env::var("OMX_PUSHOVER_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let message = render(&template, &ctx);

    // Map event severity to Pushover priority (-2 to 2)
    let priority = pushover_priority(event);

    let client = reqwest::Client::new();
    let mut form = vec![
        ("token", app_token.as_str().to_string()),
        ("user", user_key),
        ("message", message),
        ("priority", priority.to_string()),
        ("title", format!("OMX: {}", event.event)),
    ];

    if let Some(ref dev) = device {
        form.push(("device", dev.clone()));
    }

    let form_params: Vec<(&str, &str)> = form.iter().map(|(k, v)| (*k, v.as_str())).collect();

    match client
        .post("https://api.pushover.net/1/messages.json")
        .form(&form_params)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, format!("Pushover: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (false, String::new(), format!("Pushover API error {status}: {body}"))
            }
        }
        Err(e) => (false, String::new(), format!("Pushover request failed: {e}")),
    }
}

fn pushover_priority(event: &HookEvent) -> i8 {
    use omx_types::HookEventName;
    match event.event {
        HookEventName::Failed | HookEventName::TestFailed => 1,      // High priority
        HookEventName::Blocked => 0,                                   // Normal
        HookEventName::SessionStart | HookEventName::SessionEnd => -1, // Low
        HookEventName::SessionIdle => -1,                              // Low
        _ => 0,                                                        // Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{HookEventName, HookSource};
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::Failed,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "omx-team".into(),
                worker_id: Some("w-003".into()),
            },
            context: serde_json::json!({
                "mode": "team",
                "error": "build failed"
            }),
            session_id: Some("sess-push".into()),
        }
    }

    #[tokio::test]
    async fn returns_error_when_user_key_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("OMX_PUSHOVER_USER_KEY");
        std::env::remove_var("OMX_PUSHOVER_APP_TOKEN");
        let (success, _, stderr) = send_pushover_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_PUSHOVER_USER_KEY not set"));
    }

    #[tokio::test]
    async fn returns_error_when_app_token_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_PUSHOVER_USER_KEY", "fake-user");
        std::env::remove_var("OMX_PUSHOVER_APP_TOKEN");
        let (success, _, stderr) = send_pushover_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_PUSHOVER_APP_TOKEN not set"));
    }

    #[test]
    fn priority_mapping_failed_is_high() {
        let event = test_event();
        assert_eq!(pushover_priority(&event), 1);
    }

    #[test]
    fn priority_mapping_session_start_is_low() {
        let mut event = test_event();
        event.event = HookEventName::SessionStart;
        assert_eq!(pushover_priority(&event), -1);
    }

    #[test]
    fn priority_mapping_turn_complete_is_normal() {
        let mut event = test_event();
        event.event = HookEventName::TurnComplete;
        assert_eq!(pushover_priority(&event), 0);
    }

    #[test]
    fn default_template_renders() {
        let ctx = TemplateContext::from_event(&test_event());
        let result = render(DEFAULT_TEMPLATE, &ctx);
        assert!(result.contains("team"));
        assert!(result.contains("Error: build failed"));
        assert!(result.contains("w-003"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omx-notify-pushover -- --nocapture`
Expected: All 6 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/omx-notify-pushover/src/main.rs
git commit -m "feat(omx-notify-pushover): add Pushover notification hook with priority mapping"
```

---

## Task 5: Generic Webhook Notification Hook — `omx-notify-generic`

**Files:**
- Modify: `crates/omx-notify-generic/src/main.rs`

- [ ] **Step 1: Write the implementation with tests**

Replace the placeholder in `crates/omx-notify-generic/src/main.rs`:

```rust
use omx_notify_template::{render, TemplateContext};
use omx_types::HookEvent;
use std::collections::HashMap;
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str = "[OMX] {{mode}} — {{timestamp}}{{#if error}}\nError: {{error}}{{/if}}";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let event: HookEvent = serde_json::from_str(&input)?;
    let (success, stdout, stderr) = send_generic_notification(&event).await;

    let result = serde_json::json!({
        "hook": "omx-notify-generic",
        "success": success,
        "stdout": stdout,
        "stderr": stderr,
        "duration_ms": 0
    });

    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

async fn send_generic_notification(event: &HookEvent) -> (bool, String, String) {
    let url = match std::env::var("OMX_GENERIC_WEBHOOK_URL") {
        Ok(u) => u,
        Err(_) => return (false, String::new(), "OMX_GENERIC_WEBHOOK_URL not set".into()),
    };

    let auth_token = std::env::var("OMX_GENERIC_AUTH_TOKEN").ok();
    let template = std::env::var("OMX_GENERIC_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let message = render(&template, &ctx);

    // Parse extra headers from OMX_GENERIC_HEADERS (comma-separated "Key:Value" pairs)
    let extra_headers = parse_headers_env();

    let payload = serde_json::json!({
        "event": event.event.to_string(),
        "message": message,
        "timestamp": event.timestamp,
        "source": event.source.component,
        "session_id": event.session_id,
        "context": event.context,
    });

    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&payload);

    if let Some(ref token) = auth_token {
        req = req.header("Authorization", token);
    }

    for (key, value) in &extra_headers {
        req = req.header(key.as_str(), value.as_str());
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, format!("Webhook: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (false, String::new(), format!("Webhook error {status}: {body}"))
            }
        }
        Err(e) => (false, String::new(), format!("Webhook request failed: {e}")),
    }
}

fn parse_headers_env() -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if let Ok(raw) = std::env::var("OMX_GENERIC_HEADERS") {
        for pair in raw.split(',') {
            let pair = pair.trim();
            if let Some((key, value)) = pair.split_once(':') {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{HookEventName, HookSource};
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::Finished,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "omx-pipeline".into(),
                worker_id: None,
            },
            context: serde_json::json!({
                "mode": "ralph",
                "pipeline_name": "deploy"
            }),
            session_id: Some("sess-gen".into()),
        }
    }

    #[tokio::test]
    async fn returns_error_when_url_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("OMX_GENERIC_WEBHOOK_URL");
        let (success, _, stderr) = send_generic_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_GENERIC_WEBHOOK_URL not set"));
    }

    #[tokio::test]
    async fn returns_error_for_unreachable_url() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_GENERIC_WEBHOOK_URL", "http://127.0.0.1:1/fake");
        std::env::remove_var("OMX_GENERIC_AUTH_TOKEN");
        std::env::remove_var("OMX_GENERIC_HEADERS");
        let (success, _, stderr) = send_generic_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("request failed"));
    }

    #[test]
    fn parse_headers_env_parses_comma_separated_pairs() {
        std::env::set_var("OMX_GENERIC_HEADERS", "X-Foo:bar, X-Baz:qux");
        let headers = parse_headers_env();
        assert_eq!(headers.get("X-Foo").unwrap(), "bar");
        assert_eq!(headers.get("X-Baz").unwrap(), "qux");
        std::env::remove_var("OMX_GENERIC_HEADERS");
    }

    #[test]
    fn parse_headers_env_empty_when_unset() {
        std::env::remove_var("OMX_GENERIC_HEADERS");
        let headers = parse_headers_env();
        assert!(headers.is_empty());
    }

    #[test]
    fn default_template_renders() {
        let ctx = TemplateContext::from_event(&test_event());
        let result = render(DEFAULT_TEMPLATE, &ctx);
        assert!(result.contains("ralph"));
        assert!(result.contains("2026-04-05"));
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omx-notify-generic -- --nocapture`
Expected: All 5 tests PASS

- [ ] **Step 3: Commit**

```bash
git add crates/omx-notify-generic/src/main.rs
git commit -m "feat(omx-notify-generic): add generic webhook notification hook with custom headers"
```

---

## Task 6: Discord — Rich Embeds and Mention Support

**Files:**
- Modify: `crates/omx-notify-discord/Cargo.toml`
- Modify: `crates/omx-notify-discord/src/main.rs`

- [ ] **Step 1: Add omx-notify-template dependency**

Add to `[dependencies]` in `crates/omx-notify-discord/Cargo.toml`:

```toml
omx-notify-template = { path = "../omx-notify-template" }
```

- [ ] **Step 2: Replace `crates/omx-notify-discord/src/main.rs`**

```rust
use omx_notify_template::{render, TemplateContext};
use omx_types::{HookEvent, HookEventName};
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str = "{{#if error}}Error: {{error}}{{/if}}{{#if worker_id}}Worker: {{worker_id}}{{/if}}{{#if branch}}\nBranch: {{branch}}{{/if}}";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let event: HookEvent = serde_json::from_str(&input)?;
    let (success, stdout, stderr) = send_discord_notification(&event).await;

    let result = serde_json::json!({
        "hook": "omx-notify-discord",
        "success": success,
        "stdout": stdout,
        "stderr": stderr,
        "duration_ms": 0
    });

    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn embed_color(event: &HookEventName) -> u32 {
    match event {
        HookEventName::Failed | HookEventName::TestFailed => 0xFF0000,   // Red
        HookEventName::Blocked => 0xFFA500,                               // Orange
        HookEventName::Finished | HookEventName::TestFinished => 0x00FF00, // Green
        HookEventName::SessionStart => 0x0099FF,                           // Blue
        HookEventName::SessionEnd => 0x808080,                             // Gray
        _ => 0x7289DA,                                                     // Discord blurple
    }
}

async fn send_discord_notification(event: &HookEvent) -> (bool, String, String) {
    let webhook_url = match std::env::var("OMX_DISCORD_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => {
            return (
                false,
                String::new(),
                "OMX_DISCORD_WEBHOOK_URL not set".into(),
            )
        }
    };

    let mention = std::env::var("OMX_DISCORD_MENTION").ok();
    let template = std::env::var("OMX_DISCORD_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let description = render(&template, &ctx);

    let color = embed_color(&event.event);

    let mut embed = serde_json::json!({
        "title": format!("OMX: {}", event.event),
        "description": description,
        "color": color,
        "timestamp": event.timestamp,
        "footer": {
            "text": format!("Source: {}", event.source.component)
        }
    });

    if let Some(ref sid) = event.session_id {
        embed["fields"] = serde_json::json!([{
            "name": "Session",
            "value": sid,
            "inline": true
        }]);
    }

    let mut content = String::new();
    if let Some(ref m) = mention {
        content = m.clone();
    }

    let payload = serde_json::json!({
        "content": if content.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(content) },
        "embeds": [embed]
    });

    let client = reqwest::Client::new();
    match client.post(&webhook_url).json(&payload).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() || status.as_u16() == 204 {
                (true, format!("Discord: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (
                    false,
                    String::new(),
                    format!("Discord API error {status}: {body}"),
                )
            }
        }
        Err(e) => (false, String::new(), format!("Discord request failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::HookSource;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: Some("sess-123".into()),
        }
    }

    #[tokio::test]
    async fn returns_error_when_env_var_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("OMX_DISCORD_WEBHOOK_URL");
        let (success, _, stderr) = send_discord_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_DISCORD_WEBHOOK_URL not set"));
    }

    #[tokio::test]
    async fn returns_error_for_unreachable_url() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_DISCORD_WEBHOOK_URL", "http://127.0.0.1:1/fake");
        std::env::remove_var("OMX_DISCORD_MENTION");
        std::env::remove_var("OMX_DISCORD_TEMPLATE");
        let (success, _, stderr) = send_discord_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("request failed"));
    }

    #[test]
    fn embed_color_red_for_failed() {
        assert_eq!(embed_color(&HookEventName::Failed), 0xFF0000);
    }

    #[test]
    fn embed_color_green_for_finished() {
        assert_eq!(embed_color(&HookEventName::Finished), 0x00FF00);
    }

    #[test]
    fn embed_color_blue_for_session_start() {
        assert_eq!(embed_color(&HookEventName::SessionStart), 0x0099FF);
    }

    #[test]
    fn default_template_renders_without_error() {
        let ctx = TemplateContext::from_event(&test_event());
        let result = render(DEFAULT_TEMPLATE, &ctx);
        // No error or worker_id in test_event, so both conditionals are empty
        assert_eq!(result.trim(), "");
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omx-notify-discord -- --nocapture`
Expected: All 6 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-notify-discord/Cargo.toml crates/omx-notify-discord/src/main.rs
git commit -m "feat(omx-notify-discord): add rich embeds, color coding, and mention support"
```

---

## Task 7: Slack — Block Kit and Mention Support

**Files:**
- Modify: `crates/omx-notify-slack/Cargo.toml`
- Modify: `crates/omx-notify-slack/src/main.rs`

- [ ] **Step 1: Add omx-notify-template dependency**

Add to `[dependencies]` in `crates/omx-notify-slack/Cargo.toml`:

```toml
omx-notify-template = { path = "../omx-notify-template" }
```

- [ ] **Step 2: Replace `crates/omx-notify-slack/src/main.rs`**

```rust
use omx_notify_template::{render, TemplateContext};
use omx_types::{HookEvent, HookEventName};
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str = "{{#if error}}:x: Error: {{error}}{{/if}}{{#if worker_id}}\nWorker: `{{worker_id}}`{{/if}}{{#if branch}}\nBranch: `{{branch}}`{{/if}}";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let event: HookEvent = serde_json::from_str(&input)?;
    let (success, stdout, stderr) = send_slack_notification(&event).await;

    let result = serde_json::json!({
        "hook": "omx-notify-slack",
        "success": success,
        "stdout": stdout,
        "stderr": stderr,
        "duration_ms": 0
    });

    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn slack_emoji(event: &HookEventName) -> &'static str {
    match event {
        HookEventName::Failed | HookEventName::TestFailed => ":red_circle:",
        HookEventName::Blocked => ":warning:",
        HookEventName::Finished | HookEventName::TestFinished => ":white_check_mark:",
        HookEventName::SessionStart => ":rocket:",
        HookEventName::SessionEnd => ":checkered_flag:",
        HookEventName::SessionIdle => ":zzz:",
        _ => ":large_blue_circle:",
    }
}

fn build_blocks(event: &HookEvent, description: &str) -> serde_json::Value {
    let emoji = slack_emoji(&event.event);
    let mut blocks = vec![
        serde_json::json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": format!("{emoji} OMX: {}", event.event),
                "emoji": true
            }
        }),
    ];

    if !description.trim().is_empty() {
        blocks.push(serde_json::json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": description
            }
        }));
    }

    let mut fields = vec![
        serde_json::json!({
            "type": "mrkdwn",
            "text": format!("*Source:* `{}`", event.source.component)
        }),
    ];

    if let Some(ref sid) = event.session_id {
        fields.push(serde_json::json!({
            "type": "mrkdwn",
            "text": format!("*Session:* `{sid}`")
        }));
    }

    blocks.push(serde_json::json!({
        "type": "section",
        "fields": fields
    }));

    blocks.push(serde_json::json!({
        "type": "context",
        "elements": [{
            "type": "mrkdwn",
            "text": format!("Sent at {}", event.timestamp)
        }]
    }));

    serde_json::json!(blocks)
}

async fn send_slack_notification(event: &HookEvent) -> (bool, String, String) {
    let webhook_url = match std::env::var("OMX_SLACK_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => return (false, String::new(), "OMX_SLACK_WEBHOOK_URL not set".into()),
    };

    let mention = std::env::var("OMX_SLACK_MENTION").ok();
    let template = std::env::var("OMX_SLACK_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let description = render(&template, &ctx);

    let blocks = build_blocks(event, &description);

    let text = match &mention {
        Some(m) => format!("{m} OMX: {}", event.event),
        None => format!("OMX: {}", event.event),
    };

    let payload = serde_json::json!({
        "text": text,
        "blocks": blocks
    });

    let client = reqwest::Client::new();
    match client.post(&webhook_url).json(&payload).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, format!("Slack: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (
                    false,
                    String::new(),
                    format!("Slack API error {status}: {body}"),
                )
            }
        }
        Err(e) => (false, String::new(), format!("Slack request failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::HookSource;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::TurnComplete,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "test".into(),
                worker_id: Some("w-002".into()),
            },
            context: serde_json::json!({"task": "deploy"}),
            session_id: Some("sess-456".into()),
        }
    }

    #[tokio::test]
    async fn returns_error_when_env_var_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("OMX_SLACK_WEBHOOK_URL");
        let (success, _, stderr) = send_slack_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_SLACK_WEBHOOK_URL not set"));
    }

    #[tokio::test]
    async fn returns_error_for_unreachable_url() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_SLACK_WEBHOOK_URL", "http://127.0.0.1:1/fake");
        std::env::remove_var("OMX_SLACK_MENTION");
        std::env::remove_var("OMX_SLACK_TEMPLATE");
        let (success, _, stderr) = send_slack_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("request failed"));
    }

    #[test]
    fn slack_emoji_failed_is_red() {
        assert_eq!(slack_emoji(&HookEventName::Failed), ":red_circle:");
    }

    #[test]
    fn slack_emoji_finished_is_check() {
        assert_eq!(slack_emoji(&HookEventName::Finished), ":white_check_mark:");
    }

    #[test]
    fn build_blocks_has_header_and_context() {
        let blocks = build_blocks(&test_event(), "test description");
        let blocks_arr = blocks.as_array().unwrap();
        assert!(blocks_arr.len() >= 3);
        assert_eq!(blocks_arr[0]["type"], "header");
        assert_eq!(blocks_arr[blocks_arr.len() - 1]["type"], "context");
    }

    #[test]
    fn build_blocks_includes_session_field() {
        let blocks = build_blocks(&test_event(), "desc");
        let json_str = serde_json::to_string(&blocks).unwrap();
        assert!(json_str.contains("sess-456"));
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omx-notify-slack -- --nocapture`
Expected: All 6 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-notify-slack/Cargo.toml crates/omx-notify-slack/src/main.rs
git commit -m "feat(omx-notify-slack): add Block Kit formatting, emoji mapping, and mention support"
```

---

## Task 8: Telegram — HTML Formatting and Reply-To Support

**Files:**
- Modify: `crates/omx-notify-telegram/Cargo.toml`
- Modify: `crates/omx-notify-telegram/src/main.rs`

- [ ] **Step 1: Add omx-notify-template dependency**

Add to `[dependencies]` in `crates/omx-notify-telegram/Cargo.toml`:

```toml
omx-notify-template = { path = "../omx-notify-template" }
```

- [ ] **Step 2: Replace `crates/omx-notify-telegram/src/main.rs`**

```rust
use omx_notify_template::{render, TemplateContext};
use omx_types::{HookEvent, HookEventName};
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str = "{{#if error}}\n<b>Error:</b> <code>{{error}}</code>{{/if}}{{#if worker_id}}\n<b>Worker:</b> <code>{{worker_id}}</code>{{/if}}{{#if branch}}\n<b>Branch:</b> <code>{{branch}}</code>{{/if}}";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let event: HookEvent = serde_json::from_str(&input)?;
    let (success, stdout, stderr) = send_telegram_notification(&event).await;

    let result = serde_json::json!({
        "hook": "omx-notify-telegram",
        "success": success,
        "stdout": stdout,
        "stderr": stderr,
        "duration_ms": 0
    });

    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn telegram_emoji(event: &HookEventName) -> &'static str {
    match event {
        HookEventName::Failed | HookEventName::TestFailed => "\u{274c}",     // cross mark
        HookEventName::Blocked => "\u{26a0}\u{fe0f}",                         // warning
        HookEventName::Finished | HookEventName::TestFinished => "\u{2705}",  // check mark
        HookEventName::SessionStart => "\u{1f680}",                            // rocket
        HookEventName::SessionEnd => "\u{1f3c1}",                             // checkered flag
        HookEventName::SessionIdle => "\u{1f4a4}",                            // zzz
        _ => "\u{1f535}",                                                      // blue circle
    }
}

async fn send_telegram_notification(event: &HookEvent) -> (bool, String, String) {
    let bot_token = match std::env::var("OMX_TELEGRAM_BOT_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            return (
                false,
                String::new(),
                "OMX_TELEGRAM_BOT_TOKEN not set".into(),
            )
        }
    };

    let chat_id = match std::env::var("OMX_TELEGRAM_CHAT_ID") {
        Ok(c) => c,
        Err(_) => return (false, String::new(), "OMX_TELEGRAM_CHAT_ID not set".into()),
    };

    let mention = std::env::var("OMX_TELEGRAM_MENTION").ok();
    let reply_to = std::env::var("OMX_TELEGRAM_REPLY_TO").ok();
    let template = std::env::var("OMX_TELEGRAM_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let body = render(&template, &ctx);

    let emoji = telegram_emoji(&event.event);
    let mut text = format!(
        "{emoji} <b>OMX: {}</b>\n<i>Source: {}</i>",
        event.event, event.source.component
    );

    if let Some(ref sid) = event.session_id {
        text.push_str(&format!("\n<b>Session:</b> <code>{sid}</code>"));
    }

    if !body.trim().is_empty() {
        text.push_str(&body);
    }

    text.push_str(&format!("\n<i>{}</i>", event.timestamp));

    if let Some(ref m) = mention {
        text = format!("{m}\n{text}");
    }

    // Telegram max message length is 4096 chars
    if text.len() > 4096 {
        text.truncate(4093);
        text.push_str("...");
    }

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
    let client = reqwest::Client::new();
    let mut payload = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "HTML"
    });

    if let Some(ref reply_id) = reply_to {
        if let Ok(id) = reply_id.parse::<i64>() {
            payload["reply_to_message_id"] = serde_json::json!(id);
        }
    }

    match client.post(&url).json(&payload).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, format!("Telegram: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (
                    false,
                    String::new(),
                    format!("Telegram API error {status}: {body}"),
                )
            }
        }
        Err(e) => (
            false,
            String::new(),
            format!("Telegram request failed: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::HookSource;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::PrCreated,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({"pr": "#42"}),
            session_id: Some("sess-789".into()),
        }
    }

    #[tokio::test]
    async fn returns_error_when_bot_token_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("OMX_TELEGRAM_BOT_TOKEN");
        std::env::remove_var("OMX_TELEGRAM_CHAT_ID");
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_TELEGRAM_BOT_TOKEN not set"));
    }

    #[tokio::test]
    async fn returns_error_when_chat_id_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_TELEGRAM_BOT_TOKEN", "fake-token");
        std::env::remove_var("OMX_TELEGRAM_CHAT_ID");
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_TELEGRAM_CHAT_ID not set"));
    }

    #[tokio::test]
    async fn returns_error_for_unreachable_url() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_TELEGRAM_BOT_TOKEN", "fake-token");
        std::env::set_var("OMX_TELEGRAM_CHAT_ID", "12345");
        std::env::remove_var("OMX_TELEGRAM_MENTION");
        std::env::remove_var("OMX_TELEGRAM_REPLY_TO");
        std::env::remove_var("OMX_TELEGRAM_TEMPLATE");
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        if !success {
            assert!(
                stderr.contains("request failed") || stderr.contains("API error"),
                "stderr should describe the failure: {stderr}"
            );
        }
    }

    #[test]
    fn telegram_emoji_for_failed() {
        assert_eq!(telegram_emoji(&HookEventName::Failed), "\u{274c}");
    }

    #[test]
    fn telegram_emoji_for_session_start() {
        assert_eq!(telegram_emoji(&HookEventName::SessionStart), "\u{1f680}");
    }

    #[test]
    fn default_template_renders_without_error() {
        let ctx = TemplateContext::from_event(&test_event());
        let result = render(DEFAULT_TEMPLATE, &ctx);
        // No error, worker_id, or branch in test_event
        assert_eq!(result.trim(), "");
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omx-notify-telegram -- --nocapture`
Expected: All 6 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-notify-telegram/Cargo.toml crates/omx-notify-telegram/src/main.rs
git commit -m "feat(omx-notify-telegram): add HTML formatting, emoji, mention, and reply-to support"
```

---

## Task 9: Notification Router — `omx-hooks/src/router.rs`

**Files:**
- Create: `crates/omx-hooks/src/router.rs`
- Modify: `crates/omx-hooks/src/lib.rs`
- Modify: `crates/omx-hooks/Cargo.toml`

- [ ] **Step 1: Add omx-config dependency to omx-hooks**

Add to `[dependencies]` in `crates/omx-hooks/Cargo.toml`:

```toml
omx-config = { path = "../omx-config" }
```

- [ ] **Step 2: Create `crates/omx-hooks/src/router.rs`**

```rust
use omx_config::NotificationConfig;
use omx_types::HookEvent;
use serde::{Deserialize, Serialize};

/// Which notification platforms to route a given event to.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub discord: bool,
    pub slack: bool,
    pub telegram: bool,
    pub pushover: bool,
    pub generic: bool,
}

impl RoutingDecision {
    /// Returns the list of platform names that are enabled.
    pub fn enabled_platforms(&self) -> Vec<&'static str> {
        let mut platforms = Vec::new();
        if self.discord {
            platforms.push("discord");
        }
        if self.slack {
            platforms.push("slack");
        }
        if self.telegram {
            platforms.push("telegram");
        }
        if self.pushover {
            platforms.push("pushover");
        }
        if self.generic {
            platforms.push("generic");
        }
        platforms
    }

    /// Returns the binary name for each enabled platform.
    pub fn binary_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.discord {
            names.push("omx-notify-discord");
        }
        if self.slack {
            names.push("omx-notify-slack");
        }
        if self.telegram {
            names.push("omx-notify-telegram");
        }
        if self.pushover {
            names.push("omx-notify-pushover");
        }
        if self.generic {
            names.push("omx-notify-generic");
        }
        names
    }
}

/// Determine which platforms to notify based on what's configured.
/// A platform is routed to if its config section exists (i.e., user has configured it).
pub fn route(_event: &HookEvent, config: &NotificationConfig) -> RoutingDecision {
    RoutingDecision {
        discord: config.discord.is_some(),
        slack: config.slack.is_some(),
        telegram: config.telegram.is_some(),
        pushover: config.pushover.is_some(),
        generic: config.generic.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_config::{DiscordConfig, PushoverConfig, SlackConfig};
    use omx_types::{HookEventName, HookSource};

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: None,
        }
    }

    #[test]
    fn route_empty_config_routes_nowhere() {
        let config = NotificationConfig::default();
        let decision = route(&test_event(), &config);
        assert!(decision.enabled_platforms().is_empty());
        assert!(decision.binary_names().is_empty());
    }

    #[test]
    fn route_discord_only() {
        let mut config = NotificationConfig::default();
        config.discord = Some(DiscordConfig {
            webhook_url: "https://discord.com/webhook".into(),
            mention: None,
        });
        let decision = route(&test_event(), &config);
        assert_eq!(decision.enabled_platforms(), vec!["discord"]);
        assert_eq!(decision.binary_names(), vec!["omx-notify-discord"]);
    }

    #[test]
    fn route_multiple_platforms() {
        let mut config = NotificationConfig::default();
        config.discord = Some(DiscordConfig {
            webhook_url: "https://discord.com/webhook".into(),
            mention: None,
        });
        config.slack = Some(SlackConfig {
            webhook_url: "https://hooks.slack.com/webhook".into(),
            mention: None,
        });
        config.pushover = Some(PushoverConfig {
            user_key: "user".into(),
            app_token: "app".into(),
            device: None,
        });
        let decision = route(&test_event(), &config);
        let platforms = decision.enabled_platforms();
        assert_eq!(platforms.len(), 3);
        assert!(platforms.contains(&"discord"));
        assert!(platforms.contains(&"slack"));
        assert!(platforms.contains(&"pushover"));
    }

    #[test]
    fn binary_names_match_crate_names() {
        let decision = RoutingDecision {
            discord: true,
            slack: true,
            telegram: true,
            pushover: true,
            generic: true,
        };
        let names = decision.binary_names();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"omx-notify-discord"));
        assert!(names.contains(&"omx-notify-slack"));
        assert!(names.contains(&"omx-notify-telegram"));
        assert!(names.contains(&"omx-notify-pushover"));
        assert!(names.contains(&"omx-notify-generic"));
    }
}
```

- [ ] **Step 3: Add `pub mod router;` to `crates/omx-hooks/src/lib.rs`**

Add at the top, after the existing module declarations:

```rust
pub mod router;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-hooks -- --nocapture`
Expected: All tests PASS (existing + 4 new router tests)

- [ ] **Step 5: Commit**

```bash
git add crates/omx-hooks/Cargo.toml crates/omx-hooks/src/router.rs crates/omx-hooks/src/lib.rs
git commit -m "feat(omx-hooks): add NotificationRouter that maps events to configured platforms"
```

---

## Task 10: Add Notification OmxError Variant

**Files:**
- Modify: `crates/omx-types/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to the test module in `crates/omx-types/src/lib.rs`:

```rust
#[test]
fn notification_error_display() {
    let err = OmxError::Notification("pushover timeout".into());
    assert_eq!(err.to_string(), "notification error: pushover timeout");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p omx-types notification_error -- --nocapture 2>&1 | head -10`
Expected: FAIL — no variant `Notification`

- [ ] **Step 3: Add `Notification` variant to `OmxError`**

Add to the `OmxError` enum in `crates/omx-types/src/lib.rs`, after the `Pipeline` variant:

```rust
#[error("notification error: {0}")]
Notification(String),
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omx-types -- --nocapture`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-types/src/lib.rs
git commit -m "feat(omx-types): add Notification variant to OmxError"
```

---

## Task 11: Full Workspace Build Verification

**Files:** None (verification only)

- [ ] **Step 1: Full workspace build**

Run: `cargo build --workspace 2>&1 | tail -5`
Expected: Clean build, no errors

- [ ] **Step 2: Full workspace tests**

Run: `cargo test --workspace 2>&1 | tail -40`
Expected: All tests pass, 0 failures

- [ ] **Step 3: Clippy**

Run: `cargo clippy --workspace -- -D warnings 2>&1 | tail -10`
Expected: No warnings

- [ ] **Step 4: Format check**

Run: `cargo fmt --all -- --check`
Expected: No formatting issues
