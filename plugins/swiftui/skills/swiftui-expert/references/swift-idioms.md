# Swift idioms

Target: Swift 6.3 / iOS 26 / Xcode 26.

This file covers language-level patterns that aren't SwiftUI-specific but compound across a SwiftUI codebase. They're easy to overlook because each rule is small, but they accumulate — a codebase that gets these right reads twice as cleanly as one that doesn't, and several of them prevent the kinds of bugs that take hours to track down.

Cross-references:
- `@Observable`, `@Bindable`, and environment plumbing are in `state-and-observation.md`.
- `@MainActor`, `Sendable`, and structured concurrency are in `concurrency.md`.
- The modern SwiftUI API replacement table is in `modern-api.md`.
- View composition rules are in `view-composition.md`.

## Optionals

### Shorthand unwrap

Since Swift 5.7 you can drop the redundant `= value` in `if let` and `guard let` when the unwrapped name matches the optional:

```swift
// Modern
if let user { greet(user) }
guard let user else { return }

// Older
if let user = user { greet(user) }
guard let user = user else { return }
```

There's no semantic difference — it's purely visual. The longer form was always slightly silly; the language now agrees.

### `guard let` for preconditions, `if let` for branches

`guard let` keeps the happy path unindented and forces an early exit. `if let` is for code that legitimately needs to handle both branches.

```swift
// guard for preconditions
func send(_ message: String) {
    guard let recipient = currentRecipient else { return }
    transport.send(message, to: recipient)
}

// if let for branched handling
if let cached = cache[id] {
    return cached
} else {
    let fresh = await fetch(id)
    cache[id] = fresh
    return fresh
}
```

The reason this matters: deep nesting is the readability killer in Swift. Six levels of `if let` with the real work at the bottom is much harder to follow than five `guard let`s at the top and the work at the indentation level you started at.

### Nil-coalescing for defaults

`??` is purpose-built for "use this if non-nil, else that." Don't write the ternary form:

```swift
// Good
let name = user.name ?? "Guest"

// Stale
let name = user.name != nil ? user.name! : "Guest"
```

