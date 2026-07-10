# jko-claude-plugins

Multi-CLI plugin marketplace with specialized skills and commands.

## Project Structure

```
myClaudeSkills/
├── .github/plugin/marketplace.json    # GitHub Copilot CLI marketplace manifest
├── .claude-plugin/marketplace.json    # Claude Code marketplace manifest
├── plugins/
│   ├── swiftui/                       # SwiftUI expert plugin
│   ├── rust/                          # Rust expert plugin
│   ├── esp32-cpp/                     # ESP32 C++ firmware plugin
│   ├── python-backend/                # Python backend plugin
│   ├── dotnet-backend/                # .NET backend plugin
│   ├── dead-code/                     # Dead code detection plugin
│   ├── backend-validator/             # Token acquisition + HTTP/WS validation (hurl, oauth2c, websocat)
│   ├── peekaboo-macos-validator/      # macOS app UI validation via peekaboo CLI
│   ├── electron-playwright-validator/ # Electron app validation via persistent CDP session (e-cli)
│   ├── maestro-mobile-validator/      # iOS/Android flow validation via Maestro
│   ├── seam-probe/                    # FFI dylib / UDS seam probing (Rust CLI built on demand)
│   └── claude-mastery/                # Agent/skill/workflow design guidance
```

## Installing This Marketplace

```bash
# Register as a marketplace for local development
claude plugin marketplace add "$PWD"

# Then install any plugin from the local checkout
claude plugin install <plugin-name>@jko-claude-plugins
```

After publishing or updating the GitHub-hosted marketplace, end users should use:

```bash
claude plugin marketplace add johnkozaris/jko-claude-plugins
claude plugin install <plugin-name>@jko-claude-plugins
```

Or load a single plugin for one session:
```bash
claude --plugin-dir "$PWD/plugins/<plugin-name>"
```

## Local Iteration

- In Claude Code, run `/reload-plugins` after installing, enabling, disabling, or updating plugins in the current session.
- In GitHub Copilot CLI, reinstall a local plugin after edits because installed plugins are cached.

## Conventions

- Before committing plugin changes, run `python3 scripts/lint-plugins.py` (no CI — local only) and include its output. It checks description length limits, pattern-ID citation drift (CODE-/ARCH-/AP-/DN-), marketplace↔plugin.json version sync, orphaned reference files, private project-name leakage, empty hooks stubs, and dangling slash-command references.
- Never name private projects in plugin content — plugins must be app-agnostic (app specifics go in env vars, manifests, or arguments). The linter's `PRIVATE_NAMES` blocklist enforces this.
- When a plugin's content changes substantively, bump its version in BOTH plugin.json files and BOTH marketplace.json files (the linter catches mismatches).

- Plugin names: kebab-case
- Keep GitHub Copilot CLI component paths in `.github/plugin/plugin.json`; when both manifest styles exist, Copilot reads the `.github/plugin/*` manifests.
- Keep `.claude-plugin/plugin.json` metadata-only unless Claude needs non-default paths; Claude Code can use default discovery for `skills/`, `commands/`, and `hooks/` without duplicating those component paths.
- One type per file in skills
- Skills: lean SKILL.md (1,500-2,000 words), detailed references/ on-demand
- Commands: instructions FOR Claude, not messages to user
- Always use `${CLAUDE_PLUGIN_ROOT}` for portable paths
- Skill descriptions: third-person with specific trigger phrases
- Skill bodies: imperative form (verb-first)
