//! Fleet registry types: data structures for multi-repo fleet management.
//!
//! Parses `fleet-registry.json` and provides health-check logic for fleet projects.
//! Used by the `/fleet` slash commands and `fleet_*` tools.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Error types ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum FleetError {
    #[error("Failed to read fleet registry at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse fleet registry: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Project not found in fleet: {0}")]
    ProjectNotFound(String),
    #[error("Health check failed for {project}: {reason}")]
    HealthCheckFailed { project: String, reason: String },
}

// ── Fleet Registry data structures ─────────────────────────────────────────

/// Root of `fleet-registry.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetRegistry {
    pub fleet: FleetMeta,
    pub version: String,
    pub updated: String,
    pub projects: Vec<FleetProject>,
    #[serde(default)]
    pub health_check: Option<FleetHealthCheckConfig>,
    #[serde(default)]
    pub cross_project_impact: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMeta {
    pub name: String,
    pub hq_path: String,
}

/// A single project registered in the fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetProject {
    pub name: String,
    pub display: String,
    pub path: String,
    pub tier: FleetTier,
    pub description: String,
    #[serde(default)]
    pub claude_md: Option<String>,
    #[serde(default)]
    pub project_memory: Option<String>,
    #[serde(default)]
    pub repo: Option<FleetRepo>,
    #[serde(default)]
    pub runtime: Option<FleetRuntime>,
    #[serde(default)]
    pub dependencies: Option<FleetDependencies>,
    #[serde(default)]
    pub health_check: Option<FleetProjectHealthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FleetTier {
    Flagship,
    Template,
    Satellite,
    Tool,
    #[serde(untagged)]
    Custom(String),
}

