# Swift Concurrency

## Core Rules

- Always prefer `async`/`await` over completion handlers and closures.
- Never use GCD (`DispatchQueue.main.async`, `DispatchQueue.global()`, etc.).
- Assume strict concurrency checking is enabled. Flag `@Sendable` violations and data races.

## @MainActor

- All `@Observable` classes must be `@MainActor` unless the project uses default MainActor isolation (Swift 6.2 `SE-466`).
- Use `@MainActor` over `DispatchQueue.main.async` — cleaner, compiler-verified.
- In new projects with default MainActor isolation, you don't need explicit `@MainActor` annotations on most code.

```swift
@MainActor
@Observable
final class UserModel {
    var name = ""
    var isLoading = false

    func loadUser() async {
        isLoading = true
        let user = await api.fetchUser()
        name = user.name
        isLoading = false
    }
}
```

## Tasks in SwiftUI

### .task Modifier (Preferred)

```swift
var body: some View {
    List(items) { ... }
        .task {
            await loadItems()  // Auto-cancelled when view disappears
        }
        .task(id: searchQuery) {
            await search(searchQuery)  // Re-runs when id changes
        }
}
```

- Prefer `.task` over `onAppear` for async work — automatic cancellation.
- `.task(id:)` re-triggers when the id value changes.

### Unstructured Tasks

```swift
// Inherits actor context and priority
Task {
    await doWork()
}

// Detached — loses actor context. Rarely needed.
Task.detached {
    await heavyComputation()
}
```

- Prefer `Task` over `Task.detached` — inherits context.
- `Task.detached` is often a bad idea. Check usage carefully.

### Task Sleep

```swift
// Bad
try await Task.sleep(nanoseconds: 1_000_000_000)

// Good
try await Task.sleep(for: .seconds(1))
```

## Actors

Use actors for shared mutable state:

```swift
actor ImageCache {
    private var cache: [URL: UIImage] = [:]

    func image(for url: URL) -> UIImage? {
        cache[url]
    }

    func store(_ image: UIImage, for url: URL) {
        cache[url] = image
    }
}
```

### Global Actors

`@MainActor` is a global actor — ensures main thread execution. Custom global actors are rarely needed.

### Reentrancy

Actors are reentrant — `await` calls inside an actor may allow other calls to interleave. Always check state after `await`:

```swift
actor Counter {
    var value = 0

    func increment() async {
        let current = value
        let newValue = await computeNext(current)
        // State may have changed during await!
        value = newValue
    }
}
```

## Sendable

- Value types (structs, enums) with all Sendable properties are implicitly Sendable.
- Classes must be `final` + immutable, or use `@unchecked Sendable` with manual safety.
- Closures crossing actor boundaries must be `@Sendable`.
- `sending` keyword (Swift 6) for parameters that transfer ownership.

```swift
// Sendable struct — all properties are Sendable
struct UserDTO: Sendable {
    let id: UUID
    let name: String
}

// @unchecked for types with internal synchronization
final class ThreadSafeCache: @unchecked Sendable {
    private let lock = NSLock()
    private var storage: [String: Any] = [:]
}
```

## Bridging Legacy Code

Use continuations to bridge callback-based APIs:

```swift
func fetchUser() async throws -> User {
    try await withCheckedThrowingContinuation { continuation in
        legacyFetchUser { result in
            continuation.resume(with: result)
        }
    }
}
```

- Use checked continuations (safer, detect misuse) over unsafe ones.
- A continuation must be resumed exactly once.

## Swift 6.2 Changes

- `@concurrent` attribute for explicitly opting into concurrent execution.
- `nonisolated(nonsending)` (SE-461) — runs on caller's actor by default.
- Default actor isolation (SE-466) — can set project-wide default to `@MainActor`.
- When default MainActor isolation is on, most `@MainActor` annotations become unnecessary.

## Common Anti-Patterns

| Anti-Pattern | Fix |
|---|---|
| `DispatchQueue.main.async { }` | `@MainActor` or `MainActor.run { }` |
| `Task.sleep(nanoseconds:)` | `Task.sleep(for:)` |
| `Task.detached { }` everywhere | Regular `Task { }` (inherits context) |
| Silently swallowed errors | Show alert or log meaningfully |
| Mutable shared state without actor | Use `actor` or `@MainActor` |
| `MainActor.run()` when already on MainActor | Check project's default isolation first |
