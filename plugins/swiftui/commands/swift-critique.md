---
description: Deep SwiftUI / Swift code critique — read the target code and apply the full review process. Evaluates architecture, state and observation, navigation, view composition, lifecycle, concurrency, design system, accessibility, performance, animation, Liquid Glass adoption, persistence, platform conventions, and testing. Think like a senior iOS / macOS engineer giving honest feedback.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
argument-hint: "<target> [--pre-merge | --harden | --architect]"
---

# SwiftUI Critique

Conduct a thorough code critique of SwiftUI / Swift code. Think like a senior iOS or macOS engineer reviewing a PR — be direct, be specific, explain *why* each finding matters in production terms.

**First**: use the `swiftui-expert` skill for the review rubric, severity tiers, reference files, and decision rules. The skill has the framing; this command is the workflow.

## Modes

The default mode surfaces everything. The flags below prioritize a subset.

- **(default)** — holistic review across all eighteen reference categories. All severities surfaced.
- **`--pre-merge`** — prioritize polish-level findings before merging a PR: deprecated API, dead code, debug artifacts, `print()` left in, `Self._printChanges()` outside `#if DEBUG`, missing accessibility audit in CI, `PreviewProvider` not migrated to `#Preview`.
- **`--harden`** — prioritize production-hardening findings: tokens in `UserDefaults`, applicable Required Reason API declarations missing from `PrivacyInfo.xcprivacy`, `@AppStorage` inside `@Observable` (the silent-no-updates trap), SwiftData `@Model` without `VersionedSchema` from v1, measured suspension-sensitive database work lacking lifecycle coordination, icon-only buttons without accessibility labels, fingerprinting fallback after ATT denial.
- **`--architect`** — prioritize architecture-level findings: `App.swift` ownership of shared state, MV-vs-VM call, navigation router shape, file-per-type compliance, folder structure, modularization posture. Route to `/swift-architect` for design-level work.

## Preparation

Before flagging anything, gather context. Run these in parallel.

1. **Deployment target.** Inspect `Package.swift`, `*.xcodeproj/project.pbxproj`, and any `.xcconfig` files. Note the iOS / macOS / visionOS minimum. Suggest version-gated APIs only when the project actually supports them.
2. **Swift version.** Same files, `swift-tools-version` and Swift 6 mode flags.
3. **State posture.** `rg -l '@Observable' .` vs `rg -l 'ObservableObject' .`. A project that consistently uses one pattern needs a single project-level migration note, not a per-file flag.
4. **Architecture baseline.** Look for `Features/`, `DesignSystem/`, a local SPM packages folder, and `*ViewModel.swift` filenames. These tell you the team's existing conventions.
5. **Liquid Glass posture.** `Info.plist` for `UIDesignRequiresCompatibility`; code for `.glassEffect`. Tells you whether the team has taken Path A, B, or C from the skill.
6. **Concurrency posture.** `Package.swift` for `defaultIsolation(MainActor.self)`; code for `@MainActor` annotation density (low density usually means Approachable Concurrency is on).
7. **Tests posture.** Swift Testing files (`@Test`, `@Suite`) vs XCTest classes. Most active codebases are mid-migration; partial coverage is normal.

## Automated sweep

Grep is fast. Before reading code in depth, scan for the cheapest signal.

```bash
# Legacy state wrappers (signal for project-level migration)
rg -c '\bObservableObject\b|\b@Published\b|\b@StateObject\b|\b@ObservedObject\b' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "legacy state:", s+0}'

# Deprecated navigation
rg -c '\bNavigationView\b' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "NavigationView:", s+0}'

# Deprecated modifiers
rg -c '\.foregroundColor\(|\.accentColor\(|\.cornerRadius\(' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "deprecated modifiers:", s+0}'

# Dynamic Type killers
rg -c '\.font\(\.system\(size:' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "static font sizes:", s+0}'

# Identity / performance
rg -c '\bAnyView\b|UUID\(\)\s*\}' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "AnyView / UUID in identity:", s+0}'

# Unstructured Task in bodies
rg -c '\.onAppear\s*\{[^}]*Task\s*\{|^\s*Task\s*\{' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "Task in body / onAppear:", s+0}'

# Concurrency anti-patterns
rg -c 'DispatchQueue\.main|DispatchQueue\.global|Task\.sleep\(nanoseconds:|@unchecked Sendable|Task\.detached' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "GCD / @unchecked / detached:", s+0}'

# State traps
rg -c '@AppStorage[^@]*@Observable|@Observable[^@]*@AppStorage' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "@AppStorage + @Observable colocation:", s+0}'

# Production logging hygiene
rg -c '\bprint\(' . --type swift -g '!*Test*' -g '!*Preview*' 2>/dev/null | awk -F: '{s+=$2} END {print "print() in non-test code:", s+0}'

# Previews not modernized
rg -c 'PreviewProvider' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "PreviewProvider:", s+0}'

# Image picker legacy
rg -c 'UIImagePickerController' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "UIImagePickerController:", s+0}'

# Mac-only: legacy login items
rg -c 'SMLoginItemSetEnabled|SMJobBless' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print "legacy login item APIs:", s+0}'

# Privacy Manifest presence (iOS targets only)
if find . -name 'PrivacyInfo.xcprivacy' -print -quit | grep -q .; then
    echo "PrivacyInfo.xcprivacy: present"
else
    echo "PrivacyInfo.xcprivacy: absent — inventory Required Reason APIs and required SDK manifests before classifying"
fi
```

