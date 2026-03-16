---
description: Analyze and recommend project structure improvements. Checks file sizes, layer organization, module splitting, and hexagonal architecture compliance.
argument-hint: "[directory]"
allowed-tools: [Read, Grep, Glob, Bash, Agent]
user-invocable: true
---

Analyze the Python backend project structure and recommend improvements. If $ARGUMENTS is provided, analyze that directory. Otherwise auto-detect `src/` or `backend/`.

**First**: Use the python-backend-expert skill. Read `references/project-structure.md` and `references/architecture.md`.

## Analysis Steps

1. **Map the project**:
   - List all Python files with line counts
   - Identify the architectural pattern (hexagonal, layered, flat, feature-based)
   - Detect framework (Litestar/FastAPI)

2. **File size audit**:
   - Flag files >300 lines (candidate for split)
   - Flag files >500 lines (must split)
   - For each oversized file, suggest specific split strategy

3. **Layer boundary check**:
   - Verify domain has no infrastructure imports
   - Verify entrypoints don't import from infrastructure directly (except through DI)
   - Check for circular import risks

4. **Module organization check**:
   - One major class per file?
   - Logical grouping by domain?
   - Proper `__init__.py` re-exports?
   - Import order following convention?

5. **Missing components**:
   - Missing domain entities (using ORM models directly)?
   - Missing ports/protocols?
   - Missing service layer?
   - Missing exception hierarchy?

## Output

### Current Structure
Tree view with line counts and layer annotations.

### Structure Score: X/10

### Issues Found
Ordered by impact, with specific file moves/splits/creates recommended.

### Recommended Structure
If the current structure needs major changes, show the ideal target layout.

### Migration Steps
If restructuring is needed, provide step-by-step plan that keeps the app working at each step.
