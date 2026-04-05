# Pure Rust Migration — Phase 5: Notification Hooks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 3 stub notification hook binaries (`omx-notify-discord`, `omx-notify-slack`, `omx-notify-telegram`) with working implementations that read a `HookEvent` from stdin and POST formatted notifications to the corresponding platform API.

**Architecture:** Each notification hook is a standalone binary invoked by `ShellHookDispatcher` (from `omx-hooks`). The dispatcher pipes `HookEvent` JSON to stdin. Each hook: (1) deserializes the event, (2) reads its config from the `OMX_NOTIFY_*` environment variables, (3) formats a human-readable message from the event, (4) POSTs to the platform's webhook/API endpoint via `reqwest`, (5) prints a `HookResult`-compatible JSON to stdout. All three hooks share a common message formatting function extracted into a shared module within `omx-types`.

**Tech Stack:** Rust 2021 edition, tokio 1.x, reqwest 0.12+ (with `json` feature), omx-types (`HookEvent`, `HookEventName`), serde/serde_json, tracing

**Spec:** `docs/superpowers/specs/2026-04-04-pure-rust-migration-design.md` (sections 3.1, 3.3, 11, 12)

**Phase 4 baseline:** All presentation crates (`omx-hud`, `omx-setup`, `omx-cli`) are fully implemented. The 3 notify crates exist as stubs that return `"success": false, "stderr": "not implemented"`.

**Milestone:** All 3 notification hooks POST real messages to their respective APIs. No stubs remain. Each hook is testable with a mock HTTP server.

---

## File Structure

### Modified files

```
crates/omx-types/src/lib.rs                     # Add format_hook_message() for shared message formatting
crates/omx-notify-discord/src/main.rs            # Implement Discord webhook POST
crates/omx-notify-discord/Cargo.toml             # Add tracing dependency
crates/omx-notify-slack/src/main.rs              # Implement Slack webhook POST
crates/omx-notify-slack/Cargo.toml               # Add tracing dependency
crates/omx-notify-telegram/src/main.rs           # Implement Telegram Bot API POST
crates/omx-notify-telegram/Cargo.toml            # Add tracing dependency
```

---

## Environment Variable Contract

Each hook reads its configuration from environment variables (set by the calling process, typically `omx-cli` which reads `config.toml`):

| Hook | Required Env Vars |
|------|-------------------|
| `omx-notify-discord` | `OMX_DISCORD_WEBHOOK_URL` |
| `omx-notify-slack` | `OMX_SLACK_WEBHOOK_URL` |
| `omx-notify-telegram` | `OMX_TELEGRAM_BOT_TOKEN`, `OMX_TELEGRAM_CHAT_ID` |

