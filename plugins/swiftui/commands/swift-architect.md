---
description: SwiftUI/Swift architecture review — MV vs MVVM vs TCA triggers, App.swift singletons, folder structure, modularization decisions, navigation patterns.
allowed-tools:
  - Read
  - Glob
  - Grep
  - Bash
  - Skill
argument-hint: "[file-or-directory]"
---

# SwiftUI Architect

Load the `swiftui-expert` skill, then perform a focused architectural review. This command is narrower than `/swift-critique` — it skips style/modifier nits and concentrates on structural decisions.

## Pre-flight context

Before flagging anything, gather context in parallel:

1. **Project scale** — `wc -l **/*.swift`, screen count (count `View` types), engineer count from git log if accessible.
2. **Architecture baseline** — look for:
   - `*ViewModel.swift` filenames (MVVM signal)
   - `import ComposableArchitecture` or `@Reducer` (TCA signal)
   - `@Observable` density (MV signal)
   - `Coordinator` / `Router` files (nav-coordinator pattern)
3. **Folder structure** — `Features/`, `DesignSystem/`, `Core/`, layer-first or feature-first?
4. **SPM packages** — count `Package.swift` files (modularization signal).
5. **Build target structure** — single Xcode target or multi-target / SPM-shelled?
6. **Deployment target** — iOS version sets architecture options.

## What to review

If `$ARGUMENTS` specifies a file/directory, focus there. Otherwise audit the whole project's architecture.

## Architecture checks (run in order)

### 1. App.swift root state

Read `*App.swift` (the `@main` struct).

- **PASS**: `@State` of `@Observable` singletons + `.environment(_:)` injection.
- **FLAG**: `@StateObject`, custom `EnvironmentKey` for primary singletons, or no shared state at all (then where does it live?).
- **FLAG**: `ObservableObject` types injected with `.environmentObject(_:)`.

Recommended pattern:
```swift
@main struct MyApp: App {
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

→ See `references/architecture.md` § App.swift singleton pattern.

### 2. ViewModel layer audit

For every `*ViewModel.swift` file (or `*VM.swift`, `*Model.swift` that wraps a screen):

Apply the trigger check:
- ✅ Explicit state machine or orchestration complexity? (loading/loaded/error/empty, retry, pagination, optimistic updates, request deduplication)
- ✅ Stable lifecycle needed across view recreation, navigation, or scene changes?
- ✅ Meaningful test seam for sequencing, cancellation, retry policy, validation, or reconciliation?
- ✅ Migrating an existing UIKit/AppKit ViewModel, or following an intentional team architecture?

If none apply and the type only forwards a store or mirrors display state, flag the wrapper as unnecessary and show the simpler MV shape. Do not classify a ViewModel from its filename or test count alone.

If one or more apply, keep the ViewModel and review these sub-issues:
- VM imports SwiftUI (should be UI-framework-agnostic).
- VM owns navigation state (should live in Router).
- VM hosts `@Query` (impossible — `@Query` requires Environment).
- VM is `struct` (should be `@Observable class`).
- VM owned by `@StateObject` (should be `@State`).

→ See `references/architecture.md` § ViewModels — when to use, when to skip.

### 3. Navigation architecture

- One `NavigationStack(path:)` per tab? Or multiple stacks sharing a path (anti-pattern)?
- One `@Observable Router` per tab? Or coordinator chain / Stinsen / SUICoordinator (legacy for new SwiftUI)?
- Typed `Hashable` route enums, not strings?
- `.sheet(item:)` with `Identifiable` enum, not `sheet(isPresented:)` with manual bool toggling?
- `.navigationTransition(.zoom(sourceID:in:))` used where appropriate?

→ See `references/navigation.md`.

### 4. TCA presence check

If `import ComposableArchitecture` is present:

- Does the app have cross-feature state/effect coordination that vanilla Observation is making difficult?
- Does deterministic action/effect testing, dependency control, cancellation, or replayability solve a concrete product risk?
- Does state need a lifecycle independent of a particular view hierarchy?
- Has the team accepted the action/reducer model, learning cost, SourceKit cost, and third-party dependency?
- If the product is regulated, does its actual assurance plan benefit from reducer-level exhaustive tests? Regulation alone is neither necessary nor sufficient.

If those benefits do not outweigh the costs, flag TCA as overkill and suggest vanilla SwiftUI + `@Observable Router` + focused stores. If the trade-off is justified, keep TCA and flag only concrete misuse (massive reducer files, deeply nested enums, every trivial screen reduced, etc.).

→ See `references/architecture.md` § TCA — when to use, when to skip.

### 5. Folder structure

Check root-level layout:

- **PASS**: `App/`, `Features/<Feature>/`, `DesignSystem/`, `Core/<Service>/`, `Resources/`.
- **FLAG**: layer-first (`ViewModels/`, `Views/`, `Models/` at root).
- **FLAG**: monolithic with no separation when the project has clearly grown past small-app feel and build times are hurting.
- **FLAG**: SPM-modularized for a small solo project where modularization costs exceed any benefit.

Apply modularization triggers:
- Project-file merge conflicts becoming routine, build times hurting iteration, or feature boundaries that have stabilized → modularize.
- Solo / POC / early-stage project / build times still fast → flat.

→ See `references/architecture.md` § Folder structure / SPM modularization.

### 6. SPM package structure

For projects with local SPM packages:

- Is there a `DesignSystem` package extracted first (no app dependencies)?
- Is `Networking` extracted before per-feature?
- Are feature packages independent (no cross-feature imports)?
- Is `defaultIsolation(MainActor.self)` set on UI packages?

→ See `references/architecture.md` § SPM modularization.

### 7. File-per-type compliance

- Flag files with 3+ unrelated public types.
- Don't enforce strict "one type per file" — co-located private helpers are fine. IceCubesApp's `Router.swift` has 5 types; that's normal.
- Naming: `<Name>Screen.swift` (full-screen), `<Name>View.swift` (reusable component), `<Name>Service.swift` (services), `<Name>Store.swift` / `<Name>Manager.swift` (shared state). Never `<Name>ViewModel.swift` unless using the VM pattern intentionally.

### 8. Service / dependency injection

- Are services owned by `App.swift` as `@Observable` and injected via `.environment(_:)`?
- Or via constructor injection (also acceptable)?
- Or hidden behind `.shared` singletons (flag; untestable)?

### 9. Test target structure

- Swift Testing vs XCTest split.
- Test target setup (separate target or in-app?).
- Shared test helpers in their own package?

### 10. Multi-platform (iOS + macOS) structure

If the project targets both:

- Shared SPM packages for cross-platform code?
- Platform-specific code in `Platforms/iOS/` and `Platforms/macOS/`?
- Conditional compilation (`#if os(macOS)`) used sparingly?

