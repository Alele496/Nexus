//! Independent verification layer — Reviewer agent audits Worker output.
//!
//! A "reviewer" is a separate agent (ideally with a different model) that
//! critically examines another agent's work. It looks for bugs, style issues,
//! and logic concerns, then produces a structured verdict.
//!
//! ## Design
//!
//! - **Manual trigger**: `/review <agent-name>` spawns a reviewer fork of
//!   the target agent with a review-focused system prompt. The user can
//!   interact with the reviewer directly or let it run autonomously.
//! - **Auto-trigger**: When `ReviewConfig.enabled` is `true` and a Worker
//!   subagent completes, a reviewer is automatically spawned.
//! - **Config**: `ReviewConfig` lives on `AppView`, toggled via
//!   `/review --on` / `/review --off`. Defaults to **disabled** to avoid
//!   unexpected token costs.

use crate::app::agent::AgentId;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// User-facing review configuration.
///
/// Stored on [`AppView`](crate::app::app_view::AppView). Defaults to
/// disabled so users opt in consciously — reviews consume extra tokens.
#[derive(Debug, Clone)]
pub struct ReviewConfig {
    /// Whether auto-review is enabled. Default: `false`.
    pub enabled: bool,
    /// Model override for the reviewer agent. `None` means "inherit parent."
    /// Example: `"claude-sonnet-4-6"`.
    pub model: Option<String>,
    /// Review depth: light (diff only, faster) or full (full context, more
    /// thorough). Default: `Light`.
    pub mode: ReviewMode,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: None,
            mode: ReviewMode::Light,
        }
    }
}

/// How thorough the reviewer should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewMode {
    /// Quick scan: look at modified files and their diffs.
    Light,
    /// Deep analysis: full project context + modified files + conversation.
    Full,
}

// ---------------------------------------------------------------------------
// Verdict
// ---------------------------------------------------------------------------

/// Structured outcome of a code review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewVerdict {
    /// Code looks good, no issues found.
    Pass,
    /// Issues found but fixable — reviewer suggests changes.
    NeedsFix,
    /// Serious problems, code should not be merged.
    Reject,
}

impl ReviewVerdict {
    /// Short label for dashboard badges and event stream.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "review-pass",
            Self::NeedsFix => "review-fix",
            Self::Reject => "review-reject",
        }
    }

    /// Single-char icon for compact display.
    pub fn icon(self) -> &'static str {
        match self {
            Self::Pass => "\u{2705}",     // ✅
            Self::NeedsFix => "\u{26A0}", // ⚠
            Self::Reject => "\u{274C}",   // ❌
        }
    }
}

/// Full result of a completed review.
#[derive(Debug, Clone)]
pub struct ReviewResult {
    /// The agent that was reviewed.
    pub target_agent_id: AgentId,
    /// The reviewer agent that performed the review.
    pub reviewer_agent_id: AgentId,
    /// Final verdict.
    pub verdict: ReviewVerdict,
    /// One-line summary for the dashboard event stream.
    pub summary: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_config_defaults_disabled() {
        let cfg = ReviewConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.model.is_none());
        assert_eq!(cfg.mode, ReviewMode::Light);
    }

    #[test]
    fn verdict_labels_are_distinct() {
        let labels: Vec<&str> = [
            ReviewVerdict::Pass,
            ReviewVerdict::NeedsFix,
            ReviewVerdict::Reject,
        ]
        .iter()
        .map(|v| v.label())
        .collect();
        // All distinct.
        let unique: std::collections::BTreeSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn verdict_icons_are_non_empty() {
        assert!(!ReviewVerdict::Pass.icon().is_empty());
        assert!(!ReviewVerdict::NeedsFix.icon().is_empty());
        assert!(!ReviewVerdict::Reject.icon().is_empty());
    }
}
