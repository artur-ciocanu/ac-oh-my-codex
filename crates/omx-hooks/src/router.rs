use omx_config::NotificationConfig;
use omx_types::HookEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub discord: bool,
    pub slack: bool,
    pub telegram: bool,
    pub pushover: bool,
    pub generic: bool,
}

impl RoutingDecision {
    /// Returns the names of platforms that are enabled in this decision.
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

    /// Returns the binary names corresponding to enabled platforms.
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

/// Determine which notification platforms should receive the given event
/// based on whether their config section is present.
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

    fn test_event() -> HookEvent {
        HookEvent {
            schema_version: "1".into(),
            event: omx_types::HookEventName::SessionStart,
            timestamp: "2026-04-05T00:00:00Z".into(),
            source: omx_types::HookSource {
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
            webhook_url: "https://hooks.slack.com/services/T/B/X".into(),
            mention: None,
        });
        config.pushover = Some(PushoverConfig {
            user_key: "ukey".into(),
            app_token: "atoken".into(),
            device: None,
        });
        let decision = route(&test_event(), &config);
        assert_eq!(
            decision.enabled_platforms(),
            vec!["discord", "slack", "pushover"]
        );
        assert_eq!(
            decision.binary_names(),
            vec![
                "omx-notify-discord",
                "omx-notify-slack",
                "omx-notify-pushover"
            ]
        );
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
        assert_eq!(
            names,
            vec![
                "omx-notify-discord",
                "omx-notify-slack",
                "omx-notify-telegram",
                "omx-notify-pushover",
                "omx-notify-generic",
            ]
        );
        // Also verify enabled_platforms matches
        let platforms = decision.enabled_platforms();
        assert_eq!(platforms.len(), 5);
        assert_eq!(
            platforms,
            vec!["discord", "slack", "telegram", "pushover", "generic"]
        );
    }
}
