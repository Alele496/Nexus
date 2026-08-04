use crate::implementations::nexus_build::task::types::SessionIdResource;
use crate::types::requirements::{Expr, ToolRequirement};
use crate::types::tool::{ToolKind, ToolNamespace};

use super::types::{MailboxCommand, MailboxHandle};

/// Tool name for check_mailbox.
pub const CHECK_MAILBOX_TOOL_NAME: &str = "check_mailbox";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckMailboxInput {
    /// Whether to mark all retrieved messages as read. Default: true.
    #[serde(
        default = "default_true",
        deserialize_with = "crate::types::schema::deserialize_lenient_bool"
    )]
    #[schemars(description = "Mark retrieved messages as read. Default: true")]
    pub mark_read: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckMailboxOutput {
    /// Number of unread messages found.
    pub count: usize,
    /// Retrieved messages, newest first.
    pub messages: Vec<MailboxMessageSummary>,
}

/// Summary of a mailbox message for tool output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailboxMessageSummary {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub body: String,
    pub priority: String,
    pub sent_at: String,
    pub is_unread: bool,
}

impl nexus_tool_runtime::ToolOutput for CheckMailboxOutput {}

#[derive(Debug, Default)]
pub struct CheckMailboxTool;

impl crate::types::tool_metadata::ToolMetadata for CheckMailboxTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Other
    }

    fn tool_namespace(&self) -> ToolNamespace {
        ToolNamespace::SageBuild
    }

    fn description_template(&self) -> &str {
        r#"Check your mailbox for new messages from other agents.

Returns all unread messages addressed to the current session. Use this when:
- You've been notified about new messages
- You want to check if other agents have sent you information
- At the start of a session to catch up on messages sent while you were offline

Use `send_message` to send messages to other agents."#
    }

    fn emitted_notifications(&self) -> &'static [&'static str] {
        &[]
    }

    fn requires_expr(&self) -> Expr<ToolRequirement> {
        Expr::True
    }
}

impl nexus_tool_runtime::Tool for CheckMailboxTool {
    type Args = CheckMailboxInput;
    type Output = CheckMailboxOutput;

    fn id(&self) -> nexus_tool_protocol::ToolId {
        nexus_tool_protocol::ToolId::new(CHECK_MAILBOX_TOOL_NAME).expect("valid tool id")
    }

    fn description(
        &self,
        _ctx: &nexus_tool_runtime::ListToolsContext,
    ) -> nexus_tool_types::ToolDescription {
        nexus_tool_types::ToolDescription::new(
            "check_mailbox",
            crate::types::tool_metadata::ToolMetadata::description_template(self),
        )
    }

    fn capabilities(&self) -> nexus_tool_protocol::ToolCapabilities {
        nexus_tool_protocol::ToolCapabilities {
            is_read_only: false, // may mark messages as read
            tool_scope: Some(nexus_tool_protocol::ToolScope::Write),
            ..Default::default()
        }
    }

    #[tracing::instrument(
        name = "tool.check_mailbox",
        skip_all,
    )]
    async fn run(
        &self,
        ctx: nexus_tool_runtime::ToolCallContext,
        input: CheckMailboxInput,
    ) -> Result<CheckMailboxOutput, nexus_tool_runtime::ToolError> {
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

        let session_id = {
            let res = resources.lock().await;
            res.get::<SessionIdResource>()
                .map(|r| r.0.clone())
                .unwrap_or_default()
        };

        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        sender
            .send(MailboxCommand::Check {
                session_id: session_id.clone(),
                reply: reply_tx,
            })
            .map_err(|_| {
                nexus_tool_runtime::ToolError::custom(
                    "process_manager",
                    "Mailbox actor stopped",
                )
            })?;

        let messages = reply_rx.await.map_err(|_| {
            nexus_tool_runtime::ToolError::custom(
                "process_manager",
                "Mailbox actor dropped reply",
            )
        })?;

        // Mark as read if requested — one batched update (a single fsync
        // on the shared store) rather than one command per message.
        if input.mark_read {
            let unread_ids: Vec<String> = messages
                .iter()
                .filter(|m| m.is_unread())
                .map(|m| m.id.clone())
                .collect();
            if !unread_ids.is_empty() {
                let (mark_tx, _) = tokio::sync::oneshot::channel();
                let _ = sender.send(MailboxCommand::MarkReadMany {
                    message_ids: unread_ids,
                    reply: mark_tx,
                });
                // Fire-and-forget: marking read is best-effort.
            }
        }

        let count = messages.len();
        let summaries: Vec<MailboxMessageSummary> = messages
            .into_iter()
            .map(|m| {
                let is_unread = m.is_unread();
                let from_display = m.from.display_name().to_string();
                let sent_at = m.sent_at.to_rfc3339();
                MailboxMessageSummary {
                    id: m.id,
                    from: from_display,
                    subject: m.subject,
                    body: m.body,
                    priority: m.priority,
                    sent_at,
                    is_unread,
                }
            })
            .collect();

        Ok(CheckMailboxOutput {
            count,
            messages: summaries,
        })
    }
}
