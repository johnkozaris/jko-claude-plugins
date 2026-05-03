# Dead Code Expert Plugin

Language-agnostic dead code detection, duplicate elimination, and codebase simplification across any programming language.

## What It Does

A language-agnostic cleanup skill that identifies certainly dead code first, then escalates to higher-confidence duplicates, speculative abstractions, and zombie test/debug artifacts. It emphasizes proof, false-positive awareness, and safe removal instead of blind deletion.

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install dead-code@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/dead-code
```

## Commands

| Command | Purpose |
|---|---|
| `/dead-code-scan` | Read-only scan for dead code findings grouped by confidence and category |
| `/dead-code-clean` | Remove dead code, duplicates, and zombie artifacts using configurable confidence modes |

## Skill

The `dead-code-expert` skill activates automatically when finding or removing dead code. It provides:

- Unused imports, variables, functions, classes, and types
- Unreachable code and dead branches
- Commented-out code and debug artifacts
- Duplicate / dual implementations and speculative generality
- Dead test code (skipped tests, orphaned test files)
- Lint suppressions hiding dead code
- False-positive awareness (reflection, serialization, framework magic, public API)

## Supported Languages

Python, JavaScript/TypeScript, Rust, Go, Swift, C#, Java, C/C++ — with per-language tool integration (knip, vulture, clippy, periphery, deadcode, Roslyn, etc.).

## Hook

No active runtime hooks. `hooks/hooks.json` is reserved for future hook-based checks.

## References

8 reference files organized by domain:

ai-slop-patterns, detection-catalog, duplicate-code, false-positives, grep-patterns, language-tools, prevention, safe-removal

## License

MIT
