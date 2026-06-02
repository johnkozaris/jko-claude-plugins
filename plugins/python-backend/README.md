# Python Backend Expert Plugin

Expert FastAPI backend critique: async correctness, SQLAlchemy 2.0 async, Pydantic v2, project structure, and the AI-generated patterns you want to catch.

## What It Does

A senior Python reviewer for FastAPI backends. Catches the #1 production bug class (blocking the event loop inside `async def`), the #1 ORM async bug class (MissingGreenlet / N+1), Pydantic v2 mistakes, ORM-models-as-API-schemas, lifecycle gaps (startup ordering, graceful shutdown, idempotency), and the 20 most common AI-generated code and architecture mistakes. Updates the project to current versions and deprecations.

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
| `/py-harden` | Scan for anti-patterns and fix |
| `/py-structure` | Project layout analysis and restructuring guidance |

## Skill

The `python-backend-expert` skill activates automatically when working with Python backend code. It covers:

- FastAPI runtime correctness (Depends/Annotated, lifespan, response_model)
- Async correctness (event-loop blocking, route coloring, ExceptionGroup)
- SQLAlchemy 2.0 async (MissingGreenlet, N+1, pool sizing)
- Backend lifecycle (startup, graceful shutdown, request flow, idempotency, observability)
- Runtimes that aren't web (workers, ETL, CLI tools, daemons): signals, atomic writes, multiprocessing, file locking, scheduling, containers
- Validation (parse-don't-validate, smart constructors, strict vs lax)
- Class and data-structure DOs and DON'Ts (dataclass vs attrs vs Pydantic, error design, naming)
- Two project layouts (file-type for ≤5 domains, per-domain otherwise)
- Performance (Uvicorn tuning, orjson, SQLAlchemy pool, profilers)
- Current library versions and deprecations (uv, ruff, PEP 735, ORJSONResponse, etc.)
- Top 20 AI-generated anti-patterns (10 code + 10 architecture) with BAD/GOOD examples

## References

14 reference files organized by domain:

architecture, dependency-injection, sqlalchemy, async-patterns, fastapi, lifecycle, validation (covers request/response, pagination, versioning, validation layers), modern-python (covers pure-Python fundamentals plus error design), testing, project-structure, performance, 2026-currency, ai-slop, runtimes

## License

MIT
