use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum CheckStatus {
    Ok,
    Missing,
    Invalid,
    Warning,
    Info,
}

#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct CheckGroup {
    pub title: String,
    pub checks: Vec<CheckResult>,
}

fn which_ok(bin: &str) -> bool {
    std::process::Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn check_dependencies() -> CheckGroup {
    let mut checks = Vec::new();

    let tmux_ok = std::process::Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    checks.push(CheckResult {
        name: "tmux".into(),
        status: if tmux_ok {
            CheckStatus::Ok
        } else {
            CheckStatus::Missing
        },
        message: if tmux_ok {
            None
        } else {
            Some("install with: brew install tmux".into())
        },
    });

    let codex_ok = std::process::Command::new("codex")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    checks.push(CheckResult {
        name: "codex".into(),
        status: if codex_ok {
            CheckStatus::Ok
        } else {
            CheckStatus::Missing
        },
        message: None,
    });

    let claude_ok = std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    checks.push(CheckResult {
        name: "claude".into(),
        status: if claude_ok {
            CheckStatus::Ok
        } else {
            CheckStatus::Missing
        },
        message: None,
    });

    CheckGroup {
        title: "Dependencies".into(),
        checks,
    }
}

pub fn check_mcp_servers() -> CheckGroup {
    let bins = [
        "omx-mcp-state",
        "omx-mcp-memory",
        "omx-mcp-code-intel",
        "omx-mcp-trace",
        "omx-mcp-team",
    ];

    let checks = bins
        .iter()
        .map(|bin| CheckResult {
            name: (*bin).into(),
            status: if which_ok(bin) {
                CheckStatus::Ok
            } else {
                CheckStatus::Missing
            },
            message: None,
        })
        .collect();

    CheckGroup {
        title: "MCP Servers".into(),
        checks,
    }
}

pub fn check_notification_binaries() -> CheckGroup {
    let bins = [
        "omx-notify-discord",
        "omx-notify-slack",
        "omx-notify-telegram",
        "omx-notify-pushover",
        "omx-notify-generic",
    ];

    let checks = bins
        .iter()
        .map(|bin| CheckResult {
            name: (*bin).into(),
            status: if which_ok(bin) {
                CheckStatus::Ok
            } else {
                CheckStatus::Missing
            },
            message: None,
        })
        .collect();

    CheckGroup {
        title: "Notification Binaries".into(),
        checks,
    }
}

pub fn check_configuration(codex_home: &Path) -> CheckGroup {
    let config_path = codex_home.join("config.toml");
    let mut checks = Vec::new();

    if !config_path.exists() {
        checks.push(CheckResult {
            name: "config.toml".into(),
            status: CheckStatus::Missing,
            message: Some(format!(
                "expected at {}",
                config_path.display()
            )),
        });
    } else {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        match toml::from_str::<toml::Value>(&content) {
            Ok(_) => checks.push(CheckResult {
                name: "config.toml".into(),
                status: CheckStatus::Ok,
                message: None,
            }),
            Err(e) => checks.push(CheckResult {
                name: "config.toml".into(),
                status: CheckStatus::Invalid,
                message: Some(format!("TOML parse error: {e}")),
            }),
        }
    }

    CheckGroup {
        title: "Configuration".into(),
        checks,
    }
}

pub fn check_hooks(codex_home: &Path) -> CheckGroup {
    let hooks_dir = codex_home.join(".omx").join("hooks");
    let mut checks = Vec::new();

    if !hooks_dir.exists() {
        checks.push(CheckResult {
            name: ".omx/hooks/".into(),
            status: CheckStatus::Info,
            message: Some("directory not found — hooks are optional".into()),
        });
        return CheckGroup {
            title: "Hooks".into(),
            checks,
        };
    }

    let entries: Vec<_> = std::fs::read_dir(&hooks_dir)
        .map(|rd| rd.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();

    let count = entries.len();
    checks.push(CheckResult {
        name: ".omx/hooks/".into(),
        status: CheckStatus::Ok,
        message: Some(format!("{count} file(s) found")),
    });

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in &entries {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            if let Ok(meta) = std::fs::metadata(&path) {
                let mode = meta.permissions().mode();
                let executable = mode & 0o111 != 0;
                checks.push(CheckResult {
                    name: format!("hooks/{name}"),
                    status: if executable {
                        CheckStatus::Ok
                    } else {
                        CheckStatus::Warning
                    },
                    message: if executable {
                        None
                    } else {
                        Some("not executable — run: chmod +x".into())
                    },
                });
            }
        }
    }

    CheckGroup {
        title: "Hooks".into(),
        checks,
    }
}

pub fn check_ts_migration(codex_home: &Path) -> CheckGroup {
    let mut checks = Vec::new();

    // Check for legacy TypeScript-era config file
    let ts_config = codex_home.join(".omx-config.json");
    if ts_config.exists() {
        checks.push(CheckResult {
            name: ".omx-config.json".into(),
            status: CheckStatus::Warning,
            message: Some(
                "TypeScript-era config detected — migrate to config.toml".into(),
            ),
        });
    } else {
        checks.push(CheckResult {
            name: ".omx-config.json".into(),
            status: CheckStatus::Ok,
            message: Some("not present (good)".into()),
        });
    }

    // Check for rollout JSONL files in sessions/
    let sessions_dir = codex_home.join("sessions");
    let rollout_files: Vec<_> = if sessions_dir.exists() {
        std::fs::read_dir(&sessions_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        vec![]
    };

    if rollout_files.is_empty() {
        checks.push(CheckResult {
            name: "sessions/rollout-*.jsonl".into(),
            status: CheckStatus::Ok,
            message: Some("no legacy rollout files found (good)".into()),
        });
    } else {
        checks.push(CheckResult {
            name: "sessions/rollout-*.jsonl".into(),
            status: CheckStatus::Warning,
            message: Some(format!(
                "{} legacy rollout file(s) found — consider migrating",
                rollout_files.len()
            )),
        });
    }

    CheckGroup {
        title: "TypeScript Migration".into(),
        checks,
    }
}

fn status_label(status: &CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "ok     ",
        CheckStatus::Missing => "MISSING",
        CheckStatus::Invalid => "INVALID",
        CheckStatus::Warning => "WARNING",
        CheckStatus::Info => "info   ",
    }
}

pub fn run_doctor(codex_home: &Path) -> bool {
    println!("omx doctor — checking installation\n");

    let groups = vec![
        check_dependencies(),
        check_mcp_servers(),
        check_notification_binaries(),
        check_configuration(codex_home),
        check_hooks(codex_home),
        check_ts_migration(codex_home),
    ];

    let mut all_ok = true;

    for group in &groups {
        println!("  {}", group.title);
        for check in &group.checks {
            let label = status_label(&check.status);
            match &check.message {
                Some(msg) => println!("    {} {} — {}", label, check.name, msg),
                None => println!("    {} {}", label, check.name),
            }
            if check.status == CheckStatus::Missing || check.status == CheckStatus::Invalid {
                all_ok = false;
            }
        }
        println!();
    }

    if all_ok {
        println!("  All checks passed.");
    } else {
        println!("  Some checks failed. Address the issues above.");
    }

    all_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn doctor_groups_checks_correctly() {
        let dir = TempDir::new().unwrap();
        let groups = vec![
            check_dependencies(),
            check_mcp_servers(),
            check_notification_binaries(),
            check_configuration(dir.path()),
            check_hooks(dir.path()),
            check_ts_migration(dir.path()),
        ];
        assert_eq!(groups.len(), 6);
        assert_eq!(groups[0].title, "Dependencies");
        assert_eq!(groups[1].title, "MCP Servers");
        assert_eq!(groups[2].title, "Notification Binaries");
        assert_eq!(groups[3].title, "Configuration");
        assert_eq!(groups[4].title, "Hooks");
        assert_eq!(groups[5].title, "TypeScript Migration");

        // Dependencies group has exactly 3 checks
        assert_eq!(groups[0].checks.len(), 3);
        // MCP Servers group has exactly 5 checks
        assert_eq!(groups[1].checks.len(), 5);
        // Notification Binaries group has exactly 5 checks
        assert_eq!(groups[2].checks.len(), 5);
    }

    #[test]
    fn doctor_detects_ts_era_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".omx-config.json"), r#"{"key":"value"}"#).unwrap();

        let group = check_ts_migration(dir.path());
        let ts_check = group.checks.iter().find(|c| c.name == ".omx-config.json");
        assert!(ts_check.is_some());
        assert_eq!(ts_check.unwrap().status, CheckStatus::Warning);
    }

    #[test]
    fn doctor_detects_rollout_files() {
        let dir = TempDir::new().unwrap();
        let sessions = dir.path().join("sessions");
        std::fs::create_dir_all(&sessions).unwrap();
        std::fs::write(sessions.join("rollout-2024-01-01.jsonl"), "").unwrap();
        std::fs::write(sessions.join("rollout-2024-01-02.jsonl"), "").unwrap();

        let group = check_ts_migration(dir.path());
        let rollout_check = group
            .checks
            .iter()
            .find(|c| c.name == "sessions/rollout-*.jsonl");
        assert!(rollout_check.is_some());
        assert_eq!(rollout_check.unwrap().status, CheckStatus::Warning);
        let msg = rollout_check.unwrap().message.as_deref().unwrap_or("");
        assert!(msg.contains("2"), "expected count 2 in message: {msg}");
    }

    #[test]
    fn doctor_no_ts_era_files() {
        let dir = TempDir::new().unwrap();

        let group = check_ts_migration(dir.path());
        let ts_check = group.checks.iter().find(|c| c.name == ".omx-config.json");
        assert!(ts_check.is_some());
        assert_eq!(ts_check.unwrap().status, CheckStatus::Ok);

        let rollout_check = group
            .checks
            .iter()
            .find(|c| c.name == "sessions/rollout-*.jsonl");
        assert!(rollout_check.is_some());
        assert_eq!(rollout_check.unwrap().status, CheckStatus::Ok);
    }

    #[test]
    fn doctor_validates_config_schema() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "this is not valid toml = = =",
        )
        .unwrap();

        let group = check_configuration(dir.path());
        let check = group.checks.iter().find(|c| c.name == "config.toml");
        assert!(check.is_some());
        assert_eq!(check.unwrap().status, CheckStatus::Invalid);
    }

    #[test]
    fn doctor_valid_config_passes_schema() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            "[models]\nfrontier = \"gpt-4o\"\n",
        )
        .unwrap();

        let group = check_configuration(dir.path());
        let check = group.checks.iter().find(|c| c.name == "config.toml");
        assert!(check.is_some());
        assert_eq!(check.unwrap().status, CheckStatus::Ok);
    }
}
