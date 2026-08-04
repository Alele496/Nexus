use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::store::MailboxStore;
use super::types::{MailboxCommand, MailboxMessage, MailboxState};

const MAX_MAILBOX_MESSAGES: usize = 1000;

/// Background actor owning the mailbox.
///
/// All state lives in the shared [`MailboxStore`] (a file under the Nexus
/// home), so messages sent from one nexus process are visible to every
/// other process — the file, not process memory, is the source of truth.
pub struct MailboxActor {
    pub(crate) store: MailboxStore,
    pub(crate) cmd_rx: mpsc::UnboundedReceiver<MailboxCommand>,
    pub(crate) cancel_token: CancellationToken,
}

impl MailboxActor {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                biased;

                _ = self.cancel_token.cancelled() => {
                    tracing::debug!("MailboxActor shutting down (cancelled)");
                    break;
                }

                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle_command(cmd).await;
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: MailboxCommand) {
        match cmd {
            MailboxCommand::Send { message, reply } => {
                // Persisting into the shared store IS the delivery: once
                // the message is on disk, the recipient's process can read
                // it. Reflect that in the stored status so `send_message`
                // reports a truthful "delivered" instead of a hardcoded
                // "pending".
                let mut message = message;
                if message.status == super::types::MessageStatus::Pending {
                    message.mark_delivered();
                }
                match self.store.update(|state: &mut MailboxState| {
                    state.insert(message.clone());
                    state.prune(MAX_MAILBOX_MESSAGES);
                }) {
                    Ok(()) => {
                        let _ = reply.send(Ok(message));
                    }
                    Err(error) => {
                        // A failed persist means the message never reached
                        // the shared mailbox — report that truthfully
                        // instead of claiming delivery.
                        tracing::warn!(error = ?error, "mailbox: failed to persist send");
                        let _ = reply.send(Err(super::types::MailboxError::StoreError(
                            error.to_string(),
                        )));
                    }
                }
            }
            MailboxCommand::Check { session_id, reply } => {
                let messages: Vec<MailboxMessage> = self
                    .store
                    .load()
                    .inbox_for(&session_id)
                    .into_iter()
                    .cloned()
                    .collect();
                let _ = reply.send(messages);
            }
            MailboxCommand::MarkRead { message_id, reply } => {
                let found = self
                    .store
                    .update(|state| {
                        state
                            .find_mut(&message_id)
                            .map(|m| {
                                m.mark_read();
                                true
                            })
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                let _ = reply.send(found);
            }
            MailboxCommand::MarkReadMany { message_ids, reply } => {
                // Batch: one read-modify-write for all ids (one fsync
                // instead of one per message).
                let marked = self
                    .store
                    .update(|state| {
                        message_ids
                            .iter()
                            .filter(|id| {
                                state
                                    .find_mut(id)
                                    .map(|m| {
                                        m.mark_read();
                                        true
                                    })
                                    .unwrap_or(false)
                            })
                            .count()
                    })
                    .unwrap_or(0);
                let _ = reply.send(marked);
            }
            MailboxCommand::SentItems { session_id, reply } => {
                let messages: Vec<MailboxMessage> = self
                    .store
                    .load()
                    .sent_by(&session_id)
                    .into_iter()
                    .cloned()
                    .collect();
                let _ = reply.send(messages);
            }
            MailboxCommand::Prune { max_messages, reply } => {
                let _ = self
                    .store
                    .update(|state| state.prune(max_messages));
                let _ = reply.send(());
            }
            MailboxCommand::ResolveLabel { label, reply } => {
                let session_id = self.store.load().resolve_label(&label).cloned();
                let _ = reply.send(session_id);
            }
            MailboxCommand::RegisterLabel {
                session_id,
                label,
                reply,
            } => {
                let _ = self
                    .store
                    .update(|state| state.register_label(session_id, label));
                let _ = reply.send(());
            }
        }
    }
}
