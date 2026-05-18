# Testing and Debugging

Targets Swift 6.3 / iOS 26 / Xcode 26. Covers test strategy, mocking, UI tests, snapshots, previews, Instruments, logging, build profiling, CI/CD, crash reporting. State management, navigation, and architecture live in their own references.

## Test strategy in 2026 (lead with this)

- **Swift Testing for new tests.** It is the default test framework in Xcode 16+ and the mature choice in Xcode 26. Parameterization, parallelism, traits, async, and `@MainActor` work without ceremony.
- **XCTest for UI / performance / Obj-C bridges.** `XCUIApplication`, `XCTMetric`, and Objective-C test interop are XCTest-only as of Xcode 26.
- **Coexist in a single target.** Both frameworks compile side-by-side, both run on `Cmd+U` and on CI under the same test plan, both report to the same `.xcresult`. There is no "test target split" tax.
- **Migrate incrementally.** Documented community migrations have moved hundreds of XCTest files one at a time over months — never the whole repo at once. Partial migration is the universal pattern, not a transitional state.
- **File 17 reality check.** Audited real repos show partial migration is the norm: IceCubesApp 6 Swift Testing files / 13 XCTest files, NetNewsWire 27/62, CotEditor 135/1 (the migration champion). None are "100% migrated." Expect to live in both worlds for years.

Rule of thumb: every new test file is Swift Testing. You only open the XCTest editor when you're writing a UI test, a performance test, or touching an Obj-C bridge. Existing XCTest files migrate when you're already in there changing the code — not as a sprint task.

---

## Swift Testing — the basics

### `@Test` and `@Suite`

```swift
import Testing

@Test func calculatesTotalCorrectly() {
    let cart = Cart(items: [.apple, .bread])
    #expect(cart.total == 5.50)
}

@Test("Discount applies when over threshold")
func discountAboveThreshold() {
    var cart = Cart(items: Array(repeating: .apple, count: 10))
    #expect(cart.total < cart.subtotal)
}

@Suite("Cart pricing")
struct CartPricingTests {
    @Test func emptyCartIsFree() { #expect(Cart().total == 0) }
    @Test func taxAdded() { /* ... */ }
}
```

- No `XCTestCase` subclass, no `test` prefix, no `func setUp()`.
- Suites should be **structs** — deallocated when tests finish (better memory profile), no inheritance.
- Optional name string in `@Test("...")` shows up in the Test Navigator and reports.

### `#expect` and `#require`

```swift
@Test func loadsUser() async throws {
    let result = try await client.loadUser(id: "u-1")

    #expect(result.id == "u-1")          // non-fatal: collects, continues
    #expect(result.email.contains("@"))  // also runs even if above failed

    let profile = try #require(result.profile)  // fatal: halts if nil
    #expect(profile.displayName.isEmpty == false)
}
```

- `#expect` is the default — non-fatal, all expressions in the test run, failures collect with diff messages showing the actual values from the expression tree.
- `try #require` is for unwraps and prerequisites — if the result is `nil` or `throws`, the test halts (later assertions would crash or be meaningless).
- The macro captures the expression source — failure messages are precise without `XCTAssertEqual(actual, expected, "message")` boilerplate.

### Traits — tags, conditional, links, timing

```swift
extension Tag {
    @Tag static var unit: Self
    @Tag static var networking: Self
    @Tag static var smoke: Self
}

@Test(.tags(.unit, .smoke)) func fastUnit() { /* ... */ }
@Test(.tags(.networking)) func hitsAPI() async throws { /* ... */ }

@Test(.enabled(if: ProcessInfo.processInfo.environment["CI"] == nil))
func interactiveOnly() { /* runs locally, skipped on CI */ }

@Test(.disabled("Known crash on iOS 26.2 simulator — re-enable after upstream fix"))
func skipForNow() { /* ... */ }

@Test(.bug("https://github.com/apple/swift/issues/N", "AttributeGraph regression"))
func relatedToUpstreamBug() { /* ... */ }

@Test(.serialized) func touchesSharedKeychainEntry() { /* opts out of parallel */ }

@Test(.timeLimit(.minutes(1))) func mustNotHang() async throws { /* ... */ }
```

- `.tags` categorize. Filter via Test Navigator or run only-`.smoke` on PR CI / full-tag-set nightly.
- `.enabled(if:)` / `.disabled("reason")` replace `XCTSkip`. Always include a reason string — your future self will thank you.
- `.bug` links to trackers (URLs, FB IDs, GitHub issues) and renders in test reports.
- `.serialized` forces one-at-a-time when a test touches truly shared state (keychain, on-disk DB without isolation).
- `.timeLimit` is your seatbelt for tests that could hang.

### Parameterized tests

```swift
@Test("Score clamps to 0...100",
      arguments: [(input: -10, expected: 0),
                  (input: 0, expected: 0),
                  (input: 50, expected: 50),
                  (input: 100, expected: 100),
                  (input: 200, expected: 100)])
func clampScore(input: Int, expected: Int) {
    #expect(Score(raw: input).clamped == expected)
}

// Cross-product over two collections
@Test(arguments: [Feature.a, .b, .c], 0...10)
func featureBehavesAcrossDepths(feature: Feature, depth: Int) {
    #expect(feature.evaluate(at: depth) != nil)
}
```

- Each argument runs as a **separate, individually-reportable test** — pass/fail per row, individually runnable from the Test Navigator.
- Arguments must be `Sendable`. Pass data, not closures.
- Parameterized tests run in **parallel by default** — add `.serialized` only when a row touches shared state.
- Replaces every for-loop assertion pattern you wrote in XCTest. The failure pinpoints the input value, not the loop iteration number.

### Async and MainActor

```swift
@Test @MainActor
func loadsItems() async throws {
    let model = ItemListModel(client: MockClient(returning: [.sample]))
    await model.load()
    #expect(model.items.count == 1)
}
```

