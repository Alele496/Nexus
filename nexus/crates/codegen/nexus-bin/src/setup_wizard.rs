//! Nexus 首次运行设置向导。
//!
//! 在 `~/.nexus/config.toml` 不存在或 `[startup].wizard_completed` 未设置时，
//! 引导用户完成初始配置：数据目录、提供商选择、API Key、模型选择、思考深度。

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// 重定向文件名：保存在 OS 默认目录（`~/.nexus/`）下，内容为实际的 Nexus 数据目录路径。
/// 解决无 bat 脚本直接双击 exe 时，`NEXUS_HOME` 环境变量无法跨进程持久化的问题。
const NEXUS_HOME_REDIRECT_FILENAME: &str = "nexus-home-path";

/// 提供商预设：包含名称、默认模型列表、API 端点、后端类型等信息。
struct ProviderPreset {
    /// 显示名称
    name: &'static str,
    /// 获取 API Key 的网址
    key_url: &'static str,
    /// 默认 API 端点
    base_url: &'static str,
    /// API 后端类型
    api_backend: &'static str,
    /// 是否支持推理深度调节
    supports_reasoning: bool,
    /// 模型列表: (model_id, display_name, context_window)
    models: &'static [(&'static str, &'static str, u64)],
    /// 环境变量提示
    env_var_hint: &'static str,
}

const PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset {
        name: "DeepSeek",
        key_url: "https://platform.deepseek.com/api_keys",
        base_url: "https://api.deepseek.com/v1",
        api_backend: "chat_completions",
        supports_reasoning: true,
        models: &[
            ("deepseek-v4-pro", "DeepSeek V4 Pro — 最强推理，1M 上下文 (推荐)", 1_000_000),
            ("deepseek-v4-flash", "DeepSeek V4 Flash — 更快响应，日常开发", 1_000_000),
        ],
        env_var_hint: "NEXUS_API_KEY",
    },
    ProviderPreset {
        name: "OpenAI",
        key_url: "https://platform.openai.com/api-keys",
        base_url: "https://api.openai.com/v1",
        api_backend: "chat_completions",
        supports_reasoning: true,
        models: &[
            ("gpt-5.2", "GPT-5.2 — 最强综合能力 (推荐)", 128_000),
            ("gpt-5.1", "GPT-5.1 — 平衡性能与速度", 128_000),
            ("gpt-5-mini", "GPT-5 Mini — 轻量快速，日常任务", 128_000),
        ],
        env_var_hint: "OPENAI_API_KEY",
    },
    ProviderPreset {
        name: "Anthropic (Claude)",
        key_url: "https://console.anthropic.com/settings/keys",
        base_url: "https://api.anthropic.com/v1",
        api_backend: "messages",
        supports_reasoning: false,
        models: &[
            ("claude-opus-4-7", "Claude Opus 4.7 — 最强推理，适合复杂任务 (推荐)", 200_000),
            ("claude-sonnet-4-6", "Claude Sonnet 4.6 — 快速响应的主力模型", 200_000),
            ("claude-haiku-4-5", "Claude Haiku 4.5 — 极速轻量，日常任务", 200_000),
        ],
        env_var_hint: "ANTHROPIC_API_KEY",
    },
    ProviderPreset {
        name: "自定义 (OpenAI 兼容 API)",
        key_url: "",
        base_url: "",
        api_backend: "chat_completions",
        supports_reasoning: false,
        models: &[],
        env_var_hint: "NEXUS_API_KEY",
    },
];

// ── 重定向文件管理 ──────────────────────────────────────────────────────

fn user_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

fn os_default_dot_nexus() -> Option<PathBuf> {
    user_home_dir().map(|h| h.join(".nexus"))
}

fn read_nexus_home_redirect() -> Option<PathBuf> {
    let redirect_file = os_default_dot_nexus()?.join(NEXUS_HOME_REDIRECT_FILENAME);
    let content = std::fs::read_to_string(&redirect_file).ok()?;
    let path = content.trim();
    if path.is_empty() {
        return None;
    }
    let p = PathBuf::from(path);
    if p.is_absolute() && p.exists() {
        Some(p)
    } else {
        None
    }
}

