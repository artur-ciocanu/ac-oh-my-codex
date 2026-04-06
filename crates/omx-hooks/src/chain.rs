use serde::{Deserialize, Serialize};

/// A single entry in a hook chain, with a name and execution priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    pub name: String,
    pub priority: u32,
}

/// An ordered hook chain that sorts entries by priority (ascending — lower runs first).
#[derive(Debug, Clone)]
pub struct HookChain {
    entries: Vec<ChainEntry>,
}

impl HookChain {
    /// Create a new `HookChain`, sorting entries by priority ascending.
    pub fn new(mut entries: Vec<ChainEntry>) -> Self {
        entries.sort_by_key(|e| e.priority);
        Self { entries }
    }

    /// Returns the sorted entries.
    pub fn ordered(&self) -> &[ChainEntry] {
        &self.entries
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the chain has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_preserves_order_when_already_sorted() {
        let entries = vec![
            ChainEntry {
                name: "first".into(),
                priority: 1,
            },
            ChainEntry {
                name: "second".into(),
                priority: 2,
            },
            ChainEntry {
                name: "third".into(),
                priority: 3,
            },
        ];
        let chain = HookChain::new(entries);
        let names: Vec<&str> = chain.ordered().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn chain_sorts_by_priority() {
        let entries = vec![
            ChainEntry {
                name: "high".into(),
                priority: 100,
            },
            ChainEntry {
                name: "low".into(),
                priority: 1,
            },
            ChainEntry {
                name: "mid".into(),
                priority: 50,
            },
        ];
        let chain = HookChain::new(entries);
        let names: Vec<&str> = chain.ordered().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["low", "mid", "high"]);
    }

    #[test]
    fn empty_chain() {
        let chain = HookChain::new(vec![]);
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert!(chain.ordered().is_empty());
    }

    #[test]
    fn chain_length() {
        let entries = vec![
            ChainEntry {
                name: "a".into(),
                priority: 10,
            },
            ChainEntry {
                name: "b".into(),
                priority: 20,
            },
        ];
        let chain = HookChain::new(entries);
        assert_eq!(chain.len(), 2);
        assert!(!chain.is_empty());
    }
}
