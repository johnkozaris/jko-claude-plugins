# SwiftUI view lifecycle

The lifecycle model in SwiftUI is the area where UIKit habits hurt the most. The shape of it is genuinely different. There's no `viewDidLoad`, the View struct's `init` is not the view's lifecycle, and "did appear" doesn't mean "is visible". This file explains the mental model first, then walks through each of the hooks (`.task`, `.onAppear`, `.onChange`, `.onReceive`), then covers view identity, app lifecycle, and the gotchas that come up most often.

Cross-references:

- State wrappers in depth: `state-and-observation.md`.
- MainActor, Sendable, structured concurrency: `concurrency.md`.
- Body extraction and identity from a performance angle: `performance.md`.
- Navigation lifecycle (stack push/pop, sheets): `navigation.md`.

## The mental model

A SwiftUI `View` is a value, not an object. SwiftUI creates and discards `View` structs constantly during normal use. Every time the parent's body runs, every child it returns is reconstructed. The struct's `init` and `deinit` are not the view's lifecycle. What persists is a separate thing: the view's *node* in the AttributeGraph.

Three points worth holding onto:

The first is that View structs are cheap and disposable. They're descriptions of UI, not the UI itself. SwiftUI may construct the same view's struct hundreds or thousands of times during a session, and that's expected, not a bug. Treat them like the output of a render function: a description that's compared against the previous description and either applied or discarded.

The second is that the view node is the durable identity. Nodes are keyed by *identity* (structural position, or an explicit `.id(_:)` value), not by how many times you've constructed the struct. A node is born when a new identity appears in the tree and dies when that identity is removed. Things you think of as "view state" (the `@State`, an active `.task`, observation tracking) all live on the node, not the struct.

The third is that body re-evaluation is not the same as a redraw. SwiftUI's `AttributeGraph` tracks dependencies (`@State`, `@Observable` keypaths, environment keys). When something invalidates, SwiftUI re-evaluates `body` for affected nodes, diffs the result against what it had, and commits one Core Animation transaction per render loop. Many `body` evaluations can collapse into a single redraw.

If there's one thing to internalize: `init` of the struct happens constantly; lifecycle hooks fire on the node. Use `.task` or `.onAppear` for "the view appeared" work. Don't use `init`.

## `init` is not `viewDidLoad`

This is the single biggest UIKit habit to unlearn. In UIKit, `viewDidLoad` ran once per view controller instance, and a view controller was a long-lived object. Code in `viewDidLoad` was a reasonable place to put one-time setup. In SwiftUI, the View struct's `init` is none of those things.

### Why `init` runs so often

The update loop is roughly this. When something in the parent invalidates, the parent's `body` re-evaluates. The body constructs a fresh value of every child view it returns, which means a literal `Child(...)` initializer call for every child. SwiftUI then compares the new child struct against the previous one (bitwise for plain-old-data types, via `Equatable` if you've conformed, and via reflection otherwise). If the result is "different" or "non-comparable", the child's `body` runs. If "same", SwiftUI short-circuits.

So: every time a parent is dirty, every direct child's `init` fires. For a stable list of 200 rows where one row's data changed, all 200 row inits fire even though only one body re-evaluates.

### What never to put in `init`

```swift
// BAD — every parent re-render allocates a network call.
struct ProfileView: View {
    let userID: String
    @State private var user: User?

    init(userID: String) {
        self.userID = userID
        Task { user = await api.fetchUser(userID) }   // Leaks unstructured tasks.
    }

    var body: some View { /* ... */ }
}
```

```swift
// BAD — print floods the console.
struct Row: View {
    let item: Item
    init(item: Item) {
        self.item = item
        print("Row created: \(item.id)")   // Fires 200 times per re-render.
    }
    var body: some View { Text(item.title) }
}
```

```swift
// BAD — ExpensiveModel() is allocated on every parent re-render.
struct DetailView: View {
    @State private var vm = ExpensiveModel()   // The default expression runs every init.
    var body: some View { /* ... */ }
}
```

The `@State` case is the subtle one. The default expression on a `@State` property (`= ExpensiveModel()`) runs every time the struct is created. SwiftUI only *retains* the first instance per node, but the other 199 are constructed, briefly used, and thrown away. CPU still burns, the allocator still churns.

### What `init` can safely do

Plain stored-property assignment from parameters is fine. That's just struct construction:

```swift
struct Row: View {
    let title: String
    let isHighlighted: Bool

    init(title: String, isHighlighted: Bool) {
        self.title = title
        self.isHighlighted = isHighlighted
    }

    var body: some View { /* ... */ }
}
```

Seeding a `@State` value with the underscored initializer is also fine, but with a caveat:

```swift
struct CounterView: View {
    @State private var count: Int

    init(startAt: Int) {
        _count = State(initialValue: startAt)
    }

    var body: some View { Text("\(count)") }
}
```

