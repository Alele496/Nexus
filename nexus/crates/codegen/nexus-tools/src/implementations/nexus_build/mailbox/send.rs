use crate::implementations::nexus_build::task::types::SessionIdResource;
use crate::types::requirements::{Expr, ToolRequirement};
use crate::types::tool::{ToolKind, ToolNamespace};

use super::types::{AgentAddress, MailboxCommand, MailboxHandle, MailboxMessage};

/// Tool name for send_message.
pub const SEND_MESSAGE_TOOL_NAME: &str = "send_message";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageInput {
    /// Recipient address. Can be a session ID or a registered label.
    #[schemars(description = "Recipient agent address — session ID or registered label")]
    pub to: String,

    /// One-line summary of the message.
    #[schemars(description = "One-line message subject")]
    pub subject: String,

    /// Full message body.
    #[schemars(description = "Message body content")]
    pub body: String,

    /// Priority: "normal" (default) or "urgent".
    #[serde(default = "default_priority")]
    #[schemars(description = "Message priority: \"normal\" or \"urgent\"")]
    pub priority: String,

    /// Optional human-readable label for the sender.
    #[serde(default)]
    #[schemars(description = "Optional label to register for your own address")]
    pub sender_label: Option<String>,
}

fn default_priority() -> String {
    "normal".to_string()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageOutput {
    /// The sent message ID.
    pub message_id: String,
    /// Recipient display name.
    pub to: String,
    /// Delivery status.
    pub status: String,
}

impl nexus_tool_runtime::ToolOutput for SendMessageOutput {}

#[derive(Debug, Default)]
pub struct SendMessageTool;

impl crate::types::tool_metadata::ToolMetadata for SendMessageTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Other
    }

    fn tool_namespace(&self) -> ToolNamespace {
        ToolNamespace::SageBuild
    }

    fn description_template(&self) -> &str {
        r#"Send a message to another agent. The message is delivered asynchronously —
the recipient can check their mailbox with `check_mailbox` at any time,
even in a different session.

Use cases:
- Notify another agent that a task is complete
- Pass structured data between agents working on related tasks
- Coordinate cross-service changes without the user relaying information

The recipient address (`to`) can be:
- A session ID (visible in /agent-graph or /dashboard)
- A registered label (set with sender_label or via /mailbox register)"#
    }

    fn emitted_notifications(&self) -> &'static [&'static str] {
        &["MailboxMessageSent"]
    }

    fn requires_expr(&self) -> Expr<ToolRequirement> {
        Expr::True
    }
}

impl nexus_tool_runtime::Tool for SendMessageTool {
    type Args = SendMessageInput;
    type Output = SendMessageOutput;

    fn id(&self) -> nexus_tool_protocol::ToolId {
        nexus_tool_protocol::ToolId::new(SEND_MESSAGE_TOOL_NAME).expect("valid tool id")
    }

    fn description(
        &self,
        _ctx: &nexus_tool_runtime::ListToolsContext,
    ) -> nexus_tool_types::ToolDescription {
        nexus_tool_types::ToolDescription::new(
            "send_message",
            crate::types::tool_metadata::ToolMetadata::description_template(self),
        )
    }

    fn capabilities(&self) -> nexus_tool_protocol::ToolCapabilities {
        nexus_tool_protocol::ToolCapabilities {
            is_read_only: false,
            tool_scope: Some(nexus_tool_protocol::ToolScope::Write),
            ..Default::default()
        }
    }

