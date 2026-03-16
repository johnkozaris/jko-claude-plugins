# Navigation & Presentation

## NavigationStack

The primary navigation container. Always use over deprecated `NavigationView`.

```swift
@State private var path = NavigationPath()

NavigationStack(path: $path) {
    List(items) { item in
        NavigationLink(value: item) {
            ItemRow(item: item)
        }
    }
    .navigationDestination(for: Item.self) { item in
        ItemDetail(item: item)
    }
    .navigationTitle("Items")
}
```

### Rules

- Use value-based `NavigationLink` + `.navigationDestination(for:)`.
- Flag all use of `NavigationLink(destination:)` — deprecated pattern.
- Never mix `navigationDestination(for:)` and `NavigationLink(destination:)` in the same hierarchy.
- Register `navigationDestination(for:)` once per data type — flag duplicates.
- Use type-safe route enums for complex navigation:

```swift
enum Route: Hashable {
    case detail(Item)
    case settings
    case profile(User)
}
```

## NavigationSplitView

For sidebar-driven multi-column layouts (iPad, Mac):

```swift
NavigationSplitView {
    SidebarView(selection: $selection)
} content: {
    ContentListView(selection: selection)
} detail: {
    DetailView(item: selectedItem)
}
```

Two-column variant omits `content:`.

## Sheets

### Prefer item-based sheets

```swift
// Good — safe optional unwrapping
@State private var selectedItem: Item?

.sheet(item: $selectedItem) { item in
    ItemEditor(item: item)
}

// Even better — shorthand when view takes item as init param
.sheet(item: $selectedItem, content: ItemEditor.init)
```

### Multiple sheet types with enum

```swift
enum SheetType: Identifiable {
    case edit(Item)
    case create
    case settings

    var id: String {
        switch self {
        case .edit(let item): "edit-\(item.id)"
        case .create: "create"
        case .settings: "settings"
        }
    }
}

@State private var activeSheet: SheetType?

.sheet(item: $activeSheet) { sheet in
    switch sheet {
    case .edit(let item): EditView(item: item)
    case .create: CreateView()
    case .settings: SettingsView()
    }
}
```

### Sheet rules

- Sheets should manage their own dismiss via `@Environment(\.dismiss)`.
- On iOS 26, partial-height sheets get Liquid Glass background by default — remove custom `presentationBackground` to let the glass show.
- Use `.presentationSizing(.form)` or `.presentationSizing(.page)` for standard sheet sizes.

## Alerts & Confirmation Dialogs

```swift
// Alert with no-action dismiss — omit the button
.alert("Operation Complete", isPresented: $showAlert) { }

// Confirmation dialog — attach to triggering UI for Liquid Glass animation
Button("Delete") { showConfirmation = true }
    .confirmationDialog("Delete Item?", isPresented: $showConfirmation) {
        Button("Delete", role: .destructive) { delete() }
    }
```

## Inspector (iOS 17+)

Trailing-edge supplementary panel:

```swift
.inspector(isPresented: $showInspector) {
    InspectorView(item: selectedItem)
        .inspectorColumnWidth(min: 200, ideal: 300, max: 400)
}
```

## TabView

Use the `Tab` API (not `tabItem()`):

```swift
enum AppTab: Hashable {
    case home, search, profile
}

@State private var selectedTab: AppTab = .home

TabView(selection: $selectedTab) {
    Tab("Home", systemImage: "house", value: .home) {
        HomeView()
    }
    Tab("Search", systemImage: "magnifyingglass", value: .search) {
        SearchView()
    }
    Tab(role: .search) {  // iOS 26: dedicated search tab
        SearchView()
    }
    Tab("Profile", systemImage: "person", value: .profile) {
        ProfileView()
    }
}
```

### iOS 26 Tab Features

- `.tabBarMinimizeBehavior(.onScrollDown)` — auto-hide tab bar on scroll
- `.tabViewBottomAccessory` — "Now Playing" style accessory above tab bar
- `Tab(role: .search)` — dedicated search tab

## Deep Links

Handle deep links via `.onOpenURL`:

```swift
.onOpenURL { url in
    guard let route = Route(from: url) else { return }
    path.append(route)
}
```

Use `NavigationPath` for programmatic navigation control.