These numbers set context for the rest of the review.

## The review

Walk the eighteen-category order from the `swiftui-expert` skill's "Review process — the rubric" section. Load a reference only when the code under review actually shows signal in that category — most reviews touch five or six categories, not all eighteen.

For each finding, follow the for-each-finding template:

1. **File and line.** Exactly where.
2. **Severity.** One of `blocking`, `important`, `nit`, `suggestion`, `praise`.
3. **What.** Name the problem in one sentence.
4. **Why it matters.** Concrete production cost in plain language. What bug appears, what crashes, what gets rejected by App Review, what becomes hard to change. If you cannot name a concrete consequence, the finding belongs as `suggestion` or should not be raised. Vagueness is not humility — it is lower-quality work.
5. **Fix.** Before / after code block when the fix is non-obvious.

## Decision rules to apply

When a finding turns on a debated topic, consult the skill's "Conditional decisions" section before writing "consider X vs Y." The genuinely-optional choices have criteria-based answers:

- MV pattern vs. per-screen ViewModel: trigger criteria in `references/architecture.md`.
- TCA vs. vanilla SwiftUI: trigger criteria in `references/architecture.md`.
- SwiftData vs. Core Data vs. SQLiteData vs. GRDB: trigger criteria in `references/persistence.md`.
- AppKit vs. SwiftUI for Mac: trigger criteria in `references/macos-platform.md`.
- Liquid Glass Path A / B / C: trigger criteria in `references/liquid-glass.md`.
- Local SPM packages vs. flat target: trigger criteria in `references/architecture.md`.

If your finding lives in any of these decisions, cite the criteria rather than hedging.

## Generate the critique report

The report has the sections below. Skip any section with no findings.

### Quick stats

Start with the automated scan numbers and the project-context findings (deployment target, state posture, Liquid Glass posture, test stack). These set context for the rest.

### What's working

Two or three things the code does well. Be specific about *why* they work — this reinforces good patterns so they propagate. Use the `praise` severity.

### Priority issues

The most impactful findings, in severity order: `blocking` first, then `important`. Each gets file:line, severity, what, why, and fix.

### Minor observations

Quick notes on smaller issues. One sentence each. Severity `nit` or `suggestion`.

### What I'm not flagging, and why

A deliberate pass on things that look like they should be flagged but where the right call is to leave them alone. Document this — it is often more valuable than the findings themselves, because it tells the next reviewer what was already considered.

Examples of things to leave alone with a note:

- A `*ViewModel.swift` file on a screen with a real state machine and orchestration tests. The triggers in `references/architecture.md` are met; not a finding.
- `@unchecked Sendable` with a clear synchronization comment. Documented decision; mention as praise.
- A Mac app that uses pure AppKit for its core surface when that surface is a text editor, a video player, or a system monitor. Correct call for the shape of the app.
- `UIDesignRequiresCompatibility = true` in a creative or pro app's Info.plist. Apple's own pro apps shipped this way; not a finding unless the app is a productivity / RSS / utility app where Path A would be a fit.
- A working `ObservableObject` in a codebase that targets iOS 16. Note the migration once at project level; do not flag every instance.
- A `Podfile` in an actively maintained codebase. CocoaPods is not dead.

The general principle: re-litigating a decision the developer has already made costs the team more than a one-paragraph note explaining why the existing choice is fine.

### Pattern-recognition pass

Step back from line-by-line and ask:

- Is this app's architecture coherent? MV across the board, or MVVM throughout, or mixed in a way that confuses the next reader? Mixed architectures are an `important` finding.
- Does `App.swift` own the shared state? Or is shared state hiding in singletons, custom EnvironmentKeys, or scattered `@StateObject` properties? Drift here propagates everywhere.
- Is the navigation router pattern in place? One stack per tab, one router per tab, typed routes? Or are there strings, multiple stacks sharing a path, or eager `NavigationLink(destination:)` calls?
- Are advanced features (custom property wrappers, manual `Animatable` conformance, hand-rolled observation tracking) leaking into application code where the modern macros would do the job? Keep advanced features behind crisp library interfaces.
- Is the team taking on dependencies (TCA, third-party state management, a custom architecture framework) that the project size and team shape do not justify? Trigger criteria in `references/architecture.md`.

If architecture-level findings are non-trivial, suggest `/swift-architect` for a focused design-level pass.

### Questions to consider

Provocative questions that might unlock a better design:

