---
name: swiftui-expert
description: This skill should be used when the user is building, reviewing, or debugging SwiftUI/Swift code for iOS, macOS, or visionOS apps. It detects the project's iOS and Swift version and covers MV architecture (no per-screen ViewModels by default), @Observable state, NavigationStack typed routes, view lifecycle, Approachable Concurrency, persistence (SwiftData / Core Data / SQLiteData / GRDB), design tokens, selective Liquid Glass adoption, iOS and macOS platform capabilities, accessibility, performance with Instruments SwiftUI, and Swift Testing. Trigger phrases include "review my SwiftUI code", "my @State isn't updating", "should I adopt Liquid Glass", "fix my Swift concurrency warning", "set up SwiftData", "what should App.swift own", "ObservableObject to @Observable", "should I use MVVM in SwiftUI", "do I need TCA", and "AppKit or SwiftUI for my Mac app". For driving/screenshotting a RUNNING app use the peekaboo validator; for dead-code hunts use the dead-code plugin.
---

# SwiftUI Expert

This skill reviews and writes SwiftUI / Swift code for iOS, macOS, and visionOS apps.

**Take a position when the evidence lines up.** Apple's docs, the modern SwiftUI teaching community, and the popular open-source codebases (IceCubesApp, IcySky, Backyard Birds, NetNewsWire, CotEditor) agree on the modern defaults more often than people pretend. When they do, state the rule and move on — hedged advice applied across many files produces contradictory critiques.

## Default targets

This skill assumes Swift 6.3, iOS 26 / macOS 26 Tahoe, and Xcode 26 unless the project says otherwise. Check `Package.swift`, the `.xcodeproj` settings, any `.xcconfig` files, and `Info.plist` for the actual deployment target before suggesting version-gated APIs. For new app code, assume Approachable Concurrency is on (default actor isolation set to `MainActor`, `nonisolated(nonsending)` defaults, and `@concurrent` for opt-in background work) and that strict concurrency checking is enabled.

## How to use this skill

The skill ships eighteen reference files. Do not load them all. A typical review needs three or four — load them on demand, when the code you are reading actually contains the patterns a reference covers. Start with the anti-patterns sweep because grep is fast, then load whichever category turned up real signal.

The rules below are defaults for new code. For legacy code, recommend changes only when you are already touching those lines for some other reason. Working `@StateObject` infrastructure in a codebase that targets iOS 16 is not something to rewrite during a review of a network bug.

Where a project consistently uses one pattern across all its files (say, `ObservableObject` everywhere on an iOS 16 target), note the migration suggestion once at the project level. Do not flag every instance individually — that produces noise instead of insight.

## Don't invent — look it up

Most of what an AI reviewer gets wrong comes from sounding confident about a thing it has not verified: an invented Feedback ticket number, a deprecation date that does not exist, an API name that is really a UIKit concept wearing a Swift name. The rule is not "be careful" — it is **use a lookup before asserting**:

