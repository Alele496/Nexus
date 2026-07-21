//! Single-repo project management for Nexus.
//!
//! Auto-detects the current project from the working directory:
//! - Git root discovery (via `nexus_agent::repo::RepoDirChain`)
//! - Project name and type inference from common config files
//! - `.sage/` project config directory management
//! - System prompt context generation

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Error types ────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Not inside a git repository")]
    NotGitRepo,
    #[error("Failed to read directory at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse {file}: {source}")]
    Parse {
        file: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("No project config files found (Cargo.toml, package.json, etc.)")]
    UnknownProjectType,
    #[error(".sage/ directory already exists at {0}")]
    AlreadyInitialized(PathBuf),
}

// ── Project type detection ─────────────────────────────────────────────────

/// Recognized project types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Java,
    Unknown,
}

impl ProjectType {
    pub fn as_str(&self) -> &str {
        match self {
            ProjectType::Rust => "rust",
            ProjectType::JavaScript => "javascript",
            ProjectType::TypeScript => "typescript",
            ProjectType::Python => "python",
            ProjectType::Go => "go",
            ProjectType::Java => "java",
            ProjectType::Unknown => "unknown",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            ProjectType::Rust => "Rust",
            ProjectType::JavaScript => "JavaScript",
            ProjectType::TypeScript => "TypeScript",
            ProjectType::Python => "Python",
            ProjectType::Go => "Go",
            ProjectType::Java => "Java",
            ProjectType::Unknown => "Unknown",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ProjectType::Rust => "🦀",
            ProjectType::JavaScript => "🟨",
            ProjectType::TypeScript => "🟦",
            ProjectType::Python => "🐍",
            ProjectType::Go => "🔵",
            ProjectType::Java => "☕",
            ProjectType::Unknown => "📁",
        }
    }
}

// ── Project descriptor ─────────────────────────────────────────────────────

/// Detected project information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// Project name (from directory name or config file).
    pub name: String,
    /// Absolute path to the git repository root.
    pub root: PathBuf,
    /// Detected project type.
    pub project_type: ProjectType,
    /// Whether `.sage/` directory exists in the project root.
    pub has_sage_config: bool,
    /// Whether `AGENTS.md` exists in the project root.
    pub has_agents_md: bool,
    /// Config files found in the project root.
    pub config_files: Vec<String>,
}

impl ProjectInfo {
    /// Detect project information from the current working directory.
    pub fn detect(cwd: &Path) -> Result<Self, ProjectError> {
        let chain = nexus_agent::repo::RepoDirChain::resolve(cwd);

        let root = chain
            .git_root
            .clone()
            .ok_or(ProjectError::NotGitRepo)?;

        Self::from_root(&root)
    }

    /// Detect project information from a known git root path.
    pub fn from_root(root: &Path) -> Result<Self, ProjectError> {
        let name = infer_project_name(root);
        let project_type = detect_project_type(root);
        let config_files = list_config_files(root);

        let sage_dir = root.join(".sage");
        let agents_md = root.join("AGENTS.md");

        Ok(Self {
            name,
            root: root.to_path_buf(),
            project_type,
            has_sage_config: sage_dir.is_dir(),
            has_agents_md: agents_md.is_file(),
            config_files,
        })
    }

    /// Path to the `.sage/` directory for this project.
    pub fn sage_dir(&self) -> PathBuf {
        self.root.join(".sage")
    }

    /// Path to `AGENTS.md` for this project.
    pub fn agents_md_path(&self) -> PathBuf {
        self.root.join("AGENTS.md")
    }

    /// Generate a context summary suitable for injecting into the system prompt.
    pub fn context_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("当前项目: {} ({})", self.name, self.project_type.label()));
        lines.push(format!("项目路径: {}", self.root.display()));

        if !self.config_files.is_empty() {
            lines.push(format!(
                "配置文件: {}",
                self.config_files.join(", ")
            ));
        }

        if self.has_agents_md {
            lines.push("AGENTS.md: 已加载".to_string());
        }

        lines.join("\n")
    }
}

// ── Project detection helpers ──────────────────────────────────────────────

