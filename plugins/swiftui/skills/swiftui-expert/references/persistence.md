# Persistence

Swift 6.3 / iOS 26 / Xcode 26. The persistence layer is where SwiftUI apps die in production. Pick the right tool for the archetype, ship `VersionedSchema` from day one, keep secrets out of `UserDefaults`, and investigate suspension-related database terminations with lifecycle evidence before choosing a mitigation.

Cross-reference: `state-and-observation.md` for the full `@AppStorage` + `@Observable` workaround and the `access(keyPath:)` / `withMutation(keyPath:)` pattern.

---

## The decision — pick by archetype

| Persistence | Use when |
|---|---|
| **SwiftData** | iOS 17+ • small/simple data model • private CloudKit only • greenfield • willing to ship `VersionedSchema` from v1 |
| **Core Data** | shared/public CloudKit • large relational model • existing investment • complex predicates • `NSFetchedResultsController` |
| **SQLiteData (Point-Free)** | want SwiftData ergonomics + shared/public CloudKit • OK with newer ecosystem |
| **GRDB** | heavy SQL • performance critical • large datasets • fine-grained control |
| **Realm** | cross-platform sync via Atlas Device Sync • legacy Realm code |
| **File-based JSON** | small flat data, no queries, no relationships |
| **UserDefaults** | small non-sensitive prefs only — flags, last selection, theme |
| **Keychain** | small secrets and key material — tokens, credentials, OAuth refresh tokens, API keys, encryption keys |
| **Protected file/database storage** | larger personal or sensitive records, with appropriate Data Protection class and optional field/file encryption |

Defaults: SwiftData for a new consumer app on iOS 17+ that fits in one model graph. Core Data the moment you need a shared CloudKit zone. Keychain for small secrets and keys; protected files/databases for larger sensitive data.

---

## SwiftData production rules (most critical)

SwiftData is the default for new apps in 2026 and the source of the most production scars. The rules below separate broad defaults from mitigations that require evidence from the app's workload and lifecycle.

### Rule 1 — Wrap every model in `VersionedSchema` from v1.0.0

This is the single biggest trap. If your initial release uses bare `@Model` classes (no `VersionedSchema`), a future migration plan can produce `Cannot use staged migration with an unknown model version`. The store has no version checksum, so SwiftData doesn't know where to start. The escape path is shipping a bridge release that wraps existing models in `VersionedSchema V1` with no changes, waiting for users to update, *then* shipping V2 — and users who skip the bridge can still crash.

Ship the wrapper from day one, even with no migration planned:

```swift
import SwiftData

enum AppSchemaV1: VersionedSchema {
    static let versionIdentifier = Schema.Version(1, 0, 0)
    static var models: [any PersistentModel.Type] {
        [Item.self, Tag.self]
    }

    @Model
    final class Item {
        var title: String = ""
        var createdAt: Date = .now
        var isComplete: Bool = false
        @Relationship(deleteRule: .nullify, inverse: \Tag.items)
        var tags: [Tag]? = []
        init(title: String) { self.title = title }
    }

    @Model
    final class Tag {
        var name: String = ""
        var items: [Item]? = []
        init(name: String) { self.name = name }
    }
}

// Keep call-site code clean by aliasing to the current version.
typealias Item = AppSchemaV1.Item
typealias Tag  = AppSchemaV1.Tag
```

The `typealias` lets the rest of the code use `Item` without leaking the schema namespace. When you ship V2, you re-alias to `AppSchemaV2.Item`.

### Rule 2 — Protect bounded critical work that can cross suspension

iOS can terminate a process with `0xdead10cc` when it is suspended while holding a file/database lock. The code identifies a lifecycle/locking problem; it does not prove every SwiftData fetch or save needs a background task.

Keep ordinary fetches and saves short, avoid starting nonessential work as the scene backgrounds, and use `BGTaskScheduler` for work intended to run in the background. Use `beginBackgroundTask` only for bounded critical work that began in the foreground and may need a short grace period to finish after a background transition:

```swift
@MainActor
func finishCriticalSave(_ context: ModelContext) throws {
    let app = UIApplication.shared
    var taskID: UIBackgroundTaskIdentifier = .invalid
    taskID = app.beginBackgroundTask(withName: "SwiftDataSave") {
        // The grace period expired. End the assertion; design larger work to
        // be cancellable/checkpointed or move it to BGTaskScheduler.
        app.endBackgroundTask(taskID)
        taskID = .invalid
    }
    defer {
        if taskID != .invalid { app.endBackgroundTask(taskID) }
    }
    try context.save()
}
```

