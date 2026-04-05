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