    #[tracing::instrument(
        name = "tool.send_message",
        skip_all,
        fields(to = %input.to, subject = %input.subject)
    )]
    async fn run(
        &self,
        ctx: nexus_tool_runtime::ToolCallContext,
        input: SendMessageInput,
    ) -> Result<SendMessageOutput, nexus_tool_runtime::ToolError> {
        let resources = crate::types::tool_metadata::shared_resources(&ctx)?;

        let sender = {
            let res = resources.lock().await;
            res.get::<MailboxHandle>()
                .ok_or_else(|| {
                    nexus_tool_runtime::ToolError::custom("missing_resource", "MailboxHandle")
                })?
                .0
                .clone()
        };

        // Resolve label to session_id if needed.
        let resolve_sender = {
            let res = resources.lock().await;
            res.get::<MailboxHandle>()
                .map(|h| h.0.clone())
        };

        let from_session_id = {
            let res = resources.lock().await;
            res.get::<SessionIdResource>()
                .map(|r| r.0.clone())
                .unwrap_or_default()
        };

        let from = if let Some(ref label) = input.sender_label {
            // Register the label for our own session.
            if let Some(ref s) = resolve_sender {
                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                let _ = s.send(MailboxCommand::RegisterLabel {
                    session_id: from_session_id.clone(),
                    label: label.clone(),
                    reply: reply_tx,
                });
                let _ = reply_rx.await;
            }
            AgentAddress::with_label(from_session_id, label.clone())
        } else {
            AgentAddress::new(from_session_id)
        };

        // Resolve the recipient: `to` may name a registered label (e.g.
        // "build-agent") instead of a session ID. Deliver to the label's
        // real session ID so the message lands in the right inbox —
        // storing the label verbatim in `session_id` (the old behaviour)
        // made it unreachable via `inbox_for`.
        //
        // ID-shaped recipients are always addressed by ID and never look
        // up the label registry: a (possibly legacy, hostile) label entry
        // keyed by someone's session ID must not be able to redirect their
        // mail. Unregistered labels, and any actor failure here (the real
        // Send below surfaces that anyway), fall through and are treated
        // as session IDs.
        let to = if super::types::looks_like_session_id(&input.to) {
            AgentAddress::new(&input.to)
        } else {
            let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
            let _ = sender.send(MailboxCommand::ResolveLabel {
                label: input.to.clone(),
                reply: reply_tx,
            });
            match reply_rx.await.ok().flatten() {
                Some(session_id) => AgentAddress::with_label(session_id, input.to.clone()),
                None => AgentAddress::new(&input.to),
            }
        };
        let message = MailboxMessage::new(
            from.clone(),
            to.clone(),
            input.subject,
            input.body,
            input.priority,
        );

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        sender
            .send(MailboxCommand::Send {
                message,
                reply: reply_tx,
            })
            .map_err(|_| {
                nexus_tool_runtime::ToolError::custom(
                    "process_manager",
                    "Mailbox actor stopped",
                )
            })?;

        let sent = reply_rx.await.map_err(|_| {
            nexus_tool_runtime::ToolError::custom(
                "process_manager",
                "Mailbox actor dropped reply",
            )
        })?
        .map_err(|error| match error {
            super::types::MailboxError::AddressNotFound(addr) => {
                nexus_tool_runtime::ToolError::invalid_arguments(format!(
                    "recipient address not found: {addr}"
                ))
            }
            super::types::MailboxError::StoreError(message) => {
                nexus_tool_runtime::ToolError::custom("mailbox_store", message)
            }
        })?;

        // The actor persists the message into the shared store and marks it
        // delivered; report the real status rather than a hardcoded
        // "pending" (which previously implied delivery that never happened).
        let status = match sent.status {
            super::types::MessageStatus::Pending => "pending",
            super::types::MessageStatus::Delivered => "delivered",
            super::types::MessageStatus::Read => "read",
            super::types::MessageStatus::Archived => "archived",
        };

        Ok(SendMessageOutput {
            message_id: sent.id,
            to: sent.to.display_name().to_string(),
            status: status.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_message_input_defaults() {
        let input: SendMessageInput = serde_json::from_str(
            r#"{"to": "agent-b", "subject": "hi", "body": "hello"}"#,
        )
        .unwrap();
        assert_eq!(input.priority, "normal");
        assert!(input.sender_label.is_none());
    }
}
