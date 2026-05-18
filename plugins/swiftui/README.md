# SwiftUI Expert Plugin

Opinionated SwiftUI/Swift skill plugin for iOS, macOS, and visionOS APP development. Built for Claude Code consumers: strong defaults for golden rules, concrete when/when-not triggers for conditional rules. No hedging — hedged advice produces spaghetti when AI uses it to review real codebases.

## What it does

Critiques and writes SwiftUI/Swift code with the stance of a veteran architect:

- **MV pattern is the default.** ViewModels are an anti-pattern unless specific triggers apply.
- **`App.swift` owns shared `@Observable` singletons via `@State`**, injected with `.environment(_:)`.
- **`NavigationStack` with typed `Hashable` routes.** One stack per tab. One `@Observable Router` per tab.
- **Liquid Glass adoption is your call.** Three concrete paths (selective / custom chrome / opt-out).
- **Approachable Concurrency ON** for app targets. Module-by-module migration. `@unchecked Sendable` warrants a comment naming the synchronization mechanism — prefer `final class { let ... } : Sendable`, actor, or value type.
- **macOS is not iOS with a bigger screen.** Critique Mac apps missing full menus / keyboard shortcuts / drag-and-drop.

Targets the latest Swift / iOS / macOS / Xcode versions and detects from project files.

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install swiftui@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/swiftui
```

## Commands

| Command | Purpose |
|---|---|
| `/swift-critique` | Comprehensive code review across all categories — architecture, state, types, concurrency, design, accessibility, performance, security, platform, persistence, testing. The primary command. |
| `/swift-architect` | Deeper architecture review — MV/MVVM/TCA triggers, App.swift singletons, folder structure, modularization decisions, navigation patterns. |
| `/swift-ios` | iOS-focused review — Privacy Manifest, ATT, App Intents, Widgets / Live Activities, scoped permission APIs, App Attest, StoreKit 2, Foundation Models. |
| `/swift-mac` | macOS-focused review — main menu and keyboard shortcuts, MenuBarExtra, document apps, drag-and-drop via Transferable, sandboxing, Hardened Runtime, notarytool, Sparkle 2.x, SMAppService, AppKit interop. |
| `/swift-teach` | Explain a Swift/SwiftUI concept like a veteran architect — strong opinions, modern patterns, code samples, pointers to references. |

## Skill

The `swiftui-expert` skill activates automatically when writing, reviewing, or debugging SwiftUI/Swift code.

### The five non-negotiables

1. **MV pattern is the default.** View IS the view model.
2. **`App.swift` owns shared singletons via `@State` + `.environment(_:)`.**
3. **`NavigationStack` typed routes. One stack per tab. One Router per tab.**
4. **Liquid Glass on chrome only. App-level theming stays yours.**
5. **Approachable Concurrency ON for app targets.**

### The 20 golden rules

Where Apple + experts + Reddit + popular OSS all agree. Stated as defaults — no hedging.

1. `@Observable` for new code (not `ObservableObject`).
2. `@State` to own, `let` to receive, `@Bindable` for bindings.
3. One NavigationStack per tab; one Router per tab.
4. Extract real `View` structs over `@ViewBuilder` computed properties.
5. `.task { }` for view-tied async; never unstructured `Task { }` in body.
6. Modern API: `.foregroundStyle` / `.tint` / `.clipShape(.rect(cornerRadius:, style: .continuous))` / `#Preview` / `notarytool` / `SMAppService`.
7. Never `AnyView` in lists; never `UUID()` defaulted in `ForEach`.
8. Keychain for tokens/PII (never UserDefaults).
9. `os.Logger` with privacy interpolation (never `print` in production).
10. Privacy Manifest mandatory since May 1, 2024.
11. SwiftData `@Model`s wrapped in `VersionedSchema` from v1.0.0.
12. `@AppStorage` is NEVER allowed inside `@Observable` class (use nested storage class).
13. Liquid Glass on chrome only (never list rows, content tiles, full-screen backgrounds, glass-on-glass, text on glass).
14. macOS apps need full main menu with keyboard shortcuts.
15. Hardened Runtime + Notarization via `notarytool` for direct distribution.
16. Sign in with Apple required alongside any third-party social login.
17. App Intents is the unification API for Siri/Shortcuts/Spotlight/Action Button.
18. Swift Testing for new tests; XCTest for UI/performance/Obj-C bridges.
19. MainActor-default is the recommendation for app targets (file 06 rule 31 contradicts this — out of consensus).
20. Dynamic Type via `relativeTo:` (never `.font(.system(size:))` without it).

### Conditional rules (concrete when/when-not triggers)

When you ask "should I use X?":

