//! Tool infrastructure for sage-shell.
//!
//! All tool execution goes through `sage-tools` via the `ToolBridge`.
//! Types (ToolOutput, ToolInput, TodoState, etc.) come from `sage-tools` directly.

pub mod bridge;
pub mod config;
pub mod notification_bridge;
pub mod retry;
pub mod todo;
pub mod tool_context;

pub use self::{
    config::{BashToolConfig, FileToolset, ShellToolsetConfig},
    retry::{RetryConfig, execute_with_retry},
    tool_context::ToolContext,
};

// Re-export key types from sage-tools for convenience
pub use self::todo::{TodoId, TodoItem, TodoPriority, TodoStatus};
pub use sage_tools::types::output::ToolOutput;
pub use sage_tools::types::{MCPToolInput, ToolInput};
