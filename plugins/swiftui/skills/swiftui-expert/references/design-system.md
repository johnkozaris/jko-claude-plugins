# Design System

The design-token layer that sits *under* every component and *above* raw `Color` / `Font` / `CGFloat` literals. Target: Swift 6.3 / iOS 26 / Xcode 26.

Once an app crosses about three screens, raw values stop being maintainable. A token layer is not optional architecture — it is the only way to survive rebrands, dark mode, high-contrast, Dynamic Type, Liquid Glass, and accessibility audits without a rewrite.

---

## One source of truth (lead with this)

- All design tokens live in a `DesignSystem` SPM module or top-level namespace.
- Feature views import `DesignSystem` and reference *only* its semantic tokens — never raw `Color(red:...)`, raw `Font.system(size:)`, raw `CGFloat` literals, raw `cornerRadius: 14`.
- `DesignSystem` depends on **nothing** except SwiftUI / UIKit. Feature targets depend on `DesignSystem`. Never the reverse.

### Trigger to create a DesignSystem module

Always create one when ANY of:

- The app is multi-screen and color / spacing / radius values are starting to repeat across files.
- Inline hex literals appear in more than one feature.
- Spacing or radius drift exists — e.g. `cornerRadius: 14` in one file next to `cornerRadius: 16` in another.
- A ButtonStyle (or any `*Style`) gets re-implemented inline more than once.

Skip when:
- Single-screen utility (e.g. a menu-bar tool with one window).
- POC with very few screens and short expected lifetime.

A dedicated `DesignSystem` SPM package is the common shape across popular maintained SwiftUI OSS apps once they grow past a handful of screens. IceCubesApp and IcySky are the canonical reference patterns.

---

## Color tokens

### Three sources of color (use in this order)

1. **Semantic system colors** — `.primary`, `.secondary`, `Color(.systemBackground)`, `Color(.label)`, `Color(.separator)`. Already adapt to light/dark, high-contrast, and elevated material backgrounds. Always first choice for non-brand UI.
2. **Asset catalog color sets** — for every brand and product-specific color. Define `Any`, `Dark`, `Any High Contrast`, `Dark High Contrast` appearances in the same set. The only mechanism that delivers free accessibility variants.
3. **Code-defined `Color` or `ShapeStyle`** — only for runtime-derived colors (server-driven theme, user-pickable accent). Never for static brand palette.

### Rules

- **Semantic naming, not appearance naming.** `.background.primary`, `.label.secondary`, `.accent.brand`, `.status.success`, `.surface.elevated` — never `.lightGray`, `.offWhite`, `.darkBlue`.
- **Two-level naming.** Primitive layer (`brand.indigo.500`) → semantic layer (`label.primary` = `brand.gray.900` in light, `brand.gray.50` in dark). Views reference *only* the semantic layer.
- **Never use `Color.white` / `Color.black` directly.** Absolute colors silently break dark mode. Use `Color(.systemBackground)`, `.primary`, or an asset-catalog token.
- **Never hex-literal a brand color inside a view.** All hex stays inside the asset catalog or a single `Color` / `ShapeStyle` extension that wraps an asset name.
- **`.foregroundStyle(_:)` not `.foregroundColor(_:)`.** `foregroundColor` is soft-deprecated since iOS 17. `foregroundStyle` takes any `ShapeStyle`, including gradients, materials, and custom themed styles.
- **`.tint(_:)` not `.accentColor(_:)`.** `accentColor` is deprecated since iOS 16. `tint` is the current app-wide accent and the *only* one Liquid Glass respects for tinted interactive states.
- **Set the global accent in the asset catalog** (`AccentColor` color set) so previews, launch screen, and SwiftUI all read it.

### `ThemedColor: ShapeStyle` — runtime theming the modern way

When the palette is not statically known (themed app, user-selected accent, server-driven brand), wrap the theme in a `ShapeStyle` and let SwiftUI re-resolve it on environment changes.

