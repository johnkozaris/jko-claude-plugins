# iOS Platform Reference

Target: Swift 6.3 / iOS 26 / Xcode 26. Last verified 2026-05-17.

## What this file covers

iOS-specific platform capabilities + permissions + privacy. Everything an iPhone/iPad app needs to integrate with system surfaces (Siri, Shortcuts, widgets, Live Activities, Visual Intelligence), every consent surface (ATT, permissions, Privacy Manifest), and every Info.plist key required to ship in 2026.

macOS-specific guidance lives in `macos-platform.md`. State and persistence ergonomics (including `@AppStorage` traps) live in `state-and-observation.md`.

---

## App Intents — the unification API

One framework drives every "do X in app Y" entry point on iOS in 2026:

- **Siri** — voice invocation.
- **Shortcuts** — user-built automation.
- **Spotlight Top Hits** — search-typed verbs.
- **Focus Filters** — "Work Focus: show only work account."
- **Widgets** — interactive Button/Toggle bound to an intent.
- **Control Center / Lock Screen Controls / Action Button** — single-tap power-user actions.
- **Apple Pencil Pro squeeze** — same controls, on iPad.
- **Visual Intelligence (iOS 26)** — Camera Control routes on-screen entities to your app.

Reference session: **WWDC25 Session 244** ("Integrating Actions with Siri and Apple Intelligence"). If your app has no App Intents in 2026, it is invisible to the iPhone's assistive shell — that includes Apple Intelligence semantic search.

### Basic AppIntent

```swift
import AppIntents

struct LogWaterIntent: AppIntent {
    static var title: LocalizedStringResource = "Log a Glass of Water"
    static var description = IntentDescription(
        "Logs one 8 oz glass of water to today's hydration total.",
        categoryName: "Hydration"
    )

    @Parameter(title: "Amount (oz)", default: 8)
    var amount: Int

    static var openAppWhenRun: Bool = false

    func perform() async throws -> some IntentResult & ProvidesDialog {
        try await HydrationStore.shared.log(ounces: amount)
        return .result(dialog: "Logged \(amount) ounces.")
    }
}
```

### Zero-config Shortcuts via AppShortcutsProvider

```swift
struct HydrationShortcuts: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: LogWaterIntent(),
            phrases: [
                "Log water in \(.applicationName)",
                "Drink a glass in \(.applicationName)"
            ],
            shortTitle: "Log Water",
            systemImageName: "drop.fill"
        )
    }
}
```

This auto-registers the intent into the Shortcuts app + Spotlight with no user setup. Ship up to 10 of these — pick the 3–5 highest-value verbs.

### Visual Intelligence integration (iOS 26)

`IntentValueQuery` plus `SemanticContentDescriptor` exposes your catalog to Visual Intelligence's on-screen entity routing:

```swift
struct PlantLookupQuery: IntentValueQuery {
    func values(for input: SemanticContentDescriptor) async throws -> [Plant] {
        guard let imageData = input.pixelBuffer else { return [] }
        return try await PlantCatalog.shared.identify(imageData: imageData)
    }
}

struct OpenPlantIntent: OpenIntent {
    static var title: LocalizedStringResource = "Open Plant"
    @Parameter(title: "Plant") var target: Plant
    func perform() async throws -> some IntentResult {
        AppRouter.shared.navigate(to: .plant(target.id))
        return .result()
    }
}
```

iOS 26 also adds `UndoableIntent` (system-level undo) and `@DeferredProperty` / `@ComputedProperty` for async-resolved parameters.

---

## Widgets

WidgetKit framework. Same SwiftUI views render across four surfaces:

- **Home Screen** (iOS 14+)
- **Lock Screen** (iOS 16+)
- **StandBy** (iOS 17+, charging in landscape)
- **watchOS Smart Stack** (iOS 17+ paired Watch)

Smart Stack and StandBy mean the system *picks* which widget to surface at a moment. If you don't ship one, your competitor's is on the surface instead.

### Interactive widgets (iOS 17+)

`Button(intent:)` and `Toggle(isOn:intent:)` in widget bodies execute an AppIntent without launching the app. Same mechanism powers buttons inside Live Activities.

```swift
struct HydrationWidget: Widget {
    let kind: String = "HydrationWidget"

    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: HydrationTimelineProvider()) { entry in
            VStack(alignment: .leading) {
                Text("\(entry.totalOunces) oz today")
                    .font(.headline)
                Button(intent: LogWaterIntent(amount: 8)) {
                    Label("Log Glass", systemImage: "plus.circle.fill")
                }
                .buttonStyle(.borderedProminent)
            }
            .containerBackground(.fill.tertiary, for: .widget)
        }
        .configurationDisplayName("Hydration")
        .description("Track your water intake.")
        .supportedFamilies([.systemSmall, .systemMedium, .accessoryRectangular])
    }
}

struct HydrationTimelineProvider: TimelineProvider {
    func placeholder(in context: Context) -> HydrationEntry {
        HydrationEntry(date: .now, totalOunces: 0)
    }
    func getSnapshot(in context: Context, completion: @escaping (HydrationEntry) -> Void) {
        completion(HydrationEntry(date: .now, totalOunces: 32))
    }
    func getTimeline(in context: Context, completion: @escaping (Timeline<HydrationEntry>) -> Void) {
        let entry = HydrationEntry(date: .now, totalOunces: HydrationStore.shared.todayTotal)
        completion(Timeline(entries: [entry], policy: .after(.now.addingTimeInterval(900))))
    }
}
```

### ControlWidget (iOS 18+)

Control Center / Lock Screen Controls / Action Button bindings — declared once:

```swift
struct LogWaterControl: ControlWidget {
    var body: some ControlWidgetConfiguration {
        StaticControlConfiguration(kind: "LogWaterControl") {
            ControlWidgetButton(action: LogWaterIntent(amount: 8)) {
                Label("Log Water", systemImage: "drop.fill")
            }
        }
        .displayName("Log Water")
        .description("Add a glass to today's hydration.")
    }
}
```

A read-only widget in 2026 is a wasted impression. Bind every actionable widget to an AppIntent.

---

## Live Activities + Dynamic Island

ActivityKit framework. Persistent Lock Screen + Dynamic Island UI for in-progress events.

- **Lifecycle:** up to 8 hours active + 4 hours stale. Stale state is critical — many apps neglect it, so the "x minutes ago" indicator goes wrong.
- **Updates:** local app updates while foreground; APNs `liveactivity` push for background (and push-to-start since iOS 17.2).
- **Three layouts required:** Lock Screen, Dynamic Island compact, Dynamic Island expanded.
- **Use when:** anything with a start, an end, and "where am I now?" — delivery, rideshare, sports, flights, timers, workouts, file uploads.

