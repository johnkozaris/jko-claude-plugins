# Animation and motion

Target: Swift 6.3, iOS 26, Xcode 26.

Animation in SwiftUI is mostly choosing the right primitive and pointing it at the right value. The framework handles interpolation, frame timing, completion callbacks, and accessibility downgrades. The mistakes are almost always in the choice of curve (a bouncy spring on a progress bar), the scope (an `.animation` modifier hung on a screen root), or the primitive (an `onTapGesture` doing what `Button` would do for free). This file goes through the cases worth knowing.

Cross-references:
- Liquid Glass morphing via `glassEffectID` — `liquid-glass.md`.
- Zoom navigation transitions — `navigation.md`.
- `@Animatable` macro mechanics — `modern-api.md`.
- Animation performance on lists — `performance.md`.
- Motion tokens in a design system — `design-system.md`.
- Reduce Motion handling — `accessibility.md`.

---

## Implicit and explicit

There are two ways to animate a change. Implicit animation says "when this value changes, animate the transition" and is expressed as a modifier:

```swift
Rectangle()
    .frame(height: isExpanded ? 200 : 50)
    .animation(.smooth, value: isExpanded)
```

Explicit animation says "this block of code is causing an animated change" and wraps the mutation:

```swift
Button("Toggle") {
    withAnimation(.bouncy) {
        isExpanded.toggle()
    }
}
```

Both produce the same visual result for a single-value change. The differences are about scope and authorship. The implicit form is local to the affected view, which makes it easy to compose. The explicit form is local to the mutation, which makes it easy to coordinate multiple changes that should animate together.

A few rules worth knowing:

- Always provide `value:` to `.animation(_:value:)`. The valueless `.animation(_:)` form is deprecated and animates too broadly — it animates anything in scope that changes.
- Implicit animations override explicit ones when both are in scope. The implicit modifier closest to the affected view wins.
- Scope `.animation(_:value:)` to the smallest subtree that needs it. Hanging it on a screen root animates every change anywhere in the screen, which is rarely what you want.
- `withAnimation` is the right tool when a user action changes multiple properties that should move together. It keeps the changes coherent in one animation block.

---

## Springs are the default

Springs are the right default for content moves — sheets, list rows, panels, expanding elements. The named springs (`.smooth`, `.snappy`, `.bouncy`) replace the old `.spring(response:dampingFraction:)` magic-number pair.

| Style       | Personality                            | Where to use it                          |
| ----------- | -------------------------------------- | ---------------------------------------- |
| `.smooth`   | No visible bounce, asymptotic settle   | Most content moves: sheets, list updates |
| `.snappy`   | Subtle bounce, quicker settle          | Interactive, tap-driven state changes    |
| `.bouncy`   | Visible overshoot                      | Celebratory moments only                 |

```swift
withAnimation(.smooth) { isExpanded.toggle() }     // most cases
withAnimation(.snappy) { selection = item }        // tap-driven
withAnimation(.bouncy) { showConfetti = true }     // celebration
```

When you need to tune one, do it through the named spring's parameters rather than by switching to the raw `.spring` initializer:

```swift
.animation(.snappy(duration: 0.4, extraBounce: 0.1), value: state)
.animation(.smooth(duration: 0.5), value: state)
```

The reason to prefer the named springs is consistency. A codebase that uses `.smooth` / `.snappy` / `.bouncy` everywhere reads consistently and centralizes tuning in one place. A codebase that uses `.spring(response: 0.32, dampingFraction: 0.7)` in one file and `.spring(response: 0.45, dampingFraction: 0.85)` in another is harder to keep coherent — the numbers don't tell you what they're trying to be.

Springs work as a default because they asymptotically approach the target rather than snapping to it. iOS itself uses non-bouncy springs throughout — sheet presentations, navigation pushes, app launches. Linear and ease curves snap to a stop at the end, which reads as mechanical against an OS that doesn't.

For new code, I'd treat raw `.spring(response:dampingFraction:)` as the deprecated form. The named springs do the same job with better defaults and a name that explains itself.

---

## Loaders and progress are linear

The one place springs are wrong is loaders, progress bars, and spinners. Progress should be linear because the underlying work is linear. A bouncy progress bar tells the user the bar is wrong, not that the work is wrong.

