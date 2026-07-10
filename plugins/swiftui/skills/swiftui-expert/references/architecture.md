# App Architecture & Folder Structure

Targets: Swift 6.3 / iOS 26 / Xcode 26. As of 2026-05-17.

Audience: a code reviewer who must decide whether to flag an architecture choice or accept it. This file gives concrete defaults and concrete when/when-not triggers. It does not hedge on the rules where Apple + experts + Reddit + popular OSS converge. It commits to triggers on the rules where they don't.

---

## The MV pattern — your default

**The view IS the view model.** That is the architectural stance for new SwiftUI code in 2026. Not "consider it." Default to it.

- A SwiftUI `View` struct composes from sources of truth: `@State` for view-local, `@Bindable` for binding-extracting, `@Environment(_:)` for shared `@Observable` instances, `@Query` for SwiftData.
- The body recomputes when those sources mutate. AttributeGraph does keypath-precise invalidation since iOS 17. There is nothing a per-screen ViewModel wrapper adds to this loop that it doesn't already do.
- `@Observable` (Swift 5.9+, iOS 17+) killed the original *technical* reason for ViewModels: granular `@Published` plumbing. You no longer earn precision through a wrapper. You get it free.

### Why MV wins on the AttributeGraph

- `@Observable` does **per-property** tracking. A view that reads `model.title` only re-renders when `title` mutates. A view that reads `model.count` only re-renders when `count` mutates.
- A ViewModel wrapper adds an indirection but no precision. Worse: it tends to surface as a single `@Observable` whose every reader gets invalidated when any property changes — *less* precise than the underlying model.
- Computed properties on a ViewModel that combine multiple stored properties invalidate on any contributing change. Same view-direct.

### The canonical MV view

```swift
struct ArticleListView: View {
    @Environment(ArticleStore.self) private var store     // shared @Observable
    @Environment(\.modelContext) private var modelContext // SwiftData
    @Query(sort: \Article.publishedAt, order: .reverse)
    private var articles: [Article]

    @State private var searchText = ""                     // view-local
    @State private var selection: Article.ID?              // view-local

    var body: some View {
        List(selection: $selection) {
            ForEach(filtered) { ArticleRow(article: $0) }
        }
        .searchable(text: $searchText)
        .task { await store.refresh() }
    }

    private var filtered: [Article] {
        guard !searchText.isEmpty else { return articles }
        return articles.filter { $0.title.localizedStandardContains(searchText) }
    }
}
```

No ViewModel. No protocol. No mock. The view composes from `@Environment`, `@Query`, and `@State`. It is testable through preview + UI test + integration test. It re-renders precisely when any read property mutates.

### Apple ships zero ViewModels in current samples

- **Backyard Birds** (WWDC23 SwiftData sample, `apple/sample-backyard-birds`): 0 `@Observable`, 0 `ObservableObject`, 0 `*ViewModel.swift` files. Pure `@Query` + `@State` + `@Environment` + `@Bindable`. 5 `@Model` classes hold the data; views read them directly.
- **Food Truck** (WWDC22): still ships `@StateObject FoodTruckModel: ObservableObject` — the pre-Observation pattern. Useful as a *before* picture, not a current reference. (Apple has not updated this since `@Observable` shipped.)
- **Landmarks** (iOS 26 era rewrite): WWDC25 Liquid Glass update keeps the same Apple-style pattern.

If Apple's own modern samples ship zero ViewModels, the burden of proof is on a code reviewer asking why a screen has one — not on a developer who omits one.

---

## ViewModels — when to use, when to skip (with triggers)

This is the contested rule. The plugin does not pretend Reddit is settled. It gives triggers.

### DEFAULT: skip the ViewModel

Most screens in 2026 don't need one. `View` + `@State` + `@Environment` + `@Bindable` covers display, simple input, and shared state. A `*ViewModel.swift` file per screen by project convention is `View + a synonym` and produces dead files that hold a single `@MainActor func load() async`.

### USE A VIEWMODEL when ANY of these triggers fire

- The screen has an **explicit state machine**: `loading / loaded / error / empty` plus retry, pagination, optimistic updates, partial loading, cross-field validation. Encoding that as an enum on a `class` is cleaner than scattering `@State` across the view.
- You plan **significant unit test coverage** for orchestration logic (sequencing of network calls, retry policy, deduplication of requests, optimistic mutation reconciliation). The view struct alone is awkward to unit-test without UI.
- You're **migrating UIKit → SwiftUI screen-by-screen** and a `*ViewModel` already exists. Reusing the bridge is cheaper than rewriting twice.
- Team convention favors **consistency over conciseness** and the team has agreed VMs are uniform. The community defense of this position: *"If I use a VM for most pages, and certain pages don't have it, that adds a bit of mental overhead for me."*

