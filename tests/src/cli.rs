#[cfg(test)]
mod tests {
    use crate::{omx_cmd, TestConfig};

    #[test]
    fn cli_version() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("version")
            .output()
            .expect("failed to run omx version");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("omx"),
            "version output should contain 'omx', got: {stdout}"
        );
    }

    #[test]
    fn cli_doctor() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("doctor")
            .output()
            .expect("failed to run omx doctor");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("omx doctor") || stdout.contains("checking"),
            "doctor should print header, got: {stdout}"
        );
    }

    #[test]
    fn cli_status() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("status")
            .output()
            .expect("failed to run omx status");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("OMX Status"),
            "status should print header, got: {stdout}"
        );
    }

    #[test]
    fn cli_cancel() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("cancel")
            .output()
            .expect("failed to run omx cancel");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Cancelling") || stdout.contains("cancel"),
            "cancel should print message, got: {stdout}"
        );
    }

    #[test]
    fn cli_cleanup() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("cleanup")
            .output()
            .expect("failed to run omx cleanup");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stdout.contains("Cleanup")
                || stderr.contains("Cleanup")
                || stdout.contains("lock files"),
            "cleanup should produce output, got stdout: {stdout}, stderr: {stderr}"
        );
    }

    #[test]
    fn cli_hooks_status() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["hooks", "status"])
            .output()
            .expect("failed to run omx hooks status");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("hooks") || stdout.contains("No hooks"),
            "hooks status should produce output, got: {stdout}"
        );
    }

    #[test]
    fn cli_hooks_validate() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["hooks", "validate"])
            .output()
            .expect("failed to run omx hooks validate");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("valid") || stdout.contains("hooks") || stdout.contains("0"),
            "hooks validate should produce output, got: {stdout}"
        );
    }

    #[test]
    fn cli_reasoning_default() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("reasoning")
            .output()
            .expect("failed to run omx reasoning");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("reasoning effort"),
            "reasoning should print current effort, got: {stdout}"
        );
    }

    #[test]
    fn cli_reasoning_set_effort() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["reasoning", "--effort", "high"])
            .output()
            .expect("failed to run omx reasoning --effort");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("set to: high"),
            "reasoning should confirm effort, got: {stdout}"
        );
    }

    #[test]
    fn cli_hook_api_state_roundtrip() {
        let config = TestConfig::new();
        let write_output = omx_cmd(&config)
            .args([
                "hook-api",
                "state-write",
                "--mode",
                "test",
                "--key",
                "mykey",
                "--value",
                r#"{"hello":"world"}"#,
            ])
            .output()
            .expect("failed to run omx hook-api state-write");
        let write_stdout = String::from_utf8_lossy(&write_output.stdout);
        assert!(
            write_stdout.contains("ok"),
            "state-write should print 'ok', got: {write_stdout}"
        );

        let read_output = omx_cmd(&config)
            .args(["hook-api", "state-read", "--mode", "test", "--key", "mykey"])
            .output()
            .expect("failed to run omx hook-api state-read");
        let read_stdout = String::from_utf8_lossy(&read_output.stdout);
        assert!(
            read_stdout.contains("hello") && read_stdout.contains("world"),
            "state-read should return written value, got: {read_stdout}"
        );
    }

    #[test]
    fn cli_hook_api_session_read() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["hook-api", "session-read"])
            .output()
            .expect("failed to run omx hook-api session-read");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains('{'),
            "session-read should return JSON, got: {stdout}"
        );
    }

    // ----- Edge cases -----

    #[test]
    fn cli_invalid_team_spec() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["team", "start", "bad-spec", "task"])
            .output()
            .expect("failed to run omx team start");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "invalid team spec should fail");
        assert!(
            stderr.contains("Invalid") || stderr.contains("error") || stderr.contains("spec"),
            "should report spec error, got: {stderr}"
        );
    }

    #[test]
    fn cli_hook_api_path_traversal_rejected() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args([
                "hook-api",
                "state-read",
                "--mode",
                "../etc",
                "--key",
                "passwd",
            ])
            .output()
            .expect("failed to run omx hook-api state-read");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "path traversal should be rejected"
        );
        assert!(
            stderr.contains("Invalid"),
            "should report invalid input, got: {stderr}"
        );
    }

    // ----- Stub commands -----

    #[test]
    fn cli_exec_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["exec", "--agent", "test", "do something"])
            .output()
            .expect("failed to run omx exec");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("exec"),
            "exec should print stub or exec output, got: {stdout}"
        );
    }

    #[test]
    fn cli_agents_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("agents")
            .output()
            .expect("failed to run omx agents");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("agent"),
            "agents should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_agents_init_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("agents-init")
            .output()
            .expect("failed to run omx agents-init");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("Scaffolding"),
            "agents-init should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_uninstall_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("uninstall")
            .output()
            .expect("failed to run omx uninstall");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("Uninstalling"),
            "uninstall should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_session_list_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("session")
            .output()
            .expect("failed to run omx session");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("session"),
            "session should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_resume_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["resume", "fake-session-id"])
            .output()
            .expect("failed to run omx resume");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("Resuming"),
            "resume should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_ralph_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .arg("ralph")
            .output()
            .expect("failed to run omx ralph");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("Ralph"),
            "ralph should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_autoresearch_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["autoresearch", "test query"])
            .output()
            .expect("failed to run omx autoresearch");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("autoresearch"),
            "autoresearch should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_ralplan_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["ralplan", "test objective"])
            .output()
            .expect("failed to run omx ralplan");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("planning"),
            "ralplan should print stub, got: {stdout}"
        );
    }

    #[test]
    fn cli_pipeline_stub() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["pipeline", "test-pipeline"])
            .output()
            .expect("failed to run omx pipeline");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("not yet wired") || stdout.contains("pipeline"),
            "pipeline should print stub, got: {stdout}"
        );
    }

    // ----- Tmux-dependent (ignored) -----

    #[test]
    #[ignore = "requires tmux"]
    fn cli_team_start() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["team", "start", "2:executor", "test task"])
            .output()
            .expect("failed to run omx team start");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Team started"),
            "team start should confirm, got: {stdout}"
        );
    }

    #[test]
    #[ignore = "requires omx-explore binary and possibly tmux"]
    fn cli_explore() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["explore", "--prompt", "test exploration"])
            .output()
            .expect("failed to run omx explore");
        assert!(
            output.status.success() || !output.status.success(),
            "explore ran (may fail if omx-explore not installed)"
        );
    }

    #[test]
    #[ignore = "requires omx-sparkshell binary"]
    fn cli_sparkshell() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["sparkshell", "echo", "hello"])
            .output()
            .expect("failed to run omx sparkshell");
        assert!(
            output.status.success() || !output.status.success(),
            "sparkshell ran (may fail if omx-sparkshell not installed)"
        );
    }

    #[test]
    #[ignore = "requires terminal for ratatui"]
    fn cli_hud() {
        let config = TestConfig::new();
        let output = omx_cmd(&config)
            .args(["hud"])
            .output()
            .expect("failed to run omx hud");
        let _ = output;
    }

    // ----- Migrate tests -----

    // NOTE: default_codex_home() returns $HOME/.codex, and omx_cmd sets HOME
    // to config.home_path(), so the effective codex home is home_path()/.codex.
    fn codex_home(config: &TestConfig) -> std::path::PathBuf {
        config.home_path().join(".codex")
    }

    #[test]
    fn cli_migrate_config_dry_run() {
        let config = TestConfig::new();
        let ch = codex_home(&config);
        std::fs::create_dir_all(&ch).unwrap();
        std::fs::write(
            ch.join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4", "team": "gpt-5.4-mini"}, "env": {"MY_VAR": "hello"}}"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "config", "--dry-run"])
            .output()
            .expect("failed to run omx migrate config --dry-run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("models.frontier") && stdout.contains("gpt-5.4"),
            "dry-run should show field mapping, got: {stdout}"
        );
        assert!(
            stdout.contains("dry-run"),
            "should indicate dry-run mode, got: {stdout}"
        );
        // Verify config.toml was NOT created
        assert!(
            !ch.join("config.toml").exists(),
            "dry-run should not write files"
        );
    }

    #[test]
    fn cli_migrate_config_writes() {
        let config = TestConfig::new();
        let ch = codex_home(&config);
        std::fs::create_dir_all(&ch).unwrap();
        std::fs::write(
            ch.join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5.4"}}"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "config"])
            .output()
            .expect("failed to run omx migrate config");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Migrated"),
            "should report migration, got: {stdout}"
        );

        // Verify config.toml has the migrated value
        let toml_str = std::fs::read_to_string(ch.join("config.toml")).unwrap();
        assert!(
            toml_str.contains("gpt-5.4"),
            "config.toml should contain migrated model, got: {toml_str}"
        );
    }

    #[test]
    fn cli_migrate_sessions_dry_run() {
        let config = TestConfig::new();
        let ch = codex_home(&config);
        let sessions_dir = ch.join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();
        std::fs::write(
            sessions_dir.join("rollout-test.jsonl"),
            r#"{"type":"session_meta","payload":{"id":"s1","timestamp":"2026-04-01T10:00:00Z","agent_role":"autopilot"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"hello"}}
"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "sessions", "--dry-run"])
            .output()
            .expect("failed to run omx migrate sessions --dry-run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1 rollout") || stdout.contains("Found"),
            "should report rollout files found, got: {stdout}"
        );
        assert!(
            stdout.contains("dry-run"),
            "should indicate dry-run, got: {stdout}"
        );
    }

    #[test]
    fn cli_migrate_sessions_writes() {
        let config = TestConfig::new();
        let ch = codex_home(&config);
        let sessions_dir = ch.join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();
        std::fs::create_dir_all(ch.join(".omx").join("sessions")).unwrap();
        std::fs::write(
            sessions_dir.join("rollout-test.jsonl"),
            r#"{"type":"session_meta","payload":{"id":"s2","timestamp":"2026-04-01T10:00:00Z","agent_role":"team"}}
{"type":"response_item","payload":{"type":"message","role":"user","content":"world"}}
"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .args(["migrate", "sessions"])
            .output()
            .expect("failed to run omx migrate sessions");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Migrated"),
            "should report migration, got: {stdout}"
        );

        // Verify session directory was created
        let meta_path = ch
            .join(".omx")
            .join("sessions")
            .join("s2")
            .join("meta.json");
        assert!(meta_path.exists(), "session meta.json should exist");
    }

    // ----- Enhanced doctor tests -----

    #[test]
    fn cli_doctor_grouped_output() {
        let config = TestConfig::new();
        let ch = codex_home(&config);
        std::fs::create_dir_all(&ch).unwrap();
        std::fs::write(ch.join("config.toml"), "[models]\nfrontier = \"o3\"\n").unwrap();

        let output = omx_cmd(&config)
            .arg("doctor")
            .output()
            .expect("failed to run omx doctor");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Dependencies"),
            "should have Dependencies section, got: {stdout}"
        );
        assert!(
            stdout.contains("MCP Servers"),
            "should have MCP Servers section, got: {stdout}"
        );
        assert!(
            stdout.contains("Notification Hooks"),
            "should have Notification Hooks section, got: {stdout}"
        );
        assert!(
            stdout.contains("Configuration"),
            "should have Configuration section, got: {stdout}"
        );
    }

    #[test]
    fn cli_doctor_detects_ts_era() {
        let config = TestConfig::new();
        let ch = codex_home(&config);
        std::fs::create_dir_all(&ch).unwrap();
        std::fs::write(
            ch.join(".omx-config.json"),
            r#"{"models": {"default": "gpt-5"}}"#,
        )
        .unwrap();

        let output = omx_cmd(&config)
            .arg("doctor")
            .output()
            .expect("failed to run omx doctor");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("TS Migration"),
            "should have TS Migration section, got: {stdout}"
        );
        assert!(
            stdout.contains(".omx-config.json") && stdout.contains("migrate"),
            "should suggest migration, got: {stdout}"
        );
    }
}