```swift
import ActivityKit
import WidgetKit

struct DeliveryAttributes: ActivityAttributes {
    public struct ContentState: Codable, Hashable {
        var status: String         // "Preparing", "Out for delivery", "Arriving"
        var etaMinutes: Int
        var courierName: String?
    }
    var orderID: String
    var restaurantName: String
}

struct DeliveryLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: DeliveryAttributes.self) { context in
            // Lock Screen
            VStack(alignment: .leading) {
                Text(context.attributes.restaurantName).font(.headline)
                Text(context.state.status).font(.subheadline)
                Text("ETA \(context.state.etaMinutes) min")
                    .foregroundStyle(.secondary)
            }
            .padding()
            .activityBackgroundTint(.black.opacity(0.6))
        } dynamicIsland: { context in
            DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    Image(systemName: "bag.fill")
                }
                DynamicIslandExpandedRegion(.trailing) {
                    Text("\(context.state.etaMinutes)m")
                }
                DynamicIslandExpandedRegion(.bottom) {
                    Text(context.state.status)
                }
            } compactLeading: {
                Image(systemName: "bag.fill")
            } compactTrailing: {
                Text("\(context.state.etaMinutes)m")
            } minimal: {
                Image(systemName: "bag.fill")
            }
        }
    }
}

// Start an activity from the app
func startDeliveryActivity(orderID: String, restaurant: String) {
    let attrs = DeliveryAttributes(orderID: orderID, restaurantName: restaurant)
    let initialState = DeliveryAttributes.ContentState(
        status: "Preparing", etaMinutes: 35, courierName: nil
    )
    do {
        _ = try Activity.request(
            attributes: attrs,
            content: .init(state: initialState, staleDate: .now.addingTimeInterval(60 * 60)),
            pushType: .token
        )
    } catch {
        Logger.activities.error("Live Activity start failed: \(error)")
    }
}
```

Always set a `staleDate` — the system uses it to dim/mark the UI as out-of-date when fresh data hasn't arrived.

---

## TipKit

Apple's blessed tip/onboarding framework. Replaces every homemade tooltip/coach-mark hack — custom tooltips look amateurish in 2026.

```swift
import TipKit

struct LogWaterTip: Tip {
    var title: Text { Text("Track every glass") }
    var message: Text? { Text("Tap the drop to add a glass to today's total.") }
    var image: Image? { Image(systemName: "drop.fill") }

    @Parameter
    static var hasOpenedApp: Bool = false

    var rules: [Rule] {
        #Rule(Self.$hasOpenedApp) { $0 == true }
    }
}

@main
struct HydrationApp: App {
    init() {
        try? Tips.configure([.displayFrequency(.daily), .datastoreLocation(.applicationDefault)])
    }
    var body: some Scene { WindowGroup { ContentView() } }
}

struct ContentView: View {
    private let logTip = LogWaterTip()
    var body: some View {
        Button("Log") { /* ... */ }
            .popoverTip(logTip, arrowEdge: .top)
    }
}
```

iOS 18+ adds `TipGroup` for orchestrated multi-tip flows across screens.

---

## App Clips

< 50 MB instant slices of your app launched from NFC, App Clip Codes, QR, Safari Smart Banners, Maps, or peer apps. iOS 17+ default App Clip Links let any other app invoke yours.

- **Use when:** physical or in-the-moment flows — order at counter, rent scooter, register at event, pay meter, sample paid app's core flow.
- **Requires:** Universal Links + Associated Domains (the same plumbing).

```swift
// In the App Clip target's scene delegate / App body
struct OrderClip: App {
    var body: some Scene {
        WindowGroup {
            OrderView()
                .onContinueUserActivity(NSUserActivityTypeBrowsingWeb) { activity in
                    guard let url = activity.webpageURL else { return }
                    AppClipRouter.shared.handle(url)
                }
        }
    }
}
```

---

## Universal Links / Associated Domains

HTTPS URLs that deep-link into your app via an `apple-app-site-association` (AASA) JSON file. The same plumbing also powers webcredentials (passkeys), App Clips, and Handoff.

Custom URL schemes alone are 2026-obsolete — they're blocked from Mail, Messages link previews, and most third-party apps.

### AASA file (served at `https://example.com/.well-known/apple-app-site-association` with `Content-Type: application/json`)

```json
{
  "applinks": {
    "details": [{
      "appIDs": ["TEAMID123.com.acme.hydration"],
      "components": [
        { "/": "/orders/*", "comment": "Deep link to specific orders" },
        { "/": "/share/*",  "comment": "Shared content" }
      ]
    }]
  },
  "webcredentials": { "apps": ["TEAMID123.com.acme.hydration"] },
  "appclips":      { "apps": ["TEAMID123.com.acme.hydration.Clip"] }
}
```

### Entitlement (`.entitlements` file)

```xml
<key>com.apple.developer.associated-domains</key>
<array>
    <string>applinks:example.com</string>
    <string>applinks:example.com?mode=developer</string>
    <string>webcredentials:example.com</string>
    <string>appclips:example.com</string>
</array>
```

### Routing in SwiftUI

```swift
WindowGroup {
    ContentView()
        .onOpenURL { url in router.handle(url) }
        .onContinueUserActivity(NSUserActivityTypeBrowsingWeb) { activity in
            if let url = activity.webpageURL { router.handle(url) }
        }
}
```

---

## Sign in with Apple

**Required by App Store** alongside any third-party social login (Google, Facebook, GitHub, Twitter, etc.) — Apple's January 2024 policy update. Framework: AuthenticationServices.

```swift
import AuthenticationServices

struct SignInScreen: View {
    @Environment(AuthStore.self) private var auth

    var body: some View {
        SignInWithAppleButton(
            onRequest: { request in
                request.requestedScopes = [.fullName, .email]
                request.nonce = auth.makeNonce()
            },
            onCompletion: { result in
                Task { await auth.handle(result) }
            }
        )
        .signInWithAppleButtonStyle(.black)
        .frame(height: 50)
    }
}
```

The email-relay flow is a real privacy win users notice. Free, no user account on your backend until you actually need one.

---

## Passkeys

Public-key credentials in iCloud Keychain. Phishing-immune, syncs across the user's Apple devices, and consistently delivers significantly higher sign-in success than passwords (Apple's published data and partner reports — concrete percentage claims vary by source, so generalize when quoting).

