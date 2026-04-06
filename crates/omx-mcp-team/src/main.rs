use std::sync::Arc;

use rmcp::{tool, ServerHandler, ServiceExt};
use serde::Deserialize;
use tokio::sync::Mutex;

use uuid::Uuid;

use omx_team::config::parse_team_spec;
use omx_team::{DefaultTeamRuntime, TeamRuntime};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TeamStartParams {
    /// Number of workers to spawn
    pub workers: u32,
    /// Role for all workers (e.g. "executor", "planner")
    pub role: String,
    /// Task description
    pub task: String,
    /// Optional model override
    pub model: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TeamStatusParams {
    /// Team name (as returned by team_start)
    pub team_name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TeamWaitParams {
    /// Team name
    pub team_name: String,
    /// Maximum seconds to wait (default: 300)
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TeamCleanupParams {
    /// Team name
    pub team_name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TeamNudgeParams {
    /// Team name
    pub team_name: String,
    /// Worker ID to nudge
    pub worker_id: String,
    /// Optional message to send with the nudge
    pub message: Option<String>,
}

#[derive(Clone)]
struct TeamMcpServer {
    runtime: Arc<Mutex<DefaultTeamRuntime>>,
}

impl std::fmt::Debug for TeamMcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeamMcpServer").finish()
    }
}

#[rmcp::tool(tool_box)]
impl TeamMcpServer {
    #[tool(description = "Start a new team run with N workers of a given role")]
    async fn omx_run_team_start(&self, #[tool(aggr)] params: TeamStartParams) -> String {
        let config = match parse_team_spec(params.workers, &params.role, &params.task, params.model)
        {
            Ok(c) => c,
            Err(e) => return format!("{{\"error\": \"{e}\"}}"),
        };

        let team_name = config.name.0.clone();
        let job_id = Uuid::new_v4().to_string();

        let mut runtime = self.runtime.lock().await;
        match runtime.start(config).await {
            Ok(()) => serde_json::json!({
                "status": "started",
                "team_name": team_name,
                "job_id": job_id
            })
            .to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    #[tool(description = "Get current team status including phase, workers, and tasks")]
    async fn omx_run_team_status(&self, #[tool(aggr)] _params: TeamStatusParams) -> String {
        let runtime = self.runtime.lock().await;
        match runtime.monitor().await {
            Ok(snapshot) => serde_json::to_string_pretty(&snapshot).unwrap_or_default(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    #[tool(description = "Wait for team completion or timeout")]
    async fn omx_run_team_wait(&self, #[tool(aggr)] params: TeamWaitParams) -> String {
        let timeout = params.timeout_seconds.unwrap_or(300);
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout);

        loop {
            {
                let runtime = self.runtime.lock().await;
                match runtime.monitor().await {
                    Ok(snapshot) => {
                        let all_done = snapshot.tasks.iter().all(|t| {
                            t.status == omx_types::TaskStatus::Completed
                                || t.status == omx_types::TaskStatus::Failed
                        });
                        if all_done {
                            return serde_json::to_string_pretty(&snapshot).unwrap_or_default();
                        }
                    }
                    Err(e) => return format!("{{\"error\": \"{e}\"}}"),
                }
            }

            if tokio::time::Instant::now() >= deadline {
                return format!("{{\"error\": \"timeout after {timeout}s\"}}");
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    #[tool(description = "Clean up team resources (kill sessions, remove worktrees)")]
    async fn omx_run_team_cleanup(&self, #[tool(aggr)] _params: TeamCleanupParams) -> String {
        let mut runtime = self.runtime.lock().await;
        match runtime.shutdown().await {
            Ok(()) => r#"{"status": "cleaned_up"}"#.to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    #[tool(description = "Send a nudge to an idle worker to resume work")]
    async fn omx_run_team_nudge(&self, #[tool(aggr)] params: TeamNudgeParams) -> String {
        let worker_id = omx_types::WorkerId(params.worker_id.clone());
        let message = params
            .message
            .unwrap_or_else(|| "Nudge: please check your inbox and continue working.".to_string());

        let runtime = self.runtime.lock().await;
        let leader_id = omx_types::WorkerId("leader".to_string());
        match runtime.send_message(&leader_id, &worker_id, &message).await {
            Ok(()) => serde_json::json!({
                "status": "nudged",
                "worker_id": params.worker_id,
                "message": message
            })
            .to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }
}

#[rmcp::tool(tool_box)]
impl ServerHandler for TeamMcpServer {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let server = TeamMcpServer {
        runtime: Arc::new(Mutex::new(DefaultTeamRuntime::new())),
    };

    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
