use omx_notify_template::{render, TemplateContext};
use omx_types::{HookEvent, HookEventName};
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str = "{{#if error}}\n<b>Error:</b> <code>{{error}}</code>{{/if}}{{#if worker_id}}\n<b>Worker:</b> <code>{{worker_id}}</code>{{/if}}{{#if branch}}\n<b>Branch:</b> <code>{{branch}}</code>{{/if}}";

fn telegram_emoji(event: &HookEventName) -> &'static str {
    match event {
        HookEventName::Failed | HookEventName::TestFailed => "\u{274c}",
        HookEventName::Blocked => "\u{26a0}\u{fe0f}",
        HookEventName::Finished | HookEventName::TestFinished => "\u{2705}",
        HookEventName::SessionStart => "\u{1f680}",
        HookEventName::SessionEnd => "\u{1f3c1}",
        HookEventName::SessionIdle => "\u{1f4a4}",
        _ => "\u{1f535}",
    }
}

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
        Err(_) => return (false, String::new(), "OMX_TELEGRAM_CHAT_ID not set".into()),
    };

    let mention = std::env::var("OMX_TELEGRAM_MENTION").ok();
    let reply_to = std::env::var("OMX_TELEGRAM_REPLY_TO")
        .ok()
        .and_then(|v| v.parse::<i64>().ok());
    let template = std::env::var("OMX_TELEGRAM_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let body = render(&template, &ctx);

    let emoji = telegram_emoji(&event.event);
    let title = format!("{emoji} <b>[OMX] {}</b>", event.event);
    let source = format!("<i>Source: {}</i>", event.source.component);
    let session = event
        .session_id
        .as_deref()
        .map(|s| format!("<code>{s}</code>"))
        .unwrap_or_default();
    let timestamp = format!("<i>{}</i>", event.timestamp);

    let mut parts: Vec<String> = Vec::new();
    if let Some(m) = &mention {
        parts.push(m.clone());
    }
    parts.push(title);
    parts.push(source);
    if !session.is_empty() {
        parts.push(session);
    }
    if !body.is_empty() {
        parts.push(body);
    }
    parts.push(timestamp);

    let mut message = parts.join("\n");
    if message.len() > 4096 {
        message.truncate(4093);
        message.push_str("...");
    }

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
    let client = reqwest::Client::new();

    let mut payload = serde_json::json!({
        "chat_id": chat_id,
        "text": message,
        "parse_mode": "HTML"
    });

    if let Some(reply_id) = reply_to {
        payload
            .as_object_mut()
            .unwrap()
            .insert("reply_to_message_id".to_string(), serde_json::json!(reply_id));
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
        std::env::remove_var("OMX_TELEGRAM_MENTION");
        std::env::remove_var("OMX_TELEGRAM_REPLY_TO");
        std::env::remove_var("OMX_TELEGRAM_TEMPLATE");
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_TELEGRAM_BOT_TOKEN not set"));
    }

    #[tokio::test]
    async fn returns_error_when_chat_id_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_TELEGRAM_BOT_TOKEN", "fake-token");
        std::env::remove_var("OMX_TELEGRAM_CHAT_ID");
        std::env::remove_var("OMX_TELEGRAM_MENTION");
        std::env::remove_var("OMX_TELEGRAM_REPLY_TO");
        std::env::remove_var("OMX_TELEGRAM_TEMPLATE");
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
        let event = test_event();
        let ctx = TemplateContext::from_event(&event);
        let result = render(DEFAULT_TEMPLATE, &ctx);
        // test event has no error, worker_id, or branch — all conditionals should be empty
        assert!(result.is_empty(), "expected empty but got: {result}");
    }
}
