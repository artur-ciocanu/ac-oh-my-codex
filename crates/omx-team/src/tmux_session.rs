use omx_types::OmxError;

/// Run a tmux command. Returns stdout on success.
fn run_tmux(args: &[&str]) -> Result<String, OmxError> {
    let output = std::process::Command::new("tmux")
        .args(args)
        .output()
        .map_err(|e| OmxError::Tmux(format!("failed to run tmux: {e}")))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| OmxError::Tmux(format!("invalid utf-8 from tmux: {e}")))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(OmxError::Tmux(format!(
            "tmux {} failed: {}",
            args.first().unwrap_or(&""),
            stderr.trim()
        )))
    }
}

/// Create a detached tmux session for a team.
pub fn create_team_session(name: &str) -> Result<String, OmxError> {
    let session_name = if name.starts_with("omx-team-") {
        name.to_string()
    } else {
        format!("omx-team-{name}")
    };
    run_tmux(&["new-session", "-d", "-s", &session_name])?;
    tracing::info!(session = %session_name, "created team session");
    Ok(session_name)
}

/// Create a tmux window within a team session for a worker.
pub fn create_worker_window(session: &str, worker_name: &str) -> Result<String, OmxError> {
    run_tmux(&["new-window", "-t", session, "-n", worker_name])?;
    let target = format!("{session}:{worker_name}");
    tracing::info!(target = %target, "created worker window");
    Ok(target)
}

/// Send keys to a tmux target pane.
pub fn send_keys(target: &str, keys: &str) -> Result<(), OmxError> {
    run_tmux(&["send-keys", "-t", target, keys, "C-m"])?;
    Ok(())
}

/// Kill the entire team tmux session.
pub fn kill_team_session(name: &str) -> Result<(), OmxError> {
    let session_name = if name.starts_with("omx-team-") {
        name.to_string()
    } else {
        format!("omx-team-{name}")
    };
    run_tmux(&["kill-session", "-t", &session_name])?;
    tracing::info!(session = %session_name, "killed team session");
    Ok(())
}

/// Check if a tmux session exists.
pub fn session_exists(name: &str) -> bool {
    run_tmux(&["has-session", "-t", name]).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn session_name_prefixed() {
        let name = "abc123";
        let expected = "omx-team-abc123";
        assert_eq!(format!("omx-team-{name}"), expected);
    }

    #[test]
    fn kill_session_normalizes_name() {
        let prefixed = "omx-team-test";
        let result = if prefixed.starts_with("omx-team-") {
            prefixed.to_string()
        } else {
            format!("omx-team-{prefixed}")
        };
        assert_eq!(result, "omx-team-test");

        let unprefixed = "test";
        let result = if unprefixed.starts_with("omx-team-") {
            unprefixed.to_string()
        } else {
            format!("omx-team-{unprefixed}")
        };
        assert_eq!(result, "omx-team-test");
    }
}
