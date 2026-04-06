use omx_notify_template::{render, TemplateContext};
use omx_types::HookEvent;
use std::collections::HashMap;
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str =
    "[OMX] {{mode}} — {{timestamp}}{{#if error}}\nError: {{error}}{{/if}}";

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
    let webhook_url = match std::env::var("OMX_GENERIC_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => {
            return (
                false,
                String::new(),
                "OMX_GENERIC_WEBHOOK_URL not set".into(),
            )
        }
    };

    let template = std::env::var("OMX_GENERIC_TEMPLATE")
        .unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let message = render(&template, &ctx);

    let payload = serde_json::json!({
        "event": event.event.to_string(),
        "message": message,
        "timestamp": event.timestamp,
        "source": format!("{}", event.source.component),
        "session_id": event.session_id.clone().unwrap_or_default(),
        "context": event.context,
    });

    let client = reqwest::Client::new();
    let mut request = client
        .post(&webhook_url)
        .header("Content-Type", "application/json");

    if let Ok(token) = std::env::var("OMX_GENERIC_AUTH_TOKEN") {
        request = request.header("Authorization", token);
    }

    for (key, value) in parse_headers_env() {
        request = request.header(key, value);
    }

    match request.json(&payload).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, format!("Generic webhook: {status}"), String::new())
            } else {
                let body = resp.text().await.unwrap_or_default();
                (
                    false,
                    String::new(),
                    format!("Generic webhook error {status}: {body}"),
                )
            }
        }
        Err(e) => (
            false,
            String::new(),
            format!("Generic webhook request failed: {e}"),
        ),
    }
}

fn parse_headers_env() -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if let Ok(raw) = std::env::var("OMX_GENERIC_HEADERS") {
        for pair in raw.split(',') {
            let pair = pair.trim();
            if let Some((key, value)) = pair.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                if !key.is_empty() {
                    headers.insert(key, value);
                }
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
            event: HookEventName::TurnComplete,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "test".into(),
                worker_id: Some("w-002".into()),
            },
            context: serde_json::json!({"mode": "solo", "task": "deploy"}),
            session_id: Some("sess-456".into()),
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
        let (success, _, stderr) = send_generic_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("request failed"));
    }

    #[test]
    fn parse_headers_env_parses_comma_separated_pairs() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("OMX_GENERIC_HEADERS", "X-Foo:bar, X-Baz:qux");
        let headers = parse_headers_env();
        assert_eq!(headers.get("X-Foo").unwrap(), "bar");
        assert_eq!(headers.get("X-Baz").unwrap(), "qux");
        std::env::remove_var("OMX_GENERIC_HEADERS");
    }

    #[test]
    fn parse_headers_env_empty_when_unset() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("OMX_GENERIC_HEADERS");
        let headers = parse_headers_env();
        assert!(headers.is_empty());
    }

    #[test]
    fn default_template_renders() {
        let event = test_event();
        let ctx = TemplateContext::from_event(&event);
        let result = render(DEFAULT_TEMPLATE, &ctx);
        assert!(result.contains("[OMX]"));
        assert!(result.contains("solo"));
        assert!(result.contains("2026-04-05T10:00:00Z"));
    }
}
