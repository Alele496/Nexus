//! Workflow engine for Nexus.
//!
//! Define multi-step task pipelines in YAML, execute them with DAG ordering,
//! and track step-by-step progress. Steps can be bash commands, agent prompts,
//! user confirmations, or nested parallel groups.
//!
//! # YAML format
//!
//! ```yaml
//! name: "deploy"
//! description: "Build, test, and deploy"
//! on_error: stop
//! timeout_secs: 600
//!
//! steps:
//!   - id: build
//!     type: bash
//!     command: "cargo build --release"
//!   - id: test
//!     type: bash
//!     command: "cargo test"
//!     depends_on: [build]
//!   - id: review
//!     type: agent
//!     prompt: "审查代码改动"
//!     depends_on: [test]
//!   - id: push
//!     type: bash
//!     command: "git push"
//!     depends_on: [review]
//!     requires_confirm: true
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ── Error types ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Failed to read workflow file at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse workflow YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("Duplicate step id: {0}")]
    DuplicateStepId(String),
    #[error("Unknown dependency '{dep}' referenced by step '{step}'")]
    UnknownDependency { step: String, dep: String },
    #[error("Circular dependency detected involving step '{0}'")]
    CircularDependency(String),
    #[error("Step '{0}' not found")]
    StepNotFound(String),
    #[error("Workflow timed out after {0}s")]
    Timeout(u64),
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("No steps defined in workflow")]
    NoSteps,
}

// ── Workflow definition types ──────────────────────────────────────────────

/// Top-level workflow definition, loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Default error behavior for all steps.
    #[serde(default)]
    pub on_error: OnError,
    /// Global timeout in seconds. 0 = no timeout.
    #[serde(default)]
    pub timeout_secs: u64,
    /// Workflow steps.
    pub steps: Vec<StepDef>,
}

impl WorkflowDefinition {
    /// Load a workflow definition from a YAML file.
    pub fn load(path: &Path) -> Result<Self, WorkflowError> {
        let content = std::fs::read_to_string(path).map_err(|e| WorkflowError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let def: Self = serde_yaml::from_str(&content)?;
        def.validate()?;
        Ok(def)
    }

    /// Parse a workflow definition from a YAML string.
    pub fn parse(yaml: &str) -> Result<Self, WorkflowError> {
        let def: Self = serde_yaml::from_str(yaml)?;
        def.validate()?;
        Ok(def)
    }

    /// Validate the workflow: no duplicate IDs, all deps exist, no cycles.
    pub fn validate(&self) -> Result<(), WorkflowError> {
        if self.steps.is_empty() {
            return Err(WorkflowError::NoSteps);
        }

        let mut ids = HashSet::new();
        for step in &self.steps {
            if !ids.insert(step.id.clone()) {
                return Err(WorkflowError::DuplicateStepId(step.id.clone()));
            }
        }

        for step in &self.steps {
            for dep in &step.depends_on {
                if !ids.contains(dep) {
                    return Err(WorkflowError::UnknownDependency {
                        step: step.id.clone(),
                        dep: dep.clone(),
                    });
                }
            }
        }

        // Cycle detection via topological sort.
        self.topological_order()?;
        Ok(())
    }

    /// Return step IDs in topological (DAG) order.
    /// Groups steps by "depth level" — all steps at the same level can run in parallel.
    pub fn topological_order(&self) -> Result<Vec<Vec<String>>, WorkflowError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut id_to_step: HashMap<&str, &StepDef> = HashMap::new();

        for step in &self.steps {
            id_to_step.insert(&step.id, step);
            in_degree.entry(&step.id).or_insert(0);
            children.entry(&step.id).or_default();
            for dep in &step.depends_on {
                children.entry(dep.as_str()).or_default().push(&step.id);
                *in_degree.entry(&step.id).or_insert(0) += 1;
            }
        }

        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut remaining = self.steps.len();

        while remaining > 0 {
            let current_level: Vec<String> = in_degree
                .iter()
                .filter(|(_, deg)| **deg == 0)
                .map(|(&id, _)| id.to_string())
                .collect();

            if current_level.is_empty() {
                // All remaining nodes have non-zero in-degree → cycle exists.
                let stuck: Vec<String> = in_degree
                    .iter()
                    .filter(|(_, deg)| **deg > 0)
                    .map(|(&id, _)| id.to_string())
                    .collect();
                return Err(WorkflowError::CircularDependency(stuck.join(", ")));
            }

            for id in &current_level {
                in_degree.remove(id.as_str());
                remaining -= 1;
                if let Some(deps) = children.get(id.as_str()) {
                    for &child in deps {
                        if let Some(deg) = in_degree.get_mut(child) {
                            *deg = deg.saturating_sub(1);
                        }
                    }
                }
            }

            levels.push(current_level);
        }

        Ok(levels)
    }
}

