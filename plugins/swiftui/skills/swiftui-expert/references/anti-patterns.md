# Anti-patterns — the "never do this in 2026" list

Critique starts here. Any SwiftUI/Swift code review consults this file first to flag the patterns that should never reach a PR. Grouped by domain. Modern alternatives in the right column.

The skill consuming this file is opinionated by design — an AI invoking critique without this list produces hedged spaghetti reviews.

---

## State management

| Anti-pattern | Modern replacement |
|---|---|
| `ObservableObject` + `@Published` for new code | `@Observable` macro |
| `@StateObject` for new code | `@State` for owning, plain `let` for receiving |
| `@ObservedObject` for new code | plain `let model: MyModel` (read) or `@Bindable var model` (with bindings) |
| `.environmentObject(_:)` injection | `.environment(_:)` (typed for `@Observable`) |
| `Binding(get:set:)` in body | `@State` + `.onChange(of:initial:_:)` |
| `@AppStorage` directly inside `@Observable` class | IceCubesApp pattern: a **plain (NOT `@Observable`) inner `Storage` class** holds the `@AppStorage` properties; the outer `@Observable` class mirrors them as stored `var`s with `didSet` writing through to storage — full code in `state-and-observation.md`. Or: `@ObservationIgnored` + manual `UserDefaults` + `access(keyPath:)` / `withMutation(keyPath:)` |
| Mixing `@Published` and `@Observable` in the same type | All-or-nothing; pick `@Observable` |
| One huge `AppModel` with everything | Decompose: per-domain `@Observable` instances + per-view `@Observable` when warranted |
| `@State` for receiving from parent | plain `let` (read) or `@Bindable` (with bindings) |
| `@State` not marked `private` | Always `@State private var` |
| Passing values INTO `@State` from parent expecting re-seed | `@State` captures initial once; pass via plain `let` instead |
| Custom `EnvironmentKey` for primary app singletons | `.environment(_:)` of `@Observable` instances |

## Navigation

| Anti-pattern | Modern replacement |
|---|---|
| `NavigationView` | `NavigationStack` / `NavigationSplitView` |
| `NavigationLink(destination:)` (eager push) | `navigationDestination(for: Route.self)` + typed routes |
| String-based routes | `enum Route: Hashable` |
| Multiple `NavigationStack`s sharing a `NavigationPath` | One stack per tab; one Router per tab |
| `sheet(isPresented:)` with manual state | `sheet(item:)` driven by `Identifiable` enum |
| `Stinsen` / `SUICoordinator` for new SwiftUI | `@Observable Router` with `NavigationPath` |
| `Hashable` route enum without typed `navigationDestination` | Bind each route case in `navigationDestination(for: Route.self)` |

## Modifiers and style

| Anti-pattern | Modern replacement |
|---|---|
| `.foregroundColor(_:)` | `.foregroundStyle(_:)` |
| `.accentColor(_:)` | `.tint(_:)` |
| `.cornerRadius(_:)` on view | `.clipShape(.rect(cornerRadius:, style: .continuous))` |
| `.font(.system(size: 14))` without `relativeTo:` | `.font(.body)` (Dynamic Type ramp) OR `.font(.custom("Brand", size: 14, relativeTo: .body))` |
| Inline `Color(hex: "#...")` per file | Asset catalog color set OR `ThemedColor: ShapeStyle` token |
| Magic numbers in `padding(16)` / `cornerRadius: 12` | Named tokens: `Spacing.md`, `Radius.card` |
| Multiple radius values per role (drift) | One radius per role; `Radius.button`, `Radius.card`, `Radius.modal` |
| `.presentationBackground(.thinMaterial)` on iOS 26 sheets | Remove — suppresses the new Liquid Glass style |
| Custom toolbar item backgrounds in iOS 26 | Remove — they interfere with scroll-edge effect |

## Views and composition

