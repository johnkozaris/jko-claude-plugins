# Navigation

Target: Swift 6.3 / iOS 26 / Xcode 26.

The modern SwiftUI navigation story is `NavigationStack` with a path binding, a typed `Hashable` route enum, and an `@Observable` router that owns the path. This is what Apple has been building toward since iOS 16, and WWDC 2025 didn't change the picture — the model that landed in iOS 16 is the long-term direction.

Cross-references:
- `@Observable`, `@Bindable`, and environment plumbing are in `state-and-observation.md`.
- The App.swift singleton wiring and the broader architecture story are in `architecture.md`.
- Sheet animation and `glassEffectID` morphing are in `animation.md` and `liquid-glass.md`.
- Deep-link source-of-truth lifecycle is in `lifecycle.md`.

## Why `NavigationView` and `NavigationLink(destination:)` are gone

Before iOS 16, navigation in SwiftUI worked through `NavigationView` and `NavigationLink(destination:)`. You wrote the destination view inline at the call site, and tapping the link pushed it onto a stack.

The problem was that this design couldn't support several things the framework needed: a path binding you could drive programmatically (so the router could push routes, not just the user), deep linking (a URL coming in from outside the app), and state restoration across launches (re-creating a stack of views from saved data). The eager `destination:` parameter also meant SwiftUI eagerly built every potential destination view on every render — fine for two links, expensive for a long list.

iOS 16 introduced `NavigationStack(path:)`. The shape is different:

1. You hold a path — an array or a `NavigationPath` — as state somewhere the router can reach.
2. You register a single `navigationDestination(for: SomeType.self)` modifier on the stack root that says "when something of this type lands in the path, render this view."
3. You push by appending to the path. Either you call `path.append(route)` directly, or you use `NavigationLink(value: route)` which appends for you when tapped.

This separates "what's in the stack" (state) from "what gets shown" (rendering). The path is data. The router can mutate it. A deep link can mutate it. Saving and restoring it is straightforward because it's just a value.

`NavigationView` is deprecated. `NavigationLink(destination:)` is the old eager form and doesn't participate in a path. In new code, flag both on sight. They still compile, but they don't compose with anything else worth using.

## The basic shape

```swift
enum Route: Hashable {
    case articleDetail(Article.ID)
    case author(User.ID)
    case settings
}

struct ContentView: View {
    @State private var path: [Route] = []

    var body: some View {
        NavigationStack(path: $path) {
            ArticleListView()
                .navigationDestination(for: Route.self) { route in
                    switch route {
                    case .articleDetail(let id): ArticleDetailView(id: id)
                    case .author(let id):        AuthorProfileView(id: id)
                    case .settings:              SettingsView()
                    }
                }
        }
    }
}

struct ArticleRow: View {
    let article: Article
    var body: some View {
        NavigationLink(value: Route.articleDetail(article.id)) {
            ArticleRowContent(article: article)
        }
    }
}
```

The pieces:

- A `Hashable` route enum. The associated values are the navigation arguments. They must themselves be `Hashable`.
- A path. For a single-route-type stack, `[Route]` gives you nice array API. For mixed types, use `NavigationPath` instead (see below).
- A single `navigationDestination(for:)` modifier at the stack root. The switch maps each case to its view.
- `NavigationLink(value:)` to push. The link does `path.append(value)` when tapped.

There's one rule that catches people: register `navigationDestination(for:)` once per type per stack, at the root. If you register it twice the first one wins silently and the second is ignored. If you register it deep in the tree on a child view, it works in the case you tested but is brittle as the tree changes.

## Typed route enums vs strings

There's a temptation to use strings as route identifiers — `path.append("article/42")` — because it looks simple. Don't.

- Strings have no compile-time exhaustiveness. Adding a new screen doesn't force you to handle it at the destination switch. You only find out at runtime that some link points to nowhere.
- Strings have no type-safe arguments. You parse the id out of the string at the destination, which means parse errors at runtime.
- `Hashable` is synthesized for enums with `Hashable` associated values, so the boilerplate is zero.

The enum form lets the compiler tell you when you've added a route and forgotten to handle it. That's the win.

## Routers and one stack per tab

For anything beyond a single-screen prototype, you want an `@Observable` router that owns the path. The router lives at the app root, gets injected into the environment, and any view can ask it to navigate.