/// A single step in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDef {
    /// Unique identifier within the workflow.
    pub id: String,
    /// Human-readable description of what this step does.
    #[serde(default)]
    pub description: String,
    /// Step type (flattened so type tag and variant fields are at the same level).
    #[serde(flatten)]
    pub step_type: StepType,
    /// IDs of steps that must complete before this one.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Per-step error strategy. Falls back to workflow-level on_error.
    #[serde(default)]
    pub on_error: Option<OnError>,
    /// Per-step timeout in seconds. 0 = inherit from workflow.
    #[serde(default)]
    pub timeout_secs: u64,
    /// Maximum retry count on failure (only when on_error is "retry").
    #[serde(default)]
    pub max_retries: u32,
    /// Whether user confirmation is required before this step runs.
    #[serde(default)]
    pub requires_confirm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepType {
    /// Run a shell command.
    Bash {
        command: String,
        #[serde(default)]
        cwd: Option<String>,
    },
    /// Send a prompt to the AI agent.
    Agent {
        prompt: String,
    },
    /// Pause and wait for the user to confirm before proceeding.
    Confirm {
        message: String,
    },
    /// Run a group of sub-steps in parallel.
    Parallel {
        steps: Vec<StepDef>,
    },
}

impl StepType {
    pub fn label(&self) -> &str {
        match self {
            StepType::Bash { .. } => "Bash",
            StepType::Agent { .. } => "Agent",
            StepType::Confirm { .. } => "确认",
            StepType::Parallel { .. } => "并行",
        }
    }
}

// ── Error strategy ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnError {
    /// Stop the entire workflow on failure.
    Stop,
    /// Retry the failed step up to max_retries times.
    Retry,
    /// Skip the failed step and continue.
    Skip,
}

impl Default for OnError {
    fn default() -> Self {
        Self::Stop
    }
}

// ── Execution state ────────────────────────────────────────────────────────

/// Runtime state of a workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub workflow_name: String,
    /// Unique run ID.
    pub run_id: String,
    /// Overall workflow status.
    pub status: WorkflowStatus,
    /// Per-step states, keyed by step ID.
    pub steps: HashMap<String, StepState>,
    /// ISO-8601 timestamps.
    pub started_at: String,
    pub finished_at: Option<String>,
    /// Error message if workflow failed.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Pending,
    Running,
    Paused,
    Success,
    Failed,
    Cancelled,
}

impl WorkflowStatus {
    pub fn label(&self) -> &str {
        match self {
            WorkflowStatus::Pending => "等待中",
            WorkflowStatus::Running => "执行中",
            WorkflowStatus::Paused => "已暂停",
            WorkflowStatus::Success => "成功",
            WorkflowStatus::Failed => "失败",
            WorkflowStatus::Cancelled => "已取消",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            WorkflowStatus::Pending => "○",
            WorkflowStatus::Running => "◉",
            WorkflowStatus::Paused => "⏸",
            WorkflowStatus::Success => "✓",
            WorkflowStatus::Failed => "✗",
            WorkflowStatus::Cancelled => "⊘",
        }
    }
}

