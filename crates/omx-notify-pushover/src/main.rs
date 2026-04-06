use omx_notify_template::{render, TemplateContext};
use omx_types::{HookEvent, HookEventName};
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

fn pushover_priority(event: &HookEvent) -> i8 {
    match event.event {
        HookEventName::Failed | HookEventName::TestFailed => 1,
        HookEventName::SessionStart | HookEventName::SessionEnd | HookEventName::SessionIdle => -1,
        _ => 0,
    }
}

async fn send_pushover_notification(event: &HookEvent) -> (bool, String, String) {
    let user_key = match std::env::var("OMX_PUSHOVER_USER_KEY") {
        Ok(key) => key,
        Err(_) => return (false, String::new(), "OMX_PUSHOVER_USER_KEY not set".into()),
    };

    let app_token = match std::env::var("OMX_PUSHOVER_APP_TOKEN") {
        Ok(token) => token,
        Err(_) => return (false, String::new(), "OMX_PUSHOVER_APP_TOKEN not set".into()),
    };

    let device = std::env::var("OMX_PUSHOVER_DEVICE").ok();

    let template = std::env::var("OMX_PUSHOVER_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let message = render(&template, &ctx);
    let priority = pushover_priority(event);

    let client = reqwest::Client::new();
    let mut form = vec![
        ("token", app_token),
        ("user", user_key),
        ("message", message),
        ("priority", priority.to_string()),
        ("title", format!("OMX {}", event.event)),
    ];

    if let Some(dev) = device {
        form.push(("device", dev));
    }

    match client
        .post("https://api.pushover.net/1/messages.json")
        .form(&form)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, format!("Pushover: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (
                    false,
                    String::new(),
                    format!("Pushover API error {status}: {body}"),
                )
            }
        }
        Err(e) => (false, String::new(), format!("Pushover request failed: {e}")),
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
            context: serde_json::json!({"task": "deploy", "mode": "team"}),
            session_id: Some("sess-456".into()),
        }
    }

    fn event_with_name(name: HookEventName) -> HookEvent {
        HookEvent {
            event: name,
            ..test_event()
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
        std::env::set_var("OMX_PUSHOVER_USER_KEY", "test-user-key");
        std::env::remove_var("OMX_PUSHOVER_APP_TOKEN");
        let (success, _, stderr) = send_pushover_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("OMX_PUSHOVER_APP_TOKEN not set"));
        std::env::remove_var("OMX_PUSHOVER_USER_KEY");
    }

    #[test]
    fn priority_mapping_failed_is_high() {
        let event = event_with_name(HookEventName::Failed);
        assert_eq!(pushover_priority(&event), 1);
    }

    #[test]
    fn priority_mapping_session_start_is_low() {
        let event = event_with_name(HookEventName::SessionStart);
        assert_eq!(pushover_priority(&event), -1);
    }

    #[test]
    fn priority_mapping_turn_complete_is_normal() {
        let event = event_with_name(HookEventName::TurnComplete);
        assert_eq!(pushover_priority(&event), 0);
    }

    #[test]
    fn default_template_renders() {
        let event = test_event();
        let ctx = TemplateContext::from_event(&event);
        let result = render(DEFAULT_TEMPLATE, &ctx);
        assert!(result.contains("team"));
        assert!(result.contains("2026-04-05T10:00:00Z"));
        assert!(result.contains("Worker: w-002"));
    }
}
