# Nexus

Nexus — AI coding assistant in your terminal. Fast, flicker-free CLI built for plans, subagents, and parallel work.

**[Homepage](https://github.com/nexus/nexus)**

## Install

```bash
curl -fsSL https://github.com/nexus/nexus/releases/latest/install.sh | bash
```

Or install with npm:

```bash
npm i -g @nexus/nexus
```

## Get Started

```bash
# Launch the interactive TUI
nexus

# Run a single task
nexus -p "Explain this codebase"
```

On first launch, Nexus opens your browser to authenticate. For CI or headless environments, use an API key:

```bash
export SAGE_API_KEY="sk-..."
```

## Update

```bash
nexus update
```

Or if installed via npm:

```bash
npm i -g @nexus/nexus@latest
```

## Supported Platforms

| Platform | Architecture |
|---|---|
| macOS | Apple Silicon (arm64) |
| Linux | x86_64, arm64 |
| Windows | x86_64 |

## Documentation

For full documentation including configuration, MCP servers, custom models, headless mode, agent mode, and more, see the [Nexus documentation](https://github.com/nexus/nexus).

## Feedback

Run `/feedback` inside Nexus to report issues or send feedback directly.