```swift
// Sources/DesignSystem/Colors/ColorRole.swift
public enum ColorRole {
    case labelPrimary
    case labelSecondary
    case labelTertiary
    case backgroundPrimary
    case backgroundSecondary
    case surfaceElevated
    case accentBrand
    case statusSuccess
    case statusWarning
    case statusError
}

// Sources/DesignSystem/Colors/ThemedColor.swift
public struct ThemedColor: ShapeStyle {
    let role: ColorRole

    public func resolve(in environment: EnvironmentValues) -> some ShapeStyle {
        environment.theme.color(
            for: role,
            scheme: environment.colorScheme,
            contrast: environment.colorSchemeContrast
        )
    }
}

public extension ShapeStyle where Self == ThemedColor {
    static func theme(_ role: ColorRole) -> ThemedColor { .init(role: role) }
}
```

Usage in a feature view:

```swift
Text("Hello")
    .foregroundStyle(.theme(.labelPrimary))

RoundedRectangle(cornerRadius: Radius.card, style: .continuous)
    .fill(.theme(.surfaceElevated))

Image(systemName: "checkmark.circle.fill")
    .foregroundStyle(.theme(.statusSuccess))
```

Why this beats `Color` extensions: `resolve(in:)` runs whenever `EnvironmentValues` change, so dark-mode toggle, high-contrast toggle, *and* runtime theme swap all just work — no view body to re-author.

### Dark mode and contrast

- The asset catalog handles `light`, `dark`, plus `Any High Contrast` and `Dark High Contrast`. Always define all four for brand colors.
- Read scheme with `@Environment(\.colorScheme)` only when behavior differs (e.g. swapping an image asset, choosing a material). Color choice should be data-driven via tokens, never branching `if colorScheme == .dark`.
- Read contrast with `@Environment(\.colorSchemeContrast)`. The system handles the heavy lifting if you supply high-contrast variants in the catalog.
- Never `.preferredColorScheme(.dark)` at the app root without an explicit user setting — it stomps the system preference.

---

## Typography ramp

### Three font sources

1. **System SF Pro / SF Pro Rounded / New York** — via `Font.TextStyle` (`.body`, `.headline`, `.largeTitle`, etc.). Free Dynamic Type, free localized metrics, free weight axes.
2. **Custom fonts (brand)** — registered via Info.plist's `UIAppFonts` and used with `.custom(_:size:relativeTo:)` so they still respect Dynamic Type.
3. **Variable fonts (.ttf with weight/optical-size axes)** — use the named font family and adjust via `.fontWeight(_:)` rather than registering every static cut. Available on iOS 16+.

### Rules

- **Always anchor to a `Font.TextStyle` ramp.** Never `Font.system(size: 16)` raw — it kills Dynamic Type.
- **For custom fonts, always pass `relativeTo:`.** `Font.custom("Inter-Regular", size: 17, relativeTo: .body)` scales with the user's Dynamic Type setting.
- **Centralize the type ramp.** Single `AppFont` namespace; views call `.font(.app(.title))`, never `.font(.system(...))` directly.
- **Scale non-text metrics with `@ScaledMetric`.** Padding, icon size, badge offsets that must grow with type get `@ScaledMetric(relativeTo: .body) var iconSize: CGFloat = 24`.
- **Always provide `.minimumScaleFactor(_:)` floor** (0.8 is typical) for fixed-width contexts so AX5 text doesn't truncate to ellipsis.
- **Test at xSmall and AX5.** Both extremes; AX5 changes layout, xSmall reveals overdrawn hit targets.
- **Use `bold()` not `fontWeight(.bold)`** — lets the system pick the correct weight per context.

### Standard system text-style scale

| Style          | Default Size | Usage                              |
|----------------|--------------|------------------------------------|
| `.largeTitle`  | 34pt         | Screen titles, hero text           |
| `.title`       | 28pt         | Section headers                    |
| `.title2`      | 22pt         | Subsection headers                 |
| `.title3`      | 20pt         | Card titles                        |
| `.headline`    | 17pt bold    | Emphasized body text               |
| `.body`        | 17pt         | Primary content                    |
| `.callout`     | 16pt         | Supporting explanations            |
| `.subheadline` | 15pt         | Secondary labels                   |
| `.footnote`    | 13pt         | Timestamps, metadata               |
| `.caption`     | 12pt         | Tertiary info (use sparingly)      |
| `.caption2`    | 11pt         | Avoid — extremely small            |

### Custom font registration

1. Add the font file (`Inter-Regular.ttf`) to a `Resources/Fonts/` folder.
2. Add the file to `UIAppFonts` in Info.plist (the file name, e.g. `Inter-Regular.ttf`).
3. Use via the centralized namespace.