/// Runtime state of a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepState {
    pub id: String,
    pub description: String,
    pub step_type_label: String,
    pub status: StepStatus,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error: Option<String>,
    pub retries: u32,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Queued,
    Running,
    Success,
    Failed,
    Skipped,
    AwaitingConfirm,
}

impl StepStatus {
    pub fn label(&self) -> &str {
        match self {
            StepStatus::Pending => "等待",
            StepStatus::Queued => "排队",
            StepStatus::Running => "执行中",
            StepStatus::Success => "成功",
            StepStatus::Failed => "失败",
            StepStatus::Skipped => "跳过",
            StepStatus::AwaitingConfirm => "等待确认",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            StepStatus::Pending => "○",
            StepStatus::Queued => "▸",
            StepStatus::Running => "◉",
            StepStatus::Success => "✓",
            StepStatus::Failed => "✗",
            StepStatus::Skipped => "⏭",
            StepStatus::AwaitingConfirm => "?",
        }
    }
}

// ── Workflow initialization ────────────────────────────────────────────────

impl WorkflowState {
    /// Create initial state from a definition.
    pub fn new(def: &WorkflowDefinition) -> Self {
        let steps: HashMap<String, StepState> = def
            .steps
            .iter()
            .map(|s| {
                let state = StepState {
                    id: s.id.clone(),
                    description: s.description.clone(),
                    step_type_label: s.step_type.label().to_string(),
                    status: StepStatus::Pending,
                    started_at: None,
                    finished_at: None,
                    error: None,
                    retries: 0,
                    output: None,
                };
                (s.id.clone(), state)
            })
            .collect();

        Self {
            workflow_name: def.name.clone(),
            run_id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            status: WorkflowStatus::Pending,
            steps,
            started_at: Utc::now().to_rfc3339(),
            finished_at: None,
            error: None,
        }
    }

    /// Format state as a human-readable progress report.
    pub fn progress_report(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "{} 工作流: {} [{}]",
            self.status.icon(),
            self.workflow_name,
            self.status.label()
        ));
        lines.push(format!("  Run ID: {}", self.run_id));

        // Collect steps in the order they appear in the workflow
        // (HashMap doesn't preserve order, so sort by name).
        let mut step_ids: Vec<&String> = self.steps.keys().collect();
        step_ids.sort();

        for id in step_ids {
            let step = &self.steps[id];
            lines.push(format!(
                "  {} {} — {}",
                step.status.icon(),
                step.description,
                step.status.label()
            ));
            if let Some(ref err) = step.error {
                lines.push(format!("    错误: {err}"));
            }
            if let Some(ref out) = step.output {
                // Show first line of output only.
                let first_line = out.lines().next().unwrap_or("");
                if !first_line.is_empty() {
                    lines.push(format!("    输出: {first_line}"));
                }
            }
        }

        if let Some(ref err) = self.error {
            lines.push(format!("  工作流错误: {err}"));
        }

        lines.join("\n")
    }
}

// ── Workflow discovery ─────────────────────────────────────────────────────