impl FleetTier {
    pub fn as_str(&self) -> &str {
        match self {
            FleetTier::Flagship => "flagship",
            FleetTier::Template => "template",
            FleetTier::Satellite => "satellite",
            FleetTier::Tool => "tool",
            FleetTier::Custom(s) => s.as_str(),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            FleetTier::Flagship => "旗舰",
            FleetTier::Template => "模板",
            FleetTier::Satellite => "卫星",
            FleetTier::Tool => "工具",
            FleetTier::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetRepo {
    pub remote: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetRuntime {
    #[serde(rename = "type")]
    pub runtime_type: String,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetDependencies {
    #[serde(default)]
    pub system: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetProjectHealthConfig {
    #[serde(default)]
    pub git: bool,
    #[serde(default)]
    pub config_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetHealthCheckConfig {
    #[serde(default)]
    pub output_file: Option<String>,
    #[serde(default)]
    pub schedule: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub checks: Option<serde_json::Map<String, serde_json::Value>>,
}

// ── Fleet registry loading ─────────────────────────────────────────────────

impl FleetRegistry {
    /// Load fleet registry from a JSON file path.
    pub fn load(path: &Path) -> Result<Self, FleetError> {
        let content = std::fs::read_to_string(path).map_err(|e| FleetError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let registry: Self = serde_json::from_str(&content)?;
        Ok(registry)
    }

    /// Find a project by name.
    pub fn find_project(&self, name: &str) -> Option<&FleetProject> {
        self.projects.iter().find(|p| p.name == name)
    }

    /// List all project names in the fleet.
    pub fn project_names(&self) -> Vec<&str> {
        self.projects.iter().map(|p| p.name.as_str()).collect()
    }

    /// Count projects by tier.
    pub fn count_by_tier(&self, tier: &FleetTier) -> usize {
        self.projects
            .iter()
            .filter(|p| p.tier.as_str() == tier.as_str())
            .count()
    }
}

// ── Health check types ─────────────────────────────────────────────────────

/// Health status for a single project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealth {
    pub name: String,
    pub display: String,
    pub status: HealthStatus,
    pub git: Option<GitHealth>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Pass,
    Warn,
    Fail,
    Unknown,
}

impl HealthStatus {
    pub fn icon(&self) -> &str {
        match self {
            HealthStatus::Pass => "✓",
            HealthStatus::Warn => "⚠",
            HealthStatus::Fail => "✗",
            HealthStatus::Unknown => "?",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            HealthStatus::Pass => "通过",
            HealthStatus::Warn => "警告",
            HealthStatus::Fail => "失败",
            HealthStatus::Unknown => "未知",
        }
    }
}

/// Git-specific health indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHealth {
    pub branch: String,
    pub remote: String,
    pub clean: bool,
    pub uncommitted_count: usize,
    pub unpushed_count: usize,
    pub ahead: usize,
    pub behind: usize,
    pub last_commit: Option<String>,
    pub last_commit_at: Option<String>,
}

// ── Health check execution ─────────────────────────────────────────────────

/// Run health checks for a fleet project.
///
/// Currently checks git status (branch, clean/dirty, ahead/behind).
/// Returns `HealthStatus::Unknown` for projects whose path doesn't exist.
pub fn check_project_health(project: &FleetProject) -> ProjectHealth {
    let now = Utc::now();
    let path = std::path::Path::new(&project.path);

    if !path.exists() {
        return ProjectHealth {
            name: project.name.clone(),
            display: project.display.clone(),
            status: HealthStatus::Fail,
            git: None,
            checked_at: now,
        };
    }

    let git_health = check_git_health(path);
    let status = match &git_health {
        Some(g) if !g.clean => HealthStatus::Warn,
        Some(g) if g.behind > 0 => HealthStatus::Warn,
        Some(_) => HealthStatus::Pass,
        None => HealthStatus::Unknown,
    };

    ProjectHealth {
        name: project.name.clone(),
        display: project.display.clone(),
        status,
        git: git_health,
        checked_at: now,
    }
}

/// Check git health for a repository path.
fn check_git_health(repo_path: &Path) -> Option<GitHealth> {
    let repo = git2::Repository::open(repo_path).ok()?;
    let head = repo.head().ok()?;
    let branch = head.shorthand().unwrap_or("HEAD").to_string();

    // Count uncommitted changes via status.
    let uncommitted_count = repo
        .statuses(None)
        .map(|s: git2::Statuses<'_>| s.len())
        .unwrap_or(0);
    let clean = uncommitted_count == 0;

    // Count ahead/behind relative to upstream.
    let (ahead, behind) = head
        .resolve()
        .ok()
        .and_then(|head_ref: git2::Reference<'_>| {
            let upstream = repo.revparse_single(&format!("{branch}@{{upstream}}")).ok()?;
            let head_oid = head_ref.peel_to_commit().ok()?.id();
            let upstream_oid = upstream.peel_to_commit().ok()?.id();

            repo.graph_ahead_behind(head_oid, upstream_oid).ok()
        })
        .map(|(a, b)| (a as usize, b as usize))
        .unwrap_or((0, 0));

    let unpushed_count = if ahead > 0 { ahead } else { 0 };

    // Last commit info.
    let (last_commit, last_commit_at) = head
        .resolve()
        .ok()
        .and_then(|head_ref: git2::Reference<'_>| {
            let commit = head_ref.peel_to_commit().ok()?;
            let summary = commit
                .message()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            let time = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string());
            Some((summary, time))
        })
        .unwrap_or_default();

    // Detect remote name from the branch's upstream config.
    let remote = repo
        .config()
        .ok()
        .and_then(|cfg: git2::Config| cfg.get_string(&format!("branch.{}.remote", branch)).ok())
        .unwrap_or_else(|| "origin".to_string());

    Some(GitHealth {
        branch,
        remote,
        clean,
        uncommitted_count,
        unpushed_count,
        ahead,
        behind,
        last_commit: Some(last_commit),
        last_commit_at,
    })
}

/// Run health checks for all projects in a fleet.
pub fn check_fleet_health(registry: &FleetRegistry) -> Vec<ProjectHealth> {
    registry.projects.iter().map(check_project_health).collect()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_registry() {
        let json = r#"{
            "fleet": { "name": "Test Fleet", "hq_path": "/tmp/fleet" },
            "version": "1.0",
            "updated": "2026-07-20",
            "projects": []
        }"#;
        let registry: FleetRegistry = serde_json::from_str(json).unwrap();
        assert_eq!(registry.fleet.name, "Test Fleet");
        assert!(registry.projects.is_empty());
    }

    #[test]
    fn parse_project_with_all_fields() {
        let json = r#"{
            "fleet": { "name": "Test", "hq_path": "/tmp/fleet" },
            "version": "1.0",
            "updated": "2026-07-20",
            "projects": [
                {
                    "name": "test-proj",
                    "display": "Test Project",
                    "path": "/tmp/test-proj",
                    "tier": "flagship",
                    "description": "A test project",
                    "repo": {
                        "remote": "origin",
                        "url": "https://github.com/test/test",
                        "branch": "main",
                        "visibility": "public"
                    },
                    "runtime": {
                        "type": "config",
                        "engine": "nexus"
                    },
                    "dependencies": {
                        "system": ["git", "node"]
                    },
                    "health_check": {
                        "git": true,
                        "config_sync": true
                    }
                }
            ]
        }"#;
        let registry: FleetRegistry = serde_json::from_str(json).unwrap();
        assert_eq!(registry.projects.len(), 1);
        let p = &registry.projects[0];
        assert_eq!(p.name, "test-proj");
        assert_eq!(p.display, "Test Project");
        assert!(matches!(p.tier, FleetTier::Flagship));

        let repo = p.repo.as_ref().unwrap();
        assert_eq!(repo.branch, "main");
        assert_eq!(repo.visibility.as_deref(), Some("public"));

        let deps = p.dependencies.as_ref().unwrap();
        assert_eq!(deps.system, vec!["git", "node"]);

        let health = p.health_check.as_ref().unwrap();
        assert!(health.git);
        assert!(health.config_sync);
    }

    #[test]
    fn find_project_by_name() {
        let json = r#"{
            "fleet": { "name": "Test", "hq_path": "/tmp/fleet" },
            "version": "1.0",
            "updated": "2026-07-20",
            "projects": [
                { "name": "alpha", "display": "Alpha", "path": "/tmp/a", "tier": "flagship", "description": "" },
                { "name": "beta", "display": "Beta", "path": "/tmp/b", "tier": "satellite", "description": "" }
            ]
        }"#;
        let registry: FleetRegistry = serde_json::from_str(json).unwrap();
        assert!(registry.find_project("alpha").is_some());
        assert!(registry.find_project("gamma").is_none());
        assert_eq!(registry.project_names(), vec!["alpha", "beta"]);
    }

    #[test]
    fn count_by_tier() {
        let json = r#"{
            "fleet": { "name": "Test", "hq_path": "/tmp/fleet" },
            "version": "1.0",
            "updated": "2026-07-20",
            "projects": [
                { "name": "a", "display": "A", "path": "/tmp/a", "tier": "flagship", "description": "" },
                { "name": "b", "display": "B", "path": "/tmp/b", "tier": "flagship", "description": "" },
                { "name": "c", "display": "C", "path": "/tmp/c", "tier": "satellite", "description": "" }
            ]
        }"#;
        let registry: FleetRegistry = serde_json::from_str(json).unwrap();
        assert_eq!(registry.count_by_tier(&FleetTier::Flagship), 2);
        assert_eq!(registry.count_by_tier(&FleetTier::Satellite), 1);
        assert_eq!(registry.count_by_tier(&FleetTier::Tool), 0);
    }

