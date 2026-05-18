---
description: Explain a SwiftUI/Swift concept with strong opinions, modern patterns, code samples, and pointers to deeper references. Teaches like a veteran architect.
allowed-tools:
  - Read
  - Glob
  - Grep
  - Skill
argument-hint: "<topic to explain>"
---

# SwiftUI Teach

Load the `swiftui-expert` skill, then teach the requested concept like a veteran architect — strong opinions, modern patterns, concrete examples, no hedging.

## How to teach

When the user invokes `/swift-teach <topic>`:

1. **Identify the topic** from `$ARGUMENTS`. If unclear, ask one clarifying question — don't guess across multiple topics.

2. **Identify the relevant reference file(s)** from the skill:
   - Architecture / MV vs MVVM / TCA / folder structure → `architecture.md`
   - `@Observable` / `@State` / `@Bindable` / `@AppStorage` → `state-and-observation.md`
   - `NavigationStack` / routing / sheets → `navigation.md`
   - `body` re-eval / `init` / `.task` vs `onAppear` → `lifecycle.md`
   - View extraction / modifiers / `@ViewBuilder` → `view-composition.md`
   - MainActor / Sendable / actors / `.task` → `concurrency.md`
   - Tokens / typography / Dynamic Type / `@Entry` → `design-system.md`
   - SwiftData / Core Data / Keychain → `persistence.md`
   - App Intents / Widgets / Live Activities / permissions → `ios-platform.md`
   - Main menu / MenuBarExtra / sandbox / notarytool / Sparkle → `macos-platform.md`
   - Springs / `@Animatable` / matchedGeometry → `animation.md`
   - VoiceOver / Dynamic Type / Reduce Motion → `accessibility.md`
   - Lazy stacks / `_printChanges` / Instruments → `performance.md`
   - Liquid Glass → `liquid-glass.md`
   - Swift Testing / `os.Logger` / Instruments → `testing-and-debugging.md`
   - Deprecation table → `modern-api.md`
   - Language idioms (optionals, errors, generics) → `swift-idioms.md`
   - Anti-patterns → `anti-patterns.md`

3. **Teach the concept** with this structure:

### A. The one-line stance (opinionated)
- State the default position. No "consider"; pick the side.

### B. Why this is the default
- 1-2 paragraphs of mechanism + intent.
- Cite Apple framework behavior, established Swift teaching positions, or community evidence — by source (Apple HIG / WWDC session / a community blog), not by personal name.

### C. The code (modern shape)
- A complete, runnable example showing the recommended pattern.
- Comments mark the load-bearing decisions.

### D. The traps
- 2-4 mistakes Claude sees most often. For each:
  - The mistake (1 line).
  - Why it bites (1 line).
  - The fix (code snippet if non-obvious).

### E. When to deviate (concrete triggers)
- If this concept has genuine conditional rules, name them. App archetype / team size / regulatory context.
- If the rule is universal, say so explicitly. Don't manufacture exceptions.

### F. Deeper reading
- Pointer to the relevant reference file with the most informative section.
- Optional: external resource (Apple WWDC session, established Swift-teaching blog, project README).

## Style rules

- **Like a senior architect.** Don't hedge. State defaults firmly. Name the exceptions briefly.
- **Code over prose** where possible. The 30-line example is worth more than the 300-word explanation.
- **No false balance.** If MV beats MVVM by a strong corpus margin, say so. Don't pretend they're equivalent.
- **Cite sources, not people.** "Apple's Backyard Birds sample uses..." / "The MV State Pattern position argues..." / "A common Reddit refrain on the other side..." — gives the user permission to take a side without depending on personality cult.
- **Modern only.** Don't include UIKit-era artifacts. Don't include `ObservableObject` as a "still valid" pattern.
- **Length: ~400-800 words.** Tight. Don't pad.

## Example: `/swift-teach state ownership`

### The one-line stance

