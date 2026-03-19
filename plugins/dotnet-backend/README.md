# .NET Backend Expert Plugin

Pure `.NET 10` backend architecture for Kestrel-hosted services: REST endpoints, SignalR hubs, TypeScript/React and pragmatic Rust client interoperability, DI lifetimes, AppHost awareness, auth/CORS/rate-limiting/health-check posture, data access, and SOLID/OOP critique.

## What It Does

A senior `.NET` backend architect skill that reviews backend code for architecture, boundary discipline, OOP/SOLID quality, dependency injection lifetime correctness, endpoint design, SignalR hub design and connection management, TypeScript/React integration shape, pragmatic Rust interop, data access tradeoffs, concurrency safety, and distributed-system decision making.

This plugin is intentionally **backend-only**. It is for Kestrel-hosted services and related backend projects, not MAUI, Blazor, Razor UI, or desktop/mobile app guidance.

## Terminology Rule

This plugin speaks in **`.NET 10 backend`** terms.

- Treat Kestrel, REST endpoints, SignalR hubs, workers, DI, and data access as **backend** concerns.
- Do **not** describe the target stack using the web-stack brand name inside the plugin guidance.
- If an official Microsoft document title uses that brand, cite the title accurately in source notes, but keep the plugin's own framing as `.NET backend` / `Kestrel backend`.

## Scope

In scope:

- Kestrel-hosted `.NET 10` backend services
- REST endpoints, controllers, route groups, and SignalR hubs
- application/domain/infrastructure boundaries
- AppHost/Aspire awareness for backend orchestration
- EF Core, repositories, contracts, DI, background work, and distributed-system tradeoffs
- modern `.NET 8`, `.NET 9`, and `.NET 10` backend guidance with `.NET 10` as the default recommendation for new work

Out of scope:

- MAUI, Blazor, Razor UI, WinUI, WPF, and other UI concerns
- generic frontend architecture advice
- broad CI/CD or DevOps guidance that is not directly tied to backend architecture

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install dotnet-backend@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/dotnet-backend
```

## Commands

| Command | Purpose |
|---|---|
| `/dotnet-critique` | Full architecture and code review with AI-slop, SOLID, DI, data, and distributed-systems scorecard |
| `/dotnet-harden` | Scan and harden backend anti-patterns like sync-over-async, DI lifetime bugs, fat endpoints, and SignalR misuse |
| `/dotnet-structure` | Analyze solution, project, and folder layout with split guidance for oversized files and muddled layers |
| `/dotnet-teach` | Scan a real `.NET` backend project and write its conventions into `CLAUDE.md` for future sessions |

## Skill

The `dotnet-backend-expert` skill activates automatically when working with `.NET` backend code. It provides:

- Backend-only guidance for Kestrel-hosted services
- Current backend guidance across `.NET 8`, `.NET 9`, and `.NET 10`
- Pragmatic clean architecture and modular monolith defaults
- REST endpoint, SignalR, DI, EF Core, and AppHost/Aspire review rules
- backend security and operational posture for auth, CORS, rate limiting, health checks, and graceful shutdown
- SignalR hub guidance for TypeScript/React clients and pragmatic Rust interoperability
- OOP/SOLID critique tuned for modern C# and production services
- AI-slop and over-engineering detection for ceremony-heavy `.NET` code
- Interview-style architecture reasoning for tradeoffs and design reviews

## Hook

Hooks are reserved for future backend-specific post-edit checks.

## References

17 reference files organized by domain:

architecture, solid-principles, dependency-injection, project-structure, endpoints-rest, signalr, kestrel-hosting, security-and-operations, data-access, concurrency, distributed-architecture, apphost-aspire, error-handling, testing, modern-dotnet, anti-patterns, ai-slop

## License

MIT
