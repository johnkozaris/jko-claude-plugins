# Performance

Target: **Swift 6.3 / iOS 26 / Xcode 26**. As of 2026-05-17.

Two root causes of SwiftUI hitches (WWDC25 306, "Optimize SwiftUI performance with Instruments"):

1. **Long view body updates** — body computations miss the frame deadline.
2. **Unnecessary view updates** — too many views update when they shouldn't.

Everything in this file is a strategy for one of those two.

Cross-references:
- View extraction as a structural fix → `view-composition.md`.
- Per-keypath `@Observable` invalidation mechanics → `state-and-observation.md`.
- AttributeGraph + `body` re-evaluation model → `lifecycle.md`.
- Instruments / `_printChanges` / unified logging → `testing-and-debugging.md`.
- Concurrency on the main actor, `.task` cancellation → `concurrency.md`.

---

## Rule One: Factor Large Bodies into Real Subviews

The single highest-leverage performance fix. A long `body` is usually a sign that the view is doing too many independent things — the threshold isn't precise, but if you're scrolling to read it, it's earning extraction. See `view-composition.md` § Body size for the "smell vs hard rule" nuance.

```swift
// Bad — one huge body re-evaluates whole tree on any change
struct ProfileScreen: View {
    @Environment(UserStore.self) var store
    var body: some View {
        ScrollView {
            VStack {
                // 40 lines: header
                // 30 lines: stats grid
                // 50 lines: activity feed
                // 20 lines: friends list
            }
        }
    }
}

// Good — each section participates in the diff independently
struct ProfileScreen: View {
    @Environment(UserStore.self) var store
    var body: some View {
        ScrollView {
            VStack {
                ProfileHeader(user: store.user)
                StatsGrid(stats: store.stats)
                ActivityFeed(activities: store.recentActivity)
                FriendsList(friends: store.friends)
            }
        }
    }
}
```

When `store.recentActivity` mutates, only `ActivityFeed` re-evaluates. With the monolithic body, the entire body re-evaluates and rebuilds every subexpression.

See `view-composition.md` for the full extraction rules.

---

## Don't Put Expensive Work in `body`

`body` runs on every dependency change. An O(n) sort or a date formatter in `body` runs hundreds of times during a scroll session.

```swift
// Bad — sorts on every body evaluation
var body: some View {
    List(items.sorted { $0.date > $1.date }) { item in
        ItemRow(item: item)
    }
}

// Good — sort once when items change, store the result
@State private var sortedItems: [Item] = []

var body: some View {
    List(sortedItems) { ItemRow(item: $0) }
        .onChange(of: items, initial: true) { _, newItems in
            sortedItems = newItems.sorted { $0.date > $1.date }
        }
}

// Better — sort lives in the model
@Observable
final class Store {
    var rawItems: [Item] = []
    var sortedItems: [Item] { rawItems.sorted { $0.date > $1.date } }  // computed once per body that reads it
}
```

The third form is best when the sorted output is the canonical view of the data — the model owns the derived property.

### Formatters in body

```swift
// Bad — creates formatter every body call
var body: some View {
    Text(date, formatter: {
        let f = DateFormatter()
        f.dateStyle = .medium
        return f
    }())
}

// Good — use FormatStyle directly
var body: some View {
    Text(date, format: .dateTime.day().month().year())
}
```

`FormatStyle` is value-typed and internally cached, so per-call cost is far below allocating a `DateFormatter` in `body` (one of the classic scroll-hitch causes). If the exact magnitude matters, measure it — do not quote a number. See `swift-idioms.md`.

### Allocations in body

Anything that allocates — `Array`, `Dictionary`, `String` interpolation with many substitutions — should happen outside body or be cached:

```swift
// Bad — fresh dictionary every body call
var body: some View {
    ForEach(items) { item in
        Text(item.label).foregroundStyle(colorMap[item.kind] ?? .primary)
    }
}
private var colorMap: [ItemKind: Color] { [.warning: .orange, .error: .red, ...] }  // recomputed every body

// Good — colorMap is a static constant
private static let colorMap: [ItemKind: Color] = [.warning: .orange, .error: .red, ...]
```

---

## Lazy Containers for Large Lists

| Container       | Allocates rows when                          | Releases rows when                              |
| --------------- | -------------------------------------------- | ----------------------------------------------- |
| `VStack`        | All on first layout                          | Never — kept alive                              |
| `LazyVStack`    | As they enter the scroll viewport (+ buffer) | Never — retains old rows after they scroll off  |
| `List`          | On demand from the data source              | Recycles row views as they scroll off            |
| `LazyHGrid`     | Same as LazyVStack, horizontal              | Same retain behavior                             |

