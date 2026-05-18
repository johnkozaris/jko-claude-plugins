# macOS Platform Reference (Swift 6.3 / macOS 26 Tahoe / Xcode 26)

Target: native Mac apps shipping in 2026 with Liquid Glass design language, Hardened Runtime, notarization, and modern distribution. Audience: an AI reviewing macOS code in a SwiftUI plugin.

If you are reviewing a Mac SwiftUI app, start here. iOS habits do not port. Read this file before writing critique on macOS targets.


---

## macOS is not iOS with a bigger screen

This is the single most important framing for Mac development in 2026. Mac users live in the menu bar and the keyboard. Apps without those two surfaces fully wired read as iPad ports — and Mac reviewers will say so within seconds of opening the app.

Critique on sight:

- A SwiftUI Mac app whose `.commands { ... }` is empty (`File > Close Window` only).
- A primary action with no `.keyboardShortcut(...)`.
- A "hamburger" / burger-menu navigation pattern (the menu bar IS the navigation on macOS).
- A FAB (floating action button) sitting in the lower-right of a Mac window.
- Pull-to-refresh on a Mac scroll view (no such gesture exists on a trackpad).
- A view that accepts a tap but not a drag, drop, right-click, double-click, hover, or Services menu.
- A document-style app that doesn't restore windows after relaunch.
- A Settings window opened from a `Button("Preferences")` in the toolbar instead of the `Settings { ... }` scene at `Command-Comma`.

Real-codebase evidence that AppKit is alive in 2026:

| Repo | Stars | Mac-native posture |
|---|---:|---|
| IINA | 44,902 | 97 files `import Cocoa`, **1** file imports SwiftUI |
| Stats (exelban) | 38,872 | 92 files `import Cocoa`; SwiftUI only inside Widget targets |
| Rectangle | 29,058 | Pure Cocoa; zero SwiftUI |
| Ice (jordanbaird) | 27,983 | SwiftUI shell + raw `NSStatusItem` + `CGEventTap` |
| MeetingBar | 5,235 | SwiftUI + AppKit hybrid; no `MenuBarExtra` — raw `NSStatusItem` |

Combined ~146k stars of "SwiftUI-not-adopted-or-only-partially" Mac utility apps. The Apple-marketing narrative "SwiftUI everywhere by 2025" is empirically false for utility / menu-bar / media apps. AppKit is the right tool more often than iOS-focused tutorials admit.

Bridge to AppKit without guilt. Most production Mac SwiftUI apps are hybrid by necessity, not preference. The skill should never critique a Mac app for `NSViewRepresentable` use unless the wrapped API has a real SwiftUI equivalent (`Map`, `Charts`, `WebView`, `PhotosPicker`).

---

## The Main Menu — the chrome that says "Mac app"

Every important action must be reachable from the main menu, and every menu item must have a keyboard equivalent. This is the bar between "feels native" and "obviously a port."

### Anatomy

The main menu is built by attaching `.commands { ... }` to a `Scene`:

```swift
@main
struct EditorApp: App {
    @State private var doc = DocumentStore()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(doc)
        }
        .commands {
            // Replace defaults
            CommandGroup(replacing: .newItem) {
                Button("New Document") { doc.newDocument() }
                    .keyboardShortcut("n", modifiers: [.command])
                Button("New From Template...") { doc.newFromTemplate() }
                    .keyboardShortcut("n", modifiers: [.command, .shift])
            }

            // Standard Edit menu — get free Cut/Copy/Paste/Select All
            // by NOT replacing .pasteboard / .textEditing

            // Add a Format menu (new top-level menu)
            CommandMenu("Format") {
                Button("Bold") { doc.toggleBold() }
                    .keyboardShortcut("b", modifiers: [.command])
                Button("Italic") { doc.toggleItalic() }
                    .keyboardShortcut("i", modifiers: [.command])
                Button("Underline") { doc.toggleUnderline() }
                    .keyboardShortcut("u", modifiers: [.command])

                Divider()

                Menu("Alignment") {
                    Button("Left")     { doc.align(.leading) }
                        .keyboardShortcut("{", modifiers: [.command])
                    Button("Center")   { doc.align(.center) }
                        .keyboardShortcut("|", modifiers: [.command])
                    Button("Right")    { doc.align(.trailing) }
                        .keyboardShortcut("}", modifiers: [.command])
                }
            }

            // Add to existing groups
            CommandGroup(after: .toolbar) {
                Toggle("Show Inspector", isOn: $doc.showInspector)
                    .keyboardShortcut("i", modifiers: [.command, .option])
            }

            // Replace Help menu contents
            CommandGroup(replacing: .help) {
                Link("Editor Help", destination: URL(string: "https://example.com/help")!)
            }
        }
    }
}
```

### `CommandGroup` placement identifiers

These are the placement keys you'll target. Use `CommandGroup(before: ...)`, `CommandGroup(after: ...)`, or `CommandGroup(replacing: ...)`:

| Identifier | Where it lives |
|---|---|
| `.appInfo` | App menu → "About AppName" |
| `.appSettings` | App menu → "Settings..." (Command-Comma) |
| `.systemServices` | App menu → Services submenu |
| `.appTermination` | App menu → Quit |
| `.newItem` | File → New / Open |
| `.saveItem` | File → Save / Save As |
| `.importExport` | File → Import / Export |
| `.printItem` | File → Print |
| `.undoRedo` | Edit → Undo / Redo |
| `.pasteboard` | Edit → Cut / Copy / Paste / Delete / Select All |
| `.textEditing` | Edit → Find, replace, spelling submenu |
| `.textFormatting` | Format menu staples |
| `.toolbar` | View → Show Toolbar / Customize Toolbar |
| `.sidebar` | View → Show/Hide Sidebar |
| `.windowList` | Window menu list |
| `.windowSize` | Window → Minimize / Zoom |
| `.windowArrangement` | Window → Bring All to Front |
| `.help` | Help menu |

### Focused values — menus that act on the front window

The classic Mac responder chain is replaced by `@FocusedValue` + `.focusedSceneValue`. This is how a menu command in your `.commands { }` block knows which window's selection to act on.

```swift
// 1. Declare a focused-value key
struct SelectedItemsKey: FocusedValueKey {
    typealias Value = Binding<Set<Item.ID>>
}

extension FocusedValues {
    var selectedItems: Binding<Set<Item.ID>>? {
        get { self[SelectedItemsKey.self] }
        set { self[SelectedItemsKey.self] = newValue }
    }
}

// 2. Publish from the active scene
struct ContentView: View {
    @State private var selection = Set<Item.ID>()

    var body: some View {
        List(selection: $selection) { /* ... */ }
            .focusedSceneValue(\.selectedItems, $selection)
    }
}

// 3. Consume from .commands
.commands {
    CommandMenu("Item") {
        FocusedValueButton(\.selectedItems, key: "Delete") { selection in
            Button("Delete") {
                /* delete using selection.wrappedValue */
            }
            .keyboardShortcut(.delete)
            .disabled(selection.wrappedValue.isEmpty)
        }
    }
}

// (FocusedValueButton is a small helper that bridges @FocusedValue into a CommandMenu body.)
struct FocusedValueButton<Value, Content: View>: View {
    @FocusedValue<Value>(_ keyPath: KeyPath<FocusedValues, Value?>) private var value: Value?
    let key: String
    let content: (Value) -> Content
    init(_ keyPath: KeyPath<FocusedValues, Value?>, key: String,
         @ViewBuilder content: @escaping (Value) -> Content) {
        _value = .init(keyPath)
        self.key = key
        self.content = content
    }
    var body: some View {
        if let value { content(value) } else { EmptyView() }
    }
}
```

Without `.focusedSceneValue`, the Item menu greys out for every window — the menu bar has no idea which selection to operate on. This is one of the loudest "iPad port" tells in a Mac SwiftUI app.

---

## Window management

### Scene types

