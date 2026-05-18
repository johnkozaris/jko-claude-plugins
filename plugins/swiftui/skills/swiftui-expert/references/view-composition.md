# View composition

Target: Swift 6.3 / iOS 26 / Xcode 26.

This file is about how to slice a view tree so SwiftUI's diff can do its job, and how to read the modifier chains so views render and animate the way you expect. The mechanics matter — once you understand them, most of the rules below stop being rules and become natural consequences of how the framework works.

Cross-references:
- Identity stability, lazy stacks, and `_printChanges` debugging are in `performance.md`.
- `@Observable` per-property invalidation is in `state-and-observation.md`.
- AttributeGraph mechanics and `body` re-evaluation are in `lifecycle.md`.
- Design tokens and built-in styles are in `design-system.md`.

## Extracting subviews: real structs, not computed properties

This is the single most consequential composition rule in modern SwiftUI, and it's the one that produces the most visible performance wins when you get it right. Apple covered the mechanics in WWDC23 session 10160 ("Demystify SwiftUI performance"), and a similar pattern appeared in subsequent sessions.

When you write a `View` as a real struct, SwiftUI gives it an identity in the AttributeGraph. The framework keeps a record of what inputs that struct received last time it rendered. When the parent rebuilds, SwiftUI compares the new inputs to the old ones. If they're equal, the child's `body` doesn't run at all — the previous render is reused.

When you write a "view" as a computed property returning `some View`, it has no identity of its own. It's part of the parent's body. Anytime the parent rebuilds, the computed property runs again from scratch. Every expression inside it rebuilds. There's nothing for SwiftUI to compare against.

The difference is invisible in a static profile photo screen. It becomes loud the first time you put a counter or a typing field in the parent and watch the whole tree re-render on every keystroke.

```swift
// Re-renders the entire body on every parent change.
struct ProfileView: View {
    @Environment(UserStore.self) var store
    var body: some View {
        VStack { header; bio; stats }
    }
    var header: some View {
        Text(store.user.name).font(.headline)
    }
    var bio: some View { Text(store.user.bio) }
    var stats: some View { StatsRow(stats: store.user.stats) }
}

// Each child has its own identity. SwiftUI skips children whose inputs didn't change.
struct ProfileView: View {
    @Environment(UserStore.self) var store
    var body: some View {
        VStack {
            ProfileHeader(name: store.user.name)
            ProfileBio(bio: store.user.bio)
            ProfileStats(stats: store.user.stats)
        }
    }
}

struct ProfileHeader: View {
    let name: String
    var body: some View { Text(name).font(.headline) }
}
```

When the user updates their bio, only `ProfileBio` re-renders. With the computed-property version, the entire `ProfileView` body runs and every subexpression rebuilds.

The mechanism here is worth knowing because it shows up in other places. `@Observable` tracks reads through the surrounding scope. A computed property reads through its parent's tracking scope — its property reads count as the parent's reads. A separate struct has its own tracking scope. So extracting a subview both gives SwiftUI an identity to compare *and* scopes which observable reads invalidate which views.

There's a corollary that catches people: if you extract a subview and pass the entire `store` to it instead of just the property it reads, you've thrown away the win. The child still depends on the whole store and will re-render on every store change. Pass the smallest data the child actually needs.

## Body size

Long `body` blocks are a smell, not a hard rule. A 70-line body that's one `Form` with twelve `Section`s is fine — it's flat, declarative, and the lines are just describing structure. A 35-line body with three nested `if` branches and two `ForEach` loops is harder to follow and probably wants extraction.

When you find yourself scrolling within a body to remember what variable a closure captured, that's the signal. When you'd struggle to write a preview that exercises one part of it without exercising the rest, that's the signal. The Airbnb folks suggested 10 "composition units" (Text, Image, custom views, branches, ForEach iterations) as a threshold; this is fine as a starting heuristic but don't treat it as a rule.

The triggers I find more reliable than line count:

| Signal | Action |
| --- | --- |
| The same layout appears two or three times in this body | Extract a reusable view |
| The same modifier chain appears three or more times | Either a `ViewModifier` or a `View` extension |
| `ForEach` row body grows past ~20 lines | Extract a `Row` view |
| Several `@State` variables drive disjoint UI regions | The disjoint regions are probably separate views |
| You can describe two halves of the body with different verbs | Split along the verb boundary |

## View ordering convention

When views grow, consistency helps. The order below puts sources of truth first, inputs second, and helpers last. It mirrors the data flow direction:

```swift
struct ArticleDetailView: View {
    // 1. Environment values the view reads.
    @Environment(\.dismiss) private var dismiss
    @Environment(ArticleStore.self) private var store

    // 2. State the view owns or binds.
    @State private var isEditing = false
    @State private var draft = ArticleDraft()
    @Bindable var article: Article

    // 3. Inputs from the parent.
    @Binding var selection: Article.ID?
    let mode: DetailMode

    // 4. Computed properties that read from the above.
    private var canSave: Bool { draft.isValid && draft != article.draft }

    // 5. body.
    var body: some View { ... }

    // 6. Private helpers (actions, not views).
    private func save() async { ... }
    private func cancel() { dismiss() }
}
```

This is convention, not law. If your team prefers a different order, pick one and stick to it — the cost of switching back and forth between styles inside a codebase exceeds the cost of either choice.

## `@ViewBuilder` for content closures

When you build a container that takes child content from the caller, you have two ways to receive it. They're not equivalent.

The recommended form: take a `@ViewBuilder` closure in `init`, call it once, store the resulting `Content` value as a `let`.

```swift
struct Card<Content: View>: View {
    let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) { content }
            .padding(16)
            .background(.regularMaterial, in: .rect(cornerRadius: 16, style: .continuous))
    }
}
```

The form to avoid: store the closure itself.

```swift
struct Card<Content: View>: View {
    let content: () -> Content
    init(@ViewBuilder content: @escaping () -> Content) {
        self.content = content
    }
    var body: some View {
        VStack { content() }
    }
}
```

Why the first one is better: SwiftUI's diff compares the stored `Content` value. If the resolved content didn't change, the child subtree can be skipped. With the escaping closure form, SwiftUI has nothing meaningful to compare — closures don't have a useful equality — so the builder runs and the child subtree rebuilds on every parent body evaluation. The escaping closure also allocates on the heap; the resolved value doesn't.

There's an edge case. If your container needs to call the builder multiple times (a tab view that builds the content lazily, a "show this on hover" overlay that may never render the content), you have to keep the closure. That's the legitimate use of the escaping form — call it once, lazily, when actually needed. But for the common "wrap this content in some chrome" container, resolve in `init`.

## Modifier ordering

Modifier order matters in SwiftUI because each modifier wraps the previous output. The same chain in a different order produces a different view.

### Frame before padding for inner sizing

You're saying "make the content this size, then add space around it."

```swift
Image(systemName: "bell")
    .frame(width: 40, height: 40)
    .padding(8)
```

Total footprint: 56×56. The image stays 40×40 and the 8 of padding surrounds it.

### Padding before background for chrome

You're saying "pad the content, then paint the padded region."

```swift
Text("Done")
    .padding(.horizontal, 16)
    .padding(.vertical, 12)
    .background(.tint, in: .capsule)
```

The capsule fills the padded region. If you swapped the order — `.background(...)` before `.padding(...)` — the capsule would paint only the intrinsic text area, and the padding would extend beyond the painted region.

This is the rule that catches people most often. If your button has padding that looks weird outside the colored fill, check the order.

### Foreground style scope

`.foregroundStyle(_:)` cascades down the view tree to anything that reads its foreground from the environment. Apply it once at the top level for a group of icons and labels, not on each child.

```swift
HStack {
    Image(systemName: "person")
    Text("Profile")
}
.foregroundStyle(.secondary)
```

Both the icon and the text pick up `.secondary` from the cascading style.

### Fill and stroke on shapes

On iOS 17+, you can chain `.fill()` and `.stroke()` directly on a `Shape`. Before, you needed an overlay trick.

```swift
Circle()
    .fill(.blue)
    .stroke(.white, lineWidth: 2)
```

The order here is "fill first, then stroke on top." Stroking before filling gives you a different visual.

### `compositingGroup` before `clipShape` for layered content

When you stack views with shadows and then clip the whole thing, you can get antialiasing seams where the shadows poke through the clip mask. `.compositingGroup()` rasterizes the group first so the clip applies to a single image:

```swift
HStack { /* overlapping decorations with shadows */ }
    .compositingGroup()
    .clipShape(.rect(cornerRadius: 16, style: .continuous))
```

You don't always need this. Use it when you see fringing.

## `AnyView` — avoid in new code

`AnyView` erases the type information SwiftUI uses to diff. When `AnyView` wraps your view, SwiftUI can't tell if the wrapped type changed between renders. It assumes it did, and rebuilds the subtree.

In a single-use spot — like a static "show one of these three things" branch — the cost is small. In a `ForEach` row body, it's catastrophic. Every cell rebuilds on every scroll, and scroll performance falls off a cliff.

The replacements:

```swift
// Don't.
func makeView(for kind: ItemKind) -> AnyView {
    switch kind {
    case .text: AnyView(TextItem())
    case .image: AnyView(ImageItem())
    case .video: AnyView(VideoItem())
    }
}

// Do.
@ViewBuilder
func makeView(for kind: ItemKind) -> some View {
    switch kind {
    case .text: TextItem()
    case .image: ImageItem()
    case .video: VideoItem()
    }
}
```

