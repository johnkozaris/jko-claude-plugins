# Swift Concurrency

Target: Swift 6.3 / iOS 26 / Xcode 26. The mental model below is **Approachable Concurrency by default** — most app code is implicitly `@MainActor`, `nonisolated async` stays on the caller's executor, and `@concurrent` is how you opt *into* background work.

---

## Approachable Concurrency — the modern default

New Xcode 26 + Swift 6.2 / 6.3 projects opt into **Approachable Concurrency** by default. App targets get three switches that change every prior concurrency mental model:

- **Default Actor Isolation = MainActor** (SE-0466) — app code is implicitly `@MainActor` unless you opt out. No more `@MainActor` annotations on every `@Observable` class, view model, app type.
- **`nonisolated(nonsending)` by default** (SE-0461) — `nonisolated async` functions run on the caller's executor. A `@MainActor` view that calls a nonisolated async helper stays on the main actor. Kills the "subtle data race only on device" bug class.
- **`@concurrent` for explicit background opt-in** — mark a function `@concurrent` to force it onto the cooperative thread pool. This is now the modern alternative to "wrap CPU work in `Task.detached`".

**For app targets: leave Approachable Concurrency ON.** For non-UI SPM packages (Networking, Persistence, ImageCache): opt out per-target — they should not infect callers with main-thread hops.

### Per-package opt-out

```swift
// Package.swift
.target(
    name: "Networking",
    swiftSettings: [
        .defaultIsolation(nil)  // non-UI package: stays nonisolated
    ]
),
.target(
    name: "Persistence",
    swiftSettings: [
        .defaultIsolation(nil)
    ]
),
.target(
    name: "Features",
    swiftSettings: [
        .defaultIsolation(MainActor.self)  // UI package: stays MainActor
    ]
)
```

### Manual migration is mandatory

Do **not** flip the Approachable Concurrency flag alone on an existing project. Without running the migration tooling, `nonisolated async` silently changes meaning — functions that used to hop to the global executor now stay on the caller's actor, which can introduce subtle thread-confinement bugs the compiler can no longer catch.

Run the migration tool, audit each `nonisolated async` for the new semantics, mark CPU-heavy work `@concurrent` where it really needs to hop.

---

## File 06 rule 31 is out of consensus

State this explicitly: **"default to nonisolated" is wrong for app targets in 2026.**

The 2026 corpus aligns on **MainActor-default for app targets**:

- Community Swift-teaching blogs covering "What is Approachable Concurrency in Xcode 26."
- The `useyourloaf.com` writeup "Approachable Concurrency in Swift Packages."
- Other established Swift-teaching blogs covering default actor isolation in Swift 6.2.
- Apple WWDC25 session 268 ("Embrace Swift concurrency").

File 06 rule 31 (which says "default to nonisolated") contradicts the consensus and was verified wrong during validation. Treat it as legacy advice. App code is UI-adjacent and Apple-recommended MainActor-default is the right starting point.

---

## `.task` for view-tied async work

The `.task { }` modifier is **structured** — SwiftUI cancels the task automatically on view disappearance and on `id` change.

```swift
struct ItemListView: View {
    let category: Category
    @State private var items: [Item] = []
    @State private var searchQuery = ""

    var body: some View {
        List(items) { item in
            ItemRow(item: item)
        }
        .task {
            // Runs when view appears, cancels on disappear
            await loadItems()
        }
        .task(id: searchQuery) {
            // Cancels and restarts when searchQuery changes
            await search(searchQuery)
        }
        .task(id: category) {
            // Cancels and restarts when category changes
            await loadCategory(category)
        }
    }
}
```

### Rules

- **NEVER `.onAppear { Task { } }` for new code.** It's unstructured — doesn't cancel on view disappearance, leaks the in-flight work if the user navigates away. Use `.task { }` instead.
- **NEVER unstructured `Task { ... }` inside view bodies** unless you genuinely need a top-level task that outlives the view (rare; usually a smell).
- **`.task(id:)` is the canonical search/filter pattern** — old value cancels, new value starts. No manual debounce ceremony required for cheap work.
- **URLSession respects cancellation and throws `CancellationError`.** Catch silently — don't surface "your action failed" because the user scrolled away.