The ternary form involves a force-unwrap, which means a crash if the optional flips between the check and the unwrap (it can't here, but in concurrent code it can). `??` is safer and shorter.

### Optional chaining before unwrapping

When you're reaching through several optionals, chain them all and unwrap once at the end:

```swift
let firstByte = response?.data?.first  // short-circuits at the first nil
guard let firstByte else { return }
```

This is cleaner than nested `if let` for the read-only case. When you need to mutate one of the intermediates, you'll have to unwrap each step.

### Force-unwrap rules

Force unwraps (`!` and `try!`) cause production crashes. The rules:

- **Never** in app code paths that run in production.
- **Acceptable** in unit tests and previews, where a crash means the test failed loudly.
- **Acceptable** in one-off CLI tools or scripts where you'd rather crash than handle a case that "can't happen."
- **Acceptable** for IBOutlets (which the compiler basically forces) and for programmatically loaded bundle resources you control.

When you keep a force unwrap, write a one-line comment naming the invariant that guarantees non-nil:

```swift
// CGImage from a UIImage loaded from our asset catalog — guaranteed non-nil.
let cgImage = UIImage(resource: .placeholder).cgImage!
```

That comment becomes the receipt during code review. Without it, the reviewer has to reverse-engineer your assumption.

Everywhere else: `guard let`, `??`, `if let`, or surface the error.

## Error handling

### Untyped `throws` is the default

Swift 6 supports typed throws (`throws(MyError)`), which the compiler enforces. It's tempting to reach for, but on most surfaces it's a mistake.

The reason: errors evolve. You add a new failure mode, you need a new case. With untyped `throws`, that's an internal change. With typed throws, it's a breaking API change. Every caller has to update its catch.

```swift
// Default — public surfaces
func fetch(_ id: Article.ID) async throws -> Article { ... }
```

### When typed throws actually helps

Inside a feature module or library boundary where the error set is closed and stable, typed throws gives you exhaustive matching. The compiler forces every call site to handle every case. That's the win — and the cost.

```swift
// Inside a parser feature where the error set isn't expected to grow.
func parse(_ data: Data) throws(ParseError) -> Token { ... }
```

The judgment call: are you confident the error set won't grow over the next two years? If yes, typed throws makes downstream code more reliable. If no, untyped throws is easier to evolve.

I'm honestly not certain whether typed throws (SE-0413) shipped in Swift 6.0 or 6.1 — verify against the docs if you need to pin a version.

### Model errors as enums with associated values

Carry context in the error case, not in a stringly-typed description:

```swift
enum NetworkError: Error {
    case invalidURL
    case http(status: Int, body: Data?)
    case decoding(any Error)
    case offline
}
```

This lets `switch` exhaustively. Each case carries its own context. A crash report with `NetworkError.http(status: 503, body: ...)` is debuggable. A `NSError(domain: "Network", code: 503)` isn't.

### `Result` for closure-based APIs

`Result<T, Error>` exists as a value-typed alternative to throwing. With `async/await`, plain `try await` is cleaner — you don't typically need `Result`:

```swift
// Good
let article = try await store.fetch(id)

// Use Result only if you need to capture the outcome as a value to pass around.
```

The legitimate uses for `Result` in new code:

- Closure-based APIs that haven't migrated to async/await yet.
- Storing both success and failure as state (a screen that holds the last result).
- Sending across an actor boundary as data rather than as a thrown control-flow signal.

Otherwise, `try await` is the default.

### Wrap, don't swallow

When rethrowing, wrap the underlying error in a domain error that carries the original:

```swift
do {
    return try await session.data(for: request)
} catch let decoding as DecodingError {
    throw NetworkError.decoding(decoding)
} catch {
    throw NetworkError.transport(error)
}
```

A crash report with `NetworkError.decoding(_:)` plus the inner `DecodingError` tells you what went wrong. A bare `Error` doesn't.

### Never swallow user-visible errors

`print(error.localizedDescription)` from a button action is a bug. The user sees no feedback, the failure becomes invisible, and the next time the user reports "it didn't work" you have nothing to go on. Surface the error via alert, toast, or error state — anything the user can see and act on.

## Type system

### Default to `struct`. Class for identity or shared state

Models, view state, DTOs, value-shaped data — structs. Reach for class when the thing genuinely has identity in the world: an open file, a TCP connection, an Observable model the views subscribe to.

Value semantics prevent shared-mutable-state bugs and compose well with Swift 6 concurrency. Most data in an app is value-shaped.

### Model state as an enum, not a bag of booleans

This is one of the highest-leverage refactors you can make in a UI codebase. A bag of booleans makes impossible states representable. An enum makes them unrepresentable.

```swift
// Good — impossible combinations can't exist
enum LoadState<T> {
    case idle
    case loading
    case loaded(T)
    case failed(any Error)
}

// Stale — what's "isLoading && error != nil && value != nil"?
struct LoadState<T> {
    var isLoading: Bool
    var value: T?
    var error: (any Error)?
}
```

The enum form forces a single state at a time. The struct form lets a bug create combinations like "loading and loaded simultaneously," which then ripple through to UI that doesn't know which branch to render.

Same idea for routes and sheet types — they're enums for the same reason.

### Avoid `default` in switches over your own enums

When you switch on an enum you own, spell out every case. Don't write a `default:`. The reason: when you add a new case, the compiler warns you that the switch isn't exhaustive. A `default:` silently swallows new cases and they go missing from the UI.

```swift
// Good — adding a case forces an update
switch status {
case .pending:   icon = "clock"
case .shipped:   icon = "shippingbox"
case .delivered: icon = "checkmark.circle"
case .cancelled: icon = "xmark.circle"
}

// Brittle — adding .refunded just defaults to "xmark.circle" with no warning
switch status {
case .pending:   icon = "clock"
case .shipped:   icon = "shippingbox"
case .delivered: icon = "checkmark.circle"
default:         icon = "xmark.circle"
}
```

Exception: switching on an enum you don't own (typically from a framework that may add cases) — there `default:` is fine, since you don't control the case list.

### Raw-value enums for wire types

For enums that come from a server, declare a raw value and an `unknown` case for forward compatibility:

```swift
enum Status: String, Codable {
    case pending, shipped, delivered, cancelled
    case unknown
}

extension Status {
    init(from decoder: any Decoder) throws {
        let raw = try decoder.singleValueContainer().decode(String.self)
        self = Status(rawValue: raw) ?? .unknown
    }
}
```

Server adds `"refunded"` next week, your shipped app decodes it as `.unknown` instead of crashing on a missing case.

### `some` for opaque returns, generics for parameters, `any` for boxing

Three ways to express "some kind of `View`" or "some kind of `Equatable`," and they're not interchangeable.

```swift
// some — opaque, one concrete type, fixed
func makeButton() -> some View { ... }

// generic — specialized per call site
func wrap<Content: View>(@ViewBuilder content: () -> Content) -> some View { ... }

// any — existential, dynamic dispatch, boxed
var pendingViews: [any View] = []  // heterogeneous storage
```

`some` is the cheapest. The compiler picks one concrete type and uses it. `any` is the most flexible and the slowest — it boxes the value and dispatches through a witness table.

`func makeButton() -> any View` is almost always wrong. Use `some View`.

`[any View]` for storage is only correct when you genuinely need to store mixed types. In a `ForEach`, `[any View]` defeats the diff in the same way `AnyView` does — avoid.

### Synthesized conformances

Plain structs whose stored properties all conform to a protocol can synthesize the protocol themselves:

```swift
struct Article: Identifiable, Equatable, Hashable, Codable {
    let id: UUID
    var title: String
    var publishedAt: Date
}
```

Compile-time synthesis. No body needed.

### Hand-write `==` and `hash` for hot paths

Synthesized `Hashable` walks every stored property. For `Identifiable` types in large lists or sets, that can be much slower than comparing on the `id` alone:

```swift
extension Article: Equatable {
    static func == (lhs: Self, rhs: Self) -> Bool { lhs.id == rhs.id }
}
extension Article: Hashable {
    func hash(into hasher: inout Hasher) { hasher.combine(id) }
}
```

Use this when profiling shows hash performance hurting. Don't reach for it preemptively — for small lists, the synthesized version is fine.

I don't have a specific benchmark number for the speedup to cite — the difference depends heavily on the type's size and the lookup pattern.

### `~Copyable` for unique resources

Mark wrappers around file handles, sockets, or one-shot tokens as `~Copyable` so the compiler enforces single ownership:

```swift
struct FileHandle: ~Copyable {
    private let fd: CInt
    consuming func close() { _ = Darwin.close(fd) }
}
```

A copy of a file handle would mean two pieces of code each thinking they own the descriptor; that's a double-close waiting to happen. `~Copyable` makes that not compile.

### `borrowing` and `consuming` parameters

`borrowing` for temporary read access. `consuming` when the function takes ownership (the close-on-file-handle case above). The keywords communicate ownership intent and unlock noncopyable types.

These are an advanced surface — most app code doesn't need them. Reach for them when you're building primitives around unique resources.

## Generics and protocols

### Protocols express capabilities

```swift
protocol Cacheable: Sendable {
    var cacheKey: String { get }
    func encode() throws -> Data
}
```

A protocol describes what a type can do. It's not a place to put shared implementation through inheritance. When you find yourself reaching for a class hierarchy to share code, ask whether the shared piece is a capability you can express as a protocol with a default implementation.

### Constrain associated types

```swift
protocol Repository {
    associatedtype Item: Identifiable & Sendable where Item.ID: Sendable
    func fetch(_ id: Item.ID) async throws -> Item
}
```

Unconstrained associated types force callers to write the constraints everywhere they touch the protocol. Constrain at the source.

### Avoid `@retroactive` conformances

Conforming an external type to an external protocol (`extension Date: @retroactive Identifiable`) can produce duplicate-conformance issues at link time if any other module does the same. Wrap instead:

```swift
struct DatedRow: Identifiable {
    let date: Date
    var id: Date { date }
}
```

The wrapper keeps the conformance local to your module.

## Expression-form bodies

### Omit `return` in single-expression functions

```swift
// Good
var headerColor: Color { isError ? .red : .primary }

// Older
var headerColor: Color {
    if isError { return .red }
    return .primary
}
```

### `if` and `switch` as expressions

```swift
// Good
var tileColor: Color {
    if isCorrect { .green } else { .red }
}

var statusIcon: String {
    switch status {
    case .pending:   "clock"
    case .shipped:   "shippingbox"
    case .delivered: "checkmark.circle"
    case .cancelled, .unknown: "xmark.circle"
    }
}
```

This shape pairs well with the "one switch per state enum" pattern.

## Strings

### `replacing(_:with:)` not `replacingOccurrences(of:with:)`

This point has been hammered on by Swift-teaching blogs for years: `replacingOccurrences(of:with:)` walks the string at the UTF-16 code unit level. It splits emoji and combined characters apart. The modern `replacing(_:with:)` walks grapheme clusters, which is what users expect:

```swift
// Good — grapheme-cluster safe
let cleaned = title.replacing("🙃", with: "")

// Older — breaks on complex emoji
let cleaned = title.replacingOccurrences(of: "🙃", with: "")
```

For ASCII-only strings the difference is invisible; for any string the user can type, the new form is the correct one.

### `localizedStandardContains` for user-input search

For a search field where the user types arbitrary text, `localizedStandardContains` handles diacritics, case, and width-insensitivity correctly across locales. Plain `contains` doesn't. `localizedCaseInsensitiveContains` is closer but still locale-naive.

```swift
articles.filter { $0.title.localizedStandardContains(query) }
```

### `FormatStyle` for numbers, dates, currencies

Modern Swift has a typed format-style API that's locale-aware out of the box:

```swift
Text(price, format: .currency(code: "USD"))
Text(value, format: .number.precision(.fractionLength(2)))
Text(date, format: .dateTime.day().month().year())
Text(measurement, format: .measurement(width: .wide, usage: .road))
```

C-style format strings (`String(format: "$%.2f", price)`) don't respect locale and don't handle the long tail of currency, plural, and date conventions.

### Year format `"y"` not `"yyyy"`

When a format string is unavoidable, `"y"` is correct in all calendars and locales. `"yyyy"` is locale-broken for some calendars (Japanese imperial era, for example).

### Date parsing with strategies

```swift
let date = try Date(string, strategy: .iso8601)
```

Avoid `DateFormatter` for parsing in new code. The `Date.ParseStrategy` family is typed and doesn't have the global-cache footgun that `DateFormatter` has.

### `PersonNameComponents` for names

```swift
let components = PersonNameComponents(givenName: "Yoko", familyName: "Ono")
let formatted = components.formatted(.name(style: .long))
```

Handles localization and ordering. Don't write `"\(first) \(last)"` — that's wrong in cultures where surname comes first or where particles work differently.

### `count(where:)`

```swift
// One pass.
let urgent = articles.count(where: \.isUrgent)

// Two passes plus an intermediate allocation.
let urgent = articles.filter { $0.isUrgent }.count
```

## AttributedString

### Markdown for inline emphasis

`AttributedString` accepts Markdown at construction time, and `Text` accepts `AttributedString` directly:

```swift
let text = try AttributedString(markdown: "Hello **\(name)**, you have *3* new messages.")
Text(text)
```

### Localized Markdown via String Catalogs

```swift
let text = AttributedString(localized: "signin.welcome \(name)")
```

Translators can embed bold and italic via Markdown rather than concatenating styled `Text` views.

### iOS 26: per-run line height

```swift
var attributed = AttributedString("Hero")
attributed.lineHeight = .points(48)
Text(attributed)
```

See `modern-api.md` for the full line-height API.

## Collections

### `Identifiable` over `id: \.someProperty`

```swift
// Best
ForEach(articles) { article in
    ArticleRow(article: article)
}

// Acceptable when you can't add Identifiable conformance
ForEach(articles, id: \.slug) { article in
    ArticleRow(article: article)
}
```

### Never `UUID()` defaulted in `ForEach(id:)`

This is the worst SwiftUI antipattern I encounter often. A fresh UUID per call to `id:` invalidates identity every render — every row gets a new identity, every row's `@State` is reset, scroll position is lost. See `view-composition.md` for the full mechanics.

### `swift-algorithms` package

Apple's official extension package for slicing and combinatorics:

```swift
import Algorithms

for chunk in items.chunks(ofCount: 50) {
    await uploadBatch(Array(chunk))
}

for window in measurements.windows(ofCount: 3) {
    let avg = window.reduce(0, +) / 3
}
```

Don't hand-roll `stride` and `index(_:offsetBy:)` slicing. The package is maintained by Apple and the algorithms are tested and benchmarked.

### `swift-collections` package

```swift
import Collections

var lru = OrderedDictionary<URL, Image>()
var deque = Deque<Event>()
var heap = Heap<Task>()
```

`OrderedDictionary` for stable key iteration. `Deque` for queues. `Heap` for priority work. All maintained by Apple as part of the Swift ecosystem packages.

### Convert `ArraySlice` to `Array` for storage

Slices keep a reference to the parent storage. If you `prefix(20)` an array of 10,000 items and store the slice as state, the array isn't deallocated. For long-lived state, copy out:

```swift
let head = Array(articles.prefix(20))
```

## URLs and files

### `URL.documentsDirectory` family

```swift
let documents = URL.documentsDirectory
let caches = URL.cachesDirectory
let support = URL.applicationSupportDirectory
let temp = URL.temporaryDirectory
```

These replace the older `FileManager.default.url(for:in:appropriateFor:create:)` for the common case. Available since iOS 16.

### `url.appending(path:)`

```swift
let saved = URL.documentsDirectory
    .appending(path: "drafts")
    .appending(path: "\(article.id).json")
```

Never string concatenation, never the deprecated `appendingPathComponent(_:)`. The new form is type-safe and handles separator normalization.

### Document directory vs caches vs application support vs temporary

These each have a backup policy and a lifecycle:

- `URL.documentsDirectory` — user-visible files. Backed up to iCloud. Survives app updates.
- `URL.cachesDirectory` — derived, regeneratable data. Not backed up. The system can purge under storage pressure.
- `URL.applicationSupportDirectory` — generated content the user doesn't directly see (settings, generated assets). Backed up.
- `URL.temporaryDirectory` — purged at any time. Don't store anything you need past this run.

Putting a 200MB cache in `documentsDirectory` is how iCloud quotas get exhausted. Putting user-created data in `cachesDirectory` is how that data disappears.

## Foundation replacements

A grab-bag of Foundation modernizations:

- `replacing("a", with: "b")` not `replacingOccurrences(of:with:)`.
- `Subprocess` package (Swift 6.2+) for shelling out, not raw `Process`.
- `AttributedString` not `NSAttributedString` for new SwiftUI text.
- `Duration` not `TimeInterval` for time spans.
- `Date.now` not `Date()` for the current moment.
- `URL.documentsDirectory` not the `FileManager` URL builders.
- `Measurement<UnitX>` for physical quantities — distances, weights, temperatures. Free locale-aware conversion.

## Logging

### `os.Logger`, never `print`

```swift
import os.log

private let log = Logger(subsystem: "com.acme.app", category: "networking")

log.info("Fetched \(items.count, privacy: .public) items for user \(userID, privacy: .private)")
log.error("Refresh failed: \(error.localizedDescription, privacy: .public)")
```

A few things to know:

- Unified logging is fast, filterable in Console.app, and persisted across launches. `print` is none of those.
- Privacy interpolation matters. `.private` becomes `<private>` in non-Xcode log streams; `.public` doesn't. Logging raw user input as `.public` shows up in sysdiagnose dumps that get attached to feedback reports.
- One `Logger` per category. The subsystem is the bundle identifier or a stable namespace. The category is a logical area inside the app — `"networking"`, `"persistence"`, `"audio"`.

`print()` calls are stripped from release builds anyway. They're a debugging crutch, not production logging.

See `testing-and-debugging.md` for the unified logging deep dive.

## Imports

When `import SwiftUI` is present, you don't need `import UIKit` or `import AppKit`. SwiftUI transitively imports the right one for the platform you're building. `UIImage`, `NSImage`, `UIColor`, `NSColor` are already visible.

```swift
// Sufficient on iOS targets
import SwiftUI

// Redundant
import SwiftUI
import UIKit
```

`import Combine` is the exception — Combine is no longer transitive through SwiftUI. You need an explicit import for `ObservableObject`, `@Published`, and publishers.

## Localization

### `String(localized:)` not `NSLocalizedString`

```swift
let welcome = String(localized: "Welcome, \(name)")
```

Integrates with String Catalogs (`.xcstrings`, Xcode 15+) and surfaces missing keys at build time. `NSLocalizedString` still works but doesn't get the compile-time check.

### Plurals via String Catalogs or `.stringsdict`

A hardcoded English `"1 item" / "%d items"` breaks for half the planet — Arabic, Russian, Polish, and others have plural rules that don't map onto English. String Catalogs handle plurals via a UI in Xcode; `.stringsdict` does the same job in XML.

Don't hand-roll plural logic.

### `LocalizedStringResource`

For passing localized strings around as values (rather than rendering them immediately), `LocalizedStringResource` is the typed wrapper:

```swift
let resource = LocalizedStringResource("Welcome, \(name)")
Text(resource)
```

Useful when you're building an intermediate that gets rendered later — an alert message, a notification body.

### `Measurement<UnitX>` for physical quantities

```swift
let distance = Measurement(value: 5, unit: UnitLength.miles)
distance.formatted(.measurement(width: .wide, usage: .road))
```

Free unit conversion. Locale-aware output. Prevents the unit-mismatch bugs where the code compiles but the meaning is wrong.

## Codable

### Let synthesis do the work

Plain structs with all-`Codable` properties get `Codable` synthesized:

```swift
struct Article: Codable {
    let id: UUID
    var title: String
    var publishedAt: Date
}
```

### Custom `CodingKeys` only when JSON keys differ

When the JSON uses snake_case and the Swift type uses camelCase, set the decoder strategy rather than writing `CodingKeys`:

```swift
let decoder = JSONDecoder()
decoder.keyDecodingStrategy = .convertFromSnakeCase
```

Only declare `CodingKeys` when individual fields don't follow a uniform pattern.

### Custom `init(from:)` in an extension

When you do need custom decoding, put it in an extension. That preserves the synthesized memberwise initializer:

```swift
struct Article: Codable {
    let id: UUID
    var title: String
    var publishedAt: Date
}

extension Article {
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.id = try container.decode(UUID.self, forKey: .id)
        self.title = try container.decode(String.self, forKey: .title)
        // Tolerate epoch-second dates from a legacy endpoint.
        let raw = try container.decode(Double.self, forKey: .publishedAt)
        self.publishedAt = Date(timeIntervalSince1970: raw)
    }
}
```

### Optional fields

For fields that may be missing from the payload, use `decodeIfPresent`:

```swift
self.subtitle = try container.decodeIfPresent(String.self, forKey: .subtitle)
```

Not `decode(Optional<String>.self, forKey:)`, which expects the key present with a `null` value.

## Storage

- Never store tokens, refresh tokens, credentials, or sensitive personal records in `UserDefaults`. Use Keychain for small secrets/keys and appropriately protected files or databases for larger records.
- `URL.documentsDirectory` for user-created content. Backed up to iCloud.
- `URL.cachesDirectory` for derived, regeneratable data. Not backed up. System can purge.

See `persistence.md` for the SwiftData, Core Data, SQLiteData, and GRDB decision points.

## Concurrency quick wins

A few one-line idioms that come up in reviews:

- `Task.sleep(for: .seconds(1))`, never `Task.sleep(nanoseconds:)`.
- `Task { ... }` over `Task.detached { ... }` for view-driven work. `.detached` loses actor context and is almost always wrong.
- When an API offers both `async` and closure-based variants, use the `async` variant.
- `async let` and `TaskGroup` for fan-out — never sequential `await`s of independent work.
- `.task(id:)` modifier in SwiftUI re-runs the task automatically when the id changes, with automatic cancellation. Use this instead of `.onChange { Task { } }`.

Full concurrency story in `concurrency.md`.

## KeyPaths and dynamic member lookup

### Static member lookup at call sites

```swift
// Good
.clipShape(.circle)
.buttonStyle(.borderedProminent)
.toolbar { ToolbarItem(placement: .topBarTrailing) { ... } }

// Older
.clipShape(Circle())
.buttonStyle(BorderedProminentButtonStyle())
```

The static-member form reads cleaner and is the idiomatic call site. It's enabled by extensions on protocols like `Shape` and `ButtonStyle`.

### KeyPath in sort, filter, map

```swift
articles.sorted(using: KeyPathComparator(\.publishedAt, order: .reverse))
let titles = articles.map(\.title)
let urgent = articles.filter(\.isUrgent)
```

Less noise than the closure form. The KeyPath version is also slightly faster (the compiler can specialize on the keypath in a way it can't always do for a closure).

### `@dynamicMemberLookup`

```swift
@dynamicMemberLookup
struct ThemedColor {
    let theme: AppTheme
    subscript(dynamicMember keyPath: KeyPath<AppTheme.Colors, Color>) -> Color {
        theme.colors[keyPath: keyPath]
    }
}
```

Use sparingly. It hides surfaces from Xcode's autocomplete and tooling. Reach for it only when the call site savings genuinely outweigh the discoverability cost.

## Type bridging

### `Double` over `CGFloat`

Swift bridges `Double` to `CGFloat` transparently in most positions. Two exceptions where the bridge doesn't apply:

1. `inout CGFloat` parameters.
2. `CGFloat?` optionals.

For everything else, prefer `Double` literals and let the bridge handle it. Spacing tokens, padding, frame metrics are usually `CGFloat` in the design system (because the SwiftUI APIs want `CGFloat`), but you can pass `Double` literals freely.

### `Comparable` if the same sort closure appears twice

```swift
extension Article: Comparable {
    static func < (lhs: Self, rhs: Self) -> Bool {
        lhs.publishedAt < rhs.publishedAt
    }
}

articles.sorted()  // beats articles.sorted { $0.publishedAt < $1.publishedAt }
```

Make the comparison part of the type, not duplicated at every call site.

## Button actions

When a button action is a parameterless method, pass it as a reference rather than wrapping it in a closure:

```swift
// Good
Button("Save", systemImage: "tray", action: save)

// Wrapped closure does the same thing with extra noise
Button("Save", systemImage: "tray") { save() }
```

The reference form is one fewer closure for SwiftUI to compare, and participates in `Equatable` slightly better.

## Memory management

### `weak` and `unowned` for delegate-style references

For references that shouldn't extend the lifetime of the referent — typical for delegates, observers, parent pointers — use `weak`:

```swift
final class PlayerView: UIView {
    weak var delegate: (any PlayerViewDelegate)?
}
```

`unowned` works similarly but assumes the reference is always valid; it crashes if the referent goes away. Use it only when you can prove the lifetimes overlap. `weak` is the safer default.

### Capture lists in closures

The most common source of retain cycles is a closure stored on an object, capturing `self`:

```swift
// Cycle: self holds the closure, the closure captures self.
store.onUpdate = {
    self.refresh()
}

// Fixed
store.onUpdate = { [weak self] in
    self?.refresh()
}
```

When the closure is short-lived (an `await` continuation, a `.task` closure, a one-shot callback that fires and finishes), capture lists usually aren't necessary — the closure isn't around long enough to form a cycle. When the closure is stored on a long-lived object, `[weak self]` is the right default.

For SwiftUI `View` structs, this comes up less — structs don't form retain cycles with closures the same way classes do. It's `@Observable` classes and reference-typed stores where you watch for it.

## Don't

A consolidated list of patterns to flag during review:

- Force-unwraps in production code paths.
- `print()` for production logging — use `os.Logger`.
- Swallowing errors with `print(error.localizedDescription)`.
- C-style format strings — use `FormatStyle`.
- `replacingOccurrences(of:with:)` — use `replacing(_:with:)`.
- `DateFormatter` per-call — use `.formatted(.dateTime...)`.
- Bag-of-booleans state — use an enum.
- `default:` in switches over your own enums.
- `Task.detached` for view-driven work.
- `Task.sleep(nanoseconds:)` — use `Task.sleep(for:)`.
- Tokens or credentials in `UserDefaults` — use Keychain.
- `import UIKit` or `import AppKit` next to `import SwiftUI` — they're transitive.
- `@retroactive` conformances on external types — wrap them instead.
- `any P` when `some P` works — existentials are the slow path.
- Synthesized `Hashable` on `Identifiable` types in hot paths — hand-write `id`-based `==` and `hash`.
- `Result<T, Error>` in new async code — `try await` is the default.