iOS 26 adds the **Account Creation API** (`ASAuthorizationAccountCreationProvider`) — one-tap signup that bypasses email/password forms entirely.

```swift
import AuthenticationServices

func signUpWithPasskey(username: String) async throws {
    let provider = ASAuthorizationPlatformPublicKeyCredentialProvider(
        relyingPartyIdentifier: "example.com"
    )
    let challenge = try await Backend.shared.registrationChallenge(for: username)
    let request = provider.createCredentialRegistrationRequest(
        challenge: challenge, name: username, userID: Data(username.utf8)
    )
    let controller = ASAuthorizationController(authorizationRequests: [request])
    controller.delegate = AuthDelegate.shared
    controller.performRequests()
}
```

Pair with traditional password as a fallback during the transition. The iOS 26 Account Creation API removes the single biggest funnel-leak in your onboarding.

---

## Privacy Manifest (`PrivacyInfo.xcprivacy`)

**MANDATORY since May 1, 2024 (ITMS-91053).** App Store Connect auto-rejects apps missing the manifest. Third-party SDKs since Feb 12, 2025 (ITMS-91061) — they must ship a signed manifest and remain code-signed by the same author across versions.

### Required Reason API categories

| Category constant | When it applies | Common reason codes |
|---|---|---|
| `NSPrivacyAccessedAPICategoryFileTimestamp` | `creationDate`, `modificationDate`, `fstat`, `getattrlist` | `DDA9.1` (show to user), `C617.1` (in-container metadata), `3B52.1` (user-granted via document picker), `0A2A.1` (sync/health) |
| `NSPrivacyAccessedAPICategoryUserDefaults` | `UserDefaults`, `NSUbiquitousKeyValueStore` | `CA92.1` (own app only), `1C8F.1` (App Group), `AC6B.1` (CloudKit), `C56D.1` (third-party SDK) |
| `NSPrivacyAccessedAPICategorySystemBootTime` | `mach_absolute_time`, boot-time delta | `35F9.1` (elapsed time on-device), `8FFB.1` (timers) |
| `NSPrivacyAccessedAPICategoryDiskSpace` | Free space query | `E174.1` (show to user), `85F4.1` (avoid running out), `B728.1` (read/write needs) |
| `NSPrivacyAccessedAPICategoryActiveKeyboards` | Active input modes | `3EC4.1` (custom keyboard extension), `54BD.1` (language UX) |

### Sample `PrivacyInfo.xcprivacy`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>NSPrivacyTracking</key>
    <true/>
    <key>NSPrivacyTrackingDomains</key>
    <array>
        <string>analytics.example.com</string>
        <string>ads.partner.com</string>
    </array>
    <key>NSPrivacyCollectedDataTypes</key>
    <array>
        <dict>
            <key>NSPrivacyCollectedDataType</key>
            <string>NSPrivacyCollectedDataTypeEmailAddress</string>
            <key>NSPrivacyCollectedDataTypeLinked</key>
            <true/>
            <key>NSPrivacyCollectedDataTypeTracking</key>
            <false/>
            <key>NSPrivacyCollectedDataTypePurposes</key>
            <array>
                <string>NSPrivacyCollectedDataTypePurposeAppFunctionality</string>
                <string>NSPrivacyCollectedDataTypePurposeAccountManagement</string>
            </array>
        </dict>
        <dict>
            <key>NSPrivacyCollectedDataType</key>
            <string>NSPrivacyCollectedDataTypeDeviceID</string>
            <key>NSPrivacyCollectedDataTypeLinked</key>
            <true/>
            <key>NSPrivacyCollectedDataTypeTracking</key>
            <true/>
            <key>NSPrivacyCollectedDataTypePurposes</key>
            <array>
                <string>NSPrivacyCollectedDataTypePurposeAnalytics</string>
            </array>
        </dict>
    </array>
    <key>NSPrivacyAccessedAPITypes</key>
    <array>
        <dict>
            <key>NSPrivacyAccessedAPIType</key>
            <string>NSPrivacyAccessedAPICategoryUserDefaults</string>
            <key>NSPrivacyAccessedAPITypeReasons</key>
            <array>
                <string>CA92.1</string>
            </array>
        </dict>
        <dict>
            <key>NSPrivacyAccessedAPIType</key>
            <string>NSPrivacyAccessedAPICategoryFileTimestamp</string>
            <key>NSPrivacyAccessedAPITypeReasons</key>
            <array>
                <string>C617.1</string>
            </array>
        </dict>
        <dict>
            <key>NSPrivacyAccessedAPIType</key>
            <string>NSPrivacyAccessedAPICategorySystemBootTime</string>
            <key>NSPrivacyAccessedAPITypeReasons</key>
            <array>
                <string>35F9.1</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
```

### Adding the manifest

1. `File > New > File > Property List`, name it exactly `PrivacyInfo.xcprivacy`, add to the app target's Copy Bundle Resources.
2. For third-party SDKs: ship the framework as-is — Xcode merges nested manifests at archive time into the final Privacy Report.
3. Run `Product > Archive` and review the auto-generated Privacy Report PDF before every submission.

If a vendor refuses to ship an updated SDK, your only options are (a) drop the SDK or (b) declare its Required Reason API usage in your own app manifest with full justification — risky in review.

---

## App Tracking Transparency (ATT)

Required **only if you actually track** across apps/websites. "Track" = linking your app's user/device data with another company's apps/sites for ads or measurement, or sharing with a data broker.

If you don't track — skip the prompt entirely. Asking when you don't need to is a confidence destroyer.

```swift
import AppTrackingTransparency
import AdSupport

