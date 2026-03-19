---
description: Scan and harden .NET backend code against high-impact anti-patterns such as sync-over-async, lifetime bugs, fat endpoints, and fragile SignalR state.
argument-hint: "[target]"
allowed-tools: [Read, Grep, Glob, Bash, Agent]
user-invocable: true
---

Harden the `.NET` backend. If `$ARGUMENTS` is provided, focus on that project, directory, or file. Otherwise target the main backend projects.

**First**: Use the `dotnet-backend-expert` skill and read `references/anti-patterns.md`, `references/dependency-injection.md`, `references/concurrency.md`, `references/data-access.md`, `references/signalr.md`, and `references/kestrel-hosting.md`.

## Preparation

1. Detect the solution, SDK, and backend entry projects.
2. Determine whether the code uses minimal APIs, controllers, SignalR hubs, workers, AppHost, EF Core, Dapper, or mixed data access.
3. Read the composition root before making any changes.

## High-Value Scans

Run targeted scans for patterns that frequently produce production bugs:

```bash
# Sync-over-async and blocking (DN-12)
rg -n '(\.Result\b|\.Wait\(|GetAwaiter\(\)\.GetResult\()' . --glob '*.cs'

# Hidden runtime resolution / service locator (DN-04)
rg -n '(IServiceProvider|GetService\(|GetRequiredService\(|CreateScope\()' . --glob '*.cs'

# Static mutable state (DN-06)
rg -n 'static\s+(?!readonly)' . --glob '*.cs'

# DbContext misuse in endpoints, hubs, or singletons (DN-07, DN-01, DN-02)
rg -n '(DbContext|ApplicationDbContext)' . --glob '*.cs'

# Task.Run inside request/endpoint code (often DN-12 or DN-13)
rg -n 'Task\.Run\(' . --glob '*.cs'

# Fire-and-forget tasks (DN-13)
rg -n '(_\s*=\s*Task\.|Task\.Factory\.StartNew|async void)' . --glob '*.cs'

# New HttpClient instead of factory/typed client
rg -n 'new\s+HttpClient\(' . --glob '*.cs'

# Generic base abstractions and ceremony-heavy layers (DN-18, DN-10)
rg -n '(BaseService|BaseRepository|IGenericRepository|GenericRepository)' . --glob '*.cs'

# In-memory SignalR truth and fragile connection bookkeeping (DN-15)
rg -n '(ConnectionId|ConcurrentDictionary<.*connection|Dictionary<.*connection|Groups\.)' . --glob '*.cs'

# Kestrel / forwarded header posture and explicit hosting choices
rg -n '(UseForwardedHeaders|ForwardedHeaders|KnownProxies|KnownNetworks|Kestrel|ListenAnyIP|ListenLocalhost|MaximumReceiveMessageSize|KeepAliveInterval|ClientTimeoutInterval)' . --glob '*.cs'
```

## Hardening Priorities

1. Fix correctness risks first:
   - scoped service captured by singleton (`DN-05`)
   - blocking async paths (`DN-12`)
   - swallowed exceptions (`DN-20`)
   - background work without lifetime management (`DN-13`, `DN-14`)
2. Fix boundary leaks second:
   - endpoint or hub contains business logic (`DN-01`, `DN-02`)
   - entities leak directly to HTTP or SignalR contracts (`DN-08`)
   - AppHost or infrastructure concerns leak into domain logic (`DN-16`)
3. Fix operability risks third:
   - fragile SignalR connection state kept in-process (`DN-15`)
   - unbounded concurrency
   - long transactions
   - hidden ambient configuration (`DN-17`)
4. Fix ceremony only when it improves clarity:
   - collapse meaningless wrappers and fake seams (`DN-10`, `DN-18`)
   - remove interfaces with one implementation when no boundary benefit exists
   - split giant files and god services

## Output

### Hardened Issues

For each fixable issue, report:
- file and line
- anti-pattern ID
- why it matters
- exact change made or recommended

### Remaining Risks

List issues that need broader refactoring or product decisions.

### Verification

After changes, run the existing build/tests for the affected project. Report what ran and whether it passed.

**NEVER**:
- “Fix” issues by adding broad catch blocks
- Introduce abstractions without a concrete boundary reason
- Turn a modular monolith into accidental distributed architecture
- Store cross-node SignalR truth in static in-memory dictionaries
