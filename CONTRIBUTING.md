# Contributing to jko-claude-plugins

Thanks for your interest in contributing! This marketplace contains specialized Claude Code plugins built with care, and contributions that maintain that quality bar are welcome.

## Ways to Contribute

- **Bug reports** -- Found incorrect advice in a skill or a broken command? Open an issue.
- **Improve existing plugins** -- Better examples, additional anti-patterns, updated library versions.
- **New plugins** -- Propose a new domain-specific plugin via a discussion or issue first.
- **Documentation** -- Typo fixes, clearer installation instructions, better examples.

## Getting Started

1. Fork and clone the repo
2. Look at an existing plugin (e.g., `plugins/rust/`) to understand the structure

## Plugin Quality Standards

Every plugin in this marketplace follows strict quality guidelines:

- **Skills** use lean SKILL.md files (1,500-2,000 words) with detailed `references/` loaded on demand
- **Commands** contain instructions FOR Claude, not messages to the user
- **All paths** use `${CLAUDE_PLUGIN_ROOT}` for portability
- **Facts are verified** -- no fake CVEs, no unsourced statistics, no hallucinated library versions
- **Anti-patterns include both BAD and GOOD code** examples
- **AI slop is banned** -- no filler phrases, no over-engineered abstractions, no unnecessary backward compatibility

## Validation

Before submitting, validate your plugin:

```bash
# Validate marketplace and plugin manifests
python3 scripts/check_plugin_manifests.py

# Load plugin in a session and test commands
claude --plugin-dir ./plugins/<your-plugin>

# Run the plugin validator agent (if you have the plugin-dev plugin installed)
# /validate-plugin
```

To enable the checked-in pre-commit hook for this repository:

```bash
git config core.hooksPath .githooks
```

After that, `git commit` will automatically run the manifest validator before each commit.

## Conventions

| Convention | Rule |
|---|---|
| Plugin names | `kebab-case` |
| File organization | One type per file in skills |
| Skill descriptions | Third-person with specific trigger phrases |
| Skill bodies | Imperative form (verb-first) |
| Commit messages | Concise, focused on the "why" |

## Submitting Changes

1. Create a feature branch from `main`
2. Make your changes
3. Test the plugin locally with `claude --plugin-dir`
4. Open a pull request with a clear description of what changed and why

## Code of Conduct

Be respectful, constructive, and focused on quality. This is a craft-oriented project -- we care about getting the details right.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