@MainActor
func requestTrackingIfNeeded() async {
    guard ATTrackingManager.trackingAuthorizationStatus == .notDetermined else { return }
    // Trigger only after the user has interacted with a feature that benefits from tracking
    let status = await ATTrackingManager.requestTrackingAuthorization()
    switch status {
    case .authorized:
        Analytics.shared.setIDFA(ASIdentifierManager.shared().advertisingIdentifier)
    default:
        Analytics.shared.disableTracking()
    }
}
```

### ATT must-do / must-not

- **Info.plist:** `NSUserTrackingUsageDescription` — plain-language "why", 8th-grade reading level, no dark patterns on your pre-prompt.
- **Foreground only.** iOS only shows the prompt while the app is foreground + active.
- **Fingerprinting fallback is FORBIDDEN.** Combining IP + locale + carrier + timezone + brightness + storage to reconstitute an identifier gets rejections under App Store Review Guidelines 5.1.1 / 5.1.2 (tracking + identity policy). Apple actively rejects apps that do this. (Note: ITMS-91008 is "Invalid API reason declaration" — a sibling of ITMS-91053 — not the fingerprinting code. There is no single fixed ITMS code for fingerprinting; rejections come through human review.)
- The IDFA returns zeros even for `.notDetermined`. Don't gate features on a non-zero IDFA before prompting.

### SKAdNetwork 4 / AdAttributionKit

Independent of ATT. SKAN 4 supports up to 3 postbacks (Window 1 = 48h fixed) with crowd anonymity tiers 0–3. AdAttributionKit (introduced 2024, expanded WWDC 2025) is the long-term successor and extends attribution beyond installs to surfaces like Safari.

---

## App Store Review red flags

The fast lane to rejection. Inspect every release for these:

- **Missing Privacy Manifest** (ITMS-91053 / ITMS-91061).
- **Asking permissions on launch.** Onboarding is for value, not consent.
- **Fingerprinting fallback after ATT denial.**
- **Pre-prompts that mimic the system dialog** — identical iconography, "Allow"/"Don't Allow" buttons, fake iOS chrome (Guideline 5.1.1).
- **Bribing for ATT consent** — coins/lives/premium for "Allow Tracking" (Guideline 3.2.2 + 5.1.1).
- **`canOpenURL` enumeration** beyond 50 entries in `LSApplicationQueriesSchemes` (Guideline 2.5.1). iOS 26.2 closed a remaining icon side-channel.
- **Undeclared data collection** — nutrition label disagrees with manifest or binary.
- **Background location without a clear feature.** Always authorization must tie to a visible, ongoing capability.
- **HealthKit data leaving device without explicit consent UI** — Apple specifically inspects.
- **Sensitive Content Analysis telemetry** — recording "image was flagged" violates the framework guarantee.
- **Communication app without PermissionKit** when audience may include children (new 2026 rejection category).
- **Missing entitlements** for Critical Alerts, multicast, sensitive content.

---

## App Privacy Nutrition Labels

Filled in App Store Connect, not in code, but **must** match your Privacy Manifest and your actual behavior. App Review now cross-checks all three.

- **14 categories:** Contact Info, Health & Fitness, Financial Info, Location, Sensitive Info, Contacts, User Content, Browsing History, Search History, Identifiers, Purchases, Usage Data, Diagnostics, Other Data.
- **"Collect" = transmitted off-device** for longer than the request it serves. Pure on-device processing does not count.
- **For each data type:** linked to user identity? used for tracking? purpose (third-party ads / your own ads / analytics / product personalization / app functionality / other).

Auto-derived from `PrivacyInfo.xcprivacy` + your App Store Connect declarations.

---

## Permissions — per-permission guide

The 2024–2026 shift: from "ask once at first launch" to **purpose-bound, scoped, time-limited authorization**. Limited Photos (iOS 14) became Limited Contacts (iOS 18) became Write-Only Calendar (iOS 17). Apple keeps adding scoped APIs that skip the prompt entirely — prefer them whenever you can.

### Location

Three Info.plist keys depending on what you need:

- `NSLocationWhenInUseUsageDescription` — required for foreground access.
- `NSLocationAlwaysAndWhenInUseUsageDescription` — required *in addition* if you ever call `requestAlwaysAuthorization()`.
- `NSLocationTemporaryUsageDescriptionDictionary` — keyed dictionary of purpose strings for one-time precise upgrades.

**Scoped API: `LocationButton`** (iOS 15+, `CoreLocationUI`). One-tap `.authorizedWhenInUse` grant with **no Info.plist key required** and **no permission prompt**:

```swift
import CoreLocationUI

struct WeatherView: View {
    @State private var locationManager = WeatherLocationManager()
    var body: some View {
        VStack {
            LocationButton(.shareCurrentLocation) {
                locationManager.requestLocation()
            }
            .symbolVariant(.fill)
            .labelStyle(.titleAndIcon)
            .clipShape(.rect(cornerRadius: 8, style: .continuous))
            .tint(.blue)

            if let temp = locationManager.temperature {
                Text(temp.formatted())
            }
        }
    }
}
```

**iOS 17+ modern flows:**

```swift
import CoreLocation

@Observable @MainActor
final class LocationStream {
    private(set) var updates: [CLLocation] = []

    func start() async {
        let session = CLServiceSession(authorization: .whenInUse)
        do {
            for try await update in CLLocationUpdate.liveUpdates() {
                guard let loc = update.location else { continue }
                updates.append(loc)
            }
        } catch {
            Logger.location.error("Live updates failed: \(error)")
        }
        _ = session  // retain for lifetime of stream
    }
}
```

`CLMonitor` replaces legacy `startMonitoring(for:)` geofences (iOS 18 broke long-standing behavior for some legacy flows).

**iOS 26 caveat:** there have been community reports of `LocationButton` tap-delay on early iOS 26 patch releases. If you target iOS 26 and see this, ship a `CLLocationManager` fallback and re-test after each minor update.

### Photos

**Prefer scoped API: `PhotosPicker`** (SwiftUI, iOS 16+). No Info.plist key, no permission prompt — the picker runs out-of-process and hands back only the explicitly-selected assets:

```swift
import PhotosUI

struct AvatarPicker: View {
    @State private var selection: PhotosPickerItem?
    @State private var image: Image?

    var body: some View {
        PhotosPicker(selection: $selection, maxSelectionCount: 1, matching: .images) {
            Label("Choose Photo", systemImage: "photo")
        }
        .onChange(of: selection) { _, newItem in
            Task {
                if let data = try? await newItem?.loadTransferable(type: Data.self),
                   let uiImage = UIImage(data: data) {
                    image = Image(uiImage: uiImage)
                }
            }
        }
    }
}
```

For UIKit code: `PHPickerViewController` (iOS 14+). **Never `UIImagePickerController` for new code.**

Full-library access (gallery apps indexing changes): `NSPhotoLibraryUsageDescription` + `NSPhotoLibraryAddUsageDescription`. Handle `.limited` — call `PHPhotoLibrary.shared().presentLimitedLibraryPicker(from:)` to let the user expand selection without sending them to Settings.

### Contacts (iOS 18+ limited)

`Contacts` / `ContactsUI`. Three-tier auth since iOS 18: full / limited / denied.

**Scoped API: `ContactAccessButton`** — single-contact access with one tap, no escalation:

```swift
import ContactsUI