```swift
// Sources/DesignSystem/Typography/AppFont.swift
public enum AppTextStyle {
    case largeTitle, title, headline, body, callout, footnote, caption
}

public enum AppFont {
    public static func font(for style: AppTextStyle) -> Font {
        switch style {
        case .largeTitle: .custom("Inter-Bold",     size: 34, relativeTo: .largeTitle)
        case .title:      .custom("Inter-Bold",     size: 28, relativeTo: .title)
        case .headline:   .custom("Inter-Semibold", size: 17, relativeTo: .headline)
        case .body:       .custom("Inter-Regular",  size: 17, relativeTo: .body)
        case .callout:    .custom("Inter-Regular",  size: 16, relativeTo: .callout)
        case .footnote:   .custom("Inter-Regular",  size: 13, relativeTo: .footnote)
        case .caption:    .custom("Inter-Regular",  size: 12, relativeTo: .caption)
        }
    }
}

// Sources/DesignSystem/Typography/Font+App.swift
public extension Font {
    static func app(_ style: AppTextStyle) -> Font { AppFont.font(for: style) }
}
```

Usage:

```swift
Text("Welcome")
    .font(.app(.largeTitle))
    .foregroundStyle(.theme(.labelPrimary))

Text("Long description that must reflow at AX5.")
    .font(.app(.body))
    .minimumScaleFactor(0.8)
    .lineLimit(3)
```

### Variable font axes (iOS 16+)

Variable fonts ship a single file with continuous weight / width / optical-size axes. Reference the family name once and tune via `.fontWeight(_:)` and `.fontWidth(_:)` instead of registering every cut.

```swift
Text("Variable")
    .font(.custom("Inter", size: 17, relativeTo: .body))
    .fontWeight(.semibold)
    .fontWidth(.condensed)  // iOS 16+
```

### Brand fonts vs system fonts: when each

- **System fonts (SF Pro / SF Rounded / NY):** chrome (nav, toolbar, tab labels, system buttons), numerics (use monospaced digits), long-form reading. Free localized metrics in every Apple-supported script.
- **Brand fonts:** display copy, marketing surfaces, hero titles, distinctive editorial moments. Never use brand fonts inside the system toolbar/tab bar in iOS 26 — it fights Liquid Glass material vibrancy and degrades AX legibility.
- **Mixed strategy is the norm:** brand display + SF body. Pair them in a single `AppFont` so the mixing rule is centralized.

---

## Spacing scale

```swift
// Sources/DesignSystem/Spacing.swift
public enum Spacing {
    public static let xxxs: CGFloat = 2   // hairline gutters only
    public static let xxs:  CGFloat = 4
    public static let xs:   CGFloat = 8   // base unit
    public static let sm:   CGFloat = 12
    public static let md:   CGFloat = 16  // most-common padding
    public static let lg:   CGFloat = 24  // section gaps
    public static let xl:   CGFloat = 32
    public static let xxl:  CGFloat = 48
    public static let xxxl: CGFloat = 64
}
```

### Rules

- **Every padding/spacing literal in views must be a token.** `.padding(16)` is a smell — use `.padding(Spacing.md)`.
- **Smaller token = tighter relationship.** Adjacent labels in a row: `xs`. Sections in a list: `lg`. Major page breaks: `xl`+.
- **Use Apple's `.padding()` no-arg form for system defaults** (it adapts per-platform). Only specify a value when the design demands it.
- **Stay on the grid.** All padding, gap, and offset values are members of `Spacing`. The token enum is the *only* place new values are added — adding `Spacing.md17` is the failure mode.
- **No negative padding.** If you need negative offsets, you have a layout bug; switch to `alignmentGuide` or `Layout` protocol.
- **Prefer flexible frames over fixed frames** so layout adapts to device size and Dynamic Type.
- **Never `UIScreen.main.bounds`.** Use `containerRelativeFrame()` or `GeometryReader` only as last resort.

---

## Radius scale