- **`sosumi.ai` serves Apple developer docs as plain Markdown** (Apple's own pages are JS-rendered and unreadable to most tooling). Fetch the API's page before citing its availability or signature.
- **Check the project's actual deployment target** (`Package.swift`, build settings, `Info.plist`) before recommending any version-gated API.
- When you cannot verify a load-bearing claim, keep the finding but soften the precision and say how the developer can verify it ("check the iOS 26 SDK headers for this initializer"). "Available since iOS 25.3" with no source is worse than "verify availability" even when it happens to be right.
- Do not invent numerical thresholds (line counts, screen counts) to give qualitative advice a fake spine. Describe the pain that triggers the decision — build times, project-file merge conflicts, hires lost in the folder structure — and let the developer match it.

Some load-bearing claims in this skill were verified against Apple docs and IceCubesApp `main` (the Liquid Glass surface, Privacy Manifest dates, the `notarytool` cutover, the `@AppStorage` + `@Observable` workaround); community-attributed claims were not all individually fact-checked — re-verify before quoting them in a production critique.

## The five non-negotiables

These are the rules where the entire corpus agrees and where a reviewer should state the position clearly when they see violations.

### 1. The MV pattern is the default

In modern SwiftUI the `View` struct *is* the view model. It composes from sources of truth — `@State` for view-local values, `@Environment` for shared `@Observable` instances, `@Bindable` for binding extraction, `@Query` for SwiftData — and the AttributeGraph re-renders the view precisely when any property the view reads mutates. The keypath-precise tracking that arrived with `@Observable` in iOS 17 means a per-screen ViewModel wrapper adds an indirection without adding precision.

Use a ViewModel only when the screen has genuine orchestration complexity: an explicit state machine with loading / loaded / error / empty states plus retry, pagination, and optimistic updates; a real plan to test orchestration logic in isolation from the view; or a UIKit-to-SwiftUI migration in flight where an existing ViewModel is the bridge. Apple's Backyard Birds, Food Truck, and Landmarks samples ship zero ViewModels. IceCubesApp, whose own `CLAUDE.md` bans them, still ships them on its most complex screens (the timeline, the conversation view, the profile editor) — because those screens hit the triggers above. Match the rule to the trigger.

See `references/architecture.md`.

### 2. `App.swift` owns shared `@Observable` singletons

App-level shared state — theme, router, auth store, appearance store — lives as `@State` in your `@main App` struct and propagates down through `.environment(_:)`. Consumers read it with `@Environment(Type.self)`.

```swift
@main
struct MyApp: App {
    @State private var theme = Theme.shared
    @State private var router = AppRouter()
    @State private var auth = AuthStore()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(theme)
                .environment(router)
                .environment(auth)
        }
    }
}
```

Use `@State`, not `@StateObject`. `@StateObject` is the wrapper for `ObservableObject`, which is legacy now; `@Observable` instances belong in `@State`. And do not register these primary singletons through a custom `EnvironmentKey` — custom keys are useful for defaultable values like a font or a feature flag, not for shared mutable state.

This is the pattern IceCubesApp, IcySky, and Apple's Backyard Birds all use. See `references/architecture.md` and `references/state-and-observation.md`.

### 3. `NavigationStack` with typed routes, one stack per tab

Use `NavigationStack(path:)` with a typed `Hashable` route enum. Give each tab an independent stack/path when tabs preserve separate navigation histories. A bound path makes deep linking and restoration representable, not automatic: parse and validate incoming URLs, select the destination tab, persist a `Codable` path, handle decode failures/schema changes, and restore it at the correct lifecycle point.

```swift
enum Route: Hashable {
    case profile(UserID)
    case settings
}

@Observable @MainActor
final class Router {
    var path = NavigationPath()
}

NavigationStack(path: $router.path) {
    HomeView()
        .navigationDestination(for: Route.self) { route in
            switch route {
            case .profile(let id): ProfileView(id: id)
            case .settings:        SettingsView()
            }
        }
}
```

`NavigationView` has been deprecated since iOS 16. `NavigationLink(destination:)` is still legal but does eager pushes; in new code use `NavigationLink(value:)` with `navigationDestination(for:)`. String-based routes give up the type system for no gain — keep routes typed. See `references/navigation.md`.

### 4. Liquid Glass adoption is your call

The April 28, 2026 deadline often cited as "the Liquid Glass mandate" is actually the SDK 26 build requirement for new App Store submissions. When you build against SDK 26, your app's system chrome — `TabView`, `NavigationStack` toolbar, sheets, menus — automatically adopts the new material. Your content surfaces stay flat and yours.

Three adoption paths are all valid in 2026:

The default for most apps is **selective glass on your own chrome**: let the system surfaces auto-adopt, and use `.glassEffect()` on your custom accessory chrome where it fits. This is what IceCubesApp does (around ten call sites, conditional on the iOS 26 availability check).

For brand-heavy apps where the chrome is part of the product — fintech, brokerage, custom design language — you can build **entirely custom chrome** that bypasses Apple's glass surfaces. Robinhood does this; nothing about your app needs to render glass.

For creative or pro apps that are not ready this cycle, **set `UIDesignRequiresCompatibility = true` in `Info.plist`**. Apple's own iWork suite, Final Cut Pro, Logic Pro, Pixelmator Pro, iMovie, QuickTime Player, and Chess all shipped with this flag at iOS 26.0 launch (and most updated in early 2026). Apple has said the flag will be ignored in a future Xcode release, but no specific version has been published — so treat it as a one-cycle escape, not a permanent stance.

You can opt out without apology. The "you must adopt or look outdated" framing that surfaced in some early coverage was overstated. See `references/liquid-glass.md` for the full decision matrix.

### 5. Approachable Concurrency for app targets

For new app targets in 2026, turn on Approachable Concurrency: default actor isolation set to `MainActor`, `nonisolated(nonsending)` defaults, and `@concurrent` for opt-in background work. Most app code already lives on the main actor, and the new defaults let you stop writing `@MainActor` annotations everywhere.

For non-UI Swift Package Manager packages — a networking layer, a parser, anything that should be callable off the main thread — opt out per-target in `Package.swift` so those packages stay nonisolated and can be called from background work without a main-actor hop.

Migrate existing codebases to Swift 6 strict mode module by module — never the whole repo in one PR. And when you reach for `@unchecked Sendable`, treat it as a synchronization claim you owe a comment for: name the synchronization mechanism in the code, and prefer `final class { let ... } : Sendable`, an `actor`, or a value type when one of those would work.

See `references/concurrency.md`.

## Crosscutting principles

A small number of broad principles apply across every category and should color every review.

**Latest stable iOS / Swift / Xcode unless the project specifies otherwise.** Verify the deployment target from `Package.swift`, the Xcode build settings, and `Info.plist` before suggesting any version-gated API. A recommendation to use `Observations { }` in a codebase that targets iOS 24 is a recommendation that will not compile.

**Prefer SwiftUI-native solutions, but hybrid SwiftUI / UIKit is mainstream in 2026.** Do not apologize for `NSViewRepresentable` or `UIViewRepresentable` when SwiftUI has a genuine gap — `NSTextView` for rich text editing, advanced `UICollectionView` layouts that `LazyVGrid` cannot express, PDFKit, mature components without a SwiftUI peer. Reach for SwiftUI first; bridge to UIKit / AppKit when SwiftUI fights back.

**Apple's guidance and Apple's marketing are not the same.** When the HIG says one thing and the community ships another, surface the disagreement honestly. The clearest example in 2026 is Liquid Glass: Apple's developer-relations framing pushes adoption, but Apple's own pro apps shipped with `UIDesignRequiresCompatibility = true` and the named Mac critic blogs (Daring Fireball, inessential.com, lapcatsoftware) are loud and right. State the trade-off; do not pretend the question is settled when the community's lived experience says otherwise.

**macOS is not iOS with a bigger screen.** Mac users live in the menu bar and the keyboard. A Mac app shipping with an empty `.commands { }` and no keyboard shortcuts on its primary actions reads as an iPad port. Drag-and-drop via `Transferable`, document handling via `DocumentGroup`, and AppKit interop where SwiftUI has gaps — all of these are normal in shipping Mac apps. Critique any Mac SwiftUI app missing these on sight.

**One primary public type per file.** Don't enforce strict "one type per file" — every popular OSS codebase violates it pragmatically when private helpers live alongside the type they serve. The rule is "one main public type; co-locate the helpers that exist only for it."

**Feature-first folder structure.** Top-level grouping by feature, not by layer (no `ViewModels/`, `Views/`, `Models/` at the root). Inside a feature, layer-style sub-folders are fine when the feature is large enough to need them. Promote to local SPM packages when iteration time hurts — project-file merge conflicts becoming routine, build times growing past tolerable, feature boundaries stable enough to extract. Do not modularize for its own sake.

## iOS platform — what every app must do

These are the iOS-specific rules that should appear in nearly every iOS app review.

**Privacy Manifest.** Since May 1, 2024, an app or bundled SDK that uses Apple's listed Required Reason APIs must declare approved reasons in `PrivacyInfo.xcprivacy`; Apple also requires manifests and signatures from SDKs on its required-SDK list. Do not flag every app that lacks the file. First inventory the APIs and dependencies, then treat absence or incomplete reasons as blocking only when the requirement applies. The manifest can also declare collected-data categories and tracking domains; App Store Connect declarations must match actual behavior.

**Sign in with Apple.** App Review Guideline 4.8 generally requires an equivalent privacy-preserving login option when a third-party/social login authenticates the app's primary account. Check the published exceptions before flagging: apps exclusively using their own account system, enterprise/education/business apps requiring an existing organization account, government/industry-backed identity systems, and clients for a specific third-party service can qualify. Report the applicable rule and exception analysis, not a blanket button-presence check.

**App Intents are the unification API.** A single `AppIntent` exposes the action to Siri, Shortcuts, Spotlight, Focus filters, Action Button, Apple Pencil Pro squeeze, and Visual Intelligence (iOS 26). WWDC25 Session 244 covers this directly. For most consumer apps, App Intents are the realistic on-ramp into Apple Intelligence features — implement the verbs you already support and they get surfaced everywhere automatically.

**Prefer scoped permission APIs over Info.plist prompts.** Several iOS APIs let users grant access to a single item without an Info.plist usage string and without a prompt: `PhotosPicker` for photos, `LocationButton` for one-tap location, `ContactAccessButton` for a single contact (iOS 18+), `EKEventEditViewController` for write-only Calendar (iOS 17+), `DataScannerViewController` for camera scanning. Reach for these first. Full-permission APIs and Info.plist usage strings are for cases where the scoped API genuinely does not fit.

**App Tracking Transparency, only if you actually track.** If your app does not track users across other apps and websites, do not call `ATTrackingManager.requestTrackingAuthorization()`. And do not implement a fingerprinting fallback after a denied prompt — Apple's human reviewers reject this under App Store Review Guidelines 5.1.1 and 5.1.2. (`ITMS-91008` is sometimes cited here but it is actually the "Invalid API reason declaration" code, a sibling of `ITMS-91053`; fingerprinting rejections come through human review and do not have a single fixed code.)

**Interactive widgets.** Since iOS 17, widgets can host `Button` and `Toggle` controls bound to an `AppIntent`. Static read-only widgets are legacy. If your app shows up on a user's Home Screen or Lock Screen daily, it should have a widget that lets them do something from there.

**Background work with `BGContinuedProcessingTask`.** New in iOS 26. Long-running uploads, exports, and rendering tasks now have a system-presented progress UI and can keep running after the user backgrounds the app. Productivity apps that today silently fail when the user switches away should adopt this — it is one of the most useful and most overlooked iOS 26 additions.

**App Attest and DeviceCheck.** Around fifty lines of code, free, Secure-Enclave-backed proof that a request to your server came from the unmodified binary you actually shipped. Most apps with a backend skip this and end up reinventing fraud detection later. If your app has trial abuse, sign-up spam, or any reverse-engineered-client risk, adopt App Attest before building any of those workarounds.

**Use the modern variants.** `PhotosPicker` rather than `UIImagePickerController` for any new code. SwiftUI `Map { Marker; Annotation }` rather than `UIViewRepresentable(MKMapView)`. `TipKit` rather than rolling your own tooltip system. `StoreKit 2` (async/await with JWS receipt validation) rather than StoreKit 1.

**Foundation Models for narrow tasks only.** iOS 26 ships an on-device three-billion-parameter language model that you call via the `FoundationModels` framework. It is good for structured output (via the `@Generable` macro), summarization, classification, and other narrow tasks. It is *not* a GPT-class chatbot, its context window is small, and one developer's testing showed it underperforms similarly-sized open-source models. Use it where it fits; do not pitch it as the AI feature.

**Permissions, in general.** Never ask on first launch. Prime the user before the prompt by showing what the permission unlocks. Ask one permission per moment, never stack three prompts at the start of onboarding. When the user denies, give them a clear path back via `UIApplication.openSettingsURLString` (or `openNotificationSettingsURLString` for notifications).

See `references/ios-platform.md` for the full per-API treatment.

## macOS platform — make it feel native

macOS is not iOS with a bigger screen. Mac users live in the menu bar and the keyboard, and apps that ignore those conventions read as iPad ports — flag this on sight in any Mac code review.

**A real main menu.** Use `.commands { }` on the `Scene` with `CommandGroup` to populate File, Edit, View, Window, and Help with the actions your app actually supports. Scale to the app: a 2-action clipboard utility does not need a full Edit menu, but a document app without Cut/Copy/Paste shortcuts is broken. Every primary command gets a `.keyboardShortcut`. Use `.focusedSceneValue` and `@FocusedValue` so commands target the front window's selection rather than a global state.

**Drag-and-drop via `Transferable`.** With `Transferable` plus `.draggable` and `.dropDestination`, your app can accept drops from Finder, send items to Mail, and receive data from any other Mac app. This is the Mac power-user superpower most ports neglect.

**Login items, agents, and daemons via `SMAppService`.** macOS 13+ replaces `SMLoginItemSetEnabled` and `SMJobBless`. New code should not use the deprecated APIs.

**Notarization with `notarytool`.** Apple stopped accepting `altool` uploads on November 1, 2023. Use `xcrun notarytool submit --keychain-profile ... --wait` followed by `xcrun stapler staple` and `spctl --assess --type execute --verbose=4` for verification.

**Hardened Runtime, the least permissive that works.** Hardened Runtime is mandatory for notarization. The entitlement hierarchy is `com.apple.security.cs.allow-jit` (for JIT compilers like JavaScriptCore-based scripting), then `allow-unsigned-executable-memory` (broader), then `disable-executable-page-protection` (broadest, weakest). Pick the least permissive that lets your app launch.

**Auto-update with Sparkle 2.x.** EdDSA signatures are the modern signature scheme, and Sparkle 2.x supports sandboxed apps. New direct-distribution apps should use Sparkle 2 from day one — Sparkle 1 is legacy.

**MenuBarExtra and its missing state API.** `MenuBarExtra` (macOS 13+) gives you a SwiftUI menu bar app in a few lines, but it has no first-party API for controlling the popover programmatically. Real-world menu bar apps either use the third-party `MenuBarExtraAccess` / `FluidMenuBarExtra` packages, or drop to raw `NSStatusItem` (this is what Ice and MeetingBar do).

**Document-based apps.** `DocumentGroup` with `FileDocument` (for value types) or `ReferenceFileDocument` (for class semantics) gets you Open / Save / Recent / Versions and iCloud Drive sync without custom code. Skip the boilerplate.

**AppKit interop is fine — bridge without guilt.** `NSViewRepresentable`, `NSHostingView`, and `NSHostingController` are everyday tools in shipping Mac SwiftUI apps. Drop to AppKit when you need `NSTextView` (rich text, code editing), `NSDocument` semantics SwiftUI does not match, `NSXPC` privilege separation, or low-level window and accessibility APIs.

**macOS Privacy Manifest is formally exempt, but ship one anyway.** As of 2026 the manifest requirement is iOS-only. macOS submissions are not rejected for missing it. For shared SwiftPM packages that get embedded into iOS targets, you still want to ship one — and shipping it on macOS too costs nothing and prepares for any future requirement.

**TCC, signing identity, and re-prompts.** TCC permissions are bound to the bundle ID and the code signature. If you change signing identity between releases, users lose their grants. Sequoia and Tahoe re-prompt for Screen Recording monthly even on apps that previously had permission. Local Network access on macOS is a NetworkExtension packet filter, not TCC — different prompt, different recovery.

**Info.plist usage strings actually crash.** On macOS, accessing a TCC-protected resource without the corresponding Info.plist usage string crashes the app at first access. There is no graceful fallback to a denied state.

**AppKit is alive in 2026.** Pure AppKit utility, menu-bar, and media apps still ship and still thrive: IINA (44.9k stars), Rectangle (29k), Stats (38.8k), Ice (28k), MeetingBar (5.2k). Do not push SwiftUI where AppKit is the right call. For an all-windows-and-status-bar utility, AppKit is often cleaner.

See `references/macos-platform.md`.

## Other golden findings worth surfacing

A few smaller patterns are worth mentioning when relevant.

**The IceCubesApp `@AppStorage` workaround is the canonical pattern for combining `@Observable` with persistent storage.** A plain (non-`@Observable`) inner `Storage` class holds the `@AppStorage`-marked properties; the outer `@Observable` class has stored `var` properties with `didSet` that mirror writes to storage; a private `init()` seeds the outer values from storage at startup. The macro instruments the outer stored properties normally, so observation tracking works. See `references/state-and-observation.md` for the full code.

**`Self._printChanges()` is the fastest "why is this view re-rendering" debug.** Place it inside `body` behind `#if DEBUG`. It logs the dependency that caused the body to re-evaluate. Strip before ship.

**Instruments 26 SwiftUI template with the Cause & Effect Graph (WWDC25 Session 306) is the modern way to profile view-update performance.** Stop guessing at re-render reasons — the graph shows which state mutation caused which AttributeGraph invalidation.

**Liquid Glass also exists in UIKit.** Apple shipped `UIGlassEffect` and `UIGlassContainerEffect` for UIKit alongside the SwiftUI surface. Amperfy (a music app, around 1.5k stars) uses the UIKit form in production. UIKit-majority apps do not need to bridge to SwiftUI just for glass.

**AppKit is alive at scale.** When critique surfaces "this Mac app uses too much AppKit," check whether the AppKit usage is solving a real SwiftUI gap before recommending a rewrite.

**CocoaPods is not dead.** AltStore and other large active repos still use it. Do not flag a `Podfile` as legacy unless the project is actively migrating to SPM.

**Apple's Backyard Birds is the minimal-architecture reference.** Zero ViewModels, pure `@Query` + `@State` + `@Environment`. Cite it when a developer asks "what does Apple recommend" for SwiftData-heavy apps.

**The Xcode 26 tab fix.** Settings → Navigation → Pin Editor Tabs: When Tab Is Created restores the old tab behavior that many users prefer. Worth knowing when a developer complains about the new tab system.

## Conditional decisions — defaults and when to switch

When a developer asks "should I use X?" the honest answer is usually "default to Y, switch to X when the situation has these specific shapes." Below are the most common conditional decisions in modern SwiftUI work.

### MV pattern vs. MVVM

Default to MV. The `View` is the view model. Use a per-screen `@Observable` ViewModel only when the screen has an explicit state machine (loading, loaded, error, empty, with retry and pagination), when you have a real plan to test orchestration logic in isolation from the view, or when you are bridging from an existing UIKit ViewModel during migration. Apple's samples ship zero ViewModels; IceCubesApp ships forty-four on its most complex screens. Match the rule to the trigger.

### TCA vs. vanilla SwiftUI

Default to vanilla SwiftUI plus `@Observable` plus an `@Observable` router. Consider The Composable Architecture when several concrete needs align: difficult cross-feature state/effect coordination, a valuable deterministic test seam for actions and dependencies, state whose lifecycle is independent of a view tree, and a team willing to own the reducer model, learning cost, tooling cost, and third-party dependency. A regulated context may increase the value of exhaustive reducer tests, but regulation is neither required nor sufficient. Adoption among popular maintained Swift OSS apps is rare outside Point-Free's ecosystem; the canonical TCA reference is `isowords`. Judge whether TCA solves observed coordination and assurance problems rather than requiring every demographic or industry signal.

### Persistence — SwiftData vs. Core Data vs. SQLiteData vs. GRDB

There is no one right answer, but each option fits a specific shape.

**SwiftData** is the default for a new iOS 17+ app with a small to mid-sized data model, no need for shared or public CloudKit sync, and a greenfield codebase. Wrap every `@Model` in a `VersionedSchema` from v1.0.0 — without it, the first migration attempt in production fails with no path forward except a bridge release.

**Core Data** remains the right call for shared or public CloudKit sync (SwiftData supports only the private database in 2026), for large relational models, for codebases with existing Core Data investment, and for apps that need `NSCompoundPredicate` or `NSFetchedResultsController`.

**SQLiteData** (Point-Free) gives you SwiftData-style ergonomics on top of GRDB, with support for shared and public CloudKit. It is newer; production maturity is still settling. Useful when you want SwiftData's API surface but need the CloudKit scopes SwiftData does not yet support.

**GRDB** is the right call when SQL is genuinely the right abstraction for the data, when you need fine-grained query control, or when datasets are large enough that SwiftData's main-thread cost starts to show.

See `references/persistence.md` for production rules (diagnosing `0xdead10cc`, protecting bounded work that can cross suspension, CloudKit constraints, and the `@AppStorage` + `@Observable` workaround).

### AppKit vs. SwiftUI for macOS

Default to a SwiftUI shell with strategic AppKit drops. Use `NSViewRepresentable` and friends for `NSTextView`, `NSDocument`, `NSXPC`, and low-level window or accessibility APIs.

Stay pure AppKit when you are building a utility, menu-bar, or media app where SwiftUI gaps would force you to fight the framework on the most important surfaces. IINA, Rectangle, Stats, Ice, and MeetingBar combined ship tens of thousands of stars of "no SwiftUI here" — and they are all excellent Mac apps. The "rewrite to SwiftUI" instinct is sometimes wrong.

### Liquid Glass — three paths

Path A (selective glass on your own chrome) is the default for roughly nine out of ten apps. Path B (entirely custom chrome) fits brand-heavy apps like fintech, brokerage, and banking — the Robinhood model. Path C (`UIDesignRequiresCompatibility = true`) fits creative and pro apps that are not ready this cycle; Apple's own iWork, Final Cut, Logic, and Pixelmator all shipped this way at launch. Pick the path that matches your app's relationship to its chrome, not what blog posts tell you Apple wants.

### Local SPM packages vs. flat target

Default to a flat target. Modularize into local SPM packages when project-file merge conflicts become routine, when build times start to hurt iteration, when feature boundaries have stabilized and you can name them confidently, or when widget / watch / share-extension targets need to consume the same code. Do not modularize a solo project or a POC where the boundaries are still in flux — the cross-package edit cost burns more time than the modularization saves.

The bottom-up extraction order is `DesignSystem` first (no app dependencies), then `Networking`, then per-feature packages once feature boundaries are stable.

## Hard-won Swift opinions

Positions to state plainly when code violates them, each with its consequence.

- **`!`, `try!`, and implicitly-unwrapped optionals are for tests and provably-impossible states with a comment.** In app code, a force-unwrap is a crash report with a date TBD; `guard let` with a real fallback is the default.
- **Blanket `[weak self]` is cargo cult.** It belongs in *stored*, long-lived closures where a retain cycle is real. A `.task` body or a short-lived completion handler doesn't need it — and the `guard let self else { return }` dance it forces can silently skip work.
- **"The compiler is unable to type-check this expression in reasonable time" is a design signal, not a compiler bug.** Break the body into smaller `View` structs and typed sub-expressions; do not fight it with more nesting.
- **Structs by default; a class is a claim about identity.** Shared `@Observable` state objects are the legitimate class case — that is not a license for class-first design elsewhere.
- **Never do date math with `86_400`.** DST days are 23 or 25 hours long; use `Calendar.date(byAdding:)`. This bug ships constantly and only fires twice a year.
- **Prefer `some` over `any`.** Existentials cost dynamic dispatch and break type inference; reach for `any` only for genuinely heterogeneous storage.
- **Enums with associated values beat boolean blizzards.** Three `Bool` parameters describing one state is an illegal-states-representable bug waiting to be written.
- **Codable needs a fixture test.** Optional fields decode to `nil` on a key mismatch without any error — a decode test against real server JSON is the only thing that catches it.
- **`GeometryReader` is a measurement tool, not a layout system.** Wrapping content in it destroys intrinsic sizing (it greedily takes all proposed space) and is the classic cause of "why is my view suddenly full-screen." Prefer `containerRelativeFrame`, alignment guides, or `onGeometryChange` for reading sizes; reach for `GeometryReader` only when you truly need proposal-driven layout.
- **A `body` should read as layout, not logic.** Inline multi-line closures doing networking or state machines inside a `Button` action are the SwiftUI god-function. Extract intent methods (`func addTapped()`) so the body stays declarative and the logic becomes testable.
- **Magic numbers are design-token debt.** `.padding(13)` and `Color(red: 0.94, ...)` scattered through views drift apart within weeks. Numbers and colors used more than once get a token; screens built from tokens can be re-themed in one file.
- **Every view ships a `#Preview` with realistic data.** Previews are executable documentation and the fastest feedback loop in the toolchain; a view without one is a view nobody can safely edit. Empty-state and long-text variants catch most layout bugs before any simulator launch.
- **`ZStack` + manual offsets is usually alignment guides not yet learned.** Hand-tuned `.offset(x: 3, y: -7)` breaks with Dynamic Type and localization; custom alignment guides express the actual relationship and survive both.

## Zoom out before you edit

Sessions that skip this produce split-brain code (a second formatter, a second theme constant, a second date helper) and orphaned views. Non-negotiable sequence for any change:

1. **Before adding a view, modifier, or helper: search for an existing one** — `rg -i` the concept across the target and the design-system module. A second implementation drifts from the first; that's a visual inconsistency bug on a timer.
2. **Read the whole file and the parent view before editing**, not just the flagged lines — state ownership problems upstream are usually the cause of the symptom downstream.
3. **After the change, grep the symbols you replaced** and delete what is now unreferenced in the same change.
4. **Say in one sentence where the change sits** (which feature, which layer, who owns the state it reads). If you can't, read more before editing.
5. **Verification is evidence, not assertion.** A build/test run with its output shown is verification; "verified" without output is not. If you could not run the app, write "compiles, not exercised" — never imply a runtime check you didn't perform.

## Review process — the rubric

A code review under this skill works through severity-tiered findings against a known order of categories. The structure below mirrors the one the rust-expert skill uses; it works for the same reasons (signal over volume, named consequences, judgment over checklists).

### Severity tiers

When you record a finding, name its severity. The five tiers below cover everything; do not invent new ones.

**blocking** — the code is wrong in a way that will cause data loss, security failure, crash on plausible input, App Store rejection, or breakage of a core user flow. Fix before merge. Examples: `@AppStorage` placed directly inside an `@Observable` class (the silent-no-updates trap), tokens stored in `UserDefaults`, required-reason API use without the applicable privacy-manifest declaration, SwiftData `@Model` shipped without `VersionedSchema` from v1, an icon-only `Button` with no accessibility label on a primary action.

**important** — the code works today but creates a real cost: bad performance on a measured path, deprecated API that will break on the next OS, missing tests on non-trivial orchestration, an architectural drift that will hurt a later edit. Worth addressing during this work, not later. Examples: an unstructured `Task { }` inside `body`, an `if-else` flip that creates two structural identities, raw `.spring(response:dampingFraction:)` where a named spring is cleaner, MVVM wrapped around `@Query`, a Mac app shipping with an empty `.commands { }`.

**nit** — a small style or naming issue that the team's conventions cover. Mention briefly, do not lead the review with it. Examples: `.foregroundColor` for `.foregroundStyle`, `.cornerRadius(_:)` for `.clipShape(.rect(...))`, a one-parameter `onChange(of:perform:)` that still compiles.

**suggestion** — an alternative the developer might consider, with a real tradeoff. No action required. Examples: extracting a `DesignSystem` SPM package once boundaries stabilize, switching from XCTest to Swift Testing for a new test file.

**praise** — call out a pattern done well. This is not sycophancy: reinforcing the patterns the team already gets right helps them propagate.

### Order of inspection

For each review, walk these categories in order. Load the matching reference only when the code shows real signal in that category — most reviews touch five or six, not all eighteen.

1. **Anti-patterns sweep** (grep-fast first pass). See `references/anti-patterns.md`.
2. **Architecture and ownership.** Does `App.swift` own shared singletons via `@State`? Does each tab have one stack and one router? Do `*ViewModel.swift` files exist for reasons that pass the MV-vs-VM triggers? See `references/architecture.md`.
3. **State and observation.** Are the right wrappers in the right places? Any `@AppStorage` directly inside an `@Observable` class? See `references/state-and-observation.md`.
4. **Navigation.** Typed routes? One stack per tab? `sheet(item:)` rather than manual booleans? See `references/navigation.md`.
5. **View composition.** Body extracted into real `View` structs? Modifier ordering sensible? `AnyView` absent from lists? See `references/view-composition.md`.
6. **View lifecycle.** `.task` rather than `.onAppear { Task { } }`? Identity stable across `if/else` branches? Work moved out of `init`? See `references/lifecycle.md`.
7. **Concurrency.** Approachable Concurrency posture matches the project? `@unchecked Sendable` carries a comment? `.task` for view-tied async work? See `references/concurrency.md`.
8. **Design system.** Tokens in a shared module? Dynamic Type ramp respected? Modern API for color and shape? See `references/design-system.md`.
9. **Accessibility.** VoiceOver labels on every interactive element? Reduce Motion respected? Text off glass surfaces? Audit running in CI? See `references/accessibility.md`.
10. **Performance.** Identity stable in `ForEach`? Lazy stacks where needed? Expensive work out of `body`? See `references/performance.md`.
11. **Animation.** Named springs? `@Animatable` rather than hand-rolled `animatableData`? See `references/animation.md`.
12. **Liquid Glass** (iOS/macOS 26+ only). Glass on chrome rather than content? No nested glass? Path A / B / C posture matches the app's brand needs? See `references/liquid-glass.md`.
13. **Persistence** (only if SwiftData, Core Data, GRDB show up). `VersionedSchema` from v1? Any measured suspension-related database termination or bounded critical save that needs background-task protection? See `references/persistence.md`.
14. **iOS platform** (iOS targets). Privacy Manifest, Sign in with Apple, App Intents, scoped permission APIs. See `references/ios-platform.md`.
15. **macOS platform** (Mac targets). Main menu with shortcuts on primaries, `SMAppService`, `notarytool`, Hardened Runtime, Sparkle 2.x. See `references/macos-platform.md`.
16. **Testing and debugging.** Swift Testing for new tests? `os.Logger` rather than `print`? UI tests using accessibility identifiers? See `references/testing-and-debugging.md`.
17. **Modern API replacements** (any deprecated API found in earlier sweeps). See `references/modern-api.md`.
18. **Swift language idioms** (optionals, errors, generics, naming). See `references/swift-idioms.md`.

Skip categories with no signal. Do not pad with nits.

### For each finding

Record each finding the same way:

1. **File and line.** Exactly where.
2. **Severity.** From the tiers above.
3. **What.** Name the problem in one sentence.
4. **Why it matters.** Concrete production cost in plain language — what bug appears, what crashes, what gets corrupted, what gets rejected, what becomes harder to change. If you cannot name a concrete consequence, the finding belongs as `suggestion` or should not be raised.
5. **Fix.** Before / after code block when non-obvious.

### The "what I'm not flagging" pass

Before you write the summary, do a deliberate pass on things you saw that look like they should be flagged but where the right call is to leave them alone. Document this. Examples:

A `*ViewModel.swift` file that exists on a complex screen with a real state machine and tests around it does not need to be removed in the name of MV-pattern purity. Leave it; note that you considered it and the triggers are met.

`@unchecked Sendable` on a type that has a clear comment explaining the synchronization mechanism — say, an actor-replacement helper with `final` plus only `let` properties plus a `NSLock` for one shared cache — is not a smell, it is a documented decision. Note it as praise.

A Mac app that uses pure AppKit instead of SwiftUI for its core surface — if that surface is a text editor (`NSTextView`), a video player (`mpv`-driven), or a system monitor with menu-bar status items — is making a correct call. Do not flag the lack of SwiftUI; if anything, note the choice as appropriate.

`UIDesignRequiresCompatibility = true` in a creative or pro app's `Info.plist` is a deliberate one-cycle choice, not laziness. Apple's own pro apps shipped this way. Leave it; note the choice and what would change it.

A working `ObservableObject` in a codebase that targets iOS 16 is not something to rewrite during a network-bug review. Note the migration suggestion once at project level and move on.

The "what I'm not flagging" section is often more valuable than the findings themselves. It tells the next reviewer (or the same one six months from now) what was already considered and why.

### Summary

End the review with a prioritized summary: blocking issues first, then important, then a brief mention of nits and suggestions. Skip files with no issues. Keep the summary tight — the developer reading it should see the shape of the work in under a minute.

## Quick API replacement table

These are the most common modern-API replacements. The full table lives in `references/modern-api.md`.

| Stale | Modern |
|---|---|
| `ObservableObject` | `@Observable` |
| `NavigationView` | `NavigationStack` / `NavigationSplitView` |
| `.foregroundColor` | `.foregroundStyle` |
| `.accentColor` | `.tint` |
| `.cornerRadius` | `.clipShape(.rect(cornerRadius:, style: .continuous))` |
| `PreviewProvider` | `#Preview` |
| `.font(.system(size:))` | `.font(.body)` (Dynamic Type ramp) |
| `Task.sleep(nanoseconds:)` | `Task.sleep(for:)` |
| `onChange(of:perform:)` (one-parameter) | `onChange(of:initial:_:)` |
| `Binding(get:set:)` in body | `@State` + `.onChange(of:initial:_:)` |
| `AnyView` | `@ViewBuilder`, `Group`, or generics |
| `UIImagePickerController` | `PhotosPicker` / `PHPickerViewController` |
| `altool` | `notarytool` |
| `SMLoginItemSetEnabled` | `SMAppService` |

## The explicit "don't" list

These are the patterns to flag on sight. The full annotated list is in `references/anti-patterns.md`.

For state, do not use `ObservableObject` / `@Published` / `@StateObject` / `@ObservedObject` for new code, do not place `@AppStorage` directly inside an `@Observable` class, and do not write `Binding(get:set:)` inside a body.

For navigation, do not use `NavigationView` or `NavigationLink(destination:)`. Drive sheets with `sheet(item:)` and an `Identifiable` enum rather than `sheet(isPresented:)` with manual booleans.

For modifiers and style, do not use `.foregroundColor`, `.accentColor`, `.cornerRadius(_:)`, or `.font(.system(size:))` without `relativeTo:`. On iOS 26 sheets, do not apply `.presentationBackground(.thinMaterial)` — it suppresses the new presentation style.

For views, do not use `AnyView` in a list, do not default `UUID()` in `ForEach(id:)`, and do not extract reusable subviews as `@ViewBuilder` computed properties (use real `View` structs so the AttributeGraph can preserve identity).

For lifecycle, do not put async data loading in `.onAppear` on a view that can re-appear (use `.task`), and do not start unstructured `Task { }` blocks inside a body.

For concurrency, do not use `DispatchQueue.main.async` or `DispatchQueue.global()` in new code, and do not reach for `@unchecked Sendable` without a comment naming the synchronization mechanism.

For Liquid Glass, do not apply `.glassEffect()` to list rows, content tiles, full-screen backgrounds, or as a background to glass (nested glass). Do not place text directly on a glass surface — text needs an opaque layer underneath.

For iOS, inventory Required Reason API and required-SDK usage and ship the applicable Privacy Manifest declarations, do not ask permissions on first launch, do not fall back to fingerprinting after an ATT denial, and do not use `UIImagePickerController` in new code.

For macOS, do not use `SMLoginItemSetEnabled` / `SMJobBless` (use `SMAppService`), do not run `altool` (use `notarytool`), and do not ship Touch Bar code — the hardware is gone.

For logging and debugging, do not use `print()` in production code paths (use `os.Logger`), do not ship `Self._printChanges()` outside a `#if DEBUG` guard, and do not store secrets or sensitive personal data in `UserDefaults`. Use Keychain for small secrets/keys and appropriately protected files or databases for larger user data.

## Output format for reviews

Group findings by file. For each issue, name the file and line number, state the rule being violated in a short clear sentence, and show a short before / after code block when it helps. Skip files with no issues. End the review with a prioritized summary ordered by impact: accessibility bugs, data-flow errors, and security problems go first; deprecated API and architectural drift in the middle; style nits last (if at all).

For example:

````
### ContentView.swift

**Line 24: The icon-only button has no accessibility label.**

```swift
// Before
Button(action: addUser) { Image(systemName: "plus") }

// After
Button("Add User", systemImage: "plus", action: addUser)
```

### Summary

1. **Accessibility (high):** Icon-only button on line 24 has no label.
2. **State (high):** `@AppStorage` directly inside an `@Observable` class on line 41 will compile but not trigger view updates.
3. **Deprecated API (medium):** `foregroundColor` on line 12 should be `foregroundStyle`.
````

## What changed in 2026

A few features are genuinely new and worth surfacing as "new" when they fit.

Approachable Concurrency (Swift 6.2+) makes most app code implicit `@MainActor`. `Observations { }` is an async sequence over `@Observable` changes (iOS 26+). `@Animatable` (iOS 26) replaces hand-written `animatableData` boilerplate. `ConcentricRectangle` plus `.containerShape()` is the modern shape pair for iOS 26 sheets. `BGContinuedProcessingTask` (iOS 26) gives long-running background work a system-presented progress UI. Liquid Glass and the three adoption paths are new in iOS 26 / macOS 26. `PermissionKit` (iOS 26) handles parental approval for child-account apps. `DeclaredAgeRange` ships `requestAgeRange` from iOS 26.0 and `isEligibleForAgeFeatures` from iOS 26.2. Swift Testing is the Xcode 26 default for new test targets. The Xcode 26 SwiftUI instrument with the Cause & Effect Graph is the new way to profile view-update perf. Foundation Models (iOS 26, three-billion-parameter on-device LLM) is for narrow structured-output tasks, not for GPT-class chat. SwiftData CloudKit sync is still private-database-only — use Core Data when you need shared or public.

The `@Entry` macro that some sources mark as "iOS 26" is actually older — it was introduced with Xcode 16 / iOS 18 and is back-deployable through the macro expansion to earlier iOS versions.

## Commands

This skill ships five commands.

`/swift-critique` is the comprehensive code review. It runs through architecture, state, types, concurrency, design, accessibility, performance, security, platform, persistence, and testing in order, and loads the relevant references as it finds signal. This is the command for "review my SwiftUI."

`/swift-architect` is the deeper, architecture-focused review. It skips style nits and concentrates on the MV vs. MVVM call, the TCA decision, App.swift ownership, folder structure, modularization, and navigation patterns. Use this for "review my architecture."

`/swift-ios` is the iOS-focused review. It concentrates on Privacy Manifest, App Tracking Transparency, App Intents, Widgets and Live Activities, scoped permission APIs, App Attest, push notifications, Foundation Models, and StoreKit 2 alongside the cross-cutting rules. Use this for "review my iOS app."

`/swift-mac` is the macOS-focused review. It concentrates on main menu and keyboard shortcuts, MenuBarExtra, document apps, drag-and-drop via Transferable, sandboxing and Hardened Runtime, notarization with notarytool, Sparkle 2.x, SMAppService, and AppKit interop. Use this for "review my Mac app" or "does this app feel like a Mac app?"

`/swift-teach` explains a SwiftUI or Swift concept like a teacher — strong opinion, modern patterns, working code, pointers to the deeper references. Use this for "explain `@Observable` to me."

## References

The skill ships eighteen reference files. Load them on demand.

```
references/
├── anti-patterns.md           the explicit "don't" list — consulted first
├── architecture.md            MV pattern, App.swift singletons, folders, TCA/SPM triggers
├── state-and-observation.md   @Observable, ownership matrix, @AppStorage trap
├── navigation.md              NavigationStack + Router pattern
├── view-composition.md        extraction, modifiers, body rules
├── lifecycle.md               init traps, .task vs onAppear, identity, scene phase
├── concurrency.md             Approachable Concurrency, MainActor, Sendable
├── design-system.md           tokens, typography, Dynamic Type, @Entry
├── liquid-glass.md            three paths, opt-outs, anti-glass voices
├── animation.md               named springs, @Animatable, Reduce Motion
├── accessibility.md           CI audits, Dynamic Type, contrast, text-on-glass
├── performance.md             identity, lazy stacks, Instruments SwiftUI
├── persistence.md             SwiftData/CoreData/SQLiteData/GRDB triggers, traps
├── ios-platform.md            App Intents, Widgets, Live Activities, permissions
├── macos-platform.md          menus, MenuBarExtra, sandbox, notarytool, Sparkle
├── modern-api.md              deprecation/replacement table (iOS 26 entries)
├── swift-idioms.md            language patterns (Swift 6.2/6.3)
└── testing-and-debugging.md   Swift Testing, previews, Instruments, os.Logger
```