The underscore form runs the same way as the default expression. SwiftUI only honors the value on the first time it creates the node. If `startAt` changes on a later render, `count` will not update; SwiftUI ignores the new initial value and keeps the cached state. If you actually need to react to parameter changes, use `.task(id: startAt)` or `.onChange(of: startAt)`.

### When you need an expensive default

If you genuinely need an expensive instance, but only on first node init, hoist the construction behind a `.task` and keep the property optional:

```swift
struct DetailView: View {
    @State private var vm: ExpensiveModel? = nil

    var body: some View {
        Group {
            if let vm { Inner(vm: vm) }
            else { ProgressView() }
        }
        .task {
            if vm == nil { vm = ExpensiveModel() }
        }
    }
}
```

Or, often better, push the model up to a parent that owns it for the navigation, sheet, or tab's lifetime, and pass it down as a plain `let` or a `@Bindable`.

## Body re-evaluation triggers

SwiftUI re-evaluates `body` when it considers the node potentially dirty. There are a handful of specific triggers.

The first is that a `DynamicProperty` you read has changed. That's `@State`, `@SceneStorage`, `@AppStorage`, `@FocusState`, `@GestureState`, `@FocusedValue`, `@Environment(\.key)`, `@Binding`, and the legacy `@ObservedObject`, `@StateObject`, `@EnvironmentObject` wrappers.

The second is that an `@Observable` keypath you read inside `body` has changed. The Observation framework installs a tracking scope while `body` runs. Each `self[keyPath:]` read calls `ObservationRegistrar.access(self, keyPath:)`. When `withMutation(of: keyPath:)` later fires, only views whose tracking scope registered for that exact keypath on that exact instance are invalidated.

That precision is real and worth leaning into. A view that reads only `user.username` won't re-render when `user.preferences` changes, even though both live on the same observable:

```swift
@Observable final class UserModel {
    var username = ""
    var preferences = Preferences()
}

struct NameLabel: View {
    let user: UserModel
    var body: some View {
        Text(user.username)   // Tracks username. Doesn't invalidate on preferences.
    }
}
```

Reads outside of `body` aren't tracked. Constructing a `UIHostingController(rootView:)` and reading observable properties at construction time doesn't establish observation.

The third trigger is that a stored property on the View struct changed. When the parent re-emits a child, SwiftUI compares the new struct against the previous. POD structs use bitwise reflection. Complex ones use `Equatable` if conformed, otherwise reflection. Computed properties are ignored. Closures always count as "different" because closure identity isn't stable.

The fourth is cascading: the parent re-evaluated, re-emitted the child, the child's struct didn't compare equal, so the child's body runs.

The fifth is an identity change. An explicit `.id(value)` change, or a structural identity change (different branch of `if`/`switch`, different position in the tree) isn't a re-evaluation. SwiftUI throws the old node away and creates a fresh one. Fresh `@State`, fresh `.task`, `onAppear` fires.

A few things don't trigger body. Mutating an `@Observable` keypath that the view didn't read. Mutating `@State` to an equal value (SwiftUI short-circuits Equatable equal writes). Property assignment on a plain (non-observable) class held by `let`. Side-effectful work inside `body` (which you shouldn't be doing anyway, because it'll just keep firing).

## View identity: structural and explicit

Every view has an identity. SwiftUI assigns it one of two ways.

### Structural identity

This is the default. Identity comes from the view's type and its position in the static tree.

`VStack { Text("A"); Text("B") }` gives each text a stable identity from its position. `if cond { A() } else { B() }` produces a `_ConditionalContent<A, B>` whose two branches are *different identities*. Switching branches tears down the old node (losing its `@State`) and creates a fresh one. `if cond { A() }` with no else is `_ConditionalContent<A, EmptyView>`. Showing or hiding destroys and recreates `A`.

`AnyView` erases structural identity. Every re-emission can look like a fresh view to SwiftUI, which kills optimizations and resets state. Avoid `AnyView` unless you genuinely have heterogeneous child types and there's no other option.

### The `if cond { X() } else { X() }` trap

This is a common pattern that does the wrong thing:

```swift
// BAD — two distinct structural identities. @State in X() is destroyed on flip.
if isEnabled {
    ComplexEditor()
} else {
    ComplexEditor().disabled(true)
}
```

The two branches of an `if/else` are different identities even if they look like the same view. Flipping `isEnabled` tears down the first `ComplexEditor` and creates a fresh second one, which means the editor's local state is gone.

The fix is to keep one identity and vary configuration with a modifier whose effect is conditional:

```swift
// GOOD — single structural identity. The modifier is inert when isEnabled is true.
ComplexEditor().disabled(!isEnabled)
```

The same pattern applies to `.opacity()`, `.padding()`, `.allowsHitTesting()`, `.transition()`, conditional `.task(id:)`, and any other modifier whose effect can be inert when a condition holds.

