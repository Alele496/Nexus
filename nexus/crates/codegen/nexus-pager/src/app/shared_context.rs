//! Shared context for multi-agent coordination.
//!
//! Tracks what every agent is doing and detects edit conflicts when
//! two or more agents modify the same file.

use crate::app::agent::AgentId;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// Project-wide knowledge shared across all agents.
#[derive(Debug, Clone, Default)]
pub struct SharedKnowledge {
    /// High-level architecture notes (e.g. "this is a microservices project").
    pub architecture: Vec<String>,
    /// Coding conventions (e.g. "use async/await, not raw threads").
    pub conventions: Vec<String>,
    /// Things agents should NEVER do.
    pub taboos: Vec<String>,
}

/// What a single agent is currently working on.
#[derive(Debug, Clone)]
pub struct AgentManifest {
    pub agent_id: AgentId,
    /// Human-readable label (display_name or generated title).
    pub label: String,
    /// Files this agent has modified (deduplicated set).
    pub modified_files: HashSet<PathBuf>,
    /// Brief description of the current task.
    pub current_task: String,
}

/// A conflict: two or more agents have touched the same file.
#[derive(Debug, Clone)]
pub struct FileConflict {
    /// The contested file path.
    pub path: PathBuf,
    /// The agents that have modified this file.
    pub agents: Vec<AgentId>,
}

// ---------------------------------------------------------------------------
// SharedContextState
// ---------------------------------------------------------------------------

/// Aggregate shared context for all running agents.
///
/// Stored on [`AppView`](crate::app::app_view::AppView) and updated
/// whenever a file-change notification arrives from the shell.
#[derive(Debug, Clone, Default)]
pub struct SharedContextState {
    /// Project-wide shared knowledge.
    pub knowledge: SharedKnowledge,
    /// Per-agent manifest, keyed by AgentId.
    pub manifests: HashMap<AgentId, AgentManifest>,
    /// Currently active file-level conflicts.
    pub conflicts: Vec<FileConflict>,
}

impl SharedContextState {
    /// Record (or update) the set of files an agent has modified.
    ///
    /// Recomputes conflict detection after updating.
    pub fn update_agent_files(&mut self, agent_id: AgentId, label: String, files: Vec<PathBuf>) {
        let file_set: HashSet<PathBuf> = files.into_iter().collect();

        self.manifests
            .entry(agent_id)
            .and_modify(|m| {
                m.modified_files = file_set.clone();
                m.label = label.clone();
            })
            .or_insert(AgentManifest {
                agent_id,
                label,
                modified_files: file_set,
                current_task: String::new(),
            });

        self.recompute_conflicts();
    }

    /// Add a single file to an agent's manifest.
    ///
    /// Creates the manifest if the agent isn't tracked yet. Recomputes
    /// conflicts after the addition. This is the incremental counterpart
    /// to `update_agent_files` — use it when receiving per-file
    /// notifications from the shell notification bridge.
    pub fn add_agent_file(&mut self, agent_id: AgentId, label: String, file: PathBuf) {
        self.manifests
            .entry(agent_id)
            .and_modify(|m| {
                m.modified_files.insert(file.clone());
                m.label = label.clone();
            })
            .or_insert(AgentManifest {
                agent_id,
                label,
                modified_files: {
                    let mut set = HashSet::new();
                    set.insert(file);
                    set
                },
                current_task: String::new(),
            });

        self.recompute_conflicts();
    }

    /// Remove an agent (e.g. session closed).
    pub fn remove_agent(&mut self, agent_id: AgentId) {
        self.manifests.remove(&agent_id);
        self.recompute_conflicts();
    }

    /// True when at least one conflict exists.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Return conflicts that involve `agent_id`.
    pub fn conflicts_for_agent(&self, agent_id: AgentId) -> Vec<&FileConflict> {
        self.conflicts
            .iter()
            .filter(|c| c.agents.contains(&agent_id))
            .collect()
    }

