//! `/project` — single-repo project management.
//!
//! Subcommands:
//! - `/project info` — show current project details
//! - `/project init` — initialize `.sage/` in the project root

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use nexus_project::{ProjectInfo, init_sage_dir};

pub struct ProjectCommand;

impl SlashCommand for ProjectCommand {
    fn name(&self) -> &str {
        "project"
    }

    fn description(&self) -> &str {
        "项目管理：查看项目信息、初始化.sage/配置"
    }

    fn usage(&self) -> &str {
        "/project [info|init]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("info|init")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let cwd = std::env::current_dir().unwrap_or_default();
        let sub = args.trim();

        match sub {
            "" | "info" => {
                match ProjectInfo::detect(&cwd) {
                    Ok(project) => {
                        let mut lines = Vec::new();
                        lines.push(format!("📁 {} 项目信息", project.project_type.icon()));
                        lines.push(format!("  名称:     {}", project.name));
                        lines.push(format!("  类型:     {}", project.project_type.label()));
                        lines.push(format!("  根目录:   {}", project.root.display()));
                        let config_files_str = if project.config_files.is_empty() {
                            "无".to_string()
                        } else {
                            project.config_files.join(", ")
                        };
                        lines.push(format!("  配置文件: {config_files_str}"));
                        lines.push(format!(
                            "  .sage/:   {}",
                            if project.has_sage_config { "已配置" } else { "未初始化" }
                        ));
                        lines.push(format!(
                            "  AGENTS.md: {}",
                            if project.has_agents_md { "已存在" } else { "无" }
                        ));
                        CommandResult::Message(lines.join("\n"))
                    }
                    Err(e) => CommandResult::Error(format!("项目检测失败: {e}")),
                }
            }
            "init" => {
                match ProjectInfo::detect(&cwd) {
                    Ok(project) => {
                        if project.has_sage_config {
                            return CommandResult::Error(format!(
                                ".sage/ 已存在于 {}，无需重复初始化",
                                project.sage_dir().display()
                            ));
                        }
                        match init_sage_dir(&project.root, project.project_type) {
                            Ok(result) => {
                                let mut lines = Vec::new();
                                lines.push("✅ .sage/ 初始化完成".to_string());
                                for f in &result.created_files {
                                    lines.push(format!("  创建: {}", f.display()));
                                }
                                CommandResult::Message(lines.join("\n"))
                            }
                            Err(e) => CommandResult::Error(format!("初始化失败: {e}")),
                        }
                    }
                    Err(e) => CommandResult::Error(format!("项目检测失败: {e}")),
                }
            }
            _ => CommandResult::Error(format!(
                "未知子命令: {sub}。用法: /project [info|init]"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::model_state::ModelState;
    use crate::app::bundle::BundleState;
    use crate::settings::PagerLocalSnapshot;

    fn ctx<'a>(models: &'a ModelState, bundle: &'a BundleState) -> CommandExecCtx<'a> {
        CommandExecCtx {
            models,
            session_id: None,
            bundle_state: bundle,
            screen_mode: crate::app::ScreenMode::Inline,
            pager_state: PagerLocalSnapshot {
                multiline_mode: false,
                yolo_mode: false,
                ..PagerLocalSnapshot::default()
            },
        }
    }

    #[test]
    fn metadata() {
        let cmd = ProjectCommand;
        assert_eq!(cmd.name(), "project");
        assert!(cmd.takes_args());
        assert!(!cmd.description().is_empty());
        assert!(!cmd.usage().is_empty());
    }

    #[test]
    fn unknown_subcommand_errors() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match ProjectCommand.run(&mut c, "unknown") {
            CommandResult::Error(msg) => assert!(msg.contains("未知子命令")),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
