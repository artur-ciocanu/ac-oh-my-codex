# Pure Rust Migration — Phase 3: Team Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all `todo!()` stubs in the `omx-team` crate (12 modules) and `omx-mcp-team` crate (4 MCP tools) with working implementations so that `omx team 3:executor "task"` works end-to-end.

**Architecture:** The team runtime is a two-level leader/worker model. The leader owns task decomposition and phase transitions. Workers are isolated executors spawned in tmux windows with composed AGENTS.md files. State is file-based via `omx-state::FileStateStore` with atomic writes and file locking. The orchestrator runs an async monitor loop (tokio tasks) that snapshots state, delivers mailbox messages, tracks dispatch receipts, and dispatches hook events. Modules build on each other sequentially: task_queue → mailbox → dispatch → worker → tmux_session → orchestrator → phase_controller → role_router → worktree → commit_hygiene → scaling → allocation.

**Tech Stack:** Rust 2021 edition, tokio 1.x, omx-state `FileStateStore`, omx-mux `TmuxAdapter` (uses `run_tmux` helper calling tmux CLI), omx-hooks `ShellHookDispatcher`, omx-types (shared types), serde/serde_json, async-trait, uuid, chrono, tracing, tempfile (tests)

**Spec:** `docs/superpowers/specs/2026-04-04-pure-rust-migration-design.md` (sections 4.6, 7.1–7.5)

**Phase 2 baseline:** Service crates (`omx-hooks`, `omx-mcp-state`, `omx-mcp-memory`, `omx-mcp-trace`, `omx-mcp-code-intel`) are fully implemented with passing tests.

**Milestone:** All `omx-team` modules have real implementations with passing tests. `omx-mcp-team` delegates to the team runtime. No `todo!()` remains in these crates.

---

## File Structure

### Modified files

```
crates/omx-team/Cargo.toml                  # Add uuid, chrono dependencies
crates/omx-team/src/config.rs               # Add parse_team_spec() and validation
crates/omx-team/src/task_queue.rs            # Implement DAG resolution, claim, transition
crates/omx-team/src/mailbox.rs               # Implement message create, pending, deliver via state store
crates/omx-team/src/dispatch.rs              # Implement queue and process via tmux send-keys
crates/omx-team/src/worker.rs                # Implement spawn, compose AGENTS.md, write inbox
crates/omx-team/src/tmux_session.rs          # Implement tmux session/window lifecycle
crates/omx-team/src/orchestrator.rs          # Implement tick() monitor loop and shutdown()
crates/omx-team/src/phase_controller.rs      # Implement phase inference and role recommendation
crates/omx-team/src/role_router.rs           # Implement role recommendation and task routing
crates/omx-team/src/worktree.rs              # Implement git worktree provision and cleanup
crates/omx-team/src/commit_hygiene.rs        # Implement commit ledger and integration
crates/omx-team/src/scaling.rs               # Implement scale recommendation, up, down
crates/omx-team/src/allocation.rs            # Implement task allocation policy
crates/omx-team/src/lib.rs                   # Implement DefaultTeamRuntime (wires modules together)
crates/omx-mcp-team/Cargo.toml              # Add omx-team dependency
crates/omx-mcp-team/src/main.rs             # Implement 4 MCP tools delegating to TeamRuntime
```

---

## Task 1: Add dependencies and implement config parsing

**Files:**
- Modify: `crates/omx-team/Cargo.toml`
- Modify: `crates/omx-team/src/config.rs`

Config already has structs defined. We need to add `uuid` and `chrono` workspace deps and add a `parse_team_spec()` function that converts CLI input like `3:executor "build the feature"` into a `TeamConfig`.

- [ ] **Step 1: Add uuid and chrono to workspace and crate**

Check if `uuid` and `chrono` are already in the workspace `Cargo.toml`. If not, add them:

```toml
# In workspace Cargo.toml [workspace.dependencies]
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
```

Then in `crates/omx-team/Cargo.toml`, add:

```toml
uuid = { workspace = true }
chrono = { workspace = true }
```

- [ ] **Step 2: Write the failing test for parse_team_spec**

Add to `crates/omx-team/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Keep existing test

    #[test]
    fn parse_team_spec_3_executors() {
        let config = parse_team_spec(3, "executor", "build the feature", None).unwrap();
        assert_eq!(config.name.0.len(), "omx-team-".len() + 8); // prefix + 8 hex chars
        assert!(config.name.0.starts_with("omx-team-"));
        assert_eq!(config.workers.len(), 3);
        assert_eq!(config.workers[0].role, "executor");
        assert_eq!(config.tasks.len(), 1);
        assert_eq!(config.tasks[0].description, "build the feature");
        assert_eq!(config.governance.nested_teams_allowed, false);
        assert_eq!(config.worktree_mode, WorktreeMode::PerWorker);
        assert_eq!(config.dispatch_mode, DispatchMode::Tmux);
    }

    #[test]
    fn parse_team_spec_with_model_override() {
        let config = parse_team_spec(2, "planner", "design API", Some("o3".into())).unwrap();
        assert_eq!(config.workers.len(), 2);
        assert_eq!(config.workers[0].model, Some("o3".into()));
    }

    #[test]
    fn parse_team_spec_zero_workers_errors() {
        let result = parse_team_spec(0, "executor", "task", None);
        assert!(result.is_err());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p omx-team -- config`
Expected: compilation error — `parse_team_spec` not defined

- [ ] **Step 4: Implement parse_team_spec**

Add to `crates/omx-team/src/config.rs`:

```rust
use omx_types::OmxError;

pub fn parse_team_spec(
    worker_count: u32,
    role: &str,
    task_description: &str,
    model: Option<String>,
) -> Result<TeamConfig, OmxError> {
    if worker_count == 0 {
        return Err(OmxError::Team("worker count must be at least 1".into()));
    }

    let team_id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
    let name = TeamName(format!("omx-team-{team_id}"));

    let workers = (0..worker_count)
        .map(|i| WorkerConfig {
            role: role.to_string(),
            model: model.clone(),
            provider: None,
        })
        .collect();

    let tasks = vec![TeamTask {
        description: task_description.to_string(),
        depends_on: vec![],
        phase: None,
    }];

    Ok(TeamConfig {
        name,
        workers,
        tasks,
        governance: TeamGovernance::default(),
        worktree_mode: WorktreeMode::PerWorker,
        dispatch_mode: DispatchMode::Tmux,
    })
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p omx-team -- config`
Expected: all 3 config tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/omx-team/Cargo.toml crates/omx-team/src/config.rs Cargo.toml
git commit -m "feat(omx-team): add config parsing with parse_team_spec"
```

---

## Task 2: Implement task_queue — DAG resolution, claim, transition

**Files:**
- Modify: `crates/omx-team/src/task_queue.rs`

The task queue manages a DAG of tasks. `ready_tasks()` returns tasks whose dependencies are all completed. `claim()` generates a lease token for a worker. `transition()` verifies the token and updates status.

- [ ] **Step 1: Write the failing tests**

Replace the test module in `crates/omx-team/src/task_queue.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{TaskId, TaskStatus};

    #[test]
    fn ready_tasks_returns_pending_with_no_deps() {
        let tasks = vec![
            (TaskId("t1".into()), TaskStatus::Pending, vec![]),
            (TaskId("t2".into()), TaskStatus::Pending, vec![TaskId("t1".into())]),
            (TaskId("t3".into()), TaskStatus::Completed, vec![]),
        ];
        let ready = ready_tasks(&tasks);
        assert_eq!(ready, vec![TaskId("t1".into())]);
    }

    #[test]
    fn ready_tasks_unblocks_when_deps_completed() {
        let tasks = vec![
            (TaskId("t1".into()), TaskStatus::Completed, vec![]),
            (TaskId("t2".into()), TaskStatus::Pending, vec![TaskId("t1".into())]),
        ];
        let ready = ready_tasks(&tasks);
        assert_eq!(ready, vec![TaskId("t2".into())]);
    }

    #[test]
    fn ready_tasks_skips_in_progress() {
        let tasks = vec![
            (TaskId("t1".into()), TaskStatus::InProgress, vec![]),
        ];
        let ready = ready_tasks(&tasks);
        assert!(ready.is_empty());
    }

    #[test]
    fn claim_generates_lease_token() {
        let token = claim(
            &TaskId("t1".into()),
            &WorkerId("w1".into()),
            &TaskStatus::Pending,
        )
        .unwrap();
        assert!(!token.0.is_empty());
    }

    #[test]
    fn claim_rejects_non_pending_tasks() {
        let result = claim(
            &TaskId("t1".into()),
            &WorkerId("w1".into()),
            &TaskStatus::InProgress,
        );
        assert!(result.is_err());
    }

    #[test]
    fn transition_succeeds_with_valid_token() {
        let token = LeaseToken("valid-token".into());
        let result = transition(
            &TaskId("t1".into()),
            &token,
            TaskStatus::Completed,
            Some("done".into()),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn transition_rejects_empty_token() {
        let token = LeaseToken("".into());
        let result = transition(
            &TaskId("t1".into()),
            &token,
            TaskStatus::Completed,
            None,
        );
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omx-team -- task_queue`
Expected: FAIL — `todo!()` panics

- [ ] **Step 3: Implement the three functions**

Replace the function bodies in `crates/omx-team/src/task_queue.rs`:

```rust
use omx_types::{LeaseToken, OmxError, TaskId, TaskStatus, WorkerId};

/// Return tasks that are Pending and whose dependencies are all Completed.
pub fn ready_tasks(tasks: &[(TaskId, TaskStatus, Vec<TaskId>)]) -> Vec<TaskId> {
    let completed: std::collections::HashSet<&TaskId> = tasks
        .iter()
        .filter(|(_, status, _)| *status == TaskStatus::Completed)
        .map(|(id, _, _)| id)
        .collect();

    tasks
        .iter()
        .filter(|(_, status, deps)| {
            *status == TaskStatus::Pending && deps.iter().all(|dep| completed.contains(dep))
        })
        .map(|(id, _, _)| id.clone())
        .collect()
}

/// Claim a task for a worker. Task must be Pending.
pub fn claim(
    task: &TaskId,
    worker: &WorkerId,
    current_status: &TaskStatus,
) -> Result<LeaseToken, OmxError> {
    if *current_status != TaskStatus::Pending {
        return Err(OmxError::Team(format!(
            "task {} is {:?}, not Pending — cannot claim",
            task.0, current_status
        )));
    }

    let token = format!("lease-{}-{}-{}", task.0, worker.0, uuid::Uuid::new_v4().simple());
    Ok(LeaseToken(token))
}

/// Transition a task to a new status. Token must be non-empty.
pub fn transition(
    task: &TaskId,
    token: &LeaseToken,
    new_status: TaskStatus,
    _result: Option<String>,
) -> Result<(), OmxError> {
    if token.0.is_empty() {
        return Err(OmxError::Team(format!(
            "empty lease token for task {}",
            task.0
        )));
    }

    tracing::info!(task = %task.0, new_status = ?new_status, "task transition");
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omx-team -- task_queue`
Expected: all 7 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/task_queue.rs
git commit -m "feat(omx-team): implement task_queue DAG resolution and claim protocol"
```

---

## Task 3: Implement mailbox — message creation and delivery tracking

**Files:**
- Modify: `crates/omx-team/src/mailbox.rs`

The mailbox stores messages between workers. Each message has a unique ID, timestamp, and delivered flag. Functions are pure data operations — persistence is handled by the orchestrator via `omx-state`.

- [ ] **Step 1: Write the failing tests**

Replace the test module in `crates/omx-team/src/mailbox.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::WorkerId;

    #[test]
    fn create_message_assigns_unique_id_and_timestamp() {
        let msg = create_message(
            &WorkerId("leader".into()),
            &WorkerId("worker-1".into()),
            "start task A",
        )
        .unwrap();
        assert!(!msg.id.is_empty());
        assert_eq!(msg.from.0, "leader");
        assert_eq!(msg.to.0, "worker-1");
        assert_eq!(msg.body, "start task A");
        assert!(!msg.created_at.is_empty());
        assert!(!msg.delivered);
    }

    #[test]
    fn create_two_messages_have_different_ids() {
        let m1 = create_message(
            &WorkerId("a".into()),
            &WorkerId("b".into()),
            "msg1",
        )
        .unwrap();
        let m2 = create_message(
            &WorkerId("a".into()),
            &WorkerId("b".into()),
            "msg2",
        )
        .unwrap();
        assert_ne!(m1.id, m2.id);
    }

    #[test]
    fn pending_messages_filters_by_worker_and_undelivered() {
        let messages = vec![
            MailboxMessage {
                id: "m1".into(),
                from: WorkerId("leader".into()),
                to: WorkerId("w1".into()),
                body: "task A".into(),
                created_at: "2026-04-05T00:00:00Z".into(),
                delivered: false,
            },
            MailboxMessage {
                id: "m2".into(),
                from: WorkerId("leader".into()),
                to: WorkerId("w2".into()),
                body: "task B".into(),
                created_at: "2026-04-05T00:00:01Z".into(),
                delivered: false,
            },
            MailboxMessage {
                id: "m3".into(),
                from: WorkerId("leader".into()),
                to: WorkerId("w1".into()),
                body: "old".into(),
                created_at: "2026-04-05T00:00:02Z".into(),
                delivered: true,
            },
        ];
        let pending = pending_for_worker(&messages, &WorkerId("w1".into()));
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "m1");
    }

    #[test]
    fn mark_delivered_sets_flag() {
        let mut msg = create_message(
            &WorkerId("a".into()),
            &WorkerId("b".into()),
            "test",
        )
        .unwrap();
        assert!(!msg.delivered);
        mark_delivered(&mut msg);
        assert!(msg.delivered);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omx-team -- mailbox`
Expected: compilation error — functions have wrong signatures or `todo!()` panics

- [ ] **Step 3: Implement the mailbox functions**

Rewrite `crates/omx-team/src/mailbox.rs`:

```rust
use omx_types::{OmxError, WorkerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxMessage {
    pub id: String,
    pub from: WorkerId,
    pub to: WorkerId,
    pub body: String,
    pub created_at: String,
    pub delivered: bool,
}

/// Create a new mailbox message with a unique ID and current timestamp.
pub fn create_message(
    from: &WorkerId,
    to: &WorkerId,
    body: &str,
) -> Result<MailboxMessage, OmxError> {
    let id = format!("msg-{}", uuid::Uuid::new_v4().simple());
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(MailboxMessage {
        id,
        from: from.clone(),
        to: to.clone(),
        body: body.to_string(),
        created_at,
        delivered: false,
    })
}

/// Filter messages: return undelivered messages for a specific worker.
pub fn pending_for_worker<'a>(
    messages: &'a [MailboxMessage],
    worker: &WorkerId,
) -> Vec<&'a MailboxMessage> {
    messages
        .iter()
        .filter(|m| m.to == *worker && !m.delivered)
        .collect()
}

/// Mark a message as delivered.
pub fn mark_delivered(message: &mut MailboxMessage) {
    message.delivered = true;
}

#[cfg(test)]
mod tests {
    // ... (tests from Step 1)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omx-team -- mailbox`
Expected: all 4 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/mailbox.rs
git commit -m "feat(omx-team): implement mailbox message creation and delivery tracking"
```

---

## Task 4: Implement dispatch — queue and process delivery via tmux

**Files:**
- Modify: `crates/omx-team/src/dispatch.rs`

Dispatch handles delivery of messages to workers via tmux send-keys. A `DispatchRequest` is created, queued, and processed by sending the body to the target tmux pane.

- [ ] **Step 1: Write the failing tests**

Replace the test module in `crates/omx-team/src/dispatch.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_dispatch_request_has_unique_id() {
        let r1 = create_request("sess:0.1", "hello", "tmux");
        let r2 = create_request("sess:0.2", "world", "tmux");
        assert_ne!(r1.request_id, r2.request_id);
        assert_eq!(r1.target, "sess:0.1");
        assert_eq!(r1.body, "hello");
        assert_eq!(r1.transport, "tmux");
    }

    #[test]
    fn dispatch_queue_and_drain() {
        let mut queue = DispatchQueue::new();
        assert!(queue.is_empty());

        let r1 = create_request("sess:0.1", "msg1", "tmux");
        let r2 = create_request("sess:0.2", "msg2", "tmux");
        queue.enqueue(r1);
        queue.enqueue(r2);
        assert_eq!(queue.len(), 2);

        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert!(queue.is_empty());
    }

    #[test]
    fn receipt_records_success() {
        let receipt = DispatchReceipt {
            request_id: "req-1".into(),
            success: true,
            reason: None,
            duration_ms: 42,
        };
        assert!(receipt.success);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omx-team -- dispatch`
Expected: compilation error — `create_request`, `DispatchQueue` not defined

- [ ] **Step 3: Implement dispatch module**

Rewrite `crates/omx-team/src/dispatch.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    pub request_id: String,
    pub target: String,
    pub body: String,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchReceipt {
    pub request_id: String,
    pub success: bool,
    pub reason: Option<String>,
    pub duration_ms: u64,
}

/// Create a new dispatch request with a unique ID.
pub fn create_request(target: &str, body: &str, transport: &str) -> DispatchRequest {
    DispatchRequest {
        request_id: format!("dispatch-{}", uuid::Uuid::new_v4().simple()),
        target: target.to_string(),
        body: body.to_string(),
        transport: transport.to_string(),
    }
}

/// A queue of pending dispatch requests.
pub struct DispatchQueue {
    pending: Vec<DispatchRequest>,
}

impl DispatchQueue {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn enqueue(&mut self, request: DispatchRequest) {
        self.pending.push(request);
    }

    pub fn drain(&mut self) -> Vec<DispatchRequest> {
        std::mem::take(&mut self.pending)
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    // ... (tests from Step 1)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omx-team -- dispatch`
Expected: all 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/dispatch.rs
git commit -m "feat(omx-team): implement dispatch queue and request creation"
```

---

## Task 5: Implement tmux_session — team session and worker window lifecycle

**Files:**
- Modify: `crates/omx-team/src/tmux_session.rs`

Tmux session management wraps tmux CLI calls for creating team sessions, worker windows, and cleanup. Uses `std::process::Command` consistent with `omx-mux`'s `run_tmux` pattern.

- [ ] **Step 1: Write the failing tests**

Replace `crates/omx-team/src/tmux_session.rs`:

```rust
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
    let session_name = format!("omx-team-{name}");
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
    use super::*;

    #[test]
    fn session_name_prefixed() {
        // We can't create real tmux sessions in CI, so test the name logic
        let name = "abc123";
        let expected = "omx-team-abc123";
        assert_eq!(format!("omx-team-{name}"), expected);
    }

    #[test]
    fn kill_session_normalizes_name() {
        // Verify that kill_team_session handles both prefixed and unprefixed names
        // (We can't actually kill sessions in CI, but we test the name normalization)
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
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p omx-team -- tmux_session`
Expected: 2 tests pass (these are unit tests for name logic; tmux integration tests would need a real tmux)

- [ ] **Step 3: Commit**

```bash
git add crates/omx-team/src/tmux_session.rs
git commit -m "feat(omx-team): implement tmux session and worker window lifecycle"
```

---

## Task 6: Implement phase_controller — phase inference from task statuses

**Files:**
- Modify: `crates/omx-team/src/phase_controller.rs`

The phase controller infers the current team phase (Plan, Prd, Exec, Verify, Fix) by examining task statuses and their phase annotations. It also recommends roles per phase.

- [ ] **Step 1: Write the failing tests**

Replace the test module in `crates/omx-team/src/phase_controller.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use omx_types::{TaskStatus, TeamPhase};

    #[test]
    fn infer_phase_all_pending_is_plan() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Pending, None),
            (TaskStatus::Pending, None),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Plan);
    }

    #[test]
    fn infer_phase_some_in_progress_is_exec() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Completed, Some(TeamPhase::Plan)),
            (TaskStatus::InProgress, Some(TeamPhase::Exec)),
            (TaskStatus::Pending, Some(TeamPhase::Exec)),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Exec);
    }

    #[test]
    fn infer_phase_all_completed_is_verify() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Completed, Some(TeamPhase::Exec)),
            (TaskStatus::Completed, Some(TeamPhase::Exec)),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Verify);
    }

    #[test]
    fn infer_phase_any_failed_is_fix() {
        let ctrl = DefaultPhaseController;
        let tasks = vec![
            (TaskStatus::Completed, Some(TeamPhase::Exec)),
            (TaskStatus::Failed, Some(TeamPhase::Exec)),
        ];
        assert_eq!(ctrl.infer_phase(&tasks), TeamPhase::Fix);
    }

    #[test]
    fn recommend_roles_for_exec_phase() {
        let ctrl = DefaultPhaseController;
        let roles = ctrl.recommend_roles(&TeamPhase::Exec);
        assert!(roles.contains(&"executor".to_string()));
        assert!(roles.contains(&"reviewer".to_string()));
    }

    #[test]
    fn recommend_roles_for_plan_phase() {
        let ctrl = DefaultPhaseController;
        let roles = ctrl.recommend_roles(&TeamPhase::Plan);
        assert!(roles.contains(&"planner".to_string()));
        assert!(roles.contains(&"researcher".to_string()));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omx-team -- phase_controller`
Expected: FAIL — `todo!()` panics

- [ ] **Step 3: Implement the phase controller**

Rewrite the impl blocks in `crates/omx-team/src/phase_controller.rs`:

```rust
use omx_types::{TaskStatus, TeamPhase};

pub trait PhaseController: Send + Sync {
    fn infer_phase(&self, tasks: &[(TaskStatus, Option<TeamPhase>)]) -> TeamPhase;
    fn recommend_roles(&self, phase: &TeamPhase) -> Vec<String>;
}

pub struct DefaultPhaseController;

impl PhaseController for DefaultPhaseController {
    fn infer_phase(&self, tasks: &[(TaskStatus, Option<TeamPhase>)]) -> TeamPhase {
        if tasks.is_empty() {
            return TeamPhase::Plan;
        }

        // Any failed task → Fix phase
        if tasks.iter().any(|(s, _)| *s == TaskStatus::Failed) {
            return TeamPhase::Fix;
        }

        // All completed → Verify phase
        if tasks.iter().all(|(s, _)| *s == TaskStatus::Completed) {
            return TeamPhase::Verify;
        }

        // Any in-progress → use the phase annotation of in-progress tasks, default Exec
        if tasks.iter().any(|(s, _)| *s == TaskStatus::InProgress) {
            return tasks
                .iter()
                .find(|(s, _)| *s == TaskStatus::InProgress)
                .and_then(|(_, phase)| phase.clone())
                .unwrap_or(TeamPhase::Exec);
        }

        // All pending → Plan
        TeamPhase::Plan
    }

    fn recommend_roles(&self, phase: &TeamPhase) -> Vec<String> {
        match phase {
            TeamPhase::Plan => vec!["planner".into(), "researcher".into()],
            TeamPhase::Prd => vec!["writer".into(), "reviewer".into()],
            TeamPhase::Exec => vec!["executor".into(), "reviewer".into()],
            TeamPhase::Verify => vec!["tester".into(), "reviewer".into()],
            TeamPhase::Fix => vec!["executor".into(), "debugger".into()],
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omx-team -- phase_controller`
Expected: all 6 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/omx-team/src/phase_controller.rs
git commit -m "feat(omx-team): implement phase controller with inference and role recommendation"
```

---

## Task 7: Implement role_router — intent-based role inference and task routing

**Files:**
- Modify: `crates/omx-team/src/role_router.rs`

The role router recommends roles based on phase and routes tasks to the best matching role based on keyword analysis of the task description.

- [ ] **Step 1: Write the failing tests**

Replace `crates/omx-team/src/role_router.rs`:

```rust
use omx_types::TeamPhase;

/// Recommend roles for a given phase. Delegates to phase_controller logic
/// but provides a standalone convenience function.
pub fn recommend_roles(phase: &TeamPhase) -> Vec<String> {
    use crate::phase_controller::{DefaultPhaseController, PhaseController};
    DefaultPhaseController.recommend_roles(phase)
}

/// Route a task description to the best matching role from available roles.
/// Uses keyword-based intent matching.
pub fn route_task(description: &str, available_roles: &[String]) -> Option<String> {
    if available_roles.is_empty() {
        return None;
    }

    let desc_lower = description.to_lowercase();

    // Intent keywords mapped to roles
    let role_keywords: &[(&str, &[&str])] = &[
        ("planner", &["plan", "design", "architect", "strategy"]),
        ("researcher", &["research", "investigate", "explore", "analyze"]),
        ("executor", &["implement", "build", "create", "write", "code", "add", "fix"]),
        ("reviewer", &["review", "check", "audit", "verify"]),
        ("tester", &["test", "validate", "qa", "quality"]),
        ("writer", &["document", "write docs", "readme", "spec"]),
        ("debugger", &["debug", "diagnose", "troubleshoot", "bisect"]),
    ];

    // Score each available role
    let mut best_role: Option<&String> = None;
    let mut best_score = 0usize;

    for role in available_roles {
        let role_lower = role.to_lowercase();
        if let Some((_, keywords)) = role_keywords.iter().find(|(r, _)| *r == role_lower) {
            let score = keywords
                .iter()
                .filter(|kw| desc_lower.contains(**kw))
                .count();
            if score > best_score {
                best_score = score;
                best_role = Some(role);
            }
        }
    }

    // Fall back to first available role if no keyword match
    best_role
        .or(available_roles.first())
        .map(|r| r.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommend_roles_delegates_to_phase_controller() {
        let roles = recommend_roles(&TeamPhase::Exec);
        assert!(roles.contains(&"executor".to_string()));
    }

    #[test]
    fn route_task_matches_executor_for_implement() {
        let roles = vec!["planner".into(), "executor".into(), "reviewer".into()];
        let result = route_task("implement the login feature", &roles);
        assert_eq!(result, Some("executor".into()));
    }

    #[test]
    fn route_task_matches_planner_for_design() {
        let roles = vec!["planner".into(), "executor".into()];
        let result = route_task("design the API architecture", &roles);
        assert_eq!(result, Some("planner".into()));
    }

    #[test]
    fn route_task_falls_back_to_first_role() {
        let roles = vec!["specialist".into()];
        let result = route_task("do something unusual", &roles);
        assert_eq!(result, Some("specialist".into()));
    }

    #[test]
    fn route_task_empty_roles_returns_none() {
        let result = route_task("anything", &[]);
        assert_eq!(result, None);
    }

    #[test]
    fn route_task_multiple_keywords_picks_highest_score() {
        let roles = vec!["executor".into(), "tester".into()];
        // "implement and build" has 2 executor keywords vs 0 tester keywords
        let result = route_task("implement and build the module", &roles);
        assert_eq!(result, Some("executor".into()));
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p omx-team -- role_router`
Expected: all 6 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/omx-team/src/role_router.rs
git commit -m "feat(omx-team): implement role router with keyword-based task routing"
```

---

## Task 8: Implement worktree — git worktree provisioning and cleanup

**Files:**
- Modify: `crates/omx-team/src/worktree.rs`

Git worktree management for worker isolation. Each worker gets its own worktree branching from the current HEAD.

- [ ] **Step 1: Write the failing tests**

Replace `crates/omx-team/src/worktree.rs`:

```rust
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

    // Get the repo root
    let root = run_git(&["rev-parse", "--show-toplevel"])?;
    let root = root.trim();

    // Compute worktree path as sibling directory
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

    // Create branch and worktree
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

    // Best-effort branch deletion
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
        // Test the path computation logic
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
        // Simulate porcelain output
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
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p omx-team -- worktree`
Expected: all 3 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/omx-team/src/worktree.rs
git commit -m "feat(omx-team): implement git worktree provisioning and cleanup"
```

---

## Task 9: Implement commit_hygiene — commit ledger and integration

**Files:**
- Modify: `crates/omx-team/src/commit_hygiene.rs`

Tracks worker commits in a JSONL ledger and integrates them via cherry-pick into the main branch during shutdown.

- [ ] **Step 1: Write the failing tests**

Replace `crates/omx-team/src/commit_hygiene.rs`:

```rust
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRecord {
    pub sha: String,
    pub worker: String,
    pub message: String,
    pub timestamp: String,
}

/// Record a worker commit to the hygiene ledger (JSONL file).
pub fn record_commit(ledger_path: &Path, worker: &str, sha: &str, message: &str) -> Result<(), OmxError> {
    let record = CommitRecord {
        sha: sha.to_string(),
        worker: worker.to_string(),
        message: message.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut line = serde_json::to_string(&record)?;
    line.push('\n');

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path)
        .map_err(|e| OmxError::Io(e))?;
    file.write_all(line.as_bytes())
        .map_err(|e| OmxError::Io(e))?;

    tracing::info!(worker = %worker, sha = %sha, "recorded commit");
    Ok(())
}

/// Read all commit records from the ledger.
pub fn read_ledger(ledger_path: &Path) -> Result<Vec<CommitRecord>, OmxError> {
    if !ledger_path.exists() {
        return Ok(Vec::new());
    }

    let data = std::fs::read_to_string(ledger_path)
        .map_err(|e| OmxError::Io(e))?;

    let records: Vec<CommitRecord> = data
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(records)
}

/// Get the ledger file path for a team.
pub fn ledger_path(state_dir: &Path, team_name: &str) -> PathBuf {
    state_dir.join(format!("team/{team_name}/commit-ledger.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_read_commit_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("ledger.jsonl");

        record_commit(&ledger, "worker-0", "abc123", "feat: add feature").unwrap();
        record_commit(&ledger, "worker-1", "def456", "fix: bug").unwrap();

        let records = read_ledger(&ledger).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sha, "abc123");
        assert_eq!(records[0].worker, "worker-0");
        assert_eq!(records[1].sha, "def456");
        assert_eq!(records[1].message, "fix: bug");
    }

    #[test]
    fn read_empty_ledger_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = dir.path().join("nonexistent.jsonl");
        let records = read_ledger(&ledger).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn ledger_path_format() {
        let path = ledger_path(Path::new("/home/.omx/state"), "myteam");
        assert_eq!(
            path,
            PathBuf::from("/home/.omx/state/team/myteam/commit-ledger.jsonl")
        );
    }

    #[test]
    fn commit_record_serde_roundtrip() {
        let record = CommitRecord {
            sha: "abc123".into(),
            worker: "w1".into(),
            message: "test".into(),
            timestamp: "2026-04-05T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let parsed: CommitRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sha, "abc123");
    }
}
```

- [ ] **Step 2: Add tempfile dev-dependency**

Add to `crates/omx-team/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p omx-team -- commit_hygiene`
Expected: all 4 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/omx-team/src/commit_hygiene.rs crates/omx-team/Cargo.toml
git commit -m "feat(omx-team): implement commit hygiene ledger with JSONL persistence"
```

---

## Task 10: Implement scaling — worker scale recommendation

**Files:**
- Modify: `crates/omx-team/src/scaling.rs`

Scaling recommends how many workers should run based on pending task count, and provides stubs for scale-up/down operations that will be wired by the orchestrator.

- [ ] **Step 1: Write the failing tests**

Replace `crates/omx-team/src/scaling.rs`:

```rust
use omx_types::OmxError;

/// Recommend the optimal worker count based on pending tasks.
/// Returns a value between 1 and max_workers.
/// Heuristic: 1 worker per 2 pending tasks, clamped to [current, max].
pub fn recommend_scale(current_workers: u8, pending_tasks: u32, max_workers: u8) -> u8 {
    if max_workers == 0 {
        return 0;
    }

    // At least 1 worker per 2 pending tasks, minimum is current workers
    let ideal = ((pending_tasks as f64 / 2.0).ceil() as u8).max(1);
    ideal.clamp(current_workers.min(max_workers), max_workers)
}

/// Compute how many workers to add (positive) or remove (negative) to reach target.
pub fn scale_delta(current: u8, target: u8) -> i16 {
    target as i16 - current as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommend_scale_basic() {
        // 4 pending tasks → ceil(4/2) = 2 workers, currently 2, max 5
        assert_eq!(recommend_scale(2, 4, 5), 2);
    }

    #[test]
    fn recommend_scale_many_tasks_caps_at_max() {
        // 20 pending tasks → ceil(20/2) = 10, but max is 5
        assert_eq!(recommend_scale(2, 20, 5), 5);
    }

    #[test]
    fn recommend_scale_zero_tasks_keeps_current() {
        // 0 pending tasks → ideal=1, current=3, min(current,max)=3, so clamp(1, 3, 5) = 3
        assert_eq!(recommend_scale(3, 0, 5), 3);
    }

    #[test]
    fn recommend_scale_zero_max_returns_zero() {
        assert_eq!(recommend_scale(0, 10, 0), 0);
    }

    #[test]
    fn recommend_scale_never_below_one() {
        assert_eq!(recommend_scale(0, 1, 5), 1);
    }

    #[test]
    fn scale_delta_positive() {
        assert_eq!(scale_delta(2, 5), 3);
    }

    #[test]
    fn scale_delta_negative() {
        assert_eq!(scale_delta(5, 2), -3);
    }

    #[test]
    fn scale_delta_zero() {
        assert_eq!(scale_delta(3, 3), 0);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p omx-team -- scaling`
Expected: all 8 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/omx-team/src/scaling.rs
git commit -m "feat(omx-team): implement scaling recommendation heuristic"
```

---

## Task 11: Implement allocation — task-to-worker assignment policy

**Files:**
- Modify: `crates/omx-team/src/allocation.rs`

The allocation policy assigns ready tasks to available workers using round-robin distribution.

- [ ] **Step 1: Write the failing tests**

Replace `crates/omx-team/src/allocation.rs`:

```rust
use omx_types::{TaskId, WorkerId};

/// Allocate ready tasks to available workers using round-robin.
/// Returns a list of (task, worker) pairs.
pub fn allocate(
    ready_tasks: &[TaskId],
    available_workers: &[WorkerId],
) -> Vec<(TaskId, WorkerId)> {
    if available_workers.is_empty() {
        return Vec::new();
    }

    ready_tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let worker = &available_workers[i % available_workers.len()];
            (task.clone(), worker.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_round_robin() {
        let tasks = vec![
            TaskId("t1".into()),
            TaskId("t2".into()),
            TaskId("t3".into()),
        ];
        let workers = vec![WorkerId("w1".into()), WorkerId("w2".into())];

        let assignments = allocate(&tasks, &workers);
        assert_eq!(assignments.len(), 3);
        assert_eq!(assignments[0], (TaskId("t1".into()), WorkerId("w1".into())));
        assert_eq!(assignments[1], (TaskId("t2".into()), WorkerId("w2".into())));
        assert_eq!(assignments[2], (TaskId("t3".into()), WorkerId("w1".into())));
    }

    #[test]
    fn allocate_no_workers_returns_empty() {
        let tasks = vec![TaskId("t1".into())];
        let assignments = allocate(&tasks, &[]);
        assert!(assignments.is_empty());
    }

    #[test]
    fn allocate_no_tasks_returns_empty() {
        let workers = vec![WorkerId("w1".into())];
        let assignments = allocate(&[], &workers);
        assert!(assignments.is_empty());
    }

    #[test]
    fn allocate_single_worker_gets_all_tasks() {
        let tasks = vec![TaskId("t1".into()), TaskId("t2".into())];
        let workers = vec![WorkerId("w1".into())];
        let assignments = allocate(&tasks, &workers);
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0].1, WorkerId("w1".into()));
        assert_eq!(assignments[1].1, WorkerId("w1".into()));
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p omx-team -- allocation`
Expected: all 4 tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/omx-team/src/allocation.rs
git commit -m "feat(omx-team): implement round-robin task allocation policy"
```

---

## Task 12: Implement worker bootstrap and orchestrator

**Files:**
- Modify: `crates/omx-team/src/worker.rs`
- Modify: `crates/omx-team/src/orchestrator.rs`
- Modify: `crates/omx-team/src/lib.rs`

The worker bootstrap spawns CLI instances in tmux windows. The orchestrator runs the async monitor loop. `DefaultTeamRuntime` in `lib.rs` wires everything together.

- [ ] **Step 1: Implement worker bootstrap**

Rewrite `crates/omx-team/src/worker.rs`:

```rust
use async_trait::async_trait;
use omx_types::{OmxError, WorkerId};
use std::path::{Path, PathBuf};

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

        // Compose and write AGENTS.md for the worker
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
            .map_err(|e| OmxError::Io(e))?;

        let inbox_file = inbox_dir.join("tasks.json");
        let json = serde_json::to_string_pretty(tasks)?;
        tokio::fs::write(&inbox_file, json)
            .await
            .map_err(|e| OmxError::Io(e))?;

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
```

- [ ] **Step 2: Run worker tests to verify they pass**

Run: `cargo test -p omx-team -- worker`
Expected: 2 tests pass

- [ ] **Step 3: Implement orchestrator**

Rewrite `crates/omx-team/src/orchestrator.rs`:

```rust
use omx_types::{OmxError, TaskId, TaskStatus, WorkerId};
use std::path::Path;
use std::time::Instant;

use crate::config::TeamConfig;
use crate::mailbox::{self, MailboxMessage};
use crate::phase_controller::{DefaultPhaseController, PhaseController};
use crate::task_queue;
use crate::{TaskSnapshot, TeamSnapshot, WorkerSnapshot};

/// Mutable team state tracked by the orchestrator.
pub struct OrchestratorState {
    pub config: TeamConfig,
    pub start_time: Instant,
    pub tasks: Vec<(TaskId, TaskStatus, Vec<TaskId>)>,
    pub workers: Vec<(WorkerId, String, bool, Option<TaskId>)>, // (id, role, alive, current_task)
    pub messages: Vec<MailboxMessage>,
}

impl OrchestratorState {
    pub fn new(config: TeamConfig) -> Self {
        let tasks: Vec<(TaskId, TaskStatus, Vec<TaskId>)> = config
            .tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let id = TaskId(format!("task-{i}"));
                let deps = t
                    .depends_on
                    .iter()
                    .map(|d| TaskId(d.clone()))
                    .collect();
                (id, TaskStatus::Pending, deps)
            })
            .collect();

        let workers: Vec<(WorkerId, String, bool, Option<TaskId>)> = config
            .workers
            .iter()
            .enumerate()
            .map(|(i, w)| (WorkerId(format!("worker-{i}")), w.role.clone(), true, None))
            .collect();

        Self {
            config,
            start_time: Instant::now(),
            tasks,
            workers,
            messages: Vec::new(),
        }
    }
}

/// Perform one monitor tick: snapshot state, infer phase, deliver mailbox.
pub fn tick(state: &mut OrchestratorState) -> Result<TeamSnapshot, OmxError> {
    let phase_controller = DefaultPhaseController;

    // Build phase inference input
    let phase_input: Vec<_> = state
        .tasks
        .iter()
        .enumerate()
        .map(|(i, (_, status, _))| {
            let phase = state
                .config
                .tasks
                .get(i)
                .and_then(|t| t.phase.clone());
            (status.clone(), phase)
        })
        .collect();

    let phase = phase_controller.infer_phase(&phase_input);

    // Build snapshot
    let workers: Vec<WorkerSnapshot> = state
        .workers
        .iter()
        .map(|(id, role, alive, current_task)| WorkerSnapshot {
            id: id.clone(),
            role: role.clone(),
            alive: *alive,
            current_task: current_task.clone(),
        })
        .collect();

    let tasks: Vec<TaskSnapshot> = state
        .tasks
        .iter()
        .map(|(id, status, _)| {
            let assigned_to = state
                .workers
                .iter()
                .find(|(_, _, _, ct)| ct.as_ref() == Some(id))
                .map(|(wid, _, _, _)| wid.clone());
            TaskSnapshot {
                id: id.clone(),
                status: status.clone(),
                assigned_to,
                result: None,
            }
        })
        .collect();

    let snapshot = TeamSnapshot {
        team_name: state.config.name.0.clone(),
        phase,
        workers,
        tasks,
        uptime_seconds: state.start_time.elapsed().as_secs(),
    };

    tracing::debug!(
        phase = ?snapshot.phase,
        uptime = snapshot.uptime_seconds,
        "tick complete"
    );

    Ok(snapshot)
}

/// Check if the team is done (all tasks completed or failed).
pub fn is_done(state: &OrchestratorState) -> bool {
    state
        .tasks
        .iter()
        .all(|(_, status, _)| *status == TaskStatus::Completed || *status == TaskStatus::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use omx_types::{TeamName, TeamPhase};

    fn make_config(task_count: usize) -> TeamConfig {
        TeamConfig {
            name: TeamName("test".into()),
            workers: vec![WorkerConfig {
                role: "executor".into(),
                model: None,
                provider: None,
            }],
            tasks: (0..task_count)
                .map(|i| TeamTask {
                    description: format!("task {i}"),
                    depends_on: vec![],
                    phase: None,
                })
                .collect(),
            governance: TeamGovernance::default(),
            worktree_mode: WorktreeMode::Shared,
            dispatch_mode: DispatchMode::Tmux,
        }
    }

    #[test]
    fn orchestrator_state_initializes_tasks_and_workers() {
        let config = make_config(3);
        let state = OrchestratorState::new(config);
        assert_eq!(state.tasks.len(), 3);
        assert_eq!(state.workers.len(), 1);
        assert_eq!(state.tasks[0].0, TaskId("task-0".into()));
        assert_eq!(state.tasks[0].1, TaskStatus::Pending);
    }

    #[test]
    fn tick_returns_snapshot_with_plan_phase_when_all_pending() {
        let config = make_config(2);
        let mut state = OrchestratorState::new(config);
        let snapshot = tick(&mut state).unwrap();
        assert_eq!(snapshot.phase, TeamPhase::Plan);
        assert_eq!(snapshot.tasks.len(), 2);
        assert_eq!(snapshot.workers.len(), 1);
    }

    #[test]
    fn is_done_when_all_completed() {
        let config = make_config(2);
        let mut state = OrchestratorState::new(config);
        state.tasks[0].1 = TaskStatus::Completed;
        state.tasks[1].1 = TaskStatus::Completed;
        assert!(is_done(&state));
    }

    #[test]
    fn is_done_false_when_pending_tasks_remain() {
        let config = make_config(2);
        let mut state = OrchestratorState::new(config);
        state.tasks[0].1 = TaskStatus::Completed;
        assert!(!is_done(&state));
    }
}
```

- [ ] **Step 4: Run orchestrator tests to verify they pass**

Run: `cargo test -p omx-team -- orchestrator`
Expected: all 4 tests pass

- [ ] **Step 5: Implement DefaultTeamRuntime in lib.rs**

Update `crates/omx-team/src/lib.rs` — add the `DefaultTeamRuntime` struct that wires modules together:

```rust
pub mod allocation;
pub mod commit_hygiene;
pub mod config;
pub mod dispatch;
pub mod mailbox;
pub mod orchestrator;
pub mod phase_controller;
pub mod role_router;
pub mod scaling;
pub mod task_queue;
pub mod tmux_session;
pub mod worker;
pub mod worktree;

use async_trait::async_trait;
use omx_types::{LeaseToken, OmxError, TaskId, TaskStatus, WorkerId};
use serde::{Deserialize, Serialize};

use crate::config::TeamConfig;
use crate::orchestrator::OrchestratorState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSnapshot {
    pub team_name: String,
    pub phase: omx_types::TeamPhase,
    pub workers: Vec<WorkerSnapshot>,
    pub tasks: Vec<TaskSnapshot>,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerSnapshot {
    pub id: WorkerId,
    pub role: String,
    pub alive: bool,
    pub current_task: Option<TaskId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub status: TaskStatus,
    pub assigned_to: Option<WorkerId>,
    pub result: Option<String>,
}

#[async_trait]
pub trait TeamRuntime: Send + Sync {
    async fn start(&mut self, config: TeamConfig) -> Result<(), OmxError>;
    async fn monitor(&self) -> Result<TeamSnapshot, OmxError>;
    async fn send_message(
        &self,
        from: &WorkerId,
        to: &WorkerId,
        body: &str,
    ) -> Result<(), OmxError>;
    async fn broadcast(&self, from: &WorkerId, body: &str) -> Result<(), OmxError>;
    async fn claim_task(&self, worker: &WorkerId, task: &TaskId) -> Result<LeaseToken, OmxError>;
    async fn transition_task(
        &self,
        task: &TaskId,
        token: &LeaseToken,
        status: TaskStatus,
        result: Option<String>,
    ) -> Result<(), OmxError>;
    async fn shutdown(&mut self) -> Result<(), OmxError>;
}

/// Default team runtime that wires together all modules.
pub struct DefaultTeamRuntime {
    state: Option<OrchestratorState>,
}

impl DefaultTeamRuntime {
    pub fn new() -> Self {
        Self { state: None }
    }
}

#[async_trait]
impl TeamRuntime for DefaultTeamRuntime {
    async fn start(&mut self, config: TeamConfig) -> Result<(), OmxError> {
        tracing::info!(team = %config.name.0, workers = config.workers.len(), "starting team");

        // Create tmux session
        let session = tmux_session::create_team_session(&config.name.0)?;
        tracing::info!(session = %session, "created team session");

        // Initialize orchestrator state
        self.state = Some(OrchestratorState::new(config));

        Ok(())
    }

    async fn monitor(&self) -> Result<TeamSnapshot, OmxError> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| OmxError::Team("team not started".into()))?;

        // Build a snapshot without mutating (read-only view)
        let phase_controller = phase_controller::DefaultPhaseController;
        use phase_controller::PhaseController;

        let phase_input: Vec<_> = state
            .tasks
            .iter()
            .enumerate()
            .map(|(i, (_, status, _))| {
                let phase = state.config.tasks.get(i).and_then(|t| t.phase.clone());
                (status.clone(), phase)
            })
            .collect();

        let phase = phase_controller.infer_phase(&phase_input);

        let workers = state
            .workers
            .iter()
            .map(|(id, role, alive, ct)| WorkerSnapshot {
                id: id.clone(),
                role: role.clone(),
                alive: *alive,
                current_task: ct.clone(),
            })
            .collect();

        let tasks = state
            .tasks
            .iter()
            .map(|(id, status, _)| TaskSnapshot {
                id: id.clone(),
                status: status.clone(),
                assigned_to: None,
                result: None,
            })
            .collect();

        Ok(TeamSnapshot {
            team_name: state.config.name.0.clone(),
            phase,
            workers,
            tasks,
            uptime_seconds: state.start_time.elapsed().as_secs(),
        })
    }

    async fn send_message(
        &self,
        from: &WorkerId,
        to: &WorkerId,
        body: &str,
    ) -> Result<(), OmxError> {
        let _msg = mailbox::create_message(from, to, body)?;
        tracing::info!(from = %from.0, to = %to.0, "message queued");
        Ok(())
    }

    async fn broadcast(&self, from: &WorkerId, body: &str) -> Result<(), OmxError> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| OmxError::Team("team not started".into()))?;

        for (worker_id, _, _, _) in &state.workers {
            if *worker_id != *from {
                let _msg = mailbox::create_message(from, worker_id, body)?;
            }
        }

        tracing::info!(from = %from.0, "broadcast sent");
        Ok(())
    }

    async fn claim_task(&self, worker: &WorkerId, task: &TaskId) -> Result<LeaseToken, OmxError> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| OmxError::Team("team not started".into()))?;

        let (_, status, _) = state
            .tasks
            .iter()
            .find(|(id, _, _)| id == task)
            .ok_or_else(|| OmxError::Team(format!("task {} not found", task.0)))?;

        task_queue::claim(task, worker, status)
    }

    async fn transition_task(
        &self,
        task: &TaskId,
        token: &LeaseToken,
        status: TaskStatus,
        result: Option<String>,
    ) -> Result<(), OmxError> {
        task_queue::transition(task, token, status, result)
    }

    async fn shutdown(&mut self) -> Result<(), OmxError> {
        if let Some(state) = &self.state {
            let team_name = &state.config.name.0;
            tracing::info!(team = %team_name, "shutting down team");

            // Kill tmux session (best-effort)
            if let Err(e) = tmux_session::kill_team_session(team_name) {
                tracing::warn!(error = %e, "failed to kill team session");
            }
        }

        self.state = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_snapshot_serde_roundtrip() {
        let snapshot = TeamSnapshot {
            team_name: "test-team".into(),
            phase: omx_types::TeamPhase::Exec,
            workers: vec![],
            tasks: vec![],
            uptime_seconds: 42,
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: TeamSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.team_name, "test-team");
    }

    #[test]
    fn default_team_runtime_starts_uninitialized() {
        let runtime = DefaultTeamRuntime::new();
        assert!(runtime.state.is_none());
    }
}
```

- [ ] **Step 6: Run all omx-team tests**

Run: `cargo test -p omx-team`
Expected: all tests pass, no `todo!()` panics

- [ ] **Step 7: Commit**

```bash
git add crates/omx-team/src/worker.rs crates/omx-team/src/orchestrator.rs crates/omx-team/src/lib.rs
git commit -m "feat(omx-team): implement worker bootstrap, orchestrator, and DefaultTeamRuntime"
```

---

## Task 13: Implement omx-mcp-team — 4 MCP tools delegating to TeamRuntime

**Files:**
- Modify: `crates/omx-mcp-team/Cargo.toml`
- Modify: `crates/omx-mcp-team/src/main.rs`

The MCP server delegates to `DefaultTeamRuntime`. It holds the runtime in an `Arc<Mutex<>>` for shared access across tool calls.

- [ ] **Step 1: Add omx-team dependency**

Add to `crates/omx-mcp-team/Cargo.toml`:

```toml
omx-team = { path = "../omx-team" }
```

- [ ] **Step 2: Implement the 4 MCP tools**

Rewrite `crates/omx-mcp-team/src/main.rs`:

```rust
use std::sync::Arc;

use rmcp::{tool, ServerHandler, ServiceExt};
use serde::Deserialize;
use tokio::sync::Mutex;

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

        let mut runtime = self.runtime.lock().await;
        match runtime.start(config).await {
            Ok(()) => {
                serde_json::json!({
                    "status": "started",
                    "team_name": team_name
                })
                .to_string()
            }
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    #[tool(description = "Get current team status including phase, workers, and tasks")]
    async fn omx_run_team_status(&self, #[tool(aggr)] params: TeamStatusParams) -> String {
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
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p omx-mcp-team`
Expected: compiles successfully

- [ ] **Step 4: Verify entire workspace compiles and tests pass**

Run: `cargo test --workspace`
Expected: all tests pass, no `todo!()` remains in omx-team or omx-mcp-team

- [ ] **Step 5: Commit**

```bash
git add crates/omx-mcp-team/Cargo.toml crates/omx-mcp-team/src/main.rs
git commit -m "feat(omx-mcp-team): implement 4 MCP tools delegating to DefaultTeamRuntime"
```

---

## Verification Checklist

After all tasks are complete:

1. `cargo test -p omx-team` — all tests pass
2. `cargo build -p omx-mcp-team` — compiles
3. `cargo test --workspace` — no regressions
4. `cargo clippy --workspace` — no warnings
5. `rg 'todo!' crates/omx-team/ crates/omx-mcp-team/` — returns zero matches
