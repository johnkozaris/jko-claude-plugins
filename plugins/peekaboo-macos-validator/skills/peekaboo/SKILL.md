---
name: peekaboo
description: "Drive, observe, and critique a running native macOS app (SwiftUI/AppKit) end-to-end via Peekaboo — the macOS-native equivalent of Playwright + visual-regression review. Use when the user mentions the Mac app's UI, screens, screenshots, appearance, layout, accessibility identifiers, clicking, typing, or menus — or says 'see what it looks like', 'show me the UI', 'click X', 'take a screenshot of the app', 'critique the design', 'drive the app'. Use after a SwiftUI/AppKit edit or rebuild, and before claiming a UI feature done. NOT for Electron or web apps (use the electron-playwright validator), NOT for iOS simulators (use the Maestro validator), NOT for purely backend work. Screenshots go to disk and each one is read by a separate same-model sub-agent, so the driver's context never fills with pixels; no external AI keys are used."
homepage: https://peekaboo.sh
metadata:
  os: ["darwin"]
  requires:
    bins: ["peekaboo"]
    permissions: ["Screen Recording", "Accessibility", "Event Synthesizing for background keyboard input"]
  install:
    - kind: brew
      formula: steipete/tap/peekaboo
---

# Peekaboo - drive, observe, and critique macOS apps

The job is not just "is this clickable?" Verify that the workflow works and
that every meaningful state looks intentional.

Invoke `peekaboo` directly. Build orchestration belongs to the project's task
runner; UI observation and driving belong here.

**No external AI keys, and never pixels in your own context.** You are the
LLM, but do not `view` screenshots yourself: every image you load is re-sent
on every later turn and blows up the context window. Save each screenshot with
`--path`, then hand its path to a **separate sub-agent running your same
model** — one sub-agent per image — which opens it and returns a short text
report. Do not call `peekaboo agent`, `peekaboo analyze`, or any `--analyze`
mode. Deterministic CLI commands, parsed JSON, visible postconditions,
retained artifacts, and delegated visual reads are the verification contract.

## When to invoke

- After modifying a SwiftUI/AppKit view or interaction.
- When asked to show, inspect, click through, screenshot, or critique a native
  Mac app.
- When debugging a native UI failure or validating a user journey.
- Before claiming a native macOS UI feature complete.
- Skip purely backend work, Electron/web apps, and iOS simulator flows.

## Prerequisites

```bash
peekaboo --version                               # require >= 3.4
peekaboo permissions status --all-sources --json
```

Use the latest stable Peekaboo on macOS 15.0+. Install it when missing:

```bash
brew install steipete/tap/peekaboo
```

Require Screen Recording and Accessibility on the **selected runtime** shown
by `permissions status`. Event Synthesizing is additionally required for
background typing, hotkeys, key presses, paste, coordinate clicks, and
synthetic click fallback:

```bash
peekaboo permissions request-event-synthesizing
```

In Claude Code, Copilot CLI, SSH, LaunchAgents, or other subprocess hosts, the
selected runtime may be Peekaboo's reusable daemon or app Bridge rather than
the terminal itself.

## Core loop

Create one explicit artifact directory:

```bash
ARTIFACT_DIR="${PEEKABOO_ARTIFACT_DIR:-$PWD/.artifacts/peekaboo}"
mkdir -p "$ARTIFACT_DIR"
```

1. **Build** with the project's existing task runner.
2. **Launch/drive** the app into the state under test.
3. **Observe** with `see` to get pixels, AX metadata, a snapshot ID, and
   opaque element IDs.
4. **Act once** using the most semantic reliable action.
5. **Observe again** after the mutation; never keep using an old snapshot.
6. **Delegate the annotated PNG to a one-shot same-model sub-agent** (see
   below) and critique from its text report, not just the command output.
7. **Fix and repeat** until both behavior and visible quality pass.

Read [agent-runtime guidance](references/agent-runtime.md) for host selection,
permissions, background delivery, coordinate behavior, version-specific
gotchas, and recovery. Read
[visual-verification guidance](references/visual-verification.md) for the
critique rubric, state coverage, responsive checks, and evidence format.

## Read screenshots without blowing up context

Never open a screenshot in your own context — not `view`, not `Read`, not an
inline image. Each pixel buffer you load is resent on every later turn, and
that is exactly what makes long UI sessions unusable.

For **every** PNG you need to judge:

1. Keep the file on disk (always capture with `--path`).
2. Spawn a **fresh sub-agent running the same model you are** and give it just
   that one image path plus a specific question. It reads the pixels in its own
   disposable context and returns a short text report — verbatim on-screen
   text, element positions, and a scored critique.
3. **One image per sub-agent.** Never batch several PNGs into one sub-agent,
   and never reuse a sub-agent for the next image — each screenshot gets its
   own, so no pixels ever accumulate anywhere.
4. Act on the returned text.

Host mechanics:

- **Copilot CLI:** `task` tool → `agent_type: "explore"` (it has `view`),
  `model:` set to the model you are currently running, `prompt:` =
  "view `<abs path>` and report …".
- **Claude Code:** `Task` tool → a `general-purpose` sub-agent that `Read`s the
  image path and returns text; run it on your model.
- Only if the host has no sub-agent/Task capability at all: `view` exactly one
  cropped image, then move on — never in a loop.

This keeps the driver's context text-only no matter how many states you capture.

## Find and target the app

Prefer the stable bundle ID over a display name or Xcode scheme:

```bash
peekaboo list apps --json \
  | jq '.data.applications[]
        | select(.name | test("YourApp"; "i"))
        | {name,bundleIdentifier,processIdentifier}'

peekaboo app launch --bundle-id "$BID" --wait-until-ready
```

Running process names often use `CFBundleName`, which can differ from the
scheme. Cache confirmed bundle/display-name caveats in the project's
`AGENTS.md` or `CLAUDE.md`.

## Observe efficiently

Use the cheapest observation that answers the question:

```bash
# AX metadata only; no screenshot artifact
peekaboo inspect-ui --app-target "$BID" --json

# Default validator observation: screenshot + AX metadata
peekaboo see --app "$BID" --json --annotate \
  --path "$ARTIFACT_DIR/state.png" > "$ARTIFACT_DIR/state.json"

# Pixels only for AX-opaque custom rendering
peekaboo image --mode window --app "$BID" --retina \
  --path "$ARTIFACT_DIR/raw.png"
```

Always pass `--path` when the agent must inspect or retain a screenshot.
JSON-only `see` without a path can keep the image solely in managed snapshot
storage and return empty direct screenshot path fields.

Treat every element ID as an opaque string:

```bash
SID=$(jq -r .data.snapshot_id "$ARTIFACT_DIR/state.json")
ID=$(jq -r \
  '.data.ui_elements[]
   | select(.identifier=="header.utility.settings")
   | .id' "$ARTIFACT_DIR/state.json")

peekaboo click --on "$ID" --snapshot "$SID" --app "$BID"
```

Never generate, increment, or infer meaning from an ID. After `click`, `type`,
`hotkey`, `paste`, menu/dialog/app/window operations, or any other mutation,
run `see` or `inspect-ui` again before the next ID-based action.

## Choose the action

Prefer this order:

1. `set-value` for a settable, non-secure field when direct replacement is the
   intended behavior.
2. `click --on "$ID" --snapshot "$SID"` for normal button activation.
3. `perform-action` for a specific AX action such as `AXShowMenu`,
   `AXIncrement`, or `AXDecrement`.
4. A label/query when no stable identifier exists.
5. Coordinates only for custom-drawn or AX-invisible surfaces.

```bash
peekaboo set-value "hello" --on "$FIELD_ID" --snapshot "$SID"
peekaboo perform-action --on "$STEPPER_ID" \
  --action AXIncrement --snapshot "$SID"
peekaboo click "Settings" --app "$BID" --wait-for 8000
```

Targeted `click`, `type`, `press`, `hotkey`, and `paste` use background
delivery by default. Preserve that non-interference. Add `--foreground` only
when the app needs a focused key window, a real mouse event, a Space switch,
or a double-click.

With an app/PID/window target, `--coords x,y` is relative to that window.
Use `--global-coords` only for intentional screen coordinates. Normalize and
verify window geometry before coordinate fallback.

Do not default to `--no-remote` or a forced capture engine. Background agent
sessions can report valid local TCC state yet capture wallpaper/redacted
pixels; prefer a permissioned daemon/Bridge.

### Native surfaces worth using

Do not rebuild these flows from coordinate clicks:

| Surface | Prefer |
|---|---|
| App lifecycle | `peekaboo app launch`, `app quit`, `app focus` |
| Menus | `peekaboo menu list`, then `menu click --path "View > Sidebar"` |
| Open/save panels | `peekaboo dialog list`, `dialog input`, `dialog file` |
| Window geometry | `peekaboo window set-bounds`, then read back and verify |
| Clipboard | `peekaboo clipboard get/set/clear`, then `paste` |
| URLs/deep links | `peekaboo open "scheme://..."` |
| Long scenarios | `peekaboo run <scenario>` when the project already owns one |

Use live `peekaboo <command> --help` for current flags. Keep product-specific
targeting and bundle-ID facts in the app repository, not in this generic skill.

### Recovery ladder

Recover from explicit evidence instead of retrying the same command:

1. On `ELEMENT_NOT_FOUND`, stale snapshot, or changed focus, capture a fresh
   `see`/`inspect-ui` result and resolve the target again.
2. If JSON reports a truncated AX tree, increase `--max-depth`,
   `--max-elements`, or `--max-children` only enough for the target surface.
3. If background delivery reports that the action needs real input, retry once
   with `--foreground` and verify the target app became frontmost before input.
4. If capture is blank, wallpaper-only, or redacted, inspect selected-source,
   Bridge, and daemon diagnostics before changing engines.
5. If AX cannot expose custom rendering, use a raw image, normalize window
   geometry, and perform one verified coordinate fallback.
6. If a command reports success but the postcondition did not change, record
   it as a failure. Do not turn a no-op into success through repeated clicks.

## Instrument the app for stable validation

Tag interactive SwiftUI/AppKit controls with stable
`.accessibilityIdentifier(...)` values:

```swift
Button("Settings") { showSettings = true }
    .accessibilityIdentifier("header.utility.settings")
```

Use a documented namespace such as `header.tab.sessions`,
`panel.settings.save`, or `row.<id>.delete`. If a copy-based click becomes
repeated, add an identifier instead of preserving brittle text targeting.

Avoid applying one identifier to a composite parent: descendants can inherit
duplicates. Tag the precise child or use
`.accessibilityElement(children: .contain)`. Give icon-only buttons an
`.accessibilityLabel(...)`.

These identifiers also improve XCUITest.

## Verify pixels and behavior

After every meaningful state:

1. Parse the JSON envelope and assert `success`.
2. Assert a product postcondition in the fresh AX tree or app output.
3. Hand `$ARTIFACT_DIR/state_annotated.png` to its own same-model sub-agent
   and read back its text report (never `view` it yourself).
4. Critique intent, hierarchy, alignment, spacing, contrast, copy, and edge
   states.
5. Rank concrete fixes and repeat after changes.

A successful click is not a successful feature. Cover relevant initial,
loading, empty, populated, error, disabled, overflow, focus, modal, and
responsive states.

For a transition or flaky sequence, record the command itself:

```bash
peekaboo capture action --app "$BID" --duration-limit 10 \
  --post-roll-ms 800 --path "$ARTIFACT_DIR/action" \
  --video-out "$ARTIFACT_DIR/action.mp4" --json -- \
  peekaboo hotkey --keys "cmd,b" --app "$BID"
```

Parse the child exit details and inspect `contact.png`, `metadata.json`, and
the MP4. Artifact creation alone does not prove the expected transition.

## Reliability and safety

- Prefer state assertions over blind sleeps. If animation has no AX-visible
  completion state, use one short bounded settle and recapture.
- If a fresh `see` still shows a just-mutated tree, wait briefly: rapid
  captures can reuse a short-lived AX cache.
- If capture or input works in Terminal but not the agent, inspect:

  ```bash
  peekaboo permissions status --all-sources --json
  peekaboo bridge status --verbose --json
  peekaboo daemon status --json
  ```

- Custom-rendered terminals/canvases can be AX-opaque. Verify their pixels
  directly.
- `set-value` rejects secure fields. Keep secrets out of shell arguments,
  JSON, screenshots, and recordings. Require user mediation for credentials
  and explicit confirmation before irreversible actions.
- Each `see` creates `~/.peekaboo/snapshots/<UUID>/`. Clean old entries with
  `peekaboo clean --older-than 24`.

## When to use XCUITest

Use Peekaboo for interactive product validation: "does this workflow work and
look right?" Use XCUITest for deterministic regression gates. They complement
each other; do not replace durable automated tests with an agent-only flow.

## Live discovery

Prefer current CLI help over copied command catalogs:

```bash
peekaboo learn
peekaboo <command> --help
peekaboo permissions status --all-sources --json
peekaboo bridge status --verbose --json
peekaboo daemon status --json
```
