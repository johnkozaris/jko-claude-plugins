# Accessibility

Target: Swift 6.3, iOS 26, Xcode 26.

Accessibility in SwiftUI is mostly a matter of using the controls the framework already gives you correctly. A `Button` already participates in VoiceOver focus, keyboard activation, hover, and the disabled-state environment. A `Text` already scales with Dynamic Type. The framework does the heavy lifting. The mistakes are almost always shortcuts that strip those defaults away — an icon wrapped in an `onTapGesture` instead of a `Button`, a font sized with `.system(size:)` instead of `.body`, a status indicator that uses color alone. This file goes through the cases worth knowing.

Cross-references:
- Text on glass and Reduce Transparency behavior — `liquid-glass.md`.
- Reduce Motion at animation call sites — `animation.md`.
- Audit harness in CI — `testing-and-debugging.md`.
- `@ScaledMetric` and design tokens — `design-system.md`.
- Full Keyboard Access on macOS — `macos-platform.md`.

---

## Audit infrastructure in CI

The most useful single thing you can add to a SwiftUI project's accessibility story is a UI test that runs Apple's built-in audit on every screen. `XCUIApplication` has an instance method `performAccessibilityAudit()` that exercises the screen against Apple's own rule set: missing labels, low contrast, clipped text at large Dynamic Type, unreachable hit regions, conflicting traits.

```swift
import XCTest

final class AccessibilityTests: XCTestCase {
    func testHomeScreenAudit() throws {
        let app = XCUIApplication()
        app.launch()
        try app.performAccessibilityAudit()
    }

    func testHomeScreenAtLargeText() throws {
        let app = XCUIApplication()
        app.launchArguments = [
            "-UIPreferredContentSizeCategoryName",
            "UICTContentSizeCategoryAccessibilityXXXL"
        ]
        app.launch()
        try app.performAccessibilityAudit()
    }
}
```

The audit throws on any issue it finds. The CI build fails. A reviewer doesn't have to remember to check.

You can scope it with an option set if a particular category is producing noise you've decided to live with:

```swift
try app.performAccessibilityAudit(for: .all.subtracting(.dynamicType))
```

I'd avoid suppressing categories long-term — the value of the audit is the breadth — but skipping one while you stabilize a screen is fine.

One thing I'm not certain about: I haven't verified the exact set of `XCUIAccessibilityAuditType` cases that ships in Xcode 26 versus earlier. Check the current docs if you're trying to suppress something specific. The general mechanism has been stable since Xcode 15.

---

## Labels on interactive elements

Every interactive element needs a text label that VoiceOver can announce. The framework gives this to you for free when you use a `Button` with a string argument. It can't give it to you when the button's content is only an image.

```swift
// VoiceOver reads "plus" — the SF Symbol name. Useless.
Button(action: addUser) {
    Image(systemName: "plus")
}

// VoiceOver reads "Add User". The image is decorative.
Button("Add User", systemImage: "plus", action: addUser)
```

The `Button("Label", systemImage:, action:)` initializer is the modern shorthand for the icon-plus-text pattern. It treats the image as decorative and uses the string as the label. Use it everywhere you'd previously have written a `Button { Label("Add User", systemImage: "plus") }` or a hand-rolled `HStack`.

The same pattern applies to `Menu`, `Link`, and `NavigationLink`:

```swift
Menu("Options", systemImage: "ellipsis.circle") {
    Button("Export", action: exportAction)
    Button("Share", action: shareAction)
}

Link("Apple Developer", destination: URL(string: "https://developer.apple.com")!)
```

When the visible label is short or symbolic — a stock ticker, an emoji status — supply a richer label through `.accessibilityLabel`:

```swift
Button("AAPL \(price)") { showQuote() }
    .accessibilityLabel("Apple Inc. stock price \(price.formatted(.currency(code: "USD")))")
```

`.accessibilityHint` adds a second sentence that VoiceOver reads after a short delay. Use it when the label alone doesn't make the action obvious:

```swift
Button("Edit", action: edit)
    .accessibilityHint("Opens the article editor")
```

Don't use hints to compensate for a bad label. If the label needs explanation, the label is the thing to fix.

---

## Variable labels with `accessibilityInputLabels`

There's a separate problem when a button's visible label changes frequently — live prices, animated numerics, anything where the displayed text is data rather than a name. Voice Control users say "tap [label]" to activate a button. If the label is "$182.43" one second and "$182.44" the next, the voice command can't keep up.