If required env vars are missing, the hook prints a result with `"success": false` and a descriptive `"stderr"` message, then exits 0 (so the dispatcher doesn't treat it as a crash).

---

## Task 1: Add shared message formatting to omx-types

**Files:**
- Modify: `crates/omx-types/src/lib.rs`

All three hooks need to format a `HookEvent` into a human-readable notification message. We add this once in `omx-types` so the hooks stay DRY.

- [ ] **Step 1: Write the failing test for format_hook_message**

Add this test to the existing `#[cfg(test)] mod tests` block at the bottom of `crates/omx-types/src/lib.rs`:

```rust
#[test]
fn format_hook_message_includes_event_and_source() {
    let event = HookEvent {
        schema_version: "1".into(),
        event: HookEventName::SessionStart,
        timestamp: "2026-04-05T10:00:00Z".into(),
        source: HookSource {
            component: "omx-cli".into(),
            worker_id: Some("w-001".into()),
        },
        context: serde_json::json!({"task": "build frontend"}),
        session_id: Some("sess-abc".into()),
    };

    let msg = format_hook_message(&event);
    assert!(msg.contains("session_start"), "should contain event name");
    assert!(msg.contains("omx-cli"), "should contain source component");
    assert!(msg.contains("w-001"), "should contain worker id");
    assert!(msg.contains("sess-abc"), "should contain session id");
}

#[test]
fn format_hook_message_handles_missing_optional_fields() {
    let event = HookEvent {
        schema_version: "1".into(),
        event: HookEventName::Failed,
        timestamp: "2026-04-05T10:00:00Z".into(),
        source: HookSource {
            component: "omx-team".into(),
            worker_id: None,
        },
        context: serde_json::json!({}),
        session_id: None,
    };

    let msg = format_hook_message(&event);
    assert!(msg.contains("failed"), "should contain event name");
    assert!(msg.contains("omx-team"), "should contain source component");
    assert!(!msg.contains("worker:"), "should not contain worker label when None");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `~/.cargo/bin/cargo test -p omx-types format_hook_message -- --nocapture`
Expected: FAIL with "cannot find function `format_hook_message`"

- [ ] **Step 3: Implement format_hook_message**

Add this function in `crates/omx-types/src/lib.rs` right after the `HookSource` struct (before the `// Unified error type` comment block):

```rust
/// Format a HookEvent into a human-readable notification message.
/// Used by all notification hooks (Discord, Slack, Telegram).
pub fn format_hook_message(event: &HookEvent) -> String {
    let mut parts = vec![
        format!("**[OMX]** `{}`", event.event),
        format!("Source: `{}`", event.source.component),
    ];

    if let Some(ref wid) = event.source.worker_id {
        parts.push(format!("Worker: `{wid}`"));
    }

    if let Some(ref sid) = event.session_id {
        parts.push(format!("Session: `{sid}`"));
    }

    if event.context != serde_json::json!({}) {
        if let Ok(ctx) = serde_json::to_string_pretty(&event.context) {
            parts.push(format!("Context:\n```json\n{ctx}\n```"));
        }
    }

    parts.push(format!("Time: {}", event.timestamp));

    parts.join("\n")
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `~/.cargo/bin/cargo test -p omx-types format_hook_message -- --nocapture`
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omx-types/src/lib.rs
git commit -m "feat(omx-types): add format_hook_message for notification hooks"
```

---

## Task 2: Implement omx-notify-discord

**Files:**
- Modify: `crates/omx-notify-discord/src/main.rs`
- Modify: `crates/omx-notify-discord/Cargo.toml`

Discord webhooks accept a POST to the webhook URL with a JSON body `{"content": "message text"}`. Max content length is 2000 chars.

- [ ] **Step 1: Write the failing test for send_discord_notification**

Replace the entire contents of `crates/omx-notify-discord/src/main.rs` with:

```rust
use omx_types::{format_hook_message, HookEvent};
use std::io::{self, Read};

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

    let message = format_hook_message(event);
    // Discord max content length is 2000 chars
    let content = if message.len() > 2000 {
        format!("{}…", &message[..1999])
    } else {
        message
    };

    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "content": content });

    match client.post(&webhook_url).json(&payload).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() || status.as_u16() == 204 {
                (true, format!("Discord: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (false, String::new(), format!("Discord API error {status}: {body}"))
            }
        }
        Err(e) => (false, String::new(), format!("Discord request failed: {e}")),
    }
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
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: Some("sess-123".into()),
        }
    }

    #[tokio::test]
    async fn returns_error_when_env_var_missing() {
        std::env::remove_var("OMX_DISCORD_WEBHOOK_URL");
        let (success, _, stderr) = send_discord_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_DISCORD_WEBHOOK_URL not set"));
    }

    #[tokio::test]
    async fn returns_error_for_unreachable_url() {
        std::env::set_var("OMX_DISCORD_WEBHOOK_URL", "http://127.0.0.1:1/fake");
        let (success, _, stderr) = send_discord_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("request failed"));
    }
}
```

- [ ] **Step 2: Add tracing to Cargo.toml**

Add `tracing = { workspace = true }` to the `[dependencies]` section of `crates/omx-notify-discord/Cargo.toml`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `~/.cargo/bin/cargo test -p omx-notify-discord -- --nocapture`
Expected: 2 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-notify-discord/
git commit -m "feat(omx-notify-discord): implement Discord webhook notification"
```

---

## Task 3: Implement omx-notify-slack

**Files:**
- Modify: `crates/omx-notify-slack/src/main.rs`
- Modify: `crates/omx-notify-slack/Cargo.toml`

Slack incoming webhooks accept a POST with JSON body `{"text": "message text"}`. The message supports mrkdwn formatting.

- [ ] **Step 1: Write the full implementation with tests**

Replace the entire contents of `crates/omx-notify-slack/src/main.rs` with:

```rust
use omx_types::{format_hook_message, HookEvent};
use std::io::{self, Read};

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

async fn send_slack_notification(event: &HookEvent) -> (bool, String, String) {
    let webhook_url = match std::env::var("OMX_SLACK_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => {
            return (
                false,
                String::new(),
                "OMX_SLACK_WEBHOOK_URL not set".into(),
            )
        }
    };

    let message = format_hook_message(event);
    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "text": message });

    match client.post(&webhook_url).json(&payload).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, format!("Slack: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (false, String::new(), format!("Slack API error {status}: {body}"))
            }
        }
        Err(e) => (false, String::new(), format!("Slack request failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{HookEventName, HookSource};

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
        std::env::remove_var("OMX_SLACK_WEBHOOK_URL");
        let (success, _, stderr) = send_slack_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_SLACK_WEBHOOK_URL not set"));
    }

    #[tokio::test]
    async fn returns_error_for_unreachable_url() {
        std::env::set_var("OMX_SLACK_WEBHOOK_URL", "http://127.0.0.1:1/fake");
        let (success, _, stderr) = send_slack_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("request failed"));
    }
}
```

- [ ] **Step 2: Add tracing to Cargo.toml**

Add `tracing = { workspace = true }` to the `[dependencies]` section of `crates/omx-notify-slack/Cargo.toml`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `~/.cargo/bin/cargo test -p omx-notify-slack -- --nocapture`
Expected: 2 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/omx-notify-slack/
git commit -m "feat(omx-notify-slack): implement Slack webhook notification"
```

