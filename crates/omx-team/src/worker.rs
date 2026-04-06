use async_trait::async_trait;
use omx_types::{OmxError, WorkerId};
use std::path::Path;

use crate::config::{TeamConfig, TeamTask, WorkerConfig};
use crate::tmux_session;

#[async_trait]
pub trait WorkerBootstrap: Send + Sync {
    async fn spawn_worker(
        &self,
        config: &WorkerConfig,
        team: &TeamConfig,
        worker_index: u32,
    ) -> Result<WorkerId, OmxError>;

    async fn compose_agents_md(
        &self,
        worker: &WorkerConfig,
        team: &TeamConfig,
    ) -> Result<String, OmxError>;

    async fn write_inbox(
        &self,
        state_dir: &Path,
        worker: &WorkerId,
        tasks: &[TeamTask],
    ) -> Result<(), OmxError>;
}

pub struct DefaultWorkerBootstrap;

#[async_trait]
impl WorkerBootstrap for DefaultWorkerBootstrap {
    async fn spawn_worker(
        &self,
        config: &WorkerConfig,
        team: &TeamConfig,
        worker_index: u32,
    ) -> Result<WorkerId, OmxError> {
        let worker_name = format!("worker-{worker_index}");
        let worker_id = WorkerId(worker_name.clone());
        let session_name = format!("omx-team-{}", team.name.0);

        // Create tmux window for this worker
        let target = tmux_session::create_worker_window(&session_name, &worker_name)?;

        // Compose AGENTS.md for the worker
        let agents_md = self.compose_agents_md(config, team).await?;
        tracing::info!(
            worker = %worker_name,
            target = %target,
            agents_md_len = agents_md.len(),
            "spawned worker"
        );

        Ok(worker_id)
    }

    async fn compose_agents_md(
        &self,
        worker: &WorkerConfig,
        team: &TeamConfig,
    ) -> Result<String, OmxError> {
        let mut md = String::new();
        md.push_str(&format!("# Worker: {} role\n\n", worker.role));
        md.push_str(&format!("## Team: {}\n\n", team.name.0));
        md.push_str("## Tasks\n\n");

        for (i, task) in team.tasks.iter().enumerate() {
            md.push_str(&format!("{}. {}\n", i + 1, task.description));
        }

        md.push_str("\n## Constraints\n\n");
        md.push_str("- Work only on assigned tasks\n");
        md.push_str("- Use `omx team api claim-task` before starting work\n");
        md.push_str("- Use `omx team api transition-task-status` when done\n");

        if let Some(model) = &worker.model {
            md.push_str(&format!("\n## Model: {model}\n"));
        }

        Ok(md)
    }

    async fn write_inbox(
        &self,
        state_dir: &Path,
        worker: &WorkerId,
        tasks: &[TeamTask],
    ) -> Result<(), OmxError> {
        let inbox_dir = state_dir.join(format!("team/inbox/{}", worker.0));
        tokio::fs::create_dir_all(&inbox_dir)
            .await
            .map_err(OmxError::Io)?;

        let inbox_file = inbox_dir.join("tasks.json");
        let json = serde_json::to_string_pretty(tasks)?;
        tokio::fs::write(&inbox_file, json)
            .await
            .map_err(OmxError::Io)?;

        tracing::info!(worker = %worker.0, path = %inbox_file.display(), "wrote inbox");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use omx_types::TeamName;

    #[tokio::test]
    async fn compose_agents_md_includes_role_and_tasks() {
        let bootstrap = DefaultWorkerBootstrap;
        let config = WorkerConfig {
            role: "executor".into(),
            model: Some("o3".into()),
            provider: None,
        };
        let team = TeamConfig {
            name: TeamName("test-team".into()),
            workers: vec![config.clone()],
            tasks: vec![TeamTask {
                description: "build the feature".into(),
                depends_on: vec![],
                phase: None,
            }],
            governance: TeamGovernance::default(),
            worktree_mode: WorktreeMode::PerWorker,
            dispatch_mode: DispatchMode::Tmux,
            heartbeat_interval_secs: 30,
            heartbeat_stale_secs: 90,
            merge_strategy: crate::merge_strategy::MergeStrategy::default(),
            auto_commit: crate::auto_commit::AutoCommitConfig::default(),
            approval_required: false,
        };

        let md = bootstrap.compose_agents_md(&config, &team).await.unwrap();
        assert!(md.contains("executor"));
        assert!(md.contains("test-team"));
        assert!(md.contains("build the feature"));
        assert!(md.contains("Model: o3"));
    }

    #[tokio::test]
    async fn write_inbox_creates_file() {
        let bootstrap = DefaultWorkerBootstrap;
        let dir = tempfile::tempdir().unwrap();
        let worker = WorkerId("worker-0".into());
        let tasks = vec![TeamTask {
            description: "do stuff".into(),
            depends_on: vec![],
            phase: None,
        }];

        bootstrap
            .write_inbox(dir.path(), &worker, &tasks)
            .await
            .unwrap();

        let inbox_file = dir.path().join("team/inbox/worker-0/tasks.json");
        assert!(inbox_file.exists());
        let content = std::fs::read_to_string(inbox_file).unwrap();
        assert!(content.contains("do stuff"));
    }
}
