//! Mailbox ext methods.
//!
//! The mailbox is a cross-process, file-backed store (`<nexus-home>/mailbox.json`)
//! shared by every nexus process. These methods expose it to UI clients that
//! have no tool-runtime access to the store: list the shared state and mark a
//! message read. Both are cheap, lock-guarded reads/writes against
//! `MailboxStore` — no session actor involvement.

use agent_client_protocol as acp;
use serde::Deserialize;

use crate::session::ExtMethodResult;
use nexus_tools::implementations::nexus_build::mailbox::store::MailboxStore;

/// Router for `sage.local/mailbox/*` methods.
pub async fn handle(args: &acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
    match args.method.as_ref() {
        "sage.local/mailbox/list" => handle_list(args).await,
        "sage.local/mailbox/mark_read" => handle_mark_read(args).await,
        _ => Err(acp::Error::method_not_found()),
    }
}

/// `sage.local/mailbox/list` — the full shared mailbox state (all messages,
/// newest first, plus the label → session_id registry). The `MailboxMessage`
/// wire shape is camelCase via serde, so the state serializes directly.
async fn handle_list(args: &acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
    let store = MailboxStore::at_nexus_home();
    let state = store.load();
    ExtMethodResult::success(state)
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
