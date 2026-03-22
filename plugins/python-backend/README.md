# Python Backend Expert Plugin

Expert Python backend architecture with Litestar and FastAPI, SQLAlchemy, hexagonal patterns, and SOLID principles.

## What It Does

A senior Python architect skill that reviews backend code for architecture, SOLID compliance, async correctness, ORM usage, and anti-pattern detection. Dual-framework support for Litestar (OOP controllers) and FastAPI (functional routers).

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install python-backend@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/python-backend
```

## Commands

| Command | Purpose |
|---|---|
| `/py-critique` | Architecture review with scorecard across all dimensions |
| `/py-harden` | Scan for anti-patterns (AP-01 through AP-22) and fix |
| `/py-structure` | Project layout analysis and restructuring guidance |

## Skill

The `python-backend-expert` skill activates automatically when working with Python backend code. It provides:

- Dual-framework detection (Litestar vs FastAPI)
- Hexagonal architecture enforcement
- 13 reference files covering architecture through testing
- 22 anti-patterns catalog with BAD/GOOD code examples
- AI slop detection for backend code

## Hook

No active runtime hooks. `hooks/hooks.json` is reserved for future hook-based checks.

## References

13 reference files organized by domain:

architecture, solid-principles, repository-patterns, dependency-injection, sqlalchemy, async-patterns, api-design, error-handling, modern-python, anti-patterns, testing, project-structure, ai-slop

## License

MIT