| Anti-pattern | Modern replacement |
|---|---|
| `AnyView` (especially in lists) | Generics, `@ViewBuilder`, `Group` |
| `@ViewBuilder` computed properties for reusable subviews | Extract real `View` structs (preserves identity, participates in AttributeGraph diffing) |
| `UUID()` defaulted in `ForEach(id:)` | Stable `id`: `ForEach(items) { ... }` with `Identifiable` conformance |
| Stored escaping closures in container views | `@ViewBuilder let content: () -> Content` |
| `body` over 60 lines / complexity > 10 | Extract subviews and modifiers |
| Multiple public types in one file | One primary public type per file; private helpers co-located |
| `if cond { X() } else { X() }` (creates two structural identities) | `X().disabled(!cond)` or `.opacity(cond ? 1 : 0)` |
| Computed `some View` property that returns state-dependent logic | Real `View` struct that takes the dependency as a parameter |

## View lifecycle

| Anti-pattern | Modern replacement |
|---|---|
| `.onAppear` for data loading in views that can re-appear (lazy lists, tabs) | `.task { }` (auto-cancels) or `.task(id: ...) { }` |
| Unstructured `Task { }` in view bodies | `.task { }` modifier |
| `.onAppear { Task { ... } }` for new code | `.task { }` |
| One-parameter `onChange(of:perform:)` | `onChange(of: value, initial: false) { _, new in }` (iOS 17+) |
| Work in `View.init()` (it runs hundreds of times) | Move to `.task` or `.onAppear` (for a one-shot) |
| `print("view created")` in `init` reasoning about it as UIKit `viewDidLoad` | Treat `init` as zero-side-effect cheap construction |
| Storing `@State var vm = ExpensiveModel()` (the default expr runs on every parent re-render) | Compute in `.task` or lazy-init pattern |

## Concurrency

| Anti-pattern | Modern replacement |
|---|---|
| `DispatchQueue.main.async { }` | `@MainActor` annotation / `await MainActor.run { }` / Approachable Concurrency default |
| `DispatchQueue.global().async { }` | `@concurrent` function or `Task { ... await heavyFunc() }` |
| `Task.sleep(nanoseconds: 1_000_000_000)` | `Task.sleep(for: .seconds(1))` |
| `Task.detached { }` casual usage | Regular `Task { }` (inherits context) |
| `@unchecked Sendable` as a fix | `final class { let ... } : Sendable` OR `actor` OR value type |
| `withCheckedContinuation` for new async APIs | Native `async`/`await` from the start |
| Silent error swallowing | Surface error to user / log meaningfully |
| `MainActor.run()` when already on MainActor | Check project default isolation; remove if MainActor-default |

## Animation

| Anti-pattern | Modern replacement |
|---|---|
| Raw `.spring(response:dampingFraction:)` | Named springs `.smooth`, `.snappy`, `.bouncy` |
| Bouncy springs on loaders / progress indicators | Linear curves |
| `.animation(...)` without `value:` parameter | `.animation(_:value:)` |
| Manual `animatableData` implementation | `@Animatable` macro (iOS 26) |
| UIKit haptics (`UIImpactFeedbackGenerator`) | `sensoryFeedback()` |
| Animations that ignore `accessibilityReduceMotion` | Swap to `.linear(duration: 0.15)` or `nil` when reduce-motion is on |

## Accessibility

| Anti-pattern | Modern replacement |
|---|---|
| Icon-only `Button(action:)` with `Image(systemName:)` | `Button("Label", systemImage: "name", action:)` (gives VoiceOver a label) |
| `onTapGesture` where `Button` works | `Button` — gets accessibility traits for free |
| Decorative images without `accessibilityHidden()` / `Image(decorative:)` | Mark explicitly |
| Color-only state indicators | Combine color + shape/icon |
| `.font(.system(size:))` without `relativeTo:` (kills Dynamic Type) | Use ramp or custom font with `relativeTo:` |
| Skipping `try app.performAccessibilityAudit()` in CI | Add it; fail builds on missing labels |
| Text on glass surfaces | Text on opaque layers only |

## Persistence

