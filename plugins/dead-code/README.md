# Dead Code Expert Plugin

Language-agnostic dead code detection, duplicate elimination, and codebase simplification across any programming language.

## Components

### Skill: `dead-code-expert`
Auto-activates when finding or removing dead code. Covers:
- Unused imports, variables, functions, classes, and types
- Unreachable code and dead branches
- Commented-out code and debug artifacts
- Duplicate / dual implementations and speculative generality
- Dead test code (skipped tests, orphaned test files)
- Lint suppressions hiding dead code
- False-positive awareness (reflection, serialization, framework magic, public API)

### Command: `/dead-code-scan`
Read-only scan that reports dead code grouped by confidence level.

```
/dead-code-scan                  # Scan entire project
/dead-code-scan src/services/    # Scan a specific directory
```

### Command: `/dead-code-clean`
Actively finds and removes dead code with configurable confidence modes.

```
/dead-code-clean                          # Default: high confidence
/dead-code-clean src/ certain             # Safest: only compiler-confirmed
/dead-code-clean src/ aggressive          # Includes medium confidence
```

## Supported Languages

Python, JavaScript/TypeScript, Rust, Go, Swift, C#, Java, C/C++ — with per-language tool integration (knip, vulture, clippy, periphery, deadcode, Roslyn, etc.).

## Installation

Copy or symlink to your Claude Code plugins directory, or use within the myClaudeSkills marketplace.

## License

MIT