```swift
@main
struct ProAppDemo: App {
    var body: some Scene {
        // Primary multi-window scene
        WindowGroup("Document", for: Document.ID.self) { $docID in
            DocumentView(documentID: docID)
        }
        .windowResizability(.contentSize)
        .windowToolbarStyle(.unified(showsTitle: true))

        // Single-instance window (macOS 13+) — About / Welcome / global Inspector
        Window("About", id: "about") {
            AboutView()
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)
        .defaultPosition(.center)

        // Floating utility panel (macOS 15+)
        UtilityWindow("Tools", id: "tools") {
            ToolsPaletteView()
        }
        .defaultPosition(.trailing)
        .windowLevel(.floating)

        // The Settings/Preferences scene
        Settings {
            SettingsView()
        }
        .windowResizability(.contentSize)

        // Persistent menu bar status item
        MenuBarExtra("Sync", systemImage: "arrow.triangle.2.circlepath") {
            MenuBarContent()
        }
        .menuBarExtraStyle(.window)
    }
}
```

| Scene | macOS min | Use for |
|---|---:|---|
| `WindowGroup` | 11.0 | Document/library windows; multiple simultaneous instances |
| `Window` | 13.0 | Singleton: About, Welcome, single global Inspector |
| `UtilityWindow` | 15.0 | Floating inspectors, palettes (Pixelmator/Sketch style) |
| `Settings` | 11.0 | The Preferences window. ALWAYS include this. Command-Comma. |
| `MenuBarExtra` | 13.0 | Menu-bar status item app or companion |
| `DocumentGroup` | 11.0 | Doc-per-file apps wired into Open/Save/Recent/iCloud |

### Window modifiers worth knowing

```swift
WindowGroup("Editor") { EditorView() }
    .windowResizability(.contentSize)            // .automatic | .contentSize | .contentMinSize
    .windowStyle(.titleBar)                       // .titleBar | .hiddenTitleBar | .plain
    .windowToolbarStyle(.unified)                 // .automatic | .expanded | .unified | .unifiedCompact
    .defaultSize(width: 1100, height: 700)
    .defaultPosition(.center)
    .commandsRemoved()                            // suppress default commands for this scene
    .handlesExternalEvents(matching: ["mydoc"])   // URL scheme routing
    .windowLevel(.normal)                         // .floating | .submenu | .torn off | .modalPanel
    .restorationBehavior(.automatic)              // macOS 14+ — Scene restoration
```

### Scene restoration

```swift
WindowGroup("Document", for: Document.ID.self) { $docID in
    DocumentView(documentID: docID)
}
.restorationBehavior(.automatic)   // macOS 14+
.handlesExternalEvents(matching: ["mydoc"])
```

Pair with `NSUserActivity` for richer restoration of cursor, scroll, and selection. Broken restoration is an iPad-port smell — Mac users habitually Option-Quit-and-relaunch and expect every window back where it was.

---

## MenuBarExtra — the gap

`MenuBarExtra` is a SwiftUI scene for persistent menu bar status items, available since macOS 13. **No first-party state API exists** for showing/hiding the popover, reading whether it is open, or programmatically dismissing it. `SettingsLink` and `openSettings` are also unreliable from inside a MenuBarExtra on macOS 26.

Two productive workarounds plus one nuclear option:

```swift
// Option 1 — Third-party (recommended for SwiftUI-heavy apps)
//   MenuBarExtraAccess: https://github.com/orchetect/MenuBarExtraAccess
//   FluidMenuBarExtra:  https://github.com/lfroms/fluid-menu-bar-extra

@main
struct StatusApp: App {
    @State private var isOpen = false

    var body: some Scene {
        MenuBarExtra("Sync", systemImage: "checkmark.icloud") {
            StatusPopover()
                .menuBarExtraAccess(isPresented: $isOpen) // from MenuBarExtraAccess
        }
        .menuBarExtraStyle(.window)
    }
}

// Option 2 — Drop to AppKit (production reality for many real apps)
@MainActor
final class StatusItemController {
    let item: NSStatusItem

    init() {
        item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        item.button?.title = "Sync"
        item.button?.image = NSImage(systemSymbolName: "checkmark.icloud", accessibilityDescription: nil)
        item.button?.action = #selector(togglePopover(_:))
        item.button?.target = self
    }

    private lazy var popover: NSPopover = {
        let p = NSPopover()
        p.behavior = .transient
        p.contentSize = .init(width: 360, height: 480)
        p.contentViewController = NSHostingController(rootView: StatusPopover())
        return p
    }()

    @objc private func togglePopover(_ sender: NSStatusBarButton) {
        if popover.isShown {
            popover.performClose(sender)
        } else if let button = item.button {
            popover.show(relativeTo: button.bounds, of: button, preferredEdge: .minY)
        }
    }
}
```

Real-codebase evidence: **Ice (28k stars) and MeetingBar (5.2k stars) use raw `NSStatusBar.system.statusItem` despite shipping SwiftUI elsewhere.** When the user asks "why isn't this a `MenuBarExtra`," the answer is "because the SwiftUI API is incomplete."

The one piece of guidance: if your menu bar app needs a popover with custom positioning, programmatic dismiss, or a Settings entry that actually works, plan on dropping to `NSStatusItem` from day 1. Migrating from `MenuBarExtra` later is painful.

---

## Document-based apps

```swift
import SwiftUI
import UniformTypeIdentifiers

// 1. Define your file type
extension UTType {
    static let recipeDocument = UTType(exportedAs: "com.example.recipe")
}

// 2. The document
struct RecipeDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.recipeDocument] }
    static var writableContentTypes: [UTType] { [.recipeDocument] }

    var recipe: Recipe

    init(recipe: Recipe = Recipe()) {
        self.recipe = recipe
    }

    init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents else {
            throw CocoaError(.fileReadCorruptFile)
        }
        self.recipe = try JSONDecoder().decode(Recipe.self, from: data)
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        let data = try JSONEncoder().encode(recipe)
        return .init(regularFileWithContents: data)
    }
}

// 3. The app
@main
struct RecipesApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: { RecipeDocument() }) { config in
            RecipeEditor(document: config.$document)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("New Recipe") { /* DocumentGroup handles New automatically */ }
                    .keyboardShortcut("n", modifiers: [.command])
            }
        }
    }
}
```

Use `ReferenceFileDocument` (`@Observable` class) when documents are large or identity-bearing — but be aware saves run on the main actor and can beachball; prefer the struct `FileDocument` when feasible.

`DocumentGroup` gives you: File → Open/Save/Save As/Revert/Duplicate; the Recent Documents menu; the Document Browser on iOS/iPadOS; iCloud Drive sync; Versions; the standard Save sheet with format pickers. Re-implementing these by hand is the canonical "iPad-port-on-Mac" failure.

Always declare your UTType in the target's Info.plist (`UTExportedTypeDeclarations`) and register `CFBundleDocumentTypes` so Finder hands files to your app on double-click.

---

## Drag-and-drop — the Mac superpower

Drag-and-drop between apps (Finder → yours, yours → Mail, yours → Notes) is THE Mac superpower. iOS apps that ignore it feel inert on the desktop.

```swift
import SwiftUI
import UniformTypeIdentifiers

// 1. Make a domain type drag-payload-shaped
struct Note: Identifiable, Codable, Transferable {
    let id: UUID
    var title: String
    var body: String

    static var transferRepresentation: some TransferRepresentation {
        // Primary: our custom UTI
        CodableRepresentation(contentType: .note)
        // Fallbacks for cross-app drops
        ProxyRepresentation(exporting: \.title)             // → public.plain-text
        FileRepresentation(exportedContentType: .plainText) { note in
            let url = URL.temporaryDirectory.appendingPathComponent("\(note.title).txt")
            try Data(note.body.utf8).write(to: url)
            return SentTransferredFile(url)
        }
    }
}

extension UTType {
    static let note = UTType(exportedAs: "com.example.note")
}

// 2. Make the source draggable
List(notes) { note in
    NoteRow(note: note)
        .draggable(note)                     // outgoing
}

// 3. Make the destination accept drops
ScrollView {
    NoteCanvas(notes: notes)
}
.dropDestination(for: Note.self) { droppedNotes, location in
    notes.append(contentsOf: droppedNotes)
    return true
}
// AND accept file URLs from Finder
.dropDestination(for: URL.self) { urls, _ in
    for url in urls where url.startAccessingSecurityScopedResource() {
        defer { url.stopAccessingSecurityScopedResource() }
        if let data = try? Data(contentsOf: url),
           let text = String(data: data, encoding: .utf8) {
            notes.append(Note(id: UUID(), title: url.lastPathComponent, body: text))
        }
    }
    return true
}
```

