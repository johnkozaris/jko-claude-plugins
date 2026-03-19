---
description: One-time setup that scans a .NET backend project, learns its conventions and architecture, and writes them to CLAUDE.md for future sessions.
argument-hint: "[project-root]"
allowed-tools:
  - Read
  - Edit
  - Write
  - Grep
  - Glob
  - Bash
  - AskUserQuestion
user-invocable: true
---

# .NET Teach

Scan this `.NET` backend project, understand its real conventions, and persist the findings so future sessions start with the right context.

## Step 1: Explore the Codebase

Before asking questions, thoroughly scan the project:

- `global.json`, `Directory.Build.props`, `Directory.Packages.props` — SDK, language version, common props, package management
- `*.sln`, `*.csproj` — project structure, target frameworks, references
- `Program.cs` and service-registration extensions — host style, DI patterns, middleware/endpoints
- AppHost project — whether `.NET Aspire` is used and how strictly it is separated
- Data access — EF Core, Dapper, raw SQL, migrations, transaction patterns
- Realtime — SignalR hubs, group management, connection state strategy
- Background work — `BackgroundService`, channels, queue abstractions
- Logging/observability — ILogger, Serilog, OpenTelemetry, correlation patterns
- Tests — unit/integration/API test projects and tools
- Existing `CLAUDE.md` — preserve or extend existing project guidance

Note what is clearly inferable and what remains uncertain.

## Step 2: Ask Clarifying Questions

Ask only what the codebase cannot answer.

### Project Context
- Is this backend a modular monolith, product suite, or service in a larger ecosystem?
- Is AppHost only for local orchestration, or part of the team's normal distributed-dev workflow?
- Any hard latency, throughput, or reliability requirements?

### Conventions
- Is the preferred endpoint style minimal APIs, controllers, or mixed?
- Are repositories required everywhere, or only at selected boundaries?
- Are vertical slices preferred over strict layer buckets?

### Quality Rules
- Any strict rules for file size, interface usage, or architecture layering?
- Any data or messaging rules that all contributors must follow?
- Any specific DI lifetime or SignalR policies?

Skip questions where the answer is already evident from the code.

## Step 3: Write .NET Backend Context

Synthesize findings into a `## .NET Backend Conventions` section:

```markdown
## .NET Backend Conventions

### Host Style
[Minimal APIs/controllers/SignalR/workers/AppHost usage]

### Architecture
[Modular monolith, layered, vertical slice, project boundaries]

### Data Access
[EF Core, repositories, transaction rules, migration approach]

### Dependency Injection
[Built-in container patterns, lifetime rules, factory usage]

### Realtime & Background Work
[SignalR usage, connection-state rules, workers/channels]

### Testing & Quality Gates
[Test projects, required commands, style or analyzer expectations]

### Domain Rules
[Project-specific rules that should steer all future work]
```

Write or update this section in the project's `CLAUDE.md`. Do not overwrite unrelated content.

## Step 4: Confirm

Summarize the key conventions that will now guide future `.NET` backend work. Tell the user they can rerun `/dotnet-teach` when the architecture changes.
