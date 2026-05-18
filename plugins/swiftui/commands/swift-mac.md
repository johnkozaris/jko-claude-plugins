---
description: macOS-focused SwiftUI / Swift code review. Concentrates on Mac-specific concerns — main menu and keyboard shortcuts, MenuBarExtra, documents and DocumentGroup, drag-and-drop via Transferable, sandboxing and Hardened Runtime, notarization with notarytool, Sparkle 2.x distribution, AppKit interop — and on whether the app actually feels like a Mac app.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
argument-hint: "<target>"
---

# SwiftUI macOS Critique

Conduct a focused review of a macOS app target. The `swiftui-expert` skill covers the general SwiftUI rubric; this command narrows the lens to macOS-specific platform concerns and to the question that matters most for Mac apps: does this thing feel like a Mac app, or does it read as an iPad port?

**First**: use the `swiftui-expert` skill for the review rubric, severity tiers, and reference files. The reference of record for this command is `references/macos-platform.md`.

## Why this command exists

macOS is not iOS with a bigger screen. Mac users live in the menu bar and the keyboard. A Mac SwiftUI app that ships with an empty `.commands { }`, no keyboard shortcuts on its primary actions, no support for drag-and-drop from Finder, and no `DocumentGroup` for a document-style app — reads as a Catalyst-grade port even when the SwiftUI underneath is well-written. The platform conventions are the review surface.

Beyond conventions, distribution on the Mac has its own stack: the App Sandbox and its entitlements, Hardened Runtime, notarization via `notarytool` (Apple stopped accepting `altool` uploads on November 1, 2023), Sparkle 2.x with EdDSA signatures for direct distribution, `SMAppService` for login items and helpers (which replaced `SMLoginItemSetEnabled` and `SMJobBless`), and TCC's quirks where permissions silently invalidate when a signing identity changes.

This command surfaces all of that explicitly.

## Preparation

Run these in parallel before reading code.

1. **Deployment target.** Inspect `Package.swift`, `*.xcodeproj`, and `.xcconfig` files for the macOS minimum. Some APIs (`MenuBarExtra`, `Window`, `UtilityWindow`, `SMAppService`) have specific macOS-version gates.
2. **Distribution channel.** Look for an `*.entitlements` file with `com.apple.security.app-sandbox = true` (Mac App Store path) or `Sparkle` in `Package.resolved` / `Podfile.lock` (Developer ID direct-distribution path). Both shapes are common; the rules differ between them.
3. **Main menu commands.** `rg -l '\.commands\s*\{' .` — an empty or near-empty `.commands { }` is the single biggest "this is an iPad port" signal.
4. **Menu bar app.** `rg -l 'MenuBarExtra|NSStatusItem|NSStatusBar' .` — note which approach is in use.
5. **Document-based app.** `rg -l 'DocumentGroup|FileDocument|ReferenceFileDocument' .` — if the app deals in named files and the project does not use these, recommend the switch.
6. **AppKit interop.** `rg -l 'NSViewRepresentable|NSHostingView|NSHostingController|NSHostingScene' .` — hybrid Mac apps are normal; flag only when the bridging is reaching for SwiftUI gaps that have native equivalents now.
7. **Login items / helpers.** `rg -l 'SMAppService|SMLoginItemSetEnabled|SMJobBless' .` — the deprecated APIs need replacing.
8. **Notarization workflow.** `find . -name '*.sh' -o -name 'Makefile' -o -name 'Fastfile' | xargs grep -l 'notarytool\|altool' 2>/dev/null` — `altool` in any active build script is a blocking finding.

## Automated macOS sweep

