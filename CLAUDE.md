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
│   └── dead-code/                     # Dead code detection plugin
```

## Installing This Marketplace

```bash
# Register as a marketplace
claude plugin marketplace add /Users/john/Repos/myClaudeSkills

# Then install any plugin
claude plugin install <plugin-name>@jko-claude-plugins
```

Or load a single plugin for one session:
```bash
claude --plugin-dir /Users/john/Repos/myClaudeSkills/plugins/<plugin-name>
```

## Conventions

- Plugin names: kebab-case
- Keep GitHub Copilot CLI component paths in `.github/plugin/plugin.json`; when both manifest styles exist, Copilot reads the `.github/plugin/*` manifests.
- Keep `.claude-plugin/plugin.json` metadata-only unless Claude needs non-default paths; Claude Code can use default discovery for `skills/`, `commands/`, and `hooks/` without duplicating those component paths.
- One type per file in skills
- Skills: lean SKILL.md (1,500-2,000 words), detailed references/ on-demand
- Commands: instructions FOR Claude, not messages to user
- Always use `${CLAUDE_PLUGIN_ROOT}` for portable paths
- Skill descriptions: third-person with specific trigger phrases
- Skill bodies: imperative form (verb-first)