### Self-cancellation trap

Writing to `@State` inside a `.task` or `.refreshable` closure triggers a redraw, which can cancel the very task that wrote the state. Two fixes:

```swift
// Bad — partial writes cancel the task
.task {
    while let chunk = try? await stream.next() {
        items.append(chunk)  // re-render cancels this task
    }
}

// Good — batch updates at the end
.task {
    var collected: [Item] = []
    while let chunk = try? await stream.next() {
        collected.append(chunk)
    }
    items = collected
}

// Or — detach from the SwiftUI-managed handle
.task {
    await Task { await runStreamingWork() }.value
}
```

---

## `@concurrent` for background work

`@concurrent` forces a function onto the cooperative thread pool regardless of caller. Use for genuinely heavy work — image decoding, JSON parsing of large payloads, expensive transforms — when invoked from `@MainActor` code.

```swift
@MainActor
@Observable
final class ImageStore {
    private var cached: [URL: UIImage] = [:]

    func load(_ url: URL) async throws -> UIImage {
        if let img = cached[url] { return img }
        let data = try await fetch(url)         // stays on main; cheap network
        let img = try await decode(data)        // hops to background pool
        cached[url] = img                       // back on main, no manual hop
        return img
    }

    @concurrent
    nonisolated func decode(_ data: Data) async throws -> UIImage {
        // Heavy work runs on the cooperative pool
        guard let img = UIImage(data: data) else { throw DecodeError.invalid }
        return img
    }
}
```

### Rules

- **Reach for `@concurrent` only when you genuinely need a background hop.** Cheap network calls and tiny transforms don't need it under Approachable Concurrency.
- **Mark heavy off-main work `@concurrent` at the function definition.** Callers don't need to remember to wrap in `Task.detached`.
- **`@concurrent nonisolated` is the modern pattern.** Implies the function is nonisolated and runs on the cooperative pool.
- **Don't `@concurrent` everything in a service.** A `NetworkService` whose only state is an `URLSession` (thread-safe) is a `Sendable struct` or `final class` — no actor, no `@concurrent` decoration needed.

---

## Task auto-cancellation (cooperative)

Swift task cancellation is **cooperative** — your code must check `Task.isCancelled` or call `try Task.checkCancellation()` for cancellation to take effect.

- `.task(id:)` cancels + restarts on id change automatically.
- `URLSession` respects cancellation and throws `CancellationError`.
- Long-running CPU loops won't notice cancellation unless you ask.

```swift
@concurrent
nonisolated func processItems(_ items: [Item]) async throws -> [Result] {
    var results: [Result] = []
    for item in items {
        try Task.checkCancellation()  // bail out fast if cancelled
        let result = try await heavyWork(item)
        results.append(result)
    }
    return results
}
```

### Rules

- **Always check `Task.isCancelled` (or `try Task.checkCancellation()`) at suspension-free hot spots** — tight loops, CPU work that never awaits.
- **Catch `CancellationError` silently.** Don't surface it as a user-facing error.
- **Cancellation propagates through `async let`, `withTaskGroup`, and structured children** — but only into points where the child awaits or checks.

---

## Structured concurrency

### `async let` — parallel awaits with named results

Default to `async let` for fixed, small fan-out (2–4 calls). Compile-time-known children, static structure, automatic cancellation propagation.

```swift
func loadUserProfile(id: User.ID) async throws -> Profile {
    async let user = api.fetchUser(id: id)
    async let posts = api.fetchPosts(by: id)
    async let followers = api.fetchFollowers(of: id)

    return try await Profile(
        user: user,
        posts: posts,
        followers: followers
    )
}
```

### `withTaskGroup` / `withThrowingTaskGroup` — dynamic fan-out

Use when fan-out is runtime-determined (one task per item in a fetched list) and you need the results back.