/// Infer project name from config files or directory name.
fn infer_project_name(root: &Path) -> String {
    // Try Cargo.toml [package].name
    if let Some(name) = read_cargo_package_name(root) {
        return name;
    }
    // Try package.json "name"
    if let Some(name) = read_package_json_name(root) {
        return name;
    }
    // Fall back to directory name.
    root.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Detect project type from config files present in root.
pub fn detect_project_type(root: &Path) -> ProjectType {
    if root.join("Cargo.toml").is_file() {
        return ProjectType::Rust;
    }
    if root.join("tsconfig.json").is_file() {
        return ProjectType::TypeScript;
    }
    if root.join("package.json").is_file() {
        return ProjectType::JavaScript;
    }
    if root.join("pyproject.toml").is_file() || root.join("setup.py").is_file() {
        return ProjectType::Python;
    }
    if root.join("go.mod").is_file() {
        return ProjectType::Go;
    }
    if root.join("pom.xml").is_file() || root.join("build.gradle").is_file() {
        return ProjectType::Java;
    }
    ProjectType::Unknown
}

/// List known config files present in the project root.
fn list_config_files(root: &Path) -> Vec<String> {
    let candidates = [
        "Cargo.toml",
        "package.json",
        "tsconfig.json",
        "pyproject.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "Makefile",
        "CMakeLists.txt",
    ];
    candidates
        .iter()
        .filter(|f| root.join(f).is_file())
        .map(|s| s.to_string())
        .collect()
}

fn read_cargo_package_name(root: &Path) -> Option<String> {
    let cargo_toml = root.join("Cargo.toml");
    if !cargo_toml.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(&cargo_toml).ok()?;
    // Simple line-based parsing to avoid toml dependency for this one field.
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let name = val.trim().trim_matches('"');
                if !name.is_empty() && name != "workspace" {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

fn read_package_json_name(root: &Path) -> Option<String> {
    let pkg_json = root.join("package.json");
    if !pkg_json.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(&pkg_json).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed.get("name").and_then(|v| v.as_str()).map(|s| s.to_string())
}

// ── .sage/ initialization ──────────────────────────────────────────────────

/// Result of initializing `.sage/` in a project.
#[derive(Debug)]
pub struct InitResult {
    pub sage_dir: PathBuf,
    pub created_files: Vec<PathBuf>,
}

/// Default content for a project-level AGENTS.md template.
pub const DEFAULT_AGENTS_MD: &str = r#"# Project Lead 身份定义

你是本项目 AI 助手，负责理解用户意图、调度工具和子 Agent 完成任务。

## 项目信息
<!-- 在此补充项目特定的上下文、约定和规则 -->

## 代码规范
<!-- 项目的代码风格、命名约定、目录结构说明 -->

## 常用命令
<!-- 项目的构建、测试、部署命令 -->

## 注意事项
<!-- 需要注意的陷阱、已知问题、特殊配置 -->
"#;

/// Default content for a project-level `.sage/config.toml`.
pub fn default_project_config(project_name: &str, project_type: ProjectType) -> String {
    format!(
        r#"# Nexus 项目配置 — {name}
# 此文件由 `/project init` 自动生成

[project]
name = "{name}"
type = "{ptype}"

# 项目专属的 Agent 角色配置
# [subagents.roles.developer]
# default_capability_mode = "all"
"#,
        name = project_name,
        ptype = project_type.as_str(),
    )
}

/// Initialize `.sage/` in the given project root.
pub fn init_sage_dir(root: &Path, project_type: ProjectType) -> Result<InitResult, ProjectError> {
    let sage_dir = root.join(".sage");
    if sage_dir.is_dir() {
        return Err(ProjectError::AlreadyInitialized(sage_dir));
    }

    let mut created = Vec::new();

    std::fs::create_dir_all(&sage_dir).map_err(|e| ProjectError::Io {
        path: sage_dir.display().to_string(),
        source: e,
    })?;

    // Create config.toml
    let project_name = infer_project_name(root);
    let config_path = sage_dir.join("config.toml");
    let config_content = default_project_config(&project_name, project_type);
    std::fs::write(&config_path, &config_content).map_err(|e| ProjectError::Io {
        path: config_path.display().to_string(),
        source: e,
    })?;
    created.push(config_path);

    // Create AGENTS.md if it doesn't exist at project root.
    let agents_md = root.join("AGENTS.md");
    if !agents_md.is_file() {
        std::fs::write(&agents_md, DEFAULT_AGENTS_MD).map_err(|e| ProjectError::Io {
            path: agents_md.display().to_string(),
            source: e,
        })?;
        created.push(agents_md);
    }

    Ok(InitResult {
        sage_dir,
        created_files: created,
    })
}

// ── Context builder ────────────────────────────────────────────────────────

/// Build a prompt-ready context block for the current project.
///
/// Includes project name, type, path, config files, and git status summary.
pub fn build_project_context(project: &ProjectInfo) -> String {
    project.context_summary()
}

/// Build a brief project context for display in the welcome screen.
pub fn build_welcome_context(project: &ProjectInfo) -> String {
    format!(
        "{} {} | {}",
        project.project_type.icon(),
        project.name,
        project.root.display()
    )
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detect_project_type_rust() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Rust);
    }

    #[test]
    fn detect_project_type_javascript() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::JavaScript);
    }

    #[test]
    fn detect_project_type_typescript() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("tsconfig.json"), "{}").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::TypeScript);
    }

    #[test]
    fn detect_project_type_python_pyproject() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Python);
    }

    #[test]
    fn detect_project_type_python_setup() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("setup.py"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Python);
    }

    #[test]
    fn detect_project_type_go() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("go.mod"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Go);
    }

    #[test]
    fn detect_project_type_java_maven() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("pom.xml"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Java);
    }

    #[test]
    fn detect_project_type_java_gradle() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("build.gradle"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Java);
    }

    #[test]
    fn detect_project_type_unknown_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Unknown);
    }

    #[test]
    fn rust_takes_priority_over_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        // Cargo.toml + package.json: should be Rust (Cargo.toml checked first).
        fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Rust);
    }

    #[test]
    fn init_sage_dir_creates_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        let result = init_sage_dir(root, ProjectType::Rust).unwrap();
        assert!(result.sage_dir.is_dir());
        assert!(result.sage_dir.join("config.toml").is_file());
        assert!(root.join("AGENTS.md").is_file());
        assert_eq!(result.created_files.len(), 2);
    }

    #[test]
    fn init_sage_dir_idempotent_rejects() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join(".sage")).unwrap();

        let result = init_sage_dir(root, ProjectType::Rust);
        assert!(result.is_err());
        match result {
            Err(ProjectError::AlreadyInitialized(_)) => {}
            _ => panic!("expected AlreadyInitialized"),
        }
    }

    #[test]
    fn init_does_not_overwrite_existing_agents_md() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("AGENTS.md"), "# Custom\n").unwrap();

        let result = init_sage_dir(root, ProjectType::JavaScript).unwrap();
        let content = fs::read_to_string(root.join("AGENTS.md")).unwrap();
        assert_eq!(content, "# Custom\n");
        // Only config.toml was created (AGENTS.md already existed).
        assert_eq!(result.created_files.len(), 1);
    }

    #[test]
    fn project_info_context_summary_has_key_fields() {
        let info = ProjectInfo {
            name: "my-project".into(),
            root: PathBuf::from("/home/user/my-project"),
            project_type: ProjectType::Rust,
            has_sage_config: true,
            has_agents_md: true,
            config_files: vec!["Cargo.toml".into()],
        };
        let summary = info.context_summary();
        assert!(summary.contains("my-project"));
        assert!(summary.contains("Rust"));
        assert!(summary.contains("Cargo.toml"));
        assert!(summary.contains("AGENTS.md"));
    }

    #[test]
    fn project_name_from_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"my-awesome-crate\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(infer_project_name(tmp.path()), "my-awesome-crate");
    }

    #[test]
    fn project_name_from_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"name": "my-app", "version": "1.0.0"}"#,
        )
        .unwrap();
        assert_eq!(infer_project_name(tmp.path()), "my-app");
    }

    #[test]
    fn project_name_fallback_to_dir_name() {
        let tmp = tempfile::tempdir().unwrap();
        let name = tmp
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(infer_project_name(tmp.path()), name);
    }

    #[test]
    fn cargo_name_with_workspace_keyword_is_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[workspace.package]\nname = \"workspace\"\n",
        )
        .unwrap();
        // Falls back to directory name since "name = workspace" is skipped.
        let name = tmp
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(infer_project_name(tmp.path()), name);
    }

    #[test]
    fn list_config_files_detects_present() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        fs::write(tmp.path().join("Makefile"), "").unwrap();

        let files = list_config_files(tmp.path());
        assert!(files.contains(&"Cargo.toml".to_string()));
        assert!(files.contains(&"Makefile".to_string()));
        assert!(!files.contains(&"package.json".to_string()));
    }

    #[test]
    fn project_info_from_root() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test-crate\"\n",
        )
        .unwrap();

        let info = ProjectInfo::from_root(tmp.path()).unwrap();
        assert_eq!(info.name, "test-crate");
        assert_eq!(info.project_type, ProjectType::Rust);
        assert_eq!(info.root, tmp.path());
        assert!(!info.has_sage_config);
        assert!(!info.has_agents_md);
        assert!(info.config_files.contains(&"Cargo.toml".to_string()));
    }
}
