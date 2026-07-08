# Liquid Glass (iOS 26 / macOS 26)

Apple's iOS 26 / macOS Tahoe visual material. Real-time-rendered layer floating above content. Auto-adopts on structural chrome (`TabView`, `NavigationStack`, `Toolbar`, sheets) when built against SDK 26.

**The honest stance:** adoption is your call. The "non-negotiable" framing conflates three things — pick them apart:

1. **SDK 26 build requirement** (April 28, 2026) — yes, mandatory.
2. **System chrome auto-adopts glass under SDK 26** — automatic, not a choice.
3. **"You can't theme your own app"** — false. App-level theming is fully alive.

Real-world adoption among popular maintained SwiftUI OSS apps one year post-launch is still a small minority. At iOS 26.0 launch on Sept 15, 2025, many Apple pro and content apps — Pages, Numbers, Keynote, Final Cut Pro, iMovie, QuickTime Player, Pixelmator Pro, Chess — shipped without Liquid Glass (per AppleInsider, 2025-09-16); most got glass updates in early 2026. Community observation of visible UI behavior also indicates major third-party apps (WhatsApp delayed, Telegram built its own, Spotify removed a glass icon effect) handled rollout differently. Don't apologize for opting out — Apple's own pro apps did. (The `UIDesignRequiresCompatibility` flag is not directly observable outside an app, so third-party attribution is inference from UI behavior, not confirmed flag inspection.)

---

## The three adoption paths

### Path A — Selective (default, ~90% of apps)

System chrome adopts glass automatically. Your custom views and content stay yours. Add `.glassEffect()` selectively to your own chrome (accessory bars, custom toolbars).

```swift
HStack {
    Button(action: action1) { Image(systemName: "play.fill") }
    Button(action: action2) { Image(systemName: "pause.fill") }
}
.padding()
.glassEffect(.regular.interactive(), in: .capsule)
```

Productivity, RSS, social, utility apps land here.

### Path B — Custom chrome (brand-heavy apps)

Build your own tab bar / nav header / toolbar. Glass becomes invisible to your app. Robinhood model for fintech, brokerage, banking.

```swift
struct CustomTabBar: View {
    @Bindable var router: AppRouter
    var body: some View {
        HStack(spacing: 0) {
            ForEach(Tab.allCases, id: \.self) { tab in
                Button {
                    router.selectedTab = tab
                } label: {
                    VStack(spacing: 4) {
                        Image(systemName: tab.icon)
                        Text(tab.title).font(.caption2)
                    }
                    .frame(maxWidth: .infinity)
                    .foregroundStyle(router.selectedTab == tab ? Color.brand : .secondary)
                }
            }
        }
        .padding(.vertical, 8)
        .background(Color.background.primary)  // your brand, not glass
    }
}
```

### Path C — Opt out one cycle

```xml
<!-- Info.plist -->
<key>UIDesignRequiresCompatibility</key>
<true/>
```