    /// Build a human-readable "Team Context" section for injecting into
    /// `agent_id`'s system prompt at session startup.
    ///
    /// Includes:
    /// - A summary of other active agents and the files they've modified.
    /// - Any edit conflicts involving this agent (shared files).
    ///
    /// Returns `None` when there are no other agents to report on.
    pub fn build_context_section(&self, agent_id: AgentId) -> Option<String> {
        let siblings: Vec<&AgentManifest> = self
            .manifests
            .values()
            .filter(|m| m.agent_id != agent_id)
            .collect();

        let my_conflicts = self.conflicts_for_agent(agent_id);

        if siblings.is_empty() && my_conflicts.is_empty() {
            return None;
        }

        let mut section = String::from("## Team Context\n\n");
        section.push_str("You are part of a multi-agent team. ");
        section.push_str("Here is what your teammates are working on:\n");

        if !siblings.is_empty() {
            section.push('\n');
            for s in &siblings {
                let files: Vec<String> = s
                    .modified_files
                    .iter()
                    .map(|p| format!("`{}`", p.display()))
                    .collect();
                let file_list = if files.is_empty() {
                    "no files yet".to_string()
                } else {
                    files.join(", ")
                };
                section.push_str(&format!("- **{}**: {}\n", s.label, file_list));
            }
        }

        if !my_conflicts.is_empty() {
            section.push_str("\n### ⚠️ Edit Conflicts\n\n");
            section.push_str(
                "The following files are being modified by multiple agents. ",
            );
            section.push_str(
                "Coordinate with your teammates before editing these files:\n\n",
            );
            for c in &my_conflicts {
                let others: Vec<String> = c
                    .agents
                    .iter()
                    .filter(|id| **id != agent_id)
                    .filter_map(|id| self.manifests.get(id))
                    .map(|m| m.label.clone())
                    .collect();
                section.push_str(&format!(
                    "- `{}` — also edited by: {}\n",
                    c.path.display(),
                    others.join(", "),
                ));
            }
        }

        Some(section)
    }

    // ---- internal ----

