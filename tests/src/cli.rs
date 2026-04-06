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
}