struct ContactSearch: View {
    @State private var query = ""
    @State private var contactStore = CNContactStore()

    var body: some View {
        VStack {
            TextField("Search contacts", text: $query)
            ContactAccessButton(queryString: query) { identifiers in
                Task { await fetchContacts(ids: identifiers) }
            }
            .contactAccessButtonStyle(.init(imageWidth: 22))
        }
    }
}
```

Bulk grant: `.contactAccessPicker(isPresented:)` modal sheet.

Full access still needs `NSContactsUsageDescription`. Note: calling `requestAccess(for: .contacts)` while already `.limited` is a no-op — does not prompt to escalate. Send users to Settings if escalation is genuinely needed.

### Calendar / Reminders

`EventKit` / `EventKitUI`. iOS 17+ split calendar into **Full / Write-Only / Denied**. Reminders is still Full / Denied only.

**Prompt-free save: `EKEventEditViewController`** — let the user create an event without your app holding any access:

```swift
import EventKitUI

struct AddEventButton: UIViewControllerRepresentable {
    let title: String
    let start: Date
    let end: Date
    @Environment(\.dismiss) private var dismiss

    func makeUIViewController(context: Context) -> EKEventEditViewController {
        let store = EKEventStore()
        let controller = EKEventEditViewController()
        controller.eventStore = store
        let event = EKEvent(eventStore: store)
        event.title = title
        event.startDate = start
        event.endDate = end
        controller.event = event
        controller.editViewDelegate = context.coordinator
        return controller
    }
    func updateUIViewController(_ uiViewController: EKEventEditViewController, context: Context) {}
    func makeCoordinator() -> Coordinator { Coordinator(dismiss: dismiss) }

    final class Coordinator: NSObject, EKEventEditViewDelegate {
        let dismiss: DismissAction
        init(dismiss: DismissAction) { self.dismiss = dismiss }
        func eventEditViewController(_ controller: EKEventEditViewController,
                                     didCompleteWith action: EKEventEditViewAction) {
            dismiss()
        }
    }
}
```

Read-write: `NSCalendarsFullAccessUsageDescription` + `try await store.requestFullAccessToEvents()`. Write-only: `NSCalendarsWriteOnlyAccessUsageDescription` + `requestWriteOnlyAccessToEvents()`. Reminders: `NSRemindersFullAccessUsageDescription` + `requestFullAccessToReminders()`.

Write-only **cannot read events it created** in the same session — cache locally if you need to update or delete.

### Microphone / Camera

- `NSCameraUsageDescription` — required for any `AVCaptureDevice` video.
- `NSMicrophoneUsageDescription` — required for any audio capture.

```swift
import AVFoundation

func requestCameraAccess() async -> Bool {
    switch AVCaptureDevice.authorizationStatus(for: .video) {
    case .authorized: return true
    case .notDetermined: return await AVCaptureDevice.requestAccess(for: .video)
    default: return false
    }
}
```

System recording indicators are mandatory and cannot be suppressed: **orange dot** = microphone, **green dot** = camera, **red bar** = screen recording. They are SpringBoard-rendered. Verify your app only triggers them when expected — Control Center shows users which app is using each, so spurious activations land 1-star reviews.

Background camera/mic is generally disallowed; only VoIP + CallKit have exceptions.

For document/barcode scan, prefer `DataScannerViewController` (`VisionKit`, iOS 16+) — no custom capture pipeline needed and the same UI users already know from Camera.

### Push Notifications

No Info.plist usage string — Push Notifications capability + (optional) `remote-notification` background mode.

```swift
import UserNotifications

@MainActor
func requestNotificationAuth() async {
    let center = UNUserNotificationCenter.current()
    do {
        let granted = try await center.requestAuthorization(
            options: [.alert, .badge, .sound, .provisional, .timeSensitive]
        )
        if granted { await UIApplication.shared.registerForRemoteNotifications() }
    } catch {
        Logger.notifications.error("\(error)")
    }
}
```

- **Provisional (iOS 12+):** include `.provisional` in options — no prompt; notifications arrive silently in Notification Center where the user can Keep or Turn Off. Start here for any app where notifications aren't *obviously* needed at first launch.
- **Time-Sensitive:** include `.timeSensitive`, set `interruption-level: "time-sensitive"` in APNs payload, request entitlement `com.apple.developer.usernotifications.time-sensitive`.
- **Critical:** `[.criticalAlert]` + Apple-approved entitlement; bypasses silent + DND.
- **Communication Notifications:** donate `INSendMessageIntent` + `INStartCallIntent` from a Notification Service Extension. Shows sender avatar, supports tapbacks, respects per-person Focus allow-lists.

iOS 18 Priority Notifications rank apps by `relevance-score`. Apps that flag urgency correctly bubble up; apps that abuse `time-sensitive` get muted by Apple Intelligence summaries.

### HealthKit

- `NSHealthShareUsageDescription` — required when reading.
- `NSHealthUpdateUsageDescription` — required when writing.
- `NSHealthClinicalHealthRecordsShareUsageDescription` — clinical records (FHIR).
- Add the **HealthKit capability** to your target.

```swift
import HealthKit

@MainActor
final class HealthAccess {
    let store = HKHealthStore()

