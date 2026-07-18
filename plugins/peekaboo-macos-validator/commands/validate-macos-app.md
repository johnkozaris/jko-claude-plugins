---
description: Launch a macOS app, click through each major view, and report pass/fail with a per-step visual critique
argument-hint: "<bundle-id> [view-names...]"
allowed-tools:
  - Bash
  - Task
user-invocable: true
---

# Validate macOS App

Launch the target macOS app via `peekaboo`, drive each major view, capture
an annotated snapshot of each, **delegate every snapshot to its own one-shot
sub-agent (running your same model) for reading and critique**, then report a
structured pass/fail summary. Never `Read`/`view` a screenshot in this
command's own context — one fresh sub-agent per PNG keeps the walk from filling
the window with pixels.

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
peekaboo permissions status --all-sources --json
ARTIFACT_DIR="${PEEKABOO_ARTIFACT_DIR:-$PWD/.artifacts/peekaboo}"
mkdir -p "$ARTIFACT_DIR"
```

Require macOS 15+, Peekaboo 3.4+, Screen Recording, and Accessibility on the
selected runtime. Require Event Synthesizing only if the planned flow uses
background keyboard input, hotkeys, key presses, paste, or coordinate clicks.
If a requirement is missing, stop with an actionable error and point at
`/peekaboo-doctor`.

### 1. Launch

Parse `$ARGUMENTS` — first token is `BID`, the rest are view names.

```bash
BID="$1"; shift; VIEWS="$@"
peekaboo app launch --bundle-id "$BID" --wait-until-ready
```

If launch fails, report and stop.

### 2. Initial state — snapshot + critique

```bash
peekaboo see --app "$BID" --json --annotate \
  --path "$ARTIFACT_DIR/validate-initial.png" \
  > "$ARTIFACT_DIR/validate-initial.json"
SID=$(jq -r .data.snapshot_id "$ARTIFACT_DIR/validate-initial.json")
```

Spawn a sub-agent running your same model, give it only
`$ARTIFACT_DIR/validate-initial_annotated.png`, and have it return a short
scored critique (alignment, hierarchy, contrast, copy clarity). Do not read the
PNG in this command's own context.

**Pass criteria**: the AX tree has `element_count > 0` and the rendered
PNG is not a blank/white surface.

### 3. Discover views

If `$VIEWS` is empty, derive the list from the AX tree:

```bash
jq -r '.data.ui_elements[]
       | select(.role=="tab" or .role=="button" and .is_actionable)
       | .identifier // .label // .title' \
       "$ARTIFACT_DIR/validate-initial.json"
```

Pick the obvious top-level navigation entries.

### 4. Walk each view

Set `CURRENT_JSON` to the initial snapshot before the loop. For each view,
resolve the target from that current snapshot:

```bash
CURRENT_JSON="$ARTIFACT_DIR/validate-initial.json"
SID=$(jq -r .data.snapshot_id "$CURRENT_JSON")
ID=$(jq -r --arg I "$IDENTIFIER" \
   '.data.ui_elements[] | select(.identifier==$I) | .id' "$CURRENT_JSON")
peekaboo click --on "$ID" --snapshot "$SID" --app "$BID"
```

Treat `ID` as opaque. If the view has no stable identifier, use
`peekaboo click "$NAME" --app "$BID" --wait-for 8000` as the fallback.

Immediately capture the post-action state and make it the current snapshot for
the next iteration:

```bash
peekaboo see --app "$BID" --json --annotate \
  --path "$ARTIFACT_DIR/validate-${INDEX}.png" \
  > "$ARTIFACT_DIR/validate-${INDEX}.json"
CURRENT_JSON="$ARTIFACT_DIR/validate-${INDEX}.json"
```

After any state-changing action, capture a new snapshot before the next
ID-based action. Never reuse the initial `SID` across the full walk.

Use `set-value` for settable, non-secure fields when replacement semantics are
correct. Use `perform-action` only when a specific AX action is needed. Keep
normal button activation on `click`; use coordinates only as a last resort.

Then hand the new annotated PNG to its own fresh same-model sub-agent (one per
view) and write a per-view critique from its text report. Pass criteria:
- The click resolves (no `ELEMENT_NOT_FOUND`)
- The post-click snapshot's content meaningfully changed
- The PNG renders correctly (no obvious layout breakage, blank panes, or
  truncated copy)

For animations or a flaky transition, wrap the action with `peekaboo capture
action --json -- <command>` and inspect its child exit details,
`metadata.json`, and `contact.png`.

### 5. Quit

```bash
peekaboo app quit --app "$BID"
```

Always run this, even if earlier steps fail. Do not exit before reporting
whether cleanup succeeded.

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