Gotchas to know on macOS 15+:

- `loadObject` completion can fire **after** `dropExited` for some cross-app file drops. If a `Transferable`-only drop misses Finder file URLs, add an `NSItemProvider`-based AppKit fallback wrapping the same SwiftUI view via `NSHostingView`.
- `.draggable(_)` on a row inside a `List` requires the row itself to be draggable, not the row's `Text`. Modify the outermost row container.
- Cross-app drag-drop targets must declare every type they accept. A drop destination accepting only your custom UTI will reject file URLs from Finder.

---

## Continuity — Apple's secret sauce

Continuity is the cross-device magic that "just works" between iPhone/iPad/Mac sharing an Apple ID. Failing to wire your app into it is leaving free user-perceived magic on the table.

### Handoff via `NSUserActivity`

```swift
struct ArticleView: View {
    let article: Article

    var body: some View {
        ArticleContent(article: article)
            .userActivity("com.example.reading-article", isActive: true) { activity in
                activity.title = article.title
                activity.webpageURL = article.canonicalURL
                activity.isEligibleForHandoff = true
                activity.isEligibleForSearch = true
                activity.isEligibleForPrediction = true
                try? activity.setTypedPayload(ArticleHandoffPayload(id: article.id))
            }
    }
}

// Receive on launch (in App body)
.onContinueUserActivity("com.example.reading-article") { activity in
    if let payload = try? activity.typedPayload(ArticleHandoffPayload.self) {
        router.open(.article(id: payload.id))
    }
}
```

### Universal Clipboard

Automatic for the system pasteboard. Use `NSPasteboard.general` and multi-type items; iPhone/Mac handle continuity for you.

```swift
let pb = NSPasteboard.general
pb.clearContents()
let item = NSPasteboardItem()
item.setString(note.body, forType: .string)
item.setData(try JSONEncoder().encode(note), forType: .init(UTType.note.identifier))
pb.writeObjects([item])
```

### AirDrop / Share — `ShareLink`

```swift
ShareLink(item: article.canonicalURL,
          subject: Text(article.title),
          message: Text("Reading: \(article.title)"))
```

Drops you straight into AirDrop, Messages, Mail, Notes, and every installed share extension. Free.

### Continuity Camera

```swift
import AVFoundation
import SwiftUI

@MainActor
final class CameraController: NSObject {
    private let session = AVCaptureSession()

    func start() {
        // iPhone-as-camera appears here automatically when in range
        let discovery = AVCaptureDevice.DiscoverySession(
            deviceTypes: [.builtInWideAngleCamera, .external, .continuityCamera],
            mediaType: .video, position: .unspecified)

        let preferred = AVCaptureDevice.systemPreferredCamera  // Apple chooses best
        // or filter to .continuityCamera explicitly to enforce iPhone
        guard let device = preferred ?? discovery.devices.first,
              let input = try? AVCaptureDeviceInput(device: device) else { return }
        session.beginConfiguration()
        if session.canAddInput(input) { session.addInput(input) }
        session.commitConfiguration()
        session.startRunning()
    }
}
```

### Sidecar

User-managed. You get it free as a Mac display extension. No app integration required.

---

## Services menu

System-wide submenu in every app's app-menu and right-click menu. Mac power users live in this submenu.

```swift
// 1. Declare provided services in Info.plist (NSServices array)
//
// <key>NSServices</key>
// <array>
//   <dict>
//     <key>NSPortName</key>          <string>MyApp</string>
//     <key>NSMessage</key>            <string>summarize</string>
//     <key>NSMenuItem</key>           <dict><key>default</key><string>Summarize with MyApp</string></dict>
//     <key>NSSendTypes</key>          <array><string>public.utf8-plain-text</string></array>
//     <key>NSReturnTypes</key>        <array><string>public.utf8-plain-text</string></array>
//     <key>NSRequiredContext</key>    <dict><key>NSTextContent</key><string>plain</string></dict>
//   </dict>
// </array>

// 2. Register the provider at launch
@MainActor
final class ServicesProvider: NSObject {
    @objc func summarize(_ pboard: NSPasteboard,
                         userData: String,
                         error: AutoreleasingUnsafeMutablePointer<NSString?>) {
        guard let input = pboard.string(forType: .string) else { return }
        let summary = Summarizer.run(input)
        pboard.clearContents()
        pboard.setString(summary, forType: .string)
    }
}

// In App init:
NSApp.servicesProvider = ServicesProvider()
NSUpdateDynamicServices()  // refresh after install
```

To invoke another app's service: `NSPerformService("MailViewer/Send To", pboard)`.

Gotcha: SwiftUI's `contextMenu` does NOT include the Services submenu automatically. Wrap your view in `NSViewRepresentable(NSHostingView(...))` and let AppKit's right-click handler expose it on the system menu.

---

## SwiftUI `.toolbar` on macOS

```swift
ContentView()
    .toolbar(id: "main") {
        ToolbarItem(id: "new", placement: .primaryAction) {
            Button { doc.new() } label: { Label("New", systemImage: "plus") }
                .keyboardShortcut("n", modifiers: [.command])
        }
        ToolbarSpacer(.flexible)                              // iOS 26 / macOS 26
        ToolbarItem(id: "share", placement: .primaryAction) {
            ShareLink(item: doc.exportURL)
        }
        ToolbarItem(id: "inspector", placement: .primaryAction) {
            Button { doc.showInspector.toggle() } label: { Label("Inspector", systemImage: "sidebar.right") }
        }
    }
    .toolbarRole(.editor)                                     // Mac-style item layout
    .toolbarTitleDisplayMode(.inlineLarge)
```

Liquid Glass on macOS 26 groups toolbar items into shared glass "pills." Control the grouping with `ToolbarItemGroup` and `ToolbarSpacer`.

Gotcha: fully user-customizable toolbars with the drag-to-reorder palette still need AppKit `NSToolbar` (or DSFToolbar) — SwiftUI doesn't expose the customization palette as of macOS 26.

---

## Keyboard shortcuts on every command

```swift
Button("Find") { showFind = true }
    .keyboardShortcut("f", modifiers: [.command])

Button("Find Next") { findNext() }
    .keyboardShortcut("g", modifiers: [.command])

Button("Run") { run() }
    .keyboardShortcut("r", modifiers: [.command])

Button("Build") { build() }
    .keyboardShortcut("b", modifiers: [.command])

Button("Run Without Building") { runOnly() }
    .keyboardShortcut("r", modifiers: [.command, .control])
```

A primary command without a `.keyboardShortcut` is a Mac-port red flag. Critique on sight.

---

## App Sandbox

Mandatory for Mac App Store. Optional but **strongly recommended** for Developer ID direct distribution.

When sandboxed, the app gets a per-app container at `~/Library/Containers/<bundle-id>/Data/` and loses every default capability. You opt back in via entitlements.

### Core sandbox entitlements