The pattern that's converged across the modern reference apps (IceCubesApp, IcySky, Apple's Backyard Birds sample): one `NavigationStack` per tab, with its own path, and the router holds all the paths. Tabs are independent navigation universes — pushing in one tab shouldn't affect the others.

```swift
@Observable
final class TabRouter {
    var home: [Route] = []
    var search: [Route] = []
    var profile: [Route] = []
    var selectedTab: AppTab = .home

    func push(_ route: Route, in tab: AppTab) {
        switch tab {
        case .home:    home.append(route)
        case .search:  search.append(route)
        case .profile: profile.append(route)
        }
    }

    func popToRoot(_ tab: AppTab) {
        switch tab {
        case .home:    home.removeAll()
        case .search:  search.removeAll()
        case .profile: profile.removeAll()
        }
    }
}

@main
struct MyApp: App {
    @State private var router = TabRouter()
    var body: some Scene {
        WindowGroup {
            RootView()
                .environment(router)
        }
    }
}
```

The root view reads the router out of the environment and binds each tab's stack to the corresponding array:

```swift
struct RootView: View {
    @Environment(TabRouter.self) private var router
    var body: some View {
        @Bindable var router = router
        TabView(selection: $router.selectedTab) {
            Tab("Home", systemImage: "house", value: .home) {
                NavigationStack(path: $router.home) {
                    HomeRoot().routedDestinations()
                }
            }
            Tab("Search", systemImage: "magnifyingglass", value: .search) {
                NavigationStack(path: $router.search) {
                    SearchRoot().routedDestinations()
                }
            }
            Tab("Profile", systemImage: "person", value: .profile) {
                NavigationStack(path: $router.profile) {
                    ProfileRoot().routedDestinations()
                }
            }
        }
    }
}

extension View {
    func routedDestinations() -> some View {
        navigationDestination(for: Route.self) { route in
            switch route {
            case .articleDetail(let id): ArticleDetailView(id: id)
            case .author(let id):        AuthorProfileView(id: id)
            case .settings:              SettingsView()
            }
        }
    }
}
```

The `@Bindable var router = router` line is the standard pattern for getting bindings out of an `@Observable` instance you read from the environment.

Cross-tab navigation: switch the selected tab, then push.

```swift
router.selectedTab = .search
router.push(.articleDetail(id), in: .search)
```

This is the whole point of routing through a shared `@Observable` instance — any view can request navigation without reaching into another tab's view tree.

### A note on third-party navigation packages

Stinsen and SUICoordinator were popular before the `@Observable` + typed route pattern was straightforward. They wrapped UIKit-era coordinator patterns. Now that you can express the same shape natively with an `@Observable` router and a typed enum, they don't add much. Existing code using them works; for new code, the native pattern is the simpler choice.

## `NavigationPath` for mixed types

When a stack pushes multiple unrelated `Hashable` types — `Article`, `User`, `URL`, all in the same stack — use `NavigationPath` instead of `[Route]`:

```swift
@State private var path = NavigationPath()

NavigationStack(path: $path) {
    HomeView()
        .navigationDestination(for: Article.self) { ArticleDetailView(article: $0) }
        .navigationDestination(for: User.self)    { UserProfileView(user: $0) }
        .navigationDestination(for: URL.self)     { WebPageView(url: $0) }
}
```

`NavigationPath` is type-erased but retains the type information internally. You register one `navigationDestination(for:)` per type you'll push.

If you push a type and forget to register a destination, you get a silent empty view rather than a crash. This is one of the more annoying SwiftUI failure modes — log a warning in the destination resolver during debug builds if you can.

For a single-route-type stack, prefer `path: [Route]` over `NavigationPath`. The array gives you ordinary array API (`router.home.removeAll()`, `router.home.last`, `router.home.indices`). `NavigationPath` is more opaque.

## Deep links and state restoration

`NavigationPath` and a typed-array path are both `Codable` if the route type is `Codable`. That's what makes deep linking and restoration work.

A deep link is just "an external URL writes into the router." The pattern:

```swift
struct RootView: View {
    @Environment(TabRouter.self) private var router
    var body: some View {
        @Bindable var router = router
        TabView(selection: $router.selectedTab) { /* ... */ }
            .onOpenURL { url in handle(url) }
    }

    private func handle(_ url: URL) {
        guard let route = Route(deepLink: url) else { return }
        router.selectedTab = .home
        router.home.append(route)
    }
}

extension Route {
    init?(deepLink url: URL) {
        // acme://article/42 → .articleDetail(42)
        guard url.scheme == "acme" else { return nil }
        let comps = url.pathComponents.filter { $0 != "/" }
        switch url.host {
        case "article":
            guard let id = Article.ID(comps.first ?? "") else { return nil }
            self = .articleDetail(id)
        case "settings":
            self = .settings
        default:
            return nil
        }
    }
}
```

For state restoration across launches, conform `Route` to `Codable` and persist the path. Save on `.onChange(of: router.home)` and restore on app launch:

```swift
extension Array where Element == Route {
    var encoded: Data? { try? JSONEncoder().encode(self) }
    init?(_ data: Data?) {
        guard let data,
              let decoded = try? JSONDecoder().decode([Route].self, from: data)
        else { return nil }
        self = decoded
    }
}
```

`NavigationPath` has its own `codable` property that does similar work for type-erased paths. The exact restoration timing — whether you restore in `App.init`, in `.task`, or in a scene phase change — is in `lifecycle.md`.

## Programmatic navigation through the router

Views shouldn't push themselves directly. They ask the router to push. This keeps navigation testable (you can drive the router from a test) and consistent (one place is responsible for stack state).

```swift
struct ArticleRow: View {
    @Environment(TabRouter.self) private var router
    let article: Article

    var body: some View {
        Button {
            router.push(.articleDetail(article.id), in: router.selectedTab)
        } label: {
            ArticleRowContent(article: article)
        }
        .buttonStyle(.plain)
    }
}
```

When the row *is* the link — no other tap target — `NavigationLink(value:)` is the cleaner form because the system handles the gesture, accessibility, and disabled state for you:

```swift
NavigationLink(value: Route.articleDetail(article.id)) {
    ArticleRowContent(article: article)
}
.buttonStyle(.plain)
```

Both forms append to the stack's path. Pick the one that matches the visual model: if the row is interactive in a list-of-links sense, use `NavigationLink`. If the row has multiple tap targets and one of them happens to navigate, use a `Button` calling the router.

## `NavigationSplitView` for iPad and Mac

For sidebar-driven multi-column layouts:

```swift
NavigationSplitView {
    SidebarView(selection: $selection)
} content: {
    ContentListView(selection: selection)
} detail: {
    DetailView(item: selectedItem)
}
```

A two-column variant just omits `content:`. The detail column registers `navigationDestination(for:)` the same way a stack would.

On iPad and Mac, mixing tabs with split views is common: each tab is itself a `NavigationSplitView`, or the app is a single `NavigationSplitView` with a sidebar acting as the top-level selector. Which you choose depends on the app — multi-tab apps tend to keep tabs visible at the bottom and use split views inside them; single-domain apps (a mail client, a documents app) use one split view with sidebar selection driving the second column.

A few notes:

- Use `NavigationSplitViewVisibility` binding when you want programmatic control of which columns are visible (collapsing the sidebar, showing only detail on rotation).
- `.navigationSplitViewStyle(.balanced)` / `.prominentDetail` / `.automatic` control how the columns share width on iPad.
- On iOS 26, the split view automatically applies a background extension effect for the sidebar — you don't need to clip content manually.

## Sheets driven by `Identifiable` enums

For a screen with one sheet, `sheet(isPresented: $bool)` is fine. For a screen with several possible sheets, the pattern that scales is a single `sheet(item:)` driven by an `Identifiable` enum:

```swift
enum SheetType: Identifiable {
    case create
    case edit(Article)
    case share(URL)
    case settings

    var id: String {
        switch self {
        case .create: "create"
        case .edit(let a): "edit-\(a.id)"
        case .share(let u): "share-\(u.absoluteString)"
        case .settings: "settings"
        }
    }
}

struct ContentView: View {
    @State private var sheet: SheetType?

    var body: some View {
        VStack { /* ... */ }
            .sheet(item: $sheet) { sheet in
                switch sheet {
                case .create:        CreateView()
                case .edit(let a):   EditView(article: a)
                case .share(let u):  ShareSheet(url: u)
                case .settings:      SettingsView()
                }
            }
    }
}
```

