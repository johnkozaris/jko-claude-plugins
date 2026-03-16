# jko-claude-plugins

Claude Code plugin marketplace with specialized skills and commands.

## Project Structure

```
myClaudeSkills/
├── .claude-plugin/marketplace.json    # Marketplace manifest
├── plugins/
│   ├── swiftui/                       # SwiftUI expert plugin
│   ├── rust/                          # Rust expert plugin
│   ├── esp32-cpp/                     # ESP32 C++ firmware plugin
│   ├── python-backend/                # Python backend plugin
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
- One type per file in skills
- Skills: lean SKILL.md (1,500-2,000 words), detailed references/ on-demand
- Commands: instructions FOR Claude, not messages to user
- Always use `${CLAUDE_PLUGIN_ROOT}` for portable paths
- Skill descriptions: third-person with specific trigger phrases
- Skill bodies: imperative form (verb-first)