**Use `List` when you have a homogeneous list of items and want recycling.** `List` is the most efficient option for large datasets — rows are recycled like UIKit's `UITableView`.

**Use `LazyVStack` when you need custom layout that `List` can't express.** Note: `LazyVStack` does *not* recycle — once a row is allocated, it stays in memory until the parent disappears. For very long lists, that adds up.

**Use `ScrollView { VStack { ForEach { ... } } }` only for short, fixed-size lists** where you want all items pre-rendered (smooth scroll with no allocation hitches — viable only when the whole list comfortably fits in memory at once).

Choose by behavior, not by a magic item count: a plain `VStack` when everything fits on roughly one screen; a lazy container as soon as the list scrolls meaningfully; `List` when the dataset is large or unbounded, because recycling is the only thing that caps memory. When it matters, profile with the SwiftUI Instruments template rather than guessing.

For grids: `LazyVGrid` / `LazyHGrid` for moderate counts; consider `List` with a custom `listRowSeparator(.hidden)` + horizontal stack inside the row for very large grids.

### Don't put `AnyView` in a lazy container

A `ForEach { AnyView(...) }` row body destroys the diff identity. Every row is re-created on every body evaluation. The single fastest way to ruin scroll performance.

See `view-composition.md` for `AnyView` alternatives.

---

## Stable `id` in `ForEach`

`ForEach` requires stable identity per row. Without it, SwiftUI re-creates every row on every change — `@State` resets, scroll position jumps, selection drops.

```swift
// Best — Identifiable conformance
ForEach(articles) { ArticleRow(article: $0) }

// Acceptable — stable id keypath
ForEach(articles, id: \.slug) { ArticleRow(article: $0) }

// Catastrophic — fresh UUID per init
struct Article { let id = UUID() }  // wrong if Article struct is recreated; id changes every render
ForEach(articles, id: \.id) { ... }

// Catastrophic — index identity in a mutable list
ForEach(articles.indices, id: \.self) { i in ArticleRow(article: articles[i]) }
// Index changes on insert/delete → full re-render of everything after the change
```

If your model is a value type re-created on every model mutation, the `id` must be a property whose value persists across re-creations (a server id, a slug, a URL).

---

## Per-Item `@Observable` for Fine-Grained Invalidation

Broad observation dependencies are the #2 cause of unnecessary updates.

```swift
// Bad — every row depends on the full items array
@Observable
final class Store {
    var items: [Item] = []
    var favoriteIDs: Set<Item.ID> = []
}

ForEach(store.items) { item in
    ItemRow(item: item, isFavorite: store.favoriteIDs.contains(item.id))
}
// Toggling one favorite invalidates every row that reads store.favoriteIDs
```

Two paths to fix:

**(a) Per-item `@Observable` models** — push state into each item:

```swift
@Observable
final class Item: Identifiable {
    let id: UUID
    var title: String
    var isFavorite: Bool
}

ForEach(store.items) { item in
    @Bindable var item = item
    ItemRow(item: item)  // reads only item.title and item.isFavorite — invalidates only on those changes
}
```

**(b) Push the read down** — let the row decide what it reads:

```swift
struct ItemRow: View {
    @Environment(Store.self) var store
    let item: Item
    var isFavorite: Bool { store.favoriteIDs.contains(item.id) }
    var body: some View {
        // ItemRow registers a read on store.favoriteIDs — the SET's keypath is
        // what's tracked, so ANY change to favoriteIDs invalidates every row
        // that reads it, not just the rows whose membership flipped.
    }
}
```

Option (a) gives the finest granularity. Option (b) reduces the dependency surface but still couples rows to the full set. For very large lists, (a) is the right shape.

See `state-and-observation.md` for the full deep dive.

---

## Avoid `AnyView`

Type erasure hides the concrete view type from SwiftUI's structural diffing, so identity can't be preserved across the erased boundary — state resets and needless re-renders follow, worst inside lists:

```swift
// Bad
func makeView(for kind: ItemKind) -> AnyView {
    switch kind {
    case .text:  AnyView(TextItem())
    case .image: AnyView(ImageItem())
    }
}

// Good
@ViewBuilder
func makeView(for kind: ItemKind) -> some View {
    switch kind {
    case .text:  TextItem()
    case .image: ImageItem()
    }
}
```

The `@ViewBuilder` form returns `_ConditionalContent<TextItem, ImageItem>` — SwiftUI knows the two possible types and diffs them. `AnyView` is opaque.

**Never put `AnyView` in a `ForEach`.** Worst case for scroll performance.

---

## Conditional Branching vs Ternary

For property-only changes, ternary preserves identity; `if`/`else` creates two structurally different views:

```swift
// Less efficient — creates _ConditionalContent, may recreate platform views
if isActive {
    Circle().fill(.blue)
} else {
    Circle().fill(.gray)
}

// More efficient — same view identity, just property change
Circle().fill(isActive ? .blue : .gray)
```

The `if`/`else` form is correct when the two branches have **different structure** — different views, different children. For property-only changes, use the modifier-level conditional.

---

## Heavy View Initializers

`init` runs constantly — every parent body evaluation invokes child inits. Don't do heavy work in `init`:

```swift
// Bad — blocking I/O on every parent re-render
struct MyView: View {
    @State private var data: [Item]
    init() {
        _data = State(initialValue: loadFromDisk())  // blocking I/O
    }
}

// Good — defer to .task
struct MyView: View {
    @State private var data: [Item] = []
    var body: some View {
        List(data) { ... }
            .task { data = await loadFromDisk() }
    }
}
```

See `lifecycle.md` for the full `init` vs `.task` vs `.onAppear` decision matrix.

---

## Escaping `@ViewBuilder` Closures

Container views that store an escaping `() -> Content` closure force heap allocation and re-evaluation on every parent body. Resolve in `init`, store the value:

```swift
// Worse
struct Card<Content: View>: View {
    let content: () -> Content
    var body: some View { VStack { content() } }
}

// Better
struct Card<Content: View>: View {
    let content: Content
    init(@ViewBuilder content: () -> Content) { self.content = content() }
    var body: some View { VStack { content } }
}
```

See `view-composition.md` for the full mechanics.

---

## Off-Main-Thread Closure Captures

Closures in `Shape.path`, `visualEffect`, `Layout`, `onGeometryChange` are `@Sendable` — they execute on a non-main actor. They must capture values, not access `@MainActor` state directly:

```swift
// Bad — accesses @MainActor state in a Sendable closure (strict concurrency error)
.visualEffect { content, proxy in
    content.offset(y: model.offset)  // compiler error
}

// Good — capture the value before the closure
let currentOffset = model.offset
.visualEffect { content, proxy in
    content.offset(y: currentOffset)
}
```

When the value is itself a property of a `Sendable` type, you can capture the model and read it inside — but the model must be `Sendable`. See `concurrency.md` for the full strict-concurrency guidance.

---

## Pre-Compute in `init` or `let`

For values used multiple places in `body`:

```swift
// Bad — recomputes the filter every body
var body: some View {
    HStack {
        Text("\(items.filter(\.isUrgent).count) urgent")
        ProgressView(value: Double(items.filter(\.isUrgent).count) / Double(items.count))
    }
}

// Good — compute once
var body: some View {
    let urgent = items.count(where: \.isUrgent)
    HStack {
        Text("\(urgent) urgent")
        ProgressView(value: Double(urgent) / Double(items.count))
    }
}
```

Using `let` inside `body` is fine — it scopes the computation to a single body call, not every read.

---

## Debugging Tools

### `Self._printChanges()` inside `body`

Logs which properties changed when SwiftUI re-evaluates the view's body:

```swift
struct ArticleRow: View {
    let article: Article
    var body: some View {
        let _ = Self._printChanges()
        VStack { /* ... */ }
    }
}
```

Output:

```
ArticleRow: @self changed.
ArticleRow: @identity changed.
ArticleRow: article changed.
```

Strip in release builds. Wrap in `#if DEBUG`:

```swift
var body: some View {
    #if DEBUG
    let _ = Self._printChanges()
    #endif
    /* ... */
}
```

### `Self._logChanges()`

Same data via unified logging (`os.Logger`):

```swift
var body: some View {
    let _ = Self._logChanges()
    /* ... */
}
```

Visible in Console.app, filterable, persists across launches. See `testing-and-debugging.md`.

### Random background color trick

"Disco ball" diagnosis for over-invalidation. Apply a randomly colored background and watch what re-renders:

```swift
struct ArticleRow: View {
    let article: Article
    var body: some View {
        VStack { /* ... */ }
            #if DEBUG
            .background(Color(hue: .random(in: 0...1), saturation: 0.6, brightness: 0.9))
            #endif
    }
}
```

Every body re-evaluation rerolls the background color. If you scroll and a row flashes through new colors, it's re-rendering when it shouldn't.

### Instruments 26 SwiftUI template

The SwiftUI instrument (Xcode 26) shows:

- **Long view body updates** — bodies that took longer than 16 ms (60 fps) or 8 ms (120 fps).
- **Unnecessary view updates** — bodies that ran but produced identical output.
- **Cause & Effect Graph** — traces a state mutation → AttributeGraph dependencies → invalidated nodes → body re-evaluations. Includes Environment and `@Observable` chains.
- **Update Groups** — batches updates by render commit, colored orange/red by hitch likelihood.

