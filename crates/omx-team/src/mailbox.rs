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
    use super::*;

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
        let m1 = create_message(&WorkerId("a".into()), &WorkerId("b".into()), "msg1").unwrap();
        let m2 = create_message(&WorkerId("a".into()), &WorkerId("b".into()), "msg2").unwrap();
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
        let mut msg = create_message(&WorkerId("a".into()), &WorkerId("b".into()), "test").unwrap();
        assert!(!msg.delivered);
        mark_delivered(&mut msg);
        assert!(msg.delivered);
    }
}
