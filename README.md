<p align="center">
  <h1 align="center">jko-claude-plugins</h1>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/plugins-5-orange" alt="Plugins: 5">
  <img src="https://img.shields.io/badge/commands-15-green" alt="Commands: 15">
  <img src="https://img.shields.io/badge/Claude_Code-plugin_marketplace-8A2BE2" alt="Claude Code Plugin Marketplace">
</p>

---

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

**Rust:** `/rust-critique` `/rust-harden` `/rust-types` `/rust-polish` `/rust-teach`

**ESP32:** `/esp-harden` `/esp-debug` `/esp-optimize` `/esp-teach`

**Python:** `/py-critique` `/py-harden` `/py-structure`

**SwiftUI:** `/swift-critique`

**Dead Code:** `/dead-code-scan` `/dead-code-clean`

## How it works

Each plugin has a SKILL.md that tells Claude what to look for and a bunch of reference files with the actual patterns, anti-patterns, and examples. Claude only loads the references it needs for the current task so it doesn't waste context.

Hooks run automatically on file saves. `cargo check` after editing `.rs` files, flagging common mistakes in Python/C++ edits, that kind of thing.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
