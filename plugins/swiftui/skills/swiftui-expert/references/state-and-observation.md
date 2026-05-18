# State and Observation

Target: Swift 6.3 / iOS 26 / Xcode 26. May 2026.

This is the `@Observable` deep dive: ownership, environment injection, the `@AppStorage` trap, fine-grained tracking, and the `Observations { }` async sequence. Opinionated. State the rule, then the code.

For SwiftData production rules see `persistence.md`. For actor isolation and `.task` see `concurrency.md`. For body re-eval / identity see `lifecycle.md`. For `App.swift` architecture see `architecture.md`.

---

## The ownership matrix (lead with this)

This is the first thing to know. Every property wrapper question reduces to one row of this table.

| Need | Use |
|---|---|
| Local value state (`String`, `Int`, `Bool`, struct) | `@State private var` |
| Own an `@Observable` instance | `@State private var model = MyModel()` |
| Receive an `@Observable` read-only | plain `let model: MyModel` |
| Receive an `@Observable` and need `$`-bindings | `@Bindable var model: MyModel` |
| Receive shared `@Observable` from environment | `@Environment(MyModel.self) var model` |
| Receive shared from environment, need bindings | `@Environment(MyModel.self) var model` + inline `@Bindable` |
| UserDefaults persistence (view-local) | `@AppStorage("key") var ...` — NEVER inside `@Observable` |
| Scene-specific persistence | `@SceneStorage("key") var ...` |
| SwiftData query (views only) | `@Query var items: [Item]` |
| Focus management | `@FocusState var field: Field?` (use Hashable enum) |
| Child mutates a parent value | `@Binding var` |

`@StateObject` / `@ObservedObject` / `@EnvironmentObject` / `@Published` / `ObservableObject` do not appear in this table. They are deprecated for new code. If you see them in a code review, flag them.

---

## `@Observable` — the modern macro

`@Observable` replaces the entire `ObservableObject` + `@Published` + `@StateObject` + `@ObservedObject` family. One macro, four wrappers retired.

### What it does

The `@Observable` macro rewrites your stored properties into a `_$observationRegistrar`-backed pair: each getter calls `access(keyPath: \.x)`, each setter wraps in `withMutation(keyPath: \.x)`. During body evaluation, SwiftUI subscribes to exactly the keypaths the view read. When the property changes, only the views that read **that specific keypath** are invalidated.

```swift
@Observable
final class UserProfile {
    var name = ""
    var avatarURL: URL?
    var isPremium = false

    @ObservationIgnored
    var sessionToken: String?     // Service detail, not UI state.

    @ObservationIgnored
    private var imageCache: [URL: UIImage] = [:]
}
```

### Why this beats `ObservableObject`

- **Keypath precision.** A view that reads only `profile.name` does not re-render when `profile.isPremium` flips. With `@Published`, every subscriber invalidates on every emission.
- **Nested observation works.** `Model { Profile { ... } }`: mutating `model.profile.name` invalidates views that read `model.profile.name`. With `ObservableObject` you had to manually re-emit on the outer object.
- **Collections work too.** `items[42].title` mutation invalidates only the row reading `items[42]`; not the parent that iterates over `items`.
- **Less ceremony.** No `@Published` on each property. No `ObjectWillChangePublisher` plumbing.

### `@ObservationIgnored`

Mark anything that should **not** trigger view updates:

