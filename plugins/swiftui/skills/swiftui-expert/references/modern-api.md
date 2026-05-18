# Modern API replacements

Target: Swift 6.3 / iOS 26 / macOS 26 / Xcode 26.

This file is the deprecation and replacement reference. Each row lists an older API that you'll find in real codebases and what to use in new code. The replacements aren't a debate — these are the versions that compose with the rest of the modern framework. When you find an older one during a review, flag it; the older form may still compile and may still work in isolation, but it won't compose well with the rest of what you're writing in 2026.

Cross-references:
- Language idioms (typed throws, `replacing(_:with:)`, generics) are in `swift-idioms.md`.
- `@Observable` and `@Bindable` mechanics are in `state-and-observation.md`.
- `NavigationStack` details are in `navigation.md`.
- Liquid Glass surface rules are in `liquid-glass.md`.
- View composition implications are in `view-composition.md`.

## The core replacement table

These are the changes you'll find in almost any pre-2024 codebase. The floor is iOS 17 — every replacement here is available there or earlier.

| Older API | Replacement | Why |
| --- | --- | --- |
| `ObservableObject` + `@Published` + `@StateObject` / `@ObservedObject` / `@EnvironmentObject` | `@Observable` macro + `@State` / `@Bindable` / `@Environment(MyType.self)` | Per-keypath invalidation instead of whole-object. Fewer property wrappers. No Combine import needed. |
| `NavigationView` | `NavigationStack(path:)` or `NavigationSplitView` | Old form doesn't support a path binding, deep linking, or state restoration. |
| `NavigationLink(destination:)` | `NavigationLink(value:)` with a `navigationDestination(for:)` modifier | Value-based form works with the path. Eager form doesn't. |
| `.navigationBarLeading` / `.navigationBarTrailing` placements | `.topBarLeading` / `.topBarTrailing` | Renamed to match the broader platform vocabulary. |
| `.tabItem { ... }` | `Tab("Title", systemImage:, value:) { ... }` | Typed tab declarations; supports `Tab(role: .search)` and `TabSection`. |
| `tabViewStyle(.page)` for paging carousels | `ScrollView` + `.scrollTargetBehavior(.paging)` + `.scrollTargetLayout()` | Per-item width support; integrates with `.scrollPosition(id:)`. |
| `PreviewProvider` struct | `#Preview { ... }` macro | One line. Composes with `@Previewable`. |
| Manual `EnvironmentKey` + `EnvironmentValues` extension + getter/setter | `@Entry` macro on a property in an `EnvironmentValues` extension | One line for what used to take three pieces. |
| `.foregroundColor(_:)` | `.foregroundStyle(_:)` | Accepts `ShapeStyle` — gradients, materials, hierarchical styles. |
| `.accentColor(_:)` | `.tint(_:)` | New name, broader scope. |
| `.cornerRadius(_:)` | `.clipShape(.rect(cornerRadius:, style: .continuous))` | Continuous corners; clips properly under transforms. |
| `.font(.system(size:))` | `.font(.body)` and other text styles | Dynamic Type respects the named ramp; literal point sizes don't scale. |
| `onChange(of:perform:)` (single closure argument) | `onChange(of:) { }` (zero-args) or `onChange(of:) { old, new in }` | Newer signatures avoid the parameter ambiguity. |
| `Binding(get:set:)` inside `body` | `@State` plus `.onChange(of:) { ... }` | Inline `Binding` allocates per render and breaks identity. |
| `AnyView` returned from a function | Generics, `@ViewBuilder`, or `Group` | `AnyView` defeats the diff; in `ForEach` rows it's catastrophic. |
| `UIImagePickerController` (in SwiftUI) | `PhotosPicker` for photos; `PHPickerViewController` if UIKit | Out-of-process; no Info.plist photo library entry. |
| `Date()` for "now" | `Date.now` | Reads as intent; same value. |
| `Task.sleep(nanoseconds:)` | `Task.sleep(for: .seconds(_))` or `for: .milliseconds(_)` | `Duration`-based; available since iOS 16. |
| `DateFormatter` per-call | `.formatted(.dateTime.day().month())` or `Text(date, format: ...)` | `FormatStyle` is locale-aware and avoids the global cache footgun. |
| `NumberFormatter` per-call | `.formatted(.number)` or `Text(value, format: ...)` | Same reason. |
| `String(format: "%.2f", value)` | `Text(value, format: .number.precision(.fractionLength(2)))` | C format strings aren't locale-aware. |
| `filter { ... }.count` | `count(where:)` | One pass, no allocation. |
| `replacingOccurrences(of:with:)` | `replacing(_:with:)` | Walks grapheme clusters, not UTF-16 code units. Doesn't split emoji. |
| `Image("name")` from a string literal | `Image(.name)` | Generated asset symbols; compile-time check. |
| `DispatchQueue.main.async { ... }` | `@MainActor` on the function, or `await MainActor.run { ... }` | Structured concurrency expresses the same thing without the GCD layer. |
| `GeometryReader` for size-only reads | `containerRelativeFrame(.horizontal, count:, span:)` or `.visualEffect { content, proxy in ... }` | `GeometryReader` greedily fills available space; the modern API doesn't. |
| `UIImpactFeedbackGenerator` | `.sensoryFeedback(_:trigger:)` | SwiftUI-native, value-driven, works on watchOS. |
| `.animation(_)` without a `value:` parameter | `.animation(_:value:)` | Implicit animations without a value bind to "any change," which is rarely what you want. |
| `.scrollIndicators(showsIndicators:)` | `.scrollIndicators(.hidden)` / `.visible` | Enum is clearer than a Bool. |
| `UIGraphicsImageRenderer` from SwiftUI | `ImageRenderer<Content>` | Renders a SwiftUI view to an image. |
| `"public.image"` style file types | `.image` and the `UTType` enum | Type-safe; no string typos. |