| Anti-pattern | Modern replacement |
|---|---|
| `UserDefaults` for secrets or sensitive personal data | Keychain for small secrets/keys; protected files/databases for larger records |
| SwiftData `@Model` without `VersionedSchema` from v1.0.0 | Always wrap from v1 — even if no migration planned |
| `NSPredicate` strings | `#Predicate<Entity> { ... }` macro |
| `@Attribute(.unique)` on CloudKit-synced models | Remove — incompatible with CloudKit |
| Long/lock-holding persistence work crossing suspension without coordination | Shorten/defer it; use `beginBackgroundTask` only for a bounded critical section supported by lifecycle/crash evidence |
| MVVM wrapped around `@Query` | `@Query` requires Environment; use MV pattern for SwiftData apps |
| Forgetting `@ObservationIgnored` on cached derived state in `@Observable` | Mark untracked storage explicitly |
| `@AppStorage` directly inside `@Observable` | Nested storage class workaround |
| SwiftData for shared/public CloudKit (private-only in 2026) | Core Data + `NSPersistentCloudKitContainer` OR SQLiteData (Point-Free) |
| Loose string-based file paths | `URL.documentsDirectory`, `URL.cachesDirectory`, `URL.temporaryDirectory` |

## Previews and testing

| Anti-pattern | Modern replacement |
|---|---|
| `PreviewProvider` | `#Preview` macro |
| Wrapping stateful previews in helper structs | `@Previewable` (Xcode 16+) |
| XCTest as default for new unit tests | Swift Testing (`@Test`, `#expect`, `@Suite`, traits) |
| Snapshot testing every screen | Only design-system primitives; pin device + scheme |
| `sleep(_:)` in UI tests | `XCTWaiter` / polling predicates |
| Hardcoded device sizes (`as: .image(on: .iPhone16Pro)`) | Use device size constants from your snapshot config |
| Singletons reached via `.shared` in tests | Constructor inject dependencies |
| Mocking the framework | Test your code, not Apple's |
| Mocking actors | Real instances with fake clocks |
| Mocking `@Observable` types | Construct with test inputs directly |
| Ad-hoc URLSession mocking | URLProtocol injection with `URLSessionConfiguration.protocolClasses` |
| Shipping `Self._printChanges()` | `#if DEBUG` guard or strip before release |

## Liquid Glass

| Anti-pattern | Modern replacement |
|---|---|
| `.glassEffect()` on list rows / table rows | Chrome only |
| `.glassEffect()` on content tiles / media canvases | Content stays flat |
| `.glassEffect()` on full-screen backgrounds | Glass is for floating chrome; not backgrounds |
| Glass-on-glass (nested `.glassEffect()`) | Single layer of glass per region |
| Decorative tint on glass | Semantic tint only (primary action, state, error) |
| Text directly on glass surface | Text on opaque layers only |
| `.presentationBackground(.thinMaterial)` on iOS 26 sheets | Remove — suppresses the new style |
| Apologizing for `UIDesignRequiresCompatibility = true` | Apple's iWork / Final Cut / Pixelmator opted out. Don't apologize. |

## iOS platform

| Anti-pattern | Modern replacement |
|---|---|
| `UIImagePickerController` for library access | `PhotosPicker` (SwiftUI iOS 16+) / `PHPickerViewController` (UIKit) |
| Asking permissions on first launch | Prime first; ask contextually |
| Fingerprinting fallback after ATT denial | Stop tracking — Apple rejects |
| Custom URL scheme enumeration beyond 50 entries | Universal Links + Associated Domains |
| Required Reason API use without applicable `PrivacyInfo.xcprivacy` reasons | Declare an approved reason; missing manifest is not a finding when no manifest requirement applies |
| Third-party SDK without signed Privacy Manifest | Required since Feb 12, 2025 (ITMS-91061) |
| Pre-iOS-17 widget patterns (read-only billboards) | Interactive widgets (Button/Toggle bound to AppIntent) |
| Custom tooltip systems / homemade onboarding hints | TipKit |
| StoreKit 1 | StoreKit 2 (async/await + JWS validation) |
| `UIViewRepresentable(MKMapView)` for new code | SwiftUI `Map { Marker(...); Annotation(...) }` |
| `EKEventStore.requestAccess(to:completion:)` for write-only | `EKEventEditViewController` (no prompt) |
| Full `Calendar` access where write-only suffices | iOS 17+ write-only: `EKEventEditViewController` |
| Full Contacts access where limited suffices | iOS 18+ `ContactAccessButton` / `contactAccessPicker` |
| Pre-iOS-15 Location prompts where `LocationButton` works | `LocationButton` (no Info.plist key) |
| Primary-account social login without the equivalent option required by Guideline 4.8 | Add Sign in with Apple unless a published Guideline 4.8 exception applies |
| App Attest skipped on apps with fraud/abuse risk | Free, ~50 lines, Secure Enclave-backed — adopt |

