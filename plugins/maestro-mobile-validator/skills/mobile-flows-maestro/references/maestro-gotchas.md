# Maestro gotchas

## Diagnose before changing the flow

Classify a failure from evidence:

- **App defect:** the expected product state is absent in the hierarchy and
  screenshots/logs show the app did not reach it.
- **Selector problem:** the state exists but the matcher resolves zero or
  multiple elements, or required accessibility metadata is not exposed.
- **Timing sensitivity:** the state eventually appears; identify the product
  operation and expected bound before extending the wait.
- **State leak:** Keychain, app data, permissions, backend fixtures, or device
  state differ between runs.
- **Environment failure:** app install/launch, driver, Java, simulator/emulator,
  network, or tooling failed before the product assertion.

Retain the failed step, debug output, device identifier, hierarchy, and the
smallest visual evidence needed to distinguish these cases.

## Device and platform state

Use explicit device identifiers when multiple devices are attached. For
Android, accept only `adb devices` entries in the `device` state and distinguish
emulators from intentionally selected physical devices. For iOS, wait for the
chosen simulator to finish booting rather than masking boot errors.

Maestro permission configuration covers supported iOS and Android permissions,
including custom Android permission IDs. Use `adb -s <serial> shell pm ...` or
`xcrun simctl ...` only when current Maestro documentation does not expose the
required state.

System authentication, share sheets, cross-app handoffs, biometrics, WebViews,
and push delivery depend on platform-owned UI. Inspect the live hierarchy and
tool support before assuming they are automatable in the same way as app-owned
controls.

## CI facts worth preserving

- Pin Maestro using its current supported installation mechanism rather than
  downloading an unspecified latest build.
- Make device boot readiness an explicit step and pass the selected device to
  Maestro.
- Use JUnit output when the CI system consumes test reports.
- Retain the Maestro test artifact directory on failure instead of assuming a
  stable nested screenshot path.
- Upload flows and their referenced JavaScript files together.
- Keep secrets in the CI platform's secret store and pass only the values the
  flow requires.

Generate host-specific CI YAML from the repository's existing workflow and
current Maestro docs rather than copying a pinned generic workflow.
