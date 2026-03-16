# State Management & Data Flow

## Property Wrapper Decision Guide

| Need | Use |
|---|---|
| View-internal state (value types) | `@State private var` |
| View-owned observable object | `@State private var model = MyModel()` (where MyModel is `@Observable`) |
| Binding from parent to child | `@Binding var` (only if child MUTATES; use `let` for read-only) |
| Create bindings to @Observable properties | `@Bindable var model` |
| Read shared state from environment | `@Environment(MyModel.self) var model` |
| User defaults persistence | `@AppStorage("key")` (NEVER inside `@Observable` classes) |
| Scene-specific persistence | `@SceneStorage("key")` |
| SwiftData queries | `@Query var items: [Item]` (only in views) |
| Focus management | `@FocusState var field: Field?` (use Hashable enum for multi-field) |

## @Observable Rules

```swift
@MainActor   // Required unless project uses default MainActor isolation
@Observable
final class MyModel {
    var name = ""           // Tracked — triggers view updates
    var count = 0           // Tracked

    @ObservationIgnored
    var cache: [String] = [] // Not tracked — no view updates

    // @AppStorage CANNOT go here — it won't trigger updates
}
```

### Ownership pattern:
```swift
struct ParentView: View {
    @State private var model = MyModel()  // Owns the model

    var body: some View {
        ChildView(model: model)           // Passes down
    }
}

struct ChildView: View {
    @Bindable var model: MyModel          // Can create bindings

    var body: some View {
        TextField("Name", text: $model.name)
    }
}
```

### Environment injection:
```swift
// In parent
ContentView()
    .environment(myModel)

// In child
@Environment(MyModel.self) private var model
```

## Fine-Grained Observation

Avoid one big model that many views depend on. Each view should depend only on properties it reads.

```swift
// Bad: every LandmarkRow re-renders when ANY favorite changes
struct LandmarkRow: View {
    @Environment(ModelData.self) var modelData  // Reads entire favorites array

    var body: some View {
        // Checks modelData.favorites — creates broad dependency
    }
}

// Good: each row has its own observable with just isFavorite
@Observable class LandmarkViewModel {
    var isFavorite = false
}
```

## @State Rules

- Always `private`.
- Never pass values INTO `@State` from parent — `@State` captures its initial value once and ignores subsequent parent updates.
- `@State` can store non-observable classes as a persistent cache (e.g., `CIContext`).

## Binding Best Practices

- Never use `Binding(get:set:)` in `body`. Use `@State` + `onChange()`:

```swift
// Bad
TextField("Name", text: Binding(
    get: { model.name },
    set: { model.name = $0; model.save() }
))

// Good
TextField("Name", text: $model.name)
    .onChange(of: model.name) { model.save() }
```

- Numeric TextField: use `format:` initializer + keyboard type:

```swift
TextField("Score", value: $score, format: .number)
    .keyboardType(.numberPad)
```

## Identifiable

Prefer `Identifiable` conformance over `id: \.someProperty`:

```swift
// Prefer
struct Item: Identifiable {
    let id = UUID()
    var name: String
}
ForEach(items) { item in ... }

// Avoid
ForEach(items, id: \.name) { item in ... }
```

## SwiftData

- `@Query` only works inside SwiftUI views — not in `@Observable` classes.
- Use `ModelContext.fetchCount()` for counts (won't live-update without `@Query`).
- `@Model` classes get `Identifiable` for free.

### SwiftData + CloudKit rules:
- Never use `@Attribute(.unique)` — incompatible with CloudKit.
- All properties must have default values or be optional.
- All relationships must be optional.