`@State` to own, plain `let` to receive, `@Bindable` for bindings, `.environment(_:)` to share — that's the entire matrix for `@Observable` types.

### Why this is the default

SwiftUI's `@Observable` (Swift 5.9+, iOS 17+) does **keypath-precise** observation: only views that READ the property re-render. The state-wrapper choice is about **who owns** the lifetime, not who watches for changes — the observation tracking is free.

- `@State` says "this view creates and retains this instance for the node's lifetime."
- Plain `let` says "I receive this from a parent; I don't own it. SwiftUI still tracks reads."
- `@Bindable` says "I receive this and need `$model.field` binding syntax."
- `.environment(_:)` says "I share this down the tree."

This matrix replaces the UIKit-era `@StateObject` / `@ObservedObject` / `@EnvironmentObject` trio.

### The code

```swift
// 1. Define an @Observable
@Observable @MainActor
final class Theme {
    var accent: Color = .blue
    var radius: CGFloat = 12
}

// 2. Own at the App root
@main struct MyApp: App {
    @State private var theme = Theme()  // OWN
    var body: some Scene {
        WindowGroup {
            ContentView().environment(theme)  // SHARE
        }
    }
}

// 3. Receive read-only
struct ContentView: View {
    @Environment(Theme.self) private var theme  // SHARED RECEIVE
    var body: some View {
        SettingsRow(theme: theme)  // PASS DOWN
    }
}

// 4. Receive read-only as `let` (no wrapper)
struct SettingsRow: View {
    let theme: Theme  // RECEIVE READ-ONLY
    var body: some View {
        Text("Accent").foregroundStyle(theme.accent)
    }
}

// 5. Receive WITH bindings
struct ThemeEditor: View {
    @Bindable var theme: Theme  // RECEIVE WITH BINDINGS
    var body: some View {
        Slider(value: $theme.radius, in: 0...20)
    }
}
```

### The traps

1. **`@StateObject` for new `@Observable` code.** Legacy wrapper for `ObservableObject`. Use `@State` for `@Observable`.

2. **`@AppStorage` directly inside `@Observable class`.** Compiles, doesn't trigger view updates. Use a nested storage class:
   ```swift
   @Observable class Settings {
       var storage = SettingsStorage()
   }
   @Observable class SettingsStorage {
       @AppStorage("theme") var theme = "dark"
   }
   ```

3. **`@State` to receive from a parent.** `@State` captures its initial value once and ignores parent updates. Use plain `let`.

4. **`.environmentObject(_:)` for `@Observable`.** Legacy API for `ObservableObject`. Use `.environment(_:)`.

### When to deviate

This rule is universal for new `@Observable` code on iOS 17+. There are no app archetypes where the matrix doesn't apply.

For codebases still using `ObservableObject` (pre-iOS 17 or unmigrated): the legacy mapping is `@StateObject` to own, `@ObservedObject` to receive, `@EnvironmentObject` to share. But that's strictly transitional — migrate to `@Observable` when feasible.

### Deeper reading

- `references/state-and-observation.md` § The ownership matrix + § The `@AppStorage` trap.
- Apple WWDC23 Session 10149 (Discover Observation in SwiftUI).
- The `hackingwithswift.com` writeups on `@Observable` and Observation.

---

That's the teaching shape. Apply it to whatever topic `$ARGUMENTS` names.

## Edge cases

- **If the topic isn't in the skill**: say so, then teach from general Swift/SwiftUI knowledge with the same shape. Don't pretend a reference exists when it doesn't.
- **If the topic is contested (MVVM, TCA, Liquid Glass)**: present BOTH sides honestly. Then commit to a default with concrete triggers for the alternative.
- **If the user is a beginner**: keep the shape, but slow down on mechanism. Don't dumb down the stance.
- **If the user is an expert asking nuance**: cite specific WWDC sessions / blog posts / Reddit threads where appropriate.
