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

impl Default for DispatchQueue {
    fn default() -> Self {
        Self::new()
    }
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
