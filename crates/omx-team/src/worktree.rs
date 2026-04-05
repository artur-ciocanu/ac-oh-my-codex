use omx_types::OmxError;
use std::path::PathBuf;

/// Run a git command. Returns stdout on success.
fn run_git(args: &[&str]) -> Result<String, OmxError> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .map_err(|e| OmxError::Team(format!("failed to run git: {e}")))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| OmxError::Team(format!("invalid utf-8 from git: {e}")))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(OmxError::Team(format!(
            "git {} failed: {}",
            args.first().unwrap_or(&""),
            stderr.trim()
        )))
    }
}

/// Provision a git worktree for a worker.
/// Creates a new branch `omx-team/{team_name}/{worker_name}` and a worktree at
/// `../{repo}-omx-{team_name}-{worker_name}` relative to the repo root.
pub fn provision_worktree(team_name: &str, worker_name: &str) -> Result<PathBuf, OmxError> {
    let branch_name = format!("omx-team/{team_name}/{worker_name}");

    let root = run_git(&["rev-parse", "--show-toplevel"])?;
    let root = root.trim();

    let repo_dir = PathBuf::from(root);
    let repo_name = repo_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");
    let worktree_dir = repo_dir
        .parent()
        .unwrap_or(&repo_dir)
        .join(format!("{repo_name}-omx-{team_name}-{worker_name}"));

    let worktree_str = worktree_dir.to_string_lossy().to_string();

    run_git(&["worktree", "add", "-b", &branch_name, &worktree_str])?;

    tracing::info!(
        worktree = %worktree_str,
        branch = %branch_name,
        "provisioned worktree"
    );

    Ok(worktree_dir)
}

/// Remove a git worktree and delete its branch.
pub fn cleanup_worktree(worktree_path: &str, branch_name: &str) -> Result<(), OmxError> {
    run_git(&["worktree", "remove", "--force", worktree_path])?;

    if let Err(e) = run_git(&["branch", "-D", branch_name]) {
        tracing::warn!(branch = %branch_name, error = %e, "failed to delete branch");
    }

    tracing::info!(worktree = %worktree_path, "cleaned up worktree");
    Ok(())
}

/// List all worktrees for a team.
pub fn list_team_worktrees(team_name: &str) -> Result<Vec<String>, OmxError> {
    let output = run_git(&["worktree", "list", "--porcelain"])?;
    let prefix = format!("omx-{team_name}-");

    let worktrees = output
        .lines()
        .filter_map(|line| {
            if line.starts_with("worktree ") {
                let path = line.strip_prefix("worktree ")?;
                if path.contains(&prefix) {
                    return Some(path.to_string());
                }
            }
            None
        })
        .collect();

    Ok(worktrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_path_is_sibling_of_repo() {
        let repo = PathBuf::from("/home/user/my-project");
        let repo_name = repo.file_name().unwrap().to_str().unwrap();
        let worktree = repo
            .parent()
            .unwrap()
            .join(format!("{repo_name}-omx-team1-worker1"));
        assert_eq!(
            worktree,
            PathBuf::from("/home/user/my-project-omx-team1-worker1")
        );
    }

    #[test]
    fn branch_name_format() {
        let branch = format!("omx-team/{}/{}", "myteam", "worker-0");
        assert_eq!(branch, "omx-team/myteam/worker-0");
    }

    #[test]
    fn list_team_worktrees_parses_porcelain() {
        let output = "worktree /home/user/project\nHEAD abc123\nbranch refs/heads/main\n\nworktree /home/user/project-omx-team1-w0\nHEAD def456\nbranch refs/heads/omx-team/team1/w0\n\n";
        let prefix = "omx-team1-";
        let paths: Vec<&str> = output
            .lines()
            .filter_map(|line| {
                if line.starts_with("worktree ") {
                    let path = line.strip_prefix("worktree ")?;
                    if path.contains(prefix) {
                        return Some(path);
                    }
                }
                None
            })
            .collect();
        assert_eq!(paths, vec!["/home/user/project-omx-team1-w0"]);
    }
}
