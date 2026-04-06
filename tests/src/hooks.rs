#[cfg(test)]
mod tests {
    use omx_config::{DiscordConfig, NotificationConfig, SlackConfig};
    use omx_hooks::router::route;
    use omx_hooks::{HookDescriptor, HookDispatcher, ShellHookDispatcher};
    use omx_types::{HookEvent, HookEventName, HookSource};

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: HookEventName::SessionStart,
            timestamp: "2026-04-05T10:00:00Z".into(),
            source: HookSource {
                component: "integration-test".into(),
                worker_id: None,
            },
            context: serde_json::json!({}),
            session_id: Some("test-hook-sess".into()),
        }
    }

    #[test]
    fn hook_discovery_finds_executable_scripts() {
        let tmp = tempfile::tempdir().unwrap();

        let script_path = tmp.path().join("test-hook.sh");
        std::fs::write(&script_path, "#!/bin/sh\necho '{}'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        std::fs::write(tmp.path().join("readme.txt"), "not a hook").unwrap();

        let dispatcher = ShellHookDispatcher::new(5000);
        let hooks = dispatcher.discover(tmp.path()).unwrap();

        assert!(hooks.len() >= 2, "should find at least 2 files");
        let exec_hook = hooks.iter().find(|h| h.name == "test-hook").unwrap();
        #[cfg(unix)]
        assert!(exec_hook.executable, "test-hook.sh should be executable");
    }

    #[tokio::test]
    async fn hook_dispatch_executes_matching_hooks() {
        let tmp = tempfile::tempdir().unwrap();

        let script_path = tmp.path().join("echo-hook.sh");
        std::fs::write(
            &script_path,
            "#!/bin/sh\ncat > /dev/null\necho '{\"received\":true}'",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let hooks = vec![HookDescriptor {
            name: "echo-hook".into(),
            path: script_path,
            executable: true,
        }];

        let dispatcher = ShellHookDispatcher::new(5000).with_hooks(hooks);
        let results = dispatcher.dispatch(&test_event()).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].success, "hook should succeed");
        assert_eq!(results[0].hook, "echo-hook");
        assert!(
            results[0].stdout.contains("received"),
            "stdout should contain hook output"
        );
    }

    #[test]
    fn notification_router_routes_to_configured_platforms() {
        let mut config = NotificationConfig::default();
        config.discord = Some(DiscordConfig {
            webhook_url: "https://discord.com/webhook".into(),
            mention: None,
        });
        config.slack = Some(SlackConfig {
            webhook_url: "https://hooks.slack.com/services/T/B/X".into(),
            mention: None,
        });

        let decision = route(&test_event(), &config);
        let platforms = decision.enabled_platforms();

        assert_eq!(platforms.len(), 2);
        assert!(platforms.contains(&"discord"));
        assert!(platforms.contains(&"slack"));
        assert!(!platforms.contains(&"telegram"));
    }

    #[test]
    fn notification_router_empty_config_routes_nowhere() {
        let config = NotificationConfig::default();
        let decision = route(&test_event(), &config);
        assert!(
            decision.enabled_platforms().is_empty(),
            "empty config should route to no platforms"
        );
    }
}