```swift
// Linear progress bar.
ProgressView(value: progress)
    .animation(.linear(duration: 0.2), value: progress)

// Continuous spinner with linear repeating rotation.
Circle()
    .trim(from: 0, to: 0.3)
    .stroke(.accent, lineWidth: 3)
    .rotationEffect(.degrees(rotation))
    .animation(
        .linear(duration: 1).repeatForever(autoreverses: false),
        value: rotation
    )
    .onAppear { rotation = 360 }
```

If a "progress" indicator is celebratory in nature — a confetti burst at completion, a checkmark that scales in when finished — that part can be bouncy. The bar itself stays linear.

---

## Chaining with completion

`withAnimation`'s completion closure fires when the animation actually finishes. Use it instead of timed `asyncAfter` calls:

```swift
Button("Animate") {
    withAnimation(.bouncy) {
        scale = 1.4
    } completion: {
        withAnimation(.smooth) {
            scale = 1.0
        }
    }
}
```

The hardcoded-delay form (`DispatchQueue.main.asyncAfter(deadline: .now() + 0.4)`) drifts when you change the animation curve and breaks when Reduce Motion shortens the animation. The completion closure is tied to the actual end of the animation.

---

## Transitions

Transitions animate view insertion and removal. The wiring catches people the first time — the animation context has to be outside the conditional, not inside:

```swift
VStack {
    if showDetail {
        DetailView()
            .transition(.move(edge: .bottom).combined(with: .opacity))
    }
}
.animation(.smooth, value: showDetail)
```

If the `.animation` is on the conditional view (inside the `if`), there's nothing for the transition to attach to and the view appears and disappears instantly. The animation has to live on a parent that survives both the inserted and removed states.

Asymmetric transitions let insertion and removal differ:

```swift
.transition(.asymmetric(
    insertion: .scale.combined(with: .opacity),
    removal: .opacity
))
```

For custom transitions, conform to the `Transition` protocol:

```swift
struct SlideAndFade: Transition {
    func body(content: Content, phase: TransitionPhase) -> some View {
        content
            .opacity(phase == .identity ? 1 : 0)
            .offset(x: phase == .didDisappear ? 100
                     : phase == .willAppear ? -100 : 0)
    }
}

DetailView().transition(SlideAndFade())
```

`TransitionPhase` has three cases: `.willAppear` (before the transition starts), `.identity` (fully present), and `.didDisappear` (after the transition completes). Your body modifies the content based on the phase.

I'd verify the exact case names against current docs if you're targeting older OS versions; the API stabilized in iOS 17 but specific cases may have evolved.

---

## `@Animatable` for custom modifiers (iOS 26+)

For custom view modifiers and shapes with animatable state, the modern form is the `@Animatable` macro. It auto-synthesizes the `animatableData` property from your stored properties:

```swift
@Animatable
struct PulseEffect: ViewModifier {
    var progress: Double                  // animatable
    @AnimatableIgnored var color: Color   // excluded from interpolation
    @AnimatableIgnored var label: String

    func body(content: Content) -> some View {
        content
            .scaleEffect(1 + progress * 0.2)
            .opacity(1 - progress * 0.4)
            .foregroundStyle(color)
    }
}
```

The macro generates the `animatableData` boilerplate. Stored properties are included by default; `@AnimatableIgnored` excludes a property from interpolation (typically because the type isn't `VectorArithmetic` or because animating it doesn't make sense).

For two-axis interpolation, the macro builds the `AnimatablePair` for you:

```swift
@Animatable
struct WaveOffset: ViewModifier {
    var x: Double
    var y: Double

    func body(content: Content) -> some View {
        content.offset(x: x, y: y)
    }
}
```

The macro generates `var animatableData: AnimatablePair<Double, Double>` automatically.

The manual form — implementing `Animatable` and writing `animatableData` by hand — still works and is what you'll see in code that predates iOS 26. For new types, the macro is the preferred form.

One thing I haven't verified: the exact iOS version that introduced `@Animatable`. I'd check the current docs before relying on it for a project that has to support older OSs. The macro should be iOS 26-only based on the API surface; older targets use the manual `animatableData` form.

---

## Phase animators

`PhaseAnimator` drives a value through a sequence of phases, animating between each. Useful for multi-step sequences like a notification badge that pulses, peaks, and settles:

```swift
enum Pulse: CaseIterable { case start, peak, settle }

PhaseAnimator(Pulse.allCases) { phase in
    Circle()
        .scaleEffect(phase == .peak ? 1.4 : 1.0)
        .opacity(phase == .settle ? 0.3 : 1.0)
} animation: { phase in
    switch phase {
    case .start, .settle: .smooth(duration: 0.4)
    case .peak:           .snappy(duration: 0.25)
    }
}
```

Each phase advances automatically. The `animation:` closure picks the curve per transition.

For event-driven phases — animate on every like, every save — use the trigger form:

```swift
PhaseAnimator(Pulse.allCases, trigger: likeCount) { phase in
    HeartIcon().scaleEffect(phase == .peak ? 1.4 : 1.0)
}
```

The sequence runs once per change to the trigger value.

`PhaseAnimator` respects Reduce Motion on its own — when the setting is on, the framework downgrades to a less motion-heavy form.

---

## Keyframe animators

`KeyframeAnimator` is for choreographed sequences where you need multiple tracks with different timing — a shake animation where rotation and scale follow different curves, for example.

```swift
struct WiggleValues {
    var rotation = 0.0
    var scale = 1.0
}

KeyframeAnimator(initialValue: WiggleValues(), trigger: shake) { values in
    Image(systemName: "bell.fill")
        .rotationEffect(.degrees(values.rotation))
        .scaleEffect(values.scale)
} keyframes: { _ in
    KeyframeTrack(\.rotation) {
        LinearKeyframe(0, duration: 0.05)
        SpringKeyframe(15, duration: 0.1)
        SpringKeyframe(-15, duration: 0.1)
        SpringKeyframe(10, duration: 0.1)
        SpringKeyframe(0, duration: 0.1)
    }
    KeyframeTrack(\.scale) {
        SpringKeyframe(1.1, duration: 0.2)
        SpringKeyframe(1.0, duration: 0.25)
    }
}
```

Use it for "scripted" animations — attention pulses, attention shakes, dance moves — where the granularity of `PhaseAnimator` isn't enough. For simple back-and-forth or grow-and-settle motion, the simpler primitives are usually clearer.

---

## `matchedGeometryEffect`

A "hero" transition between two views — a small thumbnail in a grid expanding into a full-screen detail, for example — uses `matchedGeometryEffect` to pair the two views by ID within a `@Namespace`:

```swift
@Namespace private var heroNS

if expanded {
    Image(.banner)
        .resizable()
        .matchedGeometryEffect(id: "hero", in: heroNS)
        .frame(maxWidth: .infinity, maxHeight: 400)
} else {
    Image(.banner)
        .resizable()
        .matchedGeometryEffect(id: "hero", in: heroNS)
        .frame(width: 60, height: 60)
        .clipShape(.circle)
}
```

Wrap the state change in `withAnimation` so the geometry interpolation has a curve to follow:

```swift
withAnimation(.smooth) { expanded.toggle() }
```

A few rules:

- One id per geometric pairing. Two unrelated `matchedGeometryEffect(id: "shared")` calls in the same namespace will fight over which view to track.
- One hero per transition. Applying the effect to every view in the subtree usually looks worse than picking the load-bearing element.
- For transitions across navigation boundaries (push, sheet), `.navigationTransition(.zoom(sourceID:in:))` plus `.matchedTransitionSource(id:in:)` is the right tool. `matchedGeometryEffect` can't reach across a navigation stack. See `navigation.md`.

---

## Liquid Glass morphing (iOS 26)

Glass surfaces can morph between shapes via `glassEffectID`. The mechanism is similar to `matchedGeometryEffect` — a `@Namespace` pairs two glass elements with the same id:

```swift
@Namespace private var glassNS

if expanded {
    ExpandedPanel()
        .glassEffect(in: .rect(cornerRadius: 24, style: .continuous))
        .glassEffectID("panel", in: glassNS)
} else {
    CollapsedChip()
        .glassEffect(in: .capsule)
        .glassEffectID("panel", in: glassNS)
}
```

```swift
withAnimation(.smooth) { expanded.toggle() }
```

The glass material morphs shape and size; content above the glass cross-fades. `matchedGeometryEffect` isn't the right tool here because the glass background is the load-bearing element — the morph is about the material, not just the geometry. See `liquid-glass.md` for the longer treatment.