    fn recompute_conflicts(&mut self) {
        // file_path → set of agent IDs that have modified it
        let mut file_owners: HashMap<&PathBuf, Vec<AgentId>> = HashMap::new();
        for manifest in self.manifests.values() {
            for path in &manifest.modified_files {
                file_owners.entry(path).or_default().push(manifest.agent_id);
            }
        }

        self.conflicts.clear();
        for (path, agents) in file_owners {
            if agents.len() >= 2 {
                self.conflicts.push(FileConflict {
                    path: path.clone(),
                    agents,
                });
            }
        }
        // Deterministic order for stable rendering.
        self.conflicts.sort_by(|a, b| a.path.cmp(&b.path));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_conflicts_with_single_agent() {
        let mut ctx = SharedContextState::default();
        ctx.update_agent_files(
            AgentId(1),
            "agent-1".into(),
            vec![PathBuf::from("src/main.rs")],
        );
        assert!(!ctx.has_conflicts());
        assert!(ctx.conflicts.is_empty());
    }

    #[test]
    fn no_conflicts_when_agents_touch_different_files() {
        let mut ctx = SharedContextState::default();
        ctx.update_agent_files(AgentId(1), "a".into(), vec![PathBuf::from("src/a.rs")]);
        ctx.update_agent_files(AgentId(2), "b".into(), vec![PathBuf::from("src/b.rs")]);
        assert!(!ctx.has_conflicts());
    }

    #[test]
    fn conflict_when_two_agents_touch_same_file() {
        let mut ctx = SharedContextState::default();
        ctx.update_agent_files(AgentId(1), "a".into(), vec![PathBuf::from("src/shared.rs")]);
        ctx.update_agent_files(AgentId(2), "b".into(), vec![PathBuf::from("src/shared.rs")]);
        assert!(ctx.has_conflicts());
        assert_eq!(ctx.conflicts.len(), 1);
        assert_eq!(ctx.conflicts[0].path, PathBuf::from("src/shared.rs"));
        assert_eq!(ctx.conflicts[0].agents.len(), 2);
    }

    #[test]
    fn conflict_resolves_when_agent_removed() {
        let mut ctx = SharedContextState::default();
        ctx.update_agent_files(AgentId(1), "a".into(), vec![PathBuf::from("src/shared.rs")]);
        ctx.update_agent_files(AgentId(2), "b".into(), vec![PathBuf::from("src/shared.rs")]);
        assert!(ctx.has_conflicts());

        ctx.remove_agent(AgentId(1));
        assert!(!ctx.has_conflicts());
    }

    #[test]
    fn update_replaces_old_file_set() {
        let mut ctx = SharedContextState::default();
        ctx.update_agent_files(
            AgentId(1),
            "a".into(),
            vec![PathBuf::from("src/shared.rs")],
        );
        ctx.update_agent_files(
            AgentId(2),
            "b".into(),
            vec![PathBuf::from("src/shared.rs")],
        );
        assert!(ctx.has_conflicts());

        // Agent 1 now only touches a different file → conflict resolves.
        ctx.update_agent_files(AgentId(1), "a".into(), vec![PathBuf::from("src/other.rs")]);
        assert!(!ctx.has_conflicts());
    }

    #[test]
    fn add_agent_file_accumulates_incrementally() {
        let mut ctx = SharedContextState::default();
        ctx.add_agent_file(AgentId(1), "a".into(), PathBuf::from("src/a.rs"));
        ctx.add_agent_file(AgentId(1), "a".into(), PathBuf::from("src/b.rs"));
        ctx.add_agent_file(AgentId(2), "b".into(), PathBuf::from("src/b.rs"));
        // Agent 1 has {a.rs, b.rs}, Agent 2 has {b.rs} → conflict on b.rs
        assert!(ctx.has_conflicts());
        assert_eq!(ctx.conflicts.len(), 1);
        assert_eq!(ctx.conflicts[0].path, PathBuf::from("src/b.rs"));
    }

    #[test]
    fn add_agent_file_creates_manifest_for_new_agent() {
        let mut ctx = SharedContextState::default();
        ctx.add_agent_file(AgentId(1), "new-agent".into(), PathBuf::from("src/main.rs"));
        assert!(ctx.manifests.contains_key(&AgentId(1)));
        assert_eq!(ctx.manifests[&AgentId(1)].label, "new-agent");
        assert!(ctx.manifests[&AgentId(1)].modified_files.contains(&PathBuf::from("src/main.rs")));
    }

    #[test]
    fn conflicts_for_agent_returns_only_relevant_conflicts() {
        let mut ctx = SharedContextState::default();
        ctx.update_agent_files(
            AgentId(1),
            "a".into(),
            vec![PathBuf::from("src/a.rs"), PathBuf::from("src/shared.rs")],
        );
        ctx.update_agent_files(
            AgentId(2),
            "b".into(),
            vec![PathBuf::from("src/b.rs"), PathBuf::from("src/shared.rs")],
        );
        ctx.update_agent_files(
            AgentId(3),
            "c".into(),
            vec![PathBuf::from("src/a.rs")],
        );
        // Conflicts: src/a.rs (agents 1,3), src/shared.rs (agents 1,2)
        assert_eq!(ctx.conflicts.len(), 2);

        let c1 = ctx.conflicts_for_agent(AgentId(1));
        assert_eq!(c1.len(), 2); // agent 1 is in both conflicts

        let c2 = ctx.conflicts_for_agent(AgentId(2));
        assert_eq!(c2.len(), 1); // agent 2 only in shared.rs

        let c3 = ctx.conflicts_for_agent(AgentId(3));
        assert_eq!(c3.len(), 1); // agent 3 only in a.rs
    }

    #[test]
    fn build_context_section_returns_none_when_no_other_agents() {
        let mut ctx = SharedContextState::default();
        ctx.add_agent_file(
            AgentId(1),
            "lone-agent".into(),
            PathBuf::from("src/main.rs"),
        );
        let section = ctx.build_context_section(AgentId(1));
        assert!(section.is_none());
    }

    #[test]
    fn build_context_section_lists_siblings() {
        let mut ctx = SharedContextState::default();
        ctx.add_agent_file(AgentId(1), "Alice".into(), PathBuf::from("src/a.rs"));
        ctx.add_agent_file(AgentId(2), "Bob".into(), PathBuf::from("src/b.rs"));
        let section = ctx
            .build_context_section(AgentId(1))
            .expect("should have context");
        assert!(section.contains("Team Context"));
        assert!(section.contains("Bob"));
        assert!(section.contains("src/b.rs"));
        // Should NOT list self
        assert!(!section.contains("Alice"));
        assert!(!section.contains("src/a.rs"));
    }

    #[test]
    fn build_context_section_includes_conflicts() {
        let mut ctx = SharedContextState::default();
        ctx.add_agent_file(
            AgentId(1),
            "Alice".into(),
            PathBuf::from("src/shared.rs"),
        );
        ctx.add_agent_file(
            AgentId(2),
            "Bob".into(),
            PathBuf::from("src/shared.rs"),
        );
        let section = ctx
            .build_context_section(AgentId(1))
            .expect("should have context");
        assert!(section.contains("Edit Conflicts"));
        assert!(section.contains("src/shared.rs"));
        assert!(section.contains("Bob"));
    }
}