Treat this as a targeted mitigation, not a wrapper around every disk touch. Confirm the termination in Organizer/Sentry/Crashlytics, identify the operation crossing suspension, shorten or defer it first, and add a background-task assertion only where a bounded critical section genuinely needs completion time. A background task cannot make unbounded work safe.

Make it observable: Sentry/Crashlytics rules to flag `0xdead10cc` so you catch regressions.

### Rule 3 — CloudKit sync is private-database-only in 2026

Native SwiftData CloudKit sync still only supports the **private** database. There's no `databaseScope` analog on `ModelConfiguration`.

For CloudKit-synced models:

- **Every property has a default or is optional.** Non-optional, no-default properties block sync silently — records reject server-side.
- **Every relationship is optional.** Inverse relationships must declare both sides.
- **Never `@Attribute(.unique)` on a synced model.** CloudKit can't enforce uniqueness across devices and the sync engine errors out.
- **No non-optional `Transformable`** — non-optional transformable attributes have been reported to block future migrations. Make them optional from day one.

```swift
enum AppSchemaV1: VersionedSchema {
    static let versionIdentifier = Schema.Version(1, 0, 0)
    static var models: [any PersistentModel.Type] { [Note.self] }

    @Model
    final class Note {
        // Defaults required for CloudKit.
        var title: String = ""
        var body: String = ""
        var createdAt: Date = .now
        // Relationship optional — required for CloudKit.
        @Relationship(deleteRule: .cascade) var attachments: [Attachment]? = []
        // NO @Attribute(.unique) here.

        init(title: String, body: String) {
            self.title = title
            self.body = body
        }
    }
}

@main
struct MyApp: App {
    let container: ModelContainer = {
        let schema = Schema(versionedSchema: AppSchemaV1.self)
        let config = ModelConfiguration(
            schema: schema,
            cloudKitDatabase: .private("iCloud.com.example.MyApp")
        )
        return try! ModelContainer(for: schema, configurations: config)
    }()

    var body: some Scene {
        WindowGroup { ContentView() }
            .modelContainer(container)
    }
}
```

For shared / public CloudKit, drop to `NSPersistentCloudKitContainer` or use `SQLiteData`.

### Rule 4 — `@Query` only works inside SwiftUI views

`@Query` is a `DynamicProperty`. It only composes inside `View`, `App`, `Scene`. Putting it inside an `@Observable` class will not work — there is no view to subscribe and re-render.

For non-view code (services, model actors, view models when you use them), use `FetchDescriptor<T>` + `ModelContext.fetch()`:

```swift
@Observable
final class ItemService {
    @ObservationIgnored private let context: ModelContext
    init(context: ModelContext) { self.context = context }

    func loadIncomplete() throws -> [Item] {
        var descriptor = FetchDescriptor<Item>(
            predicate: #Predicate { !$0.isComplete },
            sortBy: [SortDescriptor(\.createdAt, order: .reverse)]
        )
        descriptor.fetchLimit = 200
        descriptor.propertiesToFetch = [\.title, \.createdAt]
        descriptor.relationshipKeyPathsForPrefetching = [\.tags]
        return try context.fetch(descriptor)
    }
}
```

Dynamic filters in views require pushing `@Query` into a child view that takes the filter as an init parameter:

```swift
struct ItemList: View {
    let search: String
    @Query private var items: [Item]

    init(search: String) {
        self.search = search
        _items = Query(filter: #Predicate<Item> { $0.title.localizedStandardContains(search) })
    }

    var body: some View {
        List(items) { Text($0.title) }
    }
}
```

### Rule 5 — MVVM doesn't mesh with SwiftData

`@Query` can't live in a view model. `ModelContext` comes from the environment. Wrapping SwiftData in MVVM bloats the view model and reintroduces the indirection SwiftData was designed to eliminate.

> "Anyone else feel like MVVM doesn't mesh well with SwiftData? ViewModels get crazy bloated or the views get too tied to the data layer." — a high-score Reddit thread on the topic