One modifier. One optional. One switch. Adding a destination is a new enum case and a new switch arm.

The alternative — a separate `@State var showCreate = false`, `@State var showEdit = false`, etc., each driving its own `sheet(isPresented:)` modifier — works for the first sheet and falls apart when you have several. State coordination becomes manual (closing one sheet to open another, restoring after deep link), and you can accidentally open two at once.

For a sheet that takes its data directly, the shorthand works:

```swift
@State private var selectedArticle: Article?

.sheet(item: $selectedArticle) { article in
    ArticleDetailView(article: article)
}

// Or, when the view's init matches:
.sheet(item: $selectedArticle, content: ArticleDetailView.init)
```

A few sheet rules worth knowing:

- The sheet dismisses itself via `@Environment(\.dismiss)`. The parent doesn't reach in to close it.
- On iOS 26, partial-height sheets pick up a Liquid Glass background by default. If you're applying `.presentationBackground(.thinMaterial)` from older code, remove it — it suppresses the new style. See `liquid-glass.md`.
- `.presentationDetents([.medium, .large])` gives a resizable sheet with a grabber.
- `.presentationSizing(.form)` / `.page)` for system-standard sized sheets.

## `fullScreenCover` and `inspector`

`.fullScreenCover` is the same shape as `.sheet` but covers the full screen, doesn't dismiss on swipe-down by default, and is for flows the user shouldn't drift out of (sign-in, onboarding, an active video call). Use it sparingly — it removes the "swipe to dismiss" affordance that users expect from sheets.

`.inspector(isPresented:)` is the trailing-edge supplementary panel for iPad and Mac. Used for "details about the current selection" — a properties panel, a comment thread, a metadata view. Not a sheet (which interrupts), not a sidebar (which holds the primary navigation). It's a third column the user can show or hide.

```swift
.inspector(isPresented: $showInspector) {
    InspectorView(item: selectedItem)
        .inspectorColumnWidth(min: 200, ideal: 300, max: 400)
}
```

## Zoom transitions

`.navigationTransition(.zoom(sourceID:in:))` paired with `.matchedTransitionSource(id:in:)` gives the system "thumbnail expands into detail" animation across stack pushes, sheets, and full-screen covers. This is the same primitive Apple uses for opening an app icon into the app, or expanding a Photos thumbnail.

```swift
struct Gallery: View {
    @Namespace private var zoomNS
    let articles: [Article]

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 100))]) {
                    ForEach(articles) { article in
                        NavigationLink(value: Route.articleDetail(article.id)) {
                            ArticleThumbnail(article: article)
                                .matchedTransitionSource(id: article.id, in: zoomNS)
                        }
                    }
                }
            }
            .navigationDestination(for: Route.self) { route in
                if case .articleDetail(let id) = route {
                    ArticleDetailView(id: id)
                        .navigationTransition(.zoom(sourceID: id, in: zoomNS))
                }
            }
        }
    }
}
```

The id you pass to `sourceID` and the id you pass to `matchedTransitionSource(id:)` need to match. The namespace passed through `in:` ties them together.

This works where `matchedGeometryEffect` cannot — `matchedGeometryEffect` is in-scene, but zoom transitions cross stack pushes and sheet boundaries. For glass-to-glass morphing (a chip expanding into a panel), `glassEffectID(_:in:)` is the better tool; see `liquid-glass.md`.

## Alerts and confirmation dialogs

Use `.alert` for blocking confirmations and informational messages, `.confirmationDialog` for "are you sure" prompts on destructive actions.

```swift
// Information-only alert.
.alert("Saved", isPresented: $showSavedAlert) { }

// Alert with destructive action.
.alert("Delete \(article.title)?", isPresented: $showDeleteAlert) {
    Button("Cancel", role: .cancel) { }
    Button("Delete", role: .destructive) { delete(article) }
} message: {
    Text("This cannot be undone.")
}

// Confirmation dialog from a button.
Button("Delete") { showDeleteConfirm = true }
    .confirmationDialog("Delete this article?", isPresented: $showDeleteConfirm) {
        Button("Delete", role: .destructive) { delete(article) }
    }
```