fn save_nexus_home_redirect(path: &Path) -> io::Result<()> {
    if let Some(dot_nexus) = os_default_dot_nexus() {
        std::fs::create_dir_all(&dot_nexus)?;
        std::fs::write(
            dot_nexus.join(NEXUS_HOME_REDIRECT_FILENAME),
            path.display().to_string(),
        )?;
    }
    Ok(())
}

fn default_nexus_home() -> PathBuf {
    if let Ok(nexus_home) = std::env::var("NEXUS_HOME") {
        let p = PathBuf::from(&nexus_home);
        if p.is_absolute() || nexus_home.starts_with("~") {
            return p;
        }
    }
    if let Some(redirected) = read_nexus_home_redirect() {
        return redirected;
    }
    if let Some(home) = user_home_dir() {
        return home.join(".nexus");
    }
    PathBuf::from(".nexus")
}

// ── 输入辅助 ────────────────────────────────────────────────────────────

fn read_line(prompt: &str) -> io::Result<String> {
    let mut stdout = io::stdout();
    write!(stdout, "{}", prompt)?;
    stdout.flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn read_line_with_default(prompt: &str, default: &str) -> io::Result<String> {
    let full_prompt = format!("{} (直接回车 = \"{}\"): ", prompt, default);
    let answer = read_line(&full_prompt)?;
    if answer.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(answer)
    }
}

// ── 公共入口 ────────────────────────────────────────────────────────────

pub fn needs_setup_wizard() -> bool {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        if arg == "--version" || arg == "-V" || arg == "--help" || arg == "-h" {
            return false;
        }
    }
    let config_path = config_toml_path();
    if !config_path.exists() {
        return true;
    }
    match std::fs::read_to_string(&config_path) {
        Ok(content) => !content.contains("wizard_completed = true"),
        Err(_) => true,
    }
}

fn config_toml_path() -> PathBuf {
    default_nexus_home().join("config.toml")
}

/// 运行首次设置向导，引导用户完成基本配置，写入 `config.toml`，
/// 返回选择的 Nexus 数据目录。
pub fn run_setup_wizard() -> io::Result<PathBuf> {
    print_banner();

    // ── 第 1 步：用户名 ───────────────────────────────────────────────
    let username = step_username()?;

    // ── 第 2 步：数据目录 ─────────────────────────────────────────────
    let nexus_home = step_data_directory()?;

    // ── 第 3 步：选择 AI 提供商 ───────────────────────────────────────
    let provider_idx = step_provider()?;
    let provider = &PROVIDERS[provider_idx];

    // ── 第 4 步：API Key ───────────────────────────────────────────────
    let api_key = step_api_key(provider, &nexus_home)?;

    // ── 第 5 步：选择模型 ─────────────────────────────────────────────
    let (model_id, model_name, context_window) = step_model_selection(provider)?;

    // ── 第 6 步：API 端点 ─────────────────────────────────────────────
    let api_base_url = step_api_endpoint(provider)?;

    // ── 第 7 步：思考深度（仅支持推理的提供商） ──────────────────────────
    let reasoning_effort = if provider.supports_reasoning {
        step_thinking_depth()?
    } else {
        String::new()
    };

    // ── 写入配置 ──────────────────────────────────────────────────────
    println!();
    print_separator("正在保存配置");
    let config_path = nexus_home.join("config.toml");
    let has_api_key = !api_key.is_empty();
    write_config_toml(
        &config_path,
        &username,
        model_id,
        model_name,
        &api_key,
        &api_base_url,
        provider.api_backend,
        context_window,
        provider.supports_reasoning,
        &reasoning_effort,
    )?;

    // 设 NEXUS_HOME 环境变量
    unsafe {
        std::env::set_var("NEXUS_HOME", nexus_home.as_os_str());
    }

    // 设 NEXUS_API_KEY 环境变量
    if has_api_key {
        unsafe {
            std::env::set_var("NEXUS_API_KEY", &api_key);
        }
    }

    // 写入 auth.json
    if has_api_key {
        let auth_json_path = nexus_home.join("auth.json");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let ts = format_time_rfc3339(now);
        let auth_entry = serde_json::json!({
            "key": &api_key,
            "auth_mode": "api_key",
            "create_time": ts,
            "user_id": &username,
            "email": serde_json::Value::Null,
        });
        let auth_map = serde_json::json!({
            "xai::api_key": auth_entry,
        });
        if let Err(e) = std::fs::write(
            &auth_json_path,
            serde_json::to_string_pretty(&auth_map).unwrap_or_default(),
        ) {
            eprintln!("  ⚠ 无法写入 auth.json: {e}");
        }
    }

    println!("  ✓ 配置文件已保存到: {}", config_path.display());
    println!();
    if has_api_key {
        println!("  Nexus 已准备就绪，正在进入主界面……");
    } else {
        println!("  ⚠ 未设置 API Key，进入主界面后请按 F2 → 设置 → API Key 进行配置。");
        println!("  （没有 API Key 将无法使用 AI 对话功能）");
    }
    println!();
    print_separator("");

    Ok(nexus_home)
}