Use MV (view IS the view model) with SwiftData. If you genuinely need orchestration (state machine + retries + pagination), wrap the SwiftData layer with a service that exposes `FetchDescriptor` queries and accept that the service is not a per-screen view model — it's an `@Observable` store.

### Rule 6 — `@ModelActor` for off-main mutations

```swift
@ModelActor
actor ImportActor {
    func importPayloads(_ payloads: [Payload]) throws -> [PersistentIdentifier] {
        var inserted: [PersistentIdentifier] = []
        for payload in payloads {
            let item = Item(title: payload.title)
            modelContext.insert(item)
            inserted.append(item.persistentModelID)
        }
        try modelContext.save()
        return inserted
    }
}

// Usage from @MainActor view:
@Environment(\.modelContext) private var context

func importTapped(_ payloads: [Payload]) {
    Task {
        let actor = ImportActor(modelContainer: context.container)
        let ids = try await actor.importPayloads(payloads)
        // Refetch on main with the ids — never pass model instances across actors.
        let descriptor = FetchDescriptor<Item>(
            predicate: #Predicate { ids.contains($0.persistentModelID) }
        )
        let items = try context.fetch(descriptor)
        // … update view state
    }
}
```

`ModelContainer` is `Sendable` and is the only thing safe to pass across actors. **Never pass `PersistentModel` instances** — pass `PersistentIdentifier` and refetch on the other side. Saving from a background actor fires `ModelContext.didSave` on that actor's queue; observe with care or hop back via `await MainActor.run`.

### Rule 7 — `#Index<Entity>` on query hot paths (iOS 18+)

```swift
@Model
final class Item {
    var timestamp: Date = .now
    var isComplete: Bool = false
    var priority: Int = 0
    var title: String = ""

    #Index<Item>([\.timestamp])
    #Index<Item>([\.isComplete, \.priority])
}
```

Add indexes for measured query hot paths, using query shape, selectivity, ordering, and store size to choose single or composite indexes. Do not index every filtered/sorted property: indexes consume storage and slow inserts/updates. `#Index` does **not** back-deploy to iOS 17 — version-gate model definitions or maintain an iOS 17-compatible schema when supporting both.

### Rule 8 — `#Predicate` not `NSPredicate` strings

```swift
// Correct: compile-time checked, ships to SQLite.
let descriptor = FetchDescriptor<Item>(
    predicate: #Predicate { $0.isComplete == false && $0.priority > 5 }
)

// Wrong: NSPredicate string. No type checking, hits .filter in Swift.
// Don't.
```

Order clauses most-restrictive-first. Favor `Int` / `Bool` / enum comparisons before string comparisons. Filter in the predicate, not in Swift — `items.filter { $0.isActive }` after a fetch loads the whole table.

### Production scars (community testimony)

> "I have a production app on the App Store since over a year with 2K monthly users and good revenue. However I am so sick of SwiftData. Predicates are limited, Performance is bad, iCloud Sync is black magic and I am hitting borders with my models." — production-app developer Reddit testimony

> "First time SwiftUI/SwiftData user recently. On one hand, it's amazing… On the other hand, I realize that with all the observable things that come with it, there is a huge performance cost. Any little change even in navigation, and certainly in an object that cascades into relationship upon relationship, can trigger crazy updates." — another community testimony on SwiftData performance

The takeaways: SwiftData performance degrades with relationship cascades and large stores. Mitigations are `#Index`, `propertiesToFetch`, `relationshipKeyPathsForPrefetching`, `fetchLimit`, and pushing heavy work to `@ModelActor` with DTOs.

---

## Migrations

A `SchemaMigrationPlan` declares an ordered list of `VersionedSchema` and the stages to migrate between them.

