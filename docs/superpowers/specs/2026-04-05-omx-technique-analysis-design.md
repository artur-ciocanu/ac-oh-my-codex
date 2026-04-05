# OMX Technique Analysis: A Deep Reference for oh-my-codex

**Date:** 2026-04-05
**Scope:** Comprehensive analysis of all 32 agent prompts, 36+ skills, 20 Rust crates, and 14 missions in the oh-my-codex repository
**Purpose:** (A) Reference taxonomy, (B) Teaching/sharing guide, (C) Extension handbook

---

## Part 1: The Philosophy

Five design principles underpin every technique in OMX. Understanding these is prerequisite to understanding the techniques themselves.

### P1. Evidence Over Plausibility

LLMs produce plausible-sounding but wrong answers. OMX treats plausibility as a bug, not a feature. Every agent prompt requires `file:line` citations and fresh tool-backed verification output before any claim is accepted. The executor's lore commit protocol encodes this into git history via structured trailers (`Constraint`, `Rejected`, `Confidence`, `Scope-risk`). The critic's simulation-based validation requires tracing execution paths through actual code rather than reasoning about what code "probably does."

**Why it works:** It converts the LLM's weakness (confident hallucination) into a structural impossibility — you can't cite a file:line that doesn't exist when the tool actually reads the file.

### P2. Explore First, Ask Last

Traditional assistant behavior: encounter ambiguity → ask the user. OMX inverts this. Every agent with an `ask_gate` section must exhaust codebase exploration before asking a single clarifying question. The explore agent's dual-layer harness (explore.md + explore-harness.md) provides graceful degradation — if MCP tools aren't available, fall back to CLI; if CLI fails, fall back to basic file reads.

**Why it works:** It respects the user's time by treating the codebase as the primary source of truth. Most "ambiguities" dissolve when you actually read the code.

### P3. Constraint Layering

Every agent prompt follows a 7-section nested structure:
1. **Identity** — who the agent is (name, archetype, one-sentence mandate)
2. **Scope Guard** — what the agent is NOT allowed to do
3. **Ask Gate** — when (and only when) the agent may ask the user
4. **Execution Loop** — the step-by-step work process
5. **Verification Loop** — how to confirm the work is correct
6. **Anti-Patterns** — explicit "never do this" examples
7. **Final Checklist** — gate before delivering output

**Why it works:** Each layer constrains the one below it. Identity sets direction, scope guard prevents drift, ask gate prevents laziness, execution loop ensures thoroughness, verification catches mistakes, anti-patterns catch subtle errors, and the final checklist is the last defense. An agent would need to violate multiple layers simultaneously to produce bad output.

### P4. Behavioral Postures

Agents aren't monolithic. Each has three orthogonal dimensions:
- **Role** — what it does (executor, critic, architect, etc.)
- **Posture** — how it approaches work (cautious, balanced, aggressive)
- **Model Class** — which LLM tier it uses (reasoning, balanced, speed)

These compose into ~90 behavioral configurations from 30 base roles. A cautious executor on a reasoning model behaves very differently from an aggressive executor on a speed model — same role, different risk tolerance and capability profile.

**Why it works:** It avoids the combinatorial explosion of writing 90 separate prompts while still providing fine-grained behavioral control.

### P5. Composable Primitives

Complex workflows are built from simple, well-tested building blocks:
- **Ultrawork** — parallel task execution with tier-based routing
- **Ralph** — persistent verification loop with deslop enforcement
- **Autopilot** — full lifecycle orchestration (6 phases)

These compose: Autopilot uses Ralph for verification, which uses Ultrawork for parallel checks. Each primitive has a single, clear contract. You can use Ultrawork without Ralph, Ralph without Autopilot, or the full stack.

**Why it works:** Same reason Unix pipes work — small tools with clear interfaces compose better than monolithic systems.

---

## Part 2: The Technique Catalog

17 techniques organized into 4 families. Each entry documents: the pattern, the problem it solves, why it's effective, canonical example, how to replicate, and extension opportunities.

### Family A: Prompt Architecture

#### A1. Seven-Section Constraint Skeleton

**Pattern:** Every agent prompt uses nested XML-tagged sections: `<identity>`, `<scope_guard>`, `<ask_gate>`, `<execution_loop>`, `<verification_loop>`, `<anti_patterns>`, `<final_checklist>`.

**Problem:** LLMs drift from instructions over long contexts. A single "be careful" instruction gets lost.

**Why effective:** Redundant constraints at multiple levels. The agent would need to violate identity AND scope AND anti-patterns simultaneously to produce disallowed output. Each section reinforces the others from a different angle.

**Canonical example:** `prompts/executor.md` — the deep-worker prompt with all 7 sections fully populated, including the lore commit protocol in the execution loop and the deslop pass in the verification loop.

**How to replicate:**
1. Start with identity: one name, one archetype, one mandate sentence
2. Add scope guard: 3-5 explicit "never" statements
3. Add ask gate: conditions that must ALL be true before asking the user
4. Write execution loop: numbered steps with tool usage requirements
5. Write verification loop: what to check and how
6. List anti-patterns: 3-5 "bad examples" with explanations of why they're bad
7. Final checklist: 5-8 yes/no gates

**Extension opportunity:** The skeleton is domain-agnostic. Any agent role — not just coding agents — can use this structure.

#### A2. Vague-Request Gating

