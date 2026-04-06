use std::collections::HashMap;

/// Maps logical worker IDs to tmux pane IDs, enabling bidirectional lookup
/// between the orchestration layer's worker identifiers and the actual tmux
/// pane handles used by `TmuxAdapter`.
#[derive(Debug, Clone, Default)]
pub struct PaneRegistry {
    map: HashMap<String, String>,
}

impl PaneRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update the mapping from `worker_id` to `pane_id`.
    pub fn register(&mut self, worker_id: &str, pane_id: &str) {
        self.map.insert(worker_id.to_owned(), pane_id.to_owned());
    }

    /// Find the pane ID associated with a worker.
    pub fn lookup(&self, worker_id: &str) -> Option<&str> {
        self.map.get(worker_id).map(String::as_str)
    }

    /// Find the worker ID that owns a given pane.
    pub fn reverse_lookup(&self, pane_id: &str) -> Option<&str> {
        self.map
            .iter()
            .find(|(_, v)| v.as_str() == pane_id)
            .map(|(k, _)| k.as_str())
    }

    /// Remove the mapping for `worker_id`.
    pub fn unregister(&mut self, worker_id: &str) {
        self.map.remove(worker_id);
    }

    /// Return all (worker_id, pane_id) pairs.
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.map
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup() {
        let mut reg = PaneRegistry::new();
        reg.register("worker-1", "%5");
        assert_eq!(reg.lookup("worker-1"), Some("%5"));
    }

    #[test]
    fn lookup_missing_returns_none() {
        let reg = PaneRegistry::new();
        assert_eq!(reg.lookup("ghost"), None);
    }

    #[test]
    fn unregister_removes_entry() {
        let mut reg = PaneRegistry::new();
        reg.register("worker-1", "%5");
        reg.unregister("worker-1");
        assert_eq!(reg.lookup("worker-1"), None);
    }

    #[test]
    fn list_all_entries() {
        let mut reg = PaneRegistry::new();
        reg.register("w1", "%1");
        reg.register("w2", "%2");

        let mut entries = reg.list();
        entries.sort();
        assert_eq!(entries, vec![("w1", "%1"), ("w2", "%2")]);
    }

    #[test]
    fn update_existing_entry() {
        let mut reg = PaneRegistry::new();
        reg.register("worker-1", "%5");
        reg.register("worker-1", "%9");
        assert_eq!(reg.lookup("worker-1"), Some("%9"));
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn reverse_lookup_by_pane_id() {
        let mut reg = PaneRegistry::new();
        reg.register("worker-1", "%5");
        reg.register("worker-2", "%8");
        assert_eq!(reg.reverse_lookup("%8"), Some("worker-2"));
        assert_eq!(reg.reverse_lookup("%99"), None);
    }
}