```swift
func fetchItems(ids: [Item.ID]) async throws -> [Item] {
    try await withThrowingTaskGroup(of: Item.self) { group in
        for id in ids {
            group.addTask { try await api.fetchItem(id: id) }
        }
        var collected: [Item] = []
        for try await item in group {
            collected.append(item)
        }
        return collected
    }
}
```

#### Fill-then-drain — limiting concurrency in TaskGroup

Without backpressure, naive `for ... addTask` can spawn thousands of concurrent network requests.

```swift
func fetchAll(ids: [Item.ID], maxConcurrent: Int = 4) async throws -> [Item] {
    try await withThrowingTaskGroup(of: Item.self) { group in
        var iterator = ids.makeIterator()

        // Prime the pool
        for _ in 0..<maxConcurrent {
            guard let id = iterator.next() else { break }
            group.addTask { try await api.fetchItem(id: id) }
        }

        var results: [Item] = []
        while let item = try await group.next() {
            results.append(item)
            // Keep the pool full
            if let id = iterator.next() {
                group.addTask { try await api.fetchItem(id: id) }
            }
        }
        return results
    }
}
```

### `withDiscardingTaskGroup` — fire-and-forget

For long-running fire-and-forget work where return values don't matter (server accept loops, file watchers, per-event tasks that never end). Completed children are released immediately — no memory growth.

```swift
func runServer() async throws {
    try await withThrowingDiscardingTaskGroup { group in
        for await connection in listener.connections {
            group.addTask {
                try await handleConnection(connection)
            }
        }
    }
}
```

- **Use `withDiscardingTaskGroup` instead of standard `withTaskGroup` for unbounded fire-and-forget loops.** Standard task groups accumulate child results in memory; discarding bounds it.
- **`withThrowingDiscardingTaskGroup`** propagates any child throw to cancel the whole group.

### Don't reach for unstructured `Task { }` from a structured context

You lose parent cancellation, priority, and task-local values. Almost always a mistake when `async let` or a task group would do.

---

## Sendable

`Sendable` types are safe to share across actor boundaries. The compiler enforces correctness.

### Rules

- **Value types (structs, enums) with all `Sendable` properties are implicitly `Sendable`.**
- **Classes need `final` + only `let` (immutable) properties + `Sendable` conformance** to be safely shared.
- **Closures crossing actor boundaries are `@Sendable`** — but in Swift 6.2, the compiler infers this for most cases. Stop adding `@Sendable` to closures unless the compiler asks.
- **`sending` keyword (Swift 6)** transfers ownership of a value into a different isolation domain.
- **`@unchecked Sendable` warrants a comment naming the synchronization mechanism.** Most uses silence the compiler without achieving safety. Real fixes that don't need `@unchecked`:
  - Make the type `final` + only `let` `Sendable` properties.
  - Wrap mutable state in `Mutex` from the `Synchronization` framework (Swift 6+).
  - Split into actor (mutation) + value type (snapshot).

```swift
// Sendable struct — all properties are Sendable
struct UserDTO: Sendable {
    let id: UUID
    let name: String
    let createdAt: Date
}

// Sendable final class — all properties are `let`
final class Configuration: Sendable {
    let apiBaseURL: URL
    let timeoutSeconds: Int

    init(apiBaseURL: URL, timeoutSeconds: Int) {
        self.apiBaseURL = apiBaseURL
        self.timeoutSeconds = timeoutSeconds
    }
}

// Locked mutable state — use Mutex, not @unchecked Sendable
import Synchronization

final class Counter: Sendable {
    private let state = Mutex(0)

    func increment() {
        state.withLock { $0 += 1 }
    }

    func value() -> Int {
        state.withLock { $0 }
    }
}
```

`@unchecked Sendable` with a manual `NSLock` and `Any` storage is the antipattern that everyone reaches for first. Don't. Use one of the three real fixes above.

---

## Actor design — when warranted

Most services do **not** need to be actors. Reserve actors for genuine mutable-state-protection cases.

### The four-question test for actor design

Add an actor only when ALL of:

1. **Non-Sendable mutable state?** Plain `Sendable` types don't need an actor.
2. **Multiple call sites?** A struct used in one place doesn't need actor isolation.
3. **Atomicity required?** Operations on the state must be atomic across `await` points.
4. **Can't already live on `@MainActor` or another actor?** If it can, fold it in.