### SKIP a ViewModel when ANY of these are true

- The screen is **display-mostly** (list, detail, settings, profile, about). View + bindings cover it.
- The screen is a **SwiftData `@Query` consumer**. Wrapping `@Query` inside a VM breaks observation. Apple's API is designed for direct view access. A common community testimony: *"ViewModels get crazy bloated or the views get too tied to the data layer"* when MVVM meets SwiftData.
- You're a **solo dev** or a **small/medium app**. The maintenance overhead of a ViewModel per screen does not pay for itself at small scale.
- The screen has only a handful of stateful interactions and async calls. The view body can handle it.

### Rules when you DO use a ViewModel

- It is `@Observable final class FooViewModel` — class, not struct. Required for `@Observable`'s reference semantics.
- It is **owned by the view via `@State`**:

  ```swift
  struct ProfileScreen: View {
      @State private var viewModel = ProfileViewModel(service: .live)
      var body: some View { /* ... */ }
  }
  ```

- It **never imports SwiftUI**. `import Foundation` + `import Observation` only. This keeps it unit-testable without UI and reusable across AppKit/visionOS if needed.
- It does **NOT own navigation**. `NavigationPath` and route stacks live in a Router, not in the VM. The modern SwiftUI architecture community converges on this point.
- It does **NOT host `@Query`**. SwiftData requires `@Environment(\.modelContext)` and `@Query` to live on the view. Pushing them behind a VM is the #1 SwiftData-MVVM friction in 2026.
- It does **NOT capture singletons**. Services come in via initializer. Use `@MainActor` only if the project does not have default actor isolation.
- It lives at the **screen root only**. Rows, cards, atoms take values via `let` and closures, not a ViewModel of their own.

### Both sides — the honest split

**Pro-MV positions (modern Swift teaching community + Apple samples):**

- The "MV State Pattern" position argues a per-screen ViewModel duplicates the role of the `View` struct in modern SwiftUI.
- Modern Swift teaching writers advocate `@Observable` Stores for shared domain logic rather than per-screen VMs.
- The Swift concurrency / observation community flagged `@StateObject` / `@ObservedObject` as legacy once `@Observable` shipped.
- IceCubesApp's `CLAUDE.md` literally says *"No ViewModels — Use native SwiftUI data flow patterns."*
- A popular community post on the topic puts it bluntly: *"Splitting giant views into real subviews is probably the most useful takeaway here."* Extraction beats wrapping.

**Pro-MVVM positions (community is genuinely split):**

- *"Genuine question, what is the controversy of MVVM + SwiftUI? I have worked on several iOS projects with this combination, and it works fine for me."* — a frequent community refrain.
- *"A SwiftUI View struct is inherently a View Model. However, this doesn't mean all logic should be placed inside the View. A well-balanced approach is to keep UI validation and presentation logic and mapping logic within the View, while business logic should be managed separately using ObservableObject instances."*
- *"MVVM works fine in SwiftUI. SwiftUI has the same reference pattern as WPF, the UI framework MVVM was originally created for by Microsoft."*
- *"If you require or want unit testing, you need a ViewModel (or whatever you want to call it). Swift views, while great, aren't easily testable."*
- *"I value consistency over conciseness."* — a defensible team-convention argument for VMs on every screen.

### The honest reality: IceCubesApp ships 44 ViewModels

IceCubesApp is the canonical modern Mastodon client. Its CLAUDE.md *bans* ViewModels. The repo ships **44 `*ViewModel.swift` files**. On the complex screens — timeline, conversation, notifications, profile editor — VMs exist because those screens hit every trigger above (state machines, pagination, optimistic updates).

So the rule isn't "MVVM is wrong." It's "MV is the default, and even the ban-ViewModels codebases use them when complexity justifies it." A code review should not insist on adding a VM to a 30-line list screen, and should not insist on removing a VM from a paginated timeline. Match the rule to the trigger.

---

## App.swift owns shared singletons via @State

The 2026 canonical pattern for app-level shared state. Universal across IceCubesApp, IcySky, Maccy, Cork, Apple's Backyard Birds.

```swift
@main
struct MyApp: App {
    // Each .shared is the canonical instance for that domain.
    @State private var theme = Theme.shared
    @State private var router = AppRouter()
    @State private var auth = AuthStore.shared
    @State private var appearance = AppearanceStore.shared
    @State private var toast = ToastCenter.shared

    // If you need a delegate for AVAudioSession / push / RevenueCat / etc.
    @UIApplicationDelegateAdaptor private var delegate: AppDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(theme)
                .environment(router)
                .environment(auth)
                .environment(appearance)
                .environment(toast)
        }
    }
}
```

### Rules baked in here