**Pattern:** Pattern matching for concrete signals (file paths, error messages, symbols, issue numbers) before accepting a task. If no concrete signals are found, the agent asks for specifics instead of guessing.

**Problem:** Users give vague instructions ("fix the auth") that lead to wasted work when the agent guesses wrong.

**Why effective:** It's a binary gate, not a judgment call. Either the request contains `src/auth/middleware.ts:42` or it doesn't. No ambiguity in whether to proceed.

**Canonical example:** `AGENTS.md` keyword detection — 47 keywords with priority routing. The `hooks/keyword-detector.ts` scans for concrete signals before dispatching to any agent.

**How to replicate:**
1. Define what "concrete" means for your domain (file paths, error codes, ticket numbers)
2. Scan the request for these signals
3. If absent, ask one specific question to obtain them
4. Never proceed on vibes alone

**Extension opportunity:** Domain-specific signal detectors (e.g., for database migration requests, require table names and migration direction).

#### A3. Pre-Context Intake Snapshots

**Pattern:** Before any expensive operation, write a `.omx/context/{slug}-{timestamp}.md` file capturing: what was asked, what's known, what's unknown, and what the plan is.

**Problem:** Long-running operations lose track of their original intent. Context window rotation makes this worse.

**Why effective:** It creates an external memory that survives context compaction. The agent can re-read its own intake snapshot to stay on track.

**Canonical example:** The ralph persistence system — `src/ralph/persistence.ts` maintains a progress ledger that tracks iteration history, current phase, and accumulated evidence.

**How to replicate:**
1. Before starting work, write a structured snapshot file
2. Include: original request, known constraints, unknowns, planned approach
3. Re-read the snapshot at the start of each work phase
4. Update it as unknowns become known

**Extension opportunity:** Automated drift detection — compare current work against the intake snapshot and flag divergence.

#### A4. Behavioral Posture Composition

**Pattern:** Three orthogonal dimensions (role x posture x model class) composed via overlay system in `src/agents/native-config.ts`.

**Problem:** Writing separate prompts for every behavioral variant doesn't scale. 30 roles x 3 postures x 3 model classes = 270 variants.

**Why effective:** Each dimension is independent. Posture overlays modify risk tolerance without touching role logic. Model class overlays adjust tool usage patterns without touching posture logic.

**Canonical example:** `src/agents/definitions.ts` — 30+ `AgentDefinition` entries, each specifying role, default posture, and default model class. `native-config.ts` composes these into final TOML configs.

**How to replicate:**
1. Define your role dimension: what the agent does
2. Define your posture dimension: how cautiously it operates (2-4 levels)
3. Define your model dimension: which capability tier to use
4. Write overlays for each posture and model class level
5. Compose: base prompt + posture overlay + model overlay = final config

**Extension opportunity:** Additional dimensions (e.g., verbosity level, collaboration style, domain expertise depth).

#### A5. Declarative Agent Catalog

**Pattern:** All agent definitions in a single `catalog-manifest.json` with schema validation via `src/catalog/schema.ts`.

**Problem:** Agent discovery and routing requires knowing what agents exist, what they do, and how to invoke them.

**Why effective:** Single source of truth. The keyword detector, the role router, the team orchestrator, and the CLI all read from the same catalog. Adding an agent means adding one entry.

**Canonical example:** `templates/catalog-manifest.json` — 39 skills + 30 agents, each with name, description, category, and routing metadata.

**How to replicate:**
1. Define a manifest schema (JSON Schema or TypeScript interface)
2. Register all agents/skills with their metadata
3. Build all routing and discovery on top of the manifest
4. Validate the manifest at startup

**Extension opportunity:** Version-aware catalogs for managing agent evolution across releases.

### Family B: Verification & Quality

#### B1. Circuit Breaker Escalation

**Pattern:** Hard stop after N failures of the same type. Debugger: 3 hypotheses. UltraQA: 3x same failure. Ralph: 10 iterations.

**Problem:** LLMs can loop forever, trying the same broken approach repeatedly.

**Why effective:** It's a mechanical stop, not a judgment call. The agent doesn't decide "I've tried enough" — the counter decides. This removes the LLM's tendency toward optimistic "one more try" reasoning.

**Canonical example:** `prompts/debugger.md` — after 3 failed hypotheses, the debugger must escalate to the user with all evidence gathered so far, rather than generating hypothesis #4.

**How to replicate:**
1. Identify your loop (retry, hypothesis, iteration)
2. Pick a failure threshold (3-10 depending on cost per iteration)
3. Track failures with a simple counter
4. On threshold: stop, summarize what was tried, escalate

**Extension opportunity:** Adaptive thresholds based on iteration cost (expensive operations get lower thresholds).

#### B2. Tiered Verification Scaling

**Pattern:** Verification depth scales with change size. Small changes (< 50 lines): 3 checks. Standard (50-200 lines): 5 checks. Large (200+ lines): 8 checks.

**Problem:** Over-verifying small changes wastes time. Under-verifying large changes misses bugs.

**Why effective:** It matches effort to risk. A one-line typo fix doesn't need 8 verification checks. A 500-line refactor does.

**Canonical example:** `AGENTS.md` verification protocols — explicit tier definitions with check counts and what each check covers.

**How to replicate:**
1. Define size tiers (2-4 levels based on line count or file count)
2. Define checks per tier (ascending)
3. Classify changes automatically at verification time
4. Run the appropriate number of checks

**Extension opportunity:** Risk-weighted tiers (security-sensitive files get higher tier regardless of size).