On iOS 26, confirmation dialogs animate from the triggering view when the source is reachable in the layout. Attaching the confirmation modifier to the button (rather than to the root of the screen) preserves that visual link.

## TabView

Use the iOS 18+ `Tab` API rather than the older `tabItem()` modifier. The new shape is typed:

```swift
@State private var selectedTab: AppTab = .home

TabView(selection: $selectedTab) {
    Tab("Home", systemImage: "house", value: .home) {
        HomeView()
    }
    TabSection("Library") {
        Tab("Songs", systemImage: "music.note", value: .songs) { SongsView() }
        Tab("Albums", systemImage: "square.stack", value: .albums) { AlbumsView() }
    }
    Tab(role: .search) {
        SearchView()
    }
}
.tabViewStyle(.sidebarAdaptable)
```

`.tabViewStyle(.sidebarAdaptable)` is the one-line "tabs on iPhone, sidebar on iPad and Mac" toggle. `TabSection` provides grouped tabs that show as section headers in sidebar mode and are flattened on iPhone.

iOS 26 adds a few features on top:

- `.tabBarMinimizeBehavior(.onScrollDown)` — the floating glass tab bar collapses on downward scroll.
- `.tabViewBottomAccessory { ... }` — a persistent row above the tab bar (the "Now Playing" pattern). Read `\.tabViewBottomAccessoryPlacement` from the environment to switch between compact and expanded layouts.
- `Tab(role: .search) { ... }` — a dedicated system search tab. One per `TabView`. Place it last unless you want the search field expanded on launch.

Don't hand-roll a custom tab bar. Custom tab bars lose compatibility with `.tabBarMinimizeBehavior` and `.tabViewBottomAccessory`, and they don't pick up the Liquid Glass appearance.

## Toolbars

```swift
.toolbar {
    ToolbarItem(placement: .topBarLeading) {
        Button("Edit", action: edit)
    }
    ToolbarSpacer(.fixed, placement: .topBarLeading)
    ToolbarItem(placement: .topBarLeading) {
        Button("Select", action: select)
    }

    ToolbarSpacer(.flexible)

    ToolbarItem(placement: .topBarTrailing) {
        Button("Done", action: done)
    }
}
.toolbarTitleDisplayMode(.inline)
```

A few rules that come up:

- Use semantic placements (`.confirmationAction`, `.cancellationAction`, `.primaryAction`) when the action is one of those roles. They adapt across iOS, iPadOS, and macOS and they keep keyboard semantics right (Return triggers `.confirmationAction`, Escape triggers `.cancellationAction`).
- `ToolbarSpacer(.fixed)` groups related items visually. `ToolbarSpacer(.flexible)` pushes items to opposite edges.
- Two visible actions max in the top bar on iPhone. Overflow goes into a `Menu`.
- Don't put custom backgrounds behind toolbar items — it interferes with the automatic scroll-edge effect and with Liquid Glass.

## Common review findings

When reviewing navigation code, the things to watch for:

- `NavigationView` anywhere. Replace with `NavigationStack` or `NavigationSplitView`.
- `NavigationLink(destination:)` — the eager form. Replace with `NavigationLink(value:)` and a `navigationDestination(for:)` modifier.
- Mixing eager `NavigationLink(destination:)` with `navigationDestination(for:)` in the same hierarchy. They fight over who owns the push.
- `navigationDestination(for:)` registered deep in the tree rather than at the stack root.
- Strings as path identifiers. Replace with a typed enum.
- A single `NavigationStack` wrapping a `TabView`. The stack should be inside each tab, not outside the whole tab view.
- `sheet(isPresented:)` repeated four or five times in one view. Refactor to `sheet(item:)` driven by an `Identifiable` enum.
- `.presentationBackground(.thinMaterial)` on iOS 26 sheets, suppressing the new Liquid Glass background.
- Row views that push their own routes by directly mutating the path. Route through the router.
- Custom hand-built tab bars. Use the `Tab` API.
- `.onOpenURL` ignored when the app has clear deep-link surfaces. Wire it into the router.
- `AnyView` returned from a navigation destination. Use a `@ViewBuilder` switch.
- `.glassEffect()` applied to toolbar content. Glass is for the toolbar surface, not the icons inside it.