```swift
enum AppSchemaV2: VersionedSchema {
    static let versionIdentifier = Schema.Version(2, 0, 0)
    static var models: [any PersistentModel.Type] { [Item.self, Tag.self, Folder.self] }

    @Model
    final class Item {
        var title: String = ""
        var createdAt: Date = .now
        var isComplete: Bool = false
        var notes: String = "" // NEW — default makes this lightweight.
        @Relationship(deleteRule: .nullify, inverse: \Tag.items)
        var tags: [Tag]? = []
        @Relationship(deleteRule: .nullify, inverse: \Folder.items)
        var folder: Folder? = nil // NEW relationship — optional, lightweight.
        init(title: String) { self.title = title }
    }
    @Model final class Tag { /* unchanged */ var name: String = ""; var items: [Item]? = []; init(name: String){self.name=name} }
    @Model final class Folder { var name: String = ""; var items: [Item]? = []; init(name: String){self.name=name} }
}

typealias Item = AppSchemaV2.Item
typealias Tag  = AppSchemaV2.Tag
typealias Folder = AppSchemaV2.Folder

enum AppMigrationPlan: SchemaMigrationPlan {
    static var schemas: [any VersionedSchema.Type] {
        [AppSchemaV1.self, AppSchemaV2.self]
    }
    static var stages: [MigrationStage] {
        [
            // Lightweight: added properties with defaults, added models, added optional relationships.
            .lightweight(fromVersion: AppSchemaV1.self, toVersion: AppSchemaV2.self)
        ]
    }
}

// Apply at container creation:
let container = try ModelContainer(
    for: Schema(versionedSchema: AppSchemaV2.self),
    migrationPlan: AppMigrationPlan.self,
    configurations: ModelConfiguration(cloudKitDatabase: .private("iCloud.com.example.MyApp"))
)
```

Custom stage when shape changes (rename / split / type change):

```swift
.custom(
    fromVersion: AppSchemaV1.self,
    toVersion: AppSchemaV2.self,
    willMigrate: { context in
        // Pre-migration hook. Read V1 shape, prepare data.
    },
    didMigrate: { context in
        // Post-migration. Backfill new fields, normalize, etc.
        let items = try context.fetch(FetchDescriptor<AppSchemaV2.Item>())
        for item in items where item.notes.isEmpty {
            item.notes = "Imported from v1"
        }
        try context.save()
    }
)
```

### Migration gotchas

- **Skip-version migrations don't work.** Users must update through each version — or you ship an interim bridge release. Two paths: declare every consecutive pair in `schemas: [V1, V2, V3]` so SwiftData walks the chain; OR force a bridge release when you can't represent the gap.
- **New non-optional properties must have defaults** or a custom stage that backfills.
- **Non-optional `Transformable` has been reported to block future migrations.** Make transformable values optional from day one.
- **Release builds can crash on schemas split across files** in cases where Debug builds work fine. Keep related model + sort logic together if you hit this.
- **Duplicate version identifiers** across `VersionedSchema` enums cause migration loops. Bump strictly: 1.0.0 → 1.1.0 → 2.0.0.

---

## Core Data

Still the right call for: shared/public CloudKit, >10 models, complex predicates, `NSCompoundPredicate`, `NSFetchedResultsController`, or any existing investment. Apple shipped no new Core Data features at WWDC25 — treat it as maintenance mode, but a stable one.

```swift
import CoreData
import SwiftUI

final class PersistenceController {
    static let shared = PersistenceController()
    let container: NSPersistentCloudKitContainer

    init(inMemory: Bool = false) {
        container = NSPersistentCloudKitContainer(name: "MyApp")
        if inMemory {
            container.persistentStoreDescriptions.first?.url = URL(fileURLWithPath: "/dev/null")
        }
        // Private + shared + public — all three scopes available with Core Data.
        guard let desc = container.persistentStoreDescriptions.first else { fatalError("No store description") }
        desc.cloudKitContainerOptions = NSPersistentCloudKitContainerOptions(
            containerIdentifier: "iCloud.com.example.MyApp"
        )
        desc.setOption(true as NSNumber, forKey: NSPersistentHistoryTrackingKey)
        desc.setOption(true as NSNumber, forKey: NSPersistentStoreRemoteChangeNotificationPostOptionKey)
        container.loadPersistentStores { _, error in
            if let error { fatalError("Core Data load failed: \(error)") }
        }
        container.viewContext.automaticallyMergesChangesFromParent = true
        container.viewContext.mergePolicy = NSMergeByPropertyObjectTrumpMergePolicy
    }
}

@main
struct MyApp: App {
    let persistence = PersistenceController.shared
    var body: some Scene {
        WindowGroup { ContentView() }
            .environment(\.managedObjectContext, persistence.container.viewContext)
    }
}

struct ItemList: View {
    @FetchRequest(
        sortDescriptors: [SortDescriptor(\Item.createdAt, order: .reverse)],
        predicate: NSPredicate(format: "isComplete == NO"),
        animation: .default
    ) private var items: FetchedResults<Item>

    var body: some View {
        List(items) { item in Text(item.title ?? "") }
    }
}
```

