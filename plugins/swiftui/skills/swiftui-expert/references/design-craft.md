# Design Craft

Design craft for SwiftUI — visual direction, typography, color, spacing, hierarchy, interaction states, motion, and HIG alignment.

## Design Direction

Every app needs a clear visual identity. Before writing code, establish:

- **Aesthetic commitment**: minimalist, warm, editorial, playful, technical — pick one and commit. Mixing aesthetics reads as indecisive.
- **Visual weight**: decide if the app is content-heavy (deference) or interface-heavy (expression). Most iOS apps should defer to content.
- **Signature elements**: one or two distinctive choices (a custom font weight, an accent color, a transition style) that make the app recognizable.

### Avoiding Generic AI Aesthetics in iOS Apps

AI-generated SwiftUI code converges on the same bland patterns. These are the telltale signs — avoid them:

**Layout tells:**
- Cards and `RoundedRectangle` for everything — use spacing, grouping, and `List` styles instead
- Wrapping every screen in a `ScrollView { VStack }` with uniform padding — use proper `List`, `Form`, or `NavigationSplitView`
- Centering everything — most iOS content should be leading-aligned
- Identical spacing everywhere — vary spacing to create rhythm and grouping

**Color and material tells:**
- `.linearGradient` or `.mesh` gradient backgrounds on views that don't need them — gradients are for intentional emphasis, not decoration
- `.ultraThinMaterial` / frosted glass on everything — material is for overlays and controls, not content backgrounds. Especially don't fake Liquid Glass with `.blur()` and materials on iOS 18 and earlier
- Heavy `.shadow()` on every card — shadows should be subtle and purposeful, indicating elevation
- `.tint(.blue)` default accent with no customization — pick a brand color

**Typography tells:**
- Every text element the same size and weight — no hierarchy
- `.font(.system(size:))` hardcoded everywhere instead of semantic styles
- Missing `.bold()`, `.secondary`, or `.foregroundStyle` variation — flat visual weight

**Interaction tells:**
- Oversized, rounded, gradient-filled buttons that look like web CTAs — iOS buttons are typically text or system-styled
- Custom tab bars and navigation when system components work fine — use native `TabView` and `NavigationStack`
- Decorative SF Symbols scattered everywhere with no functional purpose
- Bouncy `.spring()` animations on everything — animation should be purposeful, not decorative

**The test**: show your app to someone and ask "did AI make this?" If they say yes immediately, the design needs more intentionality. A well-designed app should feel like a human designer made deliberate choices — not like a model picked the statistically safest option for every element.

## Apple HIG Core Principles

Every interface decision should serve three pillars:

1. **Clarity** — every element legible and purposeful
2. **Deference** — UI never overshadows content
3. **Depth** — layers, transitions, and motion convey hierarchy

## Typography

### Use Dynamic Type Exclusively

Never hardcode font sizes. Always use semantic text styles:

```swift
// Good
Text("Title").font(.title)
Text("Body content").font(.body)
Text("Small detail").font(.caption)

// Bad — breaks Dynamic Type
Text("Title").font(.system(size: 24))
```

### Typography Scale

| Style | Default Size | Usage |
|---|---|---|
| `.largeTitle` | 34pt | Screen titles, hero text |
| `.title` | 28pt | Section headers |
| `.title2` | 22pt | Subsection headers |
| `.title3` | 20pt | Card titles |
| `.headline` | 17pt bold | Emphasized body text |
| `.body` | 17pt | Primary content |
| `.callout` | 16pt | Supporting explanations |
| `.subheadline` | 15pt | Secondary labels |
| `.footnote` | 13pt | Timestamps, metadata |
| `.caption` | 12pt | Tertiary info (use sparingly) |
| `.caption2` | 11pt | Avoid — extremely small |

### Custom Scaling

```swift
// iOS 18 and earlier
@ScaledMetric(relativeTo: .body) private var iconSize = 24.0

// iOS 26+
.font(.body.scaled(by: 1.5))
```

### Typographic Hierarchy

Establish clear hierarchy using no more than 3-4 levels:

1. **Primary**: `.title` or `.largeTitle` — one per screen, draws the eye first
2. **Secondary**: `.headline` — section headers, card titles
3. **Body**: `.body` — primary readable content
4. **Tertiary**: `.footnote` or `.caption` — metadata, timestamps, supporting info

Use weight contrast (`.bold()` vs regular) more than size contrast. A `.body.bold()` headline with `.body` text beneath it is often more elegant than jumping font sizes.

### Font Pairing (Custom Fonts)

If using custom fonts:
- Pair a distinctive display/heading font with San Francisco for body text
- Never use more than 2 font families
- Ensure the custom font supports Dynamic Type via `UIFontMetrics` or `.relativeTo:`

```swift
// Custom font that scales with Dynamic Type
.font(.custom("Serif-Bold", size: 28, relativeTo: .title))
```

### Rules

- Use `bold()` not `fontWeight(.bold)` — lets the system choose correct weight for context.
- Only use `fontWeight()` for non-bold weights when there's a specific reason.
- Use `PersonNameComponents` for people's names, not string interpolation.
- Use "y" not "yyyy" for year formatting (correct in all localizations).

## Color System

### Semantic Colors

Name colors by role, not value:

```swift
// Good — adapts to light/dark automatically
.foregroundStyle(.primary)
.foregroundStyle(.secondary)
.background(.systemBackground)

// Bad — hardcoded, breaks dark mode
.foregroundStyle(Color(red: 0.2, green: 0.2, blue: 0.2))
```

### Building a Design System Color Palette

```swift
enum DesignTokens {
    // Semantic naming
    enum Colors {
        static let textPrimary = Color.primary
        static let textSecondary = Color.secondary
        static let backgroundPrimary = Color(.systemBackground)
        static let backgroundSecondary = Color(.secondarySystemBackground)
        static let accent = Color.accentColor
        static let destructive = Color.red
        static let success = Color.green
    }
}
```

