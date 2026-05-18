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

Defers the new material for one OS cycle. Apple has indicated this flag is intended for debugging and migration and will be ignored in a future major Xcode/SDK release. The community commonly attributes that to Xcode 27 / iOS 27 SDK, but Apple has not published a specific deadline in writing — so treat it as "one-cycle escape, removal expected eventually."

At iOS 26.0 launch (Sept 15, 2025), many of Apple's own pro and content apps shipped without Liquid Glass — including Pages, Numbers, Keynote, Final Cut Pro, iMovie, QuickTime Player, Pixelmator Pro, and Chess (AppleInsider, 2025-09-16). Most got updates in early 2026. Third-party adoption was uneven: WhatsApp delayed full glass adoption (WABetaInfo, Oct 2025); Telegram built its own glass-like design rather than Apple's; Spotify removed a glass icon effect. These observations are community-derived from visible UI behavior, not from Apple-announced opt-out lists — the `UIDesignRequiresCompatibility` flag is not directly observable outside an app.

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
        // Required: declare at least one partial detent
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

Simulator doesn't render specular highlights or motion-reactive glass correctly. Always validate on hardware before shipping (Kavsoft).

---

## UIKit form (under-documented)

Apple shipped glass for BOTH SwiftUI AND UIKit. Most tutorials only document the SwiftUI form, but `UIGlassEffect` + `UIVisualEffectView` works in UIKit apps. Amperfy (1.5k stars, music app) uses this in production.

`UIGlassEffect` has a parameterless initializer — the regular/clear distinction is SwiftUI-only. Configure via properties:

```swift
let effect = UIGlassEffect()
effect.tintColor = .systemBlue                      // optional, semantic tint
effect.isInteractive = true                         // optional, opt into interactive
effect.cornerConfiguration = .containerRelative     // optional, shape from container
let view = UIVisualEffectView(effect: effect)
view.frame = bounds
addSubview(view)
```

For grouping multiple glass elements in UIKit, use `UIGlassContainerEffect`. If your app is UIKit-majority, you don't have to bridge to SwiftUI for glass.

---

## Community sentiment (honest snapshot)

Reception is **platform-split**: iOS mixed-to-positive, macOS substantially negative. The plugin presents both sides because the right call depends on what platform you're shipping to and what kind of app you're building.

### Pro-glass positions

- **MacStories** has framed glass as a generational redesign worth riding: only Apple is positioned to build a real-time material rendering system at this fidelity.
- **Apple's NYC design lab** — community reporting after attending Apple's design conversations indicates Apple was "genuinely shocked some devs think it's getting rolled back" and confirmed glass is the long-term direction. They framed it as an "iOS 7-style reset where foundational stability came first."
- **iOS user testing** generally: most users do not notice the material as a problem. The "users hate this" frame does not show up in App Store reviews of major adopters.
- **Hacker News and other community channels** carry roughly even numbers of positive and negative iPhone-specific reactions ("honestly a joy to use… delightful" appears regularly).
- **Apple's own iOS-first apps** (Messages, Photos, Music, Camera) shipped glass and have not reverted.

### Anti-glass positions

- **Daring Fireball's 2025 Apple Report Card**: gave Mac a C grade and called macOS Tahoe "the worst regression in the entire history of MacOS." The author refused to install Tahoe on his own machine. Crucially, the same report gave iOS 26 an A and called it "Apple's best implementation of the Liquid Glass concept, by far... I prefer it, in just about every way, to iOS 18." The position is platform-split, not blanket anti-glass.
- **inessential.com** (the NetNewsWire developer's blog): "Liquid Glass is Liquid-Glass-centric... blurry, illegible, and physically unstable." NetNewsWire 7 eventually shipped glass and the author called it "cool" — anti in principle, pro in adoption.
- A widely-shared industry critique: "It doesn't get out of the way of your content — it INVADES your content."
- Another: "perhaps the most getting-in-the-way user interface I've experienced in my lifetime. It never shuts up."
- **lapcatsoftware** posts call it "Liquid Crass" and document specific legibility regressions.
- A Mac-focused indie shop reported: "We've had zero customers request to adopt Liquid Glass for any of our Mac apps."

### Reddit testimony (collected via direct curl with a browser User-Agent)

- "Forcing the user to scroll the main content out of the way so they can read the tab bar is a horrible indictment of the core Liquid Glass usability problem."
- "Apple's screenshots of their notification screen with liquid glass looks impossible to read."
- "I have to use my custom tab bar."
- "It's been week+ since full version of iOS got released but absolutely none of the apps I use has any liquid glass in it."

### Net read

- **On iOS**, criticism is loud but adoption is moving forward and even prominent skeptics concede the iOS implementation is good. Defaulting to Path A (selective) is fine.
- **On Mac**, criticism is broad and named. If you're shipping a Mac-first app, take Path C seriously and don't apologize. If you're shipping iOS, the critic camp may not match your users.

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