## State and observation

| Older | Replacement |
| --- | --- |
| `class Store: ObservableObject { @Published var x = 0 }` | `@Observable class Store { var x = 0 }` |
| `@StateObject var store = Store()` | `@State private var store = Store()` |
| `@ObservedObject var store: Store` | `let store: Store` (read-only) or `@Bindable var store: Store` (with bindings) |
| `@EnvironmentObject var store: Store` | `@Environment(Store.self) private var store` |
| `.environmentObject(store)` | `.environment(store)` |

The mechanics: `@Observable` does keypath-precise tracking. A view that reads only `store.user.name` only re-renders when `name` changes, not on every store mutation. With `ObservableObject` + `@Published`, every `@Published` change invalidates every observer. The difference is most visible in views that read one field from a large store.

There's one trap to know about, covered fully in `state-and-observation.md`: `@AppStorage` doesn't work inside an `@Observable` class. The compiler accepts it; updates silently fail to propagate. The workaround uses a separate storage class.

## Navigation

| Older | Replacement |
| --- | --- |
| `NavigationView { ... }` | `NavigationStack(path: $path) { ... }` |
| `NavigationLink("Title", destination: DetailView())` | `NavigationLink(value: route) { Text("Title") }` + `navigationDestination(for: Route.self) { ... }` |
| `tabItem { Label(...) }` | `Tab("Title", systemImage:, value:) { ... }` |
| string-based paths | Typed `Hashable` route enums |

See `navigation.md` for the routing pattern, path bindings, deep links, and zoom transitions.

## Concurrency

| Older | Replacement |
| --- | --- |
| `DispatchQueue.main.async { ... }` | `@MainActor` annotation, or `await MainActor.run { ... }` from a non-isolated context |
| `Task.sleep(nanoseconds: 1_000_000_000)` | `try await Task.sleep(for: .seconds(1))` |
| `Task.detached { ... }` (for normal background work) | `Task { ... }` (with `@concurrent` annotation on the called function under Approachable Concurrency) |
| Combine `sink` for one-off async work | `for await value in stream { ... }` |
| `.onReceive(timer)` for polling | `.task { for await _ in Timer.publish(...).values { ... } }` |
| Manual `withCheckedContinuation` wrapping a closure-based API | The closure-based API, then `withCheckedThrowingContinuation` only if no async version exists |

