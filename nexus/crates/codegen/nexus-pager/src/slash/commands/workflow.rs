//! `/workflow` — workflow engine management.
//!
//! Subcommands:
//! - `/workflow list` — list available workflows
//! - `/workflow run <name>` — execute a workflow
//! - `/workflow status` — show the last workflow execution status

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use nexus_workflow::{WorkflowDefinition, WorkflowState, find_workflow_locations};
use std::path::PathBuf;

pub struct WorkflowCommand;

impl WorkflowCommand {
    /// Find a workflow file by name.
    fn find_workflow(name: &str) -> Option<PathBuf> {
        find_workflow_locations()
            .into_iter()
            .find(|p| {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.eq_ignore_ascii_case(name))
            })
    }

    fn list_workflows() -> String {
        let locations = find_workflow_locations();
        if locations.is_empty() {
            return "未找到任何工作流。\n\n将 .yml 文件放在 .sage/workflows/ 或 $NEXUS_HOME/workflows/ 目录下。".to_string();
        }

        let mut lines = vec!["可用工作流:".to_string(), String::new()];
        for path in &locations {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            // Try to load description.
            let desc = WorkflowDefinition::load(path)
                .map(|d| d.description)
                .unwrap_or_default();
            let steps = WorkflowDefinition::load(path)
                .map(|d| d.steps.len())
                .unwrap_or(0);

            let desc_display = if desc.is_empty() {
                String::new()
            } else {
                format!(" — {desc}")
            };
            lines.push(format!(
                "  {name}{desc_display} ({steps} 步骤)"
            ));
        }
        lines.join("\n")
    }

    fn run_workflow(name: &str) -> CommandResult {
        match Self::find_workflow(name) {
            Some(path) => match WorkflowDefinition::load(&path) {
                Ok(def) => {
                    let state = WorkflowState::new(&def);

                    // Validate DAG.
                    match def.topological_order() {
                        Ok(levels) => {
                            let mut lines = vec![
                                format!(
                                    "{} 工作流: {} — {}",
                                    state.status.icon(),
                                    def.name,
                                    state.status.label()
                                ),
                                format!("  Run ID: {}", state.run_id),
                                String::new(),
                                "执行计划 (拓扑序):".to_string(),
                            ];

                            for (i, level) in levels.iter().enumerate() {
                                let level_str = if level.len() == 1 {
                                    level[0].clone()
                                } else {
                                    format!("[{}]  (并行)", level.join(", "))
                                };
                                lines.push(format!("  {}. {}", i + 1, level_str));
                            }

                            lines.push(String::new());
                            lines.push(format!("共 {} 步，{} 层级", def.steps.len(), levels.len()));
                            lines.push(String::new());
                            lines.push("工作流已加载并通过 DAG 验证。运行方式:".to_string());
                            lines.push("  - 手动执行: 终端逐个执行各步骤命令".to_string());
                            lines.push(
                                "  - Agent 执行: 使用 WorkflowRun 工具交由 Agent 分步执行".to_string(),
                            );

                            CommandResult::Message(lines.join("\n"))
                        }
                        Err(e) => CommandResult::Error(format!("DAG 验证失败: {e}")),
                    }
                }
                Err(e) => CommandResult::Error(format!("工作流加载失败: {e}")),
            },
            None => {
                let available: Vec<String> = find_workflow_locations()
                    .iter()
                    .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
                    .collect();
                CommandResult::Error(format!(
                    "未找到工作流: {name}。可用: {}",
                    if available.is_empty() {
                        "无".to_string()
                    } else {
                        available.join(", ")
                    }
                ))
            }
        }
    }
}

impl SlashCommand for WorkflowCommand {
    fn name(&self) -> &str {
        "workflow"
    }

    fn description(&self) -> &str {
        "工作流引擎：查看/运行多步骤自动化工作流"
    }

    fn usage(&self) -> &str {
        "/workflow [list|run <name>|status]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("list|run <name>|status")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let trimmed = args.trim();

        match trimmed {
            "" | "list" => CommandResult::Message(Self::list_workflows()),
            "status" => CommandResult::Error(
                "工作流状态查看需要运行中的工作流实例。请先使用 /workflow run <name> 启动一个工作流。"
                    .to_string(),
            ),
            _ if trimmed.starts_with("run ") => {
                let name = trimmed.strip_prefix("run ").unwrap().trim();
                if name.is_empty() {
                    CommandResult::Error("用法: /workflow run <name>".to_string())
                } else {
                    Self::run_workflow(name)
                }
            }
            _ => CommandResult::Error(format!(
                "未知子命令: {trimmed}。用法: /workflow [list|run <name>]"
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
        let cmd = WorkflowCommand;
        assert_eq!(cmd.name(), "workflow");
        assert!(cmd.takes_args());
        assert!(!cmd.description().is_empty());
        assert!(!cmd.usage().is_empty());
    }

    #[test]
    fn unknown_subcommand_errors() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match WorkflowCommand.run(&mut c, "unknown") {
            CommandResult::Error(msg) => assert!(msg.contains("未知子命令")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn empty_run_name_errors() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match WorkflowCommand.run(&mut c, "run ") {
            CommandResult::Error(msg) => assert!(msg.contains("用法")),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
