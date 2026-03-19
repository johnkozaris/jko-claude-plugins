# Modern .NET 10 Guidance

## Detect the Real Version First

Before suggesting language or framework features, inspect:

- `global.json`
- `TargetFramework` / `TargetFrameworks`
- `LangVersion`
- shared props and package versions

Do not recommend preview-only or unsupported features into a stable project.

## Default Stance

For new backend services, `.NET 10` is a strong default because it is LTS and keeps backend tooling current. But “modern” should mean **clearer and safer**, not merely newer.

## .NET 8 -> 10 Backend Timeline

### .NET 8

- LTS release and still a common production baseline
- C# 12
- major push on minimal APIs, Native AOT direction, and EF Core 8 improvements
- Aspire entered the picture, but as an emerging orchestration stack rather than a universal default

### .NET 9

- STS release focused on cloud-native performance and operational polish
- C# 13
- stronger built-in OpenAPI story, better tracing/observability direction, and more AOT attention
- useful as a transition release, but less likely to be the long-term target in March 2026

### .NET 10

- current LTS default for new backend work
- C# 14
- stronger runtime and JIT improvements
- stricter and more efficient `System.Text.Json` options, including `PipeReader` support
- `WebSocketStream` and other low-level networking/library improvements for backend scenarios
- EF Core 10 improvements such as named query filters

## March 2026 Recommendation

- **New backend service**: default to `net10.0`
- **Established `net8.0` service**: upgrade deliberately, not blindly, but treat `.NET 10` as the likely destination
- **`net9.0` service**: assume it is transitional unless there is a specific reason to stay there

## Use Modern Features Deliberately

Adopt newer features when they improve one of these:

- clarity
- correctness
- startup or throughput characteristics
- source generation / compile-time validation
- operational simplicity

Examples worth adopting when they fit the codebase:

- route groups and typed results
- source-generated JSON and compile-time-friendly serializers
- compile-time options validation
- stricter JSON settings when duplicate-property acceptance or loose payload parsing is a risk
- `TimeProvider` for time-sensitive logic
- modern C# syntax such as file-scoped namespaces, required members, and selective primary constructors
- newer runtime/library features such as `WebSocketStream` only when they solve a concrete problem

## Performance Pragmatism

`.NET 10` may improve performance, but the review rule is unchanged:

- measure first
- optimize hot paths intentionally
- keep code readable
- audit AOT and trimming before enabling them broadly

Native AOT is not a free checkbox. Reflection-heavy or dynamic code needs proof before opting in.

## Kestrel & Hosting

Treat Kestrel hardening as part of backend design:

- explicit listener and proxy posture
- explicit request/body/time limits when internet-facing
- explicit protocol choices when using gRPC, WebSockets, or HTTP/3

Use the dedicated `kestrel-hosting.md` reference when the review question is about exposure, proxy trust, listener configuration, or raw WebSocket hosting instead of ordinary API design.

## Modern-Code Smells

- new syntax added only because it is new
- preview APIs suggested without verifying the project version
- `ValueTask` or spans introduced with no measured reason
- Native AOT enabled without auditing DI, reflection, JSON, or dynamic loading
- “.NET 10 is faster” used as an argument without measurements
