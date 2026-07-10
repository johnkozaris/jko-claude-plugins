---
description: iOS-focused SwiftUI / Swift code review. Concentrates on iOS-specific concerns — Privacy Manifest, App Tracking Transparency, App Intents, Widgets and Live Activities, scoped permission APIs, App Attest, push notifications, Foundation Models, StoreKit 2 — alongside the cross-cutting state, navigation, and concurrency rules.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
argument-hint: "<target>"
---

# SwiftUI iOS Critique

Conduct a focused review of an iOS app target. The `swiftui-expert` skill covers the general SwiftUI rubric; this command narrows the lens to iOS-specific platform concerns and the modern APIs that often get missed.

**First**: use the `swiftui-expert` skill for the review rubric, severity tiers, and reference files. The reference of record for this command is `references/ios-platform.md`.

## Why this command exists

iOS in 2026 sits on a thick stack of platform APIs that a generic SwiftUI review misses. Privacy Manifest enforcement is submission-blocking when the app or bundled SDK uses APIs covered by Apple's manifest requirements. App Intents is the unification surface for Siri, Shortcuts, Spotlight, the Action Button, and Visual Intelligence — and most apps still implement only the smallest subset. Scoped permission APIs (`PhotosPicker`, `LocationButton`, `ContactAccessButton`, `EKEventEditViewController`, `DataScannerViewController`) let an app grant single-item access without a system prompt or an Info.plist usage string, and many apps still reach for the full-permission version by reflex. Live Activities, interactive widgets, BGContinuedProcessingTask, App Attest, and Foundation Models all reward intentional adoption.

This command surfaces those concerns explicitly so an iOS code review does not stop at "your SwiftUI is fine."

## Preparation

Run these in parallel before reading code.

1. **Deployment target.** Inspect `Package.swift`, `*.xcodeproj/project.pbxproj`, and any `.xcconfig` files. Identify the iOS minimum. Many of the rules below are iOS-version-gated.
2. **Privacy Manifest applicability and coverage.** Inventory listed Required Reason APIs in app/dependency code and SDKs on Apple's required-SDK list, then inspect `PrivacyInfo.xcprivacy`. Absence or incomplete reasons are blocking only when those requirements apply.
3. **Tracking posture.** `rg -l 'ATTrackingManager|NSUserTrackingUsageDescription' .` — if the app calls ATT, verify there is no fingerprinting fallback elsewhere in the code.
4. **App Intents.** `rg -l 'AppIntent|@Parameter|AppShortcutsProvider' .` — if zero hits in a consumer app, this is usually a Path B (leaving user value on the table) finding.
5. **Widget / Live Activity targets.** `find . -path '*.xcodeproj/project.pbxproj' -exec grep -l 'WidgetKit\|ActivityKit' {} +` — note whether the project ships these surfaces.
6. **Permission usage strings.** `find . -name 'Info.plist' -exec grep -l 'NSPhotoLibraryUsageDescription\|NSContactsUsageDescription\|NSLocationWhenInUseUsageDescription\|NSCameraUsageDescription' {} +` — paired with the scoped-API check below.
7. **Scoped API usage.** `rg -l 'PhotosPicker|LocationButton|ContactAccessButton|EKEventEditViewController|DataScannerViewController' .` — when present alongside the matching usage string above, the usage string is often unneeded.

## Automated iOS sweep

```bash
# Privacy Manifest
find . -name 'PrivacyInfo.xcprivacy' -print -quit | grep -q . && \
    echo "PrivacyInfo.xcprivacy: present" || \
    echo "PrivacyInfo.xcprivacy: absent — inventory Required Reason APIs and required SDK manifests"

# Legacy image picker
echo ""
LEGACY_PICKER=$(rg -c 'UIImagePickerController' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
[ "$LEGACY_PICKER" -gt 0 ] && echo "UIImagePickerController hits: $LEGACY_PICKER — recommend PhotosPicker / PHPickerViewController"

# StoreKit 1 legacy
SK1=$(rg -c 'SKProductsRequest|SKPaymentQueue|SKPaymentTransaction' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
[ "$SK1" -gt 0 ] && echo "StoreKit 1 hits: $SK1 — recommend StoreKit 2 (async / await + JWS)"

# Custom URL scheme enumeration abuse
URL_SCHEMES=$(rg -c 'LSApplicationQueriesSchemes' . 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "LSApplicationQueriesSchemes references: $URL_SCHEMES"

# Tracking fallback signals
FINGERPRINT=$(rg -c 'identifierForVendor|IDFA|IDFV|advertisingIdentifier' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
ATT=$(rg -c 'ATTrackingManager' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "ATTrackingManager calls: $ATT / IDFA/IDFV refs: $FINGERPRINT (if ATT is denied and IDFV refs remain, review for fingerprinting)"

# App Intents
INTENTS=$(rg -c 'AppIntent|@Parameter|AppShortcutsProvider' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "App Intents hits: $INTENTS"

# Live Activities / interactive widgets
ACTIVITY=$(rg -c 'ActivityKit|ActivityAttributes' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "ActivityKit hits: $ACTIVITY"
WIDGET=$(rg -c 'WidgetKit|WidgetBundle' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "WidgetKit hits: $WIDGET"

# Push notification interruption levels
PUSH=$(rg -c '\.timeSensitive|\.critical|provisional' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "Push interruption-level mentions: $PUSH"

# Sign in with Apple alongside social login
SOCIAL=$(rg -c 'GoogleSignIn|FacebookLogin|TwitterKit|GIDSignIn' . 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
SIWA=$(rg -c 'AuthenticationServices|SignInWithAppleButton|ASAuthorizationAppleIDProvider' . --type swift 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
[ "$SOCIAL" -gt 0 ] && [ "$SIWA" -eq 0 ] && echo "Third-party social login present — review Guideline 4.8 applicability and exceptions"
```