### Explicit identity

You supply a `Hashable` or `Identifiable` ID, and SwiftUI uses it directly.

`.id(value)` binds the view's identity to that value. Changing the value destroys and recreates the entire node, including `@State` and any in-flight `.task`. That's the documented way to force a reset.

`ForEach(items)` where `items` is a collection of `Identifiable`, or `ForEach(items, id: \.someKeyPath)`, uses the ID as identity for each child. Inserting, reordering, and removing all map to insert/move/remove operations, and state is preserved across reorders as long as the ID is stable.

```swift
// Force re-creation of the view when the document changes.
DocumentView(doc: currentDoc)
    .id(currentDoc.id)
```

```swift
// BAD — array index breaks reorder and remove.
ForEach(0..<items.count, id: \.self) { i in Row(item: items[i]) }

// GOOD — stable identity survives reorder.
ForEach(items) { item in Row(item: item) }   // items: [Identifiable]
```

Don't use array index as an `id:` unless the array is genuinely append-only and immutable. Otherwise reordering recycles `@State` onto the wrong rows.

### What happens on identity change

When identity changes (an explicit `.id(_:)` change, a branch swap, an `AnyView` re-emission), SwiftUI tears down the old node and allocates a new one. On teardown it cancels the node's `.task`, fires `.onDisappear`, and releases owned `@State` instances (running `deinit` on them if no one else retains). On allocation it runs `init`, sets up fresh `@State`, runs `body`, fires `.onAppear`, and starts new `.task`s.

This is a feature, not a bug. `.id(_:)` is how you tell SwiftUI "treat this as a brand new screen, start fresh".

## `.task` vs `.task(id:)` vs `.onAppear` vs `.onChange` vs `.onReceive`

This is the decision matrix worth memorizing.

| What you need | What to reach for | Auto-cancels? | Notes |
|---|---|---|---|
| Async work tied to view lifetime | `.task { }` | Yes | Preferred default. Inherits the view's `@MainActor`. Cancels on `onDisappear`. |
| Async work that should re-run when a value changes | `.task(id: value) { }` | Yes — cancels and restarts on change | `value` must be `Equatable`. |
| One-shot work on first insertion (rare) | `.onAppear { }` | No | Not "visible". Fires on layout insertion. No auto-cancel. |
| Imperative reaction to a value change | `.onChange(of: value, initial: false) { old, new in }` | n/a | iOS 17+ signature. |
| Bridge a Combine publisher | `.onReceive(publisher) { value in }` | Tears down with the node | Legacy. Prefer `.task` + `AsyncSequence` for new code. |
| Cleanup paired with `onAppear` | `.onDisappear { }` | n/a | Sync only. Fires when the node leaves layout. |

### `.task { }` is the modern default

```swift
struct FeedView: View {
    @State private var posts: [Post] = []

    var body: some View {
        List(posts) { PostRow(post: $0) }
            .task {
                do {
                    for try await batch in api.streamPosts() {
                        try Task.checkCancellation()
                        posts.append(contentsOf: batch)
                    }
                } catch is CancellationError {
                    // Normal on disappear.
                } catch {
                    // Surface the error.
                }
            }
    }
}
```

The properties to know: it fires at the same moment as `onAppear` but starts a structured async task; it's automatically cancelled when the node leaves the layout tree; it inherits the view's `@MainActor` isolation. Cancellation is cooperative, so your code has to check for it.

### `.task(id:)` restarts when input changes

```swift
struct SearchView: View {
    @State private var query = ""
    @State private var results: [Item] = []

    var body: some View {
        VStack {
            TextField("Search", text: $query)
            List(results) { ItemRow(item: $0) }
        }
        .task(id: query) {
            do {
                try await Task.sleep(for: .milliseconds(300))   // Debounce.
                results = try await api.search(query)
            } catch is CancellationError {
                // The user typed more — that's fine.
            } catch {
                results = []
            }
        }
    }
}
```

On every render, SwiftUI compares the new id to the previous one. If they differ, it cancels the in-flight task (cooperatively) and starts a fresh one. The closure captures the new id at restart.

### `.onAppear { }` has narrow uses

```swift
// OK — synchronous one-shot setup.
.onAppear {
    AnalyticsClient.log(.viewedProfile(id: userID))
}
```

```swift
// BAD — in new code, use .task instead.
.onAppear {
    Task { await loadData() }
}
```

The unstructured `Task { }` inside `.onAppear` has no auto-cancellation, no main-actor inheritance guarantee, and leaks if the view disappears while it's still running. For async work tied to the view's lifetime, `.task` is the right hook. The honest exception is sync one-shot work like an analytics log, which is still fine in `.onAppear`.

### `.onChange(of:initial:_:)` is the current signature