```bash
# Deprecated login item APIs
LEGACY_LOGIN=$(rg -c 'SMLoginItemSetEnabled|SMJobBless' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
[ "$LEGACY_LOGIN" -gt 0 ] && echo "Legacy login item APIs: $LEGACY_LOGIN — replace with SMAppService"

# Notarization workflow
echo ""
ALTOOL=$(grep -rl 'altool' --include='Fastfile' --include='*.sh' --include='Makefile' --include='*.yml' --include='*.yaml' . 2>/dev/null | head -3)
[ -n "$ALTOOL" ] && echo "altool references found in build scripts: $ALTOOL — Apple stopped accepting altool uploads on November 1, 2023; use notarytool"

# Empty commands block
echo ""
COMMANDS=$(rg -l '\.commands\s*\{' . --type swift 2>/dev/null)
if [ -n "$COMMANDS" ]; then
    echo ".commands {} blocks: present"
else
    echo ".commands {} blocks: NONE — main menu is empty; flag unless this is a single-window utility"
fi

# Touch Bar code (deprecated hardware)
TOUCHBAR=$(rg -c 'NSTouchBar|touchBar(' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
[ "$TOUCHBAR" -gt 0 ] && echo "Touch Bar references: $TOUCHBAR — hardware is gone; remove from new code"

# Direct Csqlite3 in a Mac app
CSQLITE=$(rg -c 'import SQLite3|sqlite3_open|sqlite3_exec' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
[ "$CSQLITE" -gt 0 ] && echo "Direct sqlite3 calls: $CSQLITE — consider SwiftData / Core Data / GRDB"

# Sandbox vs Hardened Runtime
echo ""
SANDBOX=$(grep -l 'com.apple.security.app-sandbox' --include='*.entitlements' -r . 2>/dev/null)
[ -n "$SANDBOX" ] && echo "App Sandbox enabled (Mac App Store path)" || echo "App Sandbox: not detected (Developer ID path likely; verify Hardened Runtime)"

HARDENED=$(grep -l 'com.apple.security.cs.' --include='*.entitlements' -r . 2>/dev/null)
[ -n "$HARDENED" ] && echo "Hardened Runtime entitlements present"

# Sparkle for direct distribution
SPARKLE=$(rg -l 'import Sparkle|SPUStandardUpdater' . --type swift 2>/dev/null)
[ -n "$SPARKLE" ] && echo "Sparkle: detected (verify EdDSA, not legacy DSA signatures)"

# Drag-and-drop
TRANSFERABLE=$(rg -c 'Transferable|\.draggable\(|\.dropDestination\(' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "Transferable / .draggable / .dropDestination hits: $TRANSFERABLE"

# Document-based app
DOC=$(rg -c 'DocumentGroup|FileDocument|ReferenceFileDocument' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "DocumentGroup / FileDocument hits: $DOC"
```

## What to review (in order)

### 1. Does the app feel like a Mac app?

The opening question of every Mac code review. Read `App.swift`, the root `WindowGroup`, and the `.commands { }` block.

A Mac app with an empty `.commands { }` and no `.keyboardShortcut` calls on any of its primary actions reads as an iPad port. Scale the requirement to the app: a clipboard utility does not need a full Edit menu, but it needs keyboard shortcuts on its primary actions. A document app without Cut, Copy, Paste, Select All, and Save shortcuts is broken on the Mac. A content app without a custom menu for its main verbs is missing the convention.

Check the .commands block for `CommandGroup` placements: `replacing(_:)` for replacing system commands, `before(_:)` and `after(_:)` for adding alongside.

Check `.focusedSceneValue` and `@FocusedValue` for menu commands that target the front window's selection rather than a global state. If the menu's "Delete" command does the wrong thing when the user has two windows open, this is missing.

### 2. Window and scene shape

`WindowGroup` is the default for SwiftUI Mac apps. `Window` (macOS 13+) for a single-instance window. `UtilityWindow` (macOS 15+) for an inspector-style auxiliary. `Settings` is the dedicated scene for Preferences / Settings. `MenuBarExtra` (macOS 13+) for menu-bar apps.

`.windowResizability`, `.windowStyle(.hiddenTitleBar)`, and `.windowToolbarStyle` are the modifiers that control window chrome. If a Mac app has a fixed window size, the window-resizability modifier should be on the scene.

### 3. MenuBarExtra and its missing state API

`MenuBarExtra` gives a SwiftUI menu bar app in a few lines, but Apple has not shipped a first-party API for opening or closing its popover programmatically. Real-world menu bar apps either use a third-party package that exposes this (`MenuBarExtraAccess` or `FluidMenuBarExtra` — both unaffiliated with Apple) or drop to raw `NSStatusBar.system.statusItem(withLength:)`.

