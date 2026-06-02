---
description: Deep architecture critique of Python backend code. Evaluates AI slop, layer boundaries, anti-patterns, and design quality.
argument-hint: "[area]"
allowed-tools: [Read, Grep, Glob, Bash, Agent]
user-invocable: true
---

Conduct a comprehensive architecture critique of the Python backend code. If $ARGUMENTS is provided, focus on that module, file, or feature area. Otherwise critique the entire backend.

**First**: Use the python-backend-expert skill for all design principles, patterns, and anti-patterns.

## Critique Process

1. **Classify the project shape** (from `SKILL.md`): web backend (FastAPI/Starlette) vs runtime process (worker, CLI, ETL, daemon). Apply the right reference set.

2. **Confirm the stack**: FastAPI (routers, `Depends`, `Annotated`), Pydantic v2, SQLAlchemy 2.0 async. Note the Python version and whether `async def` vs `def` routes are used correctly.

3. **Map the architecture**:
   - Read the composition root (`main.py`, `app.py`) to understand DI wiring
   - Identify the layer structure: entrypoints / application / domain / infrastructure
   - Trace the dependency graph: what imports what?

4. **AI Slop scan (start here)**: Run `references/ai-slop.md`. Check every CODE-01 through CODE-10 and ARCH-01 through ARCH-10 pattern. Use the detection checklist at the bottom.

5. **Correctness pass**: For a backend, walk `async-patterns.md`, `sqlalchemy.md`, `fastapi.md`, `lifecycle.md`. For a runtime, walk `runtimes.md` (subprocess, filesystem, signals, IPC).

6. **Boundaries pass**: `validation.md` (parse-don't-validate, request/response separation, pagination, versioning), `dependency-injection.md`, the **Error design** section of `modern-python.md`.

7. **Design pass**: `architecture.md`, `project-structure.md`.

8. **File size check**: Flag any module >400 lines as a candidate for splitting. Flag >600 as urgent.

## Output Format

### AI Slop Verdict

**Start here.** Pass/fail: does this code look AI-generated? Run the detection checklist. Be brutally honest. List specific tells with file and line. If 3+ tells are found, the code needs architectural review.

### Architecture Map

Brief overview of detected architecture (layers, DI approach, framework).

### What's Working Well

Highlight 2-4 patterns done correctly. Reinforce good architecture.

### Priority Issues

Top 5-8 issues, ordered by severity (blocking > important > architecture > nit):

For each:
- **What**: Name the problem with pattern ID (CODE-xx or ARCH-xx from `ai-slop.md`)
- **Where**: File path and line
- **Why it matters**: Concrete consequence (production bug, perf cliff, data corruption, etc.)
- **Fix**: Specific code change or restructuring needed
- **Verify**: How to confirm the fix worked (test that should now pass, load behavior that should now hold)

### File Size Report

List any files >400 lines with line count and suggested split strategy.

### Summary Scorecard

| Dimension | Grade | Notes |
|---|---|---|
| AI Slop | A-F | |
| Layer Boundaries | A-F | |
| Async Correctness | A-F | |
| ORM Usage (N+1, sessions) | A-F | |
| Error Handling | A-F | |
| Type Safety | A-F | |
| Project Structure | A-F | |

**IMPORTANT**: Be direct. Vague feedback wastes time. Say what's wrong, where, WHY it matters, and how to fix it. Prioritize ruthlessly: if everything is important, nothing is.

**NEVER**:
- Soften criticism. Developers need honest feedback to ship great architecture.
- Skip the AI slop check. It's the most impactful quality signal.
- Report issues without explaining concrete impact.
- Forget to praise what works (reinforce good patterns).