The `@ViewBuilder` form returns a `_ConditionalContent` type. It has shape, and SwiftUI can diff it. The `AnyView` form is opaque.

If you're tempted to reach for `AnyView` to "fix" a type mismatch, the right answer is usually `@ViewBuilder`, generics, or `Group`.

## `ForEach` identity

`ForEach` needs to know which row is which across re-renders. If it can't tell, it rebuilds every row. The simple cases:

```swift
// Best — the type conforms to Identifiable.
ForEach(articles) { article in ArticleRow(article: article) }

// Acceptable — explicit stable id keypath.
ForEach(articles, id: \.slug) { article in ArticleRow(article: article) }

// For binding rows from a Bindable store.
@Bindable var store: ArticleStore
ForEach($store.articles) { $article in ArticleRow(article: $article) }
```

The mistake to never make: defaulting `id` to a fresh `UUID()` per row:

```swift
// Generates a new id on every render. Every row is "new." Everything rebuilds.
ForEach(articles, id: \.self) { ... }                    // wrong if Article isn't stable-by-value
ForEach(articles.map { (UUID(), $0) }, id: \.0) { ... }  // wrong, fresh UUID every render

struct Article {
    let id = UUID()  // wrong if Article gets recreated; the id changes too
}
```

If `Article` is a value type that gets recreated each render (decoded from JSON, reconstructed from a dictionary), a default-initialized `UUID` property changes with the value. Use a persistent identifier — a database id, a server id, a URL, a slug — that travels with the data.

When the dataset doesn't have a natural id and adding `Identifiable` isn't an option, you can compute a stable id (`id: \.title` for unique titles, a hash of stable fields). That's a last resort, not a default.

## Container view patterns: overlay, background, ZStack

These three are not interchangeable. The frame math is different.

`.overlay(_:alignment:)` places a decoration on top of an existing view. The decoration doesn't add to the host view's measured size — the host stays its own size and the overlay floats above it.

`.background(_:in:)` places a decoration behind. Same rule: it doesn't add to the measured size.

`ZStack` builds a new container whose size is the union of its children's sizes. It's a layout decision, not a decoration.

```swift
// Overlay for a badge that floats on a corner.
// The avatar stays its original size; the checkmark sits over it.
AvatarImage()
    .overlay(alignment: .topTrailing) {
        Image(systemName: "checkmark.circle.fill")
            .foregroundStyle(.green)
    }

// Background for a card surface.
// The card's measured size is the padded content; the material fills behind it.
CardContent()
    .padding(16)
    .background(.regularMaterial, in: .rect(cornerRadius: 16, style: .continuous))

// ZStack when both children are first-class peers.
// The stack is as large as the larger child.
ZStack {
    BackgroundGradient()
    ContentScroll()
}
```

Rule of thumb: if one of the views is "decorating" the other, use overlay or background. If both views deserve to influence the layout size, use ZStack.

## `ViewModifier` vs `View` extension vs new `View`

When you want to package up some styling for reuse, you have three options. The right choice depends on what you're packaging:

| Goal | Tool |
| --- | --- |
| Group several modifiers that always apply together | `ViewModifier` exposed via a `View` extension |
| Single-modifier convenience (e.g. `.hidden(if:)`) | `View` extension alone |
| The component has its own state, layout, or composition | A new `View` struct |
| Reusable styling for a Button or Toggle | `ButtonStyle` or `ToggleStyle` (see `design-system.md`) |

A `ViewModifier`:

```swift
struct CardStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding(16)
            .background(.regularMaterial, in: .rect(cornerRadius: 16, style: .continuous))
            .shadow(color: .black.opacity(0.06), radius: 12, y: 4)
    }
}

extension View {
    func cardStyle() -> some View { modifier(CardStyle()) }
}

// Usage
Text("Hello").cardStyle()
```

The extension exists so the call site reads as a verb on the view.

A simple `View` extension when there's no shared body to factor:

```swift
extension View {
    @ViewBuilder
    func hidden(if condition: Bool) -> some View {
        if condition { hidden() } else { self }
    }
}
```

No `ViewModifier` machinery — it's a one-line conditional.

## Conditional modifier helpers

There's a popular `View.if(condition) { transform }` extension that crops up in Stack Overflow answers and starter templates. It looks convenient. It silently breaks view identity.

```swift
extension View {
    @ViewBuilder
    func iff<V: View>(_ condition: Bool, _ transform: (Self) -> V) -> some View {
        if condition { transform(self) } else { self }
    }
}

Text("Hi").iff(isHighlighted) { $0.bold() }
```

The two branches return different concrete types (`Text` and `Bold<Text>` or similar). SwiftUI sees them as different views. The result: any `@State`, animation context, scroll position, and selection tied to that view drops when `isHighlighted` flips.

