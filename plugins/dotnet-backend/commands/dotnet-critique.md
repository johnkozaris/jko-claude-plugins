---
description: Deep architecture critique of pure .NET backend code. Evaluates AI slop, OOP/SOLID, layer boundaries, DI lifetimes, endpoints, SignalR, data access, concurrency, and distributed-system choices.
argument-hint: "[area]"
allowed-tools: [Read, Grep, Glob, Bash, Agent]
user-invocable: true
---

Conduct a comprehensive critique of the `.NET` backend. If `$ARGUMENTS` is provided, focus on that project, directory, file, or feature area. Otherwise critique the backend as a whole.

**First**: Use the `dotnet-backend-expert` skill for all architecture principles, review order, and anti-pattern detection.

## Gather Context First

1. Detect the solution and SDK context:
   - `*.sln`, `global.json`, `Directory.Build.props`, `Directory.Packages.props`
   - target frameworks from `*.csproj`
   - whether the app is a Kestrel API, worker, SignalR host, AppHost, or a mix
2. Identify the composition root:
   - `Program.cs`
   - service-registration extensions
   - AppHost project if present
3. Map the architecture:
   - endpoints/controllers/route groups
   - application/services/use cases
   - domain/entities/value objects
   - infrastructure/data/external clients
   - contracts/schemas/DTOs

## Critique Process

1. **AI slop detection (start here)**: Run the checklist from `references/ai-slop.md` before anything else.
2. **Architecture map**: Determine whether the project is a modular monolith, layered system, vertical slice app, or accidental blob.
3. **Boundary review**:
    - endpoints and hubs stay thin (`DN-01`, `DN-02`)
    - services orchestrate and enforce workflow
    - flag vague `Coordinator` / `Manager` / `Engine` / `Orchestrator` types (`DN-21`)
    - domain owns invariants
    - infrastructure stays behind boundaries
4. **Kestrel and hosting review**:
   - listener and reverse-proxy posture explicit
   - forwarded headers and trust boundaries deliberate
   - request limits and protocol choices match exposure
5. **DI and lifetimes**:
   - check singleton/scoped/transient choices
   - flag scoped-in-singleton bugs (`DN-05`)
   - flag service locator and hidden runtime resolution (`DN-04`, `DN-17`)
6. **Endpoint and contract review**:
   - request/response models separated from entities (`DN-08`)
   - validation at the boundary
   - consistent error shapes and status codes
7. **SignalR review**:
    - hubs thin, no business logic (`DN-02`)
    - no fragile in-memory connection state if the app needs scale-out (`DN-15`)
    - review whether the peer is actually a SignalR client or a generic socket need
 8. **Security and operational review**:
    - auth/authz boundaries deliberate
    - browser-facing backends have explicit CORS posture
    - public surfaces have a rate-limiting/abuse story
    - health checks and graceful shutdown present
 9. **Data access review**:
    - `DbContext` lifetime and transaction boundaries (`DN-07`, `DN-11`)
    - EF Core query shape, tracking, and repository tradeoffs (`DN-10`)
 10. **Concurrency review**:
    - sync-over-async, blocking calls, missing cancellation, shared mutable state (`DN-06`, `DN-12`, `DN-13`, `DN-14`)
 11. **Distributed systems review**:
    - challenge microservice or messaging complexity unless justified (`DN-19`)
 12. **File and project size review**:
    - flag files >300 lines as split candidates
    - flag files >500 lines as urgent
    - flag projects that mix unrelated layers
 13. **Anti-pattern scan**: Check for `DN-01` through `DN-21` from `references/anti-patterns.md`.

## Output Format

### AI Slop Verdict

Start here. Pass/fail the code on AI-slop smell. List the specific tells with file and line.

### Architecture Map

Summarize the detected shape of the backend: host style, hosting/proxy posture, layers, projects, DI approach, data strategy, SignalR usage, and whether AppHost is present.

### What's Working Well

Highlight 2-4 patterns done correctly. Reinforce good architecture.

### Priority Issues

List the top 5-8 issues in severity order (`blocking`, `important`, `nit`, `suggestion`, `praise`). For each issue include:
- **What**: Name the problem with anti-pattern ID when applicable
- **Where**: File path and line
- **Why it matters**: Concrete production consequence
- **Fix**: Specific code or structure change
- **Command**: Which follow-up command to use (`/dotnet-harden`, `/dotnet-structure`, `/dotnet-teach`)

### File and Project Size Report

List files over the size guidance and any projects that combine too many responsibilities.

### Summary Scorecard

| Dimension | Grade | Notes |
|---|---|---|
| AI Slop | A–F | |
| Boundaries | A–F | |
| OOP / SOLID | A–F | |
| Hosting | A–F | |
| DI Lifetimes | A–F | |
| Endpoints | A–F | |
| SignalR | A–F | |
| Data Access | A–F | |
| Concurrency | A–F | |
| Structure | A–F | |

**IMPORTANT**: Be direct. Do not pad the review with trivia. Say what is wrong, where it is wrong, why it matters, and how to fix it.

**NEVER**:
- Critique UI-only concerns for this backend plugin
- Recommend distributed architecture without naming the real driver
- Suggest abstractions you cannot justify with a concrete maintenance or correctness benefit
- Ignore the AI-slop pass