// ── 界面辅助 ────────────────────────────────────────────────────────────

fn print_banner() {
    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║                                                  ║");
    println!("  ║      N E X U S  —  你的 AI 编程搭档              ║");
    println!("  ║                                                  ║");
    println!("  ║      欢迎首次使用！请花一分钟完成初始设置。       ║");
    println!("  ║      (所有设置后续均可通过 /settings 修改)        ║");
    println!("  ║                                                  ║");
    println!("  ╚══════════════════════════════════════════════════╝");
}

fn print_separator(title: &str) {
    if title.is_empty() {
        println!("  ────────────────────────────────────────────────");
    } else {
        println!("  ── {} ──", title);
    }
}

// ── 步骤 1：用户名 ───────────────────────────────────────────────────────

fn step_username() -> io::Result<String> {
    println!();
    print_separator("第 1 步：你是谁？");
    println!();
    println!("  请输入你的名字（任意名字即可，仅用于本地标识）：");
    println!();
    let username = read_line_with_default("  用户名", "Developer")?;
    println!("  ✓ 你好，{}！", username);
    Ok(username)
}

// ── 步骤 2：数据目录 ─────────────────────────────────────────────────────

fn step_data_directory() -> io::Result<PathBuf> {
    println!();
    print_separator("第 2 步：数据目录");
    println!();
    println!("  Nexus 的所有数据（配置、会话记录、插件等）都存放在一个");
    println!("  目录中。建议选一个空间充足的位置（如 F:\\nexus-home）。");
    println!();
    let default = default_nexus_home();
    loop {
        let answer = read_line_with_default(
            "  请输入数据目录路径",
            &default.display().to_string(),
        )?;
        let nexus_home = PathBuf::from(&answer);
        let nexus_home = if answer.starts_with("~") {
            if let Some(home) = user_home_dir() {
                let stripped = answer.strip_prefix("~/").unwrap_or(&answer);
                let stripped = stripped.strip_prefix("~").unwrap_or(stripped);
                home.join(stripped)
            } else {
                nexus_home
            }
        } else {
            nexus_home
        };
        match std::fs::create_dir_all(&nexus_home) {
            Ok(()) => {
                if let Err(e) = save_nexus_home_redirect(&nexus_home) {
                    eprintln!("  ⚠ 无法保存目录重定向文件: {e}");
                }
                println!("  ✓ 使用目录: {}", nexus_home.display());
                return Ok(nexus_home);
            }
            Err(e) => {
                println!("  ✗ 无法创建目录: {e}");
                println!("  请检查路径是否正确，或换一个位置重试。");
            }
        }
    }
}

// ── 步骤 3：选择 AI 提供商 ───────────────────────────────────────────────

