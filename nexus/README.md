<div align="center">

<h1>
  <img alt="Nexus logo" src="" width="96">
  <br>
  Nexus (<code>nexus</code>)
</h1>

**Nexus** — 开源智能 AI 编程助手，终端原生 TUI 界面。

支持 DeepSeek V4 等主流模型，理解你的代码库，编辑文件，执行 Shell 命令，
代码搜索，多 Agent 协作，Agent Client Protocol (ACP) 扩展。

[从源码构建](#从源码构建) ·
[配置](#配置) ·
[命令参考](#命令参考) ·
[开发](#开发) ·
[许可证](#许可证)

</div>

---

## 特性

- **终端原生 TUI** — 全屏交互界面，支持极简模式
- **DeepSeek V4 深度推理** — 原生 1M token 上下文窗口，thinking 链式推理
- **多 Agent 编排** — 子 Agent 并行处理，Agent 团队协作
- **代码理解** — 全文搜索、符号跳转、项目结构分析
- **扩展系统** — MCP 服务器、Hooks、插件、技能市场
- **会话管理** — 分支会话 (/fork)、回退 (/rewind)、多会话仪表盘
- **跨平台** — Windows / macOS / Linux

## 从源码构建

环境要求：

- **Rust** — 工具链版本由 [`rust-toolchain.toml`](rust-toolchain.toml) 锁定，`rustup` 首次构建时自动安装
- **protoc** — 通过 [`bin/protoc`](bin/protoc) 提供，或使用系统 PATH 中的 `protoc`

```sh
# 开发构建
cargo run -p nexus-bin

# Release 构建
cargo build -p nexus-bin --release
# 输出: target/release/nexus.exe (Windows) / nexus (Linux/macOS)
```

## 配置

首次运行 Nexus 会自动弹出设置向导，引导你配置：

- **API Key** — DeepSeek API Key（或通过 `SAGE_API_KEY` 环境变量设置）
- **模型选择** — DeepSeek V4 Pro / Flash
- **推理深度** — high / max
- **数据目录** — 默认 `~/.sage/`（环境变量 `SAGE_HOME` 可覆盖）

配置文件：`~/.sage/config.toml`

## 命令参考

| 命令 | 说明 |
|------|------|
| `/new` | 新建会话 |
| `/model <name>` | 切换模型 |
| `/effort <level>` | 调整推理深度 |
| `/fork` | 分支当前会话 |
| `/plan` | 进入计划模式 |
| `/compact` | 压缩对话历史 |
| `/rewind` | 回退到之前的对话轮次 |
| `/help` | 浏览全部命令和快捷键 |
| `/mcps` | 查看 MCP 服务器状态 |
| `/cd <path>` | 切换工作目录 |

更多命令使用 `/help` 查看，或阅读 `~/.sage/docs/user-guide/` 下的用户指南。

## 仓库结构

| 路径 | 说明 |
|------|------|
| `crates/codegen/nexus-bin` | 二进制入口 crate，生成 `nexus` 可执行文件 |
| `crates/codegen/nexus-pager` | TUI 界面：对话、提示词、模态框、渲染 |
| `crates/codegen/nexus-shell` | Agent 运行时 + leader/stdio/headless 模式 |
| `crates/codegen/nexus-tools` | 工具实现（终端、文件编辑、搜索等） |
| `crates/codegen/nexus-agent` | Agent 生命周期、系统提示词、提示词模板 |
| `crates/codegen/...` | 其余 crate（配置、MCP、Markdown、沙箱等） |
| `crates/common/`, `crates/build/` | 共享叶 crate |
| `third_party/` | 第三方源码（Mermaid 图表渲染） |

> [!NOTE]
> 根目录 `Cargo.toml` 是工作区清单（workspace members + 依赖版本 + lint rules），由工具生成。

## 开发

```sh
cargo check -p nexus-bin         # 快速验证
cargo test -p nexus-config       # 按 crate 运行测试
cargo clippy -p nexus-bin        # Lint 检查（规则在 clippy.toml）
cargo fmt --all                  # 格式化（规则在 rustfmt.toml）
```

## 许可证

本项目采用 **Apache License, Version 2.0** — 详见 [`LICENSE`](LICENSE)。

第三方及 vendored 代码遵循其原始许可证：

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES) — crates.io / git 依赖、内置 UI 主题、工具实现移植
- [`third_party/NOTICE`](third_party/NOTICE) — vendored Mermaid 渲染栈
