//! Mailbox ext methods.
//!
//! The mailbox is a cross-process, file-backed store (`<nexus-home>/mailbox.json`)
//! shared by every nexus process. These methods expose it to UI clients that
//! have no tool-runtime access to the store: list the shared state and mark a
//! message read. Both are cheap, lock-guarded reads/writes against
//! `MailboxStore` — no session actor involvement.

use agent_client_protocol as acp;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::session::ExtMethodResult;
use nexus_tools::implementations::nexus_build::mailbox::store::MailboxStore;
use nexus_tools::implementations::nexus_build::mailbox::types::{MailboxMessage, MessageStatus};

/// Router for `sage.local/mailbox/*` methods.
pub async fn handle(args: &acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
    match args.method.as_ref() {
        "sage.local/mailbox/list" => handle_list(args).await,
        "sage.local/mailbox/mark_read" => handle_mark_read(args).await,
        _ => Err(acp::Error::method_not_found()),
    }
}

// ---- Wire forms ----
//
// `MailboxMessage` serializes its top level camelCase, but the nested
// `AgentAddress` (from/to) leaks snake_case (`session_id`) onto the wire —
// that type deliberately has no serde rename because the shared mailbox file
// format must stay stable. UI clients expect camelCase everywhere (the ACP
// convention), so the list response is rebuilt with camelCase addresses.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireAddress {
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireMessage {
    id: String,
    from: WireAddress,
    to: WireAddress,
    subject: String,
    body: String,
    priority: String,
    status: MessageStatus,
    sent_at: DateTime<Utc>,
    read_at: Option<DateTime<Utc>>,
    in_reply_to: Option<String>,
    expires_at: Option<DateTime<Utc>>,
}

impl From<&MailboxMessage> for WireMessage {
    fn from(m: &MailboxMessage) -> Self {
        Self {
            id: m.id.clone(),
            from: WireAddress {
                session_id: m.from.session_id.clone(),
                label: m.from.label.clone(),
            },
            to: WireAddress {
                session_id: m.to.session_id.clone(),
                label: m.to.label.clone(),
            },
            subject: m.subject.clone(),
            body: m.body.clone(),
            priority: m.priority.clone(),
            status: m.status,
            sent_at: m.sent_at,
            read_at: m.read_at,
            in_reply_to: m.in_reply_to.clone(),
            expires_at: m.expires_at,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireMailboxState {
    messages: Vec<WireMessage>,
    labels: HashMap<String, String>,
}

/// `sage.local/mailbox/list` — the full shared mailbox state (all messages,
/// newest first, plus the label → session_id registry), serialized camelCase.
async fn handle_list(args: &acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
    let store = MailboxStore::at_nexus_home();
    let state = store.load();
    let wire = WireMailboxState {
        messages: state.messages.iter().map(WireMessage::from).collect(),
        labels: state.labels,
    };
    ExtMethodResult::success(wire)
        .to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkReadRequest {
    message_id: String,
}

/// `sage.local/mailbox/mark_read` — mark one message read in the shared store.
/// Returns `{ ok: bool }`; `ok` is false when the message id is unknown.
async fn handle_mark_read(args: &acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
    let req: MarkReadRequest = serde_json::from_str(args.params.get())
        .map_err(|e| acp::Error::invalid_params().data(format!("invalid params: {e}")))?;
    let store = MailboxStore::at_nexus_home();
    let marked = store
        .update(|s| match s.find_mut(&req.message_id) {
            Some(m) => {
                m.mark_read();
                true
            }
            None => false,
        })
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;
    ExtMethodResult::success(serde_json::json!({ "ok": marked }))
        .to_ext_response()
        .map_err(|e| acp::Error::internal_error().data(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_tools::implementations::nexus_build::mailbox::types::AgentAddress;

    /// Regression guard: the mailbox type serializes the nested from/to
    /// addresses as snake_case (`session_id`), which the old handler leaked
    /// straight onto the wire — the UI expected `sessionId` and crashed on
    /// real data. The wire form must be camelCase with null labels omitted.
    #[test]
    fn wire_message_uses_camel_case_addresses() {
        let msg = MailboxMessage::new(
            AgentAddress::with_label("sender-id", "sender-label"),
            AgentAddress::new("recv-id"),
            "subject".into(),
            "body".into(),
            "normal".into(),
        );
        let wire = serde_json::to_value(WireMessage::from(&msg)).unwrap();
        assert_eq!(wire["from"]["sessionId"], "sender-id");
        assert_eq!(wire["from"]["label"], "sender-label");
        assert_eq!(wire["to"]["sessionId"], "recv-id");
        assert!(
            wire["to"].get("label").is_none(),
            "null label must be omitted from the wire, not sent as null",
        );
        assert_eq!(wire["status"], "pending");
        assert!(wire.get("sentAt").is_some(), "sent_at must serialize as sentAt");
    }
}