### Color Harmony & Proportion

Follow the **60-30-10 rule**:
- **60%** — dominant background/neutral (`.systemBackground`, tinted neutrals)
- **30%** — secondary color (`.secondarySystemBackground`, section backgrounds)
- **10%** — accent color (buttons, active states, key indicators)

Use **tinted neutrals** instead of pure grays — add a subtle hue from your accent color into your grays for visual warmth and cohesion.

### Dangerous Color Combinations

Avoid these — they cause vibration, readability issues, or cultural confusion:
- Red text on blue background (or vice versa)
- Red + green without shape/icon differentiation (color blindness)
- Pure black (`#000`) on pure white (`#FFF`) — too harsh. Use `.primary` which is softer.

### Rules

- Use system colors (`.primary`, `.secondary`, `.systemBackground`) as defaults.
- Define custom colors in asset catalog with light/dark variants.
- Avoid UIKit colors (`UIColor`) in SwiftUI code.
- Ensure WCAG contrast: 4.5:1 for normal text, 3:1 for large text.
- Don't rely on color alone to convey information (accessibility).

### Gotcha: `foregroundStyle` Static Members

When migrating from `foregroundColor`, `.primary` and `.secondary` become hierarchical styles, not `Color.primary`/`Color.secondary`. Be explicit: `.foregroundStyle(Color.primary)` if you need the color.

## Spacing System

Define a spacing scale and use it consistently:

```swift
enum DesignTokens {
    enum Spacing {
        static let xxs: CGFloat = 2
        static let xs: CGFloat = 4
        static let sm: CGFloat = 8
        static let md: CGFloat = 16
        static let lg: CGFloat = 24
        static let xl: CGFloat = 32
        static let xxl: CGFloat = 48
    }
}
```

### Rules

- Avoid hardcoded padding/spacing values scattered throughout views.
- Use consistent spacing scale across the app.
- Prefer flexible frames over fixed frames (adapts to device sizes and Dynamic Type).
- Never use `UIScreen.main.bounds` — use `containerRelativeFrame()` or `GeometryReader` as last resort.

## Corner Radii

```swift
enum DesignTokens {
    enum CornerRadius {
        static let sm: CGFloat = 8
        static let md: CGFloat = 12
        static let lg: CGFloat = 16
        static let xl: CGFloat = 24
    }
}

// Use
.clipShape(.rect(cornerRadius: DesignTokens.CornerRadius.md))
```

Note: `RoundedRectangle` defaults to `.continuous` corner style — don't specify it explicitly.

## Animation Timings

```swift
enum DesignTokens {
    enum Animation {
        static let quick: SwiftUI.Animation = .easeInOut(duration: 0.15)
        static let standard: SwiftUI.Animation = .smooth(duration: 0.3)
        static let emphasis: SwiftUI.Animation = .bouncy(duration: 0.4)
    }
}
```

## Visual Hierarchy

### The Squint Test

Blur your eyes or squint at the screen. You should still be able to identify:
- Where to look first (primary action or content)
- What's grouped together
- What's interactive vs informational

If everything looks the same, you have a hierarchy problem.

### Creating Hierarchy

In order of impact:
1. **Size** — larger elements draw attention first
2. **Weight** — bold text stands out from regular
3. **Color** — accent color for primary actions, muted for secondary
4. **Spacing** — more space around important elements elevates them
5. **Depth** — shadows, materials, Liquid Glass create layering

### Optical Adjustments

- Icons next to text often need 1-2pt visual offset to look centered
- Rounded shapes (circles) appear smaller than squares at the same dimensions — compensate with slightly larger frames
- Leading-aligned text feels more natural than centered in most contexts (except hero sections)

## Interaction States

SwiftUI handles many states automatically, but custom components need all of these:

| State | Visual Treatment |
|---|---|
| **Default** | Base appearance |
| **Pressed/Active** | Slight scale down (0.97), dimmed opacity, or highlight |
| **Disabled** | Reduced opacity (0.4-0.5), no interaction |
| **Loading** | Spinner or skeleton, disable interaction |
| **Error** | Red accent, error message below |
| **Success** | Brief green accent or checkmark, then return to default |
| **Focused** | System focus ring (handled by SwiftUI for standard controls) |

```swift
// Custom button with proper states
Button(action: submit) {
    Label("Submit", systemImage: "paperplane")
}
.disabled(isLoading)
.opacity(isLoading ? 0.6 : 1.0)
.overlay {
    if isLoading {
        ProgressView()
    }
}
```

### General Interaction Rules

- Minimum 44x44pt tap targets — strictly enforced.
- Use `Label` over `HStack { Image; Text }` for icon+text pairs.
- Use `ContentUnavailableView` for empty states (`.search` variant auto-includes search term).
- Wrap `Form` controls in `LabeledContent` for correct title/control layout.
- Prefer system hierarchical styles (`.secondary`, `.tertiary`) over manual opacity.

## UX Writing

- **Button labels**: use verbs, not nouns. "Save Photo" not "OK". "Delete Account" not "Continue".
- **Error messages**: say what happened + what to do. "Connection lost. Check your internet and try again." not "Error 503".
- **Empty states**: explain what goes here + how to start. Use `ContentUnavailableView` with a clear action.
- **Destructive actions**: name the action explicitly. "Delete 3 Photos" not "Delete".

## Dark Mode

- Use semantic system colors — they adapt automatically.
- Test both appearances in previews:

```swift
#Preview {
    MyView()
        .preferredColorScheme(.dark)
}
```

- Custom colors: define in asset catalog with "Any Appearance" and "Dark Appearance" variants.
