//! Approval scopes: user-defined rules that auto-approve permission requests.
//!
//! Scopes let users pre-authorize categories of tool calls (e.g. "all edits
//! under `src/auth/` for the next 30 minutes") so they don't have to answer
//! every y/n prompt. Scopes are checked in `handle_permission_request` after
//! YOLO mode but before enqueue — a matching scope acts as a silent allow-once.
//!
//! Managed via the `/approve-scope` slash command.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A single approval rule with optional path and tool-kind filters.
#[derive(Debug, Clone)]
pub struct ApprovalScope {
    /// Auto-approve tools whose target path starts with this prefix.
    pub path_prefix: Option<PathBuf>,
    /// Auto-approve tools matching this kind (case-insensitive).
    /// Examples: "Edit", "Bash", "WebFetch", "Read", "Write".
    pub tool_kind: Option<String>,
    /// When this scope expires (wall-clock instant).
    pub expires_at: Instant,
}

impl ApprovalScope {
    /// Check whether this scope matches a given tool call.
    ///
    /// Returns `true` when both the path prefix and tool kind filters
    /// match (if set). An unset filter always passes.
    pub fn matches_tool(&self, tool_kind: Option<&str>, file_path: Option<&str>) -> bool {
        // Check path prefix (if set).
        if let Some(ref prefix) = self.path_prefix {
            match file_path {
                Some(fp) if Self::path_starts_with(fp, prefix) => {}
                _ => return false,
            }
        }
        // Check tool kind (if set).
        if let Some(ref kind_filter) = self.tool_kind {
            match tool_kind {
                Some(tk) if tk.eq_ignore_ascii_case(kind_filter) => {}
                _ => return false,
            }
        }
        true
    }

    /// Whether this scope has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Human-readable description for `/approve-scope --list`.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref p) = self.path_prefix {
            parts.push(format!("path `{}`", p.display()));
        }
        if let Some(ref k) = self.tool_kind {
            parts.push(format!("kind `{k}`"));
        }
        let filter = if parts.is_empty() {
            "*".to_string()
        } else {
            parts.join(", ")
        };
        let remaining = self.expires_at.saturating_duration_since(Instant::now());
        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;
        if mins > 0 {
            format!("{filter} — {mins}m{secs}s remaining")
        } else {
            format!("{filter} — {secs}s remaining")
        }
    }

    /// Check if `file_path` starts with `prefix`, normalizing separators.
    fn path_starts_with(file_path: &str, prefix: &Path) -> bool {
        let normalized_file = file_path.replace('\\', "/");
        let normalized_prefix = prefix.to_string_lossy().replace('\\', "/");
        normalized_file.starts_with(&normalized_prefix)
    }
}

/// Collection of active approval scopes.
#[derive(Debug, Clone, Default)]
pub struct ApprovalScopes {
    scopes: Vec<ApprovalScope>,
}

impl ApprovalScopes {
    /// Create an empty scope set.
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Add a scope with the given duration.
    pub fn add(&mut self, path_prefix: Option<PathBuf>, tool_kind: Option<String>, duration: Duration) {
        self.scopes.push(ApprovalScope {
            path_prefix,
            tool_kind,
            expires_at: Instant::now() + duration,
        });
    }

    /// Remove all expired scopes and return how many were cleaned up.
    pub fn cleanup_expired(&mut self) -> usize {
        let before = self.scopes.len();
        self.scopes.retain(|s| !s.is_expired());
        before - self.scopes.len()
    }

    /// Clear all scopes (both active and expired).
    pub fn clear(&mut self) {
        self.scopes.clear();
    }

    /// Check whether any active scope auto-approves this tool call.
    ///
    /// `tool_kind` should be the string representation of the tool kind
    /// (e.g. `"Edit"`, `"Bash"`). `file_path` should be the target file
    /// path from the tool input (e.g. from `raw_input.file_path`).
    ///
    /// Expired scopes are cleaned up as a side effect.
    pub fn is_auto_approved(&mut self, tool_kind: Option<&str>, file_path: Option<&str>) -> bool {
        self.cleanup_expired();
        self.scopes.iter().any(|s| s.matches_tool(tool_kind, file_path))
    }

