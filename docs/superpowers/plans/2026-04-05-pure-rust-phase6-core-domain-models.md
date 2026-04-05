# Phase 6: Core Domain Models — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the four domain-model crates (omx-agents, omx-modes, omx-session, omx-catalog) that every later phase depends on.

**Architecture:** Four new library crates in the foundation/domain layer. Each crate owns its types, persistence, and validation. All integrate with existing omx-types for error handling and omx-state for persistence. No async in omx-agents or omx-modes (pure data + validation); omx-session and omx-catalog use async for I/O.

**Tech Stack:** Rust 2021, serde, serde_json, toml, thiserror, tokio, chrono, ulid (new dep), omx-types, omx-config, omx-state

---

## File Structure

### New crate: `crates/omx-agents/`
```
crates/omx-agents/
├── Cargo.toml
└── src/
    └── lib.rs          # AgentDef, Posture, ModelClass, RoutingRole, AgentRegistry
```

### New crate: `crates/omx-modes/`
```
crates/omx-modes/
├── Cargo.toml
└── src/
    └── lib.rs          # Mode enum, ModeState, ModeConfig, ModeManager, conflict matrix
```

### New crate: `crates/omx-session/`
```
crates/omx-session/
├── Cargo.toml
└── src/
    ├── lib.rs          # Session, SessionStatus, SessionStore trait, re-exports
    ├── store.rs        # FileSessionStore implementation
    └── search.rs       # Full-text search over transcripts
```

### New crate: `crates/omx-catalog/`
```
crates/omx-catalog/
├── Cargo.toml
└── src/
    └── lib.rs          # SkillDef, CatalogRegistry, discovery, validation
```

### Modified files:
- `Cargo.toml` (workspace root) — add 4 new members + `ulid` workspace dep
- `crates/omx-types/src/lib.rs` — add new OmxError variants (Mode, Session, Agent, Catalog)

---

## Task 1: Add workspace scaffolding and new error variants

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `crates/omx-types/src/lib.rs`

- [ ] **Step 1: Add error variant tests to omx-types**

Add these tests at the end of the existing `mod tests` block in `crates/omx-types/src/lib.rs`:

```rust
    #[test]
    fn mode_error_display() {
        let err = OmxError::Mode("conflict".into());
        assert_eq!(err.to_string(), "mode error: conflict");
    }

    #[test]
    fn session_error_display() {
        let err = OmxError::Session("not found".into());
        assert_eq!(err.to_string(), "session error: not found");
    }

    #[test]
    fn agent_error_display() {
        let err = OmxError::Agent("duplicate name".into());
        assert_eq!(err.to_string(), "agent error: duplicate name");
    }

    #[test]
    fn catalog_error_display() {
        let err = OmxError::Catalog("missing field".into());
        assert_eq!(err.to_string(), "catalog error: missing field");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omx-types`
Expected: FAIL — `OmxError::Mode`, `OmxError::Session`, `OmxError::Agent`, `OmxError::Catalog` do not exist

- [ ] **Step 3: Add the error variants**

In `crates/omx-types/src/lib.rs`, add these variants to the `OmxError` enum, after the existing `Json` variant:

```rust
    #[error("mode error: {0}")]
    Mode(String),

    #[error("session error: {0}")]
    Session(String),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("catalog error: {0}")]
    Catalog(String),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omx-types`
Expected: ALL PASS

- [ ] **Step 5: Add workspace deps and members**

In the root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
ulid = "1"
```

Add to the `members` list (after the existing notification hooks):

```toml
    "crates/omx-agents",
    "crates/omx-modes",
    "crates/omx-session",
    "crates/omx-catalog",
```

- [ ] **Step 6: Create the 4 crate directories**

Run:
```bash
mkdir -p crates/omx-agents/src crates/omx-modes/src crates/omx-session/src crates/omx-catalog/src
```

- [ ] **Step 7: Create minimal Cargo.toml for each crate**

Create `crates/omx-agents/Cargo.toml`:
```toml
[package]
name = "omx-agents"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-config = { path = "../omx-config" }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

Create `crates/omx-modes/Cargo.toml`:
```toml
[package]
name = "omx-modes"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-config = { path = "../omx-config" }
omx-state = { path = "../omx-state" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true }
```

Create `crates/omx-session/Cargo.toml`:
```toml
[package]
name = "omx-session"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-state = { path = "../omx-state" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
chrono = { workspace = true }
ulid = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

Create `crates/omx-catalog/Cargo.toml`:
```toml
[package]
name = "omx-catalog"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
omx-types = { path = "../omx-types" }
omx-config = { path = "../omx-config" }
omx-agents = { path = "../omx-agents" }
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 8: Create stub lib.rs files so workspace compiles**

Create `crates/omx-agents/src/lib.rs`:
```rust
//! Agent type definitions and registry.
```

Create `crates/omx-modes/src/lib.rs`:
```rust
//! Mode lifecycle management.
```

Create `crates/omx-session/src/lib.rs`:
```rust
//! Session tracking and history.
```

Create `crates/omx-catalog/src/lib.rs`:
```rust
//! Skill and agent catalog with discovery.
```

- [ ] **Step 9: Verify workspace compiles**

Run: `cargo check`
Expected: SUCCESS with no errors

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml crates/omx-types/src/lib.rs crates/omx-agents/ crates/omx-modes/ crates/omx-session/ crates/omx-catalog/
git commit -m "feat: scaffold Phase 6 crates and add OmxError variants for mode/session/agent/catalog"
```

---

## Task 2: Implement omx-agents — types and registry

**Files:**
- Create: `crates/omx-agents/src/lib.rs`

- [ ] **Step 1: Write failing tests for agent types and registry**

Replace the contents of `crates/omx-agents/src/lib.rs` with:

```rust
//! Agent type definitions and registry.

use serde::{Deserialize, Serialize};
use omx_types::OmxError;
use std::collections::HashMap;
use std::path::Path;