| Entitlement | What it grants |
|---|---|
| `com.apple.security.app-sandbox` | Master switch: on = sandboxed |
| `com.apple.security.network.client` | Outgoing network connections (URLSession, sockets) |
| `com.apple.security.network.server` | Listen for incoming connections |
| `com.apple.security.files.user-selected.read-only` | Open files via NSOpenPanel |
| `com.apple.security.files.user-selected.read-write` | Open + write via NSOpen/SavePanel |
| `com.apple.security.files.bookmarks.app-scope` | Persist user-grants across launches (app-scoped bookmark) |
| `com.apple.security.files.bookmarks.document-scope` | Persist grants tied to a document |
| `com.apple.security.files.downloads.read-write` | Access `~/Downloads` |
| `com.apple.security.assets.movies.read-write` | Access `~/Movies` |
| `com.apple.security.assets.pictures.read-write` | Access `~/Pictures` |
| `com.apple.security.assets.music.read-write` | Access `~/Music` |
| `com.apple.security.device.camera` | Camera |
| `com.apple.security.device.microphone` | Microphone (MAS variant) |
| `com.apple.security.device.audio-input` | Microphone (DevID variant) |
| `com.apple.security.device.bluetooth` | Bluetooth |
| `com.apple.security.device.usb` | USB devices |
| `com.apple.security.personal-information.location` | Location services |
| `com.apple.security.personal-information.addressbook` | Contacts |
| `com.apple.security.personal-information.calendars` | Calendars |
| `com.apple.security.personal-information.photos-library` | Photos library |
| `com.apple.security.print` | Printing |
| `com.apple.security.application-groups` | Shared container with sibling apps |

### App Groups

```xml
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.example.shared</string>
</array>
```

Sibling apps share `~/Library/Group Containers/group.com.example.shared/` and a small Keychain query namespace. macOS 15 added App Group Container Protection: non-MAS apps must explicitly entitle and register the group in the dev portal — Xcode automatic signing handles this only after you tick the capability.

---

## Hardened Runtime

Required for notarization, therefore effectively mandatory for any direct-distribution Mac app. Mac App Store builds enable it alongside sandbox automatically.

Hardened Runtime blocks DYLD injection, writable-and-executable memory, and unsigned code. You re-enable specific behaviors via "relaxation entitlements".

### Relaxation entitlements (least to most permissive)

| Entitlement | Use case | Security cost |
|---|---|---|
| `com.apple.security.cs.allow-jit` | JIT pages via `MAP_JIT` + `pthread_jit_write_*` | Lowest — preferred |
| `com.apple.security.cs.allow-unsigned-executable-memory` | Mark any page RWX (JVM-style runtimes) | Medium |
| `com.apple.security.cs.disable-executable-page-protection` | Disable W^X entirely | High — reviewers will challenge |
| `com.apple.security.cs.allow-dyld-environment-variables` | Honor `DYLD_*` env vars | Debugger/test runners |
| `com.apple.security.cs.disable-library-validation` | Load libraries signed by other Team IDs | Plugin hosts |
| `com.apple.security.cs.debugger` | Attach as debugger to other processes | LLDB-style tools |
| `com.apple.security.get-task-allow` | Allow other processes to attach (development only) | Disqualifies notarization — strip for release |
| `com.apple.security.cs.allow-relative-library-loads` | `@rpath` loads outside bundle | Niche |

Apple's published cascade is unambiguous: **pick the least permissive entitlement that works**. `allow-jit` forces all executable-page mutations through `pthread_jit_write_with_callback_np`, which makes the JIT region far harder to weaponize. If `allow-jit` is enough, never reach for `allow-unsigned-executable-memory`. Never reach for `disable-executable-page-protection` unless an Electron/Tauri/V8 dependency hard-requires it — App Review will object.

System extensions cannot relax Hardened Runtime. The single exception is `allow-jit` on platform extensions; `allow-unsigned-executable-memory` on a SysEx will fatally fail `mac_vnode_check_signature`.

---

## TCC — Transparency, Consent, and Control

TCC is the user-consent layer above sandbox. Even with an entitlement, the user must approve at first use. Decisions are stored in two SQLite databases:

- **User TCC DB**: `~/Library/Application Support/com.apple.TCC/TCC.db`
- **System TCC DB**: `/Library/Application Support/com.apple.TCC/TCC.db` (SIP-protected)

`tccd` enforces these and identifies apps by **bundle ID + code signature**. Two apps with the same bundle ID but different signatures **do not share TCC grants**. This is the root cause of "permissions disappeared after release" bugs.

### Required Info.plist usage strings

Apps **crash on first access** if the required `NS*UsageDescription` is missing. The string is required even when the entitlement is set.

| TCC service | Info.plist key | Sandbox entitlement |
|---|---|---|
| Camera | `NSCameraUsageDescription` | `com.apple.security.device.camera` |
| Microphone | `NSMicrophoneUsageDescription` | `device.microphone` (MAS) / `device.audio-input` (DevID) |
| Location | `NSLocationUsageDescription` | `personal-information.location` |
| Contacts | `NSContactsUsageDescription` | `personal-information.addressbook` |
| Calendars | `NSCalendarsUsageDescription` | `personal-information.calendars` |
| Reminders | `NSRemindersUsageDescription` | (none) |
| Photos | `NSPhotoLibraryUsageDescription` | `personal-information.photos-library` |
| Bluetooth | `NSBluetoothAlwaysUsageDescription` | `device.bluetooth` |
| Speech recognition | `NSSpeechRecognitionUsageDescription` | (none) |
| Apple Events (Automation) | `NSAppleEventsUsageDescription` | `temporary-exception.apple-events` |
| Screen Recording | (no usage string — System Settings) | (none — opt-in via Settings) |
| Full Disk Access | (none — user adds in Settings) | (none — bypasses sandbox) |
| Accessibility | (none — user adds in Settings) | (none) |
| Input Monitoring | (none — user adds in Settings) | (none) |

### TCC service identifiers — for `tccutil reset`

| Service | Identifier | When triggered |
|---|---|---|
| Microphone | `Microphone` | First mic access |
| Camera | `Camera` | First camera access |
| Screen Recording | `ScreenCapture` | First capture call |
| Full Disk Access | `SystemPolicyAllFiles` | Read protected files |
| Accessibility | `Accessibility` | AX events |
| Automation | `AppleEvents` | Send AppleEvent to another app |
| Contacts | `AddressBook` | First Contacts access |
| Calendars | `Calendar` | First Calendar access |
| Photos | `Photos` | First Photos access |
| Input Monitoring | `ListenEvent` + `PostEvent` | Global key/mouse capture |
| Developer Tools | `DeveloperTool` | Notarization-bypass tooling |
| Everything | `All` | Nuke the entire app's TCC |

The reset spell: `tccutil reset All com.example.MyApp` then quit and relaunch (sometimes logout). For Accessibility and Input Monitoring you may need `sudo tccutil reset Accessibility com.example.MyApp` to touch the system DB.

### Sequoia / Tahoe behavior changes

- **Screen Recording monthly re-prompt** since macOS 15 (Apple walked back the original weekly cadence after developer outcry). The user cannot disable the re-prompt; only MDM `forceBypassScreenCaptureAlert` profiles can suppress it. For long-running capture apps add `com.apple.developer.persistent-content-capture` to reduce nag frequency.
- **No more Control-click "Open Anyway" bypass** for unsigned/quarantined apps. The only path to launch an unsigned app is **System Settings > Privacy & Security > Open Anyway** per binary.
- **Local Network is a NetworkExtension packet filter, not TCC.** Apps that bind to `127.0.0.1` for IPC or hit Bonjour services hit it. Bare CLI binaries persist the grant unreliably; wrap CLI tools in a minimal signed `.app` bundle.

---

## Security-scoped bookmarks

For sandboxed apps that need durable access to a user-chosen file/folder across launches.