## What to review (in order)

Walk these categories. Load `references/ios-platform.md` for the deep treatment; the other references for cross-cutting concerns.

### 1. Privacy Manifest and tracking

Since May 1, 2024, uploads that use Apple's listed Required Reason APIs must declare approved reasons in `PrivacyInfo.xcprivacy`. SDKs on Apple's required-SDK list need their own signed manifest with stable signing identity across versions. Inventory app and dependency usage first; do not report an absent manifest as a universal failure. The manifest can also declare collected-data categories and tracking domains, and App Store Connect declarations must match actual behavior.

If the app calls `ATTrackingManager.requestTrackingAuthorization()`, the rest of the codebase must not implement a fingerprinting fallback (IDFV plus IP plus locale plus carrier plus brightness plus storage, reconstituted into a stable identifier). Apple's human reviewers reject this under App Store Review Guidelines 5.1.1 / 5.1.2. If the app does not actually track users across apps and websites, do not call ATT at all.

### 2. Permissions: prefer scoped APIs

For each permission the app uses, check whether a scoped API would have skipped the prompt:

- `PhotosPicker` (SwiftUI, iOS 16+) and `PHPickerViewController` (UIKit) for photo library access — no Info.plist key, no prompt.
- `LocationButton` (iOS 15+) for one-tap location grant — no key, no prompt.
- `ContactAccessButton` (iOS 18+) for a single contact — no key, no prompt.
- `EKEventEditViewController` for write-only Calendar (iOS 17+) — no prompt for save.
- `DataScannerViewController` for camera-based scanning (text, barcodes) — handles the camera prompt itself.

When the app uses the full-permission API instead of the scoped one, recommend the switch and explain that the prompt and the Info.plist usage string become unnecessary.

### 3. Sign in with Apple and Guideline 4.8

Guideline 4.8 generally requires an equivalent privacy-preserving login option when third-party/social login authenticates the app's primary account. Before flagging, check the published exceptions: exclusive first-party account systems, enterprise/education/business apps requiring existing organization accounts, government/industry identity systems, and clients for a specific third-party service.

### 4. App Intents — the unification surface

A single `AppIntent` exposes an action to Siri, Shortcuts, Spotlight, Focus filters, Action Button, Apple Pencil Pro squeeze, and Visual Intelligence (iOS 26). Consumer apps that have no `AppIntent` are leaving a meaningful adoption surface unexposed.

Look for the verbs the app already supports (Create X, Open Y, Save Z) and recommend implementing them as `AppIntent`. Apps with substantial functionality that hide their actions behind a private UI are losing to apps that surface those actions everywhere.

WWDC25 Session 244 is the source for the unification framing.

### 5. Widgets, interactive widgets, Live Activities

Interactive widgets have been available since iOS 17, with `Button` and `Toggle` bound to an `AppIntent`. Read-only billboard widgets are legacy.

If the app's primary user activity is daily (a tracker, a reader, a player, a productivity tool), ask whether a widget on the Home Screen, Lock Screen, or StandBy display would surface real value. Most daily-use apps ship one or both.

For temporal-event apps (delivery, ride, fitness session, live score), Live Activities (`ActivityKit`) bring the action into the Dynamic Island and the Lock Screen for up to eight active hours plus four stale.

### 6. iOS 26 additions worth surfacing

`BGContinuedProcessingTask` brings system-presented progress UI to long-running background work (uploads, exports, rendering). Productivity apps that today fail silently when the user switches away should adopt this.

`PermissionKit` (iOS 26) handles parental approval for child accounts in apps with communication features that minors might use.

