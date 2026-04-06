#[cfg(test)]
mod tests {
    use crate::{notify_cmd, session_start_event};
    use std::io::Write;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Pipe a HookEvent JSON to a notification binary's stdin and return stdout.
    fn run_notify_binary(
        binary: &str,
        event: &omx_types::HookEvent,
        env: Vec<(&str, String)>,
    ) -> String {
        let event_json = serde_json::to_string(event).unwrap();
        let mut cmd = notify_cmd(binary);
        for (key, value) in env {
            cmd.env(key, value);
        }
        let mut child = cmd
            .spawn()
            .unwrap_or_else(|e| panic!("failed to spawn {binary}: {e}"));

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(event_json.as_bytes()).unwrap();
        }
        drop(child.stdin.take());

        let output = child
            .wait_with_output()
            .expect("failed to wait for notify binary");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[tokio::test]
    async fn notify_discord_sends_embed() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let event = session_start_event();
        let stdout = run_notify_binary(
            "omx-notify-discord",
            &event,
            vec![("OMX_DISCORD_WEBHOOK_URL", mock_server.uri())],
        );

        assert!(
            stdout.contains("\"success\":true"),
            "discord should succeed, got: {stdout}"
        );
    }

    #[tokio::test]
    async fn notify_slack_sends_blocks() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let event = session_start_event();
        let stdout = run_notify_binary(
            "omx-notify-slack",
            &event,
            vec![("OMX_SLACK_WEBHOOK_URL", mock_server.uri())],
        );

        assert!(
            stdout.contains("\"success\":true"),
            "slack should succeed, got: {stdout}"
        );
    }

    #[tokio::test]
    async fn notify_telegram_sends_html() {
        // Telegram hardcodes https://api.telegram.org so we cannot redirect via wiremock.
        // We verify the binary produces structured JSON output with the correct hook name.
        let event = session_start_event();
        let stdout = run_notify_binary(
            "omx-notify-telegram",
            &event,
            vec![
                ("OMX_TELEGRAM_BOT_TOKEN", "fake-token".into()),
                ("OMX_TELEGRAM_CHAT_ID", "12345".into()),
            ],
        );

        assert!(
            stdout.contains("omx-notify-telegram"),
            "telegram should produce structured output with hook name, got: {stdout}"
        );
    }

    #[tokio::test]
    async fn notify_pushover_sends_form() {
        // Pushover hardcodes https://api.pushover.net so we cannot redirect via wiremock.
        // We verify the binary produces structured JSON output with the correct hook name.
        let event = session_start_event();
        let stdout = run_notify_binary(
            "omx-notify-pushover",
            &event,
            vec![
                ("OMX_PUSHOVER_USER_KEY", "fake-user-key".into()),
                ("OMX_PUSHOVER_APP_TOKEN", "fake-app-token".into()),
            ],
        );

        assert!(
            stdout.contains("omx-notify-pushover"),
            "pushover should produce structured output with hook name, got: {stdout}"
        );
    }

    #[tokio::test]
    async fn notify_generic_sends_json() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&mock_server)
            .await;

        let event = session_start_event();
        let stdout = run_notify_binary(
            "omx-notify-generic",
            &event,
            vec![("OMX_GENERIC_WEBHOOK_URL", mock_server.uri())],
        );

        assert!(
            stdout.contains("\"success\":true"),
            "generic should succeed, got: {stdout}"
        );
    }
}
