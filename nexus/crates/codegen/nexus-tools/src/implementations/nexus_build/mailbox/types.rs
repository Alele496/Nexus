//! Agent Mailbox — persistent inter-agent messaging.
//!
//! Agents send messages to each other asynchronously, even across sessions.
//! Messages are persisted via the Resources system (JSON serialization).
//!
//! Architecture:
//!   Agent A → send_message(to="agent-b", ...) → MailboxStore (persisted)
//!   Agent B → check_mailbox() → retrieves pending messages
//!   Leader → pushes new-message notifications to active recipient sessions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

/// Unique address for an agent in the mailbox system.
///
/// An agent's address is its session ID — each session is a unique mailbox.
/// For human-readable addressing, the `label` field can be used as an alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentAddress {
    /// Machine-readable unique address (session ID).
    pub session_id: String,
    /// Human-readable label set by the user (e.g. "gateway-service").
    pub label: Option<String>,
}

impl AgentAddress {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            label: None,
        }
    }

    pub fn with_label(session_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            label: Some(label.into()),
        }
    }

    /// Display form: label if set, otherwise truncated session ID.
    pub fn display_name(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.session_id)
    }
}

/// A single message in the mailbox system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailboxMessage {
    /// Unique message ID.
    pub id: String,
    /// Sender address.
    pub from: AgentAddress,
    /// Recipient address.
    pub to: AgentAddress,
    /// Message subject (one-line summary).
    pub subject: String,
    /// Message body.
    pub body: String,
    /// Priority: "normal" or "urgent".
    #[serde(default = "default_priority")]
    pub priority: String,
    /// Delivery status.
    pub status: MessageStatus,
    /// When the message was sent.
    pub sent_at: DateTime<Utc>,
    /// When the message was read (None if unread).
    pub read_at: Option<DateTime<Utc>>,
    /// Optional reply-to message ID for threading.
    pub in_reply_to: Option<String>,
    /// Time-to-live. Messages older than this may be auto-archived.
    /// None = no expiry.
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_priority() -> String {
    "normal".to_string()
}

/// Delivery/read status of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    /// Message has been sent but not yet delivered (recipient offline).
    Pending,
    /// Message has been delivered to the recipient's mailbox.
    Delivered,
    /// Message has been read by the recipient.
    Read,
    /// Message has been archived.
    Archived,
}

impl MailboxMessage {
    pub fn new(
        from: AgentAddress,
        to: AgentAddress,
        subject: String,
        body: String,
        priority: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string().replace('-', "")[..12].to_string(),
            from,
            to,
            subject,
            body,
            priority,
            status: MessageStatus::Pending,
            sent_at: Utc::now(),
            read_at: None,
            in_reply_to: None,
            expires_at: None,
        }
    }

    /// Mark this message as read.
    pub fn mark_read(&mut self) {
        self.status = MessageStatus::Read;
        self.read_at = Some(Utc::now());
    }

    /// Mark this message as delivered.
    pub fn mark_delivered(&mut self) {
        if self.status == MessageStatus::Pending {
            self.status = MessageStatus::Delivered;
        }
    }

    /// Whether this message is unread.
    pub fn is_unread(&self) -> bool {
        matches!(self.status, MessageStatus::Pending | MessageStatus::Delivered)
    }
}

/// Persisted mailbox state, stored via Resources + ResourcesPersistence.
/// Contains all non-archived messages across all agents and the label registry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MailboxState {
    /// All active (non-archived) messages, newest first.
    pub messages: Vec<MailboxMessage>,
    /// Label → session_id registry for human-readable addressing.
    pub labels: std::collections::HashMap<String, String>,
}

impl MailboxState {
    /// Messages addressed to a specific session, newest first, unread only.
    pub fn inbox_for(&self, session_id: &str) -> Vec<&MailboxMessage> {
        self.messages
            .iter()
            .filter(|m| m.to.session_id == session_id && m.is_unread())
            .collect()
    }

    /// All messages from a sender, newest first.
    pub fn sent_by(&self, session_id: &str) -> Vec<&MailboxMessage> {
        self.messages
            .iter()
            .filter(|m| m.from.session_id == session_id)
            .collect()
    }

    /// Find a message by ID.
    pub fn find(&self, id: &str) -> Option<&MailboxMessage> {
        self.messages.iter().find(|m| m.id == id)
    }

    /// Find a message by ID (mutable).
    pub fn find_mut(&mut self, id: &str) -> Option<&mut MailboxMessage> {
        self.messages.iter_mut().find(|m| m.id == id)
    }

    /// Insert a new message at the front (newest first).
    pub fn insert(&mut self, message: MailboxMessage) {
        self.messages.insert(0, message);
    }

    /// Archive old messages beyond the retention limit (default: 1000 most recent).
    pub fn prune(&mut self, max_messages: usize) {
        if self.messages.len() > max_messages {
            self.messages.truncate(max_messages);
        }
    }

    /// Register a label → session_id mapping for human-readable addressing.
    pub fn register_label(&mut self, session_id: String, label: String) {
        self.labels.insert(label, session_id);
    }

