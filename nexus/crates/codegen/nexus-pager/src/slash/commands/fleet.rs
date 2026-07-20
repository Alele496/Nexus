//! `/fleet` — multi-repo fleet management.
//!
//! Subcommands:
//! - `/fleet` — fleet dashboard overview
//! - `/fleet health` — run health checks on all registered projects
//! - `/fleet list` — list all registered projects
//! - `/fleet switch <name>` — switch working directory to a fleet project
//! - `/fleet add <path>` — register a new project
//! - `/fleet remove <name>` — unregister a project

use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};
use std::path::Path;

pub struct FleetCommand;

impl FleetCommand {
    /// Try to find and load the fleet registry from common locations.
    fn find_registry() -> Option<nexus_fleet_types::FleetRegistry> {
        let candidates = [
            std::env::var("NEXUS_HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join("fleet-registry.json")),
            Some(std::path::PathBuf::from("fleet-registry.json")),
        ];

        for path in candidates.iter().flatten() {
            if path.is_file() {
                match nexus_fleet_types::FleetRegistry::load(path) {
                    Ok(r) => return Some(r),
                    Err(_) => continue,
                }
            }
        }
        None
    }

    fn render_dashboard(registry: &nexus_fleet_types::FleetRegistry) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "🚢 {} — {} 个项目",
            registry.fleet.name,
            registry.projects.len()
        ));
        lines.push(String::new());

        for project in &registry.projects {
            let health = nexus_fleet_types::check_project_health(project);
            let icon = health.status.icon();
            lines.push(format!(
                "  {} {} [{}]",
                icon,
                project.display,
                project.tier.label()
            ));
            lines.push(format!("    路径: {}", project.path));

            if let Some(ref git) = health.git {
                let dirty = if !git.clean {
                    format!(" ({} 未提交)", git.uncommitted_count)
                } else {
                    String::new()
                };
                lines.push(format!(
                    "    分支: {}{} | ahead:{} behind:{}",
                    git.branch, dirty, git.ahead, git.behind
                ));
            }
            lines.push(String::new());
        }

        if let Some(ref impact) = registry.cross_project_impact {
            if !impact.is_empty() {
                lines.push("跨项目依赖:".to_string());
                for (name, info) in impact {
                    if let Some(note) = info.get("note").and_then(|v| v.as_str()) {
                        lines.push(format!("  {} → {}", name, note));
                    }
                }
            }
        }

        lines.join("\n")
    }

    fn render_health_report(registry: &nexus_fleet_types::FleetRegistry) -> String {
        let results = nexus_fleet_types::check_fleet_health(registry);
        let mut lines = Vec::new();
        lines.push("🔍 舰队健康巡检".to_string());
        lines.push(String::new());

        let mut pass = 0;
        let mut warn = 0;
        let mut fail = 0;

        for h in &results {
            lines.push(format!(
                "  {} {} — {}",
                h.status.icon(),
                h.display,
                h.status.label()
            ));

            match h.status {
                nexus_fleet_types::HealthStatus::Pass => pass += 1,
                nexus_fleet_types::HealthStatus::Warn => warn += 1,
                nexus_fleet_types::HealthStatus::Fail => fail += 1,
                _ => {}
            }

            if let Some(ref git) = h.git {
                if !git.clean {
                    lines.push(format!(
                        "    ⚠ {} 个未提交变更",
                        git.uncommitted_count
                    ));
                }
                if git.behind > 0 {
                    lines.push(format!(
                        "    ⚠ 落后远程 {git_behind} 个提交",
                        git_behind = git.behind
                    ));
                }
            }
        }

        lines.push(String::new());
        lines.push(format!(
            "总计: {} 通过 / {} 警告 / {} 失败",
            pass, warn, fail
        ));

        lines.join("\n")
    }
}

impl SlashCommand for FleetCommand {
    fn name(&self) -> &str {
        "fleet"
    }

    fn description(&self) -> &str {
        "舰队管理：查看/切换/健康巡检多仓库项目"
    }

    fn usage(&self) -> &str {
        "/fleet [health|list|switch <name>|add <path>|remove <name>]"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("health|list|switch|add|remove")
    }

    fn session_scoped(&self) -> bool {
        true
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let trimmed = args.trim();

        match trimmed {
            "" => {
                // Default: show dashboard.
                match Self::find_registry() {
                    Some(registry) => CommandResult::Message(Self::render_dashboard(&registry)),
                    None => CommandResult::Error(
                        "未找到 fleet-registry.json。\n\
                         将 fleet-registry.json 放在 $NEXUS_HOME/ 或当前目录下。"
                            .to_string(),
                    ),
                }
            }
            "health" => match Self::find_registry() {
                Some(registry) => {
                    CommandResult::Message(Self::render_health_report(&registry))
                }
                None => CommandResult::Error("未找到 fleet-registry.json".to_string()),
            },
            "list" => match Self::find_registry() {
                Some(registry) => {
                    let mut lines = Vec::new();
                    for p in &registry.projects {
                        lines.push(format!(
                            "  {} [{}] — {}",
                            p.display,
                            p.tier.label(),
                            p.path
                        ));
                    }
                    CommandResult::Message(format!(
                        "舰队项目列表 ({} 个):\n{}",
                        registry.projects.len(),
                        lines.join("\n")
                    ))
                }
                None => CommandResult::Error("未找到 fleet-registry.json".to_string()),
            },
            _ if trimmed.starts_with("switch ") => {
                let name = trimmed.strip_prefix("switch ").unwrap().trim();
                match Self::find_registry() {
                    Some(registry) => match registry.find_project(name) {
                        Some(project) => {
                            let path = Path::new(&project.path);
                            if !path.is_dir() {
                                return CommandResult::Error(format!(
                                    "项目路径不存在: {}",
                                    project.path
                                ));
                            }
                            match std::env::set_current_dir(path) {
                                Ok(()) => CommandResult::Message(format!(
                                    "✅ 已切换到: {} ({})",
                                    project.display, project.path
                                )),
                                Err(e) => CommandResult::Error(format!(
                                    "切换失败: {e}"
                                )),
                            }
                        }
                        None => CommandResult::Error(format!(
                            "舰队中未找到项目: {name}。可用项目: {}",
                            registry.project_names().join(", ")
                        )),
                    },
                    None => CommandResult::Error("未找到 fleet-registry.json".to_string()),
                }
            }
            _ if trimmed.starts_with("remove ") => {
                CommandResult::Error("通过编辑 fleet-registry.json 移除项目。暂不支持命令行移除。".to_string())
            }
            _ if trimmed.starts_with("add ") => {
                CommandResult::Error("通过编辑 fleet-registry.json 添加项目。暂不支持命令行添加。".to_string())
            }
            _ => CommandResult::Error(format!(
                "未知子命令: {trimmed}。用法: /fleet [health|list|switch <name>]"
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
        let cmd = FleetCommand;
        assert_eq!(cmd.name(), "fleet");
        assert!(cmd.takes_args());
        assert!(!cmd.description().is_empty());
        assert!(!cmd.usage().is_empty());
    }

    #[test]
    fn unknown_subcommand_errors() {
        let (models, bundle) = (ModelState::default(), BundleState::default());
        let mut c = ctx(&models, &bundle);
        match FleetCommand.run(&mut c, "unknown") {
            CommandResult::Error(msg) => assert!(msg.contains("未知子命令")),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