    func requestAuth() async throws {
        let read: Set<HKObjectType> = [
            HKObjectType.quantityType(forIdentifier: .stepCount)!,
            HKObjectType.quantityType(forIdentifier: .heartRate)!
        ]
        let write: Set<HKSampleType> = [
            HKObjectType.quantityType(forIdentifier: .dietaryWater)!,
            HKObjectType.categoryType(forIdentifier: .mindfulSession)!
        ]
        try await store.requestAuthorization(toShare: write, read: read)
    }
}
```

Per-data-type authorization with **separate read and write sets**. Read status is **not introspectable** by design (prevents inferring whether the user has data). Detect "no data" by querying for empty results — do NOT equate that with denial. Re-running `requestAuthorization` only prompts for types not yet shown; adding new types in a future release re-prompts the user.

### Bluetooth / Local Network / Speech

**Bluetooth:** `NSBluetoothAlwaysUsageDescription` required from iOS 13+. **Accessing any CoreBluetooth API without this key crashes the app at runtime.** `NSBluetoothPeripheralUsageDescription` is deprecated (only ship for pre-iOS-13 deployment targets).

**Local Network:** `NSLocalNetworkUsageDescription` (the reason string in the prompt) + `NSBonjourServices` (array of every service type your app browses/publishes, e.g., `_myapp._tcp`). The system prompts the first time you do anything LAN-y. There is **no public API to query authorization status or explicitly request it** — only attempting an operation triggers the prompt. Pre-prompt that explains *which devices* ("Find your Sonos speakers on this Wi-Fi") and have the system reason mirror it.

For non-Bonjour multicast/broadcast or arbitrary Bonjour types, request the `com.apple.developer.networking.multicast` entitlement from Apple (request-only).

**Speech:** `NSSpeechRecognitionUsageDescription` + `NSMicrophoneUsageDescription` (both required). Server-side processing unless `SFSpeechRecognizer.supportsOnDeviceRecognition` is true and you set `recognitionRequest.requiresOnDeviceRecognition = true`. Disclose if you ship to privacy-sensitive contexts.

### iOS 26 platform additions

- **PermissionKit** (iOS 26): Messages-mediated **parental approval** flow for Child Accounts in Family Sharing. Required if your app has social/communication features that may include children. Without it, kids can bypass Apple's Communication Limits via your app — growing 2026 rejection vector.
- **DeclaredAgeRange** (`requestAgeRange` available iOS 26+; `isEligibleForAgeFeatures` API iOS 26.2+): coarse age bucket binning (`<13`, `13–15`, `16–17`, `17+`) without ever seeing a birthdate. Texas-style 4-bucket binning handled automatically per region. Apple notifies your server when parental consent is revoked — you must deny access on receipt. Requires `com.apple.developer.declared-age-range` entitlement.

```swift
import DeclaredAgeRange

struct AgeGated: View {
    @Environment(\.requestAgeRange) private var requestAgeRange
    @State private var allowed = false

    var body: some View {
        Button("Continue") {
            Task {
                let response = try? await requestAgeRange(ageGates: 13, 16, 18)
                if case .sharing(let range) = response {
                    allowed = range.lowerBound >= 16
                }
            }
        }
    }
}
```

- **Wired Accessories** (iOS 26): a new Settings toggle gates USB-C/Lightning data connections when the phone is locked. `ExternalAccessory`, MFi audio/MIDI, and USB-C DriverKit code should expect connection failures while locked — present an "Unlock to continue" prompt instead of silently failing.

---

## Permission UX commandments

1. **Never ask on first launch.** Onboarding is for value, not consent. Capture an in-context need first.
2. **Prime before the prompt.** Show your own pre-permission screen explaining the user benefit. Use *Continue* / *Not now* — never *Allow* / *Deny* in your own UI (Guideline 5.1.1).
3. **One permission per moment.** Stacking camera + microphone + location + notifications at launch guarantees mass denial.
4. **Frame benefits, not features.** "Find restaurants near you" beats "We need your location."
5. **Prefer scoped APIs.** `PhotosPicker` (no prompt), `LocationButton` (no prompt), `ContactAccessButton` (one-tap), `EKEventEditViewController` (no access required) all skip the system sheet entirely.
6. **Provide a "denied" recovery path.** Link to Settings via `UIApplication.openSettingsURLString`.
7. **Don't ask repeatedly.** iOS shows each system prompt at most once. Respect the user's NO.
8. **Test under "denied" state.** Your app must function or degrade gracefully. Listen for revocation callbacks (`locationManagerDidChangeAuthorization`, `PHPhotoLibrary.register`, `EKEventStore.authorizationStatus`).
9. **Respect Limited and Approximate.** Don't beg users to upgrade to Full or Precise. First-class UX at the scoped tier wins them over.
10. **Tell users what they'll see.** ATT / Local Network / Always-location prompts have specific Apple-mandated wording — your pre-prompt should mention "the next screen is from iOS" so users don't reflexively dismiss.
11. **Audit purpose strings every release.** The string is the contract; generic or misleading copy gets rejected.

---

## Settings deep links

```swift
import UIKit

func openAppSettings() {
    if let url = URL(string: UIApplication.openSettingsURLString) {
        UIApplication.shared.open(url)
    }
}

func openNotificationSettings() {
    // iOS 15.4+
    if let url = URL(string: UIApplication.openNotificationSettingsURLString) {
        UIApplication.shared.open(url)
    }
}
```

Deep-linking to specific permission sub-screens via private `prefs:root=...` schemes is **unsupported** and grounds for rejection. Always route through your own Settings entry.

---

## StoreKit 2

Modern async/await IAP. JWS-signed transactions verified locally — **no server roundtrip required for ownership checks**. SwiftUI `StoreView`, `SubscriptionStoreView`, `SubscriptionOfferView`. iOS 18.4 adds `appTransactionID` (cross-device tracking), JWS-authenticated promotional offers, and Advanced Commerce API.

```swift
import StoreKit

@Observable @MainActor
final class Subscriptions {
    private(set) var activeProductIDs: Set<String> = []

    func refresh() async {
        var owned: Set<String> = []
        for await result in Transaction.currentEntitlements {
            guard case .verified(let txn) = result else { continue }
            if txn.revocationDate == nil { owned.insert(txn.productID) }
        }
        activeProductIDs = owned
    }

    func observeUpdates() async {
        for await result in Transaction.updates {
            guard case .verified(let txn) = result else { continue }
            await refresh()
            await txn.finish()
        }
    }
}

struct PaywallScreen: View {
    var body: some View {
        SubscriptionStoreView(groupID: "21471234") {
            VStack {
                Text("Acme Pro").font(.largeTitle.bold())
                Text("Unlock every feature.").foregroundStyle(.secondary)
            }
        }
        .subscriptionStoreControlStyle(.prominentPicker)
        .storeButton(.visible, for: .restorePurchases)
    }
}
```

**Never StoreKit 1 in new code.** 1/10 the code, JWS kills the cottage industry of receipt-validation libraries, `SubscriptionStoreView` gives you an App-Store-quality paywall in 10 lines.

---

## SharePlay (GroupActivities)

Multi-user shared experiences across FaceTime. Apple's surface for "do this together inside your app."

```swift
import GroupActivities

struct WatchTogether: GroupActivity {
    let videoID: String
    var metadata: GroupActivityMetadata {
        var meta = GroupActivityMetadata()
        meta.title = "Watch Together"
        meta.type = .watchTogether
        return meta
    }
}