Profile, find the hot view, fix the cause. Don't optimize what Instruments doesn't flag — you'll waste cycles on noise.

See `testing-and-debugging.md` for the full Instruments workflow.

---

## When to `.drawingGroup()`

`.drawingGroup()` flattens a SwiftUI subtree into a single offscreen-rendered layer. Use it when:

- An animated subtree has dozens of subviews that all redraw every frame.
- A Metal shader's input is a complex SwiftUI composition.
- You see hitches when scrolling a list whose row backgrounds animate continuously.

Don't `.drawingGroup()` by default. It costs memory (offscreen layer) and loses some SwiftUI optimizations (per-subview animation). Apply it when Instruments shows the SwiftUI compositor is the bottleneck.

---

## State Update Coalescing

Multiple state mutations in the same synchronous block coalesce into one body re-evaluation:

```swift
withAnimation(.smooth) {
    isExpanded = true
    selection = item
    scrollPosition = item.id
}
// One body re-evaluation, not three
```

Mutations across `await` suspensions don't coalesce — each resumption is a new render cycle:

```swift
await Task.yield()
isExpanded = true  // body re-eval
await Task.yield()
selection = item   // body re-eval again
```

When you have batched mutations from async work, do them after the suspension finishes:

```swift
let result = await store.fetch()
// All mutations here run in one synchronous block → one render
withAnimation(.smooth) {
    items = result.items
    selection = result.items.first?.id
    isExpanded = !result.items.isEmpty
}
```

---

## Redundant State Updates

SwiftUI skips invalidation when an `Equatable` `@State` value is written with an equal value — but that safety net does not cover reference types, non-`Equatable` values, or the cost of *computing* the redundant value in the first place. Guard when the computation or downstream reads are expensive:

```swift
// Bad — sets state every loop, invalidates dependents every loop
.onChange(of: input) { _, new in
    derived = computeDerived(from: new)
}

// Good — only update when it actually changes
.onChange(of: input) { _, new in
    let next = computeDerived(from: new)
    if next != derived { derived = next }
}
```

For `@Observable` properties, the same applies — write only when the value changes.

---

## Async Tied to View Lifecycle

Use `.task { }` for async work bound to the view's lifecycle. Never `.onAppear { Task { } }` for new code.

```swift
// Good — auto-cancels on disappear and id change
ArticleListView()
    .task { await store.refresh() }

// Re-run on input change
ArticleDetailView(id: articleID)
    .task(id: articleID) { await store.fetchArticle(articleID) }

// Stale
ArticleListView()
    .onAppear { Task { await store.refresh() } }  // no cancellation
```

See `concurrency.md` for the full async-in-views treatment.

---

## Scroll Performance Specifics

- `.scrollContentBackground(.hidden)` lets you put a gradient or material behind `List`/`Form` without the system's default opaque background fighting it.
- `.scrollIndicators(.hidden)` if your design demands; otherwise leave system defaults.
- `.scrollClipDisabled()` when child cards have shadows or matched-geometry overlays you need outside the viewport.
- iOS 26 List on macOS is significantly faster — adopt `List` where you previously avoided it for Mac.
- Don't apply `.compositingGroup()` to every list row. It rasterizes the row to an offscreen layer; useful for layered visual effects, expensive otherwise.

---

## Don't

- Don't put expensive computation in `body`. Move to `init`, `@State`, model method, or pre-compute as a `let` inside body.
- Don't put `AnyView` anywhere. Especially not in `ForEach`.
- Don't default `ForEach(id: UUID())`. Stable identity, every time.
- Don't share one broad `@Observable` model across hundreds of rows. Per-item models or push reads down.
- Don't store escaping `() -> Content` closures in container views. Resolve in `init`.
- Don't ignore strict-concurrency warnings on `visualEffect`/`Shape.path`/`Layout` closures. They run off the main actor.
- Don't `LazyVStack` when `List` would work — `LazyVStack` retains every row it allocates.
- Don't sort/filter inside `body` per render. Cache on the model.
- Don't allocate fresh dictionaries / arrays in `body`. `static let` or `init`.
- Don't `.drawingGroup()` by default. Apply when Instruments shows the compositor is the bottleneck.
- Don't ship `print()` in body. Use `#if DEBUG let _ = Self._printChanges()` for structured invalidation debugging.
- Don't run blocking I/O in view `init`. Defer to `.task`.
- Don't `.onAppear { Task { ... } }` for new code. Use `.task` for cancellation.
- Don't update `@State` to the same value in a hot path. Guard before write.
- Don't `formatter: { let f = DateFormatter(); ...; return f }()` in body. Use `FormatStyle`.