The fix is to use the modifier-level conditional where the API supports it:

```swift
Text("Hi").fontWeight(isHighlighted ? .bold : .regular)
```

Most modifiers accept this kind of conditional value. Use it. When you genuinely need a structural change, accept the identity break and write the `if` directly in `body` so the cost is visible.

## Equatable views

For views that take complex inputs and re-render too often, you can conform to `Equatable` and SwiftUI will use your equality to short-circuit. This is most useful for rows in long lists where one or two fields drive the visible output but the row receives a larger object:

```swift
struct ArticleRow: View, Equatable {
    let id: Article.ID
    let title: String
    let publishedAt: Date
    let isUnread: Bool

    static func == (lhs: Self, rhs: Self) -> Bool {
        lhs.id == rhs.id &&
        lhs.title == rhs.title &&
        lhs.publishedAt == rhs.publishedAt &&
        lhs.isUnread == rhs.isUnread
    }

    var body: some View { ... }
}
```

Use this when profiling shows you over-rendering. Don't reach for it preemptively — most views don't benefit, and a wrong `==` can hide bugs by suppressing renders that should have happened.

(The older `EquatableView` wrapper is superseded; conform `Equatable` on the view itself.)

## One purpose per view

Views tend to drift into doing several things at once. A useful frame is to ask which of three roles a view fills:

- **Render**: takes inputs, produces output. `Text`, `Image`, a leaf row.
- **Compose**: arranges other views. A list view that lays out rows.
- **Orchestrate**: owns state, runs `.task`, coordinates side effects. Usually the top of a navigation destination.

Mixing roles is where complexity lives. A row view that also fires its own network calls in `.task` is doing render *and* orchestration. The network call belongs in the parent that owns the list, not in the row. Two reasons: the row gets recycled and reused as you scroll, so its `.task` may fire repeatedly; and the row becomes harder to preview and test because previewing it requires a network stub.

```swift
// Row does too much.
struct ArticleRow: View {
    let articleID: Article.ID
    @State private var article: Article?
    var body: some View {
        Group { article.map { Text($0.title) } }
            .task { article = await fetch(articleID) }
    }
}

// Row renders. The list orchestrates.
struct ArticleListView: View {
    @Environment(ArticleStore.self) private var store
    var body: some View {
        List(store.articles) { ArticleRow(article: $0) }
            .task { await store.refresh() }
    }
}

struct ArticleRow: View {
    let article: Article
    var body: some View { Text(article.title) }
}
```

## File layout

One primary public type per file. Co-locate private helpers in the same file if they're small and not used elsewhere:

```swift
// ArticleRow.swift
struct ArticleRow: View { ... }
private struct UnreadDot: View { ... }
```

When a private helper grows past ~30 lines or starts being used in another file, give it its own file. See `architecture.md` for the broader folder convention.

## Misc composition patterns

A few more that come up in reviews:

- Prefer modifier-level changes (`.opacity`, `.scaleEffect`, `.disabled`) over `if`/`else` branches when the structure doesn't actually change. Modifiers preserve identity; conditional branches don't.
- `Label("Text", systemImage: "icon")` over a hand-built `HStack { Image; Text }`. It's accessible, respects RTL layout, and works with `labelStyle`.
- `Group` is a transparent container, not a layout decision. Use it to apply common modifiers across siblings or return multiple views from a builder.
- `ViewThatFits { HStackVersion(); VStackVersion() }` before reaching for `GeometryReader` when you want adaptive layouts.
- `containerRelativeFrame(.horizontal, count:, span:)` for proportional widths relative to the nearest scrollable container.
- `.visualEffect { content, proxy in ... }` for layout-aware transforms that run on the render thread — much cheaper than computing them in `body`.

## Common review findings

When reviewing for composition issues, watch for:

- Computed properties returning `some View` used as if they were subviews. Replace with real structs.
- `@escaping () -> Content` stored in a container. Resolve in `init` and store the `Content` value.
- `AnyView` anywhere, especially in `ForEach` row bodies.
- `ForEach(id: UUID())` or default `UUID` properties used as ids.
- Long bodies with several disjoint state machines crammed into one view.
- Row views that fetch their own data.
- `.cornerRadius(_:)` and `.foregroundColor(_:)` — soft-deprecated in favor of `.clipShape(.rect(cornerRadius:, style: .continuous))` and `.foregroundStyle(_:)`. See `modern-api.md` for the full replacement table.
- `.glassEffect()` applied to list rows or content tiles. Glass is chrome-only; see `liquid-glass.md`.
- Conditional modifier helpers (`.iff(...)`, `.applying(when:)`). They look helpful and quietly break identity.

When you find one, name the file, the line, and explain briefly what's happening. A one-line before/after is usually enough.
