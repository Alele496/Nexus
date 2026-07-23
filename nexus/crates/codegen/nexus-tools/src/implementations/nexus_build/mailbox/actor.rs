use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::types::resources::{SharedResources, State};

use super::types::{MailboxCommand, MailboxState};

const MAX_MAILBOX_MESSAGES: usize = 1000;

pub struct MailboxActor {
    pub(crate) resources: SharedResources,
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
                let mut res = self.resources.lock().await;
                let state = res.get_or_default::<State<MailboxState>>();
                state.insert(message.clone());
                state.prune(MAX_MAILBOX_MESSAGES);
                drop(res);
                let _ = reply.send(Ok(message));
            }
            MailboxCommand::Check { session_id, reply } => {
                let res = self.resources.lock().await;
                let messages = res
                    .get::<State<MailboxState>>()
                    .map(|s| {
                        s.messages
                            .iter()
                            .filter(|m| m.to.session_id == session_id && m.is_unread())
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                let _ = reply.send(messages);
            }
            MailboxCommand::MarkRead { message_id, reply } => {
                let mut res = self.resources.lock().await;
                let found = res
                    .get_or_default::<State<MailboxState>>()
                    .find_mut(&message_id)
                    .map(|m| {
                        m.mark_read();
                        true
                    })
                    .unwrap_or(false);
                let _ = reply.send(found);
            }
            MailboxCommand::SentItems { session_id, reply } => {
                let res = self.resources.lock().await;
                let messages = res
                    .get::<State<MailboxState>>()
                    .map(|s| {
                        s.messages
                            .iter()
                            .filter(|m| m.from.session_id == session_id)
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                let _ = reply.send(messages);
            }
            MailboxCommand::Prune { max_messages, reply } => {
                let mut res = self.resources.lock().await;
                res.get_or_default::<State<MailboxState>>()
                    .prune(max_messages);
                let _ = reply.send(());
            }
            MailboxCommand::ResolveLabel { label, reply } => {
                let res = self.resources.lock().await;
                let session_id = res
                    .get::<State<MailboxState>>()
                    .and_then(|s| s.resolve_label(&label).cloned());
                let _ = reply.send(session_id);
            }
            MailboxCommand::RegisterLabel {
                session_id,
                label,
                reply,
            } => {
                let mut res = self.resources.lock().await;
                res.get_or_default::<State<MailboxState>>()
                    .register_label(session_id, label);
                let _ = reply.send(());
            }
        }
    }
}