- `async throws` is fully native — no `XCTestExpectation`, no `fulfillment(of:timeout:)`, no fake DispatchGroups.
- `@MainActor` on a `@Test` or `@Suite` Just Works — no warning-every-line pain that XCTest had.
- For Swift 6.2 with default isolation: add `.defaultIsolation(MainActor.self)` to `swiftSettings` in the test target's `Package.swift` to make every test `@MainActor` by default. For CPU-heavy tests under that default, opt back to a background executor with `@concurrent @Test func computeHash() async`.

### `confirmation` — for callback / delegate APIs

```swift
@Test func debouncerFiresOnceAfterRapidTaps() async {
    await confirmation("debounced action", expectedCount: 1) { confirm in
        let debouncer = Debouncer(interval: .milliseconds(200)) { confirm() }
        for _ in 0..<10 { debouncer.call() }
        try? await Task.sleep(for: .milliseconds(500))
    }
}

@Test func logoutDoesNotTriggerSync() async {
    await confirmation("sync called", expectedCount: 0) { confirm in
        let model = AuthModel(onSync: { confirm() })
        await model.logout()
    }
}
```

- `expectedCount: N` — verify the event fires exactly N times.
- `expectedCount: 0` — verify the event **never** fires (powerful and underused).
- `expectedCount: 2...5` (Swift 6.2) — range-based for "at least", "at most", "between".
- Inside `@MainActor` async contexts, pass `isolation: #isolation` so the closure doesn't hop actors.
- For legacy completion handler APIs, wrap in `withCheckedThrowingContinuation` first — `confirmation` is not a continuation.

### Exit tests (Swift 6.2 / Xcode 26)

```swift
@Test func rejectsBadInput() async {
    await #expect(processExitsWith: .failure) {
        precondition(false, "must be > 0")
    }
}

@Test func badCLIUsageExitsWithUsageCode() async {
    await #expect(processExitsWith: .exitCode(EX_USAGE)) {
        CLI.parse(["--invalid-flag"])
    }
}
```

- Verify code terminates the process: `precondition`, `fatalError`, `abort`, custom exit codes, signals.
- Use `.success`, `.failure`, `.exitCode(EX_USAGE)`, `.signal(SIGABRT)`.
- Capture stderr with `observing: [\.standardErrorContent]` to assert on the death message.
- Replaces the XCTest pattern of "I hope this precondition death doesn't kill the test runner."

### `withKnownIssue` — better than `.disabled`

```swift
@Test func flakyExternalServiceCall() async throws {
    try await withKnownIssue("API rate limit causes intermittent 429", isIntermittent: true) {
        try await uploadAndVerify()
    }
}
```

- The test keeps **compiling and running**. Failures are reported as **expected**, not failures.
- When the underlying issue is fixed, the test auto-flips to a real failure ("unexpected success") — prompting you to remove the wrapper.
- `isIntermittent: true` tolerates pass-or-fail (genuinely external flakes only — never your own code).
- Beats `.disabled("FB-12345")` because the test still runs. Beats deleting the test because the assertion is documented.

---

## XCTest — when still required

XCTest stays in the toolbox for three specific jobs:

- **UI tests with `XCUIApplication`.** Swift Testing does not (yet) cover UI automation in Xcode 26.
- **Performance tests with `XCTMetric`.** `measure { }` blocks with metrics (CPU, memory, launch time, scroll hitches) are XCTest-only.
- **Objective-C test bridges.** Swift Testing macros don't expand into Obj-C.

Beyond those three, prefer Swift Testing — its syntax, parameterization, parallelism, and async support all win. Don't rewrite passing XCTest just for fashion: migrate when you're already touching the file.

---

## Coexisting Swift Testing + XCTest

- **Same test target.** Add Swift Testing imports next to existing XCTest imports.
- **Both run on `Cmd+U` and CI.** No flag flips, no separate test plan, no `xcodebuild` invocation differences.
- **Same `.xcresult` bundle.** Reports include both frameworks; failures cluster by file in the Test Navigator regardless of framework.
- **Don't rewrite XCTest for fashion.** If a test is green and you're not editing the code under test, leave it. Migration cost without correctness benefit is debt.
- **Mark migration during real edits.** When you change the function under test, migrate its test file. The `@MainActor` and parameterization wins pay off the conversion time.

---

## Mocking strategies

### URLSession via URLProtocol

A custom `URLProtocol` intercepts every request — your production code keeps using `URLSession` unchanged. Best for app-wide drop-in.

```swift
final class MockURLProtocol: URLProtocol {
    nonisolated(unsafe) static var handler: ((URLRequest) throws -> (HTTPURLResponse, Data))?

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for r: URLRequest) -> URLRequest { r }

    override func startLoading() {
        guard let handler = Self.handler else {
            client?.urlProtocol(self, didFailWithError: URLError(.unknown))
            return
        }
        do {
            let (response, data) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

// Test
@Test(.serialized) // static handler is shared state
func loadsRecipe() async throws {
    let config = URLSessionConfiguration.ephemeral
    config.protocolClasses = [MockURLProtocol.self]
    let session = URLSession(configuration: config)

    MockURLProtocol.handler = { req in
        let body = try JSONEncoder().encode(Recipe.sample)
        let response = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!
        return (response, body)
    }
    defer { MockURLProtocol.handler = nil }

    let client = RecipeAPI(session: session)
    let recipe = try await client.fetch(id: "r-1")
    #expect(recipe.id == Recipe.sample.id)
}
```

- Pro: production code is untouched (`URLSession.shared` still works).
- Con: `handler` is static — parallel tests collide. Use `.serialized` on the suite, or move to the protocol-based approach below for parallel safety.
- `URLSessionConfiguration.ephemeral` is the right base — no cookies, no cache, no disk side effects between tests.
- `WeTransfer/Mocker` wraps `URLProtocol` with per-test isolation if your suite is large.

### Protocol-based DI (parallel-safe)