- **MV vs MVVM**: MV default. Use ViewModel WHEN explicit state machine + >=20 orchestration tests + UIKit migration. Reality: Apple Backyard Birds = 0 VMs.
- **TCA**: don't use it by default. Use TCA WHEN the app has many screens with cross-screen state coordination, the team is large enough that standardization pays back the learning curve, regulated context demands exhaustive `TestStore` action testing, AND the team has FP/Redux experience. Reality: isowords is the canonical TCA reference; nearly every other audited modern repo skipped it.
- **SwiftData vs Core Data vs SQLiteData vs GRDB**: SwiftData WHEN iOS 17+ AND a small/simple data model AND private-CloudKit-only AND greenfield AND willing to ship VersionedSchema from v1. Core Data WHEN shared/public CloudKit or large relational model. SQLiteData WHEN you want SwiftData ergonomics + shared/public CloudKit. GRDB WHEN SQL is the right abstraction.
- **AppKit vs SwiftUI on macOS**: SwiftUI shell with AppKit drops (NSTextView / NSDocument / NSXPC / low-level window APIs). Pure AppKit valid for utility / menu-bar / media (IINA 44.9k, Stats 38.8k, Rectangle 29k, Ice 28k).
- **Liquid Glass**: Path A (selective, default ~90%). Path B (custom chrome) for fintech / brokerage / brand-heavy. Path C (`UIDesignRequiresCompatibility = true`) for creative/pro apps. Apple's iWork suite, Final Cut, Logic, Pixelmator opted out.
- **Local SPM packages**: flat by default. Modularize WHEN project-file merge conflicts become routine, build times start to hurt iteration, or feature boundaries have stabilized.

## References (18 files)

Each reference is a deep dive on its domain. The skill loads them on-demand when reviewing code.

```
references/
├── anti-patterns.md           # the explicit "don't" list — consulted first
├── architecture.md            # MV pattern, App.swift singletons, folders, TCA/SPM triggers
├── state-and-observation.md   # @Observable, ownership matrix, @AppStorage trap
├── navigation.md              # NavigationStack + Router pattern, typed routes
├── view-composition.md        # extraction, modifiers, body rules
├── lifecycle.md               # init traps, .task vs onAppear, identity, scene phase
├── concurrency.md             # Approachable Concurrency, MainActor, Sendable
├── design-system.md           # tokens, typography, Dynamic Type, @Entry
├── liquid-glass.md            # three paths, opt-outs, named anti-glass voices
├── animation.md               # named springs, @Animatable, Reduce Motion
├── accessibility.md           # CI audits, Dynamic Type, contrast, text-on-glass
├── performance.md             # identity, lazy stacks, Instruments SwiftUI
├── persistence.md             # SwiftData/CoreData/SQLiteData/GRDB triggers, traps
├── ios-platform.md            # App Intents, Widgets, Live Activities, Privacy Manifest, ATT, permissions
├── macos-platform.md          # menus, MenuBarExtra, documents, sandbox, notarytool, Sparkle, FFI/XPC
├── modern-api.md              # deprecation/replacement table (iOS 26 entries)
├── swift-idioms.md            # language patterns (Swift 6.2/6.3)
└── testing-and-debugging.md   # Swift Testing, previews, Instruments, os.Logger
```

## What's modern in 2026 (surfaced as "new")

- **Approachable Concurrency** (Swift 6.2+, default MainActor isolation).
- **`Observations { }`** async sequence (Swift 6.2+, iOS 26).
- **`@Animatable` macro** (iOS 26) replaces manual `animatableData`.
- **`@Entry` macro** for environment values.
- **`ConcentricRectangle` + `.containerShape()`** for iOS 26 sheets.
- **`BGContinuedProcessingTask`** (iOS 26) — long-running background work with system UI.
- **Liquid Glass** + the three adoption paths.
- **`PermissionKit`** (iOS 26) for parental approval in child-account apps; **`DeclaredAgeRange`** (`requestAgeRange` iOS 26+; `isEligibleForAgeFeatures` iOS 26.2+) for coarse age binning.
- **Swift Testing** is Xcode 26 default for new tests.
- **Xcode 26 SwiftUI Instrument** with Cause & Effect Graph.
- **Foundation Models** (iOS 26 on-device 3B LLM) — narrow structured-output only, not GPT-class.
- **SwiftData CloudKit is private-database-only** as of 2026. Use Core Data for shared/public.
- **`@AppStorage` + `@Observable` direct mix is broken** — wrap in nested storage class (IceCubesApp pattern).

## Stance

The plugin states defaults firmly. It cites sources, not personalities — Apple HIG and WWDC sessions, established Swift-teaching blogs by topic, real OSS codebases (IceCubesApp, IcySky, Backyard Birds, NetNewsWire, CotEditor, isowords) with concrete patterns to point at, and anonymous community testimony for contested questions. It demotes Apple-marketing-flavor phrases ("non-negotiable", "must-use", "the future of UI") to neutral framings. When the community is split (MVVM, TCA, Liquid Glass on Mac), it surfaces the split honestly — then commits to a default and lists concrete when-to-override conditions.

## Target

- Latest Swift / iOS / macOS / Xcode (detects from project).
- iOS apps, macOS apps, visionOS apps.
- NOT server-side Swift (Vapor, Hummingbird).

## License

MIT