```swift
import Foundation

@MainActor
final class FolderAccessStore {
    @AppStorage("project.folder.bookmark") private var bookmarkData: Data?

    func choose() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let url = panel.url else { return }

        do {
            bookmarkData = try url.bookmarkData(
                options: [.withSecurityScope, .securityScopeAllowOnlyReadAccess],
                includingResourceValuesForKeys: nil,
                relativeTo: nil)
        } catch {
            Logger.app.error("Failed to bookmark \(url, privacy: .public): \(error)")
        }
    }

    func resolve() -> URL? {
        guard let data = bookmarkData else { return nil }
        var isStale = false
        do {
            let url = try URL(resolvingBookmarkData: data,
                              options: [.withSecurityScope],
                              relativeTo: nil,
                              bookmarkDataIsStale: &isStale)
            if isStale {
                // Ask the user to re-pick — bookmark is no longer valid
                Logger.app.info("Bookmark stale for \(url, privacy: .public)")
                return nil
            }
            return url
        } catch {
            Logger.app.error("Failed to resolve bookmark: \(error)")
            return nil
        }
    }

    func read<T>(_ work: (URL) throws -> T) throws -> T? {
        guard let url = resolve() else { return nil }
        guard url.startAccessingSecurityScopedResource() else {
            throw CocoaError(.fileReadNoPermission)
        }
        defer { url.stopAccessingSecurityScopedResource() }
        return try work(url)
    }
}
```

Two flavors:

- **App-scoped bookmark** (`com.apple.security.files.bookmarks.app-scope`): the app stores the key.
- **Document-scoped bookmark** (`com.apple.security.files.bookmarks.document-scope`): the bookmark travels with a document.

Recovery for production:

- The `ScopedBookmarksAgent` process can wedge (Sequoia 15.0 bug fixed in 15.1). Recovery: `killall ScopedBookmarksAgent`, app relaunch.
- Always check `isStale` and surface a re-pick UI.
- Past sandbox-escape findings against corrupted bookmark stores have shown the risk of app-group locations another process could rewrite — store bookmarks where you control them, in the app's own container.

---

## Code signing

| Certificate | Purpose |
|---|---|
| Apple Development | Per-developer dev / internal builds, attached to provisioning profiles |
| Apple Distribution | Mac App Store submissions |
| Developer ID Application | Direct distribution `.app` signing |
| Developer ID Installer | Direct distribution `.pkg` signing |
| Mac Installer Distribution | Mac App Store `.pkg` |

In Xcode 26 the identity is set per-configuration under **Signing & Capabilities**:

```text
Target → Signing & Capabilities → Release
  Team:                Your Team — ABCD123456
  Signing Certificate: Developer ID Application
  Provisioning Profile: Xcode Managed Profile  (or a downloaded profile)
  Bundle Identifier:    com.example.MyApp
```

Embedded helpers, system extensions, and DriverKit drivers each need their own provisioning profile with extension-specific entitlements.

Universal binaries on Tahoe: macOS 26 is **the last** release that runs natively on Intel Macs. Apple's Rosetta deprecation path:

- macOS 26 — full Rosetta support, Intel Macs supported natively.
- macOS 26.4 (Mar 2026) — warning popup on launch of any Rosetta-translated app.
- macOS 27 (Fall 2026) — Apple Silicon only, but Rosetta still fully supported.
- macOS 28 (Fall 2027) — Rosetta reduced to legacy gaming subset.
- macOS 29 (Fall 2028) — all Rosetta support ends.

Ship Universal (arm64 + x86_64) for the lifetime of macOS 26; drop x86_64 when you bump deployment target past macOS 27.

---

## Notarization

`altool` and the Xcode-13-era notarization workflow are **dead** since November 2023. Use `notarytool` exclusively.

```bash
# 1. One-time credential store (stores in your login keychain)
xcrun notarytool store-credentials "AC_NOTARY" \
  --apple-id "you@example.com" \
  --team-id "ABCD123456" \
  --password "<app-specific-password>"

# 2. Submit — zip the .app first, or submit a .dmg/.pkg
ditto -c -k --keepParent MyApp.app MyApp.zip
xcrun notarytool submit MyApp.zip --keychain-profile "AC_NOTARY" --wait

# 3. Staple the resulting ticket so Gatekeeper trusts it offline
xcrun stapler staple MyApp.app
xcrun stapler validate MyApp.app

# 4. Inspect failures
xcrun notarytool log <submission-id> --keychain-profile "AC_NOTARY" log.json

# 5. Gatekeeper sanity check
spctl --assess --type execute --verbose=4 MyApp.app
codesign --verify --deep --strict --verbose=2 MyApp.app
```

For DMG distribution: notarize the DMG (which contains a notarized + stapled inner app) and staple **both** the inner `.app` and the outer `.dmg`. A `.dmg` whose staple succeeded but whose inner `.app` was moved between submit and staple will silently bypass Gatekeeper validation and prompt the user.

Standalone CLI binaries can be notarized but **cannot be stapled** — Gatekeeper falls back to online lookup. Wrap CLI tools in a `.app` or a signed `.pkg` if you need offline-first launches.

---

## Distribution paths

### Mac App Store

- **Sandbox**: mandatory.
- **Hardened Runtime**: mandatory (auto-enabled).
- **Notarization**: not required (MAS does its own scanning).
- **Code signing**: Apple Distribution + Mac App Store provisioning profile.
- **Review times reportedly grew several-fold in 2025** — plan for multi-day waits on macOS submissions compared to faster turnaround on iOS. Community tracking blogs cover this in detail.
- Cannot auto-launch at login without `SMAppService` registration.
- Cannot install code outside the app bundle.
- Strip `com.apple.quarantine` from any bundled files before upload (`ditto`, not `cp`).
- Receipt validation via StoreKit 2; subscription / one-time purchases follow standard StoreKit flows.

### Developer ID + Notarization (direct)

- **Sandbox**: optional but strongly recommended. Most pro apps adopt sandbox.
- **Hardened Runtime**: mandatory.
- **Notarization**: mandatory.
- **Code signing**: Developer ID Application + (optional) Developer ID Installer.
- Distribute as signed `.dmg`, signed `.pkg`, or `.zip`-wrapped signed `.app`. Staple all of them.
- Auto-updates almost universally use **Sparkle** with EdDSA signatures. App Center's hosted Sparkle infra retired March 2025, so most teams self-host appcasts on S3/CloudFront.

Most pro Mac apps (BBEdit, Charles, Pixelmator, Sketch, IINA, Rectangle, Ice, Stats) go direct or hybrid because their users need non-sandboxed system access. The skill should never default to MAS without checking the app archetype.

---

## Sparkle — the de-facto auto-update framework

Sparkle 2.x is the auto-update story for direct distribution. EdDSA signatures since 2.x. Supports sandboxed apps since 2.x. DSA-only support dropped.

### Project setup

```text
Package.swift dependency:
    .package(url: "https://github.com/sparkle-project/Sparkle", from: "2.6.0")

Target dependency:
    .product(name: "Sparkle", package: "Sparkle")
```

### Info.plist

```xml
<key>SUFeedURL</key>
<string>https://updates.example.com/appcast.xml</string>
<key>SUPublicEDKey</key>
<string>YOUR_BASE64_ENCODED_PUBLIC_EDDSA_KEY</string>
<key>SUEnableAutomaticChecks</key>
<true/>
<key>SUScheduledCheckInterval</key>
<integer>86400</integer>
```

### SwiftUI integration

Sparkle exposes `SPUStandardUpdaterController` from AppKit's NSObject world. Wrap it in an `@Observable` store and bridge the KVO/Combine `canCheckForUpdates` publisher into a plain tracked property:

```swift
import Sparkle
import SwiftUI
import Combine

@MainActor
@Observable
final class UpdaterStore {
    @ObservationIgnored private let controller: SPUStandardUpdaterController
    @ObservationIgnored private var cancellable: AnyCancellable?

    var canCheckForUpdates = false

    init() {
        self.controller = SPUStandardUpdaterController(
            startingUpdater: true,
            updaterDelegate: nil,
            userDriverDelegate: nil)
        self.cancellable = controller.updater.publisher(for: \.canCheckForUpdates)
            .receive(on: RunLoop.main)
            .sink { [weak self] value in self?.canCheckForUpdates = value }
    }

    func checkForUpdates() {
        controller.updater.checkForUpdates()
    }
}

// Hook it into the menu
.commands {
    CommandGroup(after: .appInfo) {
        Button("Check for Updates...") { updaterStore.checkForUpdates() }
            .disabled(!updaterStore.canCheckForUpdates)
    }
}
```

