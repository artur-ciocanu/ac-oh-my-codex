//! Agent type definitions and registry.

use omx_types::OmxError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Deserialize)]
struct AgentTomlFile {
    #[serde(default)]
    agents: Vec<AgentDef>,
}

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
        self.agents
            .iter()
            .filter(|a| a.posture == posture)
            .collect()
    }
    pub fn filter_by_model_class(&self, model_class: ModelClass) -> Vec<&AgentDef> {
        self.agents
            .iter()
            .filter(|a| a.model_class == model_class)
            .collect()
    }
    pub fn filter_by_role(&self, role: RoutingRole) -> Vec<&AgentDef> {
        self.agents
            .iter()
            .filter(|a| a.routing_role == role)
            .collect()
    }
    pub fn all(&self) -> &[AgentDef] {
        &self.agents
    }

    pub fn validate(&self) -> Result<(), OmxError> {
        let mut seen = HashMap::new();
        for agent in &self.agents {
            if seen.insert(&agent.name, true).is_some() {
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