- Injected dependencies / services (the network client doesn't change; tracking it is pure overhead).
- Caches, debounce timers, locks, identifiers.
- Backing storage for `@AppStorage`-replacement patterns (see below).
- Derived computed-property scratch storage.

```swift
@Observable
final class FeedViewModel {
    var items: [Post] = []
    var isLoading = false

    @ObservationIgnored private let api: FeedAPI
    @ObservationIgnored private var fetchTask: Task<Void, Never>?

    init(api: FeedAPI) { self.api = api }
}
```

### MainActor isolation

In **Swift 6.2+ projects with default MainActor isolation enabled** (the recommended Xcode 26 setting for app targets), `@Observable` classes are implicitly `@MainActor`. You do not need to annotate.

In **older projects** or SPM packages with `defaultIsolation = nil`, annotate explicitly:

```swift
@MainActor
@Observable
final class SettingsStore { ... }
```

Rule: app targets default-isolated; SPM packages annotate. See `concurrency.md` for details.

### Computed properties

Computed properties that read tracked stored properties are tracked automatically:

```swift
@Observable
final class Cart {
    var items: [LineItem] = []
    var total: Decimal { items.reduce(0) { $0 + $1.amount } }   // Tracked via items.
}
```

A view reading `cart.total` is invalidated when `cart.items` mutates — the registrar transitively records the dependency.

Computed properties that route through external state (UserDefaults, an external cache, a service call) need manual tracking — see the `@AppStorage` workarounds below.

---

## The `App.swift` singleton pattern (App owns shared state)

This is the canonical 2026 shape. App-level shared state lives as `@State` in `@main` and is injected via `.environment(_:)`. No singletons, no DI containers, no Service Locator.

```swift
@main
struct MyApp: App {
    @State private var theme = Theme.shared
    @State private var router = AppRouter()
    @State private var auth = AuthStore()
    @State private var settings = SettingsStore()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(theme)
                .environment(router)
                .environment(auth)
                .environment(settings)
        }
    }
}
```

### Why `@State`, not `@StateObject`

`@StateObject` is for `ObservableObject`. `@Observable` is a different system. `@State` is the correct wrapper for owning an `@Observable` instance — it gives the same once-and-only-once initialization semantics. Without `@State`, the object would be recreated on every body re-eval (and a fresh `Theme()` every render would defeat the whole point).

### Why `.environment(_:)`, not a custom `EnvironmentKey`

`.environment(observable)` is the type-safe path for `@Observable` instances. SwiftUI generates the key automatically from the type. Custom `EnvironmentKey`s with `defaultValue` are still useful for plain `Sendable` services where you want preview-friendly defaults, but for `@Observable` singletons inject directly:

```swift
.environment(theme)               // Type-keyed; consume with @Environment(Theme.self).
```

Consume with:

```swift
struct SomeView: View {
    @Environment(Theme.self) private var theme
    @Environment(AppRouter.self) private var router
    ...
}
```

### Why this works at scale

This is the IceCubesApp pattern. Their `@main` injects ten such singletons. Apple's Backyard Birds uses an identical shape. No "container", no module-level globals, no `static let shared` patterns surfaced to call sites. Singletons are still singletons (one instance per app) — they just live behind the environment, which means they are mockable and previews work.

```swift
// In a preview:
#Preview {
    ContentView()
        .environment(Theme.preview)
        .environment(AppRouter())
        .environment(AuthStore.loggedIn)
}
```

---

## `@AppStorage` + `@Observable` — the trap

This is the single most-hit pitfall in 2026 SwiftUI code. Read it.

### The problem

`@AppStorage` is a SwiftUI **DynamicProperty**. It works as a property wrapper on `View`, `App`, or `Scene` — types that participate in the SwiftUI view graph. It does **not** compose with `@Observable` because the macro rewrites stored properties into computed properties, and `@AppStorage` cannot wrap a computed property.

The failure mode is the worst kind: **it compiles cleanly. It silently doesn't work.**

```swift
@Observable
final class Settings {
    @AppStorage("darkMode") var darkMode = false   // COMPILES. DOES NOT TRIGGER UPDATES.
}
```

Putting `@AppStorage` directly inside an `@Observable` class will not invalidate any view. Toggle the value all you want — no view re-renders. You will spend hours debugging it.

`@ObservationIgnored @AppStorage` also compiles. Also does not notify. Same trap, marginally clearer intent.

### The working pattern — stored `var` + `didSet` over a plain `Storage` class (IceCubesApp shape, verified against main)

The actually-shipping pattern in `Packages/Env/Sources/Env/UserPreferences.swift` on IceCubesApp's `main` branch. The `@Observable` macro instruments **stored** properties — so make the outer property a stored `var` with `didSet`, and have a plain (non-`@Observable`) inner `Storage` class hold the `@AppStorage`-marked properties as a UserDefaults read/write API. A private `init()` seeds the outer stored values from inner storage at startup.

```swift
import SwiftUI

@MainActor
@Observable
public final class UserPreferences {
    // Inner Storage is a PLAIN class — NOT @Observable.
    // @AppStorage here is used purely as a UserDefaults read/write wrapper.
    final class Storage {
        @AppStorage("darkMode") var darkMode: Bool = false
        @AppStorage("fontSize") var fontSize: Double = 16
        @AppStorage("preferred_browser") var preferredBrowser: PreferredBrowser = .inAppSafari
    }

    private let storage = Storage()

    // Outer properties are STORED vars with didSet — the @Observable macro
    // instruments these normally, so views reading them re-render on change.
    public var darkMode: Bool {
        didSet { storage.darkMode = darkMode }
    }
    public var fontSize: Double {
        didSet { storage.fontSize = fontSize }
    }
    public var preferredBrowser: PreferredBrowser {
        didSet { storage.preferredBrowser = preferredBrowser }
    }

    // Private init seeds outer stored values from persistent storage at startup.
    private init() {
        darkMode = storage.darkMode
        fontSize = storage.fontSize
        preferredBrowser = storage.preferredBrowser
    }

    public static let shared = UserPreferences()
}
```

**Why it works:**
- The outer `var darkMode` is a stored property, so the `@Observable` macro synthesizes `access(keyPath: \.darkMode)` / `withMutation(keyPath: \.darkMode)` automatically. Views observing `userPreferences.darkMode` register dependencies normally and re-render on change.
- `didSet` mirrors the value to `storage.darkMode`, which writes UserDefaults via the inner `@AppStorage`'s wrappedValue setter.
- The inner `Storage` is a plain class because `@AppStorage`'s `wrappedValue` accessor reads/writes UserDefaults regardless of the host type's observation system — we just use it as a UserDefaults abstraction. (Putting `@Observable` on `Storage` would do nothing useful and adds noise.)
- The private `init` ensures outer stored values are seeded from persisted UserDefaults at startup. Without it the outer values would default to whatever Swift would synthesize.

**Pitfalls in this shape:**
- Don't mark the outer property `@ObservationIgnored` — that suppresses tracking and breaks observation. Just `public var darkMode: Bool { didSet { … } }`.
- Don't make the outer property a computed property over storage — only stored properties get observation tracking synthesized.
- Don't mark `Storage` as `@Observable` — pointless and confusing.

### Alternative — manual `access` / `withMutation` (when you need it)

If you have a single persisted property and don't want to add a `Storage` shim, you can call the macro's primitives directly. This is what the `@Observable` macro generates for stored properties — calling them manually works for computed properties over external state:

```swift
@Observable
final class Settings {
    var hasOnboarded: Bool {
        get {
            access(keyPath: \.hasOnboarded)
            return UserDefaults.standard.bool(forKey: "hasOnboarded")
        }
        set {
            withMutation(keyPath: \.hasOnboarded) {
                UserDefaults.standard.set(newValue, forKey: "hasOnboarded")
            }
        }
    }
}
```

This shape is more verbose per property than the IceCubesApp pattern. Use it for one-off persisted values; reach for the IceCubesApp shape when you have several.

### Optional — generic helper to dry up the manual shape

```swift
@propertyWrapper
struct ObservableUserDefault<Value> {
    let key: String
    let defaultValue: Value

    var wrappedValue: Value {
        get { UserDefaults.standard.object(forKey: key) as? Value ?? defaultValue }
        set { UserDefaults.standard.set(newValue, forKey: key) }
    }
}

@Observable
final class UserPreferences {
    @ObservationIgnored
    private var _darkMode = ObservableUserDefault(key: "darkMode", defaultValue: false)

    var darkMode: Bool {
        get {
            access(keyPath: \.darkMode)
            return _darkMode.wrappedValue
        }
        set {
            withMutation(keyPath: \.darkMode) {
                _darkMode.wrappedValue = newValue
            }
        }
    }
}
```

Same idea — the `@ObservationIgnored` backing wrapper handles `UserDefaults`, the outer computed property handles observation. The helper just dries up the boilerplate.

The `access(keyPath:)` call registers the view's dependency on that keypath; `withMutation(keyPath:)` fires the invalidation. This is what the `@Observable` macro generates for normal stored properties — you do it manually here because the underlying storage is external.

### Rule of thumb

- `@AppStorage` lives in **views** for view-local flags (theme toggle in Settings screen, "hasSeenOnboarding" check on launch).
- Anything that needs to live in a service or model object — use the manual `access`/`withMutation` bridge above, or wrap it in the `ObservableUserDefault` helper.
- Never `@AppStorage` directly inside `@Observable`. Never `@ObservationIgnored @AppStorage` either.

This trap is the single most-hit pitfall in 2026 SwiftUI state code. State the rule plainly when you see it.

---

## `@Bindable` — the modern `$` projection

`@Bindable` produces `$` bindings into an `@Observable` instance you received from elsewhere. It does **not** own. The source must live in `@State` higher up or in `@Environment`.

### As a property wrapper

```swift
struct EditProfileView: View {
    @Bindable var profile: UserProfile

    var body: some View {
        Form {
            TextField("Name", text: $profile.name)
            Toggle("Premium", isOn: $profile.isPremium)
        }
    }
}
```

### Inline form for environment-received models

When you grab an `@Observable` from `@Environment` and need bindings, use `@Bindable` as an inline `var` declaration inside `body`:

```swift
struct SettingsScreen: View {
    @Environment(SettingsStore.self) private var settings

    var body: some View {
        @Bindable var settings = settings    // Local re-binding for `$` access.
        Form {
            TextField("Username", text: $settings.username)
            Toggle("Notifications", isOn: $settings.notificationsEnabled)
        }
    }
}
```

That `@Bindable var settings = settings` line is the canonical pattern. It does not create a copy — the reference is shared. It just introduces the `$`-projection locally.

### The `Bindable(_:)` initializer

For places where you can't use the property wrapper directly (sheet content, inside closures), construct one:

```swift
.sheet(isPresented: $showSheet) {
    SettingsForm(settings: Bindable(settingsStore))
}
```

### What `@Bindable` is not

- Not for ownership. Use `@State` for that.
- Not for `ObservableObject` — use `@ObservedObject` (legacy) for those. New code shouldn't have `ObservableObject`.
- Not a binding type itself. It's a projection mechanism.

---

## `Binding(get:set:)` in body — forbidden

Don't write custom `Binding(get:set:)` closures inline in `body`. They re-execute on every body call and obscure flow. Use `@State` + `.onChange(of:initial:_:)` instead.

```swift
// Bad
struct EditView: View {
    @Bindable var model: UserModel

    var body: some View {
        TextField("Name", text: Binding(
            get: { model.name },
            set: { model.name = $0; model.save() }
        ))
    }
}

// Good
struct EditView: View {
    @Bindable var model: UserModel

    var body: some View {
        TextField("Name", text: $model.name)
            .onChange(of: model.name) { _, _ in
                model.save()
            }
    }
}
```

If you genuinely need a derived binding (e.g., converting between types), extract to a computed property on the model or use a custom view modifier — not inline `Binding(get:set:)`.

---

## Fine-grained observation (avoid broad models)

One `AppModel` with everything is a re-render bomb. Every view that touches it ends up depending on all of it. Even with keypath-precise tracking, that "everything in one bag" structure encourages views to read the whole model.

### Bad

```swift
@Observable
final class AppModel {
    var feed: [Post] = []
    var notifications: [Notification] = []
    var profile: UserProfile = .empty
    var settings: Settings = .default
    var currentRoute: Route = .home
    var isOnline = true
    // ...
}

@main struct MyApp: App {
    @State private var model = AppModel()
    var body: some Scene {
        WindowGroup { ContentView().environment(model) }
    }
}
```

Now `FeedRow` reads `model.feed[i]`, `NotificationBadge` reads `model.notifications.count`, `RouterView` reads `model.currentRoute`. They all share the same instance. Every change ripples through the same registrar. Worse — refactoring becomes painful, every view ends up with `@Environment(AppModel.self)` and a hand-wave dependency on "the app".

### Good

```swift
@Observable final class FeedStore { var posts: [Post] = [] }
@Observable final class NotificationStore { var items: [Notification] = [] }
@Observable final class AuthStore { var profile: UserProfile = .empty }
@Observable final class Settings { var theme: Theme = .system }
@Observable final class AppRouter { var current: Route = .home }

@main struct MyApp: App {
    @State private var feed = FeedStore()
    @State private var notifications = NotificationStore()
    @State private var auth = AuthStore()
    @State private var settings = Settings()
    @State private var router = AppRouter()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(feed)
                .environment(notifications)
                .environment(auth)
                .environment(settings)
                .environment(router)
        }
    }
}
```

Now each view consumes exactly the store it needs. `FeedRow` takes `@Environment(FeedStore.self)`. `NotificationBadge` takes `@Environment(NotificationStore.self)`. Concerns are separated, dependencies are explicit, and the observation graph stays narrow.

### When to go even narrower

Per-row or per-cell `@Observable` instances are sometimes warranted — when each row has state that mutates independently and you want the row to be the unit of invalidation:

```swift
@Observable
final class PostRowModel {
    var post: Post
    var isExpanded = false
    var likeCount: Int
    init(post: Post) { self.post = post; self.likeCount = post.likeCount }
}

struct PostRow: View {
    @State private var model: PostRowModel

    init(post: Post) {
        _model = State(initialValue: PostRowModel(post: post))
    }

    var body: some View { ... }
}
```

This is appropriate for rows with independent local mutation (expand/collapse, optimistic like, inline edit). For pure display rows that just render a `Post` value, plain `let post: Post` and let the parent's observation handle invalidation.

---

## `@State` rules

A short list. Memorize.

1. **Always `private`.** `@State var foo` (no `private`) is a code smell — it leaks the wrapper to call sites which can't usefully consume it. Even worse, makes the assumption "this is internal" non-enforced.
2. **Never accept `@State` from a parent.** Initial values are captured once; subsequent parent updates are ignored. If a parent needs to push values, the wrapper is wrong — use a plain `let` for read-only, `@Bindable` for write, or move ownership to the parent.
3. **`@State` can hold non-observable caches.** A `CIContext`, a `NumberFormatter`, an `MTLDevice` — these need to live across body re-evals but aren't observable. `@State` is the correct wrapper.

```swift
struct FilterView: View {
    @State private var context = CIContext()   // Cached, never re-created.
    @State private var input: String = ""      // View-local value state.
    @State private var processor = ImageProcessor()  // Owned @Observable.
}
```

If you find yourself wanting to initialize `@State` from a parent's data, you want a different shape:

- Read-only display → `let value: T` (no wrapper).
- Two-way → `@Binding var value: T` or pass an `@Observable` model.
- Per-`id` initialization → use `.task(id:)` to react to changes, or `.id()` modifier to force-recreate the subview.

---

## `@SceneStorage` — per-scene restoration

`@SceneStorage` persists state per scene (per window on Mac/iPad, per scene on iOS). Use it for things that should restore when the user kills and relaunches the app on a particular window: selected tab, expanded sections, scroll position.

```swift
struct RootView: View {
    @SceneStorage("rootTab") private var selection: Tab = .home

    var body: some View {
        TabView(selection: $selection) {
            Tab(value: .home) { HomeView() }
            Tab(value: .search) { SearchView() }
            Tab(value: .profile) { ProfileView() }
        }
    }
}
```

Rules:

- Limited to `RawRepresentable` of primitive types (`String`, `Int`, `Double`, `Bool`, `Data`, `URL`). For enums use `enum Tab: String, RawRepresentable`.
- Per-scene. Two windows on Mac have independent state.
- Survives kill and relaunch via the system's state restoration.

Don't use `@SceneStorage` for app-wide state (use `@AppStorage` for view-local persisted settings or a service for shared model state). Don't use it for sensitive data — same caveat as `@AppStorage`: plaintext in restoration files.

---

## `@FocusState` — keyboard focus

For multi-field forms, use a `Hashable` enum and `.focused(_:equals:)`:

```swift
struct LoginForm: View {
    enum Field: Hashable {
        case email
        case password
    }

    @FocusState private var focusedField: Field?
    @State private var email = ""
    @State private var password = ""

    var body: some View {
        Form {
            TextField("Email", text: $email)
                .focused($focusedField, equals: .email)
                .submitLabel(.next)
                .onSubmit { focusedField = .password }

            SecureField("Password", text: $password)
                .focused($focusedField, equals: .password)
                .submitLabel(.done)
                .onSubmit { focusedField = nil }
        }
        .onAppear { focusedField = .email }
    }
}
```

Rules:

- Always optional (`Field?`) when you have an enum — `nil` means no field focused.
- Enum cases for each field — avoids stringly-typed bugs.
- Set focus in `.onAppear` or in response to actions; don't bind it to derived state.
- Use `.submitLabel(_:)` + `.onSubmit { }` to make the keyboard's return key advance fields.

For a single boolean focus state (one field), `@FocusState private var isFocused: Bool` is fine.

---

## `Observations { }` async sequence (Swift 6.2+, iOS 26)

`Observations { ... }` is an `AsyncSequence` that emits whenever any `@Observable` keypath read inside the closure changes. Use it when you need to react to observable changes **outside** of SwiftUI's body tracking — in a `Task`, a service, a non-View type.

```swift
@Observable
final class SearchModel {
    var query = ""
    var results: [Result] = []
}

actor SearchService {
    func observe(_ model: SearchModel) async {
        let stream = Observations { model.query }
            .debounce(for: .milliseconds(300))

        for await query in stream {
            let results = await search(query)
            await MainActor.run { model.results = results }
        }
    }
}
```

### Semantics

- **Did-set.** Each iteration delivers the *post-mutation* value. (The older `withObservationTracking` was will-set — one-shot, fired before the mutation.)
- **Transactional snapshots.** Synchronous mutations between awaits are coalesced into one emission. Set `model.query = "a"; model.query = "ab"; model.query = "abc"` in a tight loop — you get one iteration with `"abc"`.
- **Tracks all keypaths inside the closure.** `Observations { model.query + model.filter.text }` emits when either changes.

### Cancellation

There is no `AnyCancellable`. Wrap the `for await` in a `Task`, store the handle, and `task.cancel()` on teardown. Weakly capture `self` and the observed object to avoid retain cycles.

```swift
@Observable
final class SearchViewModel {
    var query = ""
    var results: [Result] = []

    @ObservationIgnored private var watchTask: Task<Void, Never>?

    func start() {
        watchTask = Task { [weak self] in
            guard let self else { return }
            let stream = Observations { self.query }
                .debounce(for: .milliseconds(300))
            for await query in stream {
                guard !Task.isCancelled else { return }
                self.results = await fetch(query)
            }
        }
    }

    func stop() {
        watchTask?.cancel()
        watchTask = nil
    }

    deinit { watchTask?.cancel() }
}
```

### When to reach for it

- Bridging `@Observable` state to non-SwiftUI consumers (services, background actors, network coordinators).
- Replacing Combine `debounce`/`throttle` pipelines on plain models.
- Observing state from a background actor that needs to react to UI-side mutations.

### When not to

- Inside a `View` body — that's SwiftUI's job; reading the property in `body` is enough.
- For one-shot reactions to UI changes — use `.onChange(of:initial:_:)`.

### Back-deployment

`Observations` is iOS 26+ and not back-deployed. For iOS 17–25 fall back to `withObservationTracking` (will-set, re-arm via recursion) or expose state as a manual `AsyncStream`. Wrap usage with `if #available(iOS 26, *)` checks if your target floor is older.

Prefer `Observations { }` over hand-rolled `withObservationTracking` + recursion when your target permits. The hand-rolled pattern is bug-prone (missing re-arms, unwanted re-fires).

---

## `Identifiable` — prefer over `id: \.someProperty`

`ForEach` and similar APIs accept either:

```swift
ForEach(items) { item in ... }            // Item: Identifiable, uses item.id.
ForEach(items, id: \.name) { item in ... } // Manual key path.
```

Prefer `Identifiable`. It makes the identity contract explicit on the type, not at every call site. It also keeps view identity stable when the type evolves — changing the key from `\.name` to `\.uuid` across the codebase is fragile.

```swift
// Prefer
struct Item: Identifiable {
    let id = UUID()
    var name: String
}
ForEach(items) { item in ItemRow(item: item) }

// Avoid
ForEach(items, id: \.name) { item in ItemRow(item: item) }
```

For SwiftData `@Model` classes, `Identifiable` is auto-conformed via `persistentModelID` — no manual conformance needed.

Anti-pattern flagged for review: **`ForEach(items, id: \.self)` for non-Hashable identity-by-value types**. That works for primitives but masks the real identity question. If the type has a real ID, use it.

Worse anti-pattern: **`id: UUID()` in `ForEach`**. That generates a fresh `UUID` on every body re-eval, so every row "changes identity" every render — kills animation, kills performance, kills focus retention. Never do this.

---

## SwiftData (briefly — full reference in `persistence.md`)

This file covers SwiftData only as it intersects with state management. Production rules live in `persistence.md`.

- `@Query var items: [Item]` works **only in views**. Not in `@Observable` classes, not in services, not in actors.
- For non-view contexts, use `FetchDescriptor<Item>` against a `ModelContext`.
- `@Query` predicates use the `#Predicate { ... }` macro and require static constants. Dynamic filters push the `@Query` into a child view that takes the filter as an init parameter.
- `@Model` classes get `Identifiable` for free.
- `ModelContext` is **not** `Sendable` — never pass across actors. `ModelContainer` is `Sendable` — that's the only thing safe to pass.

Quick example:

```swift
@Model
final class Task {
    var title: String
    var isComplete: Bool
    var created: Date

    init(title: String, isComplete: Bool = false) {
        self.title = title
        self.isComplete = isComplete
        self.created = .now
    }
}

struct TaskList: View {
    @Query(filter: #Predicate<Task> { !$0.isComplete }, sort: \.created)
    private var tasks: [Task]

    var body: some View {
        List(tasks) { task in
            TaskRow(task: task)
        }
    }
}
```

For migrations, `VersionedSchema`, `@ModelActor`, CloudKit limitations, and the production checklist — see `persistence.md`.

---

## TextField + numeric input

For numeric input use the `value:format:` initializer plus an explicit keyboard type:

```swift
TextField("Score", value: $score, format: .number)
    .keyboardType(.numberPad)
```

For currency:

```swift
TextField("Price", value: $price, format: .currency(code: "USD"))
    .keyboardType(.decimalPad)
```

For dates:

```swift
TextField("Date", value: $date, format: .dateTime)
```

The `format:` parameter is a `FormatStyle`. It handles parsing and validation. Don't manually `Int($text)` from a `String` TextField — that's the legacy pattern and loses formatting.

For multi-line input use `TextField("Notes", text: $notes, axis: .vertical)` (iOS 16+) or `TextEditor($notes)` for full editor behavior.

---

## Combine — what survives in 2026

Combine is not deprecated. But for new state pipelines, prefer `AsyncSequence` + `Observations { }`.

### Drop Combine for

- New state observation, debounce, throttle, map, filter pipelines on `@Observable` models.
- Replacing `@Published` + `sink { }` with `Observations { }` + `for await`.
- Any new ObservableObject — don't write one.

### Keep Combine for

- Apple framework bridges:
  - `NotificationCenter.publisher(for:)` — easier than wrapping `addObserver` (though `NotificationCenter` now has async sequence APIs on iOS 26).
  - `Timer.publish(every:on:in:)` consumed with `.onReceive` in SwiftUI for ticker UIs.
  - KVO publishers on legacy Cocoa objects.
  - `URLSession.dataTaskPublisher` for legacy code (new code uses `URLSession.data(for:)`).
- Existing pipelines you don't want to rewrite. Don't migrate working Combine code unless you have a reason.

### Never mix

Do not put `@Published` and `@Observable` in the same type. They are two distinct observation systems and they will not compose. If you migrate from `ObservableObject` to `@Observable`, migrate everything on that type — not one property at a time.

See `concurrency.md` for `AsyncSequence` patterns and the `.task` rules.

---

## Cross-references

- SwiftData production rules and migrations → `persistence.md`.
- Approachable Concurrency, actor isolation, `.task`, `Sendable` → `concurrency.md`.
- Body re-eval, view identity, `.task(id:)` vs `onAppear` → `lifecycle.md`.
- `App.swift` shape, MV pattern, ViewModel triggers, modularization → `architecture.md`.
- View extraction over computed properties → `view-composition.md`.

---

## Anti-patterns

Flag these in code review:

- **`ObservableObject` / `@Published` / `@StateObject` / `@ObservedObject` / `@EnvironmentObject` for new code.** Migrate to `@Observable` + `@State` + plain `let` + `@Bindable` + `@Environment`.
- **`@AppStorage` directly inside `@Observable`** — silently doesn't trigger view updates. Use the manual `access`/`withMutation` bridge or wrap in the `ObservableUserDefault` helper.
- **`@ObservationIgnored @AppStorage`** — compiles, doesn't notify. Same trap.
- **Mixing `@Published` and `@Observable` in the same type.** Two systems; they don't compose.
- **One huge `AppModel`** with feed, notifications, settings, router, profile all in one observable. Broad observation = re-render bomb. Decompose into per-domain stores.
- **`Binding(get:set:)` in `body`.** Use `@State` + `.onChange(of:initial:_:)` or `@Bindable` + computed property on the model.
- **`@State` for receiving** an `@Observable` from a parent. Use plain `let` (read-only) or `@Bindable` (with bindings).
- **`@StateObject` for new `@Observable`.** Use `@State`. `@StateObject` is for `ObservableObject`.
- **`.environmentObject(_:)` for `@Observable`.** Use `.environment(_:)`. They are different injection systems.
- **Custom `EnvironmentKey` for primary `@Observable` singletons.** Use `.environment(_:)` directly — the type is its own key.
- **`id: UUID()` in `ForEach`.** Generates fresh IDs every render; kills identity.
- **`ForEach(items, id: \.self)`** on rich types when the type has a real ID — use `Identifiable`.
- **`@State` without `private`.** Always private.
- **Storing tokens or PII in `@AppStorage` / `UserDefaults`.** Use Keychain. See `persistence.md`.
- **`Task { ... }` in `body`** for observation. Use `.task` (auto-cancels) or `Observations { }` from a service.
- **Computing derived state in `body`** that could be a computed property on the model. Move expensive derivations to the model so they're cached by the registrar.