```swift
protocol HTTPClient: Sendable {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: HTTPClient {}

struct MockHTTPClient: HTTPClient {
    let respond: @Sendable (URLRequest) async throws -> (Data, URLResponse)
    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        try await respond(request)
    }
}

// Service uses the protocol
struct RecipeAPI {
    let client: HTTPClient
    init(client: HTTPClient = URLSession.shared) { self.client = client }
    func fetch(id: String) async throws -> Recipe { /* ... */ }
}

// Test
@Test func loadsRecipe() async throws {
    let mock = MockHTTPClient { req in
        let body = try JSONEncoder().encode(Recipe.sample)
        let response = HTTPURLResponse(url: req.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!
        return (body, response)
    }
    let api = RecipeAPI(client: mock)
    let recipe = try await api.fetch(id: "r-1")
    #expect(recipe.id == Recipe.sample.id)
}
```

- No global state, fully parallel-safe.
- Requires injecting `HTTPClient` through your service initializers — small footprint, large win.
- Use protocol-based DI as the default for new code. Use URLProtocol when you can't change service signatures.

### Protocol-based DI for any service

Same pattern generalizes to any external collaborator:

```swift
protocol AnalyticsClient: Sendable {
    func track(_ event: String, properties: [String: String])
}

struct FakeAnalytics: AnalyticsClient {
    let recorded: Locked<[(event: String, props: [String: String])]> = .init([])
    func track(_ event: String, properties: [String: String]) {
        recorded.withLock { $0.append((event, properties)) }
    }
}

@Observable
final class CheckoutModel {
    private let analytics: AnalyticsClient
    init(analytics: AnalyticsClient) { self.analytics = analytics }
    func confirm() { analytics.track("checkout.confirmed", properties: [:]) }
}

@Test func tracksCheckoutEvent() {
    let fake = FakeAnalytics()
    CheckoutModel(analytics: fake).confirm()
    #expect(fake.recorded.withLock { $0.count } == 1)
}
```

Constructor inject every external dependency. Singletons are testability landmines: a `Logger.shared` or `AnalyticsManager.shared` reference inside your model means you have no way to isolate a test. Inject from `App.swift` through `.environment` for view-level dependencies, through `init` for model-level.

### In-memory ModelContainer (SwiftData)

```swift
import SwiftData

@MainActor
func makeTestContainer() throws -> ModelContainer {
    try ModelContainer(
        for: Recipe.self, Step.self,
        configurations: ModelConfiguration(isStoredInMemoryOnly: true)
    )
}

@Test @MainActor
func recipeSavesAndQueries() async throws {
    let container = try makeTestContainer()
    let ctx = container.mainContext

    ctx.insert(Recipe(name: "Pasta"))
    try ctx.save()

    let recipes = try ctx.fetch(FetchDescriptor<Recipe>())
    #expect(recipes.count == 1)
    #expect(recipes[0].name == "Pasta")
}
```

- Fast — no disk I/O, no migration.
- Isolated — each test makes its own container, no cross-test bleed.
- No `tearDown` cleanup — when the container goes out of scope, it's gone.
- Use the **real** `ModelContainer`, not a mock. SwiftData is the system under test for persistence integration; mocking it tests your mock.

### Don't mock actors

Actors are isolation boundaries, not seams for substitution. Use a real instance with deterministic inputs:

```swift
actor RateLimiter {
    private let clock: any Clock<Duration>
    private var lastRequest: ContinuousClock.Instant?
    init(clock: any Clock<Duration> = ContinuousClock()) { self.clock = clock }
    func allow() async -> Bool { /* ... */ }
}

// Test with a fake clock, not a fake actor
@Test func rateLimiterDeniesRapidCalls() async {
    let clock = TestClock()
    let limiter = RateLimiter(clock: clock)
    #expect(await limiter.allow() == true)
    await clock.advance(by: .milliseconds(10))
    #expect(await limiter.allow() == false)
}
```

If you must inject "the actor that does X," use a protocol with the actor as one conformer and a `struct` stub as another — but try a real instance with controlled inputs first.

### Don't mock @Observable

`@Observable` types are state containers. Construct the real instance with test inputs and inject fake dependencies through its initializer:

```swift
@Observable
final class RecipeListModel {
    private let api: HTTPClient
    init(api: HTTPClient = URLSession.shared) { self.api = api }
    var recipes: [Recipe] = []
    func load() async { /* uses api */ }
}

@Test @MainActor
func loadsRecipes() async {
    // Real RecipeListModel, fake api.
    let model = RecipeListModel(api: MockHTTPClient { _ in
        (try JSONEncoder().encode([Recipe.sample]),
         HTTPURLResponse(url: URL(string: "https://x")!, statusCode: 200, httpVersion: nil, headerFields: nil)!)
    })
    await model.load()
    #expect(model.recipes.count == 1)
}
```

Mocking the `@Observable` itself defeats the test — you'd be testing your mock's stub of `load()`, not the real logic.

---

## UI testing

UI tests stay in XCTest in 2026, driving `XCUIApplication`. The workflow improvements in Xcode 26 are big: a no-code recorder, automated accessibility audits, and faster element queries.

### Launch and identify

```swift
import XCTest

final class SignInUITests: XCTestCase {
    func testSignInHappyPath() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-uiTestingMode", "1", "-skipOnboarding", "1"]
        app.launchEnvironment["MOCK_API"] = "1"
        app.launch()

        let email = app.textFields["auth.emailField"]
        let password = app.secureTextFields["auth.passwordField"]
        let signIn = app.buttons["auth.signInButton"]

        XCTAssertTrue(email.waitForExistence(timeout: 5))
        email.tap(); email.typeText("user@example.com")
        password.tap(); password.typeText("hunter2")
        signIn.tap()

        let home = app.otherElements["home.root"]
        XCTAssertTrue(home.waitForExistence(timeout: 5))
    }
}
```

### Accessibility identifiers, not labels

```swift
Button("Sign in") { /* ... */ }
    .accessibilityIdentifier("auth.signInButton")
```

