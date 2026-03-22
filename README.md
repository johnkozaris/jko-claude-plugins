<p align="center">
  <h1 align="center">jko-claude-plugins</h1>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/plugins-6-orange" alt="Plugins: 6">
  <img src="https://img.shields.io/badge/commands-19-green" alt="Commands: 19">
</p>

These are the guidelines and patterns I use across my projects. They help coding agents review code, catch mistakes, and stay consistent with how I actually want things built. Each plugin covers a stack I work in.

Skills use the shared [Agent Skills specification](https://developers.openai.com/codex/skills/) — install as full plugins or individual skills.

<h4 align="center">Works with</h4>

<p align="center">
  <a href="#claude-code"><img src="https://img.shields.io/badge/Claude_Code-D97757?style=for-the-badge&logo=claude&logoColor=fff" alt="Claude Code"></a>
  <a href="#github-copilot-cli"><img src="https://img.shields.io/badge/Copilot_CLI-000?style=for-the-badge&logo=githubcopilot&logoColor=fff" alt="GitHub Copilot CLI"></a>
  <a href="#openai-codex-cli"><img src="https://img.shields.io/badge/Codex_CLI-412991?style=for-the-badge&logo=openai&logoColor=fff" alt="OpenAI Codex CLI"></a>
  <a href="#opencode"><img src="https://img.shields.io/badge/⌬_OpenCode-18181B?style=for-the-badge" alt="OpenCode"></a>
</p>

## Plugins

| Plugin | Skill | Cmds | What it covers |
|--------|-------|:----:|----------------|
| **[rust](plugins/rust/)** | `rust` | 5 | Ownership, async, unsafe, error handling, type design, anti-patterns |
| **[esp32-cpp](plugins/esp32-cpp/)** | `esp32` | 4 | FreeRTOS, ESP-IDF/PlatformIO, peripherals, memory, all ESP32 variants |
| **[python-backend](plugins/python-backend/)** | `python-backend` | 3 | Litestar, FastAPI, SQLAlchemy, hexagonal architecture, async patterns |
| **[dotnet-backend](plugins/dotnet-backend/)** | `dotnet-backend` | 4 | Pure .NET 10 backend review for Kestrel hosting, REST, SignalR, EF Core, and DI |
| **[swiftui](plugins/swiftui/)** | `swiftui` | 1 | iOS/macOS/visionOS patterns, Liquid Glass, accessibility |
| **[dead-code](plugins/dead-code/)** | `dead-code` | 2 | Unused imports, functions, classes, duplicates, any language |

## Install

### Claude Code

```bash
# Add the marketplace
claude plugin marketplace add johnkozaris/jko-claude-plugins

# Install whichever plugins you want
claude plugin install rust@jko-claude-plugins
claude plugin install esp32-cpp@jko-claude-plugins
claude plugin install python-backend@jko-claude-plugins
claude plugin install dotnet-backend@jko-claude-plugins
claude plugin install swiftui@jko-claude-plugins
claude plugin install dead-code@jko-claude-plugins
```

Or try one without installing:

```bash
claude --plugin-dir /path/to/jko-claude-plugins/plugins/rust
```

### GitHub Copilot CLI

```bash
# Add the marketplace
copilot plugin marketplace add johnkozaris/jko-claude-plugins

# Install a plugin
copilot plugin install rust@jko-claude-plugins
```

### OpenAI Codex CLI

Codex installs skills directly — no plugin marketplace.

```bash
# Inside Codex, install a skill directory from GitHub
$skill-installer install https://github.com/johnkozaris/jko-claude-plugins/tree/main/plugins/rust/skills/rust-expert
$skill-installer install https://github.com/johnkozaris/jko-claude-plugins/tree/main/plugins/esp32-cpp/skills/esp32-expert
$skill-installer install https://github.com/johnkozaris/jko-claude-plugins/tree/main/plugins/python-backend/skills/python-backend-expert
$skill-installer install https://github.com/johnkozaris/jko-claude-plugins/tree/main/plugins/dotnet-backend/skills/dotnet-backend-expert
$skill-installer install https://github.com/johnkozaris/jko-claude-plugins/tree/main/plugins/swiftui/skills/swiftui-expert
$skill-installer install https://github.com/johnkozaris/jko-claude-plugins/tree/main/plugins/dead-code/skills/dead-code-expert
```

### OpenCode

Install skills via [skills.sh](https://skills.sh) or copy manually.

```bash
# Via skills.sh (auto-detects installed agents)
npx skills add johnkozaris/jko-claude-plugins --full-depth

# Or copy a skill directory manually
cp -r plugins/rust/skills/rust-expert ~/.config/opencode/skills/rust-expert
```

Use the shell commands above from your terminal. Inside Claude Code and Copilot CLI, the interactive `/plugin ...` equivalents work too.

## Discovery model

The repo uses one shared skill directory per plugin and thin CLI-specific packaging around it.

| Tool | What it auto-discovers in a working repo | How install/discovery works here |
|------|------------------------------------------|----------------------------------|
| **Claude Code** | `.claude/skills/`, nested `.claude/skills/`, `.claude/commands/`, `.claude/agents/` | Install the marketplace or load a plugin with `claude --plugin-dir ...`. Claude uses `.claude-plugin/marketplace.json` at the repo root. |
| **GitHub Copilot CLI** | `.github/skills/`, `.claude/skills/`, `.github/agents/`, `.claude/agents/`, `.github/hooks/` | Install the marketplace or a plugin with `copilot plugin ...`. Copilot uses `.github/plugin/marketplace.json` and prefers `.github/plugin/plugin.json` when both manifest styles exist. |
| **OpenAI Codex CLI** | `.agents/skills/` from the current directory up to repo root, `~/.agents/skills/`, `/etc/codex/skills/` | Install individual skill directories with `$skill-installer` or copy them into a scanned `.agents/skills/` location. There is no Codex plugin marketplace layer here. |

The important part is that all three tools discover the **skill directory**, not arbitrary `plugins/**/skills/**` paths. A skill can safely contain `references/`, `scripts/`, `assets/`, and other supporting files next to `SKILL.md`, but the parent skill directory still has to be reached through that tool's discovery or installation mechanism.

## Commands

Claude Code and Copilot CLI can load the plugin command adapters in `commands/`. Codex and OpenCode consume the skill directories and their supporting files, not the plugin command layer.

### Rust

| Command | What it does |
|---------|-------------|
| `/rust-critique` | Full code review for soundness, ownership, error handling, types, async, performance |
| `/rust-harden` | Replace `unwrap` with proper errors, add SAFETY comments to unsafe, validate inputs |
| `/rust-types` | Replace primitives with newtypes, booleans with enums, make illegal states unrepresentable |
| `/rust-polish` | Pre-merge cleanup: dead code, doc comments, clippy, debug artifacts |
| `/rust-teach` | One-time: scans your project and writes Rust conventions to CLAUDE.md |

### ESP32

| Command | What it does |
|---------|-------------|
| `/esp-harden` | Scan firmware for field failures, crashes, memory issues, and security problems |
| `/esp-debug` | Help debug crashes, hangs, and peripheral issues |
| `/esp-optimize` | Optimize for performance, memory, power, or binary size |
| `/esp-teach` | One-time: discover hardware, find datasheets, persist context to CLAUDE.md |

### Python

| Command | What it does |
|---------|-------------|
| `/py-critique` | Architecture review: SOLID compliance, layer boundaries, anti-patterns, design quality |
| `/py-harden` | Run the full anti-pattern catalog (AP-01 through AP-22) and fix what it finds |
| `/py-structure` | Check project layout, file sizes, module splitting, hexagonal architecture compliance |

### .NET

| Command | What it does |
|---------|-------------|
| `/dotnet-critique` | Full backend architecture review for Kestrel-hosted REST and SignalR services |
| `/dotnet-harden` | Scan for high-impact backend anti-patterns like sync-over-async and fragile state |
| `/dotnet-structure` | Check solution layout, boundaries, and oversized files |
| `/dotnet-teach` | One-time: scan a backend and persist conventions to `CLAUDE.md` |

### SwiftUI

| Command | What it does |
|---------|-------------|
| `/swift-critique` | Review SwiftUI code for patterns, design, accessibility, and performance |

### Dead Code

| Command | What it does |
|---------|-------------|
| `/dead-code-scan` | Read-only scan for unused imports, functions, classes, duplicates |
| `/dead-code-clean` | Actually remove the dead code it finds |

## How it works

Each plugin has a `SKILL.md` that tells the agent what to look for and a `references/` directory with the actual patterns, anti-patterns, and examples. The agent only loads the supporting files it needs for the current task, so it does not waste context.

Hook config files are scaffolded in the plugin directories, but this repo does not currently ship active runtime hooks.

### Cross-tool compatibility

| Feature | Claude Code | Copilot CLI | Codex CLI | OpenCode |
|---------|:-----------:|:-----------:|:---------:|:--------:|
| Skills + references | yes | yes | yes | yes |
| Slash commands | yes | yes | -- | -- |
| Hooks | yes | yes | -- | -- |
| Plugin marketplace | yes | yes | -- | -- |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
