# Modern API Migration

Replace deprecated API with modern equivalents. Organized by minimum deployment target.

## Always Use (iOS 15+)

| Deprecated | Modern Replacement |
|---|---|
| `foregroundColor(_:)` | `foregroundStyle(_:)` |
| `cornerRadius(_:)` | `clipShape(.rect(cornerRadius:))` |
| `overlay(_:alignment:)` (deprecated overload) | `overlay(alignment:content:)` or `overlay { }` |
| `NavigationView` | `NavigationStack` or `NavigationSplitView` |
| `NavigationLink(destination:)` | Value-based `NavigationLink` + `navigationDestination(for:)` |
| `.navigationBarLeading` / `.navigationBarTrailing` | `.topBarLeading` / `.topBarTrailing` |
| `UIGraphicsImageRenderer` (in SwiftUI context) | `ImageRenderer` |
| `PreviewProvider` | `#Preview` macro |
| `ObservableObject` / `@Published` / `@StateObject` / `@ObservedObject` / `@EnvironmentObject` | `@Observable` + `@State` / `@Bindable` / `@Environment` |
| `onChange(of:perform:)` (1-parameter) | `onChange(of:) { }` (0-param) or `onChange(of:) { old, new in }` (2-param) |
| `tabItem()` | `Tab` API |
| `showsIndicators: false` in ScrollView | `.scrollIndicators(.hidden)` |
| `Task.sleep(nanoseconds:)` | `Task.sleep(for: .seconds(1))` |
| `String(format: "%.2f", value)` | `Text(value, format: .number.precision(.fractionLength(2)))` |
| `Date()` | `Date.now` |
| `filter().count` | `count(where:)` |
| `Image("name")` | `Image(.name)` (generated asset symbols) |
| `DispatchQueue.main.async` | `@MainActor` or `MainActor.run` |
| `GeometryReader` (when reading size only) | `containerRelativeFrame()` or `visualEffect()` |
| Manual `EnvironmentKey` + extension | `@Entry` macro |
| UIKit haptics (`UIImpactFeedbackGenerator`) | `sensoryFeedback()` |
| `animation()` without value | `.animation(_:value:)` — always provide value |

## iOS 17+

| Feature | Notes |
|---|---|
| `@Observable` macro | Replaces `ObservableObject`. Fine-grained invalidation. |
| `@Bindable` | For creating bindings to `@Observable` properties |
| Fill + stroke without overlay | Chain `.fill()` then `.stroke()` directly |
| `ContentUnavailableView` | For empty states. `.search` variant auto-includes search term. |
| `containerRelativeFrame()` | Replace many `GeometryReader` uses |
| `visualEffect()` | Replace `GeometryReader` for visual transforms |
| `.phaseAnimator` | Multi-step animation sequences |
| `.keyframeAnimator` | Precise keyframe timing control |
| Automatic grammar agreement | `Text("^[\(count) item](inflect: true)")` |
| `.sensoryFeedback()` | Native haptic feedback |
| Inspector | `.inspector(isPresented:)` for trailing supplementary panels |

## iOS 18+

| Feature | Notes |
|---|---|
| `@Entry` macro | Simplifies custom environment/focus/transaction/container keys |
| `.scrollPosition(id:)` | Programmatic scroll position tracking |
| `MeshGradient` | Complex multi-point gradients |
| Tab section customization | `TabSection` for grouped tabs |

## iOS 26+

| Feature | Notes |
|---|---|
| `WebView` (import WebKit) | Replaces `UIViewRepresentable` WKWebView wrappers |
| `.glassEffect()` | Liquid Glass material |
| `GlassEffectContainer` | Groups multiple glass elements |
| `.buttonStyle(.glass)` / `.glassProminent` | Glass button styles |
| `glassEffectID` + `@Namespace` | Morphing transitions between glass elements |
| `.scrollEdgeEffectStyle(.soft)` | Auto-blur content under toolbars |
| `.backgroundExtensionEffect()` | Extend content behind glass chrome |
| `.tabBarMinimizeBehavior(.onScrollDown)` | Auto-hide tab bar on scroll |
| `.tabViewBottomAccessory` | "Now Playing" style accessory above tab bar |
| `Tab(role: .search)` | Dedicated search tab |
| `ToolbarSpacer(id:)` | Split toolbar items into visual groups |
| `.searchToolbarBehavior(.minimizable)` | Collapsible search in toolbar |
| `navigationZoomTransition` | Sheet morphs from source element |
| `.controlSize(.extraLarge)` | Extra-large control variant |
| `TextEditor` with `AttributedString` | Rich text editing |
| `@Animatable` macro | Auto-synthesizes `animatableData` |
| `@AnimatableIgnored` | Exclude properties from animation |
| `.font(.body.scaled(by:))` | Dynamic Type scaling alternative to `@ScaledMetric` |
| Scene bridging | Bridge UIKit/AppKit scenes to SwiftUI |

## Text Concatenation

Prefer `Text` interpolation over `+` concatenation for readability:

```swift
// Bad
Text("Hello").foregroundStyle(.red) + Text("World").foregroundStyle(.blue)

// Good
let red = Text("Hello").foregroundStyle(.red)
let blue = Text("World").foregroundStyle(.blue)
Text("\(red)\(blue)")
```

## ObservableObject Escape Hatch

If `ObservableObject` is absolutely required (e.g., Combine debouncer), always add `import Combine` explicitly — it is no longer auto-imported through SwiftUI.

## ForEach Over Enumerated

`enumerated()` returns `EnumeratedSequence` which doesn't conform to `RandomAccessCollection`, so wrap in `Array()`:

```swift
ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
    Text("\(index): \(item.name)")
}
```

For stable identity, prefer `Identifiable` conformance and avoid relying on indices for identity in dynamic lists.