fn step_provider() -> io::Result<usize> {
    println!();
    print_separator("第 3 步：选择 AI 提供商");
    println!();
    println!("  Nexus 支持多种 AI 模型提供商。请选择你使用的服务：");
    println!();

    for (i, provider) in PROVIDERS.iter().enumerate() {
        let num = i + 1;
        let desc = match provider.name {
            "DeepSeek" => "— 国产模型，1M 上下文，性价比极高 (推荐)",
            "OpenAI" => "— GPT 系列模型，综合能力强",
            "Anthropic (Claude)" => "— Claude 系列模型，擅长代码与推理",
            _ => "— 接入任意 OpenAI 兼容 API（如通义千问、Moonshot、本地模型等）",
        };
        println!("    {}. {} {}", num, provider.name, desc);
    }
    println!();

    loop {
        let choice = read_line_with_default("  请输入编号", "1")?;
        match choice.parse::<usize>() {
            Ok(n) if n >= 1 && n <= PROVIDERS.len() => {
                let idx = n - 1;
                println!("  ✓ 已选择: {}", PROVIDERS[idx].name);
                return Ok(idx);
            }
            _ => {
                println!("  ✗ 请输入 1 到 {} 之间的数字。", PROVIDERS.len());
            }
        }
    }
}

// ── 步骤 4：API Key ──────────────────────────────────────────────────────

fn step_api_key(provider: &ProviderPreset, nexus_home: &Path) -> io::Result<String> {
    println!();
    print_separator("第 4 步：API Key");
    println!();
    println!("  请输入你的 {} API Key。", provider.name);
    if !provider.key_url.is_empty() {
        println!("  获取地址: {}", provider.key_url);
    }
    println!("  环境变量: {}", provider.env_var_hint);
    println!();
    println!("  (可以留空，后续在 Nexus 内通过 F2 → 设置 添加)");
    println!();
    let api_key = read_line("  请输入 API Key (输入时可见): ")?;
    if api_key.is_empty() {
        println!();
        println!(
            "  ⚠ 未输入 API Key。你可以在 {} 中手动添加，",
            nexus_home.join("config.toml").display()
        );
        println!("    或进入 Nexus 后按 F2 → 设置 → API Key。");
    }
    Ok(api_key)
}

// ── 步骤 5：选择模型 ─────────────────────────────────────────────────────

fn step_model_selection(provider: &ProviderPreset) -> io::Result<(&'static str, &'static str, u64)> {
    println!();
    print_separator("第 5 步：默认模型");
    println!();

    if provider.models.is_empty() {
        // 自定义提供商：让用户手动输入
        println!("  请输入模型 ID（如 deepseek-v4-pro、gpt-5.2 等）：");
        println!();
        let model_id = read_line_with_default("  模型 ID", "deepseek-v4-pro")?;
        println!("  请输入该模型的上下文窗口大小（tokens）：");
        println!();
        let ctx_str = read_line_with_default("  上下文窗口", "128000")?;
        let ctx: u64 = ctx_str.parse().unwrap_or(128_000);
        println!("  ✓ 模型: {}, 上下文窗口: {} tokens", model_id, ctx);
        // 使用 model_id 的克隆作为显示名称
        let leaked_name: &'static str = Box::leak(model_id.clone().into_boxed_str());
        let leaked_id: &'static str = Box::leak(model_id.into_boxed_str());
        return Ok((leaked_id, leaked_name, ctx));
    }

    println!("  选择每次启动时默认使用的模型（进入 Nexus 后可随时切换）:");
    println!();
    for (i, &(_, display_name, _)) in provider.models.iter().enumerate() {
        println!("    {}. {}", i + 1, display_name);
    }
    println!();

    let default_choice = "1";
    let choice = read_line_with_default("  请输入编号", default_choice)?;
    let idx: usize = choice.parse::<usize>().unwrap_or(1).saturating_sub(1);
    let idx = idx.min(provider.models.len() - 1);

    let &(model_id, model_name, context_window) = &provider.models[idx];
    println!("  ✓ 默认模型: {}", model_name);
    Ok((model_id, model_name, context_window))
}

