---
description: Deep architecture critique of Python backend code. Evaluates AI slop, SOLID compliance, layer boundaries, anti-patterns, and design quality.
argument-hint: "[area]"
allowed-tools: [Read, Grep, Glob, Bash, Agent]
user-invocable: true
---

Conduct a comprehensive architecture critique of the Python backend code. If $ARGUMENTS is provided, focus on that module, file, or feature area. Otherwise critique the entire backend.

**First**: Use the python-backend-expert skill for all design principles, patterns, and anti-patterns.

## Critique Process

1. **Detect framework**: Litestar (Controllers, Provide) or FastAPI (routers, Depends). Adapt terminology.

2. **Map the architecture**:
   - Read the composition root (app.py, main.py, or container.py) to understand DI wiring
   - Identify the layer structure: entrypoints / application / domain / infrastructure
   - Trace the dependency graph: what imports what?

3. **AI Slop Detection (start here)**: Run the detection checklist from `references/ai-slop.md`. Check for AS-01 through AS-12.

4. **Review each layer** using the python-backend-expert skill's review checklist (all 13 points).

5. **Check file sizes**: Flag any module >300 lines as a candidate for splitting. Flag >500 as urgent.

6. **Anti-pattern scan**: Check for AP-01 through AP-22 from `references/anti-patterns.md`.

## Output Format

### AI Slop Verdict

**Start here.** Pass/fail: Does this code look AI-generated? Run the detection checklist. Be brutally honest. List specific tells with file and line. If 3+ tells are found, the code needs architectural review.

### Architecture Map

Brief overview of detected architecture (layers, DI approach, framework).

### What's Working Well

Highlight 2-4 patterns done correctly. Reinforce good architecture.

### Priority Issues

Top 5-8 issues, ordered by severity (blocking > important > nit):

For each:
- **What**: Name the problem with anti-pattern ID (AP-xx or AS-xx)
- **Where**: File path and line
- **Why it matters**: Concrete consequence
- **Fix**: Specific code change or restructuring needed
- **Command**: Which command to use (`/py-harden`, `/py-structure`)

### File Size Report

List any files >300 lines with line count and suggested split strategy.

### Summary Scorecard

| Dimension | Grade | Notes |
|---|---|---|
| AI Slop | A-F | |
| Layer Boundaries | A-F | |
| SOLID Compliance | A-F | |
| ORM Usage | A-F | |
| Error Handling | A-F | |
| Async Correctness | A-F | |
| Type Safety | A-F | |
| Project Structure | A-F | |

**IMPORTANT**: Be direct. Vague feedback wastes time. Say what's wrong, where, WHY it matters, and how to fix it. Prioritize ruthlessly -- if everything is important, nothing is.

**NEVER**:
- Soften criticism -- developers need honest feedback to ship great architecture
- Skip the AI slop check -- it's the most impactful quality signal in 2025+
- Report issues without explaining concrete impact
- Forget to praise what works (reinforce good patterns)
