use omx_notify_template::{render, TemplateContext};
use omx_types::{HookEvent, HookEventName};
use std::io::{self, Read};

const DEFAULT_TEMPLATE: &str =
    "{{#if error}}:x: Error: {{error}}{{/if}}{{#if worker_id}}\nWorker: `{{worker_id}}`{{/if}}{{#if branch}}\nBranch: `{{branch}}`{{/if}}";

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
    let header_text = format!("{emoji} {}", event.event);

    let mut blocks = vec![serde_json::json!({
        "type": "header",
        "text": {
            "type": "plain_text",
            "text": header_text,
            "emoji": true
        }
    })];

    if !description.is_empty() {
        blocks.push(serde_json::json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": description
            }
        }));
    }

    let mut fields = vec![serde_json::json!({
        "type": "mrkdwn",
        "text": format!("*Source:*\n{}", event.source.component)
    })];

    if let Some(ref sid) = event.session_id {
        fields.push(serde_json::json!({
            "type": "mrkdwn",
            "text": format!("*Session:*\n{sid}")
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
            "text": format!(":clock1: {}", event.timestamp)
        }]
    }));

    serde_json::Value::Array(blocks)
}

async fn send_slack_notification(event: &HookEvent) -> (bool, String, String) {
    let webhook_url = match std::env::var("OMX_SLACK_WEBHOOK_URL") {
        Ok(url) => url,
        Err(_) => return (false, String::new(), "OMX_SLACK_WEBHOOK_URL not set".into()),
    };

    let mention = std::env::var("OMX_SLACK_MENTION").unwrap_or_default();
    let template =
        std::env::var("OMX_SLACK_TEMPLATE").unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string());

    let ctx = TemplateContext::from_event(event);
    let description = render(&template, &ctx);

    let blocks = build_blocks(event, &description);

    let fallback = if mention.is_empty() {
        format!("{} {}", slack_emoji(&event.event), event.event)
    } else {
        format!("{mention} {} {}", slack_emoji(&event.event), event.event)
    };

    let payload = serde_json::json!({
        "text": fallback,
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
        let (success, _, stderr) = send_slack_notification(&test_event()).await;
        assert!(!success);
        assert!(stderr.contains("request failed"));
    }

    #[test]
    fn slack_emoji_failed_is_red() {
        assert_eq!(slack_emoji(&HookEventName::Failed), ":red_circle:");
        assert_eq!(slack_emoji(&HookEventName::TestFailed), ":red_circle:");
    }

    #[test]
    fn slack_emoji_finished_is_check() {
        assert_eq!(slack_emoji(&HookEventName::Finished), ":white_check_mark:");
        assert_eq!(
            slack_emoji(&HookEventName::TestFinished),
            ":white_check_mark:"
        );
    }

    #[test]
    fn build_blocks_has_header_and_context() {
        let event = test_event();
        let blocks = build_blocks(&event, "some description");
        let arr = blocks.as_array().unwrap();
        assert!(arr.len() >= 3);
        assert_eq!(arr.first().unwrap()["type"], "header");
        assert_eq!(arr.last().unwrap()["type"], "context");
    }

    #[test]
    fn build_blocks_includes_session_field() {
        let event = test_event();
        let blocks = build_blocks(&event, "desc");
        let serialized = serde_json::to_string(&blocks).unwrap();
        assert!(serialized.contains("sess-456"));
    }
}
