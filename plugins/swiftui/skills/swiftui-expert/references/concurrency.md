# Swift Concurrency

## Core Rules

- Always prefer `async`/`await` over completion handlers and closures.
- Never use GCD (`DispatchQueue.main.async`, `DispatchQueue.global()`, etc.).
- Assume strict concurrency checking is enabled. Flag `@Sendable` violations and data races.

## @MainActor

- In **new Swift 6.2+ projects**, the recommended setup is **default MainActor isolation enabled** (SE-466). Most code is then implicitly main-isolated — no `@MainActor` annotations needed on `@Observable` classes, view models, or app types. Reach for `@concurrent` when you actually want background work.
- In **older projects** (default isolation disabled), all `@Observable` classes used by SwiftUI views should be `@MainActor`.
- Use `@MainActor` over `DispatchQueue.main.async` — cleaner, compiler-verified.
- Before adding `MainActor.run { }` to existing code, check the project's default isolation setting; it may already be on the main actor.

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

## Swift 6.2 Changes ("Approachable Concurrency")

Swift 6.2 (Sep 2025) reframes concurrency around three opt-ins. For new SwiftUI apps, enable **all three** in the package manifest / build settings:

- **Default actor isolation = MainActor** (SE-466) — most code lands on the main actor automatically. No more `@MainActor` annotation noise on `@Observable` classes.
- **`nonisolated(nonsending)` by default** (SE-461) — `nonisolated async` functions now run on the caller's actor instead of jumping to the global executor. Eliminates a whole class of "why did my UI hop off main?" bugs.
- **`@concurrent` attribute** — explicit opt-in for code that _should_ run on the cooperative pool. Use this for image decoding, JSON parsing on large payloads, etc.

```swift
// In a default-MainActor project:
@Observable
final class ImageStore {           // Implicitly @MainActor
    var cached: [URL: Image] = [:]

    func load(_ url: URL) async throws -> Image {
        if let img = cached[url] { return img }
        let data = try await fetch(url)         // Stays on main; cheap network
        let img = try await decode(data)        // Hops to background pool
        cached[url] = img
        return img
    }

    @concurrent
    nonisolated func decode(_ data: Data) async throws -> Image { ... }
}
```

## Swift 6.3 Changes (Mar 2026)

- Improved `async` debugging in LLDB (named tasks, task-context backtraces).
- Embedded Swift parity continues to expand; not directly SwiftUI-relevant.
- Watch for new Swift Evolution proposals tightening `Sendable` inference for closures.

## Typed NotificationCenter (Swift 6.2+)

Replace stringly-typed notification observers with concrete struct types. Eliminates `userInfo` casting and concurrency errors.

```swift
struct CartUpdated: NotificationCenter.MainActorMessage {
    static let name = Notification.Name("CartUpdated")
    let itemCount: Int
}

// Post
NotificationCenter.default.post(CartUpdated(itemCount: 3))

// Observe (in a view's .task or in an @Observable class)
for await message in NotificationCenter.default.messages(of: CartUpdated.self) {
    cartBadge = message.itemCount
}
```

- Use `MainActorMessage` for UI-related notifications (delivered on main).
- Use `AsyncMessage` when delivery actor doesn't matter.
- Prefer this over the legacy `addObserver` + `userInfo` pattern in all new code.

## Common Anti-Patterns

| Anti-Pattern                                | Fix                                     |
| ------------------------------------------- | --------------------------------------- |
| `DispatchQueue.main.async { }`              | `@MainActor` or `MainActor.run { }`     |
| `Task.sleep(nanoseconds:)`                  | `Task.sleep(for:)`                      |
| `Task.detached { }` everywhere              | Regular `Task { }` (inherits context)   |
| Silently swallowed errors                   | Show alert or log meaningfully          |
| Mutable shared state without actor          | Use `actor` or `@MainActor`             |
| `MainActor.run()` when already on MainActor | Check project's default isolation first |