If all four → actor. Otherwise → `final class Sendable`, struct, or extend an existing actor.

### Canonical actor: an in-memory cache

```swift
actor ImageCache {
    private var cache: [URL: UIImage] = [:]
    private let maxEntries = 100

    func image(for url: URL) -> UIImage? {
        cache[url]
    }

    func store(_ image: UIImage, for url: URL) {
        if cache.count >= maxEntries {
            cache.removeFirst()  // simple eviction
        }
        cache[url] = image
    }

    func clear() {
        cache.removeAll()
    }
}
```

### Anti-actor: a service with no mutable state

```swift
// Bad — actor with no actual mutable state to protect
actor NetworkService {
    private let session = URLSession.shared  // already thread-safe
    func fetch(_ url: URL) async throws -> Data {
        try await session.data(from: url).0
    }
}

// Good — Sendable struct (no actor ceremony, no hop)
struct NetworkService: Sendable {
    func fetch(_ url: URL) async throws -> Data {
        try await URLSession.shared.data(from: url).0
    }
}
```

### Global actors

`@MainActor` is the global actor for UI. Custom global actors (`@DatabaseActor`, `@NetworkActor`) are rarely needed — prefer regular actors injected through the environment / dependency container.

---

## Reentrancy

Actors are **reentrant** — `await` inside an actor method may allow other calls to the actor to interleave. Always check state after `await`.

```swift
actor Counter {
    private var value = 0

    func increment() async {
        let current = value
        // ⚠️ During the await below, another increment() can run
        let newValue = await computeNext(current)
        // State may have changed during await!
        value = newValue  // races with concurrent increments
    }
}
```

### Fix patterns

- **Snapshot + retry:** after `await`, re-check the state to ensure it's still what you expected.
- **Avoid `await` inside critical sections.** Compute deltas synchronously before any suspension point.
- **Lock-style atomic helpers.** Build atomic methods (`addOne()`, `commit()`) that don't await between read and write.

```swift
actor Counter {
    private var value = 0

    func addOne() {
        value += 1  // synchronous; truly atomic
    }

    func incrementAfter(_ work: () async -> Int) async {
        let delta = await work()
        value += delta  // atomic write after await; no read-modify-write race
    }
}
```

---

## Typed throws (SE-0413)

Default to **untyped `throws`** — keep API flexible as error sets evolve. Reserve typed throws (`throws(SpecificError)`) for narrow stable boundaries where exhaustive matching pays.

### Rules

- **Default to untyped `throws` for app code.** SE-0413 explicitly recommends it. Error types in apps evolve; locking them down hurts API evolution.
- **Use `throws(SpecificError)` at module boundaries you exhaustively handle** — validators, parsers, a `Result`-returning facade where you want compiler-checked exhaustive `switch`.
- **Don't use typed throws on public framework APIs.** Adding a case is source-breaking for clients.
- **`throws(any Error)` == plain `throws`.** `throws(Never)` == non-throwing. Lets `rethrows` be expressed as `throws(E)` parameterized on the closure's error type.
- **Leading-dot syntax inside typed-throws bodies.** Swift infers the error type, so `throw .invalidInput` works without spelling the enum.

```swift
// Typed throws — narrow stable boundary
enum NetworkError: Error {
    case timeout
    case invalidResponse
    case unauthorized
    case offline
}

func fetch(url: URL) async throws(NetworkError) -> Data {
    // Compiler enforces that any throw inside is one of NetworkError's cases
    guard isOnline else { throw .offline }
    // ...
}

// Caller benefits from exhaustive switch
do {
    let data = try await fetch(url: url)
} catch {
    switch error {
    case .timeout: showRetry()
    case .invalidResponse: showError()
    case .unauthorized: signOut()
    case .offline: showOfflineBanner()
    }
}

// Untyped throws — app code default
func loadUser() async throws -> User {
    // Can throw anything; callers handle with do/catch
    let data = try await api.fetch()
    return try JSONDecoder().decode(User.self, from: data)
}
```