#### B3. Deslop Enforcement

**Pattern:** Mandatory AI cleanup pass after verification succeeds. The agent reviews its own output for AI-isms (unnecessary comments, over-engineering, verbose explanations in code) and removes them. Then re-verifies to confirm the cleanup didn't break anything.

**Problem:** LLMs add "slop" — unnecessary docstrings, redundant comments, over-abstracted helpers, chatty variable names.

**Why effective:** It uses the LLM's own pattern recognition against its own bad habits. The LLM is good at detecting AI-written slop when explicitly told to look for it.

**Canonical example:** `prompts/executor.md` verification loop — after all checks pass, a dedicated deslop phase scans for and removes AI artifacts, followed by regression verification.

**How to replicate:**
1. After your main verification passes, add a "cleanup" phase
2. Define what "slop" means for your domain (unnecessary comments, over-abstraction, etc.)
3. Have the agent review and clean its own output
4. Re-run verification to catch regressions from cleanup

**Extension opportunity:** Project-specific slop definitions (some teams want docstrings, others don't).

#### B4. Lore Commit Protocol

**Pattern:** Git commits carry structured trailers encoding decision context: `Constraint` (what limited options), `Rejected` (what was considered and discarded), `Directive` (what instruction drove the choice), `Confidence` (high/medium/low), `Scope-risk` (what could break).

**Problem:** Commit messages say WHAT changed but not WHY, or what alternatives were considered.

**Why effective:** It creates a machine-readable decision log in the permanent git history. Future agents (or humans) can understand not just what was done but why, and what was explicitly rejected.

**Canonical example:** `prompts/executor.md` execution loop — every commit must include at least `Constraint` and `Confidence` trailers.

**How to replicate:**
1. Define 3-5 trailer keys relevant to your domain
2. Require them in your commit template
3. Populate them from the decision context of each change
4. Use them in code review and debugging

**Extension opportunity:** Trailer-based analytics — track confidence distribution over time, identify high-risk-scope commits for extra review.

#### B5. Evidence-Ranked Hypotheses

**Pattern:** When debugging or analyzing, generate multiple hypotheses but rank them by available evidence, not plausibility. Pursue the highest-evidence hypothesis first.

**Problem:** LLMs generate the most "reasonable-sounding" hypothesis first, which is often wrong. Plausibility != probability.

**Why effective:** It forces tool usage before ranking. You can't rank by evidence without gathering evidence first, which means the agent reads code and runs tests before committing to an explanation.

**Canonical example:** `skills/analyze/SKILL.md` — hypotheses must include supporting evidence (file:line citations, test output, log entries) and are ranked by evidence strength, not narrative plausibility.

**How to replicate:**
1. Generate 3-5 hypotheses without ranking
2. For each, gather supporting evidence using tools
3. Rank by evidence count and quality
4. Pursue top-ranked first, with circuit breaker at 3 failures

**Extension opportunity:** Evidence types with weights (test failure > log entry > code pattern).

### Family C: Orchestration & Coordination

#### C1. Claim-Safe Task Lifecycle

**Pattern:** Workers claim tasks with UUID lease tokens that expire after 15 minutes. Version-number optimistic locking prevents concurrent modifications. If a lease expires, another worker can claim the task.

**Problem:** Multiple agents working on the same task simultaneously, or a crashed agent holding a task hostage forever.

**Why effective:** It's the same pattern distributed databases use (lease-based locks with expiration). Battle-tested in distributed systems, applied to multi-agent coordination.

**Canonical example:** `src/team/state/tasks.ts` — `claimTask()` writes a lease token with timestamp. `releaseTask()` clears it. `isLeaseExpired()` checks the 15-minute window.

**How to replicate:**
1. Generate a UUID lease token on claim
2. Write it atomically with a timestamp
3. Check lease expiration before honoring the claim
4. Use version numbers to prevent lost updates

**Extension opportunity:** Adaptive lease duration based on task complexity.

#### C2. State-First Message Dispatch

**Pattern:** All team communication goes through persistent state files (`.omx/state/`). tmux `send-keys` is a fallback notification mechanism only — the state file is the source of truth.

**Problem:** Terminal-based message passing is lossy (scrollback limits, missed messages, timing issues).

**Why effective:** State files are durable, inspectable, and don't depend on terminal state. A worker that crashes and restarts can read the state files to catch up, rather than needing message replay.

**Canonical example:** `src/team/state/dispatch.ts` — request queue with deduplication. `src/team/state/mailbox.ts` — per-worker message files.

**How to replicate:**
1. Define your message types as structured data
2. Write messages to per-recipient state files (atomic write)
3. Use terminal/UI notifications only as "you have mail" signals
4. Recipients read state files to get actual content

**Extension opportunity:** Event sourcing — append messages to a log and derive current state from replay.

#### C3. Phase-Based Progression

**Pattern:** Team work progresses through ordered phases: Plan → PRD → Exec → Verify → Fix. Phase transitions are inferred from task completion counts rather than explicitly commanded.

**Problem:** Multi-agent workflows need coordination on "what phase are we in" without a central controller bottleneck.

**Why effective:** Auto-inference from task state means no single point of failure. If the orchestrator crashes, workers can still determine the current phase by reading the task state.

**Canonical example:** `crates/omx-types/src/lib.rs` — `TeamPhase` enum with `ordinal()` and `next()` methods. `crates/omx-team/` — `PhaseController` managing transitions.

**How to replicate:**
1. Define your phases as an ordered enum
2. Define transition conditions (e.g., "all exec tasks complete → verify phase")
3. Evaluate conditions from persistent state, not in-memory flags
4. Allow workers to query current phase independently

**Extension opportunity:** Conditional phase skipping (skip verify if changes are below risk threshold).

#### C4. Dynamic Scaling

**Pattern:** The team system can add workers mid-execution based on workload. `src/team/scaling.ts` monitors queue depth and spawns additional workers when tasks accumulate.

**Problem:** Fixed worker counts either waste resources (too many for small tasks) or create bottlenecks (too few for large tasks).

**Why effective:** It adapts to actual workload rather than predicted workload. The tmux-based execution model makes spawning a new worker cheap (just open a new pane).

**Canonical example:** `src/team/scaling.ts` — monitors pending task count vs. active worker count. `crates/omx-team/` — `Scaling` module with dynamic worker allocation.

**How to replicate:**
1. Monitor your work queue depth
2. Define a ratio (e.g., max 3 pending tasks per worker)
3. When ratio is exceeded, spawn a new worker
4. Cap total workers (OMX uses max 6 concurrent child agents)

**Extension opportunity:** Scale-down logic — idle workers self-terminate after a timeout.

#### C5. Allocation & Rebalance Policies

**Pattern:** Scoring-based task assignment (`src/team/allocation-policy.ts`) matches tasks to workers based on role fit, current load, and expertise. Rebalance policy (`src/team/rebalance-policy.ts`) reassigns tasks from busy workers to idle ones.

**Problem:** Random task assignment leads to suboptimal work distribution — a security review task assigned to a frontend-specialist worker.

**Why effective:** It's a lightweight optimization that avoids the complexity of a full scheduler while still making intelligent assignments.

**Canonical example:** `src/team/allocation-policy.ts` — scoring function considers worker role, current task count, and task category. Best-scoring worker gets the task.

**How to replicate:**
1. Define scoring dimensions (role fit, load, recency)
2. Score each available worker for each pending task
3. Assign highest-scoring worker
4. Periodically rebalance if load becomes uneven

**Extension opportunity:** Learning-based scoring — track which worker types produce highest-quality output for which task types.

#### C6. File-Based Distributed Coordination

**Pattern:** All coordination state uses atomic file operations: `tempfile + rename` for writes, `fs2` exclusive locks for critical sections, JSONL append for logs, `mkdir` for lock acquisition.

**Problem:** Multi-agent systems need coordination primitives, but database servers add complexity and failure modes.

**Why effective:** Files are the simplest possible coordination mechanism. They work on every OS, survive process crashes (atomic rename is crash-safe), and are trivially inspectable (`cat` the state file to debug).

**Canonical example:** `crates/omx-state/src/lib.rs` — `FileStateStore` with `write_atomic()` using tempfile + rename. `src/team/state/locks.ts` — file-based mutex.

**How to replicate:**
1. Use `tempfile` in the target directory + `rename` for atomic writes
2. Use `fs2` file locks (or `mkdir` for cross-process locks)
3. Use JSONL append for event logs (append is atomic under OS page size)
4. Never read-modify-write without holding a lock

**Extension opportunity:** Distributed coordination across machines via NFS or shared volumes.

### Family D: System Architecture

#### D1. Dual-Layer Architecture

**Pattern:** TypeScript for orchestration, prompts, and skills. Rust for state persistence, event-sourced runtime, tmux multiplexing, and sandboxed exploration.

**Problem:** TypeScript is productive for prompt engineering and workflow logic but lacks the performance and safety guarantees needed for concurrent state management.

**Why effective:** Each language does what it's best at. TypeScript's flexibility suits the rapidly-evolving prompt layer. Rust's ownership model prevents data races in the concurrent state layer. The boundary is clean: TypeScript calls Rust binaries via JSON-based IPC (`src/runtime/bridge.ts`).

**Canonical example:** `src/runtime/bridge.ts` — `RuntimeBridge` wraps the `omx-runtime` Rust binary, sending commands and receiving JSON state responses.

**How to replicate:**
1. Identify your "hot path" — concurrent state management, I/O-heavy operations
2. Implement the hot path in a systems language with safety guarantees
3. Keep orchestration and business logic in a productive high-level language
4. Connect them via a simple IPC protocol (JSON over stdio, HTTP, etc.)

**Extension opportunity:** WASM compilation of Rust crates for browser-based tooling.

#### D2. Event-Sourced Runtime

**Pattern:** `crates/omx-runtime-core/` implements a state machine that processes `RuntimeCommand`s and emits `RuntimeEvent`s. State is derived from replaying the event log. Supports snapshot + compact for performance.

**Problem:** Traditional CRUD state management loses history. You can't answer "how did we get here?" or replay from a checkpoint.

**Why effective:** Full audit trail of every state change. Time-travel debugging is trivial — replay events up to any point. Crash recovery is automatic — replay from last snapshot.

**Canonical example:** `crates/omx-runtime-core/` — `RuntimeEngine` with `process_command()` → `RuntimeEvent` → `apply_event()` → updated state. `ReplayState` for reconstruction from event log.

**How to replicate:**
1. Define your commands (inputs) and events (state changes)
2. Process commands into events (the "decide" function)
3. Apply events to state (the "evolve" function)
4. Persist events to an append-only log
5. Add snapshot + compact for performance at scale

**Extension opportunity:** Event streaming for real-time monitoring dashboards.

#### D3. Security-Through-Allowlisting (omx-explore)

**Pattern:** The sandboxed exploration environment (`crates/omx-explore/`) restricts agents to exactly 10 allowed commands via self-wrapping binaries. Path canonicalization prevents traversal. Command arguments are validated against allowlists.

**Problem:** Agents with shell access can execute arbitrary commands, creating security and stability risks.

**Why effective:** Allowlisting is strictly more secure than blocklisting. Instead of trying to anticipate every dangerous command, only permit the known-safe ones. The self-wrapping binary approach means the sandbox is enforced at the process level, not by prompt instructions.

**Canonical example:** `crates/omx-explore/src/main.rs` (1238 lines) — `ALLOWED_COMMANDS` set, `validate_command()` function, `wrap_binary()` for creating sandboxed command wrappers.

**How to replicate:**
1. Define your allowed command set (keep it minimal)
2. Create wrapper binaries that validate commands before execution
3. Canonicalize all paths to prevent traversal
4. Validate all arguments against allowlists
5. Log all command executions for audit

**Extension opportunity:** Per-agent allowlists — the debugger might need different commands than the executor.

#### D4. MCP Server Fleet

**Pattern:** 5 specialized Rust MCP servers (state, memory, code-intel, trace, team) using `rmcp` framework + stdio transport. Each server exposes domain-specific tools via `#[tool]` attributes.

**Problem:** Agents need structured access to system capabilities without shell-level access to everything.

**Why effective:** MCP provides a typed, discoverable interface. Agents call tools by name with structured parameters, rather than constructing shell commands. The stdio transport is simple and secure — no network exposure.

**Canonical example:** `crates/omx-mcp-state/` — exposes `read_state`, `write_state`, `list_states` tools. `crates/omx-mcp-code-intel/` — wraps `tsc` and `rg` behind structured tool interfaces.

**How to replicate:**
1. Identify capabilities that agents need (state access, code search, etc.)
2. Group into domain-specific servers
3. Implement tool handlers with the `rmcp` `#[tool]` attribute
4. Connect via stdio transport (simplest, most secure)

**Extension opportunity:** New MCP servers for CI/CD, issue trackers, cloud resources, observability.

#### D5. Cascading Configuration

**Pattern:** `crates/omx-config/` implements layered config resolution: CLI flags > environment variables > `.omx-config.json` > `config.toml` > built-in defaults.

**Problem:** Configuration needs to work across different contexts (CI, local dev, team settings) with predictable override behavior.

**Why effective:** The cascade is intuitive — more specific settings override more general ones. The `ConfigLoader` trait makes the resolution chain extensible.

**Canonical example:** `crates/omx-config/src/lib.rs` — `ConfigLoader` trait with `load()` method that walks the cascade.

**How to replicate:**
1. Define your config sources in priority order
2. Implement a loader for each source
3. Merge from lowest to highest priority
4. Validate the final merged config

**Extension opportunity:** Per-directory config overrides for monorepo setups.

---

## Part 3: The Composition Map

Techniques don't exist in isolation. This section shows 5 composition patterns — how techniques combine to create emergent behavior greater than the sum of parts.

### Composition 1: The Execution Stack

```
Ultrawork (C4: parallel dispatch)
  └─ Ralph (B1: circuit breaker + B3: deslop)
       └─ Autopilot (A1: constraint skeleton per phase)
            └─ Individual agents (A4: posture composition)
```

**How it works:** Autopilot manages 6 lifecycle phases. Within each phase, Ralph handles persistence and verification looping. Within each verification cycle, Ultrawork parallelizes checks across multiple agents. Each agent runs with its own constraint skeleton and posture overlay.

**Emergent property:** Self-healing execution — a failed verification triggers Ralph's retry loop, which re-dispatches via Ultrawork, which routes to the appropriate agent via posture composition. No single component understands the full flow, but the composition handles failures end-to-end.

### Composition 2: The Planning Gate

```
Ralplan (A1 + B5: evidence-ranked options)
  ├─ Planner agent (A2: vague-request gating)
  ├─ Architect agent (review cycle)
  └─ Critic agent (simulation-based validation)
       └─ On consensus → Execution Stack (above)
```

**How it works:** Before any execution begins, Ralplan runs a multi-iteration consensus loop. The planner drafts, the architect reviews for feasibility, the critic reviews for correctness. Each uses vague-request gating to demand specifics. Only when all three agree does execution begin.

**Emergent property:** Execution never starts on a bad plan. The consensus requirement means all three perspectives (feasibility, correctness, completeness) are satisfied before any code is written.

### Composition 3: The Team Coordination System

```
Team Orchestrator (C3: phase progression)
  ├─ Task Queue (C1: claim-safe lifecycle)
  ├─ State Files (C6: atomic coordination + C2: state-first dispatch)
  ├─ Allocation Policy (C5: scoring-based assignment)
  ├─ Scaling (C4: dynamic worker addition)
  └─ Workers (A1: constraint skeleton + A4: posture per role)
```

**How it works:** The orchestrator manages phase transitions inferred from task state. Workers claim tasks via lease tokens, communicate via state files, and receive assignments via the scoring-based allocation policy. If the queue grows too deep, scaling adds workers dynamically.

**Emergent property:** Fault-tolerant parallel execution. Any worker can crash without affecting others. Leaked leases expire automatically. The orchestrator doesn't need to track worker health — it reads task state.

### Composition 4: The Pipeline Sequencer

```
Pipeline Orchestrator (sequential stages + artifact accumulation)
  ├─ Stage: ralplan (Composition 2)
  ├─ Stage: team-exec (Composition 3)
  ├─ Stage: ralph-verify (Composition 1)
  └─ Artifacts flow forward: plan → execution results → verification report
```

**How it works:** The pipeline orchestrator runs stages sequentially. Each stage's output becomes the next stage's input via artifact accumulation. The pipeline supports resume — if interrupted, it restarts from the last incomplete stage.

**Emergent property:** End-to-end traceability. The verification report references execution results, which reference the plan. Any finding can be traced back to the planning decision that led to it.

### Composition 5: The Prompt-to-Execution Contract

```
Agent Catalog (A5: declarative manifest)
  → Keyword Detector (A2: concrete signal matching)
    → Role Router (A4: posture selection)
      → Constraint Skeleton (A1: 7-section structure)
        → Execution with Verification (B1-B5)
          → Lore Commits (B4: decision context in git)
```

**How it works:** A user request hits the keyword detector, which matches concrete signals to route to the appropriate agent from the catalog. The role router selects the posture and model class. The constraint skeleton structures the agent's behavior. Verification techniques ensure output quality. Lore commits preserve the decision trail.

**Emergent property:** Deterministic agent selection and behavior from free-form user input. The same request always routes to the same agent type with the same behavioral constraints, regardless of which LLM instance handles it.

---

## Part 4: The Extension Guide

Six structural extension points the architecture already implies.

### E1. New Agent Roles via Constraint Layering

**What exists:** 32 agent prompts following the 7-section pattern.

**How to extend:** Copy the constraint skeleton from an existing prompt, fill in role-specific sections, add an `AgentDefinition` entry in `definitions.ts`, and register in `catalog-manifest.json`. The layering pattern is orthogonal to domain knowledge.

**Opportunity areas:** Migration engineer, dependency auditor, performance profiler, accessibility specialist.

### E2. New Skills via the SKILL.md Contract

**What exists:** 36+ skills, each a self-contained SKILL.md with trigger conditions, process steps, and completion criteria.

**How to extend:** Create `skills/<name>/SKILL.md`, add keyword triggers to `keyword-registry.ts`, register in `catalog-manifest.json`.

**Opportunity areas:** Domain-specific workflows (database migration, API versioning), cross-cutting concerns (accessibility, i18n), project-specific ceremonies.

### E3. New MCP Servers for External Integration

**What exists:** 5 Rust MCP servers using `rmcp` + stdio transport.

**How to extend:** New crate in `crates/omx-mcp-<domain>/`, implement tool handlers with `#[tool]` attributes, register in workspace `Cargo.toml`.

**Opportunity areas:** CI/CD integration, issue tracker queries, observability, cloud resource inspection.

### E4. New Composition Patterns

**What exists:** 5 composition patterns (Execution Stack, Planning Gate, Team Coordination, Pipeline Sequencer, Prompt-to-Execution Contract).

**How to extend:** The pipeline orchestrator's `PipelineStage` interface allows inserting new stages into any sequence. The team system's role router incorporates new agent types without changing orchestration logic.

**Concrete example:** A "review pipeline" could compose: ralplan → team-exec (parallel reviewers) → consensus stage → ralph-verify.

### E5. New Verification Strategies

**What exists:** Tiered verification, UltraQA cycling, deslop enforcement.

**How to extend:** Implement the verification contract (changes in → pass/fail with evidence out). Property-based testing, snapshot comparison, performance regression detection, visual regression via screenshot diff.

### E6. New Runtime Backends Beyond tmux

**What exists:** `MuxAdapter` trait with `TmuxAdapter` implementation.

**How to extend:** Alternative backends (Docker containers, SSH sessions, cloud sandboxes) implement `MuxAdapter`. The `MuxOperation` enum defines the interface.

---

## Appendix: Cross-Reference Matrix

| Technique | Used By (Agents) | Used By (Skills) | Used By (Crates) |
|-----------|------------------|-------------------|-------------------|
| A1. Constraint Skeleton | All 32 prompts | — | — |
| A2. Vague-Request Gating | executor, planner, architect | plan, deep-interview | omx-hooks (keyword-detector) |
| A3. Pre-Context Snapshots | executor, team-orchestrator | ralph, autopilot | omx-state |
| A4. Posture Composition | All via definitions.ts | — | omx-config |
| A5. Declarative Catalog | — | help, catalog | — |
| B1. Circuit Breaker | debugger, build-fixer | ultraqa, ralph | omx-runtime-core |
| B2. Tiered Verification | verifier | ultraqa | — |
| B3. Deslop Enforcement | executor | ralph, autopilot | — |
| B4. Lore Commits | executor, git-master | — | — |
| B5. Evidence-Ranked | debugger, critic | analyze, deepsearch | — |
| C1. Claim-Safe Tasks | team-executor | team, worker | omx-team |
| C2. State-First Dispatch | team-orchestrator | team, swarm | omx-state, omx-team |
| C3. Phase Progression | team-orchestrator, verifier | team | omx-team, omx-types |
| C4. Dynamic Scaling | team-orchestrator | team, ultrawork | omx-team |
| C5. Allocation Policy | team-orchestrator | team | omx-team |
| C6. File Coordination | All team agents | team, ralph | omx-state |
| D1. Dual-Layer Arch | — | — | All crates + src/ |
| D2. Event-Sourced Runtime | — | — | omx-runtime-core |
| D3. Allowlist Sandbox | explore | — | omx-explore |
| D4. MCP Fleet | All (via tool access) | — | omx-mcp-* |
| D5. Cascading Config | — | — | omx-config |

---

## Part 5: OMX as Reinvented Knowledge Engineering

A comparison with CommonKADS and Problem-Solving Methods (PSMs) reveals that OMX has independently reinvented — and in several cases surpassed — classical Knowledge Engineering patterns that took the academic community 15 years to formalize. The structural parallels are not superficial; they go to the core of how knowledge, tasks, and inference are modeled.

### The Constraint Skeleton IS a Problem-Solving Method

The single most striking finding: OMX's 7-section constraint skeleton (`identity` → `scope_guard` → `ask_gate` → `execution_loop` → `verification_loop` → `anti_patterns` → `final_checklist`) is structurally a PSM expressed in natural language rather than formal notation.

In classical PSM terms:
- **`identity`** = the PSM's **task type declaration** — what class of problem this method solves (diagnosis, design, classification)
- **`scope_guard`** = **assumptions and competence boundaries** — what the PSM requires to be true about the domain and what it explicitly cannot handle (Fensel's "bridge assumptions")
- **`ask_gate`** = **knowledge role specification** — what inputs the method needs and under what conditions it must acquire them (CommonKADS inference layer's input roles)
- **`execution_loop`** = **inference structure** — the ordered sequence of primitive inference steps operating over knowledge roles (the core of any PSM)
- **`verification_loop`** = **meta-inference** — reasoning about the quality of the primary inference output (no direct classical analog — see "Where OMX Goes Beyond" below)
- **`anti_patterns`** = **negative examples / constraint violations** — what the PSM must NOT produce (related to Fensel's "assumptions" but more operationally concrete)
- **`final_checklist`** = **output role validation** — confirming the output satisfies the task specification before delivery

This isn't a loose analogy. The constraint skeleton defines reusable inference patterns that are domain-independent (the same skeleton works for executor, critic, architect, debugger), separates task knowledge from domain knowledge (the skeleton structure vs. the role-specific content), and uses knowledge roles (inputs, outputs, intermediate artifacts) — the three defining characteristics of a PSM.

### Direct Structural Mappings

| CommonKADS / PSM Concept | OMX Equivalent | How OMX Differs |
|---|---|---|
| **Organization Model** (actors, roles, functions) | `AGENTS.md` + `catalog-manifest.json` (agent definitions, routing roles, categories) | OMX adds **dynamic scaling** — CommonKADS assumes static agent assignment |
| **Task Model** (hierarchical decomposition) | Pipeline stages + skill SKILL.md contracts (ralplan → team-exec → ralph-verify) | OMX tasks are **resumable** — pipeline supports crash recovery and resume from last stage |
| **Agent Model** (who can do what) | `AgentDefinition` interface (name, posture, modelClass, routingRole, tools) | OMX adds the **posture × model class** composition — 3 orthogonal dimensions vs. KADS' flat capability list |
| **Knowledge Model — Domain layer** | System prompts + `<identity>` sections (domain knowledge embedded in role prompts) | OMX embeds domain knowledge in natural language rather than formal ontologies — fits the LLM paradigm |
| **Knowledge Model — Inference layer** | Constraint skeleton sections `<execution_loop>` + `<verification_loop>` (the reasoning steps) | OMX inference steps are **tool-backed** — every inference must produce tool output, not just logical conclusions |
| **Knowledge Model — Task layer** | Skill control flow (e.g., ralph's 7-phase state machine, autopilot's 6 phases) | OMX adds **circuit breakers** — classical PSMs have no built-in loop termination |
| **Communication Model** | State-first dispatch (`.omx/state/` files + tmux fallback) | OMX's communication is **asynchronous and durable** — classical KADS assumed synchronous dialogue |
| **Design Model** | Dual-layer architecture (TS orchestration + Rust persistence) | OMX explicitly separates "fast to change" (prompts) from "must be correct" (state management) |
| **MAS-CommonKADS Coordination Model** | Team orchestration (phase progression, claim-safe tasks, allocation policy) | OMX adds **lease-based fault tolerance** — expired leases auto-release, no stuck tasks |

### PSM Pattern Correspondences

| Classical PSM | OMX Instantiation | Key Innovation |
|---|---|---|
| **Heuristic Classification** (abstract → match → refine) | Keyword detector → role router → posture selection. Raw user input is abstracted to keywords, matched to agent roles, refined to specific posture+model config | OMX makes the abstraction step **pattern-based** (47 keywords with priority) rather than requiring a domain ontology |
| **Propose-Critique-Modify** | Ralplan consensus loop: Planner proposes → Architect critiques → Critic validates → iterate until consensus (max 5 rounds) | OMX adds a **third role** (Critic does simulation-based validation, not just review) and a **hard iteration cap** |
| **Systematic Diagnosis / Cover-and-Differentiate** | Debugger prompt: generate hypotheses → gather evidence → rank by evidence strength → pursue top-ranked → circuit breaker at 3 failures | OMX adds **evidence-ranking over plausibility-ranking** — classical cover-and-differentiate doesn't specify a ranking criterion |
| **Skeletal Plan Refinement** | Plan skill's 4-mode auto-detection: selects a plan template (interview/direct/consensus/review) then refines based on task signals | OMX makes template **selection automatic** via signal detection rather than engineer choice |
| **Chandrasekaran's Generic Tasks** | Agent catalog with 30+ role definitions, each a composable building block | OMX roles are **prompt-defined** (changeable without recompilation) vs. Chandrasekaran's code-defined generic tasks |

### Where OMX Goes Beyond Classical KE

**Anti-Hallucination as a First-Class Concern.** Classical KE never had to deal with its inference engine *making things up*. Symbolic systems either derive a conclusion from rules or don't. OMX's "Evidence Over Plausibility" principle (technique B5), deslop enforcement (B3), and mandatory `file:line` citations have no classical analog because the problem didn't exist. This is OMX's most genuinely novel contribution — structural defenses against a failure mode that classical KE couldn't anticipate.

**Circuit Breakers.** Classical PSMs describe iteration (propose-critique-modify loops, hypothesis refinement cycles) but never specify termination conditions for degenerate cases. What happens when cover-and-differentiate generates hypotheses that all fail? The classical answer is: the engineer notices. OMX's answer is: the system stops at N failures and escalates. This is borrowed from distributed systems engineering (Hystrix, resilience4j), not from KE.

**Self-Correction (Deslop).** Classical KE assumes the knowledge engineer produces clean output. OMX assumes the LLM produces noisy output and builds in a mandatory cleanup pass. The deslop enforcement technique — review your own output for AI-isms, then re-verify — is a meta-cognitive pattern that classical systems never needed because symbolic systems don't add unnecessary comments to their conclusions.

**Dynamic Composition at Runtime.** CommonKADS agent models are designed at specification time. OMX's posture × model class composition happens at dispatch time — the same agent role can be instantiated with different behavioral parameters based on runtime conditions. This is closer to runtime polymorphism than to KADS' static modeling.

**Infrastructure-Free Coordination.** MAS-CommonKADS assumed FIPA-compliant agent communication infrastructure (ACL messages, directory facilitators, interaction protocols). OMX achieves equivalent coordination with atomic file writes and lease tokens — zero infrastructure dependencies. This makes the system dramatically simpler to deploy and debug.

### Where Classical KE Has Advantages OMX Lacks

**Formal Verification.** CommonKADS knowledge models can be formally verified for completeness and consistency. OMX's natural-language prompts cannot — you can't prove that a constraint skeleton prevents all failure modes. Classical KE's formal rigor is genuinely valuable for safety-critical systems.

**Systematic Knowledge Reuse.** Classical PSMs have the formal **knowledge role** abstraction that makes cross-domain reuse systematic — the same heuristic classification PSM works for medical diagnosis, mineral identification, and help-desk troubleshooting. OMX's techniques are reusable in principle but embed domain assumptions that require manual extraction to port.

**The Knowledge Level.** Allen Newell's "knowledge level" — describing agent behavior in terms of goals and knowledge independently of implementation — is missing from OMX. OMX jumps from high-level philosophy (Part 1) to implementation patterns (Part 2) without a principled intermediate layer that says: "this agent needs to *know* X and be able to *infer* Y, regardless of how we implement it."

**Principled Knowledge Acquisition.** CommonKADS has a structured methodology for knowledge elicitation (protocol analysis, repertory grids, card sorting, structured interviews). OMX's "Explore First, Ask Last" is effective for code-domain tasks but doesn't generalize to domains where knowledge lives in experts' heads.

### The Meta-Insight

The LLM agent community is reinventing classical KE, but with crucial adaptations the KE community never anticipated. The Allen et al. (2023) TGDK paper and the MAKE 2026 AAAI symposium (April 7-9, 2026) are starting to bridge this gap academically, but OMX is — perhaps unknowingly — one of the most complete practical instantiations of this bridge.

This reveals a two-way opportunity:
1. **OMX → KE:** OMX could be formalized using CommonKADS vocabulary, making its patterns more discoverable, teachable, and comparable to other systems
2. **KE → OMX:** CommonKADS could be updated with OMX's LLM-specific innovations (evidence requirements, circuit breakers, deslop) to become relevant to modern agent engineering

### Key References (2024-2026)

- Allen, Stork & Groth — *Knowledge Engineering using LLMs* (TGDK, 2023)
- Liu et al. — *Cognitive Models as Templates for Language Agents* (arXiv, Feb 2026)
- Wray, Kirk & Laird — *Cognitive Design Patterns for LLM Agents* (AGI-25, May 2025)
- Cai et al. — *Design Patterns for LLM-based MAS* (arXiv, 2025)
- EMAS 2025 — *Towards Engineering LLM-Enhanced Multi-Agent Systems*
- MAKE 2026 — AAAI Spring Symposium on ML + KE for Semantic Agents (April 7-9, 2026)
- Tiddi et al. — *HI-CommonKADS* (KCAP 2023, HHAI 2024)
- van Harmelen — *Knowledge Engineering Rediscovered* (KCAP, 2009)
- Clancey — *Heuristic Classification* (Artificial Intelligence, 1985)
- Fensel, Benjamins, Studer — *Knowledge Engineering: Principles and Methods* (DKE, 1998)
- Chandrasekaran — *Generic Tasks as Building Blocks* (IEEE Expert, 1986)
- Iglesias et al. — *MAS-CommonKADS* (IWMAS, 1997)