- "Who actually owns this `@Observable` instance?"
- "Does this view need to know about the network at all?"
- "What would happen if this `@Query` returned thousands of rows?"
- "If this app's data model needed CloudKit sharing tomorrow, where does the work happen?"
- "What does this screen do when the user is offline?"
- "If we deleted this `*ViewModel.swift` file and moved its state to the view, what would break?"
- "What does the Mac version of this look like? Are we leaving the menu bar empty?"

### Suggested follow-up

When the findings cluster around a focused workflow, suggest the matching command. Use judgment, not a count:

- If architecture findings dominate (mixed patterns, App.swift drift, navigation legacy, modularization questions): `/swift-architect`.
- If the user wants to understand a specific concept that came up (`@Observable`, Approachable Concurrency, Liquid Glass adoption paths): `/swift-teach <concept>`.

Skip the suggestion section if no follow-up genuinely helps.

## Output format

Group findings by file. For each issue: the file and line, the severity tier, a one-sentence statement of what is wrong, the why-it-matters in plain language, and a short before / after code block when non-obvious.

Skip files with no issues. End with a prioritized summary, blocking and important first, nits and suggestions last.

### Example

````
### Settings.swift

**Line 14 (blocking): `@AppStorage` inside an `@Observable` class — does not trigger view updates.**

`@AppStorage` is a SwiftUI `DynamicProperty`. The `@Observable` macro rewrites stored properties into computed properties, so the wrapper does not get attached the way it does on a view. The code compiles, but no view re-renders when the value changes — toggling the setting from the UI silently does nothing.

```swift
// Before — silently broken
@Observable
final class Settings {
    @AppStorage("darkMode") var darkMode = false
}

// After — verified IceCubesApp pattern
@MainActor
@Observable
final class Settings {
    final class Storage {
        @AppStorage("darkMode") var darkMode = false
    }
    private let storage = Storage()

    var darkMode: Bool {
        didSet { storage.darkMode = darkMode }
    }

    private init() {
        darkMode = storage.darkMode
    }

    static let shared = Settings()
}
```

### ContentView.swift

**Line 24 (important): Icon-only `Button` has no accessibility label.**

VoiceOver reads the symbol name (`"plus"`) rather than the action. Users on VoiceOver hit the button thinking it does something else, or skip it entirely.

```swift
// Before
Button(action: addUser) { Image(systemName: "plus") }

// After
Button("Add User", systemImage: "plus", action: addUser)
```

**Line 12 (nit): `.foregroundColor` deprecated since iOS 17 — use `.foregroundStyle`.**

### Summary

1. **blocking — Settings.swift:14.** `@AppStorage` inside `@Observable` will silently break. Apply the verified IceCubesApp pattern.
2. **important — ContentView.swift:24.** Icon-only button is unusable on VoiceOver.
3. **nit — ContentView.swift:12.** `.foregroundColor` → `.foregroundStyle`.

### What I'm not flagging

- `UserDefaults.standard` usage on line 67 — it stores a single boolean preference, not credentials. Keychain would be overkill.
- `Color(red: 0.9, green: 0.4, blue: 0.2)` on line 88 — appears once in this file, no drift elsewhere. Not worth pulling into a `DesignSystem` module for a single literal.
````

The example above is illustrative, not a template. Adapt the severity mix and the "what I'm not flagging" notes to what the actual code shows.

## How to do this well

Be direct and specific. "Line 41 of `Settings.swift` has `@AppStorage` directly inside an `@Observable` class — this compiles but never triggers view updates because `@AppStorage` is a SwiftUI `DynamicProperty` and the macro rewrites stored properties into computed properties" beats "some properties might not update." Vagueness is not humility.

Every fix gets a verification step. "Move `@AppStorage` to a plain inner `Storage` class; wrap the outer property as `var darkMode: Bool { didSet { storage.darkMode = darkMode } }`; seed in private init" → "after the fix, toggling the setting from the UI should re-render every observer of `userPreferences.darkMode` — if it does not, the outer property is still computed or `@ObservationIgnored`, both of which break tracking." A fix without verification is half a fix.

Prioritize ruthlessly. A finding without a concrete production cost is a `suggestion` or a `nit`, not an `important`. If everything is `important`, nothing is.

On settled questions, give the settled answer. The five non-negotiables in `SKILL.md` and the conditional-decision triggers in the references exist so the plugin does not relitigate the same calls each review. Reserve "this is unsettled" for genuinely unsettled questions (a brand-new iOS 26 API where Apple's guidance is still evolving, a SwiftData edge case where the production stories disagree).

Say "I don't know" when you do not know. If you are unsure whether an API exists in the target's iOS version, whether a deprecation is real, whether a third-party library is still maintained — say so and point the developer at the source they should check. A confident invention costs more time than an honest gap.

Be honest about what you did not check. If you scanned ten files and not eleven, say so. If you read the surface but not the call sites, say so. The team can fill the gap; they cannot fill a gap you concealed.