`@FetchRequest` is the Core Data equivalent of `@Query`. The CloudKit container identifier and capabilities live in the **entitlements file**, not Info.plist — Push Notifications + Remote notifications background mode + iCloud (CloudKit) with the container ID. Missing entitlement = silent sync failure.

For sharing: `NSPersistentCloudKitContainer.share(_:to:)` plus `CKShare`. SwiftUI exposes `CloudSharingView` for the system share UI. This is the only first-party path to CloudKit collaboration in 2026.

---

## SQLiteData (Point-Free)

Released 2026. GRDB under the hood, SwiftUI-shaped API on top. Supports private + shared + public CloudKit — the killer feature SwiftData lacks. Production maturity is still settling; pick it knowing you're early.

> "We've been working hard on a suite of tools that can act as a replacement for SwiftData. It uses SQLite under the hood (via GRDB) and it can seamlessly synchronize your user's data across all of their devices, and it is even possible to share records with other users for collaboration." — announcement from the Point-Free team

```swift
import SQLiteData
import StructuredQueries

@Table struct Item {
    let id: Int
    var title: String
    var isComplete: Bool
    var createdAt: Date
}

@main
struct MyApp: App {
    init() {
        prepareDependencies {
            $0.defaultDatabase = try! DatabaseQueue(/* config + CloudKit sync */)
        }
    }
    var body: some Scene {
        WindowGroup { ContentView() }
    }
}

struct ItemList: View {
    @FetchAll(Item.where { !$0.isComplete }.order(by: \.createdAt, .desc))
    var items

    var body: some View {
        List(items) { Text($0.title) }
    }
}
```

The query DSL (StructuredQueries) is more composable than `#Predicate` and tops out at SQL-level expressiveness. Pick SQLiteData when the SwiftData CloudKit private-only ceiling is the blocker.

---

## GRDB

Heavy SQL, performance critical, large datasets. The isowords-style stack: `GRDB` + `Csqlite3` + `swift-dependencies`. Type-safe records via the `Codable + FetchableRecord + PersistableRecord` protocols.

```swift
import GRDB

struct Item: Codable, FetchableRecord, PersistableRecord {
    static let databaseTableName = "items"
    let id: Int64?
    var title: String
    var isComplete: Bool
    var createdAt: Date

    enum Columns {
        static let id = Column(CodingKeys.id)
        static let title = Column(CodingKeys.title)
        static let isComplete = Column(CodingKeys.isComplete)
        static let createdAt = Column(CodingKeys.createdAt)
    }
}

final class AppDatabase {
    private let dbWriter: any DatabaseWriter

    init(_ dbWriter: any DatabaseWriter) throws {
        self.dbWriter = dbWriter
        var migrator = DatabaseMigrator()
        migrator.registerMigration("createItems") { db in
            try db.create(table: "items") { t in
                t.autoIncrementedPrimaryKey("id")
                t.column("title", .text).notNull()
                t.column("isComplete", .boolean).notNull().defaults(to: false)
                t.column("createdAt", .datetime).notNull()
                t.column("isComplete_createdAt", .text).indexed()
            }
        }
        try migrator.migrate(dbWriter)
    }

    func incompleteItems() throws -> [Item] {
        try dbWriter.read { db in
            try Item.filter(Item.Columns.isComplete == false)
                    .order(Item.Columns.createdAt.desc)
                    .fetchAll(db)
        }
    }
}
```

Reach for GRDB when SwiftData's predicate ceiling bites, when you need window functions / CTEs / FTS5, or when you're shipping a sync layer of your own.

---

## Realm

Atlas Device Sync gives you cross-platform sync (iOS, Android, web). MongoDB owns it now — long-term steward uncertainty is worth weighing. The open-source WWDC.app codebase is the canonical production reference and ships a custom Realm fork. Use only if Atlas sync is a hard requirement; SwiftData and SQLiteData are the default Apple-platform choices.

---

## UserDefaults limits

For: theme, accent color, hasOnboarded, last-selected tab, feature flags, layout preferences. Small Codable values are OK.

**Never:** tokens, OAuth refresh tokens, API keys, secret identifiers, biometric templates, or sensitive personal data. `UserDefaults` is preference storage, not a security boundary. Use Keychain for small secrets/key material and appropriately protected files or databases for larger personal records.

