# Maestro Gotchas and CI Patterns

Extended reference for edge cases that surface once you have >3 flows.

## Flakiness root causes

In rough order of how often they bite:

1. **Missing waits after navigation.** Use `extendedWaitUntil` instead of bare `assertVisible` after any action that triggers a transition.
2. **testID drift.** Developers rename test IDs without updating flows. Pin testIDs in a shared constants file referenced from both app code and flow YAML where possible.
3. **Keychain bleed between flows.** `clearState` doesn't clear iOS Keychain. Use `clearKeychain: true` or wipe the simulator between independent flows.
4. **System permission prompts surfacing late.** If a permission is triggered mid-flow (not at launch), Maestro may not see it. Grant all perms at `launchApp` time.
5. **Font loading races.** Custom fonts on first launch can delay text rendering by 100-500ms. `extendedWaitUntil` saves you.
6. **Network fixture drift.** If the flow hits a real backend, data changes over time. Use mock servers or seeded test accounts.
7. **Screen scale differences.** Coordinate-based taps work on one simulator size, fail on another. Always prefer text or testID targeting.

## CI patterns

### GitHub Actions — iOS

```yaml
name: mobile-e2e
on: [pull_request]
jobs:
  maestro:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - name: Install Maestro
        run: curl -Ls "https://get.maestro.mobile.dev" | bash
      - run: echo "$HOME/.maestro/bin" >> $GITHUB_PATH
      - name: Boot simulator
        run: |
          xcrun simctl boot "iPhone 15" || true
          xcrun simctl list
      - name: Build app
        run: xcodebuild -scheme MyApp -destination 'platform=iOS Simulator,name=iPhone 15' build
      - name: Install app on sim
        run: xcrun simctl install booted path/to/MyApp.app
      - name: Run Maestro flows
        run: maestro test --format junit --output report.xml tests/maestro/
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: maestro-report
          path: report.xml
      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: maestro-screenshots
          path: ~/.maestro/tests/*/screenshots/
```

### Retry on flakes

Historically, automatic retries were a Maestro Cloud / workspace-config feature, not an open-source CLI flag. Check `maestro test --help` on the installed version before assuming a `--retry` flag exists; if it doesn't, wrap the invocation in a shell retry loop or use the `retry` YAML command (verify with `maestro --help` / MCP `listDocumentation` first).

Use sparingly — retries hide real regressions. Prefer fixing the flake.

## Debugging a flaky flow

1. **Reproduce locally.** `maestro test --continuous flow.yaml` — Maestro re-runs on file save. Tighten the flow until it passes 10 runs in a row.
2. **Inspect hierarchy at the failing step.** Add a `takeScreenshot` right before the failing command, and run `maestro hierarchy` in a second terminal while the app sits in the failing state to dump the live view tree. (Do not invent JS APIs inside `runScript` — the GraalJS sandbox exposes only the documented `output`/`http`/env surface; check `maestro --help` or MCP `listDocumentation` before using anything else.)
3. **Increase timeouts temporarily.** If boosting `extendedWaitUntil.timeout` from 5000 to 30000 fixes it, the UI is slow or the network call is slow — not a Maestro bug.
4. **Disable animations on the simulator.** `xcrun simctl spawn booted defaults write com.apple.UIKit UIAnimationsEnabled NO` — faster and more deterministic.

## Simulator management

```bash
# List available devices
xcrun simctl list devices

# Boot a specific device
xcrun simctl boot "iPhone 15 Pro"

# Erase (clean slate — use between flow suites)
xcrun simctl shutdown "iPhone 15 Pro"
xcrun simctl erase "iPhone 15 Pro"
xcrun simctl boot "iPhone 15 Pro"

# Take a screenshot from outside a flow
xcrun simctl io booted screenshot /tmp/sim.png

# Record video
xcrun simctl io booted recordVideo --codec=h264 /tmp/session.mp4
# Ctrl-C to stop
```

## Android-specific notes

- Use `adb` for the equivalent of `simctl` (install, screenshot, input)
- Android app permissions are granular per-install; Maestro's `launchApp.permissions` doesn't apply to Android the same way — grant explicitly via `adb shell pm grant <pkg> <permission>` in a pre-test hook
- Emulator boot times are slower than iOS sim — bake in 60s extra at CI startup

## MCP vs CLI — when to use which

| Situation | Reach for |
|---|---|
| Exploring a new app's UI interactively | MCP — lets Claude read hierarchy, screenshot, try things |
| Running an existing flow suite | CLI — deterministic, faster, integrates with CI |
| Generating a flow from natural language | MCP — Claude can iterate with real UI feedback |
| Debugging why a flow fails in CI but not locally | CLI + `--debug` output |
| Running in parallel on multiple simulators | CLI with `--device <udid>` per invocation |

## What Maestro does NOT handle well

- **Cross-app handoff** (launch another app, do something, return) — unreliable, especially on iOS where the switcher animation varies
- **Live biometric prompts** without simctl pre-enrollment
- **WebView content in hybrid apps** — Maestro sees the native container but has limited access to WebView DOM. For Cordova/Ionic/Capacitor heavy webview testing, consider Appium with `XCUITest` driver + Safari Web Inspector instead
- **Push notifications** — deliver via `xcrun simctl push`, but timing them to land mid-flow is manual
- **Native share sheets** — OS-controlled UI that may not expose elements to Maestro reliably