### Error envelopes

For production code, wrap underlying errors in a struct with `logMessage`, `userMessage`, and `underlyingErrors` so engineers get raw errors and users get friendly text.

```swift
struct AppError: Error {
    let logMessage: String
    let userMessage: String
    let underlyingErrors: [Error]
}
```

### `try?` rule

Use `try?` only for genuinely optional/best-effort paths. Never to swallow real failures — it loses the error entirely. Use `do/catch` with at minimum a log call if you're going to ignore.

---

## `Task.sleep` modern form

```swift
// Good
try await Task.sleep(for: .seconds(1))
try await Task.sleep(for: .milliseconds(500))
try await Task.sleep(for: .nanoseconds(100_000))
try await Task.sleep(until: .now + .seconds(2), clock: .continuous)

// Bad — legacy, deprecated form
try await Task.sleep(nanoseconds: 1_000_000_000)
```

The `Duration`-based API arrived in iOS 16 / Swift 5.7. Always use it.

---

## Bridging legacy callbacks

For Apple APIs and third-party SDKs that still use completion handlers, wrap with `withCheckedContinuation` / `withCheckedThrowingContinuation`.

```swift
func fetchUser() async throws -> User {
    try await withCheckedThrowingContinuation { continuation in
        legacyFetchUser { result in
            continuation.resume(with: result)
        }
    }
}

func fetchSetting() async -> Bool {
    await withCheckedContinuation { continuation in
        legacyAPI.fetchSetting { value in
            continuation.resume(returning: value)
        }
    }
}
```

### Rules

- **Use checked continuations** (safer, detect misuse) over unsafe ones (`withUnsafeContinuation` / `withUnsafeThrowingContinuation`). Only reach for unsafe variants when you've profiled and need the perf.
- **A continuation must be resumed exactly once.** Resuming twice is undefined behavior; resuming zero times leaks the task.
- **For delegate-based callbacks**, prefer `AsyncStream` / `AsyncThrowingStream` over continuation-per-callback.

### `AsyncStream` for repeated callbacks

```swift
func locationUpdates() -> AsyncStream<CLLocation> {
    AsyncStream { continuation in
        let delegate = LocationDelegate { location in
            continuation.yield(location)
        }
        locationManager.delegate = delegate
        locationManager.startUpdatingLocation()

        continuation.onTermination = { _ in
            locationManager.stopUpdatingLocation()
        }
    }
}

// Usage
for await location in locationUpdates() {
    map.center = location
}
```

---

## Typed NotificationCenter (Swift 6.2+)

Replace stringly-typed notification observers with concrete struct types. Eliminates `userInfo` casting and concurrency errors.

```swift
struct CartUpdated: NotificationCenter.MainActorMessage {
    static let name = Notification.Name("CartUpdated")
    let itemCount: Int
}

// Post (anywhere)
NotificationCenter.default.post(CartUpdated(itemCount: 3))

// Observe in a view's .task
.task {
    for await message in NotificationCenter.default.messages(of: CartUpdated.self) {
        cartBadge = message.itemCount
    }
}

// Observe in an @Observable model
@MainActor
@Observable
final class CartStore {
    var badge = 0

    func observe() async {
        for await message in NotificationCenter.default.messages(of: CartUpdated.self) {
            badge = message.itemCount
        }
    }
}
```

### Rules

- **Use `MainActorMessage` for UI-related notifications** (delivered on main).
- **Use `AsyncMessage` when the delivery actor doesn't matter.**
- **Prefer typed messages over `addObserver` + `userInfo`** in all new code.
- **Existing `NotificationCenter.publisher(for:)`** Combine bridge still works for legacy Apple notifications, but typed messages are the new direction.

---

## Migration to Swift 6 (incremental)

Migrating an existing project to strict concurrency is **module by module**, never repo-wide in one PR. The widely-cited community migration playbooks all converge on the same approach.

### Order