/// Find workflow files in the given directory.
pub fn discover_workflows(dir: &Path) -> Vec<PathBuf> {
    let workflows_dir = dir.join("workflows");
    if !workflows_dir.is_dir() {
        return Vec::new();
    }

    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&workflows_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Search for workflows in standard locations.
pub fn find_workflow_locations() -> Vec<PathBuf> {
    let mut locations = Vec::new();

    // $NEXUS_HOME/workflows/
    if let Ok(home) = std::env::var("NEXUS_HOME") {
        locations.push(Path::new(&home).join("workflows"));
    }
    // Project-local .sage/workflows/
    if let Ok(cwd) = std::env::current_dir() {
        let sage_wf = cwd.join(".sage").join("workflows");
        if sage_wf.is_dir() || cwd.join(".sage").is_dir() {
            locations.push(sage_wf);
        }
    }

    locations
        .into_iter()
        .filter(|p| p.is_dir())
        .flat_map(|d| discover_workflows(&d))
        .collect()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_workflow() -> WorkflowDefinition {
        WorkflowDefinition {
            name: "test-wf".into(),
            description: "A test workflow".into(),
            on_error: OnError::Stop,
            timeout_secs: 0,
            steps: vec![
                StepDef {
                    id: "step1".into(),
                    description: "First step".into(),
                    step_type: StepType::Bash {
                        command: "echo hello".into(),
                        cwd: None,
                    },
                    depends_on: vec![],
                    on_error: None,
                    timeout_secs: 0,
                    max_retries: 0,
                    requires_confirm: false,
                },
                StepDef {
                    id: "step2".into(),
                    description: "Second step".into(),
                    step_type: StepType::Agent {
                        prompt: "review code".into(),
                    },
                    depends_on: vec!["step1".into()],
                    on_error: None,
                    timeout_secs: 0,
                    max_retries: 0,
                    requires_confirm: false,
                },
            ],
        }
    }

    #[test]
    fn parse_yaml_definition() {
        let yaml = r#"
name: "deploy"
description: "Build and deploy"
on_error: stop
timeout_secs: 300
steps:
  - id: build
    type: bash
    command: "cargo build --release"
    description: "Build release"
  - id: test
    type: bash
    command: "cargo test"
    depends_on: [build]
  - id: push
    type: bash
    command: "git push"
    depends_on: [test]
    requires_confirm: true
"#;
        let def = WorkflowDefinition::parse(yaml).unwrap();
        assert_eq!(def.name, "deploy");
        assert_eq!(def.steps.len(), 3);
        assert_eq!(def.timeout_secs, 300);
    }

    #[test]
    fn default_on_error_is_stop() {
        let yaml = r#"
name: "test"
steps:
  - id: s1
    type: bash
    command: "echo ok"
"#;
        let def = WorkflowDefinition::parse(yaml).unwrap();
        assert_eq!(def.on_error, OnError::Stop);
    }

    #[test]
    fn topological_order_linear() {
        let wf = make_simple_workflow();
        let levels = wf.topological_order().unwrap();
        // step1 has no deps, step2 depends on step1
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0], vec!["step1"]);
        assert_eq!(levels[1], vec!["step2"]);
    }

    #[test]
    fn topological_order_parallel() {
        let yaml = r#"
name: "parallel-test"
steps:
  - id: a
    type: bash
    command: "echo a"
  - id: b
    type: bash
    command: "echo b"
  - id: c
    type: bash
    command: "echo c"
    depends_on: [a, b]
"#;
        let def = WorkflowDefinition::parse(yaml).unwrap();
        let levels = def.topological_order().unwrap();
        assert_eq!(levels.len(), 2);
        // a and b are independent → same level
        assert_eq!(levels[0].len(), 2);
        assert!(levels[0].contains(&"a".to_string()));
        assert!(levels[0].contains(&"b".to_string()));
        assert_eq!(levels[1], vec!["c"]);
    }

    #[test]
    fn topological_order_diamond() {
        let yaml = r#"
name: "diamond"
steps:
  - id: start
    type: bash
    command: "echo start"
  - id: left
    type: bash
    command: "echo left"
    depends_on: [start]
  - id: right
    type: bash
    command: "echo right"
    depends_on: [start]
  - id: end
    type: bash
    command: "echo end"
    depends_on: [left, right]
"#;
        let def = WorkflowDefinition::parse(yaml).unwrap();
        let levels = def.topological_order().unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["start"]);
        assert_eq!(levels[1].len(), 2); // left + right
        assert_eq!(levels[2], vec!["end"]);
    }

    #[test]
    fn detect_duplicate_ids() {
        let yaml = r#"
name: "dup"
steps:
  - id: same
    type: bash
    command: "echo 1"
  - id: same
    type: bash
    command: "echo 2"
"#;
        let result = WorkflowDefinition::parse(yaml);
        assert!(result.is_err());
        match result {
            Err(WorkflowError::DuplicateStepId(id)) => assert_eq!(id, "same"),
            _ => panic!("expected DuplicateStepId"),
        }
    }

    #[test]
    fn detect_unknown_dependency() {
        let yaml = r#"
name: "bad-dep"
steps:
  - id: s1
    type: bash
    command: "echo"
    depends_on: [nonexistent]
"#;
        let result = WorkflowDefinition::parse(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn detect_cycle() {
        let yaml = r#"
name: "cycle"
steps:
  - id: a
    type: bash
    command: "echo a"
    depends_on: [b]
  - id: b
    type: bash
    command: "echo b"
    depends_on: [a]
"#;
        let result = WorkflowDefinition::parse(yaml);
        assert!(result.is_err());
        match result {
            Err(WorkflowError::CircularDependency(_)) => {}
            other => panic!("expected CircularDependency, got {other:?}"),
        }
    }

    #[test]
    fn empty_steps_rejected() {
        let yaml = r#"
name: "empty"
steps: []
"#;
        let result = WorkflowDefinition::parse(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn workflow_state_init() {
        let wf = make_simple_workflow();
        let state = WorkflowState::new(&wf);
        assert_eq!(state.workflow_name, "test-wf");
        assert_eq!(state.status, WorkflowStatus::Pending);
        assert_eq!(state.steps.len(), 2);
        assert_eq!(state.steps["step1"].status, StepStatus::Pending);
        assert!(!state.run_id.is_empty());
    }

    #[test]
    fn progress_report_has_icons() {
        let wf = make_simple_workflow();
        let state = WorkflowState::new(&wf);
        let report = state.progress_report();
        assert!(report.contains("test-wf"));
        assert!(report.contains("First step"));
        assert!(report.contains("Second step"));
    }

    #[test]
    fn parse_agent_step() {
        let yaml = r#"
name: "review"
steps:
  - id: r1
    type: agent
    prompt: "Review the code"
    description: "Code review"
"#;
        let def = WorkflowDefinition::parse(yaml).unwrap();
        let step = &def.steps[0];
        match &step.step_type {
            StepType::Agent { prompt } => assert_eq!(prompt, "Review the code"),
            _ => panic!("expected Agent step"),
        }
    }

    #[test]
    fn parse_confirm_step() {
        let yaml = r#"
name: "with-confirm"
steps:
  - id: c1
    type: confirm
    message: "Ready to deploy?"
"#;
        let def = WorkflowDefinition::parse(yaml).unwrap();
        let step = &def.steps[0];
        match &step.step_type {
            StepType::Confirm { message } => assert_eq!(message, "Ready to deploy?"),
            _ => panic!("expected Confirm step"),
        }
    }

    #[test]
    fn parse_parallel_step() {
        let yaml = r#"
name: "parallel-test"
steps:
  - id: group
    type: parallel
    steps:
      - id: p1
        type: bash
        command: "echo 1"
      - id: p2
        type: bash
        command: "echo 2"
"#;
        let def = WorkflowDefinition::parse(yaml).unwrap();
        let step = &def.steps[0];
        match &step.step_type {
            StepType::Parallel { steps } => assert_eq!(steps.len(), 2),
            _ => panic!("expected Parallel step"),
        }
    }

    #[test]
    fn status_labels_and_icons() {
        assert_eq!(WorkflowStatus::Success.icon(), "✓");
        assert_eq!(WorkflowStatus::Success.label(), "成功");
        assert_eq!(WorkflowStatus::Failed.icon(), "✗");
        assert_eq!(WorkflowStatus::Running.icon(), "◉");
        assert_eq!(StepStatus::AwaitingConfirm.icon(), "?");
        assert_eq!(StepStatus::AwaitingConfirm.label(), "等待确认");
    }
}
