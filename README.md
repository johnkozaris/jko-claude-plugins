<p align="center">
  <h1 align="center">jko-claude-plugins</h1>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/plugins-5-orange" alt="Plugins: 5">
  <img src="https://img.shields.io/badge/commands-15-green" alt="Commands: 15">
  <img src="https://img.shields.io/badge/Claude_Code-plugin_marketplace-8A2BE2" alt="Claude Code Plugin Marketplace">
</p>

These are the guidelines and patterns I use across my projects. They help Claude review code, catch mistakes, and stay consistent with how I actually want things built. Each plugin covers a stack I work in.

## Plugins

| Plugin | Skill | Cmds | What it covers |
|--------|-------|:----:|----------------|
| **[rust](plugins/rust/)** | `rust-expert` | 5 | Ownership, async, unsafe, error handling, type design, anti-patterns |
| **[esp32-cpp](plugins/esp32-cpp/)** | `esp32-expert` | 4 | FreeRTOS, ESP-IDF/PlatformIO, peripherals, memory, all ESP32 variants |
| **[python-backend](plugins/python-backend/)** | `python-backend-expert` | 3 | Litestar, FastAPI, SQLAlchemy, hexagonal architecture, async patterns |
| **[swiftui](plugins/swiftui/)** | `swiftui-expert` | 1 | iOS/macOS/visionOS patterns, Liquid Glass, accessibility |
| **[dead-code](plugins/dead-code/)** | `dead-code-expert` | 2 | Unused imports, functions, classes, duplicates, any language |

## Install

```bash
# Add the marketplace
/plugin marketplace add johnkozaris/jko-claude-plugins

# Install whichever plugins you want
/plugin install rust@jko-claude-plugins
/plugin install esp32-cpp@jko-claude-plugins
/plugin install python-backend@jko-claude-plugins
/plugin install swiftui@jko-claude-plugins
/plugin install dead-code@jko-claude-plugins
```

Or try one without installing:

```bash
claude --plugin-dir /path/to/jko-claude-plugins/plugins/rust
```

## Commands

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

Each plugin has a SKILL.md that tells Claude what to look for and a bunch of reference files with the actual patterns, anti-patterns, and examples. Claude only loads the references it needs for the current task so it doesn't waste context.

Hooks run automatically on file saves. `cargo check` after editing `.rs` files, flagging common mistakes in Python/C++ edits, that kind of thing.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