```swift
// Sources/DesignSystem/Radius.swift
public enum Radius {
    public static let none:    CGFloat = 0
    public static let chip:    CGFloat = 6     // chip, inline pill
    public static let button:  CGFloat = 10    // standard button
    public static let input:   CGFloat = 12    // text field
    public static let card:    CGFloat = 16    // surface card (most common)
    public static let modal:   CGFloat = 20
    public static let sheet:   CGFloat = 24    // matches system sheet radius
    public static let hero:    CGFloat = 32
}
```

### Rules

- **One radius per role.** `Radius.card` is *the* card radius. Never `cornerRadius: 14` next to `cornerRadius: 16` — that drift is the #1 visible "old codebase" smell.
- **`.clipShape(.rect(cornerRadius: Radius.card, style: .continuous))`** — never the deprecated `.cornerRadius(_:)` modifier.
- **Continuous corners by default.** `RoundedRectangle(cornerRadius: Radius.card, style: .continuous)` or the shorthand `.rect(cornerRadius: Radius.card, style: .continuous)`. Circular corners look dated and clash with system shapes.
- **iOS 26: prefer `ConcentricRectangle`** for nested cards inside a glass container. Concentric corners share a center with the parent's curvature and look optically correct as views resize.
- **Use `Capsule()` not `RoundedRectangle(cornerRadius: .infinity)`.** Semantic, no magic number.
- **Asymmetric corners via `UnevenRoundedRectangle`** (iOS 17+) — never via clip-mask hacks.
- **Tokens for stroke width too.**

```swift
public enum Stroke {
    public static let hairline: CGFloat = 1 / UIScreen.main.scale
    public static let thin:     CGFloat = 1
    public static let medium:   CGFloat = 2
}
```

Usage:

```swift
RoundedRectangle(cornerRadius: Radius.card, style: .continuous)
    .fill(.theme(.surfaceElevated))

// Or the modern shorthand
SomeView()
    .clipShape(.rect(cornerRadius: Radius.card, style: .continuous))
```

---

## Motion tokens

```swift
// Sources/DesignSystem/Motion.swift
public enum Motion {
    // Named springs (iOS 17+) — preferred
    public static let smooth: Animation = .smooth(duration: 0.30)    // no bounce
    public static let snappy: Animation = .snappy(duration: 0.30)    // small bounce
    public static let bouncy: Animation = .bouncy(duration: 0.45)    // playful

    // Linear / eased — only for non-physical motion
    public static let fadeIn:  Animation = .easeOut(duration: 0.20)
    public static let fadeOut: Animation = .easeIn(duration: 0.15)

    // Loading / progress — must be linear
    public static let progress: Animation =
        .linear(duration: 1.0).repeatForever(autoreverses: false)
}
```

### Rules

- **Prefer springs.** `.smooth` for content moves, `.snappy` for interactive UI, `.bouncy` only when the action is celebratory.
- **No bouncy progress.** Loaders, spinners, progress bars use linear.
- **Name the curve at the call site.** `.animation(Motion.snappy, value: state)` — not `.spring(response: 0.32, dampingFraction: 0.7)` inline.
- **Tune via parameters of the named curves**, not by inventing custom `interpolatingSpring`s. `.snappy(extraBounce: 0.1)` keeps the family.
- **Respect `Reduce Motion`.** `@Environment(\.accessibilityReduceMotion)` should swap to `.linear(duration: 0.15)` or `nil`. Bake into the token.
- **Use the new `.symbolEffect(...)` and `.contentTransition(_:)`** for symbol/number/text crossfades instead of manual opacity hacks.

Reduce-Motion-aware accessor:

```swift
public extension Motion {
    static func snappy(reduceMotion: Bool) -> Animation {
        reduceMotion ? .linear(duration: 0.15) : .snappy(duration: 0.30)
    }
}

// Usage
@Environment(\.accessibilityReduceMotion) private var reduceMotion

.animation(Motion.snappy(reduceMotion: reduceMotion), value: isExpanded)
```

---

## SF Symbols

### Rendering modes

- **`.monochrome` (default)** — chrome, list rows, toolbar.
- **`.hierarchical`** — single-tint hero icons that need depth. `Image(systemName: "...").symbolRenderingMode(.hierarchical).foregroundStyle(.theme(.accentBrand))`.
- **`.palette`** — two- or three-tone iconography. Pass colors in layer order to `.foregroundStyle(_:_:_:)`.
- **`.multicolor`** — domain icons (weather, badges) that have semantic meaning baked in. Adapts to dark mode and vibrancy automatically.

