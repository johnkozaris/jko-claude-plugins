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

### 0. Resolve the tool

`e-cli` is NOT on PATH — it ships inside this plugin:

```bash
E_CLI="${CLAUDE_PLUGIN_ROOT}/bin/e-cli"
```

Every `e-cli` below means `"$E_CLI"`.

### 1. Build and launch

```bash
"$E_CLI" launch
```

If this fails, report the error and stop.

### 2. Initial state validation

```bash
"$E_CLI" snapshot
"$E_CLI" screenshot
```

Read the screenshot file with the `Read` tool to visually inspect the initial state.

**Pass criteria:** The accessibility tree shows a `WebArea` with navigation tabs and a main content area. The screenshot shows a rendered UI (not a blank/white screen).

### 3. Determine tabs to validate

If `$ARGUMENTS` is provided, use those as tab names. Otherwise, extract tab names from the accessibility tree (look for `role=tab` nodes in the snapshot output) — never guess tab names; the snapshot is the source of truth.

### 4. Click through each tab

For each tab:

```bash
"$E_CLI" click 'role=tab[name="<TabName>"]'
"$E_CLI" wait 'role=heading[name="<TabName>"]' 5000
"$E_CLI" snapshot
```

**Pass criteria:** After clicking, the snapshot shows content relevant to that tab (heading, list, or content area changes). The previously active tab is no longer `pressed=true` and the clicked tab now is.

If `e-cli wait` times out, the tab still passes if the snapshot shows the tab is `pressed=true` (the heading may differ from the tab name).

### 5. Screenshot final state

```bash
"$E_CLI" screenshot
```

Read the file with `Read` tool for visual confirmation.

### 6. Close

```bash
"$E_CLI" close
```

### 7. Report

Print a summary table:

```
## Validation Report

| Step                | Status |
|---------------------|--------|
| Launch              | PASS   |
| Initial render      | PASS   |
| Tab: <TabName1>     | PASS   |
| Tab: <TabName2>     | FAIL   |
| Clean close         | PASS   |

Result: 4/5 passed
```

If any step fails, include the error message or unexpected snapshot output — the raw output is the evidence; a bare PASS/FAIL without it is not acceptable.

**NEVER** leave the app running after this command completes. Always run `"$E_CLI" close` even if earlier steps fail.