- `@State`, not `@StateObject`. `@StateObject` is for `ObservableObject` — legacy for new code.
- `@State` lazy-evaluates the initializer once per app lifetime in `App` scope. Same shape as `@StateObject` had, simpler wrapper.
- `.environment(value)` — the unkeyed form for `@Observable` instances. Not `.environmentObject(value)` (legacy).
- Consumers read with `@Environment(Theme.self) private var theme` — not a custom `EnvironmentKey`.
- Do not register a primary singleton via custom `EnvironmentKey`. Use `.environment(_:)`. Custom keys are for **defaultable values** (a font, a measurement unit, a feature flag) — not for shared mutable state.

### Two flavors of ownership

| Flavor | When | Example |
|---|---|---|
| **`.shared` singleton** | State must be reachable from extensions (widgets, share sheet, intents) | `Theme.shared`, `AuthStore.shared` |
| **Owned `@State var x = X()`** | State is process-local; lifetime matches app | `@State var router = AppRouter()` |

IcySky uses the second form for `Auth`/`Router` because it doesn't share state with widgets/extensions. IceCubesApp uses `.shared` because its extensions need the same instance. Choose by reachability requirement.

### Source citations

- **IceCubesApp** `IceCubesApp/App/Main/IceCubesApp.swift` — 10 `@State` properties bound to `.shared` instances (`AppAccountsManager.shared`, `CurrentInstance.shared`, `Theme.shared`, etc.).
- **IcySky** `App/IcySkyApp.swift` — 4 `@State` properties (`appState`, `auth`, `router`, `postDataControllerProvider`).
- **Backyard Birds** `BackyardBirdsApp.swift` — minimal: `@main struct BackyardBirdsApp: App { var body: some Scene { WindowGroup { ContentView().backyardBirdsShop().backyardBirdsDataContainer() } } }`. SwiftData container is the only injected resource.

---

## TCA — when to use, when to skip (with triggers)

**The Composable Architecture** (pointfreeco/swift-composable-architecture) is a Redux-shaped, exhaustively-testable architecture. It is one of the most debated topics in SwiftUI 2026. The plugin commits to a default and gives triggers.

### DEFAULT: don't use TCA

Vanilla SwiftUI + `@Observable` + `@Observable Router` suffices for most apps. Outside the Point-Free ecosystem, TCA adoption among popular maintained OSS Swift apps is rare. Even Apple's most architecturally explicit samples don't use it.

### CONSIDER TCA when several of these needs align

- The app has **many screens with cross-screen state coordination** — auth flows that mutate home-screen state down the line; sync engines that update unrelated views; deep navigation graphs that branch.
- Deterministic action/effect testing, dependency control, cancellation, or replayability addresses a concrete reliability or product risk.
- Important state and effects need a lifecycle independent of a particular view hierarchy.
- The team values uniform reducer/action conventions enough to pay the learning, SourceKit, and dependency costs.
- A regulated or high-assurance product has an actual verification plan that benefits from exhaustive reducer tests. Regulation alone does not make TCA appropriate.

### PREFER VANILLA SWIFTUI when the costs outweigh those benefits

- State and effects are local enough that Observation and focused stores remain clear.
- The app is small, experimental, or optimized for rapid iteration.
- Data-flow-driven app where SwiftData + `@Observable` already cover state.
- The team does not want to own the reducer model or third-party dependency.
- **Xcode autocomplete responsiveness matters.** TCA's nested enums tax SourceKit hard — community testimony repeatedly mentions large reducer files lagging Xcode to the point that working with them becomes impractical.

### Both sides — the honest split

**Pro-TCA positions:**