`Ice` (around 28k stars on GitHub) and `MeetingBar` (around 5.2k) both use raw `NSStatusItem` despite shipping SwiftUI elsewhere. If a Mac menu bar app is constrained by `MenuBarExtra`'s gaps, dropping to AppKit for the status item is not an anti-pattern.

### 4. Document-based apps

If the app deals in named user files, use `DocumentGroup` with `FileDocument` (for value-type documents) or `ReferenceFileDocument` (for reference-semantic documents). This gives you Open, Save, Recents, Versions, autosave, and iCloud Drive sync without custom code. Skip the boilerplate.

`UTType` declares the file format. The exported and imported UTIs must appear in `Info.plist`.

### 5. Drag-and-drop via Transferable

The Mac superpower most ports neglect. With `Transferable` plus `.draggable` and `.dropDestination`, your app accepts drops from Finder, sends content to Mail or Notes, and exchanges items with other apps using standard payloads. If a Mac app reads or writes files but does not support drag-drop to and from the Finder, this is an `important` finding.

### 6. Sandbox and Hardened Runtime

The App Sandbox is mandatory for the Mac App Store path. For Developer ID direct distribution, the sandbox is optional but recommended; Hardened Runtime is mandatory for notarization, which is itself mandatory for direct distribution since macOS 10.15.

Hardened Runtime relaxations follow a hierarchy: prefer `com.apple.security.cs.allow-jit` (the narrowest) over `allow-unsigned-executable-memory` (broader) over `disable-executable-page-protection` (broadest and weakest). The narrowest entitlement that compiles is the right answer; App Review pushes back on the broader ones.

Required Info.plist usage strings (`NSCameraUsageDescription`, `NSMicrophoneUsageDescription`, etc.) are mandatory on Mac the same as on iOS. The Mac will crash the app on first protected-resource access if the usage string is missing — there is no graceful fallback to a denied state.

### 7. Notarization with notarytool

`xcrun notarytool submit ... --keychain-profile <name> --wait` is the modern submission flow. `xcrun stapler staple <app>` attaches the notarization ticket. `spctl --assess --type execute --verbose=4 <app>` verifies before shipping. `altool` is dead and any reference to it in a build script is a `blocking` finding.

For DMG distribution, notarize the DMG and staple the DMG.

### 8. SMAppService for login items, agents, daemons

`SMAppService` (macOS 13+) replaces `SMLoginItemSetEnabled` and `SMJobBless`. The replacement is mandatory for new code and any sighting of the deprecated APIs is an `important` finding for old code (`blocking` if the app uses helper apps or privileged daemons).

The Background Task Management user-visible toggle is what shows up in System Settings, so users can disable login items the app installed. Treat this as expected behavior, not a bug.

### 9. Sparkle 2.x for direct distribution

If the app does not go through the Mac App Store, Sparkle is the de-facto auto-update framework. Sparkle 2.x supports sandboxed apps and uses EdDSA signatures. Sparkle 1.x with DSA is legacy — verify any Sparkle integration is on 2.x with EdDSA. The appcast XML must be signed; the public key must be in the app's Info.plist.

### 10. AppKit interop without guilt

`NSViewRepresentable`, `NSHostingView`, and `NSHostingController` are everyday tools in shipping Mac SwiftUI apps. Drop to AppKit when SwiftUI has a real gap: `NSTextView` for rich text or code editing, `NSDocument` semantics SwiftUI does not match, `NSXPC` privilege separation, low-level window and accessibility APIs, mpv-driven media playback. Bridging is normal.

The reverse caveat: do not flag a pure-AppKit Mac app as "should be SwiftUI." Utility, menu-bar, and media apps with substantial install bases ship pure AppKit and continue to thrive (IINA, Rectangle, Stats, Ice, MeetingBar combined have hundreds of thousands of stars on GitHub). The "rewrite to SwiftUI" instinct is sometimes wrong.

### 11. TCC quirks worth knowing