`DeclaredAgeRange` provides coarse age binning (`requestAgeRange` since iOS 26.0; `isEligibleForAgeFeatures` since iOS 26.2) without ever seeing a birthdate.

Foundation Models (iOS 26, three-billion-parameter on-device LLM via the `FoundationModels` framework with the `@Generable` macro) is for narrow structured-output tasks — classification, summarization, structured extraction. It is not a GPT-class chatbot and one developer's testing showed it underperforms similarly-sized open-source models. Use it where it fits; do not pitch it as the AI feature.

### 7. App Attest and DeviceCheck — under-adopted

App Attest gives a Secure-Enclave-backed cryptographic proof that a request to your server came from the unmodified binary you actually shipped. It is a small amount of code, has no runtime cost on users, and shuts down trial abuse, signup spam, and reverse-engineered API clients. Many apps with a server backend skip this and end up reinventing fraud detection later. If the app has any of those problems, recommend App Attest before any of the workarounds.

### 8. StoreKit 2

`StoreKit 2` (async / await with JWS receipt validation) is the only StoreKit to ship in new code. StoreKit 1 is legacy. Look for `SKProductsRequest`, `SKPaymentQueue`, and `SKPaymentTransaction` in new code as red flags.

### 9. Permission UX

Across every permission, three rules:

- Never ask on first launch. Prime the user by showing what the permission unlocks.
- Ask one permission per moment. Never stack three prompts at the start of onboarding.
- For a denied state, provide a clear recovery path. `UIApplication.openSettingsURLString` deep-links to the app's Settings page; `openNotificationSettingsURLString` (iOS 16+) goes to the notification screen.

### 10. Cross-cutting iOS-relevant rules

The general SwiftUI rules from `references/anti-patterns.md` still apply: `@AppStorage` directly inside `@Observable` is the silent-no-updates trap; SwiftData `@Model` types need `VersionedSchema` from v1; tokens belong in Keychain, not `UserDefaults`; `os.Logger` rather than `print()` for production code paths; icon-only buttons need accessibility labels.

## For each finding

Same template as `/swift-critique`:

1. File and line.
2. Severity (`blocking`, `important`, `nit`, `suggestion`, `praise`).
3. What — one sentence.
4. Why it matters — concrete production cost (App Store rejection, user-visible failure, security exposure, missing reach into Siri / Shortcuts / Spotlight).
5. Fix — short before / after when non-obvious.

## "What I'm not flagging" pass

Before the summary, document the things you considered and left alone. Common cases for iOS:

- An app that calls `requestTrackingAuthorization()` and has a clear analytics-only fallback that does not fingerprint. Working as intended.
- An app on a `tvOS` / `iPadOS` / `visionOS` shared codebase where some iOS-only APIs are guarded with `#if canImport(UIKit)`. Pragmatic.
- An app targeting iOS 15 that cannot use `LocationButton` because it shipped before. Not a finding.
- Apple Intelligence not adopted in an app that has no narrow-output use case. Adopting Foundation Models because it is shiny is worse than skipping it.
- Live Activities not adopted in an app where nothing temporal happens. Not a finding.

## Generate report

Same shape as `/swift-critique`:

1. Quick stats from the automated sweep.
2. What's working — two or three iOS-relevant things done well.
3. Priority issues — `blocking` and `important` findings.
4. Minor observations — `nit` and `suggestion`.
5. What I'm not flagging, and why.
6. Pattern-recognition pass — does the app match the platform conventions for its category (productivity, utility, communication, media, social)?
7. Questions to consider — provocative questions that might unlock better iOS integration.
8. Suggested follow-up — `/swift-critique` for the cross-cutting concerns, `/swift-architect` if iOS-specific patterns reveal architecture drift, `/swift-teach <concept>` for "explain App Intents to me" or similar.

## How to do this well

Be specific. "This target uses the file-timestamp Required Reason API but has no approved reason in `PrivacyInfo.xcprivacy`; the upload can fail" is actionable. "Every app needs a manifest" is not.

When recommending a scoped API instead of a permission prompt, name both: "Replace the `UIImagePickerController` flow with `PhotosPicker` (SwiftUI, iOS 16+); you can then remove `NSPhotoLibraryUsageDescription` from Info.plist."

For Foundation Models and other shiny iOS 26 APIs, name the trade-off honestly. "Foundation Models is on-device, free, and Apple-Intelligence-backed; it is also a small model with a short context window, and several developers report it underperforms similarly-sized open-source models on broad tasks. Use it for structured output, classification, and short-form summarization; do not use it for chat or anything requiring depth."

Say "I don't know" when you do not know. If the deployment target is unclear, the entitlement file is gated behind something you cannot read, or the app's purpose is ambiguous enough that an iOS-26 recommendation could go either way — say so, and point the developer at the specific check that would resolve the uncertainty.
