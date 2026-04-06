use omx_notify_template::{render, TemplateContext};
use omx_types::{HookEvent, HookEventName};
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str =
    "{{#if error}}Error: {{error}}{{/if}}{{#if worker_id}}Worker: {{worker_id}}{{/if}}{{#if branch}}\nBranch: {{branch}}{{/if}}";

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
        HookEventName::Failed | HookEventName::TestFailed => 0xFF0000,
        HookEventName::Blocked => 0xFFA500,
        HookEventName::Finished | HookEventName::TestFinished => 0x00FF00,
        HookEventName::SessionStart => 0x0099FF,
        HookEventName::SessionEnd => 0x808080,
        _ => 0x7289DA,
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

    let template = std::env::var("OMX_DISCORD_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let description = render(&template, &ctx);

    let mut embed = serde_json::json!({
        "title": format!("OMX: {}", event.event),
        "description": description,
        "color": embed_color(&event.event),
        "timestamp": &event.timestamp,
        "footer": { "text": format!("Source: {}", event.source.component) }
    });

    if let Some(ref sid) = event.session_id {
        embed["fields"] = serde_json::json!([
            { "name": "Session", "value": sid, "inline": true }
        ]);
    }

    let mut payload = serde_json::json!({ "embeds": [embed] });

    if let Ok(mention) = std::env::var("OMX_DISCORD_MENTION") {
        payload["content"] = serde_json::Value::String(mention);
    }

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
        assert_eq!(embed_color(&HookEventName::TestFailed), 0xFF0000);
    }

    #[test]
    fn embed_color_green_for_finished() {
        assert_eq!(embed_color(&HookEventName::Finished), 0x00FF00);
        assert_eq!(embed_color(&HookEventName::TestFinished), 0x00FF00);
    }

    #[test]
    fn embed_color_blue_for_session_start() {
        assert_eq!(embed_color(&HookEventName::SessionStart), 0x0099FF);
    }

    #[test]
    fn default_template_renders_without_error() {
        let event = HookEvent {
            schema_version: "1".into(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: None,
        };
        let ctx = TemplateContext::from_event(&event);
        let result = render(DEFAULT_TEMPLATE, &ctx);
        assert_eq!(result.trim(), "");
    }
}