- Identify elements by `.accessibilityIdentifier("screen.element")` — not by the user-visible label.
- Labels are translated and change with copy edits; identifiers are stable test contracts.
- Use a single `enum AccessibilityID` (or namespaced struct) in a shared target so tests don't carry string literals.
- Every interactive view that you intend to test or that the recorder should reliably reach gets one — from day one. Free a11y compliance, free VoiceOver labels, free recorder support.

### Wait — never sleep

```swift
// Bad — flaky, slow, lies about success
sleep(2)
app.buttons["confirm"].tap()

// Good — polls until found, fails fast if not
let confirm = app.buttons["confirm"]
XCTAssertTrue(confirm.waitForExistence(timeout: 5))
confirm.tap()

// For "exists AND hittable", chain
let confirm2 = app.buttons["confirm"]
XCTAssertTrue(confirm2.waitForExistence(timeout: 5))
XCTAssertTrue(confirm2.isHittable)
confirm2.tap()
```

- `sleep(_:)` is the #1 cause of UI test flakes. Never.
- `waitForExistence(timeout:)` polls and returns as soon as the element appears.
- For more complex conditions, use `XCTNSPredicateExpectation` with `XCTWaiter().wait(for: [expectation], timeout: 5)`.

### Xcode 26 no-code UI test recorder

Xcode 26 ships a recorder (WWDC25 session 344). Open a UI test file, click **Record** in the sidebar, drive the simulator, Xcode emits standard XCTest code. Treat the output as a starting point: the recorder is only as good as your accessibility identifiers — if elements are reachable only by index or label, the recorder produces brittle code. Review and refactor before committing.

### Accessibility audits in CI

```swift
func testAccessibilityAudit() throws {
    let app = XCUIApplication()
    app.launch()
    try app.performAccessibilityAudit()
}

// Or restricted with allowlist
func testContrastAudit() throws {
    let app = XCUIApplication()
    app.launch()
    try app.performAccessibilityAudit(for: [.dynamicType, .contrast]) { issue in
        // Return true to ignore (e.g., known third-party SDK issue)
        issue.element?.identifier == "thirdparty.banner"
    }
}
```

- Catches: missing labels, low contrast, hit-target size, clipped Dynamic Type, missing traits.
- Run on every PR. A 2026 SwiftUI app that ships without this in CI is a bug-report factory waiting for the App Store review team or a real VoiceOver user.

### Launch arguments for test configuration

```swift
// Test side
app.launchArguments = ["-uiTestingMode", "1", "-skipOnboarding", "1"]
app.launchEnvironment["MOCK_API"] = "1"
app.launch()

// App side (in @main App init or root view onAppear)
let isUITest = ProcessInfo.processInfo.arguments.contains("-uiTestingMode")
if isUITest {
    UIView.setAnimationsEnabled(false)
    // swap in MockURLProtocol, skip splash, seed test data
}
```