---

## Symbol effects

SF Symbols have first-class animation modifiers. Reach for them before `.rotationEffect` or `.scaleEffect` tricks:

```swift
// Continuous animation while a value is true.
Image(systemName: "wifi")
    .symbolEffect(.variableColor.iterative, isActive: isSearching)

// Discrete bounce on a value change.
Image(systemName: "heart")
    .symbolEffect(.bounce, value: likeCount)

// Pulse while a state is on.
Image(systemName: "recordingtape")
    .symbolEffect(.pulse, isActive: isRecording)

// Replace one symbol with another, animated.
Image(systemName: isPlaying ? "pause.fill" : "play.fill")
    .contentTransition(.symbolEffect(.replace))
```

Symbol effects use the multi-layer rendering of recent SF Symbols and respect Reduce Motion on their own. They look better than hand-rolled symbol animations because they're tuned by the people who designed the symbols.

---

## Sensory feedback (haptics)

`.sensoryFeedback(_:trigger:)` is the SwiftUI primitive for haptic and audio feedback bound to a value change:

```swift
Button("Save") { save() }
    .sensoryFeedback(.success, trigger: saveCount)

ScrollView { ... }
    .sensoryFeedback(.selection, trigger: selectedID)

Form { ... }
    .sensoryFeedback(.error, trigger: validationFailed)
```

Prefer semantic values (`.success`, `.warning`, `.error`, `.selection`, `.alignment`) when one fits. They're tuned by Apple to feel right in their named contexts. Raw `.impact(weight:)` is the fallback when no semantic value fits.

Don't reach for `UIImpactFeedbackGenerator` directly in SwiftUI code. The UIKit haptic generators predate `sensoryFeedback`, don't bind to state changes the way the modifier does, and don't respect the SwiftUI view lifecycle. The SwiftUI form picks up the user's haptic accessibility preferences (and silences accordingly) without you wiring anything up.

One haptic per user action is the right amount. Stacking impact plus selection on the same gesture reads as fatigue. See `accessibility.md` for the policy under Reduce Motion.

---

## Metal shaders

For effects that go beyond what built-in modifiers express — refraction, displacement, generative color — SwiftUI exposes three modifiers that pipe view content through a Metal fragment shader.

| Modifier              | Use for                                          | Cost      |
| --------------------- | ------------------------------------------------ | --------- |
| `.colorEffect`        | Per-pixel color transformation (tint, posterize) | Cheap     |
| `.distortionEffect`   | Warping via neighboring-pixel sampling           | Medium    |
| `.layerEffect`        | Full sampler access; most powerful               | Expensive |

Shaders live in `.metal` files and are referenced through `ShaderLibrary`. The framework uses `@dynamicMemberLookup` so you can pass shader arguments by name.

To drive a shader with time, pair it with `TimelineView(.animation)`:

```swift
TimelineView(.animation) { context in
    Image(.banner)
        .colorEffect(
            ShaderLibrary.rainbow(
                .float(context.date.timeIntervalSinceReferenceDate)
            )
        )
}
```

Argument types I'd reach for: `.float`, `.float2`, `.float3`, `.float4` for scalars and vectors; `.color` for colors; `.image` for sampling another image; `.boundingRect` for the view's bounds (auto-supplied).

A few rules:

- Wrap `TimelineView` tightly around the affected view, not the whole screen. Time-driven body re-evaluation runs every frame.
- `.colorEffect` is the cheap option. Use it when per-pixel color suffices and reach for `.layerEffect` only when you genuinely need sampler access.
- For `.distortionEffect` and `.layerEffect`, provide a real `maxSampleOffset`. Over-estimating wastes texture bandwidth; under-estimating produces sampling artifacts.

This is a brief overview. For a deep dive on shader authoring, Apple's Metal documentation and the WWDC sessions on SwiftUI shaders are the canonical references.

---

## GPU-friendly transforms

Some animation properties run on the render thread without re-running layout. Others trigger the parent's layout pass on every frame. The first group is much cheaper.

