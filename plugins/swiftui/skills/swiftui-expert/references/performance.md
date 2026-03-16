# Performance

## Two Root Causes (WWDC 2025)

1. **Long view body updates** — body computations take too long, missing frame deadlines
2. **Unnecessary view updates** — too many views update when they shouldn't

## Code Smell Catalog

### 1. Broad Observation Dependencies

```swift
// Bad — every row re-renders when ANY item in the array changes
ForEach(model.items) { item in
    ItemRow(item: item, favorites: model.favorites)  // Reads full array
}

// Good — per-item observable, only affected rows re-render
ForEach(model.items) { item in
    ItemRow(viewModel: item.viewModel)  // Each has own @Observable
}
```

### 2. Expensive Work in Body

```swift
// Bad — sorts on every body evaluation
var body: some View {
    List(items.sorted { $0.date > $1.date }) { item in ... }
}

// Good — compute once via @State or model, not on every body call
@State private var sortedItems: [Item] = []

var body: some View {
    List(sortedItems) { item in ... }
}
.onChange(of: items) { _, newItems in
    sortedItems = newItems.sorted { $0.date > $1.date }
}
```

### 3. Formatter Creation in Body

```swift
// Bad — creates formatter every body call
var body: some View {
    Text(date, formatter: {
        let f = DateFormatter()
        f.dateStyle = .medium
        return f
    }())
}

// Good — use format directly
var body: some View {
    Text(date, format: .dateTime.day().month().year())
}
```

### 4. Computed Properties Instead of Subviews

```swift
// Bad — no independent invalidation
var header: some View { Text(model.title) }

// Good — separate struct, own invalidation boundary
struct HeaderView: View {
    let title: String
    var body: some View { Text(title) }
}
```

### 5. AnyView Type Erasure

```swift
// Bad — destroys type information, prevents diffing optimization
func makeView() -> AnyView {
    if condition {
        return AnyView(ViewA())
    } else {
        return AnyView(ViewB())
    }
}

// Good — preserves type, enables efficient diffing
@ViewBuilder
func makeView() -> some View {
    if condition {
        ViewA()
    } else {
        ViewB()
    }
}
```

### 6. Conditional Branching vs Ternary

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

### 7. Eager Stacks with Large Data

```swift
// Bad — renders all items immediately
ScrollView {
    VStack {
        ForEach(thousandItems) { item in ItemRow(item: item) }
    }
}

// Good — lazy rendering
ScrollView {
    LazyVStack {
        ForEach(thousandItems) { item in ItemRow(item: item) }
    }
}
```

### 8. Unstable ForEach Identity

```swift
// Bad — indices change on insert/delete, causes full re-render
ForEach(items.indices, id: \.self) { index in ... }

// Good — stable identity per item
ForEach(items) { item in ... }  // Requires Identifiable
```

### 9. Heavy View Initializers

```swift
// Bad — work runs before view is even displayed
struct MyView: View {
    @State private var data: [Item]

    init() {
        _data = State(initialValue: loadFromDisk())  // Blocking I/O
    }
}

// Good — defer to task
struct MyView: View {
    @State private var data: [Item] = []

    var body: some View {
        List(data) { ... }
            .task { data = await loadFromDisk() }
    }
}
```

### 10. Escaping @ViewBuilder Closures

```swift
// Less efficient — stores escaping closure
struct Card<Content: View>: View {
    let content: () -> Content
}

// More efficient — stores built view result
struct Card<Content: View>: View {
    @ViewBuilder let content: Content
}
```

## Remediation Patterns

| Problem | Fix |
|---|---|
| Broad @Observable dependency | Per-item observable models |
| Computation in body | Move to `let`, `@State`, or model method |
| Frequent environment reads | Pass only needed values as properties |
| Redundant state updates | Check value before setting: `if newValue != oldValue { state = newValue }` |
| Large lists | `LazyVStack` / `LazyHStack` |
| Async in wrong place | `task()` over `onAppear()` (auto-cancellation) |
| Complex animated views | `.drawingGroup()` to rasterize |
| Opaque scroll backgrounds | `.scrollContentBackground(.visible)` |

## Debugging

- `Self._printChanges()` — prints which properties changed when body re-evaluates
- `Self._logChanges()` — same, logged to console
- Random background colors in grid items — the "disco ball" trick to spot over-invalidation
- Instruments 26: SwiftUI instrument with Cause & Effect graph, Update Groups, color-coded by hitch likelihood

## Off-Main-Thread Closures

Closures in `Shape.path`, `visualEffect`, `Layout`, `onGeometryChange` are `@Sendable` — they must capture values, not access `@MainActor` state directly:

```swift
// Bad — accesses @MainActor state in Sendable closure
.visualEffect { content, proxy in
    content.offset(y: model.offset)  // Compiler error in strict concurrency
}

// Good — capture the value
let currentOffset = model.offset
.visualEffect { content, proxy in
    content.offset(y: currentOffset)
}
```