    #[test]
    fn tier_display_labels() {
        assert_eq!(FleetTier::Flagship.label(), "旗舰");
        assert_eq!(FleetTier::Template.label(), "模板");
        assert_eq!(FleetTier::Satellite.label(), "卫星");
        assert_eq!(FleetTier::Tool.label(), "工具");
    }

    #[test]
    fn health_status_icons_and_labels() {
        assert_eq!(HealthStatus::Pass.icon(), "✓");
        assert_eq!(HealthStatus::Pass.label(), "通过");
        assert_eq!(HealthStatus::Warn.icon(), "⚠");
        assert_eq!(HealthStatus::Warn.label(), "警告");
        assert_eq!(HealthStatus::Fail.icon(), "✗");
        assert_eq!(HealthStatus::Fail.label(), "失败");
        assert_eq!(HealthStatus::Unknown.icon(), "?");
        assert_eq!(HealthStatus::Unknown.label(), "未知");
    }

    #[test]
    fn check_health_for_nonexistent_project() {
        let project = FleetProject {
            name: "ghost".into(),
            display: "Ghost".into(),
            path: "/nonexistent/path/12345".into(),
            tier: FleetTier::Tool,
            description: "".into(),
            claude_md: None,
            project_memory: None,
            repo: None,
            runtime: None,
            dependencies: None,
            health_check: None,
        };
        let health = check_project_health(&project);
        assert_eq!(health.status, HealthStatus::Fail);
        assert!(health.git.is_none());
    }

    #[test]
    fn check_fleet_health_all_empty() {
        let registry = FleetRegistry {
            fleet: FleetMeta {
                name: "Empty".into(),
                hq_path: "/tmp".into(),
            },
            version: "1.0".into(),
            updated: "2026-07-20".into(),
            projects: vec![],
            health_check: None,
            cross_project_impact: None,
        };
        let results = check_fleet_health(&registry);
        assert!(results.is_empty());
    }
}