The bridge uses Combine because Sparkle's controller is KVO-observable and the Combine `publisher(for:)` is the cleanest way to read that on the main queue. The outer type stays `@Observable` so views observe `canCheckForUpdates` through the normal SwiftUI machinery.

### Generate keys + sign updates

Sparkle 2.x uses EdDSA. The shipped tools live in the `bin/` directory of the Sparkle release archive.

```bash
# Once per project — generates an EdDSA keypair and stores the private key in the Keychain
./bin/generate_keys
# Sparkle records the public key in your build artifact lookup; keep the private key in the Keychain
# (or export to a hardware token / out-of-band secrets manager) and commit nothing.

# Per release — signs an update artifact with the EdDSA private key
./bin/sign_update MyApp-1.2.3.zip  # emits "sparkle:edSignature" + length attrs for the appcast item
```

The appcast `<item>` carries the EdDSA signature; Sparkle refuses any download whose signature doesn't validate against the public key in Info.plist.

---

## SMAppService — login items, agents, daemons

`SMAppService` (macOS 13+) replaces the deprecated `SMLoginItemSetEnabled` and `SMJobBless`. It unifies login items, user agents, and root daemons under one user-visible toggle in **System Settings > General > Login Items & Extensions**.

| Service type | Runs as | First-launch UX |
|---|---|---|
| `.mainApp` (login item) | Current user | "Background item added" notification |
| `.agent(plistName:)` | Current user | "Background item added" notification |
| `.daemon(plistName:)` | root | User authentication dialog |
| `.loginItem(identifier:)` | Current user | Same as agent, helper bundle |

### Login item

```swift
import ServiceManagement

@MainActor
final class LoginItemManager {
    func enable() {
        do {
            try SMAppService.mainApp.register()
        } catch {
            Logger.app.error("Login item register failed: \(error)")
        }
    }

    func disable() {
        do {
            try SMAppService.mainApp.unregister()
        } catch {
            Logger.app.error("Login item unregister failed: \(error)")
        }
    }

    var status: SMAppService.Status { SMAppService.mainApp.status }
}
```

### Agent (user-context helper)

```swift
let agent = SMAppService.agent(plistName: "com.example.MyAgent.plist")
try agent.register()
```

The plist lives at `Contents/Library/LaunchAgents/com.example.MyAgent.plist` inside the app bundle. The user can disable it under Login Items & Extensions.

### Daemon (root helper)

```swift
let daemon = SMAppService.daemon(plistName: "com.example.MyDaemon.plist")
try daemon.register()   // triggers an authentication prompt
```

The plist lives at `Contents/Library/LaunchDaemons/com.example.MyDaemon.plist`. Requires the SMJobBless transition manifest (`SMAuthorizedClients`, code requirement strings). The daemon binary must be signed by the same Team ID as the parent app.

Known issues:

- macOS 13.6 has had reports of disabled SMAppService items not actually stopping. Confirm with `launchctl list | grep <bundle>` after `unregister()`.
- The `sfltool dumpbtm` command lists the Background Task Management DB for diagnostics.
- Hidden launchd plists outside the app bundle are now flagged by Background Task Management and look like malware to users. New code must use `SMAppService` and ship the plist inside the bundle.

---

## XPC services — privilege separation, FFI, crash isolation

For any privileged, risky, or process-isolated operation: split it into an `XPCService.xpc` inside `Contents/XPCServices/`, exposed to the main app via `NSXPCConnection`.

### Reasons to use XPC

- **Privilege separation** — network access for a sandboxed editor.
- **Crash isolation** — rendering a risky format (RAW image, exotic PDF, untrusted JS).
- **Root operations** — via a separate daemon registered with `SMAppService.daemon`.
- **FFI to Rust / C++ helpers** — run a Rust binary in an XPC service rather than `dlopen`-ing it from the main process. The XPC boundary gives you crash isolation and a typed interface; a raw `dlopen` does not.

### Modern XPC (macOS 14+)

```swift
import Foundation

// Shared protocol — in a small framework target both sides depend on
@objc public protocol RendererServicing {
    func render(_ data: Data, options: [String: String],
                reply: @escaping (Data?, Error?) -> Void)
}

// In the main app — open a connection on demand
@MainActor
final class RendererClient {
    private lazy var connection: NSXPCConnection = {
        let c = NSXPCConnection(serviceName: "com.example.MyApp.RendererService")
        c.remoteObjectInterface = NSXPCInterface(with: RendererServicing.self)
        c.invalidationHandler = { /* surface to UI */ }
        c.interruptionHandler = { /* retry path */ }
        c.resume()
        return c
    }()

    func render(_ data: Data, options: [String: String]) async throws -> Data {
        try await withCheckedThrowingContinuation { cont in
            let proxy = connection.remoteObjectProxyWithErrorHandler { cont.resume(throwing: $0) }
            guard let proxy = proxy as? RendererServicing else {
                cont.resume(throwing: CocoaError(.xpcConnectionInterrupted)); return
            }
            proxy.render(data, options: options) { result, error in
                if let result { cont.resume(returning: result) }
                else { cont.resume(throwing: error ?? CocoaError(.xpcConnectionInvalid)) }
            }
        }
    }
}

// In the service target — the principal delegate
final class ServiceDelegate: NSObject, NSXPCListenerDelegate, RendererServicing {
    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection connection: NSXPCConnection) -> Bool {
        connection.exportedInterface = NSXPCInterface(with: RendererServicing.self)
        connection.exportedObject = self
        connection.resume()
        return true
    }
    func render(_ data: Data, options: [String: String],
                reply: @escaping (Data?, Error?) -> Void) {
        // ...
    }
}

@main
final class ServiceMain {
    static func main() {
        let delegate = ServiceDelegate()
        let listener = NSXPCListener.service()
        listener.delegate = delegate
        listener.resume()
    }
}
```

iOS 26 / macOS 26 ship modernized async XPC APIs (`XPCSession`, `XPCListener`) for Swift — prefer them over `NSXPCConnection` for greenfield, but `NSXPCConnection` remains supported.

For a sandboxed app the service is **private** to the bundle — other processes can't dial it. XPC services alone **cannot** elevate to root; for that, install a privileged helper via `SMAppService.daemon`.

Anti-pattern: `JoinExistingSession=YES` in a service's Info.plist defeats privilege separation. Avoid.

---

## Endpoint Security + System Extensions

Kernel Extensions (kexts) are deprecated. The replacement story is user-space frameworks activated as System Extensions.

| Extension type | Framework | Entitlement | Approval path |
|---|---|---|---|
| Endpoint Security | `EndpointSecurity` (C API) + `SystemExtensions` | `com.apple.developer.endpoint-security.client` | Request from Apple via System Extensions Request Form; FDA at runtime |
| Network Extension (filter, content filter, packet tunnel) | `NetworkExtension` | `com.apple.developer.networking.networkextension` + sub-keys | Sometimes by-app |
| DriverKit | `DriverKit` (USB, HID, audio, PCI, SCSI families) | `com.apple.developer.driverkit` + family-specific keys | Request from Apple |

System extensions live in `Contents/Library/SystemExtensions/`, are activated via `OSSystemExtensionRequest.activationRequest`, and the user must approve them in **System Settings > Privacy & Security** the first time. They **cannot** disable Hardened Runtime protections — the only relaxation allowed is `allow-jit`.

DriverKit on Tahoe: many existing dexts still target DriverKit 25 because the DriverKit 26 SDK isn't fully wired up in Xcode 26.2 stable. Test on hardware — `systemextensionsctl developer on` plus SIP disable is required to load unsigned dexts during development.