// Types and AgentRegistry will be implemented in Step 3

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posture_serde_roundtrip() {
        let p = Posture::Autonomous;
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Posture = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, p);
    }

    #[test]
    fn model_class_display() {
        assert_eq!(ModelClass::Heavy.to_string(), "heavy");
        assert_eq!(ModelClass::Medium.to_string(), "medium");
        assert_eq!(ModelClass::Light.to_string(), "light");
    }

    #[test]
    fn routing_role_serde_roundtrip() {
        let r = RoutingRole::Planner;
        let json = serde_json::to_string(&r).unwrap();
        let parsed: RoutingRole = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, r);
    }

    #[test]
    fn agent_def_serde_roundtrip() {
        let def = AgentDef {
            name: "ralph-verifier".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Critic,
            system_prompt_ref: "prompts/ralph-verifier.md".into(),
        };
        let json = serde_json::to_string(&def).unwrap();
        let parsed: AgentDef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "ralph-verifier");
        assert_eq!(parsed.posture, Posture::Autonomous);
        assert_eq!(parsed.routing_role, RoutingRole::Critic);
    }

    #[test]
    fn registry_lookup_returns_matching_agent() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "test-worker".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "prompts/test-worker.md".into(),
        });

        let found = registry.lookup("test-worker");
        assert!(found.is_some());
        assert_eq!(found.unwrap().model_class, ModelClass::Medium);
    }

    #[test]
    fn registry_lookup_returns_none_for_missing() {
        let registry = AgentRegistry::new();
        assert!(registry.lookup("nonexistent").is_none());
    }

    #[test]
    fn registry_filter_by_posture() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "a1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/a1.md".into(),
        });
        registry.register(AgentDef {
            name: "a2".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/a2.md".into(),
        });
        registry.register(AgentDef {
            name: "a3".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::General,
            system_prompt_ref: "p/a3.md".into(),
        });

        let autonomous = registry.filter_by_posture(Posture::Autonomous);
        assert_eq!(autonomous.len(), 2);
        assert!(autonomous.iter().all(|a| a.posture == Posture::Autonomous));
    }

    #[test]
    fn registry_filter_by_model_class() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "heavy1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/h1.md".into(),
        });
        registry.register(AgentDef {
            name: "light1".into(),
            posture: Posture::Advisory,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::General,
            system_prompt_ref: "p/l1.md".into(),
        });

        let heavy = registry.filter_by_model_class(ModelClass::Heavy);
        assert_eq!(heavy.len(), 1);
        assert_eq!(heavy[0].name, "heavy1");
    }

    #[test]
    fn registry_filter_by_role() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "planner1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Planner,
            system_prompt_ref: "p/p1.md".into(),
        });
        registry.register(AgentDef {
            name: "worker1".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/w1.md".into(),
        });

        let planners = registry.filter_by_role(RoutingRole::Planner);
        assert_eq!(planners.len(), 1);
        assert_eq!(planners[0].name, "planner1");
    }

    #[test]
    fn registry_validate_rejects_duplicate_names() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "dup".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/dup.md".into(),
        });
        registry.register(AgentDef {
            name: "dup".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/dup2.md".into(),
        });

        let result = registry.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn registry_validate_passes_when_unique() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "unique1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/u1.md".into(),
        });
        registry.register(AgentDef {
            name: "unique2".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/u2.md".into(),
        });

        assert!(registry.validate().is_ok());
    }

    #[test]
    fn registry_all_returns_all_agents() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "a".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/a.md".into(),
        });
        registry.register(AgentDef {
            name: "b".into(),
            posture: Posture::Advisory,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::General,
            system_prompt_ref: "p/b.md".into(),
        });

        assert_eq!(registry.all().len(), 2);
    }

    #[test]
    fn registry_from_toml_parses_agent_definitions() {
        let toml_str = r#"
[[agents]]
name = "toml-agent"
posture = "autonomous"
model_class = "heavy"
routing_role = "leader"
system_prompt_ref = "prompts/toml-agent.md"

[[agents]]
name = "toml-worker"
posture = "supervised"
model_class = "medium"
routing_role = "worker"
system_prompt_ref = "prompts/toml-worker.md"
"#;
        let registry = AgentRegistry::from_toml(toml_str).unwrap();
        assert_eq!(registry.all().len(), 2);
        assert!(registry.lookup("toml-agent").is_some());
        assert!(registry.lookup("toml-worker").is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p omx-agents`
Expected: FAIL — types and `AgentRegistry` not defined

- [ ] **Step 3: Implement the types and registry**

Replace the contents of `crates/omx-agents/src/lib.rs` with the full implementation:

```rust
//! Agent type definitions and registry.

use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Posture {
    Autonomous,
    Supervised,
    Advisory,
}

impl std::fmt::Display for Posture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Autonomous => write!(f, "autonomous"),
            Self::Supervised => write!(f, "supervised"),
            Self::Advisory => write!(f, "advisory"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelClass {
    Heavy,
    Medium,
    Light,
}

impl std::fmt::Display for ModelClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Heavy => write!(f, "heavy"),
            Self::Medium => write!(f, "medium"),
            Self::Light => write!(f, "light"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingRole {
    Leader,
    Worker,
    Critic,
    Planner,
    Architect,
    General,
}

impl std::fmt::Display for RoutingRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Leader => write!(f, "leader"),
            Self::Worker => write!(f, "worker"),
            Self::Critic => write!(f, "critic"),
            Self::Planner => write!(f, "planner"),
            Self::Architect => write!(f, "architect"),
            Self::General => write!(f, "general"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub posture: Posture,
    pub model_class: ModelClass,
    pub routing_role: RoutingRole,
    pub system_prompt_ref: String,
}

// ---------------------------------------------------------------------------
// TOML deserialization wrapper
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct AgentTomlFile {
    #[serde(default)]
    agents: Vec<AgentDef>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    agents: Vec<AgentDef>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self { agents: Vec::new() }
    }

    pub fn register(&mut self, def: AgentDef) {
        self.agents.push(def);
    }

    pub fn lookup(&self, name: &str) -> Option<&AgentDef> {
        self.agents.iter().find(|a| a.name == name)
    }

    pub fn filter_by_posture(&self, posture: Posture) -> Vec<&AgentDef> {
        self.agents.iter().filter(|a| a.posture == posture).collect()
    }

    pub fn filter_by_model_class(&self, model_class: ModelClass) -> Vec<&AgentDef> {
        self.agents.iter().filter(|a| a.model_class == model_class).collect()
    }

    pub fn filter_by_role(&self, role: RoutingRole) -> Vec<&AgentDef> {
        self.agents.iter().filter(|a| a.routing_role == role).collect()
    }

    pub fn all(&self) -> &[AgentDef] {
        &self.agents
    }

    pub fn validate(&self) -> Result<(), OmxError> {
        let mut seen = HashMap::new();
        for agent in &self.agents {
            if let Some(prev_idx) = seen.insert(&agent.name, true) {
                return Err(OmxError::Agent(format!(
                    "duplicate agent name: '{}'",
                    agent.name
                )));
            }
        }
        Ok(())
    }

    pub fn from_toml(toml_str: &str) -> Result<Self, OmxError> {
        let file: AgentTomlFile = toml::from_str(toml_str)
            .map_err(|e| OmxError::Agent(format!("invalid agent TOML: {e}")))?;
        let mut registry = Self::new();
        for def in file.agents {
            registry.register(def);
        }
        Ok(registry)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posture_serde_roundtrip() {
        let p = Posture::Autonomous;
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Posture = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, p);
    }

    #[test]
    fn model_class_display() {
        assert_eq!(ModelClass::Heavy.to_string(), "heavy");
        assert_eq!(ModelClass::Medium.to_string(), "medium");
        assert_eq!(ModelClass::Light.to_string(), "light");
    }

    #[test]
    fn routing_role_serde_roundtrip() {
        let r = RoutingRole::Planner;
        let json = serde_json::to_string(&r).unwrap();
        let parsed: RoutingRole = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, r);
    }

    #[test]
    fn agent_def_serde_roundtrip() {
        let def = AgentDef {
            name: "ralph-verifier".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Critic,
            system_prompt_ref: "prompts/ralph-verifier.md".into(),
        };
        let json = serde_json::to_string(&def).unwrap();
        let parsed: AgentDef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "ralph-verifier");
        assert_eq!(parsed.posture, Posture::Autonomous);
        assert_eq!(parsed.routing_role, RoutingRole::Critic);
    }

    #[test]
    fn registry_lookup_returns_matching_agent() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "test-worker".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "prompts/test-worker.md".into(),
        });

        let found = registry.lookup("test-worker");
        assert!(found.is_some());
        assert_eq!(found.unwrap().model_class, ModelClass::Medium);
    }

    #[test]
    fn registry_lookup_returns_none_for_missing() {
        let registry = AgentRegistry::new();
        assert!(registry.lookup("nonexistent").is_none());
    }

    #[test]
    fn registry_filter_by_posture() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "a1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/a1.md".into(),
        });
        registry.register(AgentDef {
            name: "a2".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/a2.md".into(),
        });
        registry.register(AgentDef {
            name: "a3".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::General,
            system_prompt_ref: "p/a3.md".into(),
        });

        let autonomous = registry.filter_by_posture(Posture::Autonomous);
        assert_eq!(autonomous.len(), 2);
        assert!(autonomous.iter().all(|a| a.posture == Posture::Autonomous));
    }

    #[test]
    fn registry_filter_by_model_class() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "heavy1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/h1.md".into(),
        });
        registry.register(AgentDef {
            name: "light1".into(),
            posture: Posture::Advisory,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::General,
            system_prompt_ref: "p/l1.md".into(),
        });

        let heavy = registry.filter_by_model_class(ModelClass::Heavy);
        assert_eq!(heavy.len(), 1);
        assert_eq!(heavy[0].name, "heavy1");
    }

    #[test]
    fn registry_filter_by_role() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "planner1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Planner,
            system_prompt_ref: "p/p1.md".into(),
        });
        registry.register(AgentDef {
            name: "worker1".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/w1.md".into(),
        });

        let planners = registry.filter_by_role(RoutingRole::Planner);
        assert_eq!(planners.len(), 1);
        assert_eq!(planners[0].name, "planner1");
    }

    #[test]
    fn registry_validate_rejects_duplicate_names() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "dup".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/dup.md".into(),
        });
        registry.register(AgentDef {
            name: "dup".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/dup2.md".into(),
        });

        let result = registry.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn registry_validate_passes_when_unique() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "unique1".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/u1.md".into(),
        });
        registry.register(AgentDef {
            name: "unique2".into(),
            posture: Posture::Supervised,
            model_class: ModelClass::Medium,
            routing_role: RoutingRole::Worker,
            system_prompt_ref: "p/u2.md".into(),
        });

        assert!(registry.validate().is_ok());
    }

    #[test]
    fn registry_all_returns_all_agents() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "a".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/a.md".into(),
        });
        registry.register(AgentDef {
            name: "b".into(),
            posture: Posture::Advisory,
            model_class: ModelClass::Light,
            routing_role: RoutingRole::General,
            system_prompt_ref: "p/b.md".into(),
        });

        assert_eq!(registry.all().len(), 2);
    }

    #[test]
    fn registry_from_toml_parses_agent_definitions() {
        let toml_str = r#"
[[agents]]
name = "toml-agent"
posture = "autonomous"
model_class = "heavy"
routing_role = "leader"
system_prompt_ref = "prompts/toml-agent.md"

[[agents]]
name = "toml-worker"
posture = "supervised"
model_class = "medium"
routing_role = "worker"
system_prompt_ref = "prompts/toml-worker.md"
"#;
        let registry = AgentRegistry::from_toml(toml_str).unwrap();
        assert_eq!(registry.all().len(), 2);
        assert!(registry.lookup("toml-agent").is_some());
        assert!(registry.lookup("toml-worker").is_some());
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omx-agents`
Expected: ALL PASS (12 tests)

- [ ] **Step 5: Commit**

```bash
git add crates/omx-agents/
git commit -m "feat(omx-agents): implement AgentDef types and AgentRegistry with TOML loading"
```

---

## Task 3: Implement omx-modes — mode lifecycle and conflict matrix

**Files:**
- Create: `crates/omx-modes/src/lib.rs`

- [ ] **Step 1: Write the full implementation with tests**

Replace `crates/omx-modes/src/lib.rs` with:

```rust
//! Mode lifecycle management with exclusive conflict checking.

use chrono::{DateTime, Utc};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Autopilot,
    Autoresearch,
    DeepInterview,
    Ralph,
    Ultrawork,
    Team,
    Ultraqa,
    Ralplan,
}

