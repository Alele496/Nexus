//! `/approve-scope` — define auto-approval rules for permission requests.
//!
//! Lets users pre-authorize categories of tool calls so they don't have to
//! answer every y/n prompt during long-running agent sessions.
//!
//! # Syntax
//!
//! ```text
//! /approve-scope <path> <duration>    # auto-approve edits under <path>
//! /approve-scope --<kind> <duration>  # auto-approve tools of <kind>
//! /approve-scope --clear              # remove all scopes
//! /approve-scope --list               # show active scopes
//! ```
//!
//! # Examples
//!
//! ```text
//! /approve-scope src/auth/ 30m       # auto-approve edits under src/auth/ for 30 min
//! /approve-scope --bash 10m          # auto-approve all bash commands for 10 min
//! /approve-scope --edit 5m           # auto-approve all edits for 5 min
//! ```

use crate::app::actions::Action;
use crate::slash::command::{CommandExecCtx, CommandResult, SlashCommand};

pub struct ApproveScopeCommand;

impl SlashCommand for ApproveScopeCommand {
    fn name(&self) -> &str {
        "approve-scope"
    }

    fn description(&self) -> &str {
        "定义自动批准规则（路径/工具类型 + 时长）"
    }

    fn usage(&self) -> &str {
        "/approve-scope [<path>|--<kind>] <duration> | --clear | --list"
    }

    fn takes_args(&self) -> bool {
        true
    }

    fn args_required(&self) -> bool {
        false
    }

    fn arg_placeholder(&self) -> Option<&str> {
        Some("<path> 30m | --bash 10m | --clear | --list")
    }

    fn run(&self, _ctx: &mut CommandExecCtx, args: &str) -> CommandResult {
        let args = args.trim();

        // --clear: remove all scopes.
        if args == "--clear" {
            return CommandResult::Action(Action::ApproveScopeClear);
        }

        // --list: show active scopes.
        if args == "--list" {
            return CommandResult::Action(Action::ApproveScopeList);
        }

        // Parse: [<path>|--<kind>] <duration>
        // Duration format: <number><unit> where unit is m (minutes) or h (hours).
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.is_empty() {
            return CommandResult::Message(
                "用法: /approve-scope [<path>|--<kind>] <duration> | --clear | --list".into(),
            );
        }

        let (path, tool_kind, duration_str) = match parts.len() {
            1 => {
                // Could be just a duration or --clear/--list (already handled).
                return CommandResult::Error(
                    "需要指定路径或工具类型。用法: /approve-scope <path> 30m 或 /approve-scope --bash 10m"
                        .into(),
                );
            }
            2 => {
                // <path_or_kind> <duration>
                let first = parts[0];
                let dur = parts[1];
                if let Some(kind) = first.strip_prefix("--") {
                    (None, Some(kind.to_string()), dur)
                } else {
                    (Some(first.to_string()), None, dur)
                }
            }
            _ => {
                return CommandResult::Error(
                    "参数过多。用法: /approve-scope [<path>|--<kind>] <duration>".into(),
                );
            }
        };

        // Parse duration.
        let duration_mins = match parse_duration_mins(duration_str) {
            Some(m) => m,
            None => {
                return CommandResult::Error(format!(
                    "无效的时长格式 '{}'。示例: 30m, 1h, 90m",
                    duration_str
                ));
            }
        };

        CommandResult::Action(Action::ApproveScopeAdd {
            path,
            tool_kind,
            duration_mins,
        })
    }
}

/// Parse a duration string like "30m" or "1h" or "90m" into minutes.
fn parse_duration_mins(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(num_str) = s.strip_suffix('h') {
        let hours: u64 = num_str.trim().parse().ok()?;
        Some(hours * 60)
    } else if let Some(num_str) = s.strip_suffix('m') {
        num_str.trim().parse().ok()
    } else {
        // Try as raw minutes.
        s.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_m_minutes() {
        assert_eq!(parse_duration_mins("30m"), Some(30));
        assert_eq!(parse_duration_mins("90m"), Some(90));
        assert_eq!(parse_duration_mins("0m"), Some(0));
    }

    #[test]
    fn parse_h_hours() {
        assert_eq!(parse_duration_mins("1h"), Some(60));
        assert_eq!(parse_duration_mins("2h"), Some(120));
    }

    #[test]
    fn parse_raw_number() {
        assert_eq!(parse_duration_mins("30"), Some(30));
    }

    #[test]
    fn parse_invalid() {
        assert_eq!(parse_duration_mins("abc"), None);
        assert_eq!(parse_duration_mins(""), None);
    }
}
