# Accessibility

## VoiceOver

### Interactive Elements Must Have Text Labels

```swift
// Bad — VoiceOver reads "plus" which is meaningless
Button(action: addUser) {
    Image(systemName: "plus")
}

// Good — VoiceOver reads "Add User"
Button("Add User", systemImage: "plus", action: addUser)
```

Same for `Menu`:
```swift
// Good
Menu("Options", systemImage: "ellipsis.circle") { ... }
```

### Images

- Decorative images: use `Image(decorative: .banner)` or `.accessibilityHidden(true)`.
- Meaningful images: add `.accessibilityLabel("Description of what this shows")`.
- Flag unclear auto-generated labels (e.g., `Image(.newBanner2026)`).

### Buttons and Gestures

- **Always prefer `Button` over `onTapGesture()`** — `Button` provides free VoiceOver support, focus, and keyboard interaction.
- Only use `onTapGesture()` if you need tap location or tap count.
- If you must use `onTapGesture()`, add `.accessibilityAddTraits(.isButton)`.

### Complex Labels

For buttons with frequently changing labels (e.g., stock prices):

```swift
Button("AAPL \(price)") { ... }
    .accessibilityInputLabels(["Apple", "Apple stock", "AAPL"])
```

## Dynamic Type

### Never Hardcode Font Sizes

```swift
// Bad
.font(.system(size: 17))

// Good
.font(.body)
```

### Custom Scaling

```swift
// iOS 18 and earlier — scales a numeric value with Dynamic Type
@ScaledMetric(relativeTo: .body) private var iconSize = 24.0

Image(systemName: "star")
    .frame(width: iconSize, height: iconSize)

// iOS 26+
.font(.body.scaled(by: 1.5))
```

### Test Dynamic Type in Previews

```swift
#Preview {
    MyView()
        .environment(\.sizeCategory, .extraExtraLarge)
}
```

## Color & Contrast

- WCAG minimum contrast: 4.5:1 for normal text, 3:1 for large text.
- Use system semantic colors (`.primary`, `.secondary`) — they maintain contrast in both light and dark modes.
- Respect `accessibilityDifferentiateWithoutColor`:

```swift
@Environment(\.accessibilityDifferentiateWithoutColor) private var noDifferentiation

// Add icons/patterns/strokes alongside color coding
```

- Never use color as the sole indicator of meaning (red = error). Always add icons or text.

## Reduce Motion

```swift
@Environment(\.accessibilityReduceMotion) private var reduceMotion

// Replace motion with opacity
withAnimation(reduceMotion ? nil : .bouncy) {
    isExpanded.toggle()
}
```

## Element Grouping

Control how VoiceOver groups elements:

```swift
// Combine children into one accessible element
HStack {
    Image(systemName: "star.fill")
    Text("Favorites")
}
.accessibilityElement(children: .combine)

// Ignore children and provide custom label
VStack { ... }
.accessibilityElement(children: .ignore)
.accessibilityLabel("Custom description")
```

### Grouping modes:
- `.combine` — merges children labels (most common)
- `.ignore` — ignores children, uses parent's label
- `.contain` — treats children as separate elements within a group

## Custom Controls

Make custom controls behave like native ones:

```swift
// Tell VoiceOver this is a slider
.accessibilityRepresentation {
    Slider(value: $progress, in: 0...1)
}

// Or for increment/decrement controls
.accessibilityAdjustableAction { direction in
    switch direction {
    case .increment: value += 1
    case .decrement: value -= 1
    @unknown default: break
    }
}
```

## Label Pairing

Associate labels with their content:

```swift
@Namespace private var labelNS

Text("Email")
    .accessibilityLabeledPair(role: .label, id: "email", in: labelNS)
TextField("Enter email", text: $email)
    .accessibilityLabeledPair(role: .content, id: "email", in: labelNS)
```

## Charts Accessibility

For Swift Charts:
- Add `.accessibilityLabel()` and `.accessibilityValue()` to marks.
- Implement `AXChartDescriptorRepresentable` for Audio Graph support.
- Provide a text summary of chart data for screen readers.

## iOS 26: Accessibility Nutrition Labels

Apps can now declare supported accessibility features in App Store Connect. Users can filter by:
- VoiceOver support
- Dynamic Type support
- High contrast
- Reduced motion
- Voice control