@MainActor
func startSharePlay(videoID: String) async {
    let activity = WatchTogether(videoID: videoID)
    switch await activity.prepareForActivation() {
    case .activationPreferred: _ = try? await activity.activate()
    case .activationDisabled, .cancelled: break
    @unknown default: break
    }
}
```

Skipping SharePlay forfeits the FaceTime virality channel for video, audio, games, drawing, and fitness apps.

---

## Background Tasks

`BGTaskScheduler` for arbitrary background work. Three task types:

- `BGAppRefreshTask` — short-lived periodic refresh.
- `BGProcessingTask` — cleanup/maintenance with network/charging requirements.
- `BGContinuedProcessingTask` (iOS 26) — long-running work that survives foreground→background transition with **system-presented progress UI** and optional **background GPU access**.

```swift
import BackgroundTasks

@main
struct UploaderApp: App {
    init() {
        BGTaskScheduler.shared.register(forTaskWithIdentifier: "com.acme.export",
                                        using: nil) { task in
            handleExport(task: task as! BGContinuedProcessingTask)
        }
    }
    var body: some Scene { WindowGroup { ContentView() } }
}

func scheduleExport() {
    let request = BGContinuedProcessingTaskRequest(
        identifier: "com.acme.export",
        title: "Exporting Project",
        subtitle: "Rendering 4K video"
    )
    request.requiresGPU = true
    do { try BGTaskScheduler.shared.submit(request) }
    catch { Logger.background.error("\(error)") }
}

func handleExport(task: BGContinuedProcessingTask) {
    let progress = task.progress
    progress.totalUnitCount = 100

    let work = Task {
        for i in 0..<100 {
            try Task.checkCancellation()
            await Exporter.shared.renderStep(i)
            progress.completedUnitCount = Int64(i + 1)
            task.updateTitle("Exporting", subtitle: "\(i + 1)%")
        }
        task.setTaskCompleted(success: true)
    }
    task.expirationHandler = { work.cancel() }
}
```

BGContinuedProcessingTask finally answers "the user hit Export and walked away — please don't kill my upload."

---

## Foundation Models (iOS 26)

3B-parameter on-device LLM via `SystemLanguageModel`. Free, offline, private. Falls back to **Private Cloud Compute (PCC)** for harder requests when allowed — Apple-silicon servers with Secure Enclave, Secure Boot, attested code, zero retained data.

```swift
import FoundationModels

@Generable
struct ExtractedRecipe {
    var title: String
    var servings: Int
    var ingredients: [String]
    var stepsMinutes: Int
}

func extractRecipe(from text: String) async throws -> ExtractedRecipe {
    let session = LanguageModelSession()
    let prompt = "Extract the recipe from the following text:\n\(text)"
    return try await session.respond(to: prompt, generating: ExtractedRecipe.self).content
}
```

**Use when:** summarization, classification, structured extraction, rewriting, tagging — narrow tasks with type-safe `@Generable` output.

**Don't use when:** GPT-class chat, world-knowledge Q&A, code generation, >4K context, multimodal generation. Apple is explicit about this. Community testing has reported the 3B Apple Foundation Model performs worse than similarly-sized open-source models on broad tasks — generalize the claim: "Apple's 3B on-device model is a narrow-task tool, not a general assistant."

Reach for it when on-device privacy + narrow output matter (PII, health, finance, photos by default). For chat, use a server-side LLM with proper consent UI and a privacy nutrition label entry.

---

## Visual Intelligence (iOS 26)

System-level visual search activated from Camera Control on iPhone 16 (or via screenshot pipeline). Your app participates via App Intents conforming to `IntentValueQuery<SemanticContentDescriptor>` (see App Intents section above). Without an `IntentValueQuery`, your catalog is invisible to it.

Use for: catalog/commerce, travel lookup, plant/pet ID, recipe/ingredient, social ("who is this artist?"), education, identification utilities.

---

## SF Symbols 6+

System symbol library. Use them — don't ship custom icons unless brand-required.

### Rendering modes

```swift
Image(systemName: "cloud.sun.rain.fill")
    .symbolRenderingMode(.multicolor)    // .monochrome / .hierarchical / .palette / .multicolor

Image(systemName: "heart.fill")
    .symbolRenderingMode(.palette)
    .foregroundStyle(.red, .pink)

Image(systemName: "wifi")
    .symbolVariableValue(0.4)            // 0…1 fills the variable bars
```

### Animations

```swift
Image(systemName: "bell.fill")
    .symbolEffect(.bounce, value: notificationCount)

Image(systemName: "wifi")
    .symbolEffect(.variableColor.iterative.reversing)

Image(systemName: "ellipsis")
    .symbolEffect(.pulse)
```

### SF Symbols rules

- Match symbol weight to text weight via `.font(.body)` first, *then* `Image`.
- Multi-color symbols look gaudy next to single-color UI — establish a per-app rule.
- Prefer `.foregroundStyle()` over deprecated `.foregroundColor()` (see `modern-api.md`).

---

## App Attest / DeviceCheck

Anti-fraud. Cryptographic proof from the Secure Enclave that requests come from your unmodified binary on a real device. DeviceCheck adds two persistent bits per device (e.g., fraud flag) that survive reinstalls.

```swift
import DeviceCheck

func attestBackend() async throws {
    guard DCAppAttestService.shared.isSupported else { return }
    let service = DCAppAttestService.shared
    let keyID = try await service.generateKey()
    let challenge = try await Backend.shared.fetchAttestChallenge()
    let clientDataHash = Data(SHA256.hash(data: challenge))
    let attestation = try await service.attestKey(keyID, clientDataHash: clientDataHash)
    try await Backend.shared.verifyAttestation(attestation, keyID: keyID, challenge: challenge)
}
```

~50 lines, free, prevents trial abuse / spam signups / reverse-engineered API clients. One of the most under-used iOS APIs.

---

## MapKit modern

SwiftUI-native `Map { Marker; Annotation; MapPolyline; UserAnnotation }` (iOS 17+). `MapCameraPosition` for control. Drop UIKit `MKMapView` bridging for new code.

```swift
import MapKit

struct PinMap: View {
    @State private var camera: MapCameraPosition = .automatic
    let places: [Place]

    var body: some View {
        Map(position: $camera) {
            ForEach(places) { place in
                Marker(place.name, systemImage: "fork.knife", coordinate: place.coord)
                    .tint(.orange)
            }
            UserAnnotation()
        }
        .mapStyle(.standard(elevation: .realistic))
        .mapControls { MapUserLocationButton(); MapCompass() }
    }
}
```

iOS 26 adds Liquid Glass styling and better dark-mode tiles. If you're still bridging `UIViewRepresentable(MKMapView)` in 2026, you're carrying tech debt for no reason.

---

## Charts framework

`Chart` + `BarMark` / `LineMark` / `AreaMark` / `PointMark`. Native, performant, accessible.

```swift
import Charts

