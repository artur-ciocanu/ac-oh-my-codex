use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitContext {
    pub branch: String,
    pub dirty: bool,
    pub ahead: u32,
    pub behind: u32,
}

impl GitContext {
    pub fn status_line(&self) -> String {
        let dirty_marker = if self.dirty { "*" } else { "" };
        let sync = match (self.ahead, self.behind) {
            (0, 0) => String::new(),
            (a, 0) => format!(" +{a}"),
            (0, b) => format!(" -{b}"),
            (a, b) => format!(" +{a}/-{b}"),
        };
        format!("{}{}{}", self.branch, dirty_marker, sync)
    }
}

pub fn read_git_context() -> Option<GitContext> {
    let branch_output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !branch_output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    let dirty = !String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .is_empty();

    let ahead_behind = std::process::Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<&str> = s.split('\t').collect();
                if parts.len() == 2 {
                    let ahead = parts[0].parse::<u32>().unwrap_or(0);
                    let behind = parts[1].parse::<u32>().unwrap_or(0);
                    return Some((ahead, behind));
                }
            }
            None
        })
        .unwrap_or((0, 0));

    Some(GitContext {
        branch,
        dirty,
        ahead: ahead_behind.0,
        behind: ahead_behind.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_line_clean_no_sync() {
        let ctx = GitContext {
            branch: "main".into(),
            dirty: false,
            ahead: 0,
            behind: 0,
        };
        assert_eq!(ctx.status_line(), "main");
    }

    #[test]
    fn status_line_dirty_with_ahead() {
        let ctx = GitContext {
            branch: "feat/x".into(),
            dirty: true,
            ahead: 3,
            behind: 0,
        };
        assert_eq!(ctx.status_line(), "feat/x* +3");
    }

    #[test]
    fn status_line_behind_only() {
        let ctx = GitContext {
            branch: "main".into(),
            dirty: false,
            ahead: 0,
            behind: 2,
        };
        assert_eq!(ctx.status_line(), "main -2");
    }

    #[test]
    fn status_line_ahead_and_behind() {
        let ctx = GitContext {
            branch: "dev".into(),
            dirty: true,
            ahead: 1,
            behind: 4,
        };
        assert_eq!(ctx.status_line(), "dev* +1/-4");
    }

    #[test]
    fn serde_roundtrip() {
        let ctx = GitContext {
            branch: "main".into(),
            dirty: true,
            ahead: 1,
            behind: 0,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: GitContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.branch, "main");
        assert!(parsed.dirty);
    }
}