### Variable values for progress / state

```swift
Image(systemName: "speaker.wave.3", variableValue: volume)  // 0...1
```

### Variants for consistency

```swift
Image(systemName: "heart")
    .symbolVariant(.fill)        // heart.fill
    .symbolVariant(.circle.fill) // heart.circle.fill
```

### Symbol effects (iOS 17+) — animation tokens

```swift
Image(systemName: "bell")
    .symbolEffect(.bounce, value: notificationCount)

Image(systemName: "arrow.triangle.2.circlepath")
    .symbolEffect(.pulse)

Image(systemName: "checkmark.circle.fill")
    .symbolEffect(.bounce.up.byLayer, value: didConfirm)
```

Centralize the most-used effects:

```swift
public enum SymbolEffectToken {
    public static let alert: SymbolEffect = .bounce
    public static let loading: SymbolEffect = .pulse
    public static let success: SymbolEffect = .bounce.up.byLayer
}
```

### Rules

- **SF Symbols for every utility icon.** They scale with Dynamic Type and inherit `.foregroundStyle`.
- **Pick a rendering mode deliberately** — don't leave it at the default and hope.
- **Custom symbols** ship as `.symbolset` files (SF Symbols app exports) and are referenced by name like Apple's.
- **Never mix glyph fonts and SF Symbols** in the same surface (e.g. FontAwesome + SF). One iconography source per app.
- **Never `Image.foregroundColor(.red)` on multi-layer SF Symbols.** Pick a rendering mode and pass a `ShapeStyle` to `.foregroundStyle`.

---

## `@Entry` macro for environment values

The `@Entry` macro (iOS 18+, back-deploys via the macro itself) generates the `EnvironmentKey`, default value, and computed `EnvironmentValues` property in one line — replacing ~12 lines of EnvironmentKey + extension + default boilerplate.

### Before (the old boilerplate)

```swift
private struct ThemeKey: EnvironmentKey {
    static let defaultValue: AppTheme = .default
}

extension EnvironmentValues {
    var theme: AppTheme {
        get { self[ThemeKey.self] }
        set { self[ThemeKey.self] = newValue }
    }
}
```

### After (`@Entry`)

```swift
// Sources/DesignSystem/Environment/Theme+Entry.swift
extension EnvironmentValues {
    @Entry public var theme: AppTheme = .default
    @Entry public var spacing: SpacingScale = .default
    @Entry public var symbolEffects: SymbolEffectToken.Set = .standard
}
```

Use for the *theme manager*, not for individual primitives — passing `xs/sm/md/lg` through environment is overkill; static enum values are fine.

### Reference-type caveat

`@Entry`'s default is re-created during environment preparation. For an `@Observable ThemeManager` you mutate, **inject explicitly at scene root** so SwiftUI doesn't manufacture duplicate state:

```swift
@main
struct MyApp: App {
    @State private var theme = ThemeManager()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(theme)  // explicit injection
                .tint(.theme(.accentBrand))
        }
    }
}
```

---

## Style protocols (the theming layer)

Every interactive control has a `*Style` protocol — `ButtonStyle`, `LabelStyle`, `ProgressViewStyle`, `ToggleStyle`, `PickerStyle`, `GaugeStyle`, `MenuStyle`, `ListStyle`, `TableColumnStyle`. Each style automatically handles pressed / disabled / accessibility / Reduce Transparency state.

### Rules

- **Reach for built-in styles first.** `.buttonStyle(.borderedProminent)`, `.buttonStyle(.glass)`, `.buttonStyle(.glassProminent)` (iOS 26), `.toggleStyle(.switch)`, `.labelStyle(.iconOnly)`, `.pickerStyle(.segmented)`.
- **Wrap brand variants in a `*Style` struct.** Never re-implement the same "filled rectangle with shadow" inline in five places.
- **Use `ButtonStyle` when only customizing appearance.** Use `PrimitiveButtonStyle` when also customizing interaction (long-press, double-tap, motion-triggered). Most apps need only `ButtonStyle`.
- **One prominent button per region.** `.borderedProminent` and `.glassProminent` are intended for the primary action — two on one screen is a design smell.
- **`controlSize(_:)`** sets `.mini`, `.small`, `.regular`, `.large`, `.extraLarge` on supported styles instead of writing five style structs.
- **Compose style + role.** `Button("Delete", role: .destructive) {}.buttonStyle(.bordered)` — role drives color semantics, style drives shape.