```swift
// Two-param closure with old and new values.
.onChange(of: query) { oldValue, newValue in
    analytics.queryChanged(from: oldValue, to: newValue)
}

// Zero-param closure — read the state directly.
.onChange(of: query) {
    debounceTimer.reschedule()
}

// initial: true fires once on first appear and on every change.
.onChange(of: filter, initial: true) {
    applyFilter()
}
```

The deprecated single-param `.onChange(of:perform:)` is gone in new code; flag it on sight when you see it. The `initial: true` variant is useful for avoiding code duplication between `.onAppear` and `.onChange`.

### `.onReceive(publisher) { }` for legacy bridges

```swift
.onReceive(NotificationCenter.default.publisher(for: UIApplication.didEnterBackgroundNotification)) { _ in
    save()
}
```

Still useful for Combine bridges, NotificationCenter, and KVO publishers. For new SwiftUI code, prefer the async sequence form:

```swift
.task {
    for await _ in NotificationCenter.default.notifications(named: .didEnterBackground) {
        save()
    }
}
```

There's a subtle difference worth knowing. `.task` cancels when the view disappears. In a `TabView`, `List`, or `LazyVStack`, the view's `onDisappear` can fire while the node still exists, which means `.task`-based listeners stop. `.onReceive` keeps subscribing for the node's full lifetime. If you need a listener that lives for the lifetime of the screen even when scrolled off, `.onReceive` or hoisting the subscription up to a parent that stays mounted is the way.

## `.task` auto-cancellation is cooperative

SwiftUI sends a cancellation signal when the view disappears or when `.task(id:)`'s id changes. Your code has to honor it. The cancellation isn't pre-emptive; it sets a flag that your code is responsible for checking.

```swift
// BAD — ignores cancellation, will run forever.
.task {
    while true {
        try? await Task.sleep(for: .seconds(1))
        tick += 1
    }
}
```

```swift
// GOOD — periodic Task.isCancelled check.
.task {
    while !Task.isCancelled {
        try? await Task.sleep(for: .seconds(1))
        tick += 1
    }
}
```

```swift
// GOOD — async APIs that throw CancellationError.
.task {
    do {
        for try await event in eventStream {
            try Task.checkCancellation()
            handle(event)
        }
    } catch is CancellationError {
        // Expected on disappear.
    } catch {
        log.error("\(error)")
    }
}
```

Most modern Apple async APIs (`URLSession`, `Task.sleep(for:)`, the `AsyncSequence` notification APIs) honor cancellation automatically. Your own loops need explicit checks.

I'm not certain of the exact timing of when `.task` cancellation runs relative to `.onDisappear` across iOS versions, and the behavior inside `TabView` and lazy containers has historically been version-sensitive. If your code depends on precise ordering between cancellation and cleanup, verify it on the iOS versions you support rather than assuming consistency.

### `defer { }` inside `.task` for cleanup

Pairing cleanup with `.onDisappear` works most of the time, but it relies on `.onDisappear` firing reliably. The more bulletproof shape is to put cleanup inside the `.task` body with `defer`:

```swift
.task {
    let subscription = await stream.subscribe()
    defer { Task { await subscription.cancel() } }

    for await event in subscription.events {
        handle(event)
    }
}
```

The `defer` runs in the same async context as the work, so there's no race against the node being torn down on a different thread. It also works correctly in `TabView` and lazy containers where `onDisappear` timing has been historically inconsistent.

## `TabView` per-tab quirks

Tab lifecycle is the most version-sensitive area in SwiftUI lifecycle. Apple's documentation is sparse here, and the behavior has shifted across iOS releases. I don't have a confident version-by-version chart; verify any specific claim against the iOS versions you target.

What's generally true on recent iOS: `TabView` builds a tab only on first navigation to it. The tab's node persists across tab switches. `onAppear` and `onDisappear` fire on visibility changes. `.task` typically cancels when the tab becomes inactive, which means background work inside `.task` stops when the user switches tabs.

What's worth not relying on is precise `.onDisappear` timing inside `TabView`. Different iOS versions have behaved differently here. The robust pattern is to put cleanup in `defer` inside `.task`:

```swift
// FRAGILE — onDisappear timing varies across versions inside TabView.
.onDisappear { saveDraft() }
```

```swift
// ROBUST — defer in .task body runs on cancellation regardless of timing details.
.task {
    defer { saveDraft() }
    for await edit in editStream { /* ... */ }
}
```

If you need a listener to persist across tab switches (push notifications, background sync, audio playback), lift it up to a parent that stays mounted (the App itself, or a parent View that holds the tab selection). Don't put it inside a tab.

## State ownership (brief)

Full details are in `state-and-observation.md`. The short version of the matrix:

| Situation | What to use |
|---|---|
| Local value (int/string/struct) | `@State` |
| Local `@Observable` model that the view owns | `@State` |
| Receiving an `@Observable` for read-only use | `let foo: Foo` (no wrapper) |
| Receiving an `@Observable` plus needing `$foo.bar` bindings | `@Bindable var foo: Foo` |
| Passing a value-type binding | `@Binding var x: Int` |
| App-wide shared `@Observable` | `.environment(model)` plus `@Environment(Foo.self)` |
| Legacy `ObservableObject` you can't drop yet | `@StateObject` / `@ObservedObject` |

Three specifics that come up often:

`@StateObject` is for `ObservableObject` and isn't the right wrapper for new `@Observable` types. Use `@State` to own an `@Observable` instance. The semantics are the same; no extra wrapper needed.

`@Bindable` doesn't establish observation. The `@Observable` macro does that on any property read inside `body`. `@Bindable` is purely a binding factory; it gives you `$model.property` syntax.

A plain `let model: MyObservable` is still observed when read in `body`. The macro doesn't require a wrapper at the property declaration level. The tracking scope is `body`, not the property.

See `state-and-observation.md` for the keypath-precision deep dive, the `@AppStorage` trap, and the singleton injection pattern.

## App lifecycle

### `@main App` and Scenes

```swift
@main
struct MyApp: App {
    @State private var store = AppStore()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environment(store)
        }
        .onChange(of: scenePhase) { _, phase in
            if phase == .background { store.persist() }
        }
    }
}
```

The `App` protocol's `body` is a `Scene`, not a `View`. The scenes available:

- `WindowGroup` — main app windows on iOS, iPadOS, macOS, and visionOS.
- `Window` — a single-instance window on macOS or visionOS.
- `Settings` — the macOS preferences pane.
- `MenuBarExtra` — a macOS menu bar item.
- `DocumentGroup` — document-based apps.
- `UtilityWindow` — auxiliary palette or tool window on macOS.
- `WKNotificationScene` — rich notifications on watchOS.
- `ImmersiveSpace` — visionOS spatial scenes.

### `ScenePhase`

Three values, reached through `@Environment(\.scenePhase)`:

| Phase | What it means | Typical work |
|---|---|---|
| `.active` | Foreground, receiving events | Resume timers, refresh data. |
| `.inactive` | Foreground but not interactive (transitions, Control Center pulled down, an incoming call) | Pause animations, hide sensitive UI. |
| `.background` | Not visible | Persist state, flush caches. The system may terminate without further warning. |

`.background` is your last reliable save point. SwiftUI doesn't expose a `willTerminate` notification. The system may suspend and then kill the app any time after `.background` fires, and you don't get another chance.

Scoping rules: inside a `Scene`, `scenePhase` reports that scene's phase. Inside the `App`, it aggregates across scenes. Inside a `View`, it reports the enclosing scene's phase. On iOS with a single scene, all three are equivalent.

### `UIApplicationDelegateAdaptor` for AppDelegate hooks

Some things SwiftUI doesn't expose well, and you fall back to an AppDelegate. Push notification registration, background fetch via `BGTaskScheduler`, third-party SDKs (Firebase, some analytics) that demand an AppDelegate, and the older lifecycle hooks like `willTerminate` and `willResignActive`.

```swift
@main
struct MyApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        WindowGroup { RootView() }
    }
}

final class AppDelegate: NSObject, UIApplicationDelegate {
    func application(_ app: UIApplication,
                     didFinishLaunchingWithOptions options: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        UNUserNotificationCenter.current().delegate = /* ... */
        return true
    }
}
```

### Deep links

`.onOpenURL` and `.onContinueUserActivity` are the SwiftUI entry points for URL handling.

```swift
WindowGroup {
    RootView()
        .onOpenURL { url in
            router.handle(url)
        }
        .onContinueUserActivity("com.example.viewArticle") { activity in
            if let url = activity.webpageURL {
                router.openArticle(url: url)
            }
        }
}
```

`.onOpenURL { url in }` fires for custom URL schemes and Universal Links when those are configured. `.onContinueUserActivity(_:perform:)` covers Handoff, Spotlight, Siri Shortcuts, persistent `NSUserActivity`, and Universal Links from browsing.

Both deliver on the main actor. Place the handler at the navigation root, not deep in leaf views. The leaf may not be mounted when the URL arrives, and the navigation has to happen from the root anyway. Multiple `.onOpenURL` handlers in the tree all fire; design each one to handle only what it understands.

If a project still has a `UISceneDelegate` and you've added a SwiftUI lifecycle, you'll see a runtime warning saying you can't use Scene methods for URL, NSUserActivity, and other external events without SwiftUI lifecycle. Drop the scene delegate or move its responsibilities into SwiftUI.

## Sheet, popover, fullScreenCover, inspector lifecycle

```swift
struct ContentView: View {
    @State private var showSheet = false

    var body: some View {
        Button("Open") { showSheet = true }
            .sheet(isPresented: $showSheet, onDismiss: { print("dismissed") }) {
                SheetContent()
            }
    }
}
```