## macOS platform

| Anti-pattern | Modern replacement |
|---|---|
| Empty `.commands { }` / missing main menu | Full Edit/File/View/Window/Help menus with `CommandGroup` + keyboard shortcuts |
| Burger menu navigation on Mac | Main menu + keyboard shortcuts |
| Missing keyboard shortcut on primary commands | Every action gets `.keyboardShortcut("c", modifiers:)` |
| FAB (floating action button) in corner of Mac window | Toolbar primary action |
| Pull-to-refresh on Mac scroll view | Toolbar refresh button + keyboard shortcut (⌘R) |
| iPad → Mac Catalyst "just works" framing | macOS is not iOS with a bigger screen |
| `SMLoginItemSetEnabled` / `SMJobBless` for new code | `SMAppService` (macOS 13+) |
| `altool` for notarization | `notarytool` (since November 2023) |
| Kexts (deprecated) | Endpoint Security + System Extensions |
| Touch Bar code | Deprecated hardware — drop |
| `Csqlite3` direct usage in new Mac app | SwiftData / Core Data / GRDB / SQLiteData |
| `NSTextView.layoutManager` access in macOS 26 (silently downgrades to TextKit 1) | TextKit 2 APIs explicitly |
| Saving sandboxed file URL without security-scoped bookmark | `url.bookmarkData(options: .withSecurityScope)` |
| Missing Hardened Runtime for direct distribution | Required for notarization |
| Distributing unnotarized app outside App Store | Notarize via `notarytool` + staple |
| Sparkle 1.x (legacy) | Sparkle 2.x with EdDSA signatures |

## Debugging and logging

| Anti-pattern | Modern replacement |
|---|---|
| `print()` for production logging | `os.Logger` with subsystem/category + privacy interpolation |
| Logging raw PII / tokens | `os.Logger` privacy interpolation: `.public` / `.private` / `.private(mask: .hash)` |
| Symbolicating SwiftUI crashes with third-party tools | Xcode Organizer is the source of truth |
| Eyeballing performance | Instruments SwiftUI template + Cause & Effect Graph |
| `Self._printChanges()` shipped to production | `#if DEBUG` wrap |
| Custom log levels via global functions | `os.Logger.debug/info/notice/warning/error/fault` |
| In-memory log buffers for diagnostics | `OSLogStore` for in-app retrieval |

## Build, distribution, workflow

| Anti-pattern | Modern replacement |
|---|---|
| `altool` for App Store / notarization | `notarytool` |
| Sparkle 1.x without EdDSA | Sparkle 2.x with EdDSA signatures |
| Default Xcode 26 tab system on existing workflow | Settings → Navigation → Pin Editor Tabs: When Tab Is Created (community fix) |
| Updating macOS Tahoe 26.2 while on Xcode 16 | Don't — known incompatibility |
| Spam-accepting AI autocomplete (e.g., the "dateOfKidnapping" cautionary tale) | Review every suggestion |
| Repo-wide Swift 6 strict migration in one PR | Module by module |
| Default Combine for new pipelines | AsyncSequence + `Observations { }` + `@Observable` |

## Cross-references

Each anti-pattern points back to a deeper reference for the WHY and the full alternative:

- State management → `references/state-and-observation.md`
- Navigation → `references/navigation.md`
- Modifiers / style / tokens → `references/design-system.md`, `references/modern-api.md`
- Views / composition → `references/view-composition.md`
- View lifecycle → `references/lifecycle.md`
- Concurrency → `references/concurrency.md`
- Animation → `references/animation.md`
- Accessibility → `references/accessibility.md`
- Persistence → `references/persistence.md`
- Previews / testing → `references/testing-and-debugging.md`
- Liquid Glass → `references/liquid-glass.md`
- iOS platform → `references/ios-platform.md`
- macOS platform → `references/macos-platform.md`
