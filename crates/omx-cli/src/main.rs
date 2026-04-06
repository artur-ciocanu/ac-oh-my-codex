mod cleanup;
mod doctor;
mod launch;

use clap::{Parser, Subcommand};
use omx_config::ConfigLoader;
use omx_hooks::HookDispatcher;
use omx_state::StateStore;
use omx_team::TeamRuntime;

/// OMX — orchestration layer for LLM CLIs
#[derive(Debug, Parser)]
#[command(name = "omx", version, about)]
struct Cli {
    /// Model override
    #[arg(long)]
    model: Option<String>,

    /// Provider override (codex or claude)
    #[arg(long)]
    provider: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Generate config, agents, prompts, skills, MCP entries
    Setup {
        #[arg(long, default_value = "user")]
        scope: String,
    },

    /// Verify installation and dependencies
    Doctor,

    /// Print version
    Version,

    /// Start team with N workers
    Team {
        #[command(subcommand)]
        action: TeamAction,
    },

    /// Read-only codebase exploration
    Explore {
        /// Prompt for exploration
        #[arg(long)]
        prompt: String,
    },

    /// Shell execution + output summarization
    Sparkshell {
        /// Command to execute
        command: Vec<String>,
    },

    /// ratatui status display
    Hud {
        /// Watch mode — continuously update
        #[arg(long)]
        watch: bool,
    },

    /// Direct provider query
    Ask {
        /// Provider name
        provider: String,
        /// Prompt text
        prompt: String,
    },

    /// Cancel active modes
    Cancel,