## Output format

Group by architectural area, not by file. Use headers per concern.

### Example

**Architecture overview**

This is a ~50-screen, 5-engineer iOS app with one local SPM package (`Networking`). The architecture is hybrid MV + per-screen ViewModels. Liquid Glass selectively adopted (5 call sites).

**1. App.swift root state — PASS**
`AppDelegate.swift` correctly owns `@State` of `Theme`, `Router`, `AuthStore` and injects via `.environment(_:)`. No issues.

**2. ViewModel layer — FLAG (3 issues)**

The project has 22 `*ViewModel.swift` files. Audited each:

- **PASS (8 files)**: complex state-machine screens (timeline pagination, conversation orchestration) — keep.
- **FLAG (14 files)**: display-mostly screens. Recommend MV pattern — extract `View` structs and remove the wrapper.

Example: `ProfileViewModel.swift` has:
```swift
@Observable class ProfileViewModel {
    var profile: Profile?
    func load() async { profile = await api.fetchProfile() }
}
```

This adds no testable orchestration. Replace with `.task` in `ProfileScreen.swift`:
```swift
struct ProfileScreen: View {
    @State private var profile: Profile?
    var body: some View {
        Group { ... }
            .task { profile = await api.fetchProfile() }
    }
}
```

**3. Navigation — FLAG**

`AppRouter.swift` uses string-based routes:
```swift
// Before
router.push("profile/\(userId)")
```

Recommend typed `Hashable` enum:
```swift
// After
enum Route: Hashable {
    case profile(UserID)
}
router.push(.profile(userId))
```

**4. Folder structure — PASS** — feature-first, correctly modularized.

**5. SPM packages — FLAG**

Build times >30s reported. Recommend extracting `DesignSystem` to a local SPM package next. Bottom-up order: DesignSystem → Networking (done) → per-feature.

### Summary

1. **ViewModel layer (high)**: 14 of 22 VMs are anti-pattern wrappers — recommend MV pattern.
2. **Navigation (medium)**: string-based routes → typed `Hashable` enum.
3. **SPM (low, future)**: extract `DesignSystem` to package when build time pain warrants.

### Recommended follow-up

- Run `/swift-critique` for the full anti-pattern sweep (its state-and-observation category covers state/typing depth).
- If considering TCA migration, read `references/architecture.md` § TCA triggers carefully — your project doesn't fit the trigger profile yet.

## Output rules

- Don't pad with style nits — those belong in `/swift-critique`.
- Cite the architecture-md reference where decisions are explained.
- For "depends" decisions, name the concrete trigger conditions you applied.
- If the project's architecture is genuinely sound, say so — don't manufacture findings.