- The shared-state problem has improved over recent TCA releases; teams that committed early describe it as the missing piece for going all in.
- Point-Free's `isowords` is the canonical TCA reference codebase: many SPM packages, fully reducer-driven, demonstrates the pattern at scale.
- A widely-covered MVVM-C → TCA migration at large engineering-team scale (InfoQ and the company's engineering blog) reported improved performance and testability as the wins. The lesson is "TCA can pay off when the team and app shape fit," not "TCA is universally a step up."

**Anti-TCA positions:**

- *"It's crap for sure... it adds unnecessary complexity and a central dependency"* — a representative skeptic position.
- Large reducers lag Xcode and SourceKit visibly on real codebases.
- *"You can't get back from it unless rewriting the whole app... The whole project depends on two guys who maintain TCA."* The vendor-lock concern is real.
- *"The rationale of literally putting all of your eggs in a third party framework's basket when you're developing, as you say, a huge app. If TCA is abandoned or you decide it is no longer fit for your purpose, you're either stuck with it or forced to do a near-full rewrite."*

### Reality check

- **Among the popular maintained Swift OSS apps with public repositories, TCA adoption is nearly nonexistent outside Point-Free's own** (isowords).
- Even IceCubesApp — feature-rich, 7k stars, full Swift 6 + `@Observable` — explicitly does **not** use TCA.
- The Browser Company / Arc pinned an early TCA fork and ate the maintenance cost (years out of date). This is a cautionary tale for vendor lock-in, not a TCA indictment — but it is a real risk surfaced repeatedly in Reddit threads.
- TCA-or-bust is a team-shape decision more than a technology one.

### If you adopt TCA

- Single-responsibility reducers from day 1.
- Decompose into per-feature reducers early.
- Treat root `State` aggregation discipline as architectural law.
- Use `@Dependency` injection (Point-Free's `swift-dependencies`) instead of singleton capture.
- Plan for the SourceKit cost — break large reducers into smaller ones before Xcode starts to chew.

---

## Folder structure

Feature-first, not layer-first.

### The rule

- **Top level groups by feature and shared infrastructure.** Not by `Views/`, `ViewModels/`, `Models/`, `Services/`.
- **Inside a feature**, sub-folders by `Views/`, `Models/`, `Services/` are allowed when the feature is large enough to justify them. Small features stay flat.
- A folder is a feature module if you can **delete it** and the rest of the app still compiles after a few `import` / route edits. Use that as the litmus test.

### Why feature-first

- Most edits touch one feature. Layer-first folders force every change to span 4 directories.
- Code review is by feature, not by layer.
- `git log` is interpretable when commits stay in feature folders.
- Feature-first scales naturally to local SPM packages later (one folder → one `Sources/Foo` directory).

### Concrete tree — single-target iOS app

```
MyApp/
├── App/
│   ├── MyAppApp.swift             # @main
│   ├── AppRoot.swift              # tab/root view, .environment(_:) installation
│   ├── AppState.swift             # @Observable, truly-global state
│   └── Bootstrap.swift            # font registration, logging, analytics init
├── Features/
│   ├── Library/
│   │   ├── LibraryScreen.swift
│   │   ├── LibraryStore.swift     # @Observable store (NOT *ViewModel.swift unless using the VM pattern)
│   │   ├── LibraryRoute.swift     # feature-local enum
│   │   ├── Components/
│   │   │   ├── BookRow.swift
│   │   │   └── ShelfCard.swift
│   │   └── Services/
│   │       └── BookCatalogService.swift
│   ├── Reader/
│   ├── Settings/
│   └── Onboarding/
├── DesignSystem/
│   ├── Tokens/
│   │   ├── Color+Tokens.swift
│   │   ├── Spacing.swift
│   │   ├── Typography.swift
│   │   └── Radii.swift
│   ├── Components/
│   │   ├── PrimaryButton.swift
│   │   ├── LoadingView.swift
│   │   ├── EmptyStateView.swift
│   │   └── ErrorView.swift
│   └── Modifiers/
├── Core/
│   ├── Networking/
│   │   ├── APIClient.swift
│   │   └── Endpoints/
│   ├── Persistence/
│   │   ├── ModelContainer+App.swift
│   │   └── Migrations/
│   ├── Auth/
│   ├── Analytics/
│   └── Logging/
├── Navigation/
│   ├── AppRouter.swift            # @Observable
│   ├── AppRoute.swift             # Hashable enum, Codable if deep-linkable
│   └── DeepLinkResolver.swift
├── Shared/
│   ├── Extensions/
│   ├── Protocols/
│   └── Formatters/
├── Resources/
│   ├── Assets.xcassets
│   ├── Localizable.xcstrings
│   └── Fonts/
└── Tests/
    ├── LibraryTests/
    ├── CoreTests/
    └── UITests/
```

### Concrete tree — iOS + macOS with shared SPM packages

```
MyApp/
├── MyApp.xcodeproj
├── Packages/
│   ├── DesignSystem/              # SPM — tokens + atoms; no app deps
│   ├── Core/                      # SPM — networking, persistence, auth
│   ├── LibraryFeature/            # SPM — both apps consume
│   ├── ReaderFeature/             # SPM
│   └── SettingsFeature/           # SPM
├── Apps/
│   ├── iOS/
│   │   ├── iOSApp.swift           # composes features, picks iOS navigation
│   │   ├── Resources/
│   │   └── Info.plist
│   └── macOS/
│       ├── macOSApp.swift         # composes features, picks NavigationSplitView
│       ├── Commands.swift         # main menu commands
│       ├── Resources/
│       └── Info.plist
├── Extensions/
│   ├── Widgets/
│   ├── ShareExtension/
│   └── Watch/
└── Tests/
```

### Apple's Backyard Birds layout (reference)

```
sample-backyard-birds/
├── Backyard Birds.xcodeproj
├── BackyardBirdsData/             # SPM — @Model classes (Bird, Backyard, Plant, …), persistence
├── BackyardBirdsUI/               # SPM — shared screens and components
├── LayeredArtworkLibrary/         # SPM — art assets and layered rendering
├── Configuration/
├── Multiplatform/                 # iOS/iPadOS/macOS/visionOS target
├── Watch/                         # watchOS target
└── Widgets/                       # widget extension
```

Note: data and UI are separate SPM packages from day 1. Models/persistence in one, screens/components in another. The app target composes them.

### Folder name conventions

- `App/` — singular, scope-defining.
- `Features/<Feature>/` — plural at the collection level, singular per feature folder.
- `DesignSystem/` — single word. Avoid `UI/` or `Components/` at the top level; both are too vague.
- `Core/Networking/`, `Core/Persistence/`, `Core/Auth/`, `Core/Analytics/`, `Core/Logging/` — stateless infrastructure or singleton-friendly clients.
- `Resources/` — `Assets.xcassets`, `Localizable.xcstrings`, fonts, `.storekit` files.
- `Shared/` — last resort. If only two features need a thing, put it in one and import. `Shared/` is the rot folder.
- `Navigation/` — top-level only if Routers and deep-link resolution are project-wide. Otherwise per-feature `Route.swift`.

### Anti-patterns at the folder level

- **Top-level `ViewModels/`, `Views/`, `Models/`, `Services/`** as siblings. Forces every change to span four folders.
- **Sibling feature imports** (`LibraryFeature` importing `ReaderFeature`). Dependencies must point inward. Cross-feature communication goes through the app shell or shared protocols.
- **A bloated `Shared/`** — two-consumer-only utilities don't belong there.
- **`Common/`, `Utilities/`, `Misc/`** as top-level folders. Same rot dynamic.
- **`#if os(iOS)` 12 times in one view body** — split into `HomeScreen+iOS.swift` and `HomeScreen+macOS.swift` once you cross ~5 conditional branches.

---

## Local SPM packages — when to modularize (with triggers)

The question that splits Reddit into "modularize from day 1" vs "monolith forever." The answer is neither.

### DEFAULT: flat single target

- Solo or small team.
- Small or medium app where build times are still snappy.
- No widget / watch / share / Live Activity extension to feed shared code into.
- Early-stage project where feature boundaries are still in flux.

A flat target ships faster. Cross-package edits in modular projects burn time. Premature modularization is a tax.

### MODULARIZE to local SPM packages when ANY of these hold

- **Project-file merge conflicts have become routine.** Local SPM avoids the `.xcodeproj` for most edits.
- **Build times start to hurt iteration.** Modularization isolates rebuilds. The exact threshold is whatever the team perceives as painful — not a fixed number.
- **The codebase has grown past "small app" feel.** The single-target compile graph starts to saturate.
- **Feature boundaries are stable** — you can articulate which file belongs to which feature without ambiguity.
- **You need to share code with widget / watch / share-extension targets.** SPM packages let multiple targets consume the same code without `Compile Sources` duplication.

### DON'T MODULARIZE when ANY of these hold

- Solo dev. A widely-cited Reddit comment: *"A perfect example of when 'purism' in software inhibits you rather than benefits you... If there will only ever be one developer on the project over-architecting the code will just be a burden."*
- POC, prototype, or early-stage project.
- Build times are fine without modularization.
- Feature boundaries are still in flux — you don't yet know what a feature is. Multiple high-score community posts make the same point.

### Community consensus on this one is sharp

Several high-score Reddit threads converge: modularize when the team or codebase needs it, not before. *"It sounds like you're confusing over-engineering with proper architecture."*

### Bottom-up extraction order

1. **DesignSystem first**. Has zero app deps. Pure tokens and atoms. Easy to extract.
2. **Networking** next. Stateless API client + endpoints.
3. **Per-feature** last. Each feature becomes its own package once the boundary is stable.

This order avoids extracting a feature into SPM only to discover it depends on something that should also be extracted but isn't yet.

### Real-world numbers

- **IceCubesApp**: 13 SPM packages, full Swift 6, `defaultIsolation(MainActor.self)` in 9 of them. Mature modular codebase.
- **IcySky**: 2 SPM packages (`Model`, `Features`) split into 9 product libraries. Pragmatic medium scale.
- **isowords**: ~100 SPM packages. TCA + Pavel-Holec-style modularization. Outlier.
- **Backyard Birds**: 3 SPM packages (`BackyardBirdsData`, `BackyardBirdsUI`, `LayeredArtworkLibrary`). Apple's canonical small-modular blueprint.
- **CodeEdit**: multi-package via product libraries, 22k stars, single Xcode project + multiple SPM packages.

### Tooling

- **Tuist** — Swift-typed manifests, dependency graph, build cache. The 2026 favorite for many-package projects.
- **XcodeGen** — spec-driven `.xcodeproj` generation. No caching, no graph tooling. Fine for small projects.
- **Bazel** — Spotify/Airbnb scale. Not a default.

### `defaultIsolation(MainActor.self)` per package

```swift
// In Package.swift, for UI-heavy packages
.target(
    name: "MyUI",
    dependencies: ["DesignSystem"],
    swiftSettings: [
        .defaultIsolation(MainActor.self),
        .swiftLanguageMode(.v6),
    ]
)
```

Approachable Concurrency (Swift 6.2+) is opt-in **per SPM target**, even in Xcode 26 projects with the app target opted in. Set it for UI-adjacent packages; leave it off for networking/parsing/persistence packages where background work dominates.

IceCubesApp enables it on 9 of 13 packages. Networking and data layers leave it off.

---

## File organization rules

### One PRIMARY public type per file

Not strict one-type-per-file. **One primary public type**; private/internal helpers co-located.

```swift
// AccountDetailScreen.swift — primary public type
struct AccountDetailScreen: View { /* ... */ }

// Private helper that's only used inside this screen — keep co-located
private struct AccountHeaderRow: View { /* ... */ }
private enum DetailSection { case header, posts, media, replies }
```

Pulling those private helpers into separate files adds noise without adding clarity. They aren't reused; they're scoped to this screen.

### When to split a helper into its own file

- It's referenced by **2+ files** in the same feature.
- It's a **public** type (consumers outside the feature can use it).
- It exceeds **~80 lines** and lives alongside another type that already dominates the file.

### Don't enforce strict file-per-type

- IceCubesApp's CLAUDE.md mandates file-per-type. The codebase ships `Router.swift` with 5 types and `PushNotificationsService.swift` with 3.
- Stats's `popup.swift`, `readers.swift`, `settings.swift` each hold multiple structs per module.
- Insisting on strict one-type-per-file contradicts every popular SwiftUI codebase audited.

The realistic rule is **one *primary public type* per file; private helpers and small co-types stay co-located when they aren't reused elsewhere**.

### File naming convention

| File suffix | Type | When |
|---|---|---|
| `<Name>Screen.swift` | `struct <Name>Screen: View` | Full-screen container. Owns Router connection. Receives stores from environment. |
| `<Name>View.swift` | `struct <Name>View: View` | Reusable sub-view used in multiple screens. |
| `<Name>Row.swift` / `Card.swift` / `Bar.swift` | atomic component | Single-purpose UI primitive. |
| `<Name>Store.swift` | `@Observable final class <Name>Store` | Bounded-context shared state (Auth, Cart, Library). Inject via `.environment(_:)`. |
| `<Name>Manager.swift` | `@Observable final class <Name>Manager` | Long-lived coordinator (PushNotifications, BackgroundTasks). Same shape as Store. |
| `<Name>ViewModel.swift` | `@Observable final class <Name>ViewModel` | **Only when using the VM pattern** for a screen with state-machine complexity. Default folder convention: no VMs. |
| `<Name>Service.swift` | `final class / actor <Name>Service` | Stateless capability boundary (AuthService, ImageLoaderService). |
| `<Name>Repository.swift` | `final class <Name>Repository` | Only when you genuinely have one second implementation. Don't add for "Clean" symmetry. |
| `<Name>Route.swift` | `enum <Name>Route: Hashable` | Feature-local route enum. Cross-feature is `AppRoute` in `Navigation/`. |
| `<Name>Modifier.swift` | `struct + extension View` | A `ViewModifier` and its `.<lowercaseName>()` helper in the same file. |

### Naming caveats

- **Avoid `Manager` for fresh stores** if you can. Prefer `Store`. `Manager` reads as legacy.
- **Avoid `*ViewModel.swift`** as a default naming convention — only use it when you've consciously chosen the VM pattern for that screen.
- **Avoid `*Protocol`** suffix on protocols. Use the noun (`AuthClient`), with concrete `LiveAuthClient` and `MockAuthClient`.

---

## Navigation architecture

The 2026 pattern is unanimous: **`NavigationStack(path:)` + `@Observable Router` + typed `Hashable` routes**. See `references/navigation.md` for the full deep dive. Architecturally:

### One NavigationStack per tab

```swift
struct AppTabsRoot: View {
    @Environment(AppRouter.self) private var router
    @Bindable var routerBindable: AppRouter

    var body: some View {
        TabView(selection: $routerBindable.selection) {
            ForEach(AppTab.allCases) { tab in
                NavigationStack(path: $routerBindable[tab]) {
                    rootView(for: tab)
                        .navigationDestination(for: AppDestination.self) { destination in
                            view(for: destination)
                        }
                }
                .tag(tab)
            }
        }
    }
}
```

### One @Observable Router per tab (or one with per-tab path)

```swift
@Observable
final class AppRouter {
    var selection: AppTab = .home
    private var paths: [AppTab: [AppDestination]] = [:]

    subscript(tab: AppTab) -> [AppDestination] {
        get { paths[tab] ?? [] }
        set { paths[tab] = newValue }
    }

    func push(_ destination: AppDestination, on tab: AppTab? = nil) {
        let target = tab ?? selection
        paths[target, default: []].append(destination)
    }

    func popToRoot(_ tab: AppTab) { paths[tab] = [] }
}
```

IcySky's `AppRouter` is the cleanest reference for this exact subscript pattern (`@Bindable var router = router` then `$router[tab]`).

### Typed Hashable route enums

```swift
enum AppDestination: Hashable, Codable {
    case profile(userID: User.ID)
    case post(postID: Post.ID)
    case settings
}
```

`Codable` lets the routes survive state restoration and deep links.

### Cross-tab navigation

```swift
// From any view
@Environment(AppRouter.self) private var router

Button("Open Profile") {
    router.selection = .home
    router.push(.profile(userID: user.id), on: .home)
}
```

The Router is the single coordination point. Views don't reach into each other.

### Sheets driven by Identifiable enum

```swift
enum SheetDestination: Identifiable, Hashable {
    case compose
    case settings
    case profile(User.ID)

    var id: String {
        switch self {
        case .compose: "compose"
        case .settings: "settings"
        case .profile(let id): "profile-\(id)"
        }
    }
}

// In view
@State private var sheet: SheetDestination?

.sheet(item: $sheet) { destination in
    switch destination {
    case .compose: ComposeScreen()
    case .settings: SettingsScreen()
    case .profile(let id): ProfileScreen(userID: id)
    }
}
```

**Never** use one bool toggle per modal. The enum scales; toggles don't.

### Programmatic navigation via Router methods

```swift
// Bad — view reaches into Router internals
router.paths[.home, default: []].append(.profile(userID: id))

// Good — view calls a Router method
router.push(.profile(userID: id))
```

The Router exposes intent (push, pop, popToRoot, present, dismiss). Views don't mutate the path directly.

### Anti-patterns

- **`NavigationView`** — deprecated. Always replace.
- **`NavigationLink(destination:)`** — deprecated. Use `NavigationLink(value:)` + `.navigationDestination(for:)`.
- **String-based routes** (`router.path.append("profile/\(id)")`) — every audited repo uses typed enums.
- **Sharing one `NavigationPath` across tabs** — the most common 2025/2026 SwiftUI bug. Each tab gets its own path.
- **Storing navigation state in a ViewModel** — Routers live above ViewModels.
- **Stinsen / SUICoordinator / SwiftUIRouter** for new SwiftUI — these frameworks pre-date `NavigationStack` typed routes + `@Observable`. Adding them is fighting the framework.

---

## VIPER / Clean Architecture

### Verdict for new SwiftUI in 2026: legacy

- **VIPER**'s Router was a UIKit workaround for tight `UIViewController` coupling. SwiftUI's declarative navigation removes the original justification.
- The Interactor/Presenter split adds protocol-and-mock weight that fights SwiftUI's value-type, identity-driven rendering.
- **Clean Architecture's dependency rule** (dependencies point inward; outer layers depend on inner) is still a useful **check** — but not a literal folder spec.
- Translating Clean's `Entities/UseCases/Interfaces/Presentation/Data` into top-level folders produces over-abstracted SwiftUI apps that ship slower than feature-first MV.

### Use Clean's dependency rule as a check, not a folder spec

- "Does this feature depend on app-shell types?" — bad. Reverse it.
- "Does Core depend on a Feature?" — bad. Core is inward.
- "Does DesignSystem depend on Core?" — bad. DesignSystem is the most inward.
- "Does Feature A depend on Feature B?" — bad. Cross-feature goes through shared protocols or app shell.

### The Reddit verdict

- A widely-shared community post titled *"Why I've stopped using modular/clean architecture for SwiftUI"* makes the canonical "I tried it, it overcomplicated everything for my team size" argument.

### If you need testability that Clean promises in 2026

- `@Observable` stores + protocol-based services + small mocks.
- Organize **by feature, not by layer**.
- Use Swift Testing parameterized tests for orchestration coverage.
- Snapshot test design-system primitives only.

You get the testability without the folder weight.

---

## Anti-patterns (architecture-specific)

Flag these on review.

### Code organization

- **One huge file with many public types** — `Models.swift` with 12 structs, or `Helpers.swift` with 8 unrelated extensions. Each primary public type gets its own file.
- **Strict layer-first folders at the top level** — `ViewModels/`, `Models/`, `Views/`, `Services/` as siblings of each other. Forces every change to span 4 folders.
- **Sibling feature imports** — `LibraryFeature` importing `ReaderFeature`. Cross-feature deps go through the app shell.
- **A bloated `Shared/` or `Common/` folder** — two-consumer utilities don't live there. Folder rot.

### Patterns

- **Per-screen ViewModel imposed by convention before complexity warrants it** — A 30-line `SettingsScreen` with a `SettingsViewModel` that holds one `@MainActor func load()`. The VM adds no value.
- **Stinsen / SUICoordinator-style nav frameworks for new SwiftUI** — these predate `NavigationStack` typed routes. Pure `@Observable Router` covers their use case.
- **Coordinator pattern as a separate class hierarchy** in SwiftUI — same as above. Use a Router + `NavigationPath`.
- **`@StateObject` for new `@Observable` code** — `@StateObject` is for `ObservableObject`. Use `@State` for `@Observable` instances.
- **Custom `EnvironmentKey` for primary app singletons** — use `.environment(_:)` and `@Environment(Theme.self)`. Custom keys are for defaultable values (a feature flag, a measurement unit), not for shared mutable state.
- **Mixing `@Observable` with `ObservableObject`/`@Published`** in the same type. They don't compose.
- **Putting `@Query` behind a repository** for testability when no second implementation exists.
- **Storing navigation state inside ViewModels** — views own presentation intent. ViewModels expose state.

### Process

- **Repo-wide Swift 6 strict migration in one PR** — never. Module by module. The widely-cited migration playbooks describe moving hundreds of files incrementally over months, not weeks.
- **Premature SPM split** — don't break a 5-screen app into 8 packages. Each cross-package edit becomes a chore.
- **Reaching for SwiftData abstractions on day one** — hide `@Model`/`@Query` behind a repo "for testability" without a real second implementation, and you'll regret the cost.
- **Adding VIPER scaffolding to SwiftUI** — wrong toolset for the framework's grain.

---

## Quick decision matrix — when reviewing architecture

| Question | Answer |
|---|---|
| New screen, no state machine, list-driven? | MV. No ViewModel. |
| Screen has `loading/loaded/error/empty + retry + pagination`? | Add a `@Observable` ViewModel owned by `@State`. |
| Project is a small utility, solo dev? | Flat target. No SPM split. |
| Project-file merge conflicts routine + build times painful? | Local SPM packages. Bottom-up: DesignSystem first. |
| Should I add TCA? | Consider it when cross-feature coordination, independent state lifecycle, deterministic effect testing, and team conventions create benefits that outweigh reducer/tooling/dependency cost. Regulation is neither required nor sufficient. |
| Reusing `NavigationLink(destination:)`? | Replace with `NavigationLink(value:)` + `.navigationDestination(for:)`. |
| New `Router` class? | `@Observable final class`, owned by app via `@State`, injected via `.environment(_:)`, holds `NavigationPath` or per-tab `[Destination]`. |
| Top-level `ViewModels/` folder exists? | Refactor to feature-first. Flag. |
| `@StateObject` in new code? | Replace with `@State` of `@Observable` instance. |
| Custom `EnvironmentKey` for `Theme`? | Replace with `.environment(theme)` + `@Environment(Theme.self)`. |
| 12 `#if os(iOS)` in one view body? | Split into `Screen+iOS.swift` and `Screen+macOS.swift`. |

---

## Cite real codebases in code reviews

When reviewing real code, anchor opinions in real codebases the team can read:

- **Apple Backyard Birds** — zero ViewModels, pure SwiftData + `@Query` + `@Environment`. Apple's own reference architecture.
- **IceCubesApp** — 13 SPM packages, full Swift 6, `.shared` singletons + `@State`, `@Observable` Router in `Env` package. Ships 44 ViewModels despite the CLAUDE.md ban — be honest about that.
- **IcySky** — 2 SPM packages, iOS 26 target, `@Observable` from day 1, `AppRouter` subscript pattern.
- **CotEditor** — modern Mac app proving SwiftUI is not required everywhere. Many Swift Testing files. AppKit where it counts.
- **isowords** — the canonical TCA reference codebase. Cite when discussing TCA.
- **Maccy** — SwiftUI menu-bar utility with selective AppKit. Modern `@Observable`-based.
- **Cork** — modern SwiftUI Mac app with `@Observable`. Tuist-managed.
- **Community testimony** — when a topic is contested, quote both sides anonymously. The quote teaches; the handle is noise.

Anchor reviews in these codebases. A real file at a real URL carries more weight than "best practice."