// ── 步骤 6：API 端点 ─────────────────────────────────────────────────────

fn step_api_endpoint(provider: &ProviderPreset) -> io::Result<String> {
    println!();
    print_separator("第 6 步：API 端点");
    println!();

    if provider.base_url.is_empty() {
        // 自定义提供商：让用户输入
        println!("  请输入 API 端点地址：");
        println!("  (如 https://api.deepseek.com/v1 或 http://localhost:11434/v1)");
        println!();
        read_line_with_default("  API 端点", "https://api.deepseek.com/v1")
    } else {
        println!("  {} API 地址，一般无需修改。", provider.name);
        println!();
        read_line_with_default("  请输入 API 地址", provider.base_url)
    }
}

// ── 步骤 7：思考深度 ─────────────────────────────────────────────────────

fn step_thinking_depth() -> io::Result<String> {
    println!();
    print_separator("第 7 步：思考深度");
    println!();
    println!("  推理模型支持两种思考深度:");
    println!();
    println!("    1. 标准 (high) — 平衡速度与质量，适合大多数任务 (推荐)");
    println!("    2. 深度 (max)  — 更深推理，适合复杂算法/调试，速度较慢");
    println!();
    let choice = read_line_with_default("  请输入编号 1 或 2", "1")?;
    Ok(match choice.as_str() {
        "2" => "xhigh".to_string(),
        _ => "high".to_string(),
    })
}

// ── 写入 config.toml ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn write_config_toml(
    path: &Path,
    username: &str,
    model_id: &str,
    model_name: &str,
    api_key: &str,
    api_base_url: &str,
    api_backend: &str,
    context_window: u64,
    supports_reasoning: bool,
    reasoning_effort: &str,
) -> io::Result<()> {
    let has_api_key = !api_key.is_empty();
    let has_reasoning = supports_reasoning && !reasoning_effort.is_empty();

    let mut content = String::new();
    content.push_str("# Nexus 配置文件 — 由首次运行向导自动生成\n");
    content.push_str("# 可手动编辑。运行 `nexus --help` 查看所有选项。\n\n");

    // [startup]
    content.push_str("[startup]\n");
    content.push_str("wizard_completed = true\n");
    content.push_str(&format!("username = \"{}\"\n\n", username));

    // [models]
    content.push_str("[models]\n");
    content.push_str(&format!("default = \"{}\"\n", model_id));
    if has_reasoning {
        content.push_str(&format!(
            "default_reasoning_effort = \"{}\"\n",
            reasoning_effort
        ));
    }
    content.push('\n');

    // [model."<id>"]
    content.push_str(&format!("[model.\"{}\"]\n", model_id));
    content.push_str(&format!("model = \"{}\"\n", model_id));
    content.push_str(&format!("name = \"{}\"\n", model_name));
    content.push_str(&format!("base_url = \"{}\"\n", api_base_url));
    content.push_str(&format!("api_backend = \"{}\"\n", api_backend));
    content.push_str(&format!("context_window = {}\n", context_window));
    if supports_reasoning {
        content.push_str("supports_reasoning_effort = true\n");
    }
    if has_api_key {
        content.push_str(&format!("api_key = \"{}\"\n", api_key));
    }

    // [endpoints]
    content.push_str("\n[endpoints]\n");
    content.push_str(&format!("xai_api_base_url = \"{}\"\n", api_base_url));
    if has_api_key {
        content.push_str(&format!("management_api_key = \"{}\"\n", api_key));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

// ── 时间格式化 ───────────────────────────────────────────────────────────

fn format_time_rfc3339(unix_secs: u64) -> String {
    let secs_per_day: u64 = 86400;
    let days_since_epoch = unix_secs / secs_per_day;
    let remaining_secs = unix_secs % secs_per_day;

    let (year, month, day) = days_to_date(days_since_epoch);
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let secs = remaining_secs % 60;

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{secs:02}Z")
}

fn days_to_date(mut days: u64) -> (u64, u64, u64) {
    days += 719468;
    let era = days / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}