TCC permissions (Screen Recording, Accessibility, Full Disk Access, Camera, Microphone, Contacts, Automation) bind to the bundle ID and the signing identity. Changing signing identity between releases drops all user grants — users have to re-authorize. This catches teams off guard during the first signing-identity rotation.

Sequoia and Tahoe re-prompt for Screen Recording on a monthly cadence even when the user has previously granted access. There is no fix for this on the app side; it is a system-level recurring confirmation.

Local Network access on macOS is a NetworkExtension packet filter, not TCC. The prompt looks different and the recovery path is different.

### 12. Privacy Manifest on macOS — formally exempt, ship one anyway

The Privacy Manifest requirement (`PrivacyInfo.xcprivacy`) is iOS-only as a hard submission gate. macOS submissions do not fail without it. For shared SwiftPM packages that get embedded into iOS targets, you still want one — and shipping a manifest on a Mac app costs nothing and prepares for any future requirement.

### 13. Touch Bar is deprecated

The hardware is gone. Touch Bar code in a Mac app is `blocking` if it adds maintenance cost; otherwise `important`. The Touch Bar SDK still exists, but new code should not target it.

## For each finding

Same template as `/swift-critique`: file and line, severity, what, why it matters, fix. The "why it matters" for Mac findings is often specific to the platform: this code crashes the app on first protected-resource access; this code makes the app look like an iPad port to a Mac power user; this code will fail notarization; this code makes the app fall out of the Mac App Store review queue.

## "What I'm not flagging" pass

Common cases for Mac:

- A Mac app that uses pure AppKit for its core surface, when that surface is a text editor, video player, or system monitor. Correct call for the shape of the app — do not recommend a SwiftUI rewrite.
- A menu bar app that drops to raw `NSStatusItem` rather than using `MenuBarExtra`. Working around a real platform gap — note as praise, not a finding.
- A direct-distribution Mac app that does not have an Info.plist Privacy Manifest. Formally exempt; not a finding.
- A Mac app that opted out of Liquid Glass with `UIDesignRequiresCompatibility = true` (relevant for Catalyst targets). Many pro apps shipped this way at iOS 26 launch.
- A Mac app that bridges to `WKWebView` rather than the new `WebView` SwiftUI struct because the deployment target predates the SwiftUI version. Pragmatic.
- A `CocoaPods`-based Mac app. CocoaPods is not dead; if the project is actively maintained, do not flag for a migration the team has not asked for.

## Generate report

Same shape as `/swift-critique`:

1. Quick stats from the automated sweep, plus a one-line "does this feel like a Mac app" verdict.
2. What's working — two or three Mac-relevant things done well.
3. Priority issues — `blocking` and `important`.
4. Minor observations — `nit` and `suggestion`.
5. What I'm not flagging.
6. Pattern-recognition pass — is the app matching the platform conventions for its category (productivity utility, document-based, menu-bar, media player, system monitor)?
7. Questions to consider — provocative questions specific to Mac apps.
8. Suggested follow-up — `/swift-critique` for cross-cutting concerns, `/swift-architect` if patterns reveal architecture drift, `/swift-teach <concept>` for explaining a specific Mac API.

## How to do this well

Be specific about the Mac conventions. "Line 18 of `MyApp.swift` has an empty `.commands { }` block and the only keyboard shortcut in the app is on the Help menu. Mac users will not find the primary actions" beats "your menu is incomplete."

When recommending a switch from a deprecated API, name the version gate: "`SMAppService` requires macOS 13. Your project target is macOS 14, so the migration is safe." If the deployment target is too low, say so and recommend either bumping it or keeping the legacy API behind an `#available` check.

Honor the platform gaps. `MenuBarExtra`'s missing state API is a real gap, not user error. The correct review move for a `MenuBarExtra`-based app fighting that gap is to recommend `MenuBarExtraAccess` / `FluidMenuBarExtra` or a raw `NSStatusItem` fallback — not to suggest the developer is doing it wrong.

Say "I don't know" when you do not know. If the entitlements file is not in the parts of the repo you can read, the distribution channel is unclear, or you cannot tell whether the app's signing identity changed between releases — say so and point the developer at the specific check that would resolve the question.
