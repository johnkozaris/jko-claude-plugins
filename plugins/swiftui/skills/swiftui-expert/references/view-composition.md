# View Composition & Clean Code

## The Golden Rule

**Strongly prefer separate `View` structs over computed properties or methods returning `some View`.** Computed properties do not benefit from `@Observable`'s fine-grained invalidation — they re-evaluate with the parent every time. Separate structs let SwiftUI skip unchanged subviews. Computed properties are acceptable only for trivial, stateless decomposition where performance is not a concern.

```swift
// Bad — computed property means no independent invalidation
var headerView: some View {
    Text(model.title).font(.headline)
}

// Good — separate struct, own file, independent invalidation
struct HeaderView: View {
    let title: String
    var body: some View {
        Text(title).font(.headline)
    }
}
```

## View Ordering Convention

Follow this consistent ordering within every view struct:

```swift
struct MyView: View {
    // 1. Environment
    @Environment(\.dismiss) private var dismiss
    @Environment(MyModel.self) private var model

    // 2. State & bindings
    @State private var isEditing = false
    @Binding var selection: Item?

    // 3. Let properties (injected data)
    let title: String

    // 4. Init (only if custom logic needed)

    // 5. Body
    var body: some View { ... }

    // 6. Private helper methods (actions, not views)
    private func save() { ... }
}
```

## When to Extract

| Signal | Action |
|---|---|
| `body` exceeds ~40 lines | Extract subviews |
| Same layout appears 2+ times | Extract into reusable `View` |
| Same styling appears 3+ times | Create `ViewModifier` |
| Button has non-trivial action | Extract into method |
| Logic in `task()`/`onAppear()` | Move to model |

## DRY with ViewModifier

```swift
// Define
struct CardStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding()
            .background(.ultraThinMaterial)
            .clipShape(.rect(cornerRadius: 12))
    }
}

// Extension for discoverability
extension View {
    func cardStyle() -> some View {
        modifier(CardStyle())
    }
}

// Use
Text("Hello").cardStyle()
```

### When to use ViewModifier vs View extension vs new View:

- **ViewModifier**: grouping multiple modifiers that always apply together
- **View extension**: simple single-modifier convenience (e.g., `.hidden(if:)`)
- **New View struct**: when the component has its own state or layout

## Single Responsibility

Each view does ONE thing:

```swift
// Bad — list + detail + empty state all in one
struct ContentView: View {
    var body: some View {
        if items.isEmpty {
            ContentUnavailableView("No Items", systemImage: "tray")
        } else {
            List(items) { item in
                // 50 lines of row layout...
                // 20 lines of swipe actions...
            }
        }
    }
}

// Good — separated concerns
struct ContentView: View {
    var body: some View {
        if items.isEmpty {
            EmptyItemsView()
        } else {
            ItemListView(items: items)
        }
    }
}
```

## Open/Closed Principle

Extend behavior through protocols and extensions:

```swift
// Protocol for any view that can be bookmarked
protocol Bookmarkable {
    var isBookmarked: Bool { get set }
}

// Extension adds the UI behavior without modifying existing types
extension View {
    func bookmarkOverlay(_ item: some Bookmarkable) -> some View {
        overlay(alignment: .topTrailing) {
            if item.isBookmarked {
                Image(systemName: "bookmark.fill")
                    .foregroundStyle(.yellow)
            }
        }
    }
}
```

## Container Views

Use `@ViewBuilder let` for content, not stored closures:

```swift
// Preferred: stores the built view value
struct Card<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading) { content }
            .cardStyle()
    }
}

// Anti-pattern: stores an escaping closure
struct Card<Content: View>: View {
    let content: () -> Content  // Escaping closure — worse for performance
}
```

## Composition Patterns

- Use `overlay`/`background` for decoration (badges, borders, shadows).
- Use `ZStack` for peer views that coexist in the same space.
- Use `.compositingGroup()` before `.clipShape()` on layered views to avoid antialiasing fringes.
- Prefer `Label("Text", systemImage: "icon")` over `HStack { Image; Text }`.
- Prefer modifiers over conditional views for state changes (preserves view identity).

## Anti-Patterns

- **Conditional modifier extensions** that use `if` — these break view identity by changing return types.
- **Storing formatters** as properties — use `Text(date, format:)` directly.
- **Business logic in body** — move to model/method.
- **Multiple types in one file** — one type per file.
- **`AnyView`** — use `@ViewBuilder`, `Group`, or generics.