Use launch args to: inject network mocks, skip onboarding, disable analytics, seed deterministic data, disable animations (the #1 stability win).

---

## Snapshot testing (pointfreeco/swift-snapshot-testing)

Snapshots compare a rendered view to a stored PNG. The package version 1.18+ supports both XCTest and Swift Testing.

### When snapshots earn their keep

- **Design-system primitives.** Buttons, labels, color tokens, typography ramp, banners — the components your whole app depends on.
- **Edge cases visual review misses.** Long names that overflow, empty states, RTL, accessibility text sizes, dark/light variants.
- **Locked-down components.** Once the visual is correct, you want a CI failure when someone breaks it accidentally.

### When snapshots are a trap

- **Every screen.** Gigabytes of PNGs, every Xcode update turns the suite red, real visual regressions drown in the noise.
- **Animated states.** Frame timing differs between simulator versions.
- **WKWebView-based views.** Known timeout issues on Xcode 16.4+ as of mid-2025.
- **Marketing-changing surfaces.** Copy and layout churn defeats the value.

isowords (Point-Free's reference codebase) has 17 snapshot test files — all design-system focused. That's the right ratio. If your app has 17 snapshot tests per screen, you're using the wrong tool.

### Setup and example

```swift
// Package.swift
.package(url: "https://github.com/pointfreeco/swift-snapshot-testing", from: "1.18.0")

// Test
import SnapshotTesting
import SwiftUI
import Testing

@Test func recipeCardMatchesSnapshot() {
    let view = RecipeCard(recipe: .preview)
        .frame(width: 390, height: 200)

    assertSnapshot(
        of: view,
        as: .image(precision: 0.99, layout: .fixed(width: 390, height: 200))
    )
}

@Test(arguments: [DynamicTypeSize.medium, .accessibility3])
func cardAcrossDynamicType(size: DynamicTypeSize) {
    let view = RecipeCard(recipe: .preview)
        .environment(\.dynamicTypeSize, size)
        .frame(width: 390, height: 200)

    assertSnapshot(
        of: view,
        as: .image(precision: 0.99, layout: .fixed(width: 390, height: 200)),
        named: "\(size)"
    )
}
```

### Snapshot rules

- **Pin device size and scheme explicitly** with `.image(layout: .fixed(width:height:))`. Never `.device(.iPhone16Pro)` — drifts across Xcode versions and your snapshots redraw on every Xcode update.
- **`precision: 0.99`** tolerates anti-aliasing noise without missing real diffs.
- **One component per test**, not whole screens. Use parameterized tests for permutations (dynamic type, color scheme, locale).
- **Review the diff in PR**, not as a rubber stamp. Snapshot tests that auto-record fail-then-accept are an unguarded loaded gun.
- **Consider HEIC** (`SnapshotTestingHEIC`) for repo size if you have many snapshots.

---

## Previews

Previews aren't tests, but they're the fastest visual + accessibility feedback loop. Xcode 26 dramatically extended the modifier vocabulary and canvas controls.

### `#Preview` macro

```swift
#Preview("Default") {
    RecipeList(recipes: .sample)
}

#Preview("Empty") {
    RecipeList(recipes: [])
}

#Preview("Dark + Large Text") {
    RecipeList(recipes: .sample)
        .environment(\.colorScheme, .dark)
        .environment(\.dynamicTypeSize, .accessibility3)
}

#Preview("Landscape", traits: .landscapeLeft) {
    RecipeList(recipes: .sample)
}
```

- Multiple `#Preview` blocks per file. Each gets its own canvas pane.
- No more `PreviewProvider` boilerplate. (`PreviewProvider` is deprecated and a flag-on-sight code smell — see anti-patterns.)
- Built-in traits include `.portrait`, `.landscapeLeft`, `.landscapeRight`, `.portraitUpsideDown`, and accessibility traits.

### `@Previewable` (Xcode 16+) — stateful previews

```swift
#Preview("Toggle") {
    @Previewable @State var isOn = false
    Toggle("Notifications", isOn: $isOn)
}

#Preview("Search") {
    @Previewable @State var search = ""
    SearchableList(query: $search)
}
```

- No more helper struct dance for previews that need `@State`, `@Bindable`, `@Environment`, or `@Query`.
- Property wrappers work directly in the preview closure.

### `PreviewModifier` (iOS 18+) — shared preview environments

```swift
struct MockDataPreview: PreviewModifier {
    static func makeSharedContext() async throws -> ModelContainer {
        let container = try ModelContainer(
            for: Recipe.self,
            configurations: ModelConfiguration(isStoredInMemoryOnly: true)
        )
        let ctx = ModelContext(container)
        for recipe in Recipe.previewData { ctx.insert(recipe) }
        return container
    }

    func body(content: Content, context: ModelContainer) -> some View {
        content.modelContainer(context)
    }
}

#Preview(traits: .modifier(MockDataPreview())) {
    RecipeList()
}
```

- `makeSharedContext` runs **once** and the result is shared across every preview using this modifier — fast canvas refresh, deterministic data.
- The killer feature for SwiftData previews and dependency-injected views — one mock container, every preview uses it.
- Stack multiple modifiers: `.modifier(MockDataPreview()).modifier(MockThemePreview())`.

### Preview variants

- **Canvas controls** (Xcode 26): two buttons in the bottom-left toggle through Dynamic Type sizes and color schemes without code edits. Use for quick spot checks.
- **Named previews** for permanent cases that PR reviewers should see. Every screen should have at least one Dark + Large Text variant.
- **`.previewDevice(_)` and `.previewLayout(_)`** still exist for specific frame requirements; prefer modifying environment values for most variants.

---

## Instruments (Xcode 26)

Profiling SwiftUI updates is the single most valuable workflow change in 2026. The new Instruments SwiftUI template + Cause & Effect Graph turns "why is this slow?" from guesswork into a directed search.

- **Instruments 26 SwiftUI template** (WWDC25 session 306) — the new way to profile SwiftUI. Open Instruments → SwiftUI. Four lanes:
  - **Update Groups** — when SwiftUI is doing main-thread work.
  - **Long View Body Updates** — body evaluations that exceeded budget (orange/red color-coded).
  - **Long Representable Updates** — `UIViewRepresentable` / `NSViewRepresentable` slowness.
  - **Other Long Updates** — environment changes, layout, transitions.
- **Cause & Effect Graph** — the headline feature. Stack traces are useless for "why did this view update?" in a declarative framework. The graph traces from state change → transaction → invalidated attributes → bodies recomputed. Click any update and walk back to the original mutation. WWDC25 demo: tapping one heart in a list invalidates every row because they all depend on the full `favorites` array; fix is to derive `isFavorite` per row from a stable identity.
- **Time Profiler** — still the workhorse for CPU hot spots. Sample-based; use when you know "something is hot" but not what.
- **Hangs and Hitches** — main-thread blockages. >250ms = hang; >16ms = hitch. Use for scroll stutter and launch hangs.
- **Allocations + Leaks** — memory growth and unreleased objects. Pair with the Memory Graph Debugger for cycle detection.
- **Swift Concurrency** — visualizes Tasks, awaits, suspension points, actor flow, task contention. Much improved in Xcode 26.
- **Processor Trace** (Xcode 16.3+, M4 / iPhone 16+) — hardware-level branch tracing. Niche but powerful for micro-optimizations.
- **Power Profiler** (new in Xcode 26) — correlates app activity with system energy and thermals. Critical for background-active apps.
- **`OSSignposter` markers** — custom signposts surface in the os_signpost lane, aligning your CPU samples with high-level app events (see the OSLog section below).

---

## OSLog / os.Logger

`os.Logger` is the only logger to use in 2026 production code. `print` is for `Playground` work and nothing else — bypasses unified logging, leaks PII in shipped builds, won't show in `OSLogStore`.

### Setup

```swift
import OSLog

extension Logger {
    static let subsystem = Bundle.main.bundleIdentifier ?? "com.example.app"
    static let networking = Logger(subsystem: subsystem, category: "networking")
    static let persistence = Logger(subsystem: subsystem, category: "persistence")
    static let ui = Logger(subsystem: subsystem, category: "ui")
    static let auth = Logger(subsystem: subsystem, category: "auth")
}

// At call sites
Logger.networking.info("Fetched \(items.count) items in \(elapsed.formatted())s")
Logger.persistence.error("Save failed: \(error.localizedDescription, privacy: .public)")
```

- **Subsystem = bundle ID.** **Category = subsystem name** (networking, persistence, ui, auth). Filter the Console.app by either.
- Levels:
  - `.debug` — high volume, **not persisted** unless debugger attached.
  - `.info` — diagnostic, **persisted to memory** by default.
  - `.notice` — default, **persisted to disk** for a short window.
  - `.error` — recoverable failures.
  - `.fault` — programmer errors (precondition-like), highest priority.

### Privacy interpolation

By default, dynamic values are **redacted in production logs** as `<private>`. Mark non-sensitive data as public explicitly.

```swift
Logger.networking.error("Failed to load recipe id=\(id, privacy: .public): \(error.localizedDescription, privacy: .public)")

Logger.auth.info("Sign-in attempt for \(email, privacy: .private)")
Logger.auth.notice("Token rotated for user \(userID, privacy: .private(mask: .hash))")
```

- `.public` for non-PII strings (resource IDs, status codes, durations).
- `.private` (default) — redacted in shipped builds unless device is unlocked and paired to a Mac with the right entitlement.
- `.private(mask: .hash)` for stable identifiers in logs — you can correlate across log lines without exposing the value. The right choice for user IDs.

### OSLogStore (in-app log retrieval)

```swift
import OSLog

func recentLogs(category: String, lines: Int = 200) throws -> [String] {
    let store = try OSLogStore(scope: .currentProcessIdentifier)
    let position = store.position(timeIntervalSinceLatestBoot: 0)
    let entries = try store.getEntries(at: position)
        .compactMap { $0 as? OSLogEntryLog }
        .filter { $0.subsystem == Logger.subsystem && $0.category == category }
        .suffix(lines)
    return entries.map { "[\($0.date)] \($0.level.rawValue) \($0.composedMessage)" }
}
```

- Useful for: in-app diagnostic export ("attach logs to bug report"), crash diagnostics post-restart, debug screens for QA.
- `.currentProcessIdentifier` for in-process; `.system` requires entitlements.

### OSSignposter — performance markers for Instruments

```swift
let signposter = OSSignposter(subsystem: Logger.subsystem, category: "rendering")

func renderFrame() {
    let id = signposter.makeSignpostID()
    let interval = signposter.beginInterval("render frame", id: id)
    defer { signposter.endInterval("render frame", interval) }
    // expensive rendering work
}

// One-shot event markers
signposter.emitEvent("user tapped reload")
```

- Surfaces in Instruments → Time Profiler with the **os_signpost** lane visible.
- Align CPU samples with high-level app events: "user tap → load started → first byte → first frame."
- Use intervals for spans, events for instants. Both are zero-overhead in production.

---

## `Self._printChanges()` — the SwiftUI debug trick

The fastest way to find out "why is this view re-rendering?"

```swift
struct RecipeRow: View {
    let recipe: Recipe
    var body: some View {
        #if DEBUG
        let _ = Self._printChanges()
        #endif
        HStack { /* ... */ }
    }
}
```

- Logs to the console which dependency caused the body to re-evaluate. Tags:
  - `@self` — entire struct value changed (often: parent passes a fresh instance every time).
  - `@identity` — view identity changed (forces full teardown — usually a bug, often from `.id()` or a non-stable `ForEach(id:)`).
  - Property name — that specific input changed (the normal case).
- **Strip before ship — `#if DEBUG` it.** Underscore-prefixed = private API. Apple has historically tolerated it across versions but could remove it without warning.
- **`Self._logChanges()` variant** (Xcode 15.1+) — emits to the unified log under `com.apple.SwiftUI` / "Changed Body Properties". Survives device runs, viewable in Console.app.

---

## Random background color trick — visualize re-renders

```swift
struct RecipeRow: View {
    let recipe: Recipe
    var body: some View {
        HStack { /* ... */ }
        #if DEBUG
            .background(Color(
                red: .random(in: 0...1),
                green: .random(in: 0...1),
                blue: .random(in: 0...1)
            ))
        #endif
    }
}
```

- Each body evaluation picks a new random color. If a cell flashes (color changes constantly), it's re-rendering when it shouldn't.
- Place on a parent to find which subtree re-invalidates.
- Place on a leaf to confirm the fix worked — the color should stay constant for stationary cells.
- Cheap, visual, no Instruments setup required. The "disco ball" diagnostic.

---

## View hierarchy debugger

Xcode → Debug → "Capture View Hierarchy" (while running).

- 3D rendered tree showing all views, hosting controllers, and constraints.
- Inspect frames, modifiers, identifier, accessibility traits.
- Confirms what SwiftUI **actually built** vs what your code declared — useful for opaque type issues.
- Especially useful for hybrid SwiftUI/UIKit screens to see the hosting controller boundaries.

---

## Memory graph debugger

Xcode → Debug → "Capture Memory Graph".

- Heap snapshot showing every live object and reference.
- Filter by class to find unexpected retainers ("why are there 7 instances of `RecipeViewModel`?").
- Purple cycle icons flag retain cycles automatically.
- Pair with Allocations in Instruments for time-series memory growth analysis.

---

## Performance debugging recipe

This is the step-by-step. When a view is slow, do these in order — don't skip steps.

1. **Reproduce the slow case.** Build a minimum-action sequence in the simulator that consistently triggers the slowness. "Sometimes it's slow" is not actionable.
2. **Capture in Instruments SwiftUI template.** Run Product → Profile (`Cmd+I`), choose SwiftUI. Drive the slow case while recording.
3. **Look at the Cause & Effect Graph.** Which view re-renders? Why? Walk back to the originating state change. This usually reveals the root cause: a broad observation dependency or an unstable identity.
4. **Add `Self._printChanges()` to suspect bodies (DEBUG-only).** Confirms the Instruments finding at the source-code level. Tells you exactly which property is triggering the re-render.
5. **Extract subviews if a parent re-render triggers many leaf re-renders.** As a popular community comment puts it: *"If instead you have the things that rely on one piece of state be fairly small and distinct View types, then the system need only recompute those particular small views, which is a LOT less work."* Real `View` structs (not `@ViewBuilder` computed properties) participate in AttributeGraph diffing.
6. **Verify with the random background color trick.** Confirms the parent re-render is contained and the leaf stops flashing.
7. **Re-capture in Instruments to confirm the fix.** "Long View Body Updates" lane should be empty for that view; Cause & Effect Graph should show only the intentional updates.

The order matters. Skipping Instruments and going straight to `_printChanges` finds individual re-renders but misses macro patterns ("every row updates whenever any heart toggles"). Skipping `_printChanges` after Instruments leaves you guessing which property triggered the re-render.

---

## Build profiling

Slow builds are death by a thousand cuts. Diagnose before optimizing.

```
// Other Swift Flags (Debug)
-Xfrontend -warn-long-function-bodies=100
-Xfrontend -warn-long-expression-type-checking=100
```

- Emits a warning for any function body that takes >100ms to type-check, or any expression >100ms.
- Usual culprits: ternary chains, dictionary literals with mixed types, opaque `some View` with many chained modifiers.
- Refactor flagged code — split into smaller pieces, annotate types to short-circuit inference.

For overall build-phase analysis: Product → Perform Action → Build With Timing Summary (or `xcodebuild ... -showBuildTimingSummary`). Xcode 26 adds a **Build Timeline** in the Reports navigator showing compilation parallelism as a Gantt chart.

---

## Code coverage

Enable per scheme: Edit Scheme → Test → Options → **Gather coverage**.

- Reports appear in the Test Reports navigator.
- Export: `xcrun xccov view --report --json TestResults.xcresult > coverage.json`.
- **Target ~70-80% as a floor**, not a ceiling. Chasing 100% pushes engineers to test trivial getters and setters at the expense of meaningful tests.
- Exclude generated code (`*.generated.swift`, mock files, asset catalogs).
- Treat coverage as a signal of where you haven't tested — not as a number to grow.

---

## CI/CD

### Xcode Cloud

Apple's hosted CI. The strongest fit when:

- App is iOS/macOS-only.
- Team is small or new to CI/CD — automatic code signing is the killer feature (no Fastlane match, no certificate gymnastics).
- 25 compute hours/month included with Apple Developer membership is enough or affordable to exceed (claim flagged — verify your seat tier).
- Tight integration with TestFlight, App Store Connect, Xcode Organizer crash reports matters.

Configured via App Store Connect or Xcode → Reports → Cloud. No YAML — a UI-driven workflow editor.

### GitHub Actions + Fastlane

Choose this when:

- You ship across iOS, Android, web, and backend — one CI for everything beats two.
- You need extensive customization (Slack pings on specific lanes, Firebase Crashlytics dSYM uploads, conditional release notes, complex branch policies).
- You already have a GitHub-centric workflow.
- Cross-team policy demands GitHub for audit/SOC reasons.

Expect to maintain:
- `.github/workflows/ios.yml` (50-150+ lines).
- `Fastfile`, `Appfile`, `Matchfile`, `Gemfile`.
- A `match` Git repo (or App Store Connect API key) for signing.
- Awareness that pinned Xcode versions on `macos-*` runners disappear after Apple updates.

### TestFlight

- **Internal testing groups** — no Apple review, instant distribution to up to 100 internal team members.
- **External testing groups** — Apple TestFlight review (usually <24h, sometimes longer), distribution to up to 10,000 external testers.
- **Builds expire after 90 days** — automation pipelines should refresh stale builds.

### Hybrid is fine and common

Many teams use Xcode Cloud for iOS build → TestFlight (signing simplicity wins) and GitHub Actions for backend, web, linting, and PR checks. Don't force one tool to do everything.

---

## Fastlane in 2026

Still relevant for two specific jobs:

1. **Cross-platform release orchestration** (when paired with non-Apple CI).
2. **App Store Connect API operations** Xcode Cloud doesn't fully cover: dSYM uploads to Crashlytics (`upload_symbols_to_crashlytics`), mass metadata localization (`deliver`), TestFlight tester management (`pilot`), screenshot generation (`snapshot`), Slack/Discord notifications.

Inside Xcode Cloud, you can still install Fastlane via the post-clone `ci_post_clone.sh` script — Homebrew is preinstalled.

Less relevant when Xcode Cloud covers your needs end-to-end.

---

## Profile-Guided Optimization (PGO)

PGO can yield a significant release-build speedup for performance-critical paths (5-10% claim flagged — verify with your workload).

Workflow:
1. Product → Perform Action → Generate Optimization Profile.
2. Either exercise the app manually through typical user paths, or run your performance test suite.
3. Xcode writes a `.profdata` file (default location: `OptimizationProfiles/` in the project).
4. Release builds automatically pick up the profile. The compiler warns when the profile drifts and should be regenerated.

Caveats:
- Single profile per project — multi-architecture builds share it.
- Not a substitute for fixing slow algorithms. Apply only **after** profiling has identified hot paths.
- Debug builds ignore PGO entirely.

---

## DocC

The only documentation toolchain to use for Swift code in 2026 — Xcode-integrated, web-publishable, supports articles + tutorials + symbol references.

```swift
/// Loads recipes from the remote source.
///
/// This call may take several seconds on cold launch as the on-disk cache is populated.
///
/// - Parameter category: Filter to a single ``RecipeCategory``. Pass `nil` for everything.
/// - Returns: An array of ``Recipe`` instances, sorted by ``Recipe/rating``.
/// - Throws: ``NetworkError`` on connectivity failures.
public func loadRecipes(in category: RecipeCategory? = nil) async throws -> [Recipe]
```

- Triple-slash `///` for documented (public) members; double-slash `//` for non-doc comments.
- Double-backticks for symbol links: ` ``Recipe`` `, ` ``Recipe/rating`` `.
- `> Note:`, `> Tip:`, `> Warning:`, `> Important:` for callouts.
- `- Parameters:` / `- Returns:` / `- Throws:` for structured fields.
- Build: Product → Build Documentation (Shift-Ctrl-Cmd-D). CLI: `xcrun docc convert` or `xcodebuild docbuild`.
- Hosting: export `.doccarchive` (single-page web app inside) and serve as static files.
- Invest in DocC for SPM packages exposed to consumers (mandatory) and app's internal core modules. For app feature code that one team uses, inline doc comments are usually enough — don't over-invest.

---

## Crash reporting

### Xcode Organizer (source of truth for SwiftUI crashes)

- **Xcode Organizer is the source of truth for SwiftUI crash symbolication.** Third-party tools often cannot symbolicate SwiftUI's compiler-generated frames because Apple doesn't ship SwiftUI symbols.
- Free. Window → Organizer → Crashes. Covers App Store and TestFlight builds.
- MetricKit (`MXMetricManager`) provides aggregated diagnostics — crashes, hangs, disk writes, scrolling hitches — sampled across users daily.
- For SwiftData 0xdead10cc detection (background-task termination): monitor via Crashlytics/Sentry, cross-reference against Organizer.

### Third-party (pick one)

- **Sentry** — best for cross-platform (mobile + backend), source maps, release health tracking, AI-grouped issues, generous free tier.
- **Firebase Crashlytics** — fastest integration, free unlimited, weaker filtering than Sentry.
- **Bugsnag / Embrace** — enterprise tier features (RUM, session replay).

Always cross-reference Organizer for SwiftUI-stack crashes that third-party tools render as unsymbolicated hex.

### Practical setup

- Automatic dSYM upload via your CI (Sentry: `sentry-cli upload-dif`; Crashlytics: built-in Run Script Phase; Xcode Cloud: `ci_post_xcodebuild.sh` or `ci_post_archive.sh`).
- Track **crash-free user %** as a release KPI.
- Use `record(error:)` for caught-but-noteworthy errors; `setCustomValue` for build flags and session context.

---

## Workflow commandments

1. **Swift Testing for new tests.** XCTest only where Swift Testing genuinely cannot help (UI, performance, Obj-C). Migrate legacy XCTest incrementally — coexist.
2. **Constructor inject dependencies** — never reach for singletons in tests. `URLSession.shared`, `ModelContainer.shared`, ad-hoc `Logger()` instances are testability landmines.
3. **URLProtocol for URLSession mocking** — never global state in production code. Pick URLProtocol or protocol-based DI; never both.
4. **In-memory ModelContainer for SwiftData tests** — `isStoredInMemoryOnly: true`. Use the real container with test data, not a mock.
5. **Don't mock actors** — use real instances with a fake clock or fake dependencies if needed.
6. **Don't mock `@Observable`** — construct the real instance with test inputs and inject fake collaborators through its initializer.
7. **UI tests by `accessibilityIdentifier`**, not labels. Namespace as `"screen.element"`. Free a11y compliance, free VoiceOver labels, free recorder support.
8. **Run `try app.performAccessibilityAudit()` in CI.** A SwiftUI app shipping in 2026 without this is a bug-report factory.
9. **Strip `Self._printChanges()` before ship** — `#if DEBUG` it. Underscore-prefixed is private API; Apple could remove it in a point release.
10. **Use Instruments SwiftUI template + Cause & Effect Graph** for view performance — not guesswork. Profile before optimizing.

---

## Real-world testimony

- A widely-shared post titled *SwiftUI in Production after 9 months* condenses the workflow as: *"Start small, profile often, and keep views tiny."*
- A high-score community post titled *Apple barely documents how SwiftUI actually works* explains the gap: *"I spent a long time digging through WWDC videos and running my own tests to understand the AttributeGraph — the private framework that drives every SwiftUI update."* The AttributeGraph is the system you're profiling in Instruments — understanding it is the difference between guessing and debugging.
- A widely-shared community post on Apple Intelligence on-device testing: *"Apple Intelligence isn't very intelligent."* Relevant when testing Foundation Models output — assert on shapes and behavior, not specific generated strings. The model output is non-deterministic across versions.

---

## Anti-patterns

- **XCTest as default for new unit tests.** Swift Testing wins on syntax, parameterization, async, and `@MainActor`. XCTest stays only for UI / performance / Obj-C.
- **Mocking the framework.** "Does `@State` re-render when I set it?" is Apple's test, not yours. Test your behavior, your invariants, your error paths — not Apple's framework guarantees.
- **Snapshot testing every screen.** Reserve for design-system primitives. Whole-screen snapshots churn red on every Xcode update and drown real visual regressions in noise.
- **`sleep(_:)` in UI tests.** The #1 source of UI-test CI flakes. Use `waitForExistence(timeout:)` or `XCTNSPredicateExpectation`.
- **Hardcoded device sizes in snapshot tests** (`as: .image(on: .iPhone16Pro)`). Drifts across Xcode versions. Use `.fixed(width:height:)`.
- **Ad-hoc URLSession mocking.** Pick URLProtocol or protocol-based DI and stick with one. Mixing both creates parallel-test landmines.
- **`PreviewProvider` for new code.** Use `#Preview`. `PreviewProvider` is deprecated boilerplate.
- **Test plans ignored on CI.** Always run on CI with the same plan you run locally — otherwise CI passes while developers see fails (or vice versa).
- **Shipping `Self._printChanges()` without `#if DEBUG`.** Underscore-prefixed is private API. App Review accepts it today; Apple could remove it tomorrow.
- **`print()` for production logging.** Bypasses unified logging, leaks PII in shipped builds, won't show in `OSLogStore`. Use `os.Logger`.
- **Symbolicating SwiftUI crashes with third-party tools only.** Xcode Organizer is the source of truth — third-party tools often can't symbolicate SwiftUI frames. Cross-reference always.
- **Singletons instead of constructor injection.** Untestable. `URLSession.shared`, `ModelContainer.shared`, ad-hoc `Logger()` in business logic — all replace with injected dependencies.
- **Flaky tests masked with `.disabled` or retries.** Use `withKnownIssue(isIntermittent: true)` for legitimately external flakes only; put the underlying flake on the backlog.
- **Skipping accessibility audits.** Bake `try app.performAccessibilityAudit()` into every UI test from day one.
- **Coverage as a target, not a floor.** 70-80% with intent beats 100% with `XCTAssertNotNil`.
- **CI without dSYM upload.** Every crash without dSYMs is a wasted bug report. Bake automatic dSYM upload into CI on day one.