`.accessibilityInputLabels(_:)` lets you provide a stable set of phrases that Voice Control will accept regardless of the current label:

```swift
Button("AAPL \(price)") { showQuote() }
    .accessibilityInputLabels(["Apple", "Apple stock", "AAPL", "Apple Incorporated"])
```

Users can say "tap Apple" no matter what the visible price is at that moment. Use this anywhere the displayed label is data and the conceptual identity of the control is stable.

---

## Decorative images

By default, `Image(_:)` is treated by VoiceOver as meaningful and read using the asset name. If your asset is named `bannerBackground`, VoiceOver will say "banner background." That's almost always wrong. The asset name was for you, not the user.

Two ways to mark an image as decorative:

```swift
Image(decorative: "bannerBackground")
Image(.banner).accessibilityHidden(true)
```

`Image(decorative:)` is the more declarative form — it's a static signal that this image carries no meaning. `.accessibilityHidden(true)` is a modifier you can apply to any view, useful when the image lives inside a larger composition you're hiding wholesale.

For meaningful images — a chart, a photo with content the user needs to know about — provide a real description:

```swift
Image(.revenueChart)
    .accessibilityLabel("Revenue trended up 12 percent over the last six months")
```

A common review finding is a banner or hero image with no accessibility treatment. The asset name leaks into VoiceOver and reads as gibberish. Flag any `Image(.someAutoGeneratedName)` you see without one of the two treatments above.

---

## Buttons, not gestures

`onTapGesture` looks like the simple way to make any view tappable. It's a trap. A `Button` provides VoiceOver activation, keyboard activation with Space and Return, focus traversal, hover state on iPad and Mac, the standard press animation, and the `\.isEnabled` environment for `.disabled(_)` propagation. `onTapGesture` provides none of those. It just calls a closure on tap.

```swift
// Invisible to VoiceOver. No focus. No keyboard. No disabled state.
Rectangle()
    .fill(.tint)
    .onTapGesture { action() }

// Same visuals. All the accessibility comes along for free.
Button(action: action) {
    Rectangle().fill(.tint)
}
.buttonStyle(.plain)
```

`.buttonStyle(.plain)` strips the default button chrome while keeping the behavior. Use it when you need the appearance of a tappable region rather than a styled button.

There are cases where `onTapGesture` is legitimately the right primitive — you need the tap location (`onTapGesture(coordinateSpace:)`) or a tap count (`onTapGesture(count: 2)`). In those cases, add the missing accessibility yourself:

```swift
DrawingCanvas()
    .onTapGesture(coordinateSpace: .local) { location in
        handle(tapAt: location)
    }
    .accessibilityAddTraits(.isButton)
    .accessibilityLabel("Drawing canvas")
    .accessibilityHint("Double tap to add a point")
```

If you find yourself writing `.accessibilityAddTraits(.isButton)` on a `Rectangle`, ask whether a `Button` with a custom label and `.buttonStyle(.plain)` would do the same job. Usually it would.

---

## Dynamic Type

Dynamic Type is the user-controlled font-size system on iOS and macOS. Users can scale text from very small to very large, and there's a separate "Accessibility Sizes" range that goes larger still. The system text styles — `.body`, `.title`, `.headline`, and so on — scale automatically. Hardcoded point sizes don't.

```swift
// Stuck at 17pt no matter what the user has chosen.
.font(.system(size: 17))

// Scales with Dynamic Type. This is the default for body copy.
.font(.body)
```

When you genuinely need a custom font — a brand typeface — anchor it to a system text style with `relativeTo:` so it scales:

```swift
.font(.custom("Inter-Regular", size: 17, relativeTo: .body))
```

The `relativeTo:` parameter tells SwiftUI which Dynamic Type ramp to follow. Without it, your custom font sits at 17pt forever. With it, the font scales the same way `.body` does.

I'd flag any `.font(.system(size:))` call without `relativeTo:` during review. It's almost always a mistake. The cases where you genuinely want a fixed size — a number in a clock display, a non-text glyph composited into a larger graphic — are rare enough to call out explicitly with a comment.

### `@ScaledMetric` for non-text dimensions

Type scaling isn't only about font size. Padding around text, the size of an icon next to text, the height of a row that contains text — all of those should grow when type grows. `@ScaledMetric` is the property wrapper for this:

```swift
struct AvatarBadge: View {
    @ScaledMetric(relativeTo: .body) private var iconSize: CGFloat = 24
    @ScaledMetric(relativeTo: .body) private var horizontalPadding: CGFloat = 16

    var body: some View {
        Image(systemName: "checkmark")
            .frame(width: iconSize, height: iconSize)
            .padding(.horizontal, horizontalPadding)
    }
}
```

When the user is on Accessibility Large, `iconSize` and `horizontalPadding` scale up proportionally. You don't write branches for it.

Use `@ScaledMetric` for any numeric dimension that should track type: icon size, badge size, padding next to text, row height, the corner radius on a chip that contains text. Don't use it for things that shouldn't scale — a navigation bar's height, a thumbnail grid's cell size, a fixed UI element with no text inside.

### Testing at the extremes

Write previews at both ends of the range:

```swift
#Preview("xSmall") {
    MyView().environment(\.dynamicTypeSize, .xSmall)
}

#Preview("AX5") {
    MyView().environment(\.dynamicTypeSize, .accessibility5)
}
```

xSmall is the smallest setting. AX5 is the largest accessibility size. Most layout bugs surface at AX5 — text wraps differently, vertical compositions reflow, fixed-width regions clip.

`.minimumScaleFactor(0.8)` lets text shrink slightly to fit a fixed-width container before truncating. Use it sparingly — shrinking text fights the user who chose a larger size in the first place. Prefer letting layout reflow.

---

## Color and contrast

The system semantic colors adapt to dark mode, increased contrast, and tinted backgrounds automatically. Use them when you can:

```swift
.foregroundStyle(.primary)
.foregroundStyle(.secondary)
.foregroundStyle(.tertiary)
Color(.systemBackground)
Color(.label)
Color(.separator)
```

The hierarchy levels — `.primary` through `.quinary` — are the cleanest way to express "this is the main text" or "this is supporting text" without picking a literal color. They adapt across appearances without any conditional code on your side.

For custom colors in an asset catalog, define four variants: Any, Dark, Any High Contrast, Dark High Contrast. If you only define Any and Dark, your color is wrong for users with Increased Contrast turned on. The user-facing symptom is text that's just legible enough to read on a normal screen becoming unreadable when contrast is requested.

To branch on contrast in code, read `\.colorSchemeContrast`:

```swift
@Environment(\.colorSchemeContrast) private var contrast

var borderColor: Color {
    contrast == .increased ? .primary : .secondary
}
```

WCAG contrast minimums, for reference: 4.5:1 for normal text, 3:1 for large text (about 18pt and up, or 14pt bold and up), and 3:1 for the edges of interactive components like button borders. I'd treat these as the floor, not the goal.

### Color is never the only signal

A red dot that means "error" is invisible to a colorblind user, fades on a sun-lit screen, and disappears under Increased Contrast in some appearance combinations. Color is a useful signal in combination with something else, never alone.

```swift
@Environment(\.accessibilityDifferentiateWithoutColor) private var noColor

HStack {
    Image(systemName: status.icon)        // shape carries meaning
    Text(status.label)                     // text carries meaning
    if noColor {
        Image(systemName: "exclamationmark.triangle.fill")
    }
}
.foregroundStyle(status.color)             // color still helpful, just not load-bearing
```

The combinations that work: color plus icon (a red exclamation mark), color plus text (a red "Error" label), color plus shape (a red filled circle versus a green hollow ring). The `\.accessibilityDifferentiateWithoutColor` environment value is the user's explicit request that you not rely on color as a signal. Honor it by adding shapes or labels rather than removing color.

A pure-color state indicator — a row that's just red, with no icon and no text — warrants a closer look during review. If the color carries the only information, the indicator fails for the users who most need it to work.

---

## Reduce Transparency

iOS 26's Liquid Glass material adapts automatically when the user turns on Reduce Transparency — `.glassEffect()` becomes more opaque on its own. Custom transparency you've added by hand doesn't adapt unless you wire it up:

```swift
@Environment(\.accessibilityReduceTransparency) private var reduceTransparency

ZStack {
    if reduceTransparency {
        Color(.systemBackground)
    } else {
        BlurredHero()
    }
    ContentLayer()
}
```

Don't override the system's reduce-transparency behavior on `.glassEffect()`. The system raises opacity by design; fighting it makes your app harder to read for the user who asked for it.

Text on a glass surface is a particular case. The HIG instructs you not to put body text directly on glass — the readability under various conditions is too unpredictable. Glass is for chrome (toolbars, sheets, accessory bars). Text content lives on opaque surfaces above or below the glass. See `liquid-glass.md` for the longer treatment.