```swift
// Simple flags inside a view — fine.
struct SettingsView: View {
    @AppStorage("hasCompletedOnboarding") private var onboarded = false
    @AppStorage("themeStyle") private var theme: ThemeStyle = .system
    var body: some View {
        Form {
            Toggle("Onboarded", isOn: $onboarded)
            Picker("Theme", selection: $theme) { /* … */ }
        }
    }
}

// Slightly richer Codable + UserDefaults pattern.
struct LayoutPrefs: Codable, Equatable {
    var density: Density = .comfortable
    var showAvatars: Bool = true
}

extension UserDefaults {
    private static let layoutKey = "layoutPrefs"

    var layout: LayoutPrefs {
        get {
            guard let data = data(forKey: Self.layoutKey),
                  let value = try? JSONDecoder().decode(LayoutPrefs.self, from: data)
            else { return LayoutPrefs() }
            return value
        }
        set {
            let data = try? JSONEncoder().encode(newValue)
            set(data, forKey: Self.layoutKey)
        }
    }
}
```

### `@AppStorage` + `@Observable` — the silent footgun

`@AppStorage` is a SwiftUI `DynamicProperty`. It only works as expected inside `View`, `App`, `Scene`. Putting it directly inside an `@Observable` class compiles but **does not trigger view updates**:

```swift
// ❌ DOES NOT NOTIFY
@Observable final class Settings {
    @AppStorage("theme") var theme = "dark"
}

// ❌ ALSO DOES NOT NOTIFY (just clearer intent)
@Observable final class Settings {
    @ObservationIgnored @AppStorage("theme") var theme = "dark"
}
```

The verified-against-IceCubesApp-main fix: stored `var` + `didSet` over a plain (non-`@Observable`) inner `Storage` class, seeded by a private `init`:

```swift
@MainActor
@Observable
public final class ThemeStore {
    final class Storage {
        @AppStorage("theme.accent") var accent: AccentChoice = .blue
    }

    private let storage = Storage()

    public var accent: AccentChoice {
        didSet { storage.accent = accent }
    }

    private init() {
        accent = storage.accent
    }

    public static let shared = ThemeStore()
}
```

The outer `accent` is a stored property — the `@Observable` macro instruments it normally, so views observing `themeStore.accent` re-render on change. `didSet` mirrors to `storage.accent`, which writes UserDefaults via `@AppStorage`'s wrappedValue setter. The `init` seeds the outer value at startup.

For multiple persisted properties, see `state-and-observation.md` § The `@AppStorage` trap for the full deep dive, including the manual `access(keyPath:)`/`withMutation(keyPath:)` alternative for one-off persisted values.

---

## Keychain

Use Keychain for small secrets and key material: tokens, credentials, OAuth refresh tokens, API keys, encryption keys, and biometric-gated secrets. Do not use it as a general-purpose PII database; protect larger personal records in files or a database and keep only the encryption key in Keychain when application-level encryption is required. Popular wrappers such as `KeychainAccess` and `KeychainSwift` remove `SecItemCopyMatching` boilerplate. Alternatively, write a small typed wrapper:

```swift
import Security
import Foundation

enum KeychainError: Error { case status(OSStatus) }

struct KeychainStore {
    let service: String
    let accessGroup: String?

    init(service: String = Bundle.main.bundleIdentifier ?? "app",
         accessGroup: String? = nil) {
        self.service = service
        self.accessGroup = accessGroup
    }

    func set(_ value: String, for account: String, accessibility: CFString = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly) throws {
        let data = Data(value.utf8)
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
        if let accessGroup { query[kSecAttrAccessGroup as String] = accessGroup }

        // Delete then add — simplest correct upsert.
        SecItemDelete(query as CFDictionary)
        query[kSecValueData as String] = data
        query[kSecAttrAccessible as String] = accessibility
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainError.status(status) }
    }

    func get(_ account: String) throws -> String? {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        if let accessGroup { query[kSecAttrAccessGroup as String] = accessGroup }

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw KeychainError.status(status) }
        guard let data = item as? Data, let value = String(data: data, encoding: .utf8) else {
            return nil
        }
        return value
    }

    func delete(_ account: String) throws {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
        if let accessGroup { query[kSecAttrAccessGroup as String] = accessGroup }
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.status(status)
        }
    }
}
```

