//! Sage 首次运行设置向导。
//!
//! 在 `~/.sage/config.toml` 不存在或 `[startup].wizard_completed` 未设置时，
//! 引导用户完成初始配置：数据目录、API Key、模型选择、思考深度。

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// 重定向文件名：保存在 OS 默认目录（`~/.sage/`）下，内容为实际的 Sage 数据目录路径。
/// 解决无 bat 脚本直接双击 exe 时，`SAGE_HOME` 环境变量无法跨进程持久化的问题。
const SAGE_HOME_REDIRECT_FILENAME: &str = "sage-home-path";

/// 获取用户主目录，不依赖 `dirs` crate。
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

/// OS 默认的 `.sage` 目录路径（不跟随重定向）。
fn os_default_dot_sage() -> Option<PathBuf> {
    user_home_dir().map(|h| h.join(".sage"))
}

/// 读取重定向文件，返回用户之前选择的 Sage 数据目录路径。
fn read_sage_home_redirect() -> Option<PathBuf> {
    let redirect_file = os_default_dot_sage()?.join(SAGE_HOME_REDIRECT_FILENAME);
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

/// 将用户选择的 Sage 数据目录路径写入重定向文件。
/// 这样后续启动（即使没有 `SAGE_HOME` 环境变量）也能找到正确的目录。
fn save_sage_home_redirect(path: &Path) -> io::Result<()> {
    if let Some(dot_sage) = os_default_dot_sage() {
        std::fs::create_dir_all(&dot_sage)?;
        std::fs::write(dot_sage.join(SAGE_HOME_REDIRECT_FILENAME), path.display().to_string())?;
    }
    Ok(())
}

/// Sage 默认数据目录：
/// 1. `SAGE_HOME` 环境变量
/// 2. `~/.sage/sage-home-path` 重定向文件（首次向导写入）
/// 3. 回退到 `~/.sage`
fn default_sage_home() -> PathBuf {
    if let Ok(sage_home) = std::env::var("SAGE_HOME") {
        let p = PathBuf::from(&sage_home);
        if p.is_absolute() || sage_home.starts_with("~") {
            return p;
        }
    }
    if let Some(redirected) = read_sage_home_redirect() {
        return redirected;
    }
    if let Some(home) = user_home_dir() {
        return home.join(".sage");
    }
    PathBuf::from(".sage")
}

fn read_line(prompt: &str) -> io::Result<String> {
    let mut stdout = io::stdout();
    write!(stdout, "{}", prompt)?;
    stdout.flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// 读取一行输入，带有默认值。
/// 提示格式: "  请输入路径 (直接回车使用默认: {}): "
fn read_line_with_default(prompt: &str, default: &str) -> io::Result<String> {
    let full_prompt = format!("{} (直接回车 = \"{}\"): ", prompt, default);
    let answer = read_line(&full_prompt)?;
    if answer.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(answer)
    }
}

/// 判断是否需要运行首次设置向导。
///
/// 以下情况跳过向导：
/// - `--version` / `--help` 等纯信息参数
/// - `config.toml` 已存在且包含 `wizard_completed = true`
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
    default_sage_home().join("config.toml")
}

/// 运行首次设置向导，引导用户完成基本配置，写入 `config.toml`，
/// 返回选择的 Sage 数据目录。
pub fn run_setup_wizard() -> io::Result<PathBuf> {
    print_banner();

    // ── 第 1 步：数据目录 ─────────────────────────────────────────────
    let sage_home = step_data_directory()?;

    // ── 第 2 步：API Key ───────────────────────────────────────────────
    let api_key = step_api_key(&sage_home)?;

    // ── 第 3 步：默认模型 ─────────────────────────────────────────────
    let (model_id, model_name) = step_model_selection()?;

    // ── 第 4 步：API 端点 ─────────────────────────────────────────────
    let api_base_url = step_api_endpoint()?;

    // ── 第 5 步：思考深度 ─────────────────────────────────────────────
    let reasoning_effort = step_thinking_depth()?;

    // ── 写入配置 ──────────────────────────────────────────────────────
    println!();
    print_separator("正在保存配置");
    let config_path = sage_home.join("config.toml");
    write_config_toml(
        &config_path,
        model_id,
        model_name,
        &api_key,
        &api_base_url,
        &reasoning_effort,
    )?;

    // 设 SAGE_HOME 环境变量，确保后续代码使用正确的目录。
    // SAFETY: 此时单线程，没有其他线程在读取环境变量。
    unsafe {
        std::env::set_var("SAGE_HOME", sage_home.as_os_str());
    }

    println!("  ✓ 配置文件已保存到: {}", config_path.display());
    println!();
    println!("  Sage 已准备就绪，正在进入主界面……");
    println!();
    print_separator("");

    Ok(sage_home)
}

// ── 各步骤函数 ────────────────────────────────────────────────────────