---

## Reduce Motion

`@Environment(\.accessibilityReduceMotion)` is the user's signal that they'd like less motion. Common reasons include vestibular sensitivity to large parallax or zoom effects.

```swift
@Environment(\.accessibilityReduceMotion) private var reduceMotion

withAnimation(reduceMotion ? .linear(duration: 0.15) : .bouncy) {
    isExpanded.toggle()
}

.transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
```

A short cross-fade is usually the right substitute for a motion-heavy animation. The user still gets a visible change-of-state without the motion that bothered them.

Symbol effects and `PhaseAnimator` respect Reduce Motion on their own. Custom `Transition` types and bespoke `withAnimation` calls don't — that's where you have to do the work. See `animation.md` for the full breakdown.

---

## Element grouping

Compound UI — a row with an icon, a title, a subtitle, and a chevron — should usually be read by VoiceOver as one element, not four. `.accessibilityElement(children:)` controls that:

```swift
HStack {
    Image(systemName: "star.fill")
    Text("Favorites")
}
.accessibilityElement(children: .combine)
// Read as "Favorites" — the image's role contributes but isn't announced separately
```

The three modes:

- `.combine` — merge child labels into a single announcement. The most common choice for a row composed of icon plus text.
- `.ignore` — skip the children entirely and use the parent's label. Useful when the composition is busy enough that you want to write a custom narration.
- `.contain` — children stay as separate elements but are grouped together for rotor navigation.

```swift
VStack {
    Text("$1,234")
    Text("12 transactions")
}
.accessibilityElement(children: .ignore)
.accessibilityLabel("Balance one thousand two hundred thirty-four dollars, twelve transactions")
```

Don't reach for `.accessibilityHidden(true)` to clean up cluttered VoiceOver navigation. Grouping is the right answer; hiding interactive elements makes them unreachable.

---

## Accessibility identifiers for UI tests

`.accessibilityIdentifier(_:)` is a separate channel from `.accessibilityLabel(_:)`. The identifier is invisible to users and exists only as a stable hook for `XCUIElement` queries in UI tests. The label changes when you localize; the identifier doesn't.

```swift
Button("Save", systemImage: "tray", action: save)
    .accessibilityIdentifier("articleEditor.saveButton")

TextField("Title", text: $title)
    .accessibilityIdentifier("articleEditor.titleField")
```

In the UI test:

```swift
let saveButton = app.buttons["articleEditor.saveButton"]
XCTAssertTrue(saveButton.exists)
saveButton.tap()
```

A convention I've seen work well is `screen.element` in camelCase — `articleEditor.saveButton`, `loginScreen.passwordField`. One identifier per testable control. Don't put an identifier on a root view that holds many testable elements; the queries will be ambiguous.

---

## Custom controls

When you've built something interactive that doesn't map cleanly to a standard control, you have two options. One is to tell VoiceOver what the control behaves like:

```swift
.accessibilityRepresentation {
    Slider(value: $progress, in: 0...1)
}
```

The system reads the represented view's accessibility, even though the visible UI is your custom design. Use this when your control is conceptually a slider, a toggle, or a picker, just drawn differently.

The other is to wire up the accessibility actions yourself:

```swift
.accessibilityAdjustableAction { direction in
    switch direction {
    case .increment: value = min(value + 1, maxValue)
    case .decrement: value = max(value - 1, 0)
    @unknown default: break
    }
}
.accessibilityValue("\(value) of \(maxValue)")
```

For toggle behavior:

```swift
.accessibilityAddTraits(.isButton)
.accessibilityValue(isOn ? "On" : "Off")
.accessibilityAction { isOn.toggle() }
```

The honest reality is that custom controls are where accessibility bugs hide. If a standard control could plausibly do the job, prefer the standard control.

---

## Forms and label pairing

A label-and-field pair should be one announcement: "Email, name@example.com text field." `LabeledContent` does this for you in most form layouts:

```swift
Form {
    LabeledContent("Email") {
        TextField("name@example.com", text: $email)
    }
}
```

When you need a more custom layout but still want the pairing, `accessibilityLabeledPair` is the explicit form:

```swift
@Namespace private var emailNS

VStack(alignment: .leading) {
    Text("Email")
        .accessibilityLabeledPair(role: .label, id: "email", in: emailNS)
    TextField("name@example.com", text: $email)
        .accessibilityLabeledPair(role: .content, id: "email", in: emailNS)
}
```