    /// Resolve a label to its session_id.
    pub fn resolve_label(&self, label: &str) -> Option<&String> {
        self.labels.get(label)
    }
}

crate::register_resource!("nexus_build", "Mailbox", MailboxState);

/// Ephemeral handle for tools to communicate with the MailboxActor.
/// Inserted via `resources.insert()`, not persisted.
#[derive(Clone)]
pub struct MailboxHandle(pub mpsc::UnboundedSender<MailboxCommand>);

/// Commands that tools can send to the MailboxActor.
pub enum MailboxCommand {
    /// Send a new message.
    Send {
        message: MailboxMessage,
        reply: oneshot::Sender<Result<MailboxMessage, MailboxError>>,
    },
    /// Check mailbox for a session.
    Check {
        session_id: String,
        reply: oneshot::Sender<Vec<MailboxMessage>>,
    },
    /// Mark a specific message as read.
    MarkRead {
        message_id: String,
        reply: oneshot::Sender<bool>,
    },
    /// List sent messages for a session.
    SentItems {
        session_id: String,
        reply: oneshot::Sender<Vec<MailboxMessage>>,
    },
    /// Archive old messages.
    Prune {
        max_messages: usize,
        reply: oneshot::Sender<()>,
    },
    /// Resolve a label to a session_id (for human-readable addressing).
    ResolveLabel {
        label: String,
        reply: oneshot::Sender<Option<String>>,
    },
    /// Register a label for a session.
    RegisterLabel {
        session_id: String,
        label: String,
        reply: oneshot::Sender<()>,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum MailboxError {
    #[error("recipient address not found: {0}")]
    AddressNotFound(String),
    #[error("mailbox store error: {0}")]
    StoreError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_message_has_pending_status() {
        let from = AgentAddress::new("session-a");
        let to = AgentAddress::new("session-b");
        let msg = MailboxMessage::new(
            from,
            to,
            "Test".into(),
            "Body".into(),
            "normal".into(),
        );
        assert_eq!(msg.status, MessageStatus::Pending);
        assert!(msg.is_unread());
        assert_eq!(msg.id.len(), 12);
    }

    #[test]
    fn mark_read_updates_status() {
        let mut msg = MailboxMessage::new(
            AgentAddress::new("a"),
            AgentAddress::new("b"),
            "S".into(),
            "B".into(),
            "normal".into(),
        );
        msg.mark_read();
        assert_eq!(msg.status, MessageStatus::Read);
        assert!(!msg.is_unread());
        assert!(msg.read_at.is_some());
    }

    #[test]
    fn inbox_filters_by_session() {
        let mut state = MailboxState::default();
        let from = AgentAddress::new("sender");
        let to_a = AgentAddress::new("session-a");
        let to_b = AgentAddress::new("session-b");

        state.insert(MailboxMessage::new(
            from.clone(), to_a.clone(), "For A".into(), "a".into(), "normal".into(),
        ));
        state.insert(MailboxMessage::new(
            from.clone(), to_b.clone(), "For B".into(), "b".into(), "normal".into(),
        ));
        state.insert(MailboxMessage::new(
            from.clone(), to_a.clone(), "For A again".into(), "aa".into(), "normal".into(),
        ));

        let inbox_a = state.inbox_for("session-a");
        assert_eq!(inbox_a.len(), 2);
        let inbox_b = state.inbox_for("session-b");
        assert_eq!(inbox_b.len(), 1);
        // No messages for unknown session.
        assert!(state.inbox_for("unknown").is_empty());
    }

    #[test]
    fn sent_by_filters_by_sender() {
        let mut state = MailboxState::default();
        let from = AgentAddress::new("sender");
        let other = AgentAddress::new("other");

        state.insert(MailboxMessage::new(
            from.clone(), AgentAddress::new("r1"), "1".into(), "b".into(), "normal".into(),
        ));
        state.insert(MailboxMessage::new(
            other.clone(), AgentAddress::new("r2"), "2".into(), "b".into(), "normal".into(),
        ));

        assert_eq!(state.sent_by("sender").len(), 1);
        assert_eq!(state.sent_by("other").len(), 1);
        assert!(state.sent_by("nobody").is_empty());
    }

    #[test]
    fn agent_address_display_name_prefers_label() {
        let a = AgentAddress::with_label("sid-1", "gateway-service");
        assert_eq!(a.display_name(), "gateway-service");

        let b = AgentAddress::new("sid-2");
        assert_eq!(b.display_name(), "sid-2");
    }

    #[test]
    fn messages_are_inserted_newest_first() {
        let mut state = MailboxState::default();
        let from = AgentAddress::new("s");
        let to = AgentAddress::new("r");

        let mut msg1 = MailboxMessage::new(from.clone(), to.clone(), "old".into(), "b".into(), "normal".into());
        msg1.sent_at = msg1.sent_at - chrono::Duration::hours(1);
        let msg2 = MailboxMessage::new(from.clone(), to.clone(), "new".into(), "b".into(), "normal".into());

        state.insert(msg1);
        state.insert(msg2);

        // newest first
        assert_eq!(state.messages[0].subject, "new");
        assert_eq!(state.messages[1].subject, "old");
    }
}
