---
description: Scan Python backend code for anti-patterns and fix them. Runs the full anti-pattern catalog (AP-01 through AP-22) against the codebase.
argument-hint: "[file-or-directory]"
allowed-tools: [Read, Edit, Grep, Glob, Bash, Agent]
user-invocable: true
---

Systematically scan the Python backend for anti-patterns and fix every instance found. If $ARGUMENTS is provided, scan that file or directory. Otherwise scan the entire `src/` directory.

**First**: Use the python-backend-expert skill. Read `references/anti-patterns.md` for the full catalog.

## Process

1. **Scan phase** (read-only):
   - Read every `.py` file in scope
   - Check each file against all anti-patterns AP-01 through AP-22
   - Record findings with file, line, pattern ID, and severity

2. **Report phase**:
   - Present all findings grouped by severity (blocking first)
   - For each finding, show the problematic code and the proposed fix
   - Ask user which findings to fix (or "all")

3. **Fix phase**:
   - Apply fixes in order: blocking -> important -> nit
   - After each fix, verify the file still parses (no syntax errors)
   - Run any available linters/formatters

## Priority Order

Fix in this order:
1. **blocking**: AP-01 (fat controller), AP-05 (N+1), AP-07 (session leak), AP-08 (expire_on_commit), AP-09 (blocking in async), AP-11 (bare except), AP-12 (mutable default), AP-14 (global state), AP-19 (god class)
2. **important**: AP-02 (god module), AP-03 (import spaghetti), AP-06 (naked SQLAlchemy), AP-10 (fire-forget), AP-13 (stringly config), AP-15 (star imports), AP-17 (dict return), AP-22 (exception flow control)
3. **nit**: AP-04 (anemic service), AP-16 (missing types), AP-18 (print debug), AP-20 (premature abstraction), AP-21 (boolean params)