### Accessibility flags

Pick the most restrictive that still lets your code run:

- `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` — secrets that should not sync via iCloud Keychain, only available when device is unlocked.
- `kSecAttrAccessibleAfterFirstUnlock` — tokens needed by background tasks / silent push handlers (most apps).
- `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` — same, never synced.
- `kSecAttrAccessibleWhenUnlocked` — only while unlocked, syncs via iCloud Keychain (rare).

Biometric gating uses `kSecAttrAccessControl`:

```swift
var error: Unmanaged<CFError>?
guard let access = SecAccessControlCreateWithFlags(
    nil,
    kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
    .userPresence, // or .biometryCurrentSet for stricter binding
    &error
) else { throw KeychainError.status(errSecParam) }

let attrs: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrService as String: "biometric.token",
    kSecAttrAccount as String: "user.session",
    kSecValueData as String: tokenData,
    kSecAttrAccessControl as String: access
]
SecItemAdd(attrs as CFDictionary, nil)
```

### Sharing across app + extensions

Add a Keychain Sharing capability and set the `keychain-access-groups` entitlement. Then construct the store with `accessGroup: "<TEAM_ID>.com.example.MyApp.shared"`. Widgets, share extensions, and Siri intents read the same item without re-auth.

### Don't roll your own crypto

Use `CryptoKit` for ChaChaPoly / AES.GCM if you need to encrypt a payload before storing it. Don't invent ciphers, IV schedules, or KDFs. Keychain handles the hard part — encryption at rest, hardware-backed when Secure Enclave is available. Most apps just store tokens directly; the wrapping is overkill.

---

## File I/O

Use `URL` API, not string-based paths. The standard locations:

```swift
let docs = URL.documentsDirectory     // .documentDirectory — backed up, user-facing
let caches = URL.cachesDirectory      // .cachesDirectory — purgeable by system
let tmp = URL.temporaryDirectory      // .itemReplacementDirectory — system-cleaned
let appSupport = URL.applicationSupportDirectory // .applicationSupportDirectory — app files not user-facing
```

`FileManager.default` for create/move/delete; `JSONEncoder`/`JSONDecoder` for `Codable`:

```swift
struct Cache: Codable {
    var lastSyncedAt: Date
    var items: [Item.ID]
}

actor CacheStore {
    private let url: URL
    init(filename: String = "cache.json") {
        self.url = URL.cachesDirectory.appending(path: filename)
    }

    func load() throws -> Cache {
        guard FileManager.default.fileExists(atPath: url.path(percentEncoded: false)) else {
            return Cache(lastSyncedAt: .distantPast, items: [])
        }
        let data = try Data(contentsOf: url)
        return try JSONDecoder().decode(Cache.self, from: data)
    }

    func save(_ cache: Cache) throws {
        let data = try JSONEncoder().encode(cache)
        // Atomic write: writes to temp, swaps in place.
        try data.write(to: url, options: [.atomic])
    }
}
```