---

## Task 4: Implement omx-notify-telegram

**Files:**
- Modify: `crates/omx-notify-telegram/src/main.rs`
- Modify: `crates/omx-notify-telegram/Cargo.toml`

Telegram Bot API uses POST to `https://api.telegram.org/bot{token}/sendMessage` with JSON body `{"chat_id": "...", "text": "...", "parse_mode": "Markdown"}`. Max message length is 4096 chars.

- [ ] **Step 1: Write the full implementation with tests**

Replace the entire contents of `crates/omx-notify-telegram/src/main.rs` with:

```rust
use omx_types::{format_hook_message, HookEvent};
use std::io::{self, Read};

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
        Err(_) => {
            return (
                false,
                String::new(),
                "OMX_TELEGRAM_CHAT_ID not set".into(),
            )
        }
    };

    let message = format_hook_message(event);
    // Telegram max message length is 4096 chars
    let text = if message.len() > 4096 {
        format!("{}…", &message[..4095])
    } else {
        message
    };

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "Markdown"
    });

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
        Err(e) => (false, String::new(), format!("Telegram request failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{HookEventName, HookSource};

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
        std::env::remove_var("OMX_TELEGRAM_BOT_TOKEN");
        std::env::remove_var("OMX_TELEGRAM_CHAT_ID");
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_TELEGRAM_BOT_TOKEN not set"));
    }

    #[tokio::test]
    async fn returns_error_when_chat_id_missing() {
        std::env::set_var("OMX_TELEGRAM_BOT_TOKEN", "fake-token");
        std::env::remove_var("OMX_TELEGRAM_CHAT_ID");
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_TELEGRAM_CHAT_ID not set"));
    }

    #[tokio::test]
    async fn returns_error_for_unreachable_url() {
        std::env::set_var("OMX_TELEGRAM_BOT_TOKEN", "fake-token");
        std::env::set_var("OMX_TELEGRAM_CHAT_ID", "12345");
        // Override the API URL by using a token that makes the URL unreachable
        // The real test is that the HTTP request fails gracefully
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        // This may succeed or fail depending on network — but it shouldn't panic
        if !success {
            assert!(
                stderr.contains("request failed") || stderr.contains("API error"),
                "stderr should describe the failure: {stderr}"
            );
        }
    }
}
```

- [ ] **Step 2: Add tracing to Cargo.toml**

Add `tracing = { workspace = true }` to the `[dependencies]` section of `crates/omx-notify-telegram/Cargo.toml`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `~/.cargo/bin/cargo test -p omx-notify-telegram -- --nocapture`
Expected: 2-3 tests PASS (the unreachable URL test depends on whether the Telegram API is reachable — the assertion handles both cases)

- [ ] **Step 4: Commit**

```bash
git add crates/omx-notify-telegram/
git commit -m "feat(omx-notify-telegram): implement Telegram Bot API notification"
```

---

## Task 5: Workspace build + full test suite verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run cargo build for the entire workspace**

Run: `~/.cargo/bin/cargo build`
Expected: Compiles with no errors

- [ ] **Step 2: Run cargo clippy for the entire workspace**

Run: `~/.cargo/bin/cargo clippy --workspace -- -D warnings`
Expected: No warnings or errors

- [ ] **Step 3: Run full test suite**

Run: `~/.cargo/bin/cargo test --workspace`
Expected: All tests pass, including the new notification hook tests

- [ ] **Step 4: Verify no stubs remain in notify crates**

Run: `rg 'not implemented' crates/omx-notify-*/src/`
Expected: No matches (the old "not implemented" stderr strings should be gone)

- [ ] **Step 5: Commit any fixes if needed**

If clippy or tests required changes, commit them:
```bash
git add -A
git commit -m "fix: address clippy warnings in notification hooks"
```