    /// Return all non-expired scopes for display.
    pub fn active_scopes(&self) -> Vec<&ApprovalScope> {
        self.scopes.iter().filter(|s| !s.is_expired()).collect()
    }

    /// Whether any scopes are currently active.
    pub fn has_active(&self) -> bool {
        self.scopes.iter().any(|s| !s.is_expired())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_scopes_no_match() {
        let mut scopes = ApprovalScopes::new();
        assert!(!scopes.is_auto_approved(Some("Edit"), Some("src/main.rs")));
    }

    #[test]
    fn path_prefix_match() {
        let mut scopes = ApprovalScopes::new();
        scopes.add(
            Some(PathBuf::from("src/auth/")),
            None,
            Duration::from_secs(600),
        );
        assert!(scopes.is_auto_approved(None, Some("src/auth/login.rs")));
        assert!(!scopes.is_auto_approved(None, Some("src/other/file.rs")));
    }

    #[test]
    fn tool_kind_match() {
        let mut scopes = ApprovalScopes::new();
        scopes.add(None, Some("Edit".into()), Duration::from_secs(600));
        assert!(scopes.is_auto_approved(Some("Edit"), Some("any/path.rs")));
        assert!(scopes.is_auto_approved(Some("edit"), Some("any/path.rs")));
        assert!(!scopes.is_auto_approved(Some("Bash"), Some("any/path.rs")));
    }

    #[test]
    fn both_filters_and_logic() {
        let mut scopes = ApprovalScopes::new();
        scopes.add(
            Some(PathBuf::from("src/")),
            Some("Edit".into()),
            Duration::from_secs(600),
        );
        // Both match → OK
        assert!(scopes.is_auto_approved(Some("Edit"), Some("src/main.rs")));
        // Only path matches, wrong tool kind → no match
        assert!(!scopes.is_auto_approved(Some("Bash"), Some("src/main.rs")));
        // Only tool kind matches, wrong path → no match
        assert!(!scopes.is_auto_approved(Some("Edit"), Some("tests/main.rs")));
    }

    #[test]
    fn path_normalization() {
        let mut scopes = ApprovalScopes::new();
        scopes.add(
            Some(PathBuf::from("src/auth/")),
            None,
            Duration::from_secs(600),
        );
        // Windows-style backslash paths should match forward-slash prefix
        assert!(scopes.is_auto_approved(None, Some("src\\auth\\login.rs")));
    }

    #[test]
    fn expired_scopes_cleaned_up() {
        let mut scopes = ApprovalScopes::new();
        // Add a scope that expires immediately (zero duration)
        scopes.add(None, Some("Edit".into()), Duration::ZERO);
        // Force immediate expiry by waiting a tiny bit
        std::thread::sleep(Duration::from_millis(1));
        assert!(!scopes.is_auto_approved(Some("Edit"), Some("src/main.rs")));
        assert_eq!(scopes.active_scopes().len(), 0);
    }

    #[test]
    fn clear_removes_all() {
        let mut scopes = ApprovalScopes::new();
        scopes.add(None, Some("Edit".into()), Duration::from_secs(600));
        scopes.add(None, Some("Bash".into()), Duration::from_secs(600));
        assert_eq!(scopes.active_scopes().len(), 2);
        scopes.clear();
        assert_eq!(scopes.active_scopes().len(), 0);
    }

    #[test]
    fn describe_shows_remaining_time() {
        let scope = ApprovalScope {
            path_prefix: Some(PathBuf::from("src/")),
            tool_kind: None,
            expires_at: Instant::now() + Duration::from_secs(125),
        };
        let desc = scope.describe();
        assert!(desc.contains("src/"));
        assert!(desc.contains("2m"));
    }

    #[test]
    fn no_filter_matches_everything() {
        let mut scopes = ApprovalScopes::new();
        scopes.add(None, None, Duration::from_secs(600));
        assert!(scopes.is_auto_approved(Some("Edit"), Some("any/file.rs")));
        assert!(scopes.is_auto_approved(Some("Bash"), None));
        assert!(scopes.is_auto_approved(None, None));
    }
}
