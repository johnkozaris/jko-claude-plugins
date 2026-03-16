# Liquid Glass (iOS 26+)

Apple's most significant design evolution since iOS 7. A translucent, dynamic material that reflects and refracts content using real-time light bending (lensing).

## Automatic Adoption

Recompile with Xcode 26 and these adopt Liquid Glass automatically:
- NavigationBar, TabBar, Toolbar
- Sheets, Popovers, Menus, Alerts
- Search bars, Control Center
- Toggles, Sliders, Pickers (during interaction)

Remove custom `presentationBackground` from sheets to let glass show.

## Core Principle

**Glass on controls, NOT content.** Content sits at the bottom; glass controls float on top.

## Basic Usage

```swift
if #available(iOS 26, *) {
    Text("Hello")
        .padding()
        .glassEffect(.regular, in: .rect(cornerRadius: 16))
} else {
    Text("Hello")
        .padding()
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 16))
}
```

### Modifier Order

Apply `.glassEffect()` **last** — after layout and visual modifiers:

```swift
Text("Label")
    .font(.headline)           // 1. Typography
    .padding()                 // 2. Spacing
    .frame(minWidth: 100)      // 3. Layout
    .glassEffect(.regular)     // 4. Glass LAST
```

## Styles

| Style | Use Case |
|---|---|
| `.regular` | Default glass for most surfaces |
| `.prominent` | Emphasized elements that need more visual weight |

### Modifiers on styles

```swift
.glassEffect(.regular.tint(.blue))           // Tinted glass
.glassEffect(.regular.interactive())          // Responds to touch/pointer
.glassEffect(.prominent.tint(.red).interactive())  // Combined
```

- `.interactive()` — only on tappable or focusable elements
- `.tint()` — semantic coloring for the glass

## GlassEffectContainer

Required when multiple glass elements coexist — **glass cannot sample other glass**:

```swift
GlassEffectContainer(spacing: 24) {
    HStack(spacing: 24) {
        Button("Edit", systemImage: "pencil") { }
            .glassEffect()
        Button("Share", systemImage: "square.and.arrow.up") { }
            .glassEffect()
    }
}
```

The container merges overlapping glass shapes, applies consistent blur/lighting, and enables smooth morphing between elements.

## Button Styles

```swift
Button("Confirm") { }
    .buttonStyle(.glass)

Button("Delete", role: .destructive) { }
    .buttonStyle(.glassProminent)
```

## Morphing Transitions

Glass elements can morph between states using `glassEffectID`:

```swift
@Namespace private var glassNS

// Collapsed state
Text("Summary")
    .glassEffect(in: .capsule)
    .glassEffectID("panel", in: glassNS)

// Expanded state (after animation)
DetailPanel()
    .glassEffect(in: .rect(cornerRadius: 20))
    .glassEffectID("panel", in: glassNS)
```

### Sheet morphing from toolbar

```swift
.toolbar {
    ToolbarItem {
        Button("Details") { showSheet = true }
            .navigationZoomTransition(sourceID: "details", in: glassNS)
    }
}
.sheet(isPresented: $showSheet) {
    DetailSheet()
        .navigationZoomTransition(sourceID: "details", in: glassNS)
}
```

## Distant Control Grouping

For controls too far apart to morph naturally, use `glassEffectUnion`:

```swift
@Namespace private var unionNS

Button("Left") { }
    .glassEffect()
    .glassEffectUnion(id: "group", in: unionNS)

// ... other content in between ...

Button("Right") { }
    .glassEffect()
    .glassEffectUnion(id: "group", in: unionNS)
```

## Toolbars

```swift
.toolbar {
    ToolbarItem(placement: .topBarTrailing) {
        Button("Add", systemImage: "plus") { }
    }

    // iOS 26: split actions into visual groups
    ToolbarSpacer(.fixed)

    ToolbarItem(placement: .topBarTrailing) {
        Button("Settings", systemImage: "gear") { }
    }
}
```

Toolbar icons use monochrome rendering mode by default in Liquid Glass.

## Scroll Edge Effects

```swift
// Auto-blur content scrolling under toolbar glass
.scrollEdgeEffectStyle(.soft, for: .top)

// Extend content behind glass chrome
.backgroundExtensionEffect()
```

## Tab View Accessories

```swift
TabView {
    // tabs...
}
.tabViewBottomAccessory {
    NowPlayingBar()
}
```

## Availability Gating

Always gate and provide fallback:

```swift
if #available(iOS 26, *) {
    content.glassEffect(.regular, in: .capsule)
} else {
    content.background(.ultraThinMaterial, in: Capsule())
}
```

## Design System Notes

- Only adopt Liquid Glass when explicitly requested or when the project targets iOS 26+.
- Glass should feel lightweight — don't overuse it.
- Keep shapes consistent across related elements.
- Apple gives a one-year grace period to adopt — apps can disable Liquid Glass on iOS 26 temporarily.
