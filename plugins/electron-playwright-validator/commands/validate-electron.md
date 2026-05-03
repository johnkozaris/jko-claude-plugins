---
description: Launch the Electron app, validate all major views render correctly, and report pass/fail for each step
argument-hint: "[tab-names...]"
allowed-tools:
  - Bash
  - Read
user-invocable: true
---

# Validate Electron

Launch the Electron app via `e-cli`, click through each major view tab, and verify each renders correctly. Report a structured pass/fail summary.

## Steps

### 1. Build and launch

```bash
e-cli launch
```

If this fails, report the error and stop.

### 2. Initial state validation

```bash
e-cli snapshot
e-cli screenshot
```

Read the screenshot file with the `Read` tool to visually inspect the initial state.

**Pass criteria:** The accessibility tree shows a `WebArea` with navigation tabs and a main content area. The screenshot shows a rendered UI (not a blank/white screen).

### 3. Determine tabs to validate

If `$ARGUMENTS` is provided, use those as tab names. Otherwise, extract tab names from the accessibility tree (look for `role=tab` nodes in the snapshot output).

Common tabs in Kodosi: `Sessions`, `Rooms`, `Friends`, `Public`.

### 4. Click through each tab

For each tab:

```bash
e-cli click 'role=tab[name="<TabName>"]'
e-cli wait 'role=heading[name="<TabName>"]' 5000
e-cli snapshot
```

**Pass criteria:** After clicking, the snapshot shows content relevant to that tab (heading, list, or content area changes). The previously active tab is no longer `pressed=true` and the clicked tab now is.

If `e-cli wait` times out, the tab still passes if the snapshot shows the tab is `pressed=true` (the heading may differ from the tab name).

### 5. Screenshot final state

```bash
e-cli screenshot
```

Read the file with `Read` tool for visual confirmation.

### 6. Close

```bash
e-cli close
```

### 7. Report

Print a summary table:

```
## Validation Report

| Step                | Status |
|---------------------|--------|
| Launch              | PASS   |
| Initial render      | PASS   |
| Tab: Sessions       | PASS   |
| Tab: Rooms          | PASS   |
| Tab: Friends        | PASS   |
| Tab: Public         | FAIL   |
| Clean close         | PASS   |

Result: 6/7 passed
```

If any step fails, include the error message or unexpected snapshot output.

**NEVER** leave the app running after this command completes. Always run `e-cli close` even if earlier steps fail.
