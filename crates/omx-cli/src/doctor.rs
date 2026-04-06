use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum CheckStatus {
    Ok,
    Missing,
    Invalid(String),
    Warning(String),
    Info(String),
}

#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
}

#[derive(Debug)]
pub struct CheckGroup {
    pub name: &'static str,
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
    });

    CheckGroup {
        name: "Dependencies",
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
        })
        .collect();

    CheckGroup {
        name: "MCP Servers",
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
        })
        .collect();

    CheckGroup {
        name: "Notification Hooks",
        checks,
    }
}

pub fn check_configuration(codex_home: &Path) -> CheckGroup {
    let config_path = codex_home.join("config.toml");
    let mut checks = Vec::new();

    if !config_path.exists() {
        checks.push(CheckResult {
            name: "config.toml exists".into(),
            status: CheckStatus::Missing,
        });
    } else {
        checks.push(CheckResult {
            name: "config.toml exists".into(),
            status: CheckStatus::Ok,
        });

        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        match toml::from_str::<toml::Value>(&content) {
            Ok(_) => checks.push(CheckResult {
                name: "config.toml schema valid".into(),
                status: CheckStatus::Ok,
            }),
            Err(e) => checks.push(CheckResult {
                name: "config.toml schema valid".into(),
                status: CheckStatus::Invalid(format!("TOML parse error: {e}")),
            }),
        }
    }

    CheckGroup {
        name: "Configuration",
        checks,
    }
}

pub fn check_hooks(codex_home: &Path) -> CheckGroup {
    let hooks_dir = codex_home.join(".omx").join("hooks");
    let mut checks = Vec::new();

    if !hooks_dir.exists() {
        checks.push(CheckResult {
            name: ".omx/hooks/".into(),
            status: CheckStatus::Info("directory not found — hooks are optional".into()),
        });
        return CheckGroup {
            name: "Hooks",
            checks,
        };
    }

    let entries: Vec<_> = std::fs::read_dir(&hooks_dir)
        .map(|rd| rd.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();

    let count = entries.len();
    checks.push(CheckResult {
        name: ".omx/hooks/".into(),
        status: CheckStatus::Info(format!("{count} file(s) found")),
    });

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in &entries {
            let path = entry.path();
            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            if let Ok(meta) = std::fs::metadata(&path) {
                let mode = meta.permissions().mode();
                let executable = mode & 0o111 != 0;
                checks.push(CheckResult {
                    name: format!("hooks/{fname}"),
                    status: if executable {
                        CheckStatus::Ok
                    } else {
                        CheckStatus::Warning("not executable — run: chmod +x".into())
                    },
                });
            }
        }
    }

    CheckGroup {
        name: "Hooks",
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
            status: CheckStatus::Warning(
                "TypeScript-era config detected — migrate to config.toml".into(),
            ),
        });
    } else {
        checks.push(CheckResult {
            name: ".omx-config.json".into(),
            status: CheckStatus::Ok,
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
        });
    } else {
        checks.push(CheckResult {
            name: "sessions/rollout-*.jsonl".into(),
            status: CheckStatus::Warning(format!(
                "{} legacy rollout file(s) found — consider migrating",
                rollout_files.len()
            )),
        });
    }

    CheckGroup {
        name: "TS Migration",
        checks,
    }
}

fn status_label(status: &CheckStatus) -> &'static str {
    match status {
        CheckStatus::Ok => "ok     ",
        CheckStatus::Missing => "MISSING",
        CheckStatus::Invalid(_) => "INVALID",
        CheckStatus::Warning(_) => "WARNING",
        CheckStatus::Info(_) => "info   ",
    }
}

fn status_message(status: &CheckStatus) -> Option<&str> {
    match status {
        CheckStatus::Ok | CheckStatus::Missing => None,
        CheckStatus::Invalid(msg) | CheckStatus::Warning(msg) | CheckStatus::Info(msg) => {
            Some(msg.as_str())
        }
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
        println!("  {}", group.name);
        for check in &group.checks {
            let label = status_label(&check.status);
            match status_message(&check.status) {
                Some(msg) => println!("    {} {} — {}", label, check.name, msg),
                None => println!("    {} {}", label, check.name),
            }
            if matches!(check.status, CheckStatus::Missing | CheckStatus::Invalid(_)) {
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
        assert_eq!(groups[0].name, "Dependencies");
        assert_eq!(groups[1].name, "MCP Servers");
        assert_eq!(groups[2].name, "Notification Hooks");
        assert_eq!(groups[3].name, "Configuration");
        assert_eq!(groups[4].name, "Hooks");
        assert_eq!(groups[5].name, "TS Migration");

        // Dependencies group has exactly 3 checks
        assert_eq!(groups[0].checks.len(), 3);
        // MCP Servers group has exactly 5 checks
        assert_eq!(groups[1].checks.len(), 5);
        // Notification Hooks group has exactly 5 checks
        assert_eq!(groups[2].checks.len(), 5);
    }

    #[test]
    fn doctor_detects_ts_era_config() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".omx-config.json"), r#"{"key":"value"}"#).unwrap();

        let group = check_ts_migration(dir.path());
        let ts_check = group.checks.iter().find(|c| c.name == ".omx-config.json");
        assert!(ts_check.is_some());
        match &ts_check.unwrap().status {
            CheckStatus::Warning(msg) => {
                assert!(msg.contains("TypeScript-era config detected"));
            }
            other => panic!("expected Warning, got {:?}", other),
        }
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
        match &rollout_check.unwrap().status {
            CheckStatus::Warning(msg) => {
                assert!(msg.contains("2"), "expected count 2 in message: {msg}");
            }
            other => panic!("expected Warning, got {:?}", other),
        }
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
        // Should have 2 checks: exists + schema valid
        assert_eq!(group.checks.len(), 2);

        let exists_check = &group.checks[0];
        assert_eq!(exists_check.name, "config.toml exists");
        assert_eq!(exists_check.status, CheckStatus::Ok);

        let schema_check = &group.checks[1];
        assert_eq!(schema_check.name, "config.toml schema valid");
        match &schema_check.status {
            CheckStatus::Invalid(msg) => {
                assert!(msg.contains("TOML parse error"));
            }
            other => panic!("expected Invalid, got {:?}", other),
        }
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
        // Should have 2 checks: exists + schema valid
        assert_eq!(group.checks.len(), 2);

        let exists_check = &group.checks[0];
        assert_eq!(exists_check.name, "config.toml exists");
        assert_eq!(exists_check.status, CheckStatus::Ok);

        let schema_check = &group.checks[1];
        assert_eq!(schema_check.name, "config.toml schema valid");
        assert_eq!(schema_check.status, CheckStatus::Ok);
    }
}
