---
description: Launch a macOS app, click through each major view, and report pass/fail with a per-step visual critique
argument-hint: "<bundle-id> [view-names...]"
allowed-tools:
  - Bash
  - Read
user-invocable: true
---

# Validate macOS App

Launch the target macOS app via `peekaboo`, drive each major view, capture
an annotated snapshot of each, **read the PNG inline and critique it**,
then report a structured pass/fail summary.

`$ARGUMENTS` is `<bundle-id> [view-names...]`. The bundle ID is required
(e.g. `com.example.myapp`). View names are optional — if omitted, discover
tab names from the AX tree.

This command is the macOS-native equivalent of `/validate-electron`. It
**does not stop at "the click succeeded"**: every captured view gets a
short critique against the rubric in the `peekaboo` skill.

## Steps

### 0. Pre-flight

```bash
peekaboo --version
peekaboo list permissions --json | jq '.permissions[]|{name,isGranted}'
```

If permissions are missing OR `peekaboo` is not installed, stop with an
actionable error (point at `/peekaboo-doctor`).

### 1. Launch

Parse `$ARGUMENTS` — first token is `BID`, the rest are view names.

```bash
BID="$1"; shift; VIEWS="$@"
peekaboo app launch --bundle-id "$BID" --wait-until-ready
sleep 0.5
```

If launch fails, report and stop.

### 2. Initial state — snapshot + critique

```bash
peekaboo see --app "$BID" --json --annotate \
  --path /tmp/validate-initial.png > /tmp/validate-initial.json
SID=$(jq -r .data.snapshot_id /tmp/validate-initial.json)
```

Use the `Read` tool on `/tmp/validate-initial_annotated.png`. Write a
short scored critique (alignment, hierarchy, contrast, copy clarity).

**Pass criteria**: the AX tree has `element_count > 0` and the rendered
PNG is not a blank/white surface.

### 3. Discover views

If `$VIEWS` is empty, derive the list from the AX tree:

```bash
jq -r '.data.ui_elements[]
       | select(.role=="tab" or .role=="button" and .is_actionable)
       | .identifier // .label // .title' /tmp/validate-initial.json
```

Pick the obvious top-level navigation entries.

### 4. Walk each view

For each view name:

```bash
peekaboo click "$NAME" --app "$BID"
sleep 0.5
peekaboo see --app "$BID" --json --annotate \
  --path /tmp/validate-${NAME}.png > /tmp/validate-${NAME}.json
```

Then `Read` the annotated PNG and write a per-view critique. Pass criteria:
- The click resolves (no `ELEMENT_NOT_FOUND`)
- The post-click snapshot's content meaningfully changed
- The PNG renders correctly (no obvious layout breakage, blank panes, or
  truncated copy)

If targeting by visible label is flaky, switch to AX-identifier targeting:

```bash
ID=$(jq -r --arg I "$NAME" \
   '.data.ui_elements[]|select(.identifier==$I).id' \
   /tmp/validate-initial.json)
peekaboo click --on "$ID" --snapshot "$SID" --app "$BID"
```

### 5. Quit

```bash
peekaboo app quit --app "$BID"
```

Always run this — even if earlier steps fail.

### 6. Report

Print a Markdown summary:

```
## Validation Report — <bundle-id>

| Step                | Status | Critique score |
|---------------------|--------|----------------|
| Launch              | PASS   | —              |
| Initial render      | PASS   | 4.2 / 5        |
| View: Sessions      | PASS   | 4.0 / 5        |
| View: Friends       | PASS   | 3.6 / 5        |
| View: Settings      | FAIL   | —              |
| Clean quit          | PASS   | —              |

Result: 5/6 passed.
Top issues across views: <bullets>.
```

If any step failed, include the failing command's error output and the
relevant snapshot path.

**NEVER** leave the app running. Always quit, even on failure.