    /// Hook management
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },

    /// Internal callback API for hook executables
    HookApi {
        #[command(subcommand)]
        action: HookApiAction,
    },

    /// Run a single agent task (non-team)
    Exec {
        #[arg(long)]
        agent: String,
        prompt: String,
    },
    /// List available agent definitions
    Agents,
    /// Scaffold agent config files
    AgentsInit,
    /// Remove OMX configuration and artifacts
    Uninstall,
    /// Remove stale sessions, worktrees, lock files
    Cleanup,
    /// List/inspect session history
    Session {
        #[command(subcommand)]
        action: Option<SessionAction>,
    },
    /// Restore a previous session
    Resume { session_id: String },
    /// Start/resume Ralph persistent workflow
    Ralph { prompt: Option<String> },
    /// Start autoresearch loop
    Autoresearch { prompt: String },
    /// Start consensus planning session
    Ralplan { prompt: String },
    /// Run a named pipeline
    Pipeline { name: String },
    /// Invoked by tmux hooks (resize, pane close)
    TmuxHook {
        event: String,
        #[arg(long)]
        target: Option<String>,
    },
    /// Show current mode, active team, session metrics
    Status,
    /// Display/configure model reasoning settings
    Reasoning {
        #[arg(long)]
        effort: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum TeamAction {
    /// Start team: omx team start 3:executor "task"
    Start {
        /// Worker spec (e.g., "3:executor")
        spec: String,
        /// Task description
        task: String,
    },
    /// Check team health
    Status { name: String },
    /// Resume interrupted team
    Resume { name: String },
    /// Graceful cleanup
    Shutdown { name: String },
    /// Internal worker APIs
    Api {
        #[command(subcommand)]
        action: TeamApiAction,
    },
}

#[derive(Debug, Subcommand)]
enum TeamApiAction {
    ClaimTask {
        #[arg(long)]
        task_id: String,
    },
    TransitionTaskStatus {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        status: String,
    },
    ReleaseTaskClaim {
        #[arg(long)]
        task_id: String,
    },
}

#[derive(Debug, Subcommand)]
enum HooksAction {
    /// Show hook status
    Status,
    /// Validate hook configurations
    Validate,
    /// Test hook execution
    Test,
}

#[derive(Debug, Subcommand)]
enum SessionAction {
    List,
    Show { session_id: String },
}

#[derive(Debug, Subcommand)]
enum HookApiAction {
    /// Send tmux keys
    TmuxSendKeys {
        #[arg(long)]
        target: String,
        text: String,
    },
    /// Read state
    StateRead {
        #[arg(long)]
        mode: String,
        #[arg(long)]
        key: String,
    },
    /// Write state
    StateWrite {
        #[arg(long)]
        mode: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        value: String,
    },
    /// Read session info
    SessionRead,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        None => {
            let home = omx_config::default_codex_home();
            let env: std::collections::HashMap<String, String> = std::env::vars().collect();
            let config = omx_config::DefaultConfigLoader::load(&home, &env)
                .unwrap_or_else(|_| omx_config::OmxConfig::default());

            // Phase 1: Validate config
            let warnings = launch::validate_config(&config);
            for w in &warnings {
                eprintln!("warning: {w}");
            }

            // Phase 2: Inject AGENTS.md overlay
            let agents_path = home.join("AGENTS.md");
            let gen = omx_setup::DefaultSetupGenerator;
            use omx_setup::SetupGenerator;
            if let Ok(overlay) = gen.generate_agents_md(&config) {
                if let Err(e) = launch::inject_agents_overlay(&agents_path, &overlay) {
                    eprintln!("warning: failed to inject AGENTS.md: {e}");
                }
            }

            // Phase 3: Start session — launch provider
            let provider = cli.provider.as_deref().unwrap_or("codex");
            let model = cli.model.as_deref().unwrap_or(&config.models.frontier);
            let bin = match provider {
                "codex" => "codex",
                "claude" => "claude",
                other => {
                    eprintln!("Unknown provider: {other}. Supported: codex, claude");
                    std::process::exit(1);
                }
            };
            let args: Vec<String> = vec!["--model".into(), model.into()];
            tracing::info!("Launching {bin} with model {model}");
            let status = std::process::Command::new(bin)
                .args(&args)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Some(Commands::Setup { scope }) => {
            let scope = match scope.as_str() {
                "project" => omx_setup::SetupScope::Project,
                _ => omx_setup::SetupScope::User,
            };
            let config = omx_config::OmxConfig::default();
            let gen = omx_setup::DefaultSetupGenerator;
            use omx_setup::SetupGenerator;
            gen.sync_mcp_servers(&config, scope.clone())?;
            println!("MCP servers synced");
            let agents_md = gen.generate_agents_md(&config)?;
            let home = omx_config::default_codex_home();
            let agents_path = home.join("AGENTS.md");
            std::fs::write(&agents_path, agents_md)?;
            println!("AGENTS.md written to {}", agents_path.display());
            gen.copy_prompts(scope.clone())?;
            println!("Prompts synced");
            gen.copy_skills(scope)?;
            println!("Skills synced");
            println!("\nSetup complete.");
        }
        Some(Commands::Doctor) => {
            let home = omx_config::default_codex_home();
            let all_ok = doctor::run_doctor(&home);
            if !all_ok {
                std::process::exit(1);
            }
        }
        Some(Commands::Version) => {
            println!("omx {}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::Team { action }) => match action {
            TeamAction::Start { spec, task } => {
                let parts: Vec<&str> = spec.splitn(2, ':').collect();
                if parts.len() != 2 {
                    eprintln!("Invalid spec format. Expected N:role, e.g., 3:executor");
                    std::process::exit(1);
                }
                let count: u32 = parts[0]
                    .parse()
                    .map_err(|_| format!("Invalid worker count: {}", parts[0]))?;
                let role = parts[1];
                let config =
                    omx_team::config::parse_team_spec(count, role, &task, cli.model.clone())
                        .map_err(|e| format!("Invalid team spec: {e}"))?;
                let mut runtime = omx_team::DefaultTeamRuntime::new();
                runtime
                    .start(config)
                    .await
                    .map_err(|e| format!("Team start failed: {e}"))?;
                println!("Team started. Use `omx team status <name>` to check progress.");
            }
            TeamAction::Status { name } => {
                let runtime = omx_team::DefaultTeamRuntime::new();
                match runtime.monitor().await {
                    Ok(snapshot) => {
                        println!("Team: {}", snapshot.team_name);
                        println!("Phase: {:?}", snapshot.phase);
                        println!("Workers: {}", snapshot.workers.len());
                        println!("Tasks: {} total", snapshot.tasks.len());
                        println!("Uptime: {}s", snapshot.uptime_seconds);
                        let _ = name;
                    }
                    Err(e) => eprintln!("Failed to get team status: {e}"),
                }
            }
            TeamAction::Resume { name } => {
                let runtime = omx_team::DefaultTeamRuntime::new();
                println!("Resuming team '{name}'...");
                match runtime.monitor().await {
                    Ok(snapshot) => println!(
                        "Team '{}' resumed, phase: {:?}",
                        snapshot.team_name, snapshot.phase
                    ),
                    Err(e) => eprintln!("Failed to resume team: {e}"),
                }
            }
            TeamAction::Shutdown { name } => {
                let mut runtime = omx_team::DefaultTeamRuntime::new();
                println!("Shutting down team '{name}'...");
                runtime
                    .shutdown()
                    .await
                    .map_err(|e| format!("Shutdown failed: {e}"))?;
                println!("Team '{name}' shut down.");
            }
            TeamAction::Api { action } => match action {
                TeamApiAction::ClaimTask { task_id } => {
                    let runtime = omx_team::DefaultTeamRuntime::new();
                    let worker_id = omx_types::WorkerId("cli".into());
                    let task_id = omx_types::TaskId(task_id);
                    match runtime.claim_task(&worker_id, &task_id).await {
                        Ok(token) => println!("{}", serde_json::to_string(&token)?),
                        Err(e) => eprintln!("Claim failed: {e}"),
                    }
                }
                TeamApiAction::TransitionTaskStatus { task_id, status } => {
                    let runtime = omx_team::DefaultTeamRuntime::new();
                    let task_id = omx_types::TaskId(task_id);
                    let status: omx_types::TaskStatus =
                        serde_json::from_str(&format!("\"{status}\""))
                            .map_err(|e| format!("Invalid status '{status}': {e}"))?;
                    let token = omx_types::LeaseToken("cli".into());
                    runtime
                        .transition_task(&task_id, &token, status, None)
                        .await
                        .map_err(|e| format!("Transition failed: {e}"))?;
                    println!("ok");
                }
                TeamApiAction::ReleaseTaskClaim { task_id } => {
                    let runtime = omx_team::DefaultTeamRuntime::new();
                    let task_id = omx_types::TaskId(task_id);
                    let token = omx_types::LeaseToken("cli".into());
                    runtime
                        .transition_task(&task_id, &token, omx_types::TaskStatus::Pending, None)
                        .await
                        .map_err(|e| format!("Release failed: {e}"))?;
                    println!("ok");
                }
            },
        },
        Some(Commands::Explore { prompt }) => {
            let status = std::process::Command::new("omx-explore")
                .arg("--prompt")
                .arg(&prompt)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Some(Commands::Sparkshell { command }) => {
            let status = std::process::Command::new("omx-sparkshell")
                .args(&command)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Some(Commands::Hud { watch: _ }) => {
            let state = omx_hud::HudState::default();
            omx_hud::run_hud(state).await?;
        }
        Some(Commands::Ask { provider, prompt }) => {
            let bin = match provider.as_str() {
                "codex" => "codex",
                "claude" => "claude",
                other => {
                    eprintln!("Unknown provider: {other}. Supported: codex, claude");
                    std::process::exit(1);
                }
            };
            let status = std::process::Command::new(bin)
                .arg(&prompt)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Some(Commands::Cancel) => {
            println!("Cancelling active modes...");
            println!("No active modes to cancel.");
        }
        Some(Commands::Hooks { action }) => match action {
            HooksAction::Status => {
                let home = omx_config::default_codex_home();
                let hooks_dir = home.join(".omx").join("hooks");
                let dispatcher = omx_hooks::ShellHookDispatcher::new(5000);
                match dispatcher.discover(&hooks_dir) {
                    Ok(hooks) => {
                        if hooks.is_empty() {
                            println!("No hooks found in {}", hooks_dir.display());
                        } else {
                            println!("Discovered {} hooks:", hooks.len());
                            for hook in &hooks {
                                let exe = if hook.executable { "+" } else { "-" };
                                println!("  [{exe}] {} — {}", hook.name, hook.path.display());
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to discover hooks: {e}"),
                }
            }
            HooksAction::Validate => {
                let home = omx_config::default_codex_home();
                let hooks_dir = home.join(".omx").join("hooks");
                let dispatcher = omx_hooks::ShellHookDispatcher::new(5000);
                match dispatcher.discover(&hooks_dir) {
                    Ok(hooks) => {
                        let mut valid = 0;
                        let mut invalid = 0;
                        for hook in &hooks {
                            if hook.executable {
                                valid += 1;
                            } else {
                                invalid += 1;
                                eprintln!("  WARN: {} is not executable", hook.path.display());
                            }
                        }
                        println!("{valid} valid, {invalid} invalid hooks");
                    }
                    Err(e) => eprintln!("Failed to discover hooks: {e}"),
                }
            }
            HooksAction::Test => {
                let home = omx_config::default_codex_home();
                let hooks_dir = home.join(".omx").join("hooks");
                let dispatcher = omx_hooks::ShellHookDispatcher::new(5000);
                let hooks = dispatcher.discover(&hooks_dir).unwrap_or_default();
                let dispatcher = dispatcher.with_hooks(hooks);
                let event = omx_types::HookEvent {
                    schema_version: "1".into(),
                    event: omx_types::HookEventName::SessionStart,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    source: omx_types::HookSource {
                        component: "cli".into(),
                        worker_id: None,
                    },
                    context: serde_json::json!({"test": true}),
                    session_id: None,
                };
                let results = dispatcher.dispatch(&event).await;
                for r in &results {
                    let status = if r.success { "PASS" } else { "FAIL" };
                    println!("  [{status}] {} ({}ms)", r.hook, r.duration_ms);
                    if !r.stderr.is_empty() {
                        eprintln!("    stderr: {}", r.stderr);
                    }
                }
                println!("{} hooks tested", results.len());
            }
        },
        Some(Commands::HookApi { action }) => {
            match action {
                HookApiAction::TmuxSendKeys { target, text } => {
                    if !target.starts_with("omx-") && !target.contains("omx-") {
                        eprintln!("Warning: target '{target}' does not appear to be an OMX-managed session");
                    }
                    let output = std::process::Command::new("tmux")
                        .args(["send-keys", "-t", &target, &text, "Enter"])
                        .output()
                        .map_err(|e| format!("tmux send-keys failed: {e}"))?;
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("tmux send-keys error: {stderr}");
                    }
                }
                HookApiAction::StateRead { mode, key } => {
                    if mode.contains("..")
                        || mode.contains('/')
                        || mode.contains('\\')
                        || key.contains("..")
                        || key.contains('/')
                        || key.contains('\\')
                    {
                        eprintln!("Invalid mode or key: must not contain path separators or '..'");
                        std::process::exit(1);
                    }
                    let home = omx_config::default_codex_home();
                    let store = omx_state::FileStateStore::new(home.join(".omx"));
                    let path = std::path::PathBuf::from(format!("{mode}/{key}.json"));
                    let value: Option<serde_json::Value> = store.read(&path).await?;
                    match value {
                        Some(v) => println!("{}", serde_json::to_string_pretty(&v)?),
                        None => eprintln!("no value found for {mode}/{key}"),
                    }
                }
                HookApiAction::StateWrite { mode, key, value } => {
                    if mode.contains("..")
                        || mode.contains('/')
                        || mode.contains('\\')
                        || key.contains("..")
                        || key.contains('/')
                        || key.contains('\\')
                    {
                        eprintln!("Invalid mode or key: must not contain path separators or '..'");
                        std::process::exit(1);
                    }
                    let home = omx_config::default_codex_home();
                    let store = omx_state::FileStateStore::new(home.join(".omx"));
                    let path = std::path::PathBuf::from(format!("{mode}/{key}.json"));
                    let parsed: serde_json::Value = serde_json::from_str(&value)?;
                    store.write(&path, &parsed).await?;
                    println!("ok");
                }
                HookApiAction::SessionRead => {
                    let home = omx_config::default_codex_home();
                    let store = omx_state::FileStateStore::new(home.join(".omx"));
                    let path = std::path::PathBuf::from("session/current.json");
                    let value: Option<serde_json::Value> = store.read(&path).await?;
                    match value {
                        Some(v) => println!("{}", serde_json::to_string_pretty(&v)?),
                        None => println!("{{}}"),
                    }
                }
            }
        }
        Some(Commands::Exec { agent, prompt }) => {
            println!("exec: agent={agent}, prompt={prompt}");
            println!("(not yet wired)");
        }
        Some(Commands::Agents) => {
            println!("Available agents:");
            println!("  (agent listing not yet wired)");
        }
        Some(Commands::AgentsInit) => {
            println!("Scaffolding agent config files...");
            println!("  (not yet wired)");
        }
        Some(Commands::Uninstall) => {
            println!("Uninstalling OMX...");
            println!("  (not yet wired — would remove ~/.codex/.omx)");
        }
        Some(Commands::Cleanup) => {
            let home = omx_config::default_codex_home();
            match cleanup::run_cleanup(&home) {
                Ok(report) => {
                    println!("Cleanup complete:");
                    println!("  {} lock files removed", report.stale_locks_removed);
                    println!("  {} stale sessions removed", report.stale_sessions_removed);
                    println!(
                        "  {} stale worktrees removed",
                        report.stale_worktrees_removed
                    );
                }
                Err(e) => eprintln!("Cleanup failed: {e}"),
            }
        }
        Some(Commands::Session { action }) => match action {
            Some(SessionAction::List) | None => {
                println!("Recent sessions:");
                println!("  (session listing not yet wired)");
            }
            Some(SessionAction::Show { session_id }) => {
                println!("Session: {session_id}");
                println!("  (session detail not yet wired)");
            }
        },
        Some(Commands::Resume { session_id }) => {
            println!("Resuming session {session_id}...");
            println!("  (not yet wired)");
        }
        Some(Commands::Ralph { prompt }) => {
            let desc = prompt.as_deref().unwrap_or("(interactive)");
            println!("Starting Ralph workflow: {desc}");
            println!("  (not yet wired)");
        }
        Some(Commands::Autoresearch { prompt }) => {
            println!("Starting autoresearch: {prompt}");
            println!("  (not yet wired)");
        }
        Some(Commands::Ralplan { prompt }) => {
            println!("Starting consensus planning: {prompt}");
            println!("  (not yet wired)");
        }
        Some(Commands::Pipeline { name }) => {
            println!("Running pipeline: {name}");
            println!("  (not yet wired)");
        }
        Some(Commands::TmuxHook { event, target }) => {
            let tgt = target.as_deref().unwrap_or("(none)");
            println!("tmux-hook: event={event}, target={tgt}");
        }
        Some(Commands::Status) => {
            let home = omx_config::default_codex_home();
            let env: std::collections::HashMap<String, String> = std::env::vars().collect();
            let config = omx_config::DefaultConfigLoader::load(&home, &env)
                .unwrap_or_else(|_| omx_config::OmxConfig::default());
            println!("OMX Status");
            println!("  Model: {}", config.models.frontier);
            println!("  Mode: (idle)");
            println!("  Session: (none active)");
        }
        Some(Commands::Reasoning { effort }) => match effort {
            Some(e) => println!("Reasoning effort set to: {e}"),
            None => println!("Current reasoning effort: (default)"),
        },
    }

    Ok(())
}