| Cheap (renders without layout)    | Expensive (triggers layout)              |
| --------------------------------- | ---------------------------------------- |
| `.offset(x:y:)`                   | Animating `.frame(width:height:)`        |
| `.scaleEffect(_:)`                | Animating `.padding(_)`                  |
| `.rotationEffect(_:)`             | Animating `Spacer().frame()`             |
| `.opacity(_)`                     | Animating insertion or removal           |
| `.blur(radius:)` (cached layer)   | Animating any layout-driving `@State`    |

The cheap group is composited by the render thread without re-running layout. The expensive group re-runs the parent's layout pass every frame.

So when you need to "grow" a view on tap, animating a `.scaleEffect` is much cheaper than animating its `.frame` height. When you need a hero effect, an `offset` plus `scaleEffect` on a fixed-size view is cheaper than morphing the frame.

There are cases where you must animate layout — a sheet that expands, a cell that reveals a long body. Use `.smooth` and accept the cost; the animation is still smooth at 60 or 120 fps on modern hardware as long as the body itself isn't expensive.

I don't have a specific benchmark I'd cite for "how much faster scale is than frame." The structural difference is real — one re-runs layout per frame, the other doesn't — but the practical impact depends on the layout complexity. Measure with Instruments if it's a hot spot.

---

## Reduce Motion

`@Environment(\.accessibilityReduceMotion)` is the user's signal that they'd prefer less motion. Vestibular sensitivity is the common reason, but the setting is for any user who wants a calmer interface.

```swift
@Environment(\.accessibilityReduceMotion) private var reduceMotion

withAnimation(reduceMotion ? .linear(duration: 0.15) : .bouncy) {
    isExpanded.toggle()
}
```

A short linear or `nil` animation is a reasonable substitute for a motion-heavy one. The change of state is still visible; the motion that bothered the user isn't.

Transitions should swap to opacity-only when Reduce Motion is on:

```swift
.transition(reduceMotion
    ? .opacity
    : .move(edge: .bottom).combined(with: .opacity))
```

`PhaseAnimator` and symbol effects respect Reduce Motion automatically. Custom `Transition` types and bespoke `withAnimation` calls don't — that's where you have to do the work. See `accessibility.md` for the full Reduce Motion policy.

---

## Centralizing motion as design tokens

Animation values, like color and spacing values, deserve a single source of truth. A small `Motion` enum keeps the names consistent across the app:

```swift
enum Motion {
    static let smooth:   Animation = .smooth(duration: 0.30)
    static let snappy:   Animation = .snappy(duration: 0.30)
    static let bouncy:   Animation = .bouncy(duration: 0.45)
    static let fadeIn:   Animation = .easeOut(duration: 0.20)
    static let fadeOut:  Animation = .easeIn(duration: 0.15)
    static let progress: Animation = .linear(duration: 1.0)
        .repeatForever(autoreverses: false)
}

.animation(Motion.snappy, value: selection)
```

This lets you tune one value and have the whole codebase pick it up. It also makes the intent of an animation more readable at the call site — `Motion.fadeIn` reads better than `.easeOut(duration: 0.20)`. See `design-system.md` for the broader pattern.

---

## A few specific don'ts

- Raw `.spring(response:dampingFraction:)` in new code. Use `.smooth` / `.snappy` / `.bouncy`; tune through their parameters.
- `.animation(_:)` without a `value:`. The valueless form is deprecated and animates too broadly.
- Bouncy springs on progress bars or loaders. Loaders are linear.
- Hand-written `animatableData` on new types. Use the `@Animatable` macro.
- `UIImpactFeedbackGenerator` in SwiftUI code. `.sensoryFeedback(_:trigger:)` instead.
- Animating `.frame()` for hero transitions. `matchedGeometryEffect` or `.navigationTransition(.zoom(...))` instead.
- `.animation(_:value:)` on a screen-root when one subtree needs the animation. Scope it as tightly as you can.
- `DispatchQueue.main.asyncAfter` to chain animations. `withAnimation { } completion:` is the wired-up form.
- Magic-number `Animation(...)` constants scattered across files. Centralize as design tokens.
- `matchedGeometryEffect` for morphing two glass elements. `glassEffectID(_:in:)` is the right tool.
- Ignoring Reduce Motion. Read the environment value and downgrade.
- `.layerEffect` where `.colorEffect` would do. The expensive option for cases the cheap option can't express.
- Hundreds of subviews animating individually. Consider `.drawingGroup()` to rasterize or a single shader if the bottleneck is real.