### `PrimaryButtonStyle: ButtonStyle` — the canonical pattern

```swift
// Sources/DesignSystem/Styles/PrimaryButtonStyle.swift
public struct PrimaryButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.app(.headline))
            .foregroundStyle(.theme(.labelPrimary))
            .padding(.horizontal, Spacing.md)
            .padding(.vertical, Spacing.sm)
            .frame(maxWidth: .infinity)
            .background(
                RoundedRectangle(cornerRadius: Radius.button, style: .continuous)
                    .fill(.theme(.accentBrand))
            )
            .opacity(isEnabled ? 1.0 : 0.4)
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(Motion.snappy(reduceMotion: reduceMotion),
                       value: configuration.isPressed)
    }

    public init() {}
}

// Sources/DesignSystem/Styles/ButtonStyle+Tokens.swift
public extension ButtonStyle where Self == PrimaryButtonStyle {
    static var primary: PrimaryButtonStyle { .init() }
}
```

Calling code reads like Apple's built-ins:

```swift
Button("Save Changes") { save() }
    .buttonStyle(.primary)
    .disabled(!isValid)
```

### Style file convention

```
Styles/
├── Buttons/
│   ├── PrimaryButtonStyle.swift
│   ├── SecondaryButtonStyle.swift
│   ├── DestructiveButtonStyle.swift
│   ├── GlassPrimaryButtonStyle.swift   # iOS 26
│   └── ButtonStyle+Tokens.swift        # .primary / .secondary / .destructive shortcuts
├── Toggle/
│   └── BrandSwitchToggleStyle.swift
├── Label/
│   ├── IconLeadingLabelStyle.swift
│   └── TitleCaptionLabelStyle.swift
├── Picker/
│   └── BrandSegmentedPickerStyle.swift
└── ProgressView/
    ├── BrandLinearProgressViewStyle.swift
    └── BrandCircularProgressViewStyle.swift
```

---

## Material vs Color (Liquid Glass era)

### Materials available

- `.ultraThin` — most transparent
- `.thin`
- `.regular` — default for most chrome
- `.thick`
- `.ultraThick` — most opaque

### Rules

- **Materials are for chrome, not content.** Tab bar, toolbar, nav bar, sheet, popover, floating accessory, menu — yes. Cards and content rows stay opaque.
- **Two material layers max.** Bar + sheet is fine. Bar + card + sheet is mush.
- **Reduce Transparency is automatic.** The system raises opacity when `UIAccessibility.isReduceTransparencyEnabled`; do not override.
- **For Liquid Glass adoption** → see `references/liquid-glass.md`. The short version: three paths (selective glass on chrome, custom chrome opt-out, full opt-out via `UIDesignRequiresCompatibility`).

---

## Dark mode

- **`@Environment(\.colorScheme) var scheme`** — read only when behavior differs (asset swap, material choice). Never to branch on color.
- **Asset catalog variants** are the right place for color appearance differences. Define `Any`, `Dark`, `Any High Contrast`, `Dark High Contrast` for every brand color.
- **Manual override on a view: `.preferredColorScheme(.dark)`** — use only for explicit user-driven setting on a specific subtree, never at app root without a setting.
- **Test both appearances in previews.**

```swift
#Preview("Light") {
    ContentView()
}

#Preview("Dark") {
    ContentView()
        .preferredColorScheme(.dark)
}
```

---

## Accessibility variants

Reduce Transparency / Reduce Motion / Increase Contrast / Bold Text / Differentiate Without Color / VoiceOver / Dynamic Type are all covered in `references/accessibility.md`. The design-system layer's job is to **expose tokens that already respect these settings** so feature views don't branch on them.

Short version:

- **Reduce Transparency** is handled by the system on materials and glass — don't override.
- **Reduce Motion** — Motion tokens (`Motion.snappy(reduceMotion:)`) and ButtonStyles already swap to linear when on.
- **Increase Contrast** — asset catalog `Any High Contrast` / `Dark High Contrast` variants are the source of truth.
- **Dynamic Type** — every font goes through `Font.app(_:)` with `relativeTo:`; `@ScaledMetric` scales metrics with text.