1. **Smallest leaf package first.** Often `DesignSystem` — no app deps, no service deps.
2. **Working up the dependency graph.** Networking, then Persistence, then per-feature, then the app target.
3. **One package at a time.** Each gets `.defaultIsolation(MainActor.self)` if UI-adjacent, or stays nonisolated for non-UI.
4. **Audit `nonisolated async` semantics** at every flip — they now stay on the caller's executor under Approachable Concurrency.

### Per-target opt-in

```swift
.target(
    name: "DesignSystem",
    swiftSettings: [
        .defaultIsolation(MainActor.self),
        .enableUpcomingFeature("NonisolatedNonsendingByDefault"),
        .enableUpcomingFeature("InferIsolatedConformances")
    ]
)
```

### Rules

- **Never enable strict concurrency repo-wide in one PR.** Hundreds of warnings make the project un-reviewable.
- **For new projects: enable Approachable Concurrency from day 1.** Xcode 26 default.
- **For existing apps: enable the smallest leaf package first.** Validate. Then move up.

---

## SwiftUI lifecycle integration

Concurrency inside SwiftUI views is dominated by `.task`. The cross-references:

- **`.task { }` / `.task(id:)`** — auto-cancelling view-tied async work. See "Task auto-cancellation" above and `lifecycle.md`.
- **`Observations { ... }`** (iOS 26+) — AsyncSequence over `@Observable` state changes. Bridge `@Observable` to non-SwiftUI consumers, replace Combine debounce/throttle. Capture `weak self` inside the closure. See `state-and-observation.md`.
- **SwiftData `@ModelActor`** — background ModelContext that's safe to use across isolation boundaries. Pass `PersistentIdentifier` across actors, never `PersistentModel` instances. See `persistence.md`.

---

## Common anti-patterns

| Anti-pattern                                       | Fix                                                                                |
|----------------------------------------------------|------------------------------------------------------------------------------------|
| `DispatchQueue.main.async { ... }`                 | `@MainActor` / `MainActor.run { ... }` / Approachable Concurrency default          |
| `DispatchQueue.global().async { ... }`             | `@concurrent` function or `Task { ... await heavyFn() }`                           |
| `Task.sleep(nanoseconds:)`                         | `Task.sleep(for:)`                                                                 |
| `Task.detached { }` for heavy work                 | Regular `Task { }` (inherits context) or `@concurrent nonisolated func`            |
| `@unchecked Sendable` casual usage                 | `final class Sendable` with only `let` properties OR actor OR `Mutex`-backed state |
| Silently swallowed errors                          | Surface to user; log meaningfully                                                  |
| `.onAppear { Task { ... } }`                       | `.task { ... }` (auto-cancels)                                                     |
| `Task { ... }` inside view body                    | `.task { }` modifier (structured, auto-cancels)                                    |
| Pre-Swift-6 `withCheckedContinuation` everywhere   | Use typed `MainActorMessage` / `AsyncMessage` patterns                             |
| `MainActor.run { }` when already on MainActor      | Check project's default isolation first                                            |
| Actor with no mutable state                        | `Sendable struct` or `final class Sendable`                                        |
| `@MainActor` on every `@Observable` class          | Redundant under Approachable Concurrency default                                   |
| `Combine.@Published` + `@Observable` on same class | Don't mix — pick one (prefer `@Observable`)                                        |
| `nonisolated async` for "background" work          | `@concurrent nonisolated async` — semantics changed in SE-0461                     |
| Repo-wide strict-concurrency flip in one PR        | Module-by-module, leaf-first migration                                             |
| Long CPU loop with no cancellation check           | `try Task.checkCancellation()` inside the loop                                     |

---

## Cross-references

- View lifecycle `.task` rules and `.onAppear` vs `.task` decision → `lifecycle.md`.
- State ownership (`@State` / `@Bindable` / `@Environment`), `Observations { }` AsyncSequence → `state-and-observation.md`.
- SwiftData `@ModelActor`, `ModelContext` thread safety, background save trap → `persistence.md`.
- Swift-language idioms (typed throws, generics, regex, result builders) → `swift-idioms.md`.
- Deprecated API replacement table (`.task` over `.onAppear`, `Task.sleep(for:)`, etc.) → `modern-api.md`.