impl Mode {
    pub const ALL: &'static [Mode] = &[
        Mode::Autopilot,
        Mode::Autoresearch,
        Mode::DeepInterview,
        Mode::Ralph,
        Mode::Ultrawork,
        Mode::Team,
        Mode::Ultraqa,
        Mode::Ralplan,
    ];

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "autopilot" => Some(Self::Autopilot),
            "autoresearch" => Some(Self::Autoresearch),
            "deep-interview" | "deep_interview" | "deepinterview" => Some(Self::DeepInterview),
            "ralph" => Some(Self::Ralph),
            "ultrawork" => Some(Self::Ultrawork),
            "team" => Some(Self::Team),
            "ultraqa" => Some(Self::Ultraqa),
            "ralplan" => Some(Self::Ralplan),
            _ => None,
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Autopilot => write!(f, "autopilot"),
            Self::Autoresearch => write!(f, "autoresearch"),
            Self::DeepInterview => write!(f, "deep-interview"),
            Self::Ralph => write!(f, "ralph"),
            Self::Ultrawork => write!(f, "ultrawork"),
            Self::Team => write!(f, "team"),
            Self::Ultraqa => write!(f, "ultraqa"),
            Self::Ralplan => write!(f, "ralplan"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HudPreset {
    Minimal,
    Standard,
    Verbose,
}

impl Default for HudPreset {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub agent_overrides: HashMap<String, String>,
    pub hud_preset: HudPreset,
    pub env_overrides: HashMap<String, String>,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            agent_overrides: HashMap::new(),
            hud_preset: HudPreset::Standard,
            env_overrides: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeState {
    pub active: Mode,
    pub activated_at: DateTime<Utc>,
    pub session_id: String,
    pub config: ModeConfig,
}

// ---------------------------------------------------------------------------
// Conflict matrix
// ---------------------------------------------------------------------------

/// Returns true if modes `a` and `b` can be active simultaneously.
/// Orchestration modes (Ralph, Team, Autoresearch, Ralplan) conflict with each other.
pub fn is_compatible(a: Mode, b: Mode) -> bool {
    if a == b {
        return true; // Same mode is always compatible with itself
    }
    let orchestration = [Mode::Ralph, Mode::Team, Mode::Autoresearch, Mode::Ralplan];
    let a_is_orch = orchestration.contains(&a);
    let b_is_orch = orchestration.contains(&b);
    // Two orchestration modes conflict
    !(a_is_orch && b_is_orch)
}

// ---------------------------------------------------------------------------
// Mode manager
// ---------------------------------------------------------------------------

pub struct ModeManager {
    current: Option<ModeState>,
}

impl ModeManager {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn current(&self) -> Option<&ModeState> {
        self.current.as_ref()
    }

    pub fn activate(
        &mut self,
        mode: Mode,
        session_id: String,
        config: ModeConfig,
    ) -> Result<&ModeState, OmxError> {
        if let Some(ref current) = self.current {
            if !is_compatible(current.active, mode) {
                return Err(OmxError::Mode(format!(
                    "cannot activate '{}': conflicts with active mode '{}'",
                    mode, current.active
                )));
            }
        }

        self.current = Some(ModeState {
            active: mode,
            activated_at: Utc::now(),
            session_id,
            config,
        });

        Ok(self.current.as_ref().unwrap())
    }

    pub fn deactivate(&mut self) -> Result<(), OmxError> {
        if self.current.is_none() {
            return Err(OmxError::Mode("no active mode to deactivate".into()));
        }
        self.current = None;
        Ok(())
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_serde_roundtrip() {
        let mode = Mode::Ralph;
        let json = serde_json::to_string(&mode).unwrap();
        let parsed: Mode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mode);
    }

    #[test]
    fn mode_display() {
        assert_eq!(Mode::Autopilot.to_string(), "autopilot");
        assert_eq!(Mode::DeepInterview.to_string(), "deep-interview");
        assert_eq!(Mode::Ralplan.to_string(), "ralplan");
    }

    #[test]
    fn mode_from_str_parses_known_modes() {
        assert_eq!(Mode::from_str("autopilot"), Some(Mode::Autopilot));
        assert_eq!(Mode::from_str("Ralph"), Some(Mode::Ralph));
        assert_eq!(Mode::from_str("deep-interview"), Some(Mode::DeepInterview));
        assert_eq!(Mode::from_str("deep_interview"), Some(Mode::DeepInterview));
        assert_eq!(Mode::from_str("unknown"), None);
    }

    #[test]
    fn mode_all_has_eight_modes() {
        assert_eq!(Mode::ALL.len(), 8);
    }

    #[test]
    fn same_mode_is_compatible() {
        assert!(is_compatible(Mode::Ralph, Mode::Ralph));
        assert!(is_compatible(Mode::Team, Mode::Team));
    }

    #[test]
    fn orchestration_modes_conflict() {
        assert!(!is_compatible(Mode::Ralph, Mode::Team));
        assert!(!is_compatible(Mode::Team, Mode::Autoresearch));
        assert!(!is_compatible(Mode::Ralplan, Mode::Ralph));
        assert!(!is_compatible(Mode::Autoresearch, Mode::Ralplan));
    }

    #[test]
    fn non_orchestration_modes_are_compatible_with_orchestration() {
        assert!(is_compatible(Mode::Autopilot, Mode::Team));
        assert!(is_compatible(Mode::Ultrawork, Mode::Ralph));
        assert!(is_compatible(Mode::DeepInterview, Mode::Ralplan));
        assert!(is_compatible(Mode::Ultraqa, Mode::Autoresearch));
    }

    #[test]
    fn non_orchestration_modes_are_compatible_with_each_other() {
        assert!(is_compatible(Mode::Autopilot, Mode::Ultrawork));
        assert!(is_compatible(Mode::DeepInterview, Mode::Ultraqa));
    }

    #[test]
    fn mode_manager_starts_with_no_active_mode() {
        let mgr = ModeManager::new();
        assert!(mgr.current().is_none());
    }

    #[test]
    fn mode_manager_activate_sets_current() {
        let mut mgr = ModeManager::new();
        let state = mgr
            .activate(Mode::Autopilot, "sess-1".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Autopilot);
        assert_eq!(state.session_id, "sess-1");
    }

    #[test]
    fn mode_manager_activate_conflicting_mode_fails() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Team, "sess-1".into(), ModeConfig::default())
            .unwrap();

        let result = mgr.activate(Mode::Ralph, "sess-1".into(), ModeConfig::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("conflicts"));
    }

    #[test]
    fn mode_manager_activate_compatible_mode_replaces_current() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Autopilot, "sess-1".into(), ModeConfig::default())
            .unwrap();

        let state = mgr
            .activate(Mode::Ultrawork, "sess-1".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Ultrawork);
    }

    #[test]
    fn mode_manager_deactivate_clears_current() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Ralph, "sess-1".into(), ModeConfig::default())
            .unwrap();

        mgr.deactivate().unwrap();
        assert!(mgr.current().is_none());
    }

    #[test]
    fn mode_manager_deactivate_when_none_returns_error() {
        let mut mgr = ModeManager::new();
        let result = mgr.deactivate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no active mode"));
    }

    #[test]
    fn mode_manager_activate_after_deactivate_works() {
        let mut mgr = ModeManager::new();
        mgr.activate(Mode::Team, "sess-1".into(), ModeConfig::default())
            .unwrap();
        mgr.deactivate().unwrap();

        let state = mgr
            .activate(Mode::Ralph, "sess-2".into(), ModeConfig::default())
            .unwrap();
        assert_eq!(state.active, Mode::Ralph);
        assert_eq!(state.session_id, "sess-2");
    }

    #[test]
    fn hud_preset_default_is_standard() {
        assert_eq!(HudPreset::default(), HudPreset::Standard);
    }

    #[test]
    fn mode_state_serde_roundtrip() {
        let state = ModeState {
            active: Mode::Ralph,
            activated_at: Utc::now(),
            session_id: "sess-test".into(),
            config: ModeConfig::default(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: ModeState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.active, Mode::Ralph);
        assert_eq!(parsed.session_id, "sess-test");
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p omx-modes`
Expected: ALL PASS (17 tests)

- [ ] **Step 3: Commit**

```bash
git add crates/omx-modes/
git commit -m "feat(omx-modes): implement Mode enum, conflict matrix, and ModeManager lifecycle"
```

---

## Task 4: Implement omx-session — session tracking and search

**Files:**
- Create: `crates/omx-session/src/lib.rs`
- Create: `crates/omx-session/src/store.rs`
- Create: `crates/omx-session/src/search.rs`

- [ ] **Step 1: Write lib.rs with types and re-exports**

Replace `crates/omx-session/src/lib.rs` with:

```rust
//! Session tracking and history.

pub mod search;
pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Completed,
    Crashed,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Completed => write!(f, "completed"),
            Self::Crashed => write!(f, "crashed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub mode: String,
    pub turns: u32,
    pub tokens_used: u64,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub turn_number: u32,
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub content: String,
    pub tokens: u64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub session_id: String,
    pub turn_number: u32,
    pub snippet: String,
    pub score: f64,
}

pub use store::FileSessionStore;
```

- [ ] **Step 2: Write store.rs with FileSessionStore implementation**

Create `crates/omx-session/src/store.rs`:

```rust
//! File-based session store implementation.

use crate::{Session, SessionStatus, SessionTurn};
use chrono::Utc;
use omx_types::OmxError;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct FileSessionStore {
    root: PathBuf,
}

impl FileSessionStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    fn session_dir(&self, id: &str) -> PathBuf {
        self.sessions_dir().join(id)
    }

    fn session_meta_path(&self, id: &str) -> PathBuf {
        self.session_dir(id).join("meta.json")
    }

    fn transcript_path(&self, id: &str) -> PathBuf {
        self.session_dir(id).join("transcript.jsonl")
    }

    pub async fn create(&self, mode: &str) -> Result<Session, OmxError> {
        let id = ulid::Ulid::new().to_string().to_lowercase();
        let session = Session {
            id: id.clone(),
            started_at: Utc::now(),
            ended_at: None,
            mode: mode.to_string(),
            turns: 0,
            tokens_used: 0,
            status: SessionStatus::Active,
        };

        let dir = self.session_dir(&id);
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| OmxError::Session(format!("failed to create session dir: {e}")))?;

        let json = serde_json::to_string_pretty(&session)
            .map_err(|e| OmxError::Session(format!("failed to serialize session: {e}")))?;
        fs::write(self.session_meta_path(&id), json)
            .await
            .map_err(|e| OmxError::Session(format!("failed to write session meta: {e}")))?;

        Ok(session)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Session>, OmxError> {
        let path = self.session_meta_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&path)
            .await
            .map_err(|e| OmxError::Session(format!("failed to read session: {e}")))?;
        let session: Session = serde_json::from_str(&contents)
            .map_err(|e| OmxError::Session(format!("failed to parse session: {e}")))?;
        Ok(Some(session))
    }

    pub async fn update(&self, session: &Session) -> Result<(), OmxError> {
        let json = serde_json::to_string_pretty(session)
            .map_err(|e| OmxError::Session(format!("failed to serialize session: {e}")))?;
        fs::write(self.session_meta_path(&session.id), json)
            .await
            .map_err(|e| OmxError::Session(format!("failed to write session: {e}")))?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Session>, OmxError> {
        let dir = self.sessions_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&dir)
            .await
            .map_err(|e| OmxError::Session(format!("failed to read sessions dir: {e}")))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| OmxError::Session(format!("failed to read entry: {e}")))?
        {
            let meta_path = entry.path().join("meta.json");
            if meta_path.exists() {
                let contents = fs::read_to_string(&meta_path).await.map_err(|e| {
                    OmxError::Session(format!("failed to read session meta: {e}"))
                })?;
                if let Ok(session) = serde_json::from_str::<Session>(&contents) {
                    sessions.push(session);
                }
            }
        }
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(sessions)
    }

    pub async fn append_turn(&self, session_id: &str, turn: &SessionTurn) -> Result<(), OmxError> {
        let path = self.transcript_path(session_id);
        let mut line = serde_json::to_string(turn)
            .map_err(|e| OmxError::Session(format!("failed to serialize turn: {e}")))?;
        line.push('\n');

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|e| OmxError::Session(format!("failed to open transcript: {e}")))?;
        file.write_all(line.as_bytes())
            .await
            .map_err(|e| OmxError::Session(format!("failed to write turn: {e}")))?;

        Ok(())
    }

    pub async fn read_transcript(&self, session_id: &str) -> Result<Vec<SessionTurn>, OmxError> {
        let path = self.transcript_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(&path)
            .await
            .map_err(|e| OmxError::Session(format!("failed to read transcript: {e}")))?;
        let mut turns = Vec::new();
        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let turn: SessionTurn = serde_json::from_str(line)
                .map_err(|e| OmxError::Session(format!("failed to parse turn: {e}")))?;
            turns.push(turn);
        }
        Ok(turns)
    }

    pub async fn complete(&self, session_id: &str) -> Result<(), OmxError> {
        let mut session = self
            .get(session_id)
            .await?
            .ok_or_else(|| OmxError::Session(format!("session not found: {session_id}")))?;
        session.status = SessionStatus::Completed;
        session.ended_at = Some(Utc::now());
        self.update(&session).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn create_session_generates_ulid_and_writes_meta() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());

        let session = store.create("autopilot").await.unwrap();
        assert!(!session.id.is_empty());
        assert_eq!(session.mode, "autopilot");
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.turns, 0);
        assert_eq!(session.tokens_used, 0);
    }

    #[tokio::test]
    async fn get_returns_created_session() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());

        let created = store.create("ralph").await.unwrap();
        let fetched = store.get(&created.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().mode, "ralph");
    }

    #[tokio::test]
    async fn get_returns_none_for_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        assert!(store.get("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_returns_sessions_sorted_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());

        let s1 = store.create("mode1").await.unwrap();
        let s2 = store.create("mode2").await.unwrap();

        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 2);
        // Newest first
        assert_eq!(all[0].id, s2.id);
        assert_eq!(all[1].id, s1.id);
    }

    #[tokio::test]
    async fn list_returns_empty_when_no_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());
        assert!(store.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn append_and_read_transcript() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());

        let session = store.create("team").await.unwrap();

        let turn1 = SessionTurn {
            turn_number: 1,
            timestamp: Utc::now(),
            role: "user".into(),
            content: "hello".into(),
            tokens: 5,
        };
        let turn2 = SessionTurn {
            turn_number: 2,
            timestamp: Utc::now(),
            role: "assistant".into(),
            content: "hi there".into(),
            tokens: 10,
        };

        store.append_turn(&session.id, &turn1).await.unwrap();
        store.append_turn(&session.id, &turn2).await.unwrap();

        let turns = store.read_transcript(&session.id).await.unwrap();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].content, "hello");
        assert_eq!(turns[1].content, "hi there");
    }

    #[tokio::test]
    async fn read_transcript_returns_empty_for_new_session() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());

        let session = store.create("autopilot").await.unwrap();
        let turns = store.read_transcript(&session.id).await.unwrap();
        assert!(turns.is_empty());
    }

    #[tokio::test]
    async fn complete_marks_session_completed() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());

        let session = store.create("ralph").await.unwrap();
        store.complete(&session.id).await.unwrap();

        let fetched = store.get(&session.id).await.unwrap().unwrap();
        assert_eq!(fetched.status, SessionStatus::Completed);
        assert!(fetched.ended_at.is_some());
    }

    #[tokio::test]
    async fn update_persists_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileSessionStore::new(tmp.path().to_path_buf());

        let mut session = store.create("team").await.unwrap();
        session.turns = 42;
        session.tokens_used = 5000;
        store.update(&session).await.unwrap();

        let fetched = store.get(&session.id).await.unwrap().unwrap();
        assert_eq!(fetched.turns, 42);
        assert_eq!(fetched.tokens_used, 5000);
    }
}
```

- [ ] **Step 3: Write search.rs with transcript search**

Create `crates/omx-session/src/search.rs`:

```rust
//! Full-text search over session transcripts.

