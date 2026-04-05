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
        let (success, _, stderr) = send_telegram_notification(&test_event()).await;
        // May succeed or fail depending on network — but shouldn't panic
        if !success {
            assert!(
                stderr.contains("request failed") || stderr.contains("API error"),
                "stderr should describe the failure: {stderr}"
            );
        }
    }
}