---

## AppKit interop — bridge without guilt

SwiftUI on macOS still has more gaps than on iOS. The "AppKit sandwich" (`NSViewRepresentable` -> `NSHostingView` -> SwiftUI) is the daily escape hatch.

### Embed AppKit in SwiftUI

```swift
import SwiftUI
import AppKit

struct CodeEditorView: NSViewRepresentable {
    @Binding var text: String

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSTextView.scrollableTextView()
        let textView = scroll.documentView as! NSTextView
        textView.isRichText = false
        textView.usesFindBar = true
        textView.delegate = context.coordinator
        textView.font = .monospacedSystemFont(ofSize: 13, weight: .regular)

        // TextKit 2 is the default on macOS 13+; do NOT touch .layoutManager
        // on macOS 26 — it silently downgrades to TextKit 1.
        // Use NSTextLayoutManager-aware APIs instead.

        return scroll
    }

    func updateNSView(_ nsView: NSScrollView, context: Context) {
        guard let textView = nsView.documentView as? NSTextView else { return }
        if textView.string != text { textView.string = text }
    }

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: CodeEditorView
        init(_ parent: CodeEditorView) { self.parent = parent }
        func textDidChange(_ notification: Notification) {
            guard let tv = notification.object as? NSTextView else { return }
            parent.text = tv.string
        }
    }
}
```

### Embed SwiftUI in AppKit

```swift
import AppKit
import SwiftUI

final class AssistantWindowController: NSWindowController {
    convenience init() {
        let host = NSHostingController(rootView: AssistantView())
        let window = NSWindow(contentViewController: host)
        window.title = "Assistant"
        window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
        self.init(window: window)
    }
}

// In a status item popover
let popover = NSPopover()
popover.contentViewController = NSHostingController(rootView: StatusPopoverView())
```

### When to bridge

Bridge when SwiftUI has a real gap:

- `NSTextView` for rich text, code editing, find-bar, ruler.
- `NSDocument` doc-based app with auto-save / file coordination.
- `NSXPC` privilege separation.
- Low-level `NSWindow` APIs (custom titlebar accessory, sheet positioning).
- Custom NSResponder behavior (Touch Bar removed, but `keyDown(with:)` still useful).
- Custom `NSAccessibility` roles (canvas-drawn controls).
- `NSToolbar` with user-customizable items (drag-to-reorder palette).

Don't bridge when SwiftUI has a modern native equivalent: `Map`, `Charts`, `WebView` (macOS 26), `PhotosPicker`, `ShareLink`.

Almost universal in macOS SwiftUI apps. The skill should treat hybrid as the norm, not the exception.

---

## NSTextView and TextKit

For rich text editing on macOS there is no SwiftUI native equivalent yet. `NSTextView` (wrapped via `NSViewRepresentable`) remains the answer. TextKit 2 (`NSTextLayoutManager`) is the modern engine — viewport-based, no glyph layer, better international support, default in NSTextView from macOS 13.

A new SwiftUI rich-text editor lands in macOS 26 (`TextEditor` with attributed-string binding), but serious code/text apps still drop to TextKit 2 or `STTextView` for full control.

**Gotcha (macOS 26):** Touching `.layoutManager` on `NSTextView` silently downgrades to TextKit 1. Audit your code. Use `NSTextLayoutManager`-aware APIs explicitly when you need layout control. If a SwiftUI-Mac code editor seems to perform poorly on Tahoe, this is the first thing to check.

---

## Mac App Store specifics

### iCloud entitlement

```xml
<key>com.apple.developer.icloud-services</key>
<array>
    <string>CloudKit</string>
    <string>CloudDocuments</string>
</array>
<key>com.apple.developer.icloud-container-identifiers</key>
<array>
    <string>iCloud.com.example.MyApp</string>
</array>
<key>com.apple.developer.icloud-container-environment</key>
<string>Production</string>
```

The `*-environment` key **must live in the entitlements file, not Info.plist** for new containers. This is the gotcha that trips up most first-time CloudKit shippers.

Container IDs must be registered in the developer portal before Xcode signing succeeds. No wildcards.

### Keychain sharing

```xml
<key>keychain-access-groups</key>
<array>
    <string>$(AppIdentifierPrefix)com.example.shared</string>
</array>
```

The leading `$(AppIdentifierPrefix)` (your Team ID) is mandatory. Catalyst apps need both this and `app-sandbox` set, otherwise `SecItem` calls return `errSecMissingEntitlement` (-34018).

### Receipt validation

Use StoreKit 2 — `Transaction.currentEntitlements`, `AppTransaction.shared`, `Product.products(for:)`. The Receipt validation library era is over for new code.

### Trial flow patterns

Mac App Store does not support free trials directly. Pattern: ship as a free app, gate features behind a non-consumable in-app purchase or a non-renewing subscription. Combined Sparkle + MAS builds use a `#if SPARKLE` flag to disable in-app update UI on the MAS variant.

---

## Direct distribution checklist

Before shipping a Developer ID build:

- [ ] **Hardened Runtime enabled.** Signing & Capabilities → Hardened Runtime capability checked. Verify with `codesign -d --entitlements - MyApp.app`.
- [ ] **Sandbox enabled** (recommended). Pick the least-permissive entitlements set that works. Audit `com.apple.security.temporary-exception.*` and document each in a comment.
- [ ] **`get-task-allow` stripped from release.** Xcode strips this on Archive automatically — confirm via `codesign -d --entitlements - MyApp.app | grep get-task-allow` (expect zero matches).
- [ ] **Notarized via `notarytool`.** Submission returned `status: Accepted`. Capture the submission UUID.
- [ ] **Stapled** via `xcrun stapler staple` (the .app AND the .dmg if you ship one).
- [ ] **`spctl --assess --type execute --verbose=4 MyApp.app`** returns `source=Notarized Developer ID`.
- [ ] **`codesign --verify --deep --strict --verbose=2 MyApp.app`** clean — no warnings.
- [ ] **All `NS*UsageDescription` strings populated** for any TCC service you touch. Localized via `InfoPlist.strings`.
- [ ] **`LSApplicationCategoryType`** set (required for MAS, conventional for DevID).
- [ ] **Sparkle appcast signed** with EdDSA. Public key embedded under `SUPublicEDKey`. Test update flow in a sandboxed build.
- [ ] **Crash reporter / analytics opt-in** (Sentry, MetricKit, custom). Default to off.
- [ ] **Universal binary** (arm64 + x86_64) for the lifetime of macOS 26.
- [ ] **App icon at all required sizes** (16, 32, 64, 128, 256, 512 + @2x variants).
- [ ] **Provenance / quarantine sane** — if you ship via DMG-from-CDN, verify Gatekeeper accepts the downloaded copy on a clean Mac before announcing.

---

## Privacy Manifest on macOS

`PrivacyInfo.xcprivacy` and the Required Reason API regime currently target iOS, iPadOS, tvOS, visionOS, and watchOS. **macOS apps are formally exempt as of 2026.**

Ship one anyway when:

- The same SwiftPM package is consumed by an iOS target — the iOS bar applies via the shared dependency.
- You build third-party SDKs other Mac apps embed.
- You want to stay ahead of Apple's known extend-platform-by-platform pattern.

Required Reason API codes are the same as iOS. Keys to fill:

- `NSPrivacyTracking`, `NSPrivacyTrackingDomains`
- `NSPrivacyCollectedDataTypes`
- `NSPrivacyAccessedAPITypes` — UserDefaults access, system boot time, file timestamps, disk space, active keyboard, etc.

---

## Print support