The lifecycle, in order: the presented view's content closure is referenced when the modifier is added but not invoked. When the trigger flips to `true`, the closure runs, the presented view's node is created, `init` runs, `body` runs, `.onAppear` fires, `.task` starts. On dismiss, the node is torn down: `.task` cancels, `.onDisappear` fires, owned `@State` releases. `onDismiss:` runs after the dismissal animation completes.

A couple of practical notes. For cleanup, prefer `defer` inside `.task` rather than `.onDisappear`. Sheets have had retain-cycle bugs in past iOS versions where `.onDisappear` didn't fire cleanly. `defer` inside `.task` doesn't depend on `.onDisappear` firing.

For self-dismissal, use `@Environment(\.dismiss)` inside the presented view, not in the parent:

```swift
struct SheetContent: View {
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack {
            Text("Hello")
            Button("Close") { dismiss() }
        }
    }
}
```

I've seen retain-cycle reports for sheets and `fullScreenCover` on iOS 17.x where the content didn't deinit on dismiss. Most cases were fixed by iOS 26, but if you depend on teardown timing for memory reasons, verify in your build with a `deinit { print(...) }` on the model class.

## Lazy view children and the scroll trap

### `LazyVStack`, `LazyHStack`, `LazyVGrid`, `LazyHGrid`

Children are constructed when they're about to enter the viewport, not before. Once constructed, the node is retained for the lifetime of the lazy container. They do not recycle.

Two consequences. First, memory grows monotonically. Scroll through 10,000 rows with images and you've allocated 10,000 nodes. If you need that scale, cap content, paginate, or use `List`. Second, `.onAppear` fires repeatedly per scroll. The same node fires `.onAppear` every time it re-enters the viewport. Don't do "first appear" work in `.onAppear` inside a lazy container without an `if items.isEmpty` guard.