Caches that should not survive a low-disk reclamation: `URL.cachesDirectory`. Large user-visible documents: `URL.documentsDirectory` (consider `.isExcludedFromBackup = true` if it's regenerable). `URL.temporaryDirectory` for download intermediates — clean up on success.

Use `URLSession` background configuration to write large downloads straight to disk; iOS handles the file move atomically when the task completes.

---

## Offline-first / optimistic updates

The pattern: update the local store immediately on user action, mark the row `pending`, fire the network request, reconcile on response. The user sees instant feedback; sync errors surface as retry affordances, not failed taps.

```swift
enum SyncState: String, Codable { case pending, synced, failed }

@Model
final class Note {
    var id: UUID = UUID()
    var title: String = ""
    var body: String = ""
    var updatedAt: Date = .now
    var syncStateRaw: String = SyncState.synced.rawValue
    var syncState: SyncState {
        get { SyncState(rawValue: syncStateRaw) ?? .pending }
        set { syncStateRaw = newValue.rawValue }
    }
    init(title: String, body: String) {
        self.title = title; self.body = body
    }
}

@Observable
final class NoteService {
    @ObservationIgnored private let context: ModelContext
    @ObservationIgnored private let api: NotesAPI
    init(context: ModelContext, api: NotesAPI) {
        self.context = context; self.api = api
    }

    func updateTitle(_ note: Note, to newTitle: String) async {
        // Optimistic local write — view updates immediately.
        note.title = newTitle
        note.updatedAt = .now
        note.syncState = .pending
        try? context.save()

        do {
            try await api.update(id: note.id, title: newTitle)
            note.syncState = .synced
        } catch {
            note.syncState = .failed
        }
        try? context.save()
    }
}
```

In the view, render `syncState == .pending` as a subtle indicator (small spinner, faded text) and `.failed` as a tappable retry. Don't block the UI on the network — that's what offline-first means.

---

## SwiftData production checklist

- [ ] Every model wrapped in a `VersionedSchema` from v1.0.0 — even if no migration planned.
- [ ] `typealias` to the latest schema namespace so call-site code stays clean.
- [ ] `SchemaMigrationPlan` declared and registered at container creation.
- [ ] Suspension-sensitive persistence work identified from lifecycle/crash evidence; bounded critical sections use a background-task assertion only when needed.
- [ ] Sentry/Crashlytics rule for `0xdead10cc` exception codes when the app has seen or is actively guarding against this failure.
- [ ] CloudKit-synced models: every property has a default or is optional; every relationship is optional; no `@Attribute(.unique)`; no non-optional `Transformable`.
- [ ] Indexes chosen for measured query hot paths, with write/storage cost considered (iOS 18+).
- [ ] No MVVM wrapping `@Query` — view IS the view model with SwiftData.
- [ ] Background mutations live in `@ModelActor` types; `try modelContext.save()` always called before the actor returns.
- [ ] Cross-actor data passed as `PersistentIdentifier`, never `PersistentModel`.
- [ ] `@Query` predicates use static constants; dynamic filters live in child views that take the filter as init param.
- [ ] `FetchDescriptor` with `fetchLimit` + `fetchOffset` for any view that could load > 1k rows.
- [ ] `propertiesToFetch` when not all fields are displayed; `relationshipKeyPathsForPrefetching` when iterating relationships.
- [ ] Each new release bumps `Schema.Version` strictly — never reuse identifiers.
- [ ] Tests use an in-memory `ModelConfiguration(isStoredInMemoryOnly: true)`.
- [ ] Container ID + entitlements + push capability set for CloudKit. Verified with `NSPersistentCloudKitContainer.initializeCloudKitSchema()` in debug if relationships involved.

---

## Anti-patterns

- SwiftData without `VersionedSchema` from v1. Future migrations will fail with `Cannot use staged migration with an unknown model version`.
- `NSPredicate` string predicates with SwiftData. Use `#Predicate { … }`.
- `@Attribute(.unique)` on a CloudKit-synced model. CloudKit can't enforce it; sync silently breaks.
- Secrets or sensitive personal records in `UserDefaults`. Small secrets/keys go in Keychain; larger records use protected file/database storage.
- MVVM wrapped around `@Query`. ViewModels bloat or get tied to the data layer. Use MV with SwiftData.
- Long or lock-holding persistence work allowed to cross suspension without lifecycle coordination. Use evidence to shorten, defer, checkpoint, or protect only the bounded critical section.
- Forgetting `@ObservationIgnored` on cached derived state in `@Observable` classes. Every read triggers tracking, every write triggers redraw.
- `@AppStorage` directly inside an `@Observable` class. Compiles, but does not invalidate views. Use the manual `access`/`withMutation` bridge over `UserDefaults`. See `state-and-observation.md`.
- Passing `PersistentModel` instances across actors. Pass `PersistentIdentifier` and refetch.
- Non-optional `Transformable` attributes. Have been reported to block future migrations.
- Filtering in Swift after a fetch (`items.filter { … }`). Loads the whole table. Filter in `#Predicate`.
- `@Query` over many thousand rows on iOS 26 without pagination. Main-thread cost is measurable. Use `FetchDescriptor` with `fetchLimit` and a `@ModelActor` for heavy work.
- String-based file paths. Use `URL` everywhere.
- Skipping `try modelContext.save()` before `task.setTaskCompleted` in a background handler. iOS suspends the process and the write disappears.
- Rolling your own crypto on top of Keychain. Use `CryptoKit` if you really need to encrypt before storing; otherwise let Keychain do its job.
- Mixing `@Published` and `@Observable` on the same class. They don't compose. Pick one.