```swift
// SwiftUI — simple cases
ContentView()
    .toolbar {
        ToolbarItem(placement: .primaryAction) {
            Button("Print", systemImage: "printer") { showPrintPanel = true }
                .keyboardShortcut("p", modifiers: [.command])
        }
    }

// AppKit — full control
@MainActor
func print(_ view: some View) {
    let host = NSHostingView(rootView: view)
    host.frame = NSRect(x: 0, y: 0, width: 612, height: 792)   // US Letter
    let printInfo = NSPrintInfo.shared
    printInfo.orientation = .portrait
    printInfo.topMargin = 36
    printInfo.bottomMargin = 36
    printInfo.leftMargin = 54
    printInfo.rightMargin = 54
    printInfo.horizontalPagination = .fit
    printInfo.verticalPagination = .automatic
    let op = NSPrintOperation(view: host, printInfo: printInfo)
    op.showsPrintPanel = true
    op.showsProgressPanel = true
    op.run()
}

// For PDF-shaped output
let pdfData = host.dataWithPDF(inside: host.bounds)
```

macOS print integration includes Save as PDF, Open in Preview, send to Printer — a complete subsystem iOS doesn't have. Document apps should always wire this up.

---

## Quick Look / Share / Action / Finder Sync extensions

App Extension bundle types live alongside the main app target.

| Extension | Bundle target | Use case |
|---|---|---|
| Quick Look Preview | `QLPreviewingController` + `QLThumbnailProvider` | Spacebar previews in Finder for your custom UTI |
| Share | `NSExtension` "Share" point + `NSExtensionPrincipalClass` | Appear in any app's Share menu |
| Action | "Action" extension point | Selection-driven transforms (image filter, text tool) |
| Finder Sync | `FIFinderSync` subclass | Badges + context menu items in Finder for monitored paths |
| Markup | "Markup" extension point | Image/PDF annotation inside Mail, Photos, Preview |
| Photos | "Photos Editing" / "Photos Project" | Plug into Photos.app workflow |
| Mail | MailKit (macOS 12+) | Mail compose / message processing |

All extensions share the same App Group entitlement, registered in the dev portal. Quick Look extensions need a UTI you own (`UTExportedTypeDeclarations` in the host app's Info.plist).

Cloud-storage clients and version-controlled folder tools live or die on `FIFinderSync` — badges and right-click actions in Finder for your monitored paths.

---

## Touch Bar — deprecated

`NSTouchBar` API still exists but the hardware is gone and Apple has shipped no new Touch Bar features in years. Tahoe updates have been quietly breaking it. **Don't ship Touch Bar code in new apps.** If you have legacy code, plan its removal; if you have a specific Touch-Bar-only audience, you already know who you are.

---

## Trackpad gestures and the scroll wheel

`NSGestureRecognizer` family (`NSPanGestureRecognizer`, `NSMagnificationGestureRecognizer`, `NSRotationGestureRecognizer`) and `NSResponder.scrollWheel(with:)` are still the right tool for trackpad-heavy canvases (image editors, CAD, maps), and for modifier-aware zoom (`event.hasPreciseScrollingDeltas`).

SwiftUI's own gesture system doesn't expose finger count or trackpad pressure on macOS — drop to AppKit when you need them.

```swift
struct CanvasView: NSViewRepresentable {
    func makeNSView(context: Context) -> CanvasNSView { CanvasNSView() }
    func updateNSView(_ nsView: CanvasNSView, context: Context) {}
}

final class CanvasNSView: NSView {
    override func scrollWheel(with event: NSEvent) {
        if event.modifierFlags.contains(.command) {
            // Zoom — gestural pinch on trackpad
        } else if event.hasPreciseScrollingDeltas {
            // Trackpad two-finger pan
        } else {
            // Mouse wheel — coarse units
        }
    }
    override var acceptsFirstResponder: Bool { true }
}
```

---

## Cross-references

- iOS-specific platform capabilities → `references/ios-platform.md`.
- Persistence choice on Mac (SwiftData vs Core Data vs SQLiteData vs GRDB) → `references/persistence.md`.
- Modern API replacements (notarytool, SMAppService, TextKit 2) → `references/modern-api.md`.
- Anti-patterns canonical list → `references/anti-patterns.md`.

---

## Anti-patterns

Critique on sight when reviewing a Mac SwiftUI / AppKit codebase in 2026:

- **`SMLoginItemSetEnabled` or `SMJobBless` in new code.** Both deprecated. Use `SMAppService.mainApp.register()`, `SMAppService.agent(plistName:)`, or `SMAppService.daemon(plistName:)`.
- **`altool` for notarization.** Dead since November 2023. Use `xcrun notarytool submit --keychain-profile ... --wait`.
- **Kernel extensions (kexts).** Deprecated. Replace with Endpoint Security, Network Extension, or DriverKit running as System Extensions.
- **iPad → Mac Catalyst "just works" framing.** Catalyst is a starting point, not a finish line; menus, keyboard shortcuts, drag-drop, NSToolbar, NSDocument all need separate work.
- **Touch Bar code.** Hardware gone, software bit-rotting. Strip on encounter unless explicitly scoped to legacy MacBook Pro support.
- **`Csqlite3` direct usage in new Mac apps.** Use SwiftData, Core Data, or GRDB. Direct sqlite3 binding is a recipe for thread-safety bugs and missed CloudKit sync.
- **Burger-menu navigation on Mac.** Use main menu + keyboard shortcuts + sidebars + inspectors. A hamburger icon on a Mac toolbar is a port tell.
- **Missing keyboard shortcut on a primary command.** Critique on sight. Every menu item, every toolbar button, every important inline action.
- **`NSTextView.layoutManager` access in macOS 26.** Silently downgrades to TextKit 1 with no warning. Use `NSTextLayoutManager`-aware APIs instead.
- **`SMLoginItemSetEnabled`-era LaunchAgents installed outside the app bundle.** Background Task Management now flags these as malware-shaped. Move the plist inside `Contents/Library/LaunchAgents/` and register via `SMAppService.agent(plistName:)`.
- **A `MenuBarExtra` app that needs popover state.** No first-party state API. Use `MenuBarExtraAccess`, `FluidMenuBarExtra`, or drop to `NSStatusItem`.
- **A doc-style app without `DocumentGroup` + `FileDocument` / `ReferenceFileDocument`.** You'll re-implement Open/Save/Recent/Versions badly.
- **A SwiftUI Mac app with empty `.commands { }`.** The result is File menu = "Close Window" only. Indistinguishable from a sloppy Catalyst port.
- **Hardcoded paths assuming iOS-style private app container** on Mac. The sandbox container is at `~/Library/Containers/<bundle-id>/Data/` and the user's Documents folder is **not** inside it. NSOpenPanel + security-scoped bookmarks or bust.
- **Storing security-scoped bookmark data in an app-group location another process can rewrite.** Store bookmarks in the app's own container, not somewhere a helper or extension can tamper with them.
- **`com.apple.security.cs.disable-executable-page-protection`** when `allow-jit` would suffice. App Review will object, and you've given attackers RWX pages.
- **`@unchecked Sendable`** as a quick "fix" for Swift 6 warnings on a Mac actor-isolated type, without a comment explaining the synchronization mechanism. Prefer explicit isolation or `final class { let ... } : Sendable`.
- **`.presentationBackground(.thinMaterial)`** on iOS 26 sheets carried over to a Mac SwiftUI view — suppresses the new sheet style.
- **A FAB (floating action button) in the corner of a Mac window.** Indistinguishable from an iPad port.
- **Pull-to-refresh on a Mac scroll view.** Mac scroll views don't pull-to-refresh; use a refresh toolbar button + keyboard shortcut.
- **A non-`@MainActor` `NSStatusItem` controller.** Crashes on first access; `NSStatusItem` is main-thread-only.
- **Asking for Local Network when you bind to `127.0.0.1` for IPC.** Sequoia/Tahoe requires the NetworkExtension Local Network permission, which is keyed on main-executable UUID; re-signing drops the prior allow rule and you vanish from the Local Network list in System Settings. Use an XPC service over Mach instead, or surface a "needs Local Network" UI when the bind fails.
