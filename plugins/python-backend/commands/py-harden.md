---
description: Scan Python backend code for AI-slop and architectural anti-patterns from ai-slop.md (CODE-01 through CODE-10, ARCH-01 through ARCH-10) and fix every instance found.
argument-hint: "[file-or-directory]"
allowed-tools: [Read, Edit, Grep, Glob, Bash, Agent]
user-invocable: true
---

Systematically scan the Python code for AI-slop and architectural anti-patterns, then fix every instance found. If $ARGUMENTS is provided, scan that file or directory. Otherwise scan the entire `src/` directory.

**First**: Use the python-backend-expert skill. Read `references/ai-slop.md` for the full catalog (20 patterns: 10 code-level + 10 architectural). For backend correctness, also load `async-patterns.md`, `sqlalchemy.md`, `fastapi.md`. For non-backend code (workers, CLIs, ETL), load `runtimes.md` instead.

## Process

1. **Scan phase** (read-only):
   - Read every `.py` file in scope.
   - Check each file against CODE-01 through CODE-10 (code patterns) and ARCH-01 through ARCH-10 (architectural patterns).
   - Record findings with file path, line number, pattern ID, and severity.

2. **Report phase**:
   - Present findings grouped by severity (blocking first).
   - For each finding, show the problematic code and the proposed fix (the GOOD example from `ai-slop.md`).
   - Ask which findings to fix (or "all").

3. **Fix phase**:
   - Apply fixes in order: blocking → important → architecture → nit.
   - After each fix, verify the file still parses (`python -m py_compile <file>`).
   - Run any available linters/formatters (`ruff check`, `ruff format`).

## Severity Order

Fix in this order:

1. **blocking** (guaranteed production bug): CODE-04 (Pydantic v1 ghosts that crash on v2), CODE-05 (exception swallowing destroys context), CODE-06 (async singleton race → FD leak), CODE-07 (`shell=True` with user input → RCE), CODE-08 (naive datetimes), ARCH-08 (N+1 lazy loading in async → MissingGreenlet), plus any blocking call inside `async def` from `async-patterns.md`.

2. **important** (wrong on a real path): CODE-01 (no validation at the boundary), CODE-02 (eager string log formatting), CODE-03 (`dict[str, Any]` tunneling), CODE-09 (`# type: ignore` without reason), CODE-10 (boolean flags), ARCH-01 (routes querying the DB), ARCH-02 (ORM as response schema), ARCH-03 (services taking `Request`), ARCH-05 (`BackgroundTasks` for durable work), ARCH-06 (mocked-DB unit tests pretending to be integration tests), ARCH-07 (inline auth check in every route).

3. **architecture** (structural misfit): ARCH-04 (settings as module global), ARCH-09 (Pydantic in the domain), ARCH-10 (flat src/ with no bounded contexts).

After fixing, re-run the scan to confirm zero findings.