Reach for `LabeledContent` first. The explicit pairing API is there for the cases `LabeledContent` can't express.

---

## Charts

Swift Charts is accessible by default — each mark has a label and value derived from the data — but a chart benefits from a high-level summary that screen readers can read in place of describing every mark:

```swift
Chart(salesData) { sale in
    BarMark(
        x: .value("Month", sale.month),
        y: .value("Revenue", sale.revenue)
    )
    .accessibilityLabel("\(sale.month)")
    .accessibilityValue(sale.revenue.formatted(.currency(code: "USD")))
}
.accessibilityLabel("Monthly revenue chart")
.accessibilityValue(chartSummary)

private var chartSummary: String {
    "Revenue across \(salesData.count) months, total \(total.formatted(.currency(code: "USD")))"
}
```

For audio graphs (chart sonification — the "play the chart as sound" feature), implement `AXChartDescriptorRepresentable`. I'd consult the current Apple docs for the exact protocol shape; the API has evolved over recent releases.

---

## Localization

Hardcoded English in an accessibility label breaks screen readers in every other language. Anything that VoiceOver reads aloud is user-facing text, even though it doesn't appear on screen.

```swift
// Localized — works in every supported language
.accessibilityLabel(String(localized: "Add user to favorites"))

// English-only — broken everywhere else
.accessibilityLabel("Add user to favorites")
```

String Catalogs (`.xcstrings`) are the modern format. Provide accessibility-specific keys when the verbose VoiceOver label differs from a short visible label.

---

## Accessibility Nutrition Labels

App Store Connect surfaces per-platform accessibility support. The user-facing categories I've seen are VoiceOver, Voice Control, Dynamic Type, Reduced Motion, Increased Contrast, and Captions / Audio Descriptions, though I'd verify the exact list against the current App Store Connect UI.

Users can filter the App Store by these. Declaring support you don't actually provide invites review rejection. Before declaring VoiceOver support, walk every screen with VoiceOver on. Before declaring Dynamic Type, test at xSmall and AX5. Before declaring Reduced Motion, confirm motion-heavy effects are suppressed when the setting is on.

The CI audit described earlier catches the common mistakes. Manual verification catches the rest.

---

## Haptics

`.sensoryFeedback(_:trigger:)` is the SwiftUI primitive for haptic and audio feedback bound to a state change. It respects the user's system haptic preferences automatically — including the accessibility settings that soften or disable haptics.

```swift
Button("Save") { save() }
    .sensoryFeedback(.success, trigger: saveCount)
```

Use semantic values (`.success`, `.warning`, `.error`, `.selection`, `.alignment`) when one fits. Raw impact (`.impact(weight:)`) is the fallback for cases the semantic vocabulary doesn't cover.

Don't reach for `UIImpactFeedbackGenerator` directly in SwiftUI code. The UIKit haptic generators don't bind to state changes the way `.sensoryFeedback` does, don't respect the SwiftUI lifecycle, and don't pick up accessibility preferences as cleanly. See `animation.md` for the broader treatment of haptics.

---

## A few specific don'ts

- An icon-only button with no label. `Button("Add", systemImage: "plus", action:)` is the fix.
- `onTapGesture` where a `Button` would do the same work. The button comes with free accessibility traits.
- `.font(.system(size: N))` without `relativeTo:`. Dynamic Type stops working.
- Hardcoded padding or icon sizes around text. `@ScaledMetric` so they grow with type.
- Color as the only signal of state. Combine with an icon, a shape, or text.
- Text placed directly on a glass surface. Glass is for chrome; opaque layers hold content.
- Overriding Reduce Transparency on `.glassEffect()`. The system handles it; don't fight it.
- Ignoring Reduce Motion in `withAnimation`. Read the environment value and downgrade.
- Asset names leaking into VoiceOver because the image wasn't marked decorative or labeled.
- Skipping High Contrast variants in the asset catalog. Define all four appearances.
- Claiming VoiceOver support in App Store Connect on an app that hasn't been audited end-to-end.
- `UIImpactFeedbackGenerator` calls in SwiftUI code. Use `.sensoryFeedback`.
- A single `.accessibilityIdentifier` on a root view that holds many test targets. One identifier per element.
- Hardcoded English in `.accessibilityLabel`. Localize through String Catalogs.
- `.accessibilityHidden(true)` on interactive elements to "clean up" navigation. Group with `children:` instead.