Defers the new material for one OS cycle. Apple has indicated this flag is intended for debugging and migration and will be ignored in a future major Xcode/SDK release. The community commonly attributes that to Xcode 27 / iOS 27 SDK, but Apple has not published a specific deadline in writing — so treat it as "one-cycle escape, removal expected eventually." (The launch-day evidence that Apple's own pro apps shipped this way is in the intro above.)

---

## Applying `.glassEffect()` correctly

```swift
// Default (regular glass, default capsule shape)
.glassEffect()

// Less prominence — high-transparency variant
.glassEffect(.clear)

// Interactive (tappable / focusable) — chain .interactive() on Glass
.glassEffect(.regular.interactive(), in: .capsule)

// Tinted (semantic — primary action / state / error)
.glassEffect(.regular.tint(.accentColor).interactive(), in: .capsule)

// Custom clip shape
.glassEffect(.regular, in: .rect(cornerRadius: 16))

// Disable per-view (defer this region to the underlying chrome)
.glassEffect(.identity)
```

`Glass` exposes `.regular`, `.clear`, and `.identity` as variants. Chainable instance methods: `.tint(_:)` and `.interactive()`. There is no `.thin` variant — use `.clear` for less prominence.

### `GlassEffectContainer` for groups

When multiple glass elements coexist in one region, wrap in `GlassEffectContainer` to share the sampling region (avoids per-element re-sampling).

```swift
GlassEffectContainer {
    HStack {
        button1.glassEffect()
        button2.glassEffect()
        button3.glassEffect()
    }
}
```

### Morphing with `glassEffectID(_:in:)`

```swift
@Namespace private var morphNamespace

VStack {
    if isExpanded {
        ExpandedView()
            .glassEffectID("panel", in: morphNamespace)
    } else {
        CompactView()
            .glassEffectID("panel", in: morphNamespace)
    }
}
.animation(.smooth, value: isExpanded)
```

### Buttons

```swift
Button("Done") { dismiss() }
    .buttonStyle(.glassProminent)

Button("Cancel") { dismiss() }
    .buttonStyle(.glass)
```

### Tinting

```swift
// Good — semantic tint for primary action
.glassEffect(.regular.tint(.accentColor))

// Good — error state
.glassEffect(.regular.tint(.red))

// Bad — decorative tint with no semantic meaning
.glassEffect(.regular.tint(.purple))  // ✗
```

### iOS 26 sheets

```swift
.sheet(isPresented: $showSettings) {
    SettingsView()
        // Partial detents opt the sheet into the new iOS 26 presentation styling
        .presentationDetents([.medium, .large])
        // DO NOT add .presentationBackground(.thinMaterial) — suppresses the new style
}
```

Use `ConcentricRectangle` + `.containerShape()` for corner harmony with sheet edges.

---

## Never apply glass

- ❌ List rows / table rows
- ❌ Content tiles, media canvases
- ❌ Full-screen backgrounds
- ❌ Nested glass (glass-on-glass)
- ❌ Text directly on glass — text on opaque layers only
- ❌ Decorative tint (semantic tint only)
- ❌ `.presentationBackground(.thinMaterial)` on iOS 26 sheets
- ❌ Custom toolbar item backgrounds (they interfere with scroll-edge effect)

---

## Accessibility integration

- **Reduce Transparency** → glass becomes frosted/opaque. Test under this.
- **Increase Contrast** → predominantly black/white with borders.
- **Reduce Motion** → disables elastic glass properties.

Many Mac users run with Increase Contrast permanently. Test all three.

---

## Test on device

Simulator doesn't render specular highlights or motion-reactive glass correctly. Always validate on hardware before shipping.

---

## UIKit form (under-documented)

Apple shipped glass for BOTH SwiftUI AND UIKit. Most tutorials only document the SwiftUI form, but `UIGlassEffect` + `UIVisualEffectView` works in UIKit apps. Amperfy (1.5k stars, music app) uses this in production.

The shape of the UIKit API (sketch — the exact property surface is under-documented; **verify against the SDK headers or sosumi.ai before citing specifics in a review**):

```swift
let effect = UIGlassEffect()
effect.tintColor = .systemBlue          // semantic tint
effect.isInteractive = true             // opt into interactive
let view = UIVisualEffectView(effect: effect)
view.frame = bounds
addSubview(view)
```

For grouping multiple glass elements in UIKit, use `UIGlassContainerEffect`. If your app is UIKit-majority, you don't have to bridge to SwiftUI for glass.

---

## Community sentiment — the verdict that matters for reviews

Reception is **platform-split**, and the split is what changes your advice:

- **On iOS**: mixed-to-positive. Even the loudest skeptic camp concedes the iOS implementation is good — Daring Fireball's 2025 Apple Report Card gave iOS 26 an A ("Apple's best implementation of the Liquid Glass concept, by far") while giving macOS Tahoe a C and calling it a historic regression. Apple's iOS-first apps (Messages, Photos, Music, Camera) shipped glass and have not reverted. Defaulting to Path A is fine.
- **On macOS**: substantially negative among the named Mac critic blogs (Daring Fireball, inessential.com, lapcatsoftware), with specific legibility complaints, and even glass-critical developers who eventually adopted it (NetNewsWire) did so reluctantly. For a Mac-first app, take Path C seriously and don't apologize.

When a developer asks "is Liquid Glass hated?", give the platform-split answer, not a blanket one — and note that user-facing App Store reviews of major adopters do not show the developer-community criticism.

---

## The decision

| App archetype | Path | Why |
|---|---|---|
| Productivity / utility / RSS / social | A (selective) | System chrome adopts; your content stays yours |
| Fintech / brokerage / banking / brand-heavy | B (custom chrome) | Brand chrome IS the product — bypass glass |
| Creative / pro / document-heavy not ready | C (opt out) | One-cycle escape — Apple's own pro apps did this |

Don't apologize for opting out. Apple's own apps did.

---

## Anti-patterns recap

- `.glassEffect()` on list rows, content tiles, full-screen backgrounds
- Glass-on-glass (nested)
- Text directly on glass surfaces
- Decorative (non-semantic) tint on glass
- `.presentationBackground(.thinMaterial)` on iOS 26 sheets
- Apologizing for `UIDesignRequiresCompatibility = true` — Apple's own iWork suite did

---

## Cross-references

- `references/design-system.md` — materials and token strategy
- `references/accessibility.md` — Reduce Transparency / Increase Contrast / Reduce Motion testing
- `references/anti-patterns.md` — full don't list