fn print_banner() {
    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║                                                  ║");
    println!("  ║      S A G E  —  你的 AI 编程搭档                ║");
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

fn step_data_directory() -> io::Result<PathBuf> {
    println!();
    print_separator("第 1 步：数据目录");
    println!();
    println!("  Sage 的所有数据（配置、会话记录、插件等）都存放在一个");
    println!("  目录中。建议选一个空间充足的位置（如 F:\\sage-home）。");
    println!();
    let default = default_sage_home();
    loop {
        let answer = read_line_with_default(
            "  请输入数据目录路径",
            &default.display().to_string(),
        )?;
        let sage_home = PathBuf::from(&answer);
        // 尝试展开 ~ 路径
        let sage_home = if answer.starts_with("~") {
            if let Some(home) = user_home_dir() {
                let stripped = answer.strip_prefix("~/").unwrap_or(&answer);
                let stripped = stripped.strip_prefix("~").unwrap_or(stripped);
                home.join(stripped)
            } else {
                sage_home
            }
        } else {
            sage_home
        };
        match std::fs::create_dir_all(&sage_home) {
            Ok(()) => {
                // 持久化：写入重定向文件，下次启动无需环境变量也能找到此目录
                if let Err(e) = save_sage_home_redirect(&sage_home) {
                    eprintln!("  ⚠ 无法保存目录重定向文件: {e}");
                }
                println!("  ✓ 使用目录: {}", sage_home.display());
                return Ok(sage_home);
            }
            Err(e) => {
                println!("  ✗ 无法创建目录: {e}");
                println!("  请检查路径是否正确，或换一个位置重试。");
            }
        }
    }
}

fn step_api_key(sage_home: &std::path::Path) -> io::Result<String> {
    println!();
    print_separator("第 2 步：API Key");
    println!();
    println!("  请输入你的 DeepSeek API Key。");
    println!("  获取地址: https://platform.deepseek.com/api_keys");
    println!("  (可以留空，后续在 Sage 内通过 F2 → 设置 添加)");
    println!();
    let api_key = read_line("  请输入 API Key (输入时可见): ")?;
    if api_key.is_empty() {
        println!();
        println!("  ⚠ 未输入 API Key。你可以在 {} 中手动添加，", sage_home.join("config.toml").display());
        println!("    或进入 Sage 后按 F2 → 设置 → API Key。");
    }
    Ok(api_key)
}

fn step_model_selection() -> io::Result<(&'static str, &'static str)> {
    println!();
    print_separator("第 3 步：默认模型");
    println!();
    println!("  选择每次启动时默认使用的模型（进入 Sage 后可随时切换）:");
    println!();
    println!("    1. DeepSeek V4 Pro   — 最强推理，适合复杂编程任务 (推荐)");
    println!("    2. DeepSeek V4 Flash — 更快响应，适合日常开发");
    println!();
    let choice = read_line_with_default("  请输入编号 1 或 2", "1")?;
    Ok(match choice.as_str() {
        "2" => ("deepseek-v4-flash", "DeepSeek V4 Flash"),
        _   => ("deepseek-v4-pro",   "DeepSeek V4 Pro"),
    })
}

fn step_api_endpoint() -> io::Result<String> {
    println!();
    print_separator("第 4 步：API 端点");
    println!();
    println!("  DeepSeek API 地址，一般无需修改。");
    println!();
    read_line_with_default("  请输入 API 地址", "https://api.deepseek.com/v1")
}

fn step_thinking_depth() -> io::Result<String> {
    println!();
    print_separator("第 5 步：思考深度");
    println!();
    println!("  DeepSeek V4 支持两种推理深度:");
    println!();
    println!("    1. 标准 (high) — 平衡速度与质量，适合大多数任务 (推荐)");
    println!("    2. 深度 (max)  — 更深推理，适合复杂算法/调试，速度较慢");
    println!();
    let choice = read_line_with_default("  请输入编号 1 或 2", "1")?;
    // 注意：内部使用 "xhigh".to_string() 是因为 ReasoningEffort 枚举的变体名，
    // 实际发送到 DeepSeek API 时会通过 as_deepseek_str() 映射为 "max"。
    Ok(match choice.as_str() {
        "2" => "xhigh".to_string(),
        _   => "high".to_string(),
    })
}

// ── 写入 config.toml ──────────────────────────────────────────────────

fn write_config_toml(
    path: &std::path::Path,
    model_id: &str,
    model_name: &str,
    api_key: &str,
    api_base_url: &str,
    reasoning_effort: &str,
) -> io::Result<()> {
    let mut content = String::new();
    content.push_str("# Sage 配置文件 — 由首次运行向导自动生成\n");
    content.push_str("# 可手动编辑。运行 `sage --help` 查看所有选项。\n\n");

    // [startup]
    content.push_str("[startup]\n");
    content.push_str("wizard_completed = true\n\n");

    // [models]
    content.push_str("[models]\n");
    content.push_str(&format!("default = \"{}\"\n", model_id));
    content.push_str(&format!("default_reasoning_effort = \"{}\"\n\n", reasoning_effort));

    // [model."<id>"]
    content.push_str(&format!("[model.\"{}\"]\n", model_id));
    content.push_str(&format!("model = \"{}\"\n", model_id));
    content.push_str(&format!("name = \"{}\"\n", model_name));
    content.push_str(&format!("base_url = \"{}\"\n", api_base_url));
    content.push_str("api_backend = \"chat_completions\"\n");
    content.push_str("context_window = 1000000\n");
    content.push_str("supports_reasoning_effort = true\n");
    if !api_key.is_empty() {
        content.push_str(&format!("api_key = \"{}\"\n", api_key));
    }

    // [endpoints]
    content.push_str("\n[endpoints]\n");
    content.push_str(&format!("xai_api_base_url = \"{}\"\n", api_base_url));
    if !api_key.is_empty() {
        content.push_str(&format!("management_api_key = \"{}\"\n", api_key));
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}
