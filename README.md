<h1 align="center">jko-claude-plugins</h1>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/plugins-8-orange" alt="Plugins: 8">
  <img src="https://img.shields.io/badge/commands-10-green" alt="Commands: 10">
</p>

Focused plugins and skills for Claude Code, GitHub Copilot CLI, Codex CLI, and
OpenCode.

This marketplace deliberately avoids generic language handbooks. Modern agents
can inspect a repository, run its tools, and research current framework APIs.
Plugins earn their context by contributing one of two things:

- opinionated domain knowledge that is difficult to infer from the code alone;
- an executable interface for verification, probing, or automation.

Detailed material lives behind progressive disclosure. Commands are thin entry
points rather than alternate copies of a skill.

## Plugins

| Plugin | Skill | Cmds | Purpose |
|---|---|:---:|---|
| **[backend-architecture](plugins/backend-architecture/)** | `backend-architecture` | 0 | Small cross-stack guide for consequential production architecture decisions |
| **[swiftui](plugins/swiftui/)** | `swiftui-expert` | 1 | SwiftUI-specific state, lifecycle, platform, accessibility, and architecture judgment |
| **[esp32-cpp](plugins/esp32-cpp/)** | `esp32-expert` | 1 | ESP32 hardware, FreeRTOS, memory, peripheral, and field-failure judgment |
| **[backend-validator](plugins/backend-validator/)** | `backend-validation` | 3 | Authenticated HTTP and WebSocket validation with Hurl, websocat, and oauth2c |
| **[maestro-mobile-validator](plugins/maestro-mobile-validator/)** | `mobile-flows-maestro` | 1 | iOS and Android flow validation with Maestro |
| **[electron-playwright-validator](plugins/electron-playwright-validator/)** | `electron-playwright-validator` | 1 | Persistent Electron validation through Playwright CDP |
| **[peekaboo-macos-validator](plugins/peekaboo-macos-validator/)** | `peekaboo` | 2 | Native macOS UI automation and bounded-context visual critique |
| **[seam-probe](plugins/seam-probe/)** | `seam-probe` | 1 | Manifest-driven FFI dylib and Unix-domain socket probing |

## Install

### Claude Code

```bash
claude plugin marketplace add johnkozaris/jko-claude-plugins
claude plugin install <plugin-name>@jko-claude-plugins
```

For local development:

```bash
claude plugin marketplace add "$PWD"
claude plugin install <plugin-name>@jko-claude-plugins
```

Or load one plugin for a session:

```bash
claude --plugin-dir "$PWD/plugins/<plugin-name>"
```

Run `/reload-plugins` after installing, enabling, disabling, or updating a
plugin in the current session.

### GitHub Copilot CLI

```bash
copilot plugin marketplace add johnkozaris/jko-claude-plugins
copilot plugin install <plugin-name>@jko-claude-plugins
```

Installed local plugins are cached; reinstall after edits.

### Codex CLI

Codex installs a skill directory directly:

```bash
$skill-installer install https://github.com/johnkozaris/jko-claude-plugins/tree/main/plugins/<plugin>/skills/<skill>
```

Direct skill installs include assets inside the skill directory. `seam-probe`
also relies on plugin-level build hooks and its bundled Rust crate, so install or
clone that full plugin rather than copying only its skill.

### OpenCode

```bash
pnpm dlx skills add johnkozaris/jko-claude-plugins --full-depth
```

## Commands

| Plugin | Commands |
|---|---|
| SwiftUI | `/swift-critique` |
| ESP32 | `/esp-debug` |
| Backend validation | `/validate-api`, `/validate-ws`, `/get-dev-token` |
| Maestro | `/validate-mobile` |
| Electron | `/validate-electron` |
| Peekaboo | `/peekaboo-macos-validator:peekaboo-doctor`, `/peekaboo-macos-validator:validate-macos-app` |
| seam-probe | `/seam-probe:seam-probe-setup` |

## Discovery model

Each plugin owns one shared skill directory and thin host-specific manifests.
Claude Code and Copilot CLI install plugins from their marketplaces. Codex and
OpenCode consume the same skill directories directly.

`SKILL.md` is a lightweight router and opinion layer. References are loaded only
when the task exposes a matching signal. Executable tools and current primary
documentation take precedence over copied command catalogues or version trivia.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

Before committing:

```bash
python3 scripts/lint-plugins.py
```

## License

[MIT](LICENSE)
