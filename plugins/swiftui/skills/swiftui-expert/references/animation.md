# Animation & Motion

## Implicit vs Explicit

**Implicit** — `.animation(_:value:)` modifier. Animates property changes automatically.
**Explicit** — `withAnimation { }` block. Only changes inside the block animate.

```swift
// Implicit — animates whenever `isExpanded` changes
Rectangle()
    .frame(height: isExpanded ? 200 : 50)
    .animation(.smooth, value: isExpanded)

// Explicit — animates only when you say so
Button("Toggle") {
    withAnimation(.bouncy) {
        isExpanded.toggle()
    }
}
```

Always provide `value:` with `.animation()`. Never use the `animation(_ animation: Animation?)` variant without a value — it animates too broadly and is strongly discouraged.

Implicit animations override explicit ones (later in view tree wins).

## Spring Animations

Springs feel natural. Use them as defaults:

| Style                       | Use Case                         |
| --------------------------- | -------------------------------- |
| `.smooth`                   | Subtle, professional transitions |
| `.bouncy`                   | Playful, attention-grabbing      |
| `.snappy`                   | Quick, responsive                |
| `.spring(duration:bounce:)` | Custom control                   |

Springs never end abruptly — they asymptotically approach the target. Non-bouncy springs are used throughout iOS (app launches, sheet presentations, navigation).

## Chaining Animations

Use `withAnimation` completion closures, never delays:

```swift
Button("Animate") {
    withAnimation(.bouncy) {
        scale = 2
    } completion: {
        withAnimation(.smooth) {
            scale = 1
        }
    }
}
```

## Transitions

Transitions animate view insertion/removal. They need animation context **outside** the conditional:

```swift
VStack {
    if showDetail {
        DetailView()
            .transition(.move(edge: .bottom).combined(with: .opacity))
    }
}
.animation(.smooth, value: showDetail)  // Animation context here
```

### Asymmetric Transitions

```swift
.transition(.asymmetric(
    insertion: .scale.combined(with: .opacity),
    removal: .opacity
))
```

### Custom Transitions (iOS 17+)

Conform to `Transition` protocol for reusable custom transitions.

## Phase Animator (iOS 17+)

Multi-step animation sequences:

```swift
PhaseAnimator([false, true]) { value in
    Circle()
        .scaleEffect(value ? 1.2 : 1.0)
        .opacity(value ? 0.5 : 1.0)
}
```

## Keyframe Animator (iOS 17+)

Precise timing control with multiple properties:

```swift
KeyframeAnimator(initialValue: AnimationValues()) { values in
    Circle()
        .scaleEffect(values.scale)
        .offset(y: values.yOffset)
} keyframes: { _ in
    KeyframeTrack(\.scale) {
        SpringKeyframe(1.5, duration: 0.3)
        SpringKeyframe(1.0, duration: 0.3)
    }
    KeyframeTrack(\.yOffset) {
        LinearKeyframe(-50, duration: 0.2)
        SpringKeyframe(0, duration: 0.4)
    }
}
```

## @Animatable Macro (iOS 26+)

Auto-synthesizes `animatableData`:

```swift
@Animatable
struct PulseEffect: ViewModifier {
    var progress: Double        // Automatically animatable
    @AnimatableIgnored var color: Color  // Excluded

    func body(content: Content) -> some View {
        content.scaleEffect(1 + progress * 0.2)
    }
}
```

Replaces manual `AnimatablePair` boilerplate.

## matchedGeometryEffect

Hero-style transitions between views:

```swift
@Namespace private var animation

// Source
Image(.photo)
    .matchedGeometryEffect(id: "hero", in: animation)

// Destination (shown conditionally)
Image(.photo)
    .matchedGeometryEffect(id: "hero", in: animation)
```

Wrap state change in `withAnimation` to control timing. Use unique identifiers. Avoid overuse on complex layouts.

## Liquid Glass Morphing (iOS 26+)

Glass elements can morph between states:

```swift
@Namespace private var glassNS

// Source
Text("Tap me")
    .glassEffect(in: .capsule)
    .glassEffectID("morph", in: glassNS)

// After state change
ExpandedView()
    .glassEffect(in: .rect(cornerRadius: 20))
    .glassEffectID("morph", in: glassNS)
```

Sheets can morph from toolbar items using `navigationZoomTransition`.

## Symbol Effects

```swift
Image(systemName: "wifi")
    .symbolEffect(.variableColor.iterative, isActive: isSearching)

Image(systemName: "heart")
    .symbolEffect(.bounce, value: likeCount)
```

## Metal Shaders (iOS 17+)

SwiftUI exposes three modifiers that pipe view content through a Metal fragment shader. Shaders live in a `.metal` file and are referenced through `ShaderLibrary`.

| Modifier                                          | Use Case                                                                       |
| ------------------------------------------------- | ------------------------------------------------------------------------------ |
| `.colorEffect(_:isEnabled:)`                      | Per-pixel color transformation. Cheap.                                         |
| `.distortionEffect(_:maxSampleOffset:isEnabled:)` | Warps geometry by sampling neighboring pixels. Must declare `maxSampleOffset`. |
| `.layerEffect(_:maxSampleOffset:isEnabled:)`      | Full sampler access — most powerful, most expensive.                           |

### Driving a shader with time

Pair with `TimelineView(.animation)` to feed a continuously updating `time` parameter:

```swift
TimelineView(.animation) { context in
    Image(.banner)
        .colorEffect(
            ShaderLibrary.rainbow(.float(context.date.timeIntervalSinceReferenceDate))
        )
}
```

### Argument passing

`ShaderLibrary` uses dynamic member lookup. Pass arguments using `Shader.Argument`:

- `.float(...)`, `.float2(...)`, `.float3(...)`, `.float4(...)` for scalars/vectors
- `.color(...)` for colors
- `.image(Image(...))` for sampling another image
- `.boundingRect` for the view's bounds (auto-supplied)

### Performance rules

- Shaders run on the GPU — cheap to scale, but every frame re-evaluates `body` if you bind to time.
- Wrap the `TimelineView` tightly around the affected view, not whole screens.
- `.layerEffect` is expensive; prefer `.colorEffect` when per-pixel color is enough.
- Always provide a real `maxSampleOffset` for `.distortionEffect` / `.layerEffect` — over-estimating wastes texture bandwidth, under-estimating causes sampling artifacts.
- Combine with `.drawingGroup()` only when SwiftUI's compositor is the bottleneck — shader effects already rasterize.

### Liquid Glass interplay

Shader effects sit _underneath_ `.glassEffect()`. To produce custom glass-like materials on iOS 25 and earlier, a `.layerEffect` with a refraction shader is the canonical fallback (see r/SwiftUI shader showcases).

## Performance

- Prefer GPU-friendly transforms (`offset`, `scaleEffect`, `rotationEffect`) over layout changes (`frame`).
- Use `.drawingGroup()` to rasterize complex animated views.
- Avoid animating views with many subviews.

## Accessibility

Always respect Reduce Motion:

```swift
@Environment(\.accessibilityReduceMotion) private var reduceMotion

withAnimation(reduceMotion ? nil : .bouncy) {
    isExpanded.toggle()
}
```

Replace motion-based animations with opacity when Reduce Motion is on.