---

## App-wide accent

```swift
@main
struct MyApp: App {
    @State private var theme = ThemeManager()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(theme)
                .tint(.theme(.accentBrand))  // app root
        }
    }
}
```

- `.tint(.brand)` at the root colors interactive elements (buttons, switches, segmented pickers) consistently.
- Per-view `.tint(_)` overrides for a region of the tree.
- Set `AccentColor` in the asset catalog as the default — used by previews, Spotlight, launch screen.

---

## Iconography module sketch

```swift
// Sources/DesignSystem/Iconography/Symbol+Tokens.swift
public enum AppSymbol: String {
    case home       = "house"
    case search     = "magnifyingglass"
    case settings   = "gearshape"
    case profile    = "person.crop.circle"
    case favorite   = "heart"
    case trash      = "trash"
    case checkmark  = "checkmark.circle.fill"
    case warning    = "exclamationmark.triangle.fill"
    case error      = "xmark.octagon.fill"
}

public extension Image {
    init(_ symbol: AppSymbol) {
        self = Image(systemName: symbol.rawValue)
    }
}

// Usage
Image(.home)
Image(.checkmark)
    .symbolRenderingMode(.palette)
    .foregroundStyle(.white, .theme(.statusSuccess))
```

---

## DesignSystem module structure (concrete)

```
Sources/DesignSystem/
├── Colors/
│   ├── ColorRole.swift              # enum ColorRole (semantic)
│   ├── ThemedColor.swift            # ShapeStyle with resolve(in:)
│   ├── Color+Tokens.swift           # static .theme(_:) extension
│   └── Resources/
│       └── Assets.xcassets          # all .colorset entries, namespaced folders
├── Typography/
│   ├── AppTextStyle.swift           # enum AppTextStyle (semantic)
│   ├── AppFont.swift                # font(for:) returning Font
│   ├── Font+Tokens.swift            # static func app(_:)
│   ├── DynamicTypeMetrics.swift     # @ScaledMetric helpers
│   └── Resources/
│       └── Fonts/                   # *.ttf / *.otf shipped via UIAppFonts
├── Spacing.swift
├── Radius.swift
├── Stroke.swift
├── Motion.swift
├── Materials.swift
├── Iconography/
│   ├── Symbol+Tokens.swift          # enum AppSymbol: String
│   └── Resources/
│       └── Symbols.xcassets         # custom .symbolset entries
├── Styles/
│   ├── PrimaryButtonStyle.swift
│   ├── SecondaryButtonStyle.swift
│   ├── DestructiveButtonStyle.swift
│   ├── ButtonStyle+Tokens.swift     # .primary / .secondary / .destructive
│   ├── BrandSwitchToggleStyle.swift
│   ├── IconLeadingLabelStyle.swift
│   └── BrandLinearProgressViewStyle.swift
├── Environment/
│   ├── Theme+Entry.swift            # @Entry var theme: AppTheme
│   └── ThemeManager.swift           # @Observable, persists user choice
├── Strings/
│   └── Resources/
│       └── Localizable.xcstrings    # one catalog per module
└── DesignSystem.swift               # re-exports
```

### Module rules

- **Zero feature imports.** `DesignSystem` depends on nothing except SwiftUI / UIKit.
- **Public API is the tokens + styles.** Internal asset catalog and ShapeStyle plumbing is `internal` / `fileprivate`.
- **Every feature target depends on `DesignSystem`**, never on raw `Color`, raw `Font`, raw `CGFloat` literals.
- **Snapshot-test the surface at Light / Dark / Increased Contrast / AX5** — the only way drift gets caught.

---

## Anti-patterns

