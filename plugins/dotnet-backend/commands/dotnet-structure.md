---
description: Analyze and recommend .NET backend solution and folder structure improvements. Checks projects, layers, oversized files, and boundary clarity.
argument-hint: "[directory-or-solution]"
allowed-tools: [Read, Grep, Glob, Bash, Agent]
user-invocable: true
---

Analyze the `.NET` backend structure. If `$ARGUMENTS` is provided, analyze that solution, directory, or project. Otherwise auto-detect the main backend solution.

**First**: Use the `dotnet-backend-expert` skill. Read `references/project-structure.md` and `references/architecture.md`.

## Analysis Steps

1. **Map the solution**:
   - list `*.sln` and `*.csproj`
   - classify projects as API/host, AppHost, contracts, application, domain, infrastructure, workers, tests
   - detect whether the codebase is layered, vertical slice, modular monolith, or tangled
2. **File size audit**:
   - flag `.cs` files over 300 lines as split candidates
   - flag `.cs` files over 500 lines as urgent
   - suggest the split by responsibility, not by arbitrary regions
3. **Project boundary audit**:
   - domain should not depend on infrastructure or host projects
   - contracts should stay transport-safe
   - AppHost should orchestrate, not contain business rules
4. **Folder organization audit**:
   - check for one major type per file
   - check for feature grouping versus giant technical buckets
   - check whether endpoints, services, repositories, entities, and contracts are easy to locate
5. **Missing pieces**:
   - missing contracts project or folder
   - missing application/service boundary
   - missing test projects
   - missing composition root extensions

## Output

### Current Structure

Show a tree with project purpose and notable line counts.

### Structure Score: X/10

Explain the score briefly.

### Issues Found

Order by impact. For each issue include the exact project/file and the recommended move, split, or create action.

### Recommended Target Layout

If a change is needed, show the ideal target shape for this backend.

### Migration Steps

Give a safe incremental path that keeps the app working at each step.