struct StepsChart: View {
    let entries: [DailySteps]
    var body: some View {
        Chart(entries) { entry in
            BarMark(
                x: .value("Day", entry.date, unit: .day),
                y: .value("Steps", entry.count)
            )
            .foregroundStyle(.blue.gradient)
        }
        .chartYAxis { AxisMarks(position: .leading) }
    }
}
```

---

## HealthKit / WeatherKit / Translation

- **WeatherKit** (iOS 16+): first-party weather. Current, 10-day hourly, minute-by-minute precip, severe-weather alerts. 500k calls/month free per dev membership. No need for OpenWeatherMap / Dark Sky replacements.

```swift
import WeatherKit
import CoreLocation

func fetchWeather(for location: CLLocation) async throws -> CurrentWeather {
    let weather = try await WeatherService.shared.weather(for: location)
    return weather.currentWeather
}
```

- **Translation framework** (iOS 17.4+): `import Translation`. `.translationTask` / `.translationPresentation` SwiftUI modifiers. iOS 18 adds programmatic batch translation. On-device for 19+ language pairs when supported, falls back to server.

```swift
import Translation

struct TranslateField: View {
    @State private var text = ""
    @State private var config: TranslationSession.Configuration?
    var body: some View {
        TextField("Translate", text: $text)
            .translationPresentation(isPresented: .constant(true), text: text)
    }
}
```

---

## Communication Notifications

For messaging-style apps — face avatars, communication intents. Donate `INSendMessageIntent` from a Notification Service Extension; the system upgrades the notification to show the sender's photo, respects per-person Focus allow-lists, and integrates with Siri suggestions.

```swift
import Intents

func donateIncomingMessage(from sender: String, body: String) {
    let person = INPerson(
        personHandle: INPersonHandle(value: sender, type: .unknown),
        nameComponents: nil, displayName: sender, image: nil,
        contactIdentifier: nil, customIdentifier: sender,
        isContactSuggestion: false, suggestionType: .none
    )
    let intent = INSendMessageIntent(
        recipients: nil, outgoingMessageType: .outgoingMessageText,
        content: body, speakableGroupName: nil,
        conversationIdentifier: sender, serviceName: "Acme", sender: person, attachments: nil
    )
    let interaction = INInteraction(intent: intent, response: nil)
    interaction.donate { _ in }
}
```

Without it, your app's chats compete with system Messages and lose.

---

## Real-time and time-sensitive

Push interruption levels: `.passive` / `.active` / `.timeSensitive` / `.critical`. Match user expectation — abuse leads to users disabling all your notifications, and Apple Intelligence rankings reward apps that flag urgency honestly.

```json
{
  "aps": {
    "alert": { "title": "Door is unlocked", "body": "Front entry was just opened." },
    "interruption-level": "time-sensitive",
    "relevance-score": 0.9
  }
}
```

---

## iOS 26 platform additions (recap)

The capabilities introduced or expanded in iOS 26 worth tracking:

- **BGContinuedProcessingTask** — long-running background with system progress UI + background GPU.
- **Foundation Models** — 3B on-device LLM via `LanguageModelSession`.
- **Visual Intelligence** — `IntentValueQuery<SemanticContentDescriptor>` integration via App Intents.
- **PermissionKit** — parental approval for child accounts.
- **DeclaredAgeRange** (`requestAgeRange` iOS 26+; `isEligibleForAgeFeatures` iOS 26.2+) — coarse age bucket binning.
- **Wired Accessories** — Settings toggle gating USB-C/Lightning data while locked.
- **Live Activities updates** — `ContentMargins`, tinted icons, AppIntent buttons (iOS 18+ carried forward).
- **Account Creation API** — `ASAuthorizationAccountCreationProvider` for one-tap passkey signup.
- **Map / glass styling refinements.**

---

## Cross-references

- **State and observation** (`@AppStorage`, `@Observable`, ownership) → `state-and-observation.md`.
- **Navigation** (typed routes, deep-link routing from `.onOpenURL`) → `navigation.md`.
- **macOS-specific platform guidance** → `macos-platform.md`.
- **Modern API replacement table** (e.g., `.foregroundStyle` vs `.foregroundColor`) → `modern-api.md`.
- **Anti-patterns** (UIImagePickerController, StoreKit 1, `canOpenURL` abuse, etc.) → `anti-patterns.md`.

---

## Anti-patterns

Hard "no" list for 2026 iOS code:

- **`UIImagePickerController`** for new code. Use `PhotosPicker` / `PHPickerViewController`.
- **Asking permission on first launch.** Always prime in-context.
- **Fingerprinting fallback after ATT denial.** Human review rejection under Guidelines 5.1.1 / 5.1.2. (ITMS-91008 is a separate code — "Invalid API reason declaration" — not fingerprinting.)
- **`canOpenURL` enumeration** beyond 50 entries in `LSApplicationQueriesSchemes`.
- **Missing `PrivacyInfo.xcprivacy`.** ITMS-91053 submission failure.
- **Pre-iOS-17 widget patterns** (read-only billboards with no `Button(intent:)` or `Toggle(isOn:intent:)`).
- **Custom tooltip systems.** Use TipKit.
- **StoreKit 1** in new code.
- **UIKit `MKMapView` bridging** for new code. Use SwiftUI `Map`.
- **`requestAuthorization` on launch** for any permission.
- **`NSBluetoothPeripheralUsageDescription`** alone (deprecated; needs the always key).
- **Background scanning without throttled filters** — power drain + system kill.
- **Sensitive Content Analysis telemetry** — recording flags violates the framework guarantee and is a rejection vector.
- **Custom URL schemes only** — use Universal Links + AASA.
- **Pre-prompts that mimic system dialogs** (Allow/Deny buttons, fake iOS chrome).
- **Bribing for ATT consent.** Guideline 3.2.2 + 5.1.1.
- **Background location without a visible feature.** Always-auth must tie to ongoing capability.
- **Read-only AppIntents** (no `AppShortcutsProvider`) on apps with recurring user verbs.
- **HealthKit data leaving device without explicit consent UI.**
- **Storing tokens or PII in `UserDefaults`.** Use Keychain (see `state-and-observation.md`).
- **`@AppStorage` for `@Observable` state coupling** without the `UserDefaults` observation workaround (see `state-and-observation.md`).