use crate::{SearchResult, SessionTurn};

/// Search turns for a query string (case-insensitive substring match).
/// Returns matching turns ranked by simple relevance (exact match > partial).
pub fn search_turns(
    session_id: &str,
    turns: &[SessionTurn],
    query: &str,
) -> Vec<SearchResult> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for turn in turns {
        let content_lower = turn.content.to_lowercase();
        if content_lower.contains(&query_lower) {
            let count = content_lower.matches(&query_lower).count();
            let score = count as f64 / content_lower.len().max(1) as f64;
            let snippet = make_snippet(&turn.content, query, 80);
            results.push(SearchResult {
                session_id: session_id.to_string(),
                turn_number: turn.turn_number,
                snippet,
                score,
            });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

fn make_snippet(content: &str, query: &str, max_len: usize) -> String {
    let lower = content.to_lowercase();
    let query_lower = query.to_lowercase();
    if let Some(pos) = lower.find(&query_lower) {
        let start = pos.saturating_sub(max_len / 2);
        let end = (pos + query.len() + max_len / 2).min(content.len());
        let slice = &content[start..end];
        if start > 0 || end < content.len() {
            format!("...{}...", slice.trim())
        } else {
            slice.to_string()
        }
    } else {
        content.chars().take(max_len).collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_turn(num: u32, content: &str) -> SessionTurn {
        SessionTurn {
            turn_number: num,
            timestamp: Utc::now(),
            role: "assistant".into(),
            content: content.into(),
            tokens: 10,
        }
    }

    #[test]
    fn search_finds_matching_turns() {
        let turns = vec![
            make_turn(1, "Hello world"),
            make_turn(2, "Fix the bug in auth module"),
            make_turn(3, "The authentication is broken"),
        ];

        let results = search_turns("sess-1", &turns, "auth");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_is_case_insensitive() {
        let turns = vec![make_turn(1, "ERROR: connection refused")];

        let results = search_turns("sess-1", &turns, "error");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let turns = vec![make_turn(1, "Hello world")];

        let results = search_turns("sess-1", &turns, "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn search_returns_sorted_by_score() {
        let turns = vec![
            make_turn(1, "auth auth auth is critical"),
            make_turn(2, "the auth module handles authentication"),
        ];

        let results = search_turns("sess-1", &turns, "auth");
        assert_eq!(results.len(), 2);
        // Turn 1 has higher density of "auth"
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn snippet_includes_context_around_match() {
        let snippet = make_snippet(
            "The quick brown fox jumps over the lazy dog near the authentication module",
            "authentication",
            80,
        );
        assert!(snippet.contains("authentication"));
    }

    #[test]
    fn snippet_short_content_returns_full_string() {
        let snippet = make_snippet("auth error", "auth", 80);
        assert_eq!(snippet, "auth error");
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p omx-session`
Expected: ALL PASS (13 tests — 7 async store tests + 6 search tests)

- [ ] **Step 5: Commit**

```bash
git add crates/omx-session/
git commit -m "feat(omx-session): implement FileSessionStore with ULID IDs, transcript JSONL, and full-text search"
```

---

## Task 5: Implement omx-catalog — skill registry and discovery

**Files:**
- Create: `crates/omx-catalog/src/lib.rs`

- [ ] **Step 1: Write the full implementation with tests**

Replace `crates/omx-catalog/src/lib.rs` with:

```rust
//! Skill and agent catalog with filesystem discovery and validation.

use omx_agents::{AgentDef, AgentRegistry};
use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Skill definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub dependencies: Vec<String>,
    pub path: PathBuf,
}

// ---------------------------------------------------------------------------
// TOML deserialization for skill files
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SkillTomlFile {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Catalog registry
// ---------------------------------------------------------------------------

pub struct CatalogRegistry {
    skills: Vec<SkillDef>,
    agents: AgentRegistry,
}

impl CatalogRegistry {
    pub fn new(agents: AgentRegistry) -> Self {
        Self {
            skills: Vec::new(),
            agents,
        }
    }

    pub fn register_skill(&mut self, skill: SkillDef) {
        self.skills.push(skill);
    }

    pub fn list_skills(&self) -> &[SkillDef] {
        &self.skills
    }

    pub fn get_skill(&self, name: &str) -> Option<&SkillDef> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn list_agents(&self) -> &[AgentDef] {
        self.agents.all()
    }

    /// Scan a directory for .toml skill definitions.
    pub async fn discover_skills(&mut self, dir: &Path) -> Result<usize, OmxError> {
        if !dir.exists() {
            return Ok(0);
        }
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(dir)
            .await
            .map_err(|e| OmxError::Catalog(format!("failed to read skill dir: {e}")))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| OmxError::Catalog(format!("failed to read entry: {e}")))?
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let contents = tokio::fs::read_to_string(&path).await.map_err(|e| {
                    OmxError::Catalog(format!("failed to read {}: {e}", path.display()))
                })?;
                let toml_file: SkillTomlFile = toml::from_str(&contents).map_err(|e| {
                    OmxError::Catalog(format!("invalid skill TOML in {}: {e}", path.display()))
                })?;
                self.register_skill(SkillDef {
                    name: toml_file.name,
                    description: toml_file.description.unwrap_or_default(),
                    version: toml_file.version,
                    dependencies: toml_file.dependencies,
                    path,
                });
                count += 1;
            }
        }
        Ok(count)
    }

    /// Validate: no duplicate skill names, all dependencies resolvable.
    pub fn validate(&self) -> Result<(), OmxError> {
        let mut names = HashSet::new();
        for skill in &self.skills {
            if !names.insert(&skill.name) {
                return Err(OmxError::Catalog(format!(
                    "duplicate skill name: '{}'",
                    skill.name
                )));
            }
        }

        for skill in &self.skills {
            for dep in &skill.dependencies {
                if !names.contains(dep) {
                    return Err(OmxError::Catalog(format!(
                        "skill '{}' depends on '{}' which is not registered",
                        skill.name, dep
                    )));
                }
            }
        }

        Ok(())
    }

    /// Return skills in dependency order (topological sort).
    pub fn resolve_order(&self) -> Result<Vec<&SkillDef>, OmxError> {
        let name_to_idx: HashMap<&str, usize> = self
            .skills
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.as_str(), i))
            .collect();

        let n = self.skills.len();
        let mut in_degree = vec![0usize; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

        for (i, skill) in self.skills.iter().enumerate() {
            for dep in &skill.dependencies {
                if let Some(&dep_idx) = name_to_idx.get(dep.as_str()) {
                    adj[dep_idx].push(i);
                    in_degree[i] += 1;
                }
            }
        }

        let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut order = Vec::with_capacity(n);

        while let Some(idx) = queue.pop() {
            order.push(&self.skills[idx]);
            for &next in &adj[idx] {
                in_degree[next] -= 1;
                if in_degree[next] == 0 {
                    queue.push(next);
                }
            }
        }

        if order.len() != n {
            return Err(OmxError::Catalog(
                "circular dependency detected among skills".into(),
            ));
        }

        Ok(order)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use omx_agents::{AgentRegistry, ModelClass, Posture, RoutingRole};
    use std::fs;

    fn empty_registry() -> AgentRegistry {
        AgentRegistry::new()
    }

    #[test]
    fn catalog_starts_empty() {
        let catalog = CatalogRegistry::new(empty_registry());
        assert!(catalog.list_skills().is_empty());
    }

    #[test]
    fn register_and_get_skill() {
        let mut catalog = CatalogRegistry::new(empty_registry());
        catalog.register_skill(SkillDef {
            name: "test-skill".into(),
            description: "A test".into(),
            version: Some("1.0".into()),
            dependencies: vec![],
            path: PathBuf::from("skills/test.toml"),
        });

        assert_eq!(catalog.list_skills().len(), 1);
        let found = catalog.get_skill("test-skill");
        assert!(found.is_some());
        assert_eq!(found.unwrap().description, "A test");
    }

    #[test]
    fn get_skill_returns_none_for_missing() {
        let catalog = CatalogRegistry::new(empty_registry());
        assert!(catalog.get_skill("nope").is_none());
    }

    #[test]
    fn validate_passes_with_resolved_deps() {
        let mut catalog = CatalogRegistry::new(empty_registry());
        catalog.register_skill(SkillDef {
            name: "base".into(),
            description: "".into(),
            version: None,
            dependencies: vec![],
            path: PathBuf::from("base.toml"),
        });
        catalog.register_skill(SkillDef {
            name: "derived".into(),
            description: "".into(),
            version: None,
            dependencies: vec!["base".into()],
            path: PathBuf::from("derived.toml"),
        });

        assert!(catalog.validate().is_ok());
    }

    #[test]
    fn validate_fails_with_unresolved_dep() {
        let mut catalog = CatalogRegistry::new(empty_registry());
        catalog.register_skill(SkillDef {
            name: "orphan".into(),
            description: "".into(),
            version: None,
            dependencies: vec!["missing".into()],
            path: PathBuf::from("orphan.toml"),
        });

        let result = catalog.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing"));
    }

    #[test]
    fn validate_fails_with_duplicate_names() {
        let mut catalog = CatalogRegistry::new(empty_registry());
        catalog.register_skill(SkillDef {
            name: "dup".into(),
            description: "first".into(),
            version: None,
            dependencies: vec![],
            path: PathBuf::from("dup1.toml"),
        });
        catalog.register_skill(SkillDef {
            name: "dup".into(),
            description: "second".into(),
            version: None,
            dependencies: vec![],
            path: PathBuf::from("dup2.toml"),
        });

        let result = catalog.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn resolve_order_returns_deps_before_dependents() {
        let mut catalog = CatalogRegistry::new(empty_registry());
        catalog.register_skill(SkillDef {
            name: "c".into(),
            description: "".into(),
            version: None,
            dependencies: vec!["b".into()],
            path: PathBuf::from("c.toml"),
        });
        catalog.register_skill(SkillDef {
            name: "a".into(),
            description: "".into(),
            version: None,
            dependencies: vec![],
            path: PathBuf::from("a.toml"),
        });
        catalog.register_skill(SkillDef {
            name: "b".into(),
            description: "".into(),
            version: None,
            dependencies: vec!["a".into()],
            path: PathBuf::from("b.toml"),
        });

        let order = catalog.resolve_order().unwrap();
        let names: Vec<&str> = order.iter().map(|s| s.name.as_str()).collect();
        let a_pos = names.iter().position(|&n| n == "a").unwrap();
        let b_pos = names.iter().position(|&n| n == "b").unwrap();
        let c_pos = names.iter().position(|&n| n == "c").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn resolve_order_detects_circular_deps() {
        let mut catalog = CatalogRegistry::new(empty_registry());
        catalog.register_skill(SkillDef {
            name: "x".into(),
            description: "".into(),
            version: None,
            dependencies: vec!["y".into()],
            path: PathBuf::from("x.toml"),
        });
        catalog.register_skill(SkillDef {
            name: "y".into(),
            description: "".into(),
            version: None,
            dependencies: vec!["x".into()],
            path: PathBuf::from("y.toml"),
        });

        let result = catalog.resolve_order();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[tokio::test]
    async fn discover_skills_reads_toml_files() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("skills");
        fs::create_dir_all(&skill_dir).unwrap();

        fs::write(
            skill_dir.join("first.toml"),
            r#"
name = "first-skill"
description = "The first skill"
version = "1.0"
"#,
        )
        .unwrap();

        fs::write(
            skill_dir.join("second.toml"),
            r#"
name = "second-skill"
description = "Depends on first"
dependencies = ["first-skill"]
"#,
        )
        .unwrap();

        // Non-TOML file should be ignored
        fs::write(skill_dir.join("readme.md"), "# Skills").unwrap();

        let mut catalog = CatalogRegistry::new(empty_registry());
        let count = catalog.discover_skills(&skill_dir).await.unwrap();
        assert_eq!(count, 2);
        assert_eq!(catalog.list_skills().len(), 2);
        assert!(catalog.get_skill("first-skill").is_some());
        assert!(catalog.get_skill("second-skill").is_some());
    }

    #[tokio::test]
    async fn discover_skills_returns_zero_for_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nonexistent");

        let mut catalog = CatalogRegistry::new(empty_registry());
        let count = catalog.discover_skills(&missing).await.unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn list_agents_delegates_to_agent_registry() {
        let mut agents = AgentRegistry::new();
        agents.register(AgentDef {
            name: "test-agent".into(),
            posture: Posture::Autonomous,
            model_class: ModelClass::Heavy,
            routing_role: RoutingRole::Leader,
            system_prompt_ref: "p/test.md".into(),
        });

        let catalog = CatalogRegistry::new(agents);
        assert_eq!(catalog.list_agents().len(), 1);
        assert_eq!(catalog.list_agents()[0].name, "test-agent");
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p omx-catalog`
Expected: ALL PASS (12 tests)

- [ ] **Step 3: Commit**

```bash
git add crates/omx-catalog/
git commit -m "feat(omx-catalog): implement CatalogRegistry with TOML discovery, validation, and topological sort"
```

---

## Task 6: Final verification — full workspace build and test

**Files:** None (verification only)

- [ ] **Step 1: Run full workspace check**

Run: `cargo check`
Expected: SUCCESS — all 25 crates compile

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: ALL PASS across entire workspace

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

- [ ] **Step 4: Fix any clippy warnings**

If clippy flags issues, fix them in the relevant crate files.

- [ ] **Step 5: Commit any clippy fixes**

```bash
git add -A
git commit -m "fix: address clippy warnings in Phase 6 crates"
```

(Skip this step if no clippy warnings.)

- [ ] **Step 6: Verify final crate count**

Run: `cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(len(json.load(sys.stdin)['packages']))"`
Expected: `25` (21 existing + 4 new)
