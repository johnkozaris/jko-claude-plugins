<h1 align="center">jko-skills</h1>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/plugins-9-orange" alt="Plugins: 9">
  <img src="https://img.shields.io/badge/skills-13-green" alt="Skills: 13">
</p>

Focused plugins and skills for Claude Code, GitHub Copilot CLI, Codex CLI, and
OpenCode.

This marketplace deliberately avoids generic language handbooks. Modern agents
can inspect a repository, run its tools, and research current framework APIs.
Plugins earn their context by contributing one of two things:

- opinionated domain knowledge that is difficult to infer from the code alone;
- an executable interface for verification, probing, or automation.

Detailed material lives behind progressive disclosure. Additional skills
provide thin explicit workflows rather than duplicating their primary skill.

## Plugins

| Plugin | Primary skill | Extra skills | Purpose |
|---|---|:---:|---|
| **[backend-architecture](plugins/backend-architecture/)** | `backend-architecture` | 0 | Small cross-stack guide for consequential production architecture decisions |
| **[code-cleanup](plugins/code-cleanup/)** | `code-cleanup` | 0 | Removes agent trajectory residue, dead islands, split-brain authority, and stale dependencies |
| **[swiftui](plugins/swiftui/)** | `swiftui-expert` | 0 | SwiftUI-specific state, lifecycle, platform, accessibility, and architecture judgment |
| **[esp32-cpp](plugins/esp32-cpp/)** | `esp32-expert` | 0 | ESP32 hardware, FreeRTOS, memory, peripheral, and field-failure judgment |
| **[backend-validator](plugins/backend-validator/)** | `validate-api` | 2 | Authenticated HTTP and WebSocket validation with Hurl, websocat, and oauth2c |
| **[maestro-mobile-validator](plugins/maestro-mobile-validator/)** | `mobile-flows-maestro` | 1 | iOS and Android flow validation with Maestro |
| **[electron-playwright-validator](plugins/electron-playwright-validator/)** | `electron-playwright-validator` | 0 | Persistent Electron validation through Playwright CDP |
| **[peekaboo-macos-validator](plugins/peekaboo-macos-validator/)** | `peekaboo` | 1 | Native macOS UI automation and bounded-context visual critique |
| **[seam-probe](plugins/seam-probe/)** | `seam-probe` | 0 | Manifest-driven FFI dylib and Unix-domain socket probing |

## Install

### Claude Code

```bash
claude plugin marketplace add johnkozaris/jko-claude-plugins
claude plugin install <plugin-name>@jko-skills
```

For local development:

```bash
claude plugin marketplace add "$PWD"
claude plugin install <plugin-name>@jko-skills
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
copilot plugin install <plugin-name>@jko-skills
```

Installed local plugins are cached; reinstall after edits.

### Existing marketplace registrations

The repository remains `johnkozaris/jko-claude-plugins`, but its marketplace
name is now `jko-skills`. Existing users should uninstall plugins tied to
`jko-claude-plugins`, remove that marketplace registration, add the repository
again, and reinstall using the `@jko-skills` suffix.

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

## Additional workflow skills

Claude Code invokes plugin skills as `/plugin-name:skill-name`. Copilot CLI
uses the skill name directly.

| Plugin | Skills |
|---|---|
| Backend validation | `validate-ws`, `get-dev-token` |
| Maestro | `validate-mobile` |
| Peekaboo | `validate-macos-app` |

## Discovery model

Each plugin owns shared skill directories and thin host-specific manifests.
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