- **Magic numbers in views.** `.padding(16)`, `.cornerRadius(12)`, `.frame(width: 320, height: 80)`. Use `Spacing.md`, `Radius.input`, `Layout.cardWidth`.
- **Inline hex literals.** `Color(hex: "#FF5733")` or `Color(red: 0.95, green: 0.4, blue: 0.2)` belongs in the asset catalog.
- **`Color.white` / `Color.black` defaults.** Dark mode silently broken — use `Color(.systemBackground)`, `.primary`, themed tokens.
- **`.font(.system(size:))` without `relativeTo:`.** Dynamic Type dead on arrival.
- **`.foregroundColor` / `.accentColor` / `cornerRadius`** — deprecated. Use `.foregroundStyle` / `.tint` / `.clipShape(.rect(cornerRadius:, style: .continuous))`.
- **Multiple radii values for the same role.** `cornerRadius: 14` in one file next to `cornerRadius: 16` in another — drift.
- **Custom fonts without Dynamic Type ramp.** Every custom font callsite passes `relativeTo:`.
- **ButtonStyle re-implemented inline per usage.** Inline `.padding(16).background(.blue).foregroundColor(.white).cornerRadius(12)` chain is `PrimaryButtonStyle`.
- **`.preferredColorScheme(.dark)` at the root** without a user setting. Stomps system preference.
- **Inline `LinearGradient(colors: [.purple, .pink], ...)`.** Gradients are tokens too — `Gradient.brandHero` lives in the design system.
- **Inline `shadow(color:radius:x:y:)` hand-tuned.** Elevation tokens (`Elevation.card`, `Elevation.floating`) deduplicate this.
- **`.spring(response: 0.32, dampingFraction: 0.65)`.** Use `.smooth` / `.snappy` / `.bouncy`; tune via their parameters.
- **`if colorScheme == .dark { ... } else { ... }` branching in views.** Color tokens encode the variant; views should not know about scheme.
- **`Image.foregroundColor(.red)` on multi-layer SF Symbols.** Pick a rendering mode and pass a `ShapeStyle` to `.foregroundStyle`.
- **Brand font on system tab/toolbar in iOS 26.** Fights Liquid Glass vibrancy, hurts legibility, costs free localized metrics.
- **Bouncy progress indicators.** Loaders and progress are linear, always.
- **`@Entry` for reference types** without an explicit injection at scene root — SwiftUI re-creates defaults during environment preparation, you get split-brain state.
- **Hand-rolled glyph fonts (FontAwesome, Material Icons, etc.).** SF Symbols is the answer — variable value, multicolor, hierarchical, AX-scaled, all free.
- **Centralized "Theme" struct that just wraps `Color` constants.** That's a primitive bag — a real theme is a `ShapeStyle` that resolves against environment.
- **String literals in views.** Localized strings are design tokens; use String Catalogs with semantic keys.
- **Adding `Spacing.md17`** instead of fixing the design. Token sets are deliberately small; new value = design is drifting.

---

## Centralization commandments (the recap)

1. **No magic numbers in views.** Every literal in a SwiftUI body is a bug.
2. **No hex codes outside the asset catalog.** Brand colors live in `.colorset` files with all four appearance variants, or they don't exist.
3. **No `Color.black` / `Color.white` in views.** Use semantic system colors or themed tokens.
4. **`.foregroundStyle` not `.foregroundColor`. `.tint` not `.accentColor`. `.clipShape(.rect(cornerRadius:, style: .continuous))` not `.cornerRadius`.** Modern APIs everywhere.
5. **Every typography callsite uses `relativeTo:`.** No `Font.system(size: 16)` without a text-style anchor.
6. **One radius per role, continuous corner style.** Drift between `cornerRadius: 14` and `cornerRadius: 16` is the most visible "stale codebase" symptom.
7. **Named springs, not raw `.spring(response:dampingFraction:)`.** Use `.smooth`, `.snappy`, `.bouncy`.
8. **Glass on chrome only.** Liquid Glass for nav / tab / toolbar / sheet / menu, never on content cards or full-screen backgrounds. Two glass layers max — see `liquid-glass.md`.
9. **Styles, not view modifiers.** Buttons, toggles, pickers, labels, progress views, gauges, lists all have a `*Style` protocol — use it.
10. **Generate the primitive layer when possible.** Spacing / Radius / Color primitives flow Figma → Tokens Studio → Style Dictionary → Swift. Hand-curated semantic layer wraps the generated primitives.

---

## Cross-references

- Liquid Glass adoption paths → `liquid-glass.md`
- Accessibility variants (Reduce Transparency / Motion / Contrast) → `accessibility.md`
- Animation curves and `@Animatable` macro → `animation.md`
- View composition rules (one purpose per view) → `view-composition.md`
- Deprecated API replacements → `modern-api.md`
