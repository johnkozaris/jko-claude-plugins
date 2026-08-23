# Contributing to jko-skills

Thanks for your interest in contributing! This marketplace contains specialized multi-CLI plugins built with care, and contributions that maintain that quality bar are welcome.

## Ways to Contribute

- **Bug reports** -- Found incorrect advice in a skill or a broken command? Open an issue.
- **Improve existing plugins** -- Add demonstrated failure modes, sharpen
  triggers, or replace repeated prose with a better executable interface.
- **New plugins** -- Propose a new domain-specific plugin via a discussion or issue first.
- **Documentation** -- Typo fixes, clearer installation instructions, better examples.

## Getting Started

1. Fork and clone the repo
2. Look at an existing plugin such as `plugins/backend-validator/` to understand the structure

## Plugin Quality Standards

Every plugin in this marketplace follows strict quality guidelines:

- **Skills** are lightweight routing and opinion layers; generic framework knowledge does not belong in a plugin
- **References** load only when repository evidence makes them relevant
- **Additional skills** are thin explicit workflows, not duplicate primary skill bodies
- **Interfaces beat prose** -- prefer executable verification, schemas, and live tool help over copied catalogues
- **Judgment beats blanket rules** -- reserve hard constraints for safety or demonstrated failure modes
- **All paths** use `${CLAUDE_PLUGIN_ROOT}` for portability
- **Facts are verified** -- no fake CVEs, no unsourced statistics, no hallucinated library versions
- **Examples earn their space** -- keep only examples that expose a non-obvious interface or failure mode
- **AI slop is banned** -- no filler, repeated doctrine, ornamental taxonomies, or speculative compatibility

## Validation

Before submitting, validate your plugin:

```bash
# Validate marketplace and plugin manifests
python3 scripts/check_plugin_manifests.py

# Check context budgets, reference routing, versions, and stale command links
python3 scripts/lint-plugins.py

# Load the plugin in a session and test its skills
claude --plugin-dir ./plugins/<your-plugin>

# Run the plugin validator agent (if you have the plugin-dev plugin installed)
# /validate-plugin
```

If you iterate on a plugin locally:

- In **Claude Code**, run `/reload-plugins` after installing, enabling, disabling, or updating plugins in the current session.
- In **GitHub Copilot CLI**, reinstall the plugin after local edits because installed plugins are cached.

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
