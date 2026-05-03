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
- stronger push on minimal APIs, Native AOT direction, and EF Core 8 improvements

### .NET 9

- STS release focused on cloud-native performance and operational polish
- C# 13
- stronger built-in OpenAPI story, better tracing/observability direction, and more AOT attention
- useful as a transition release, but less likely to be the long-term target in 2026

### .NET 10

- current LTS default for new backend work
- C# 14
- stronger runtime and JIT improvements
- stricter and more efficient `System.Text.Json` options, including `PipeReader` support
- `WebSocketStream` and other low-level networking/library improvements for backend scenarios
- EF Core 10 improvements such as named query filters

## May 2026 Recommendation

- **New backend service**: default to `net10.0`
- **Established `net8.0` service**: upgrade deliberately, not blindly, but treat `.NET 10` as the likely destination
- **`net9.0` service**: assume it is transitional unless there is a specific reason to stay there

## C# 14 Features Worth Knowing

Adopt when they improve clarity or correctness, not because they are new.

- **`extension` blocks** — declare extension _properties_, static extensions, and grouped extension members. Useful for fluent registration helpers and DTO conveniences. Replaces a lot of static helper class noise.
- **`field` keyword** — reference the compiler-generated backing field inside a property accessor without declaring it. Cuts boilerplate when a property needs validation in `set`.
- **Null-conditional assignment** — `customer?.Order = GetCurrentOrder();` evaluates the right side only when the left side is non-null. Removes a class of `if (x is not null)` noise.
- **Partial constructors and partial events** — relevant when working with source generators (validation, OpenAPI XML doc, custom infrastructure).
- **First-class span conversions** — implicit conversions among `Span<T>`, `ReadOnlySpan<T>`, and `T[]`. Removes friction in hot paths without `.AsSpan()` ceremony.
- **`nameof` on unbound generics** — `nameof(List<>)` returns `"List"`. Small but useful in logging and diagnostics.
- **Lambda parameter modifiers without types** — `(text, out result) => ...` no longer requires explicit types when using `ref`/`in`/`out`/`scoped`.

## .NET 10 ASP.NET Core Features Worth Knowing

These are the features most likely to change a code review verdict:

- **Built-in Minimal API validation** — `builder.Services.AddValidation()` enables a source-generator-driven validation filter that processes `[DataAnnotations]` and `IValidatableObject` on parameters and request bodies. Often replaces a hand-rolled FluentValidation pipeline.
- **`Microsoft.Extensions.Validation`** — the validation APIs moved out of ASP.NET Core into a general-purpose package. Application-layer validation can now live without an HTTP dependency.
- **`IApiEndpointMetadata`** — cookie auth on endpoints carrying this metadata returns 401/403 instead of redirecting to a login URL. Auto-applied to `[ApiController]`, JSON-reading/writing minimal APIs, `TypedResults` returns, and SignalR. Long-requested fix.
- **OpenAPI 3.1 by default** — nullable types now use `oneOf` with `null`, schemas use full JSON Schema 2020-12, and `Microsoft.OpenApi` 2.0 introduces breaking changes for transformers (`OpenApiAny` → `JsonNode`, `Nullable` → `JsonSchemaType.Null`).
- **OpenAPI in YAML** — `app.MapOpenApi("/openapi/{documentName}.yaml")`.
- **XML doc → OpenAPI source generator** — enable `<GenerateDocumentationFile>true</GenerateDocumentationFile>` and XML comments on methods/classes flow into the OpenAPI document automatically.
- **`TypedResults.ServerSentEvents`** — first-class SSE support for one-way streams. Often the right answer for use cases that previously defaulted to SignalR.
- **`Microsoft.AspNetCore.JsonPatch.SystemTextJson`** — new JSON Patch implementation built on `System.Text.Json`. Replaces the legacy `Newtonsoft.Json`-based package; significantly faster and lower allocation. Not a drop-in replacement for `ExpandoObject` scenarios.
- **JSON + `PipeReader` deserialization** — MVC, Minimal APIs, and `ReadFromJsonAsync` now parse from a `PipeReader` by default. Custom `JsonConverter`s must handle `Utf8JsonReader.HasValueSequence` or set the `Microsoft.AspNetCore.UseStreamBasedJsonParsing` AppContext switch as a temporary workaround.
- **Authentication, authorization, and Identity metrics** — new built-in counters under `Microsoft.AspNetCore.Authorization` and `Microsoft.AspNetCore.Identity` meters. Drop-in observability for auth flows.
- **Automatic memory pool eviction** — Kestrel, IIS, and HTTP.sys memory pools release blocks under low load. Reduces idle memory footprint without configuration.
- **`.localhost` TLD support in Kestrel** — `*.localhost` names bind to loopback automatically. The dev cert covers `*.dev.localhost`. Useful for separating cookies/sessions across local apps.
- **`Microsoft.Testing.Platform` (MTP)** — supported in `dotnet test`. Faster, modern test runner.
- **EF Core 10 named query filters** — multiple filters per entity type with selective disabling. Removes the historical "only one global filter" workaround.

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