Under Approachable Concurrency (Swift 6.2's default for new app targets), `nonisolated async` functions stay on the caller's actor by default. You don't sprinkle `@MainActor` annotations everywhere — the type is on the main actor unless you opt out. See `concurrency.md` for the full story.

## Preview and testing

| Older | Replacement |
| --- | --- |
| `struct ContentView_Previews: PreviewProvider { ... }` | `#Preview { ContentView() }` |
| XCTest for new tests | Swift Testing (`@Test`, `#expect`, `#require`) |
| `XCTAssertEqual(a, b)` | `#expect(a == b)` |
| `setUp` / `tearDown` | `init` and `deinit` on the suite struct, or `.serialized` traits |

XCTest is still required for UI tests (`XCUIApplication`), performance tests (`XCTMetric`), and Objective-C bridge tests. New unit tests should be Swift Testing. See `testing-and-debugging.md`.

## Modifier-level updates

A few smaller modifier replacements that come up:

| Older | Replacement |
| --- | --- |
| `.fileImporter(allowedContentTypes: ["public.image"])` | `.fileImporter(allowedContentTypes: [.image])` (UTType enum) |
| `.fixedSize(horizontal:vertical:)` for shrinking text | `.lineLimit(_)` plus `.minimumScaleFactor(_)` in most cases |
| `EquatableView(content: row)` wrapper | Conform the view itself to `Equatable` |
| `.onAppear { Task { await load() } }` | `.task { await load() }` |
| `.onAppear { ... }` re-running on every recycle in lazy stacks | `.task { ... }` (lifetime-tied) or `.task(id:) { ... }` (re-runs on id change) |

## iOS 17 features worth knowing about

| Feature | What it does |
| --- | --- |
| `@Observable` macro | Replaces `ObservableObject` with per-keypath tracking. |
| `@Bindable` | Lets you write `$model.property` against an `@Observable` instance the view received. |
| `ContentUnavailableView` | First-class empty state view. `.search(text:)` variant for "no results." |
| `containerRelativeFrame` | "X% of the nearest scrollable container" without `GeometryReader`. |
| `visualEffect { content, proxy in ... }` | Read geometry without `GeometryReader`; runs on the render thread. |
| `.scrollTargetLayout()` + `.scrollTargetBehavior(.viewAligned)` | Paging that respects per-item width. |
| `.scrollPosition(id:)` | Two-way binding to the visible item id. |
| `PhaseAnimator` | Multi-step state-driven animation. |
| `KeyframeAnimator` | Precise keyframe control with multiple tracks. |
| `.sensoryFeedback(_:trigger:)` | Haptic and audio feedback bound to a value change. |
| `.inspector(isPresented:)` | Trailing supplementary panel on iPad and Mac. |
| Fill + stroke chain on `Shape` | `Circle().fill(.blue).stroke(.white, lineWidth: 2)` directly. |
| `.scrollClipDisabled()` | Lets shadows and overlays escape the `ScrollView` viewport. |
| `.scrollContentBackground(.hidden)` | Puts gradients or images behind `List` and `Form`. |

## iOS 18 features

| Feature | What it does |
| --- | --- |
| `@Entry` macro | One-line custom environment, focus, transaction, or container key. Replaces the three-piece `EnvironmentKey` + extension + storage. Back-deployable via macro expansion to iOS 13. |
| `Tab` + `TabSection` API | Typed tab declarations. Required for `Tab(role: .search)` and the iOS 26 tab features below. |
| `.tabViewStyle(.sidebarAdaptable)` | Tabs on iPhone, sidebar on iPad and Mac, from one declaration. |
| `.onScrollPhaseChange { _, newPhase in ... }` | Reports `.idle`, `.tracking`, `.interacting`, `.decelerating`, `.animating`. |
| `.scrollTransition { content, phase in ... }` | Per-row rendering effects from scroll position. |
| `MeshGradient` | Multi-point gradient surface. Useful for branded hero areas. |
| `.colorEffect` / `.distortionEffect` / `.layerEffect` | Metal shader modifiers. |
| `TextRenderer` protocol | Custom per-glyph drawing for `Text`. |
| `.searchable(text:tokens:suggestedTokens:)` | First-class compound search filters. |
| String Catalogs (`.xcstrings`) compile-time references | Localization keys are checked at build time in Xcode 16. |

`@Entry` is an iOS 18 / Xcode 16 feature but the macro is back-deployable to iOS 13. You can use it in projects targeting older iOS versions as long as you build with Xcode 16 or later. Same for `MeshGradient` — iOS 18, not iOS 26.

## iOS 26 features (the 2026 set)

This is the new surface. Group by area for clarity.

### Liquid Glass surfaces

Glass goes on chrome (toolbars, tab bars, sheets) and decorative containers. Never on list rows, content tiles, or full-screen backgrounds. See `liquid-glass.md` for the surface rules.

| API | Purpose |
| --- | --- |
| `.glassEffect(_:in:)` | Apply Liquid Glass material to a custom view. The `in:` parameter takes any `Shape`. |
| `GlassEffectContainer { ... }` | Group adjacent glass views so they share a sampling region — they look like one continuous material. |
| `.buttonStyle(.glass)` / `.buttonStyle(.glassProminent)` | Glass button styles. Use prominent for the single primary action per region. |
| `glassEffectID(_:in:)` + `@Namespace` | Morph one glass element into another between states. |
| `.scrollEdgeEffectStyle(.soft)` | Automatic blur ramp under toolbars and tab bars. |
| `.backgroundExtensionEffect()` | In `NavigationSplitView`, mirror and blur content outside the safe area instead of clipping. |
| `.tabBarMinimizeBehavior(.onScrollDown)` | Floating glass tab bar collapses on downward scroll. iPhone default. |
| `.tabViewBottomAccessory { ... }` | Persistent "Now Playing"-style row above the tab bar. Read `\.tabViewBottomAccessoryPlacement` from the environment to switch between compact and expanded layouts. |
| `Tab(role: .search) { ... }` | Dedicated system search tab. One per `TabView`. |
| `ToolbarSpacer(_:placement:)` | `.fixed` and `.flexible` toolbar grouping. |
| `.searchToolbarBehavior(.minimizable)` | Collapsible search field in the toolbar. |
| `.navigationTransition(.zoom(sourceID:in:))` | Pairs with `.matchedTransitionSource(id:in:)` for "thumbnail expands to detail" transitions. |
| `.containerShape(_:)` | Tells children the shape they live inside, so concentric children can match curvature. |
| `ConcentricRectangle` | Corners that share a center with the parent's container shape. Beats stacked `RoundedRectangle` guessing. |
| `.controlSize(.extraLarge)` | Fifth control size for hero primary buttons. |
| `Slider(... ticks: { ... }, neutralValue:)` | Native step, tick, and center-fill slider. |

### Observation, animation, intents

| API | Purpose |
| --- | --- |
| `@Animatable` macro | Synthesizes `animatableData` from your stored properties. Pair with `@AnimatableIgnored` for properties that shouldn't animate. |
| `Observations { @Observable read }` async sequence | Stream of transactional changes from `@Observable` instances. Replaces hand-rolling `withObservationTracking` plus an `AsyncStream` continuation. |
| `BGContinuedProcessingTask` | Long-running background processing that the system can reschedule rather than terminating mid-task. |
| `IntentValueQuery` + `SemanticContentDescriptor` | Surface app entities in Spotlight and Visual Intelligence. See `ios-platform.md`. |
| `PermissionKit` | Unified permission framework — request and status across camera, contacts, calendar, location. |
| `DeclaredAgeRange` | Age-range API for age-gating without storing date of birth. |
| `UndoableIntent` | System undo gesture wired to an `AppIntent`. |
| `@Generable` (Foundation Models) | Mark a struct as generable by an on-device model for structured-output prompts. |

### UI primitives

| API | Purpose |
| --- | --- |
| `WebView(url:)` / `WebPage` (from WebKit) | First-class SwiftUI web view. Retires the `UIViewRepresentable` wrappers around `WKWebView`. |
| `lineHeight(_:)` modifier on `Text` | Line height control on `Text` without manual line-spacing hacks. |
| `AttributedString.lineHeight` | Per-run line height on attributed text. |
| `TextEditor` accepts `AttributedString` | Native rich text editing. |
| `.font(.body.scaled(by: 1.5))` | Multiplier on a text style. Lighter than `@ScaledMetric` when you only need text scale. |
| `SemanticContentDescriptor` | Declares the semantic role of view content for assistive tech and Visual Intelligence. |

### Scenes and bridging

| API | Purpose |
| --- | --- |
| `UIHostingSceneDelegate` | UIKit apps can host SwiftUI-only scenes (volumes, immersive spaces, `MenuBarExtra`). |
| `NSGestureRecognizerRepresentable` | AppKit gesture recognizer bridged into SwiftUI. |
| `NSHostingView` in Interface Builder | AppKit/SwiftUI hybrid in storyboards and XIBs without code. |

## Swift 6.2 / 6.3 language and library features

These aren't SwiftUI APIs, but they change the shape of SwiftUI code. See `swift-idioms.md` for fuller coverage and `concurrency.md` for the concurrency surfaces.

| Feature | What it does |
| --- | --- |
| Default actor isolation = `@MainActor` (Approachable Concurrency) | New app targets get MainActor by default — no explicit annotation on `@Observable` types and view code. |
| `nonisolated(nonsending)` semantics | `nonisolated async` functions run on the caller's actor by default instead of hopping to a background executor. |
| `@concurrent` | Explicit opt-in for parallel execution. Use when you want to be on a background thread. |
| `Observations { }` sequence | Async stream of mutations on `@Observable` types. |
| Typed `NotificationCenter` | Notifications as concrete types conforming to `MainActorMessage` or `AsyncMessage`. |
| Typed `throws(MyError)` | Compile-time-checked error types. Use sparingly — public surfaces are usually better as untyped `throws`. |
| `InlineArray<N, T>` / `[N of T]` | Stack-allocated fixed-size arrays. Useful for hot paths. |
| `Span<T>` / `MutableSpan<T>` | Safe contiguous-memory views. Replace `UnsafeBufferPointer` for C and Metal bridging. |
| Swift Testing 6.2 | Exit testing, attachments, raw identifier display names. |
| `Subprocess` package | Replaces ad-hoc `Process` wrappers for shelling out. |

I'm noting Approachable Concurrency under Swift 6.2 specifically; I'd verify the exact Swift version a project is on (6.2 vs 6.3) before recommending the language features that depend on it.

## Code examples for the 2026 set

### `@Animatable` and `@AnimatableIgnored`

```swift
@Animatable
struct PulseEffect: ViewModifier {
    var progress: Double
    @AnimatableIgnored var color: Color = .accentColor

    func body(content: Content) -> some View {
        content
            .scaleEffect(1 + progress * 0.2)
            .opacity(1 - progress * 0.4)
            .foregroundStyle(color)
    }
}
```

No manual `var animatableData: Double { get progress / set progress = newValue }`.

### `@Entry`

```swift
extension EnvironmentValues {
    @Entry var theme: AppTheme = .default
    @Entry var hapticBudget: HapticBudget = .normal
}
```

That's the full declaration. No separate `EnvironmentKey`, no `static var defaultValue`, no getter/setter on `EnvironmentValues`.

### `ConcentricRectangle` and `.containerShape`

```swift
struct Card<Content: View>: View {
    @ViewBuilder let content: Content
    var body: some View {
        content
            .padding(16)
            .background(.regularMaterial, in: ConcentricRectangle())
            .containerShape(ConcentricRectangle())
    }
}
```

Children inside the card can read the container shape and match its curvature.

### `Observations { }` async sequence

```swift
let stream = Observations {
    (model.title, model.isFavorited)
}

for await (title, favorited) in stream {
    log.info("title=\(title) favorited=\(favorited)")
}
```

Replaces the hand-rolled `withObservationTracking` + `AsyncStream.makeStream()` pattern that used to be the only way to observe `@Observable` mutations as a stream.

### `BGContinuedProcessingTask`

```swift
import BackgroundTasks

BGTaskScheduler.shared.register(
    forTaskWithIdentifier: "com.acme.app.sync",
    using: nil
) { task in
    Task {
        let work = ContinuedSync()
        let result = await work.run()
        task.setTaskCompleted(success: result)
    }
}
```

Use when work may continue past a single deadline; the system can reschedule rather than terminate.

### `PermissionKit`

```swift
import PermissionKit

let permission = Permission.camera
let status = await permission.requestAccess()
switch status {
case .granted:    startSession()
case .denied:     showSettingsPrompt()
case .restricted: showRestrictedAlert()
}
```

### `DeclaredAgeRange`

```swift
import DeclaredAgeRange

let ageRange = await AgeRangeService.requestRange()
guard ageRange.includes(.thirteenAndOver) else {
    return showAgeGate()
}
```

The app gets an age range without ever knowing the user's date of birth.

### `tabBarMinimizeBehavior` and `tabViewBottomAccessory`

```swift
TabView(selection: $tab) {
    Tab("Home", systemImage: "house", value: .home) { HomeScreen() }
    Tab("Library", systemImage: "books.vertical", value: .library) { LibraryScreen() }
    Tab(role: .search) { SearchScreen() }
}
.tabBarMinimizeBehavior(.onScrollDown)
.tabViewBottomAccessory { NowPlayingBar() }
```

### `ToolbarSpacer`

```swift
.toolbar {
    ToolbarItem(placement: .topBarLeading) { Button("Edit", action: edit) }
    ToolbarSpacer(.fixed, placement: .topBarLeading)
    ToolbarItem(placement: .topBarLeading) { Button("Select", action: select) }

    ToolbarSpacer(.flexible)

    ToolbarItem(placement: .topBarTrailing) { Button("Done", action: done) }
}
```

The fixed spacer groups Edit and Select as related items. The flexible spacer pushes Done to the trailing edge.

### `glassEffectID` for morphing

```swift
@Namespace private var glassNS

if isExpanded {
    ExpandedPanel()
        .glassEffect(in: .rect(cornerRadius: 24))
        .glassEffectID("panel", in: glassNS)
} else {
    CollapsedChip()
        .glassEffect(in: .capsule)
        .glassEffectID("panel", in: glassNS)
}
```

### `MeshGradient` (iOS 18, included here for completeness)

```swift
MeshGradient(
    width: 3,
    height: 3,
    points: [
        [0, 0],    [0.5, 0],   [1, 0],
        [0, 0.5],  [0.5, 0.5], [1, 0.5],
        [0, 1],    [0.5, 1],   [1, 1]
    ],
    colors: [.indigo, .purple, .pink, .pink, .orange, .pink, .indigo, .purple, .blue]
)
```

`MeshGradient` is iOS 18, not iOS 26 — easy to misremember because it's the kind of effect that looks new.

### `WebView`

```swift
import WebKit
import SwiftUI

struct ArticleViewer: View {
    let url: URL
    var body: some View {
        WebView(url: url)
            .toolbar { ShareLink(item: url) }
    }
}
```

The `WKWebView` `UIViewRepresentable` wrappers can finally retire.

### `Text.lineHeight` and `AttributedString.lineHeight`

```swift
Text("Long body copy that needs editorial line height.")
    .font(.body)
    .lineHeight(.lines(1.4))

var attributed = AttributedString("Hero")
attributed.lineHeight = .points(48)
Text(attributed)
```

## Text concatenation

Prefer `Text` interpolation over `+` concatenation when possible:

```swift
// Concatenation works but is less composable.
Text("Hello").foregroundStyle(.red) + Text(" World").foregroundStyle(.blue)

// Interpolation is the modern form.
let red = Text("Hello").foregroundStyle(.red)
let blue = Text("World").foregroundStyle(.blue)
Text("\(red) \(blue)")
```

## `ObservableObject` escape hatch

If a third-party library forces `ObservableObject`, or you have a Combine debouncer wired through `@Published`, you can still use it — but you need to `import Combine` explicitly. SwiftUI no longer transitively imports it.

```swift
import Combine
import SwiftUI

final class LegacyStore: ObservableObject {
    @Published var query = ""
}
```

This is an escape hatch, not a default. Migrate to `@Observable` when the dependency allows.

## `ForEach` over `enumerated()`

`enumerated()` returns `EnumeratedSequence`, which doesn't conform to `RandomAccessCollection`. Two paths:

```swift
// When you really need the index.
ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
    Text("\(index): \(item.name)")
}

// Preferred — let Identifiable carry identity.
ForEach(items) { item in
    Text(item.name)
}
```

Never use `id: \.self` on indices for a dynamic list. The index isn't stable identity when items can be inserted or removed.

## Common review findings

When reviewing for outdated API usage:

- `NavigationView` anywhere — replace with `NavigationStack` or `NavigationSplitView`.
- `ObservableObject` and `@Published` in new code — replace with `@Observable`.
- `@StateObject` / `@ObservedObject` / `@EnvironmentObject` — replace with `@State`, `@Bindable`, and `@Environment`.
- `.foregroundColor`, `.accentColor`, `.cornerRadius` — replace with the `.foregroundStyle`, `.tint`, `.clipShape(.rect(...))` forms.
- `Date()` for "now" — `Date.now` reads as intent.
- `Task.sleep(nanoseconds:)` — `Task.sleep(for: .seconds(_))`.
- `DateFormatter` / `NumberFormatter` per-call — `FormatStyle` via `.formatted(...)`.
- `String(format:)` — `FormatStyle`.
- `UIImpactFeedbackGenerator` and other UIKit haptics — `.sensoryFeedback`.
- `.animation(_)` without a `value:` — bind to a value.
- `Image("name")` from a string literal — use the generated symbol.
- `DispatchQueue.main.async` — `@MainActor` or `await MainActor.run`.
- Manual `EnvironmentKey` declarations — `@Entry`.
- Manual `animatableData` boilerplate — `@Animatable`.
- `import UIKit` or `import AppKit` alongside `import SwiftUI` — they're transitively imported.
- `.glassEffect()` applied to list rows or content tiles. Glass is chrome-only.

Each of these still compiles. They don't compose well with the rest of what you'd write today.