`LazyVStack` historically only fires `onDisappear` on the lower side (per Apple's layout estimate). `List` fires on both sides. I haven't verified this on every iOS 26 build, so if you're relying on `onDisappear` timing inside a lazy container, test it.

```swift
// BAD — fires the fetch on every scroll-back.
LazyVStack {
    ForEach(items) { item in
        ItemRow(item: item)
            .onAppear { Task { await prefetch(item) } }
    }
}

// GOOD — debounced via .task(id:), cancels on scroll-out.
LazyVStack {
    ForEach(items) { item in
        ItemRow(item: item)
            .task(id: item.id) {
                await prefetch(item)
            }
    }
}
```

### `List`

`List` recycles offscreen rows the way UIKit's `UITableViewCell` does. That means `@State` inside a `List` row isn't reliable storage for long-term state. Scroll far enough and the node is destroyed and recreated, resetting `@State`. Lift important per-row state into a parent model or use a per-row `@Observable`.

### `ScrollView` (non-lazy)

```swift
ScrollView { VStack { ForEach(items) { Row(item: $0) } } }
```

Constructs all children up front and fires `.onAppear` for all of them at first layout. For more than 50 items, switch to `LazyVStack` or `List`.

### Choosing between `LazyVStack` and `List`

| What you need | What to use |
|---|---|
| Built-in cell separators, swipe actions, refreshable | `List` |
| 10,000+ rows where memory matters | `List` (recycles) |
| Custom layout with no cell chrome | `LazyVStack` |
| Static or short content (under 50 rows) | Plain `VStack` |

## `NavigationStack` lifecycle

Pushing a destination creates a fresh node: `init` runs, `body` runs, `.onAppear` fires, `.task` starts. Popping destroys it: `.task` cancels, `.onDisappear` fires, `@State`-owned models release.

```swift
NavigationStack(path: $router.path) {
    HomeView()
        .navigationDestination(for: Route.self) { route in
            switch route {
            case .profile(let id): ProfileView(id: id)
            case .settings: SettingsView()
            }
        }
}
```

Pushing the same route value while it's already on the stack doesn't reuse the node; it stacks a new instance. If you want to force a deliberate reset on a destination, use `.id(routeValue)` on the destination view. See `navigation.md` for the router pattern.

## Memory and deallocation

### When does a node get torn down?

A node is torn down when its parent stops emitting it (branch swap, `ForEach` element removed, `.id(_:)` changed), when a lazy container removes it from its tracked range (only `List` recycles; `LazyVStack` retains), when the containing sheet, popover, cover, or inspector is dismissed, when the `NavigationStack` pops it, or when the enclosing `Scene` is destroyed (window closed, scene reclaimed).

On teardown, `.onDisappear` fires synchronously, `.task` tasks are cancelled (cooperatively), `.onReceive` subscriptions tear down, and `@State`-owned `@Observable` instances release. Their `deinit` runs if nothing else retains them.

### The escaping-closure leak

The most common SwiftUI memory leak shape:

```swift
final class Model {
    var pending: Task<Void, Never>?

    func start() {
        pending = Task { [weak self] in
            await self?.work()
        }
    }
}

struct ContentView: View {
    @State private var model = Model()

    var body: some View {
        Text("…")
            .onAppear { model.start() }   // Unstructured Task. Leaks if the view disappears mid-flight.
    }
}
```

`model.start()` spawns an unstructured `Task` that outlives the view. If the view disappears, the task keeps running. The fix is `.task`:

```swift
.task { await model.work() }   // Auto-cancelled, structured.
```

### Hold `@Observable` instances in `@State`

```swift
// GOOD — @State owns the instance; deinit fires on node teardown.
@State private var vm = ChatViewModel()
```

```swift
// BAD — plain stored property. Lifetime is the struct's, not the node's.
var vm = ChatViewModel()   // Re-instantiated on every parent re-render.
```

### Struct vs node, one more time

The View struct is a temporary description. It's created, compared, and thrown away on every render. The view node (the AttributeGraph entry that holds `@State` and observation registrations) has a much longer lifetime.

So `deinit` of the struct does not mean the view is gone. `init` of the struct does not mean the view is new. Trying to `print("created")` in struct `init` and reasoning about it as `viewDidLoad` is a category error; you'll see hundreds of prints, and the prints are not telling you what you think they are.

If you need to know when an `@Observable` model actually deallocates, don't trust `init` or `deinit` logging on the View struct. The default expression on `@State` runs on every struct creation, even though only the first instance is retained. Use Instruments → Allocations or Leaks to see real instance lifetimes.

## Debugging recipes

### `Self._printChanges()` inside body

```swift
var body: some View {
    let _ = Self._printChanges()
    // …
}
```

The console output names the dependency that changed: `@self` means the whole view value (parent emitted a different struct), `@identity` means identity changed (node recycled), and property names like `_count` or `_model` name a specific dependency that mutated.

This is underscored API. Don't ship it. Guard it with `#if DEBUG`:

```swift
#if DEBUG
let _ = Self._printChanges()
#endif
```

### `Self._logChanges()` for OSLog

Same behavior as `_printChanges`, but emits to `os_log` under `com.apple.SwiftUI` and category `Changed Body Properties`. Filter for it in Xcode's debug console. Still underscored, so strip before submission.

### Random background color to visualize re-renders

```swift
.background(Color(hue: .random(in: 0...1), saturation: 0.5, brightness: 1))
```

Every time `body` runs, the color changes. It's a quick visual signal for "is this re-rendering when I move my finger over there?". It doesn't distinguish body re-evaluation from actual redraw, but the visual is immediate and obvious.

### Instruments SwiftUI template

Xcode 26's Instruments has a SwiftUI template with Update Groups (a timeline of when SwiftUI was working), Long View Body Updates (body evaluations exceeding the frame budget, colored orange or red), Long Representable Updates (slow `UIViewRepresentable` or `NSViewRepresentable` updates), and the Cause & Effect Graph from WWDC25 Session 306.

The Cause & Effect Graph visualizes which `@Observable` keypath, `@Environment` change, or `@State` mutation caused which view's invalidation. It's the answer to "why did *this* view re-render?". Run it on device; the simulator is misleading for performance work. Profile → SwiftUI template.

### Decompose to find the dependency

The strongest diagnostic, and often the fix, is to split a large `body` into smaller subviews. Each subview becomes a leaf in the AttributeGraph with its own dependency set. A re-render that previously fired the entire screen now fires only the leaf with the actual changed dependency.

```swift
// BAD — every body re-eval recomputes everything.
var body: some View {
    VStack {
        // 200 lines of mixed-dependency content.
    }
}

// GOOD — leaves invalidate independently.
var body: some View {
    VStack {
        HeaderView(user: user)
        FeedView(posts: posts)
        ComposerView()
    }
}
```

Each subview reads only the data it displays. Changes to `user` don't invalidate `FeedView`; changes to `posts` don't invalidate `HeaderView`.

### Equatable views

For pure leaf views driven by a struct, conforming the view to `Equatable` and wrapping with `.equatable()` (or `EquatableView`) makes SwiftUI use your `==` for diffing. Use this sparingly. An incorrect `==` hides updates instead of fixing them, which is harder to debug than the original re-render problem.

### Stateless leaves

A view should depend only on what it actually displays. Pass minimal data (a `String`, a `URL`, an `Int`) instead of the whole model. The view becomes a pure function of its inputs, which is trivially testable and previewable. Push state up to where it logically belongs (the feature root); leave leaves stateless.

## Common gotchas

A short list of the things that show up over and over.

`init` runs on every parent re-render. Don't put work there. SwiftUI re-creates the struct constantly. Use `.task` (preferred) or `.onAppear` (for sync one-shot work).

`@State` default expressions evaluate constantly; only the first instance is retained. `@State private var vm = ExpensiveModel()` calls `ExpensiveModel()` on every parent render, and only the first is kept. Hoist expensive defaults, or gate them behind a `.task`.

`.onAppear` is not the same as "visible". In a `ZStack` and non-lazy stacks, every child fires `.onAppear` regardless of being on-screen. In `List` and `LazyVStack`, `.onAppear` fires repeatedly as the node enters and re-enters the viewport.

In lazy containers, `.onAppear` fires on every scroll-in. Don't use it for a one-shot fetch inside `LazyVStack` or `List` without an `items.isEmpty` guard. Prefer `.task(id:)`.

`if cond { X() } else { X() }` creates two identities. All `@State` in `X` is destroyed when the condition flips. Use `X().disabled(!cond)` (or whatever inert modifier fits) to keep a single identity.

`.task` auto-cancels. `.onAppear { Task { } }` does not. Never `.onAppear { Task { } }` in new code.

`@State` ignores its initializer after the first time the node is created. Passing a parameter to `@State`'s default value only seeds it on the first render of that node. Subsequent parent changes don't propagate; use `@Binding`, an `@Observable` in `@State`, or `.task(id:)`.

`@StateObject` is legacy for `@Observable` types. For new `@Observable` code, use `@State`.

`.onChange(of:perform:)` (the single-parameter closure) is deprecated. Always use `.onChange(of:initial:_:)` with the zero-or-two-parameter closure.

`TabView`'s per-tab `onDisappear` timing has been version-sensitive. Don't rely on it for cleanup; use `defer` inside `.task`. And `.task` cancels when a tab becomes inactive, so persistent listeners (background sync, audio) must live above the tab.

## Anti-patterns to flag

```swift
// BAD — using init as viewDidLoad.
struct Row: View {
    init(/* ... */) { print("created") }   // Fires hundreds of times.
    var body: some View { /* ... */ }
}
```

```swift
// BAD — async work in init.
init(/* ... */) {
    Task { await fetch() }   // Leaks per struct re-creation.
}
```

```swift
// BAD — .onAppear { Task { } } in new code.
.onAppear { Task { await loadData() } }   // Use .task.
```

```swift
// BAD — deprecated single-param onChange.
.onChange(of: query) { newValue in       // Pre-iOS-17.
    refresh(with: newValue)
}
```

```swift
// BAD — @StateObject for a new @Observable type.
@StateObject private var vm = MyObservableViewModel()   // Use @State.
```

```swift
// BAD — if/else with the same view, expecting state to persist.
if isEditing { ProfileEditor() } else { ProfileEditor() }
```

```swift
// BAD — Binding(get:set:) in body.
TextField("", text: Binding(
    get: { self.query },
    set: { self.query = $0; self.search() }
))
// GOOD — @State + .onChange.
TextField("", text: $query)
    .onChange(of: query) { _, new in search(new) }
```

```swift
// BAD — stored @ViewBuilder closure escaping the view scope.
final class Coordinator {
    var makeRow: () -> AnyView   // Erases identity, kills updates.
}
```

```swift
// BAD — array index as ForEach id.
ForEach(0..<items.count, id: \.self) { i in Row(items[i]) }
// GOOD — stable identity.
ForEach(items) { Row($0) }
```

```swift
// BAD — AnyView wrapper when you don't need it.
func makeView() -> AnyView { AnyView(Text("…")) }
// GOOD — opaque return plus @ViewBuilder.
@ViewBuilder func makeView() -> some View { Text("…") }
```

```swift
// BAD — relying on .onDisappear inside TabView.
.onDisappear { saveDraft() }   // Timing varies.
// GOOD — defer in .task body.
.task {
    defer { saveDraft() }
    for await edit in editStream { /* ... */ }
}
```

```swift
// BAD — heavy work inside body.
var body: some View {
    let sorted = items.sorted { /* ... */ }   // Re-sorts every body call.
    let filtered = sorted.filter { /* ... */ }
    return List(filtered) { /* ... */ }
}
// GOOD — derive in the model.
var body: some View { List(model.displayItems) { /* ... */ } }
```

## TL;DR

View structs are descriptions. View nodes own state and lifecycle.

Don't put work in `init`. Use `.task` for async work tied to view lifetime.

`.task` auto-cancels cooperatively. `.onAppear { Task { } }` doesn't, and shouldn't appear in new code.

`.onAppear` is not "visible". It fires on layout insertion, and in lazy containers it fires repeatedly.

`if cond { X() } else { X() }` is two identities. State dies on flip. Keep one identity with an inert modifier like `.disabled(!cond)`.

`@Observable` tracking is keypath-precise. Views only invalidate on properties they actually read in `body`.

`.background` is your last save point. There's no `willTerminate` in SwiftUI.

The single most useful debugging technique is to decompose a big `body` into small subviews. Leaves invalidate independently and the AttributeGraph does precise work.
