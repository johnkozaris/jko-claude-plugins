---
name: peekaboo
description: "Drive, observe, and critique any running macOS app end-to-end via Peekaboo — the macOS-native equivalent of Playwright + visual-regression review for SwiftUI/AppKit. Use proactively whenever the user mentions UI, screens, screenshots, the app's appearance, layout, spacing, alignment, copy/labels, sidebar, panels, settings, theme, accessibility identifiers, clicking, typing, menus, file pickers, responsive sizing, or animations — or says any of: 'verify it works', 'see what it looks like', 'show me the UI', 'click X', 'take a screenshot', 'critique the design', 'is this still balanced', 'drive the app'. Use after any SwiftUI/AppKit edit, after a rebuild, when fixing a UI bug, or before claiming a UI feature done — even when the user does not explicitly ask. The agent reads pixels itself via `view`; no external AI keys are used."
homepage: https://peekaboo.boo
metadata:
  os: ["darwin"]
  requires:
    bins: ["peekaboo"]
    permissions: ["Screen Recording", "Accessibility"]
  install:
    - kind: brew
      formula: steipete/tap/peekaboo
---

# Peekaboo — drive, observe, and **critique** any macOS app

> **The job is not "is this clickable". The job is "does it work, *and* does
> it look right". Every screenshot deserves a critique pass.**

Peekaboo is a single Swift CLI that combines screenshot, AX-tree inspection,
synthetic input, and change-aware video capture against macOS apps. It's
the macOS-native equivalent of Playwright + visual-regression review.

You invoke `peekaboo` directly — it's a full toolkit, not a fixed menu of
recipes. Build orchestration belongs to your project's task runner
(Justfile, Makefile, `xcodebuild`, etc.); UI driving is your domain.

**No external AI keys. Ever.** You are the LLM. When you need to "look at"
the screenshot, `view` the PNG yourself. Skip `peekaboo agent`,
`peekaboo see --analyze`, `peekaboo image --analyze`, `peekaboo analyze`
— they all require an external API key.

## When to invoke this skill

- ✅ User says "see what it looks like", "show me the UI", "verify it works",
  "click X", "type Y", "take a screenshot", "drive the app", "test it
  end-to-end"
- ✅ After modifying any SwiftUI/AppKit view — verify it renders, works, and
  *still looks intentional*
- ✅ When debugging a UI bug — capture before/after + critique each
- ✅ Before claiming a UI feature done in autopilot mode
- ❌ Skip if the change is purely backend / non-UI

## Prerequisites

```bash
peekaboo --version                           # ≥ 3.0
peekaboo list permissions --json             # both must be granted
```

If peekaboo is not installed:
```bash
brew install steipete/tap/peekaboo
```

Then grant **Screen Recording** AND **Accessibility** to your terminal in
System Settings → Privacy & Security. Both are required.

## The loop (copy this checklist into your response and tick items off)

- [ ] **Build** the app — use the project's task runner
- [ ] **Drive** — get the app to the state under test
- [ ] **Snapshot** — `peekaboo see --json --annotate --path /tmp/k.png`
- [ ] **CRITIQUE** — `view /tmp/k_annotated.png` inline; score it against the
      rubric. Optionally chain a design skill (`critique`/`polish`/`layout`)
- [ ] **Fix** any findings
- [ ] **Re-verify** — repeat snapshot + critique; diff against previous PNG

---

## Discovering the target app

Before running anything, you need the **bundle ID** (preferred) or app name:

```bash
# Find a running app's bundle ID
peekaboo list apps --json | jq '.data.applications[]|select(.name|test("YourApp";"i"))|{name,bundleIdentifier,processIdentifier}'

# Always prefer --bundle-id over --app <DisplayName>:
#   - bundle ID is stable (e.g. com.example.myapp)
#   - the running process registers as CFBundleName, which often differs
#     from the Xcode scheme. Using the scheme name fails APP_NOT_FOUND.
```

Cache the bundle ID and any display-name caveats in your project's
`AGENTS.md` / `CLAUDE.md` so future runs don't re-discover them.

---

## Terminology (one-to-one with peekaboo's command names)

| Word           | Means                                                    | Command                  |
|----------------|----------------------------------------------------------|--------------------------|
| **Snapshot**   | Annotated PNG **plus** AX-tree JSON (the "DOM+screenshot") | `peekaboo see`         |
| **Screenshot** | Raw PNG only, no AX tree                                 | `peekaboo image`         |
| **Recording**  | Change-aware MP4 video                                   | `peekaboo capture live`  |

Default to **snapshot** (`peekaboo see`) — the AX JSON lets the next step
click by element ID without re-parsing the screen.

---

## Step 1 — Drive the UI

The full driving surface (run `peekaboo <cmd> --help` for details, or
`peekaboo learn` for the comprehensive guide):

| Need                                  | Command                                                  |
|---------------------------------------|----------------------------------------------------------|
| Launch / quit app                     | `peekaboo app launch --bundle-id $BID --wait-until-ready`<br>`peekaboo app quit --app $BID` |
| Inspect UI (annotated PNG + JSON)     | `peekaboo see --app $BID --json --annotate --path /tmp/k.png > /tmp/k.json` |
| Click by AX identifier (most robust)  | See "Click by `.accessibilityIdentifier`" recipe below   |
| Click by visible label                | `peekaboo click "Settings" --app $BID`                   |
| Click by element ID from `see`        | `peekaboo click --on elem_42 --snapshot $SID --app $BID` |
| Type text                             | `peekaboo type "hello" --app $BID`<br>(append `--return` to press Enter) |
| Single key                            | `peekaboo press return` / `peekaboo press escape`        |
| Hotkey                                | `peekaboo hotkey --keys "cmd,b" --app $BID`              |
| **Menu bar**                          | `peekaboo menu list --app $BID` (discover)<br>`peekaboo menu click --app $BID --path "View > Toggle Sidebar"` |
| **System file/save dialog**           | `peekaboo dialog list --app $BID`<br>`peekaboo dialog click --button "Open" --app $BID`<br>`peekaboo dialog input --text "..." --field "..." --app $BID`<br>`peekaboo dialog file --path "/Users/me" --select "Open" --app $BID` |
| **Scroll inside a list / pane**       | `peekaboo scroll --direction down --amount 5 --on elem_N` |
| **Drag-and-drop**                     | `peekaboo drag --from elem_A --to elem_B --app $BID`     |
| **Resize/reposition window**          | `peekaboo window set-bounds --app $BID --x 0 --y 0 --width 1024 --height 640` |
| **Read/write clipboard**              | `peekaboo clipboard get` / `peekaboo clipboard set --text "x"` |
| **Paste from clipboard**              | `peekaboo paste --app $BID`                              |
| **Open URL / deep link**              | `peekaboo open "x-myapp://..."`                          |
| **Replay a saved scenario**           | `peekaboo run scripts/onboarding.peekaboo.json`          |

Throughout this skill, `$BID` is the target app's bundle ID
(`com.example.myapp`).

### Click by `.accessibilityIdentifier` — the killer pattern

If your SwiftUI / AppKit code tags interactive views with
`.accessibilityIdentifier(...)`, Peekaboo surfaces those exact strings as
`ui_elements[].identifier`. Queries are then immune to copy changes:

```bash
peekaboo see --app $BID --json --annotate --path /tmp/k.png > /tmp/k.json

SID=$(jq -r .data.snapshot_id /tmp/k.json)
ID=$(jq -r '.data.ui_elements[]|select(.identifier=="header.utility.settings").id' /tmp/k.json)

peekaboo click --on "$ID" --snapshot "$SID" --app $BID
```

If you reach for a copy-based click (`peekaboo click "Some Label"`) more
than once on the same control, **add a stable identifier** in your project's
chosen namespace (e.g. `header.tab.sessions`, `panel.settings.save`,
`row.<id>.delete`). Both peekaboo queries and any XCUITest become resilient
to copy changes:

```swift
Button("Settings") { … }
    .accessibilityIdentifier("header.utility.settings")
```

Document the namespace in your project's `AGENTS.md` so additions stay
consistent.

---

## Step 2 — Capture (anatomy of `peekaboo see`)

```bash
peekaboo see --app $BID --json --annotate --path /tmp/k.png > /tmp/k.json
```

This writes **two PNGs**:
- `/tmp/k.png` — raw screenshot
- `/tmp/k_annotated.png` — same image overlaid with `elem_N` markers
  (this is the one you usually want to `view`)

…and prints a JSON envelope `{success, data, debug_logs}` where `data` has:

| Field                  | Meaning                                                |
|------------------------|--------------------------------------------------------|
| `snapshot_id`          | UUID — pass to `--snapshot` for click stability        |
| `screenshot_raw`       | path to `/tmp/k.png`                                   |
| `screenshot_annotated` | path to `/tmp/k_annotated.png`                         |
| `application_name`     | display name (CFBundleName)                            |
| `window_title`         | currently focused window                               |
| `capture_mode`         | e.g. `window`                                          |
| `is_dialog`            | true if the captured surface is a modal dialog         |
| `element_count`        | total AX elements                                      |
| `interactable_count`   | how many are `is_actionable`                           |
| `ui_elements`          | array of elements (see fields below)                   |
| `ui_map`               | absolute path to a richer cache `snapshot.json`        |

Each `ui_elements` entry exposes:

```json
{
  "id": "elem_42",
  "role": "button",
  "role_description": "button",
  "title": "Settings",
  "label": "Settings",
  "description": "Open settings panel",
  "help": "",
  "identifier": "header.utility.settings",
  "is_actionable": true,
  "keyboard_shortcut": ""
}
```

`identifier` is the SwiftUI / AppKit `.accessibilityIdentifier(...)`.
Bounding boxes live on disk (`ui_map` path) under
`uiMap.<id>.frame = [[x,y],[w,h]]` if you need pixel-precise targeting.

Other capture commands:

```bash
# Just a PNG, retina, window-only — no AX overhead
peekaboo image --mode window --app $BID --retina --path /tmp/shot.png

# Animation/transition: change-aware MP4, 5s
peekaboo capture live --app $BID --duration 5 \
  --video-out /tmp/anim.mp4 --threshold 1.0 --highlight-changes
```

The snapshot cache lives at `~/.peekaboo/snapshots/<UUID>/`
(`raw.png`, `annotated.png`, `snapshot.json`). Default keeps 24h;
`peekaboo clean --older-than 24` purges. Each `see` call creates a new UUID.

---

## Step 3 — **CRITIQUE** (mandatory, not optional)

Click-verification proves a button is hooked up. It does not prove the UI
is good. After every meaningful snapshot, critique it.

**You are the LLM — read the pixels yourself.**

```
1. peekaboo see --app $BID --json --annotate --path /tmp/k.png > /tmp/k.json
2. view /tmp/k_annotated.png              # read it into your context
3. (optional) chain a design skill — see catalog below
4. Write the critique using the rubric template at the end of this section
```

### Design skills available for deeper passes

When the snapshot exposes a specific weakness, chain to the matching skill
instead of writing the whole critique from scratch:

| Concern surfaced by the snapshot              | Skill to chain |
|-----------------------------------------------|----------------|
| Visual quality, hierarchy, anti-pattern smell | `critique`     |
| Accessibility, theming, perf, anti-patterns   | `audit`        |
| Final pre-ship pass: alignment, spacing nits  | `polish`       |
| Layout/grid/spacing rhythm specifically       | `layout`       |
| Typography hierarchy, sizing, readability     | `typeset`      |
| Color, palette, contrast                      | `colorize`     |
| Microcopy clarity                             | `clarify`      |
| The SwiftUI *code* producing the issue        | `swift-critique` / `swiftui-expert` |

Apply the recommended fixes, then loop back to Step 1.

### What a critique pass answers

- **Did the change achieve its intent?** (e.g. user asked for "more breathing
  room" — does the screenshot actually have more breathing room?)
- **Visual hierarchy**: where does the eye land first? Is that the primary
  action?
- **Alignment & spacing**: are elements on a consistent grid? Inconsistent
  gaps between siblings?
- **Contrast & legibility**: WCAG AA on critical text? Disabled states still
  readable?
- **Copy clarity**: any jargon, truncation, ambiguous labels?
- **Anti-patterns**: AI-slop tells, generic gradient buttons, hero-stat
  layouts that don't fit the data?
- **Edge cases captured?** Empty state, loading, error, overflow.

If "did the change achieve intent" is "I don't know without seeing it",
snapshot more states (hover, focus, error, empty) and re-critique.

### Example: what a finished critique reads like

A good critique appended to your response is concrete, scored, and ranked.
Match this shape:

> **Critique of `/tmp/k_annotated.png` (Settings panel, 1440×900)**
>
> | Dimension          | Score | Note                                            |
> |--------------------|------:|-------------------------------------------------|
> | Visual hierarchy   |   4/5 | Primary action ("Save") clearly leads.          |
> | Alignment          |   3/5 | Section labels off the 8px grid by 2–4px.       |
> | Spacing rhythm     |   4/5 | Consistent except around the toggle row.        |
> | Contrast           |   5/5 | All copy ≥ AA on the dark surface.              |
> | Copy clarity       |   3/5 | "Auto-stage" reads as a verb but sits as label. |
> | AI-slop tells      |   ✓   | None — no gradient text, glassmorphism, etc.    |
>
> **Top 3 fixes ranked by impact**
> 1. Snap section labels to the 8px grid (alignment 3 → 5).
> 2. Rewrite "Auto-stage" → "Stage new sessions automatically" (copy 3 → 5).
> 3. Tighten the toggle/description gap from 14px to 8px so the toggle
>    visually owns its description.
>
> **Did the change achieve intent?** Yes — the new spacing reads calmer than
> the before/after diff in `/tmp/before.png`.

This shape is the contract. Anything less detailed is undercooked.

---

## Step 4 — Fix and re-verify

After fixing, re-run Steps 1–3 and diff:

```bash
open /tmp/before.png /tmp/after.png            # eyeball
compare /tmp/before.png /tmp/after.png /tmp/diff.png   # ImageMagick
```

---

## Responsive verification (test at multiple sizes)

Don't assume the layout works at 1440×900 just because it works at your
window size:

```bash
for size in "1024 640" "1280 800" "1440 900" "1920 1200"; do
  W=${size%% *}; H=${size##* }
  peekaboo window set-bounds --app $BID --x 0 --y 0 --width $W --height $H
  sleep 0.3
  peekaboo see --app $BID --json --annotate \
    --path /tmp/k-${W}x${H}.png > /tmp/k-${W}x${H}.json
done
# Then `view` each annotated PNG and critique inline.
```

---

## Common scenarios

### Verify a tab switch + critique the resulting view

```bash
peekaboo click "Friends" --app $BID
sleep 0.5
peekaboo see --app $BID --json --annotate \
  --path /tmp/friends.png > /tmp/friends.json
# `view /tmp/friends_annotated.png` and critique the empty state inline.
```

### Drive a system file picker

```bash
# Trigger the action that opens NSOpenPanel/NSSavePanel, then:
peekaboo dialog file --path "$HOME/Documents" --select "Open" --app $BID
sleep 0.5
peekaboo see --app $BID --json --annotate --path /tmp/post-pick.png > /tmp/post-pick.json
```

### Verify a "Copy …" button actually copies

```bash
peekaboo clipboard clear
peekaboo click "Copy session ID" --app $BID
sleep 0.2
peekaboo clipboard get   # should print the expected ID
```

### Capture a transition for review

```bash
peekaboo capture live --app $BID --duration 4 \
  --video-out /tmp/transition.mp4 --threshold 1.0 --highlight-changes &
sleep 0.3
peekaboo hotkey --keys "cmd,b" --app $BID  # trigger the transition
wait
# Open the MP4, look for jank, compare frame timing.
```

---

## Pitfalls

- **Bundle ID vs display name vs Xcode scheme**: the running process
  registers as `CFBundleName`, which often *differs* from the Xcode scheme.
  Always use `--bundle-id` or the actual `CFBundleName`. Cache this in
  `AGENTS.md`.
- **Composite views broadcast their identifier**: applying
  `.accessibilityIdentifier("panel.settings")` to a view whose body contains
  many controls makes several AX elements share that identifier. Fine for
  presence checks; for child-specific clicks, give the child its own
  identifier or use `.accessibilityElement(children: .contain)`.
- **Icon-only buttons** (`Image(systemName: …)` inside a `Button`) often
  expose no AX label by default. Add `.accessibilityLabel("…")` so peekaboo
  / XCUITest can target them.
- **Animation settle time**: SwiftUI `withAnimation(...)` durations need a
  matching `sleep` after triggering — usually 0.3–0.5s — before the next
  click, or use `peekaboo click --wait-for 1500`.
- **Custom-rendered panes** (e.g. terminals, canvases) render into views
  whose contents are opaque to AX. For text verification, take a
  `peekaboo image` and `view` the PNG — read the rendered text directly.
- **Snapshot UUIDs accumulate**: each `see` creates a new
  `~/.peekaboo/snapshots/<UUID>/` (≈ 100–500 KB). Run
  `peekaboo clean --older-than 24` periodically.

---

## When to use XCUITest instead

Peekaboo is for **interactive iteration** ("did my change work, and does it
look right?"). XCUITest is for **deterministic regression gates** ("does
this still work?"). Both coexist; identifiers added for peekaboo make
XCUITest queries simpler too.

---

## Discovery / health checks

```bash
peekaboo --version                                                     # ≥ 3.0
peekaboo list permissions --json | jq '.permissions[]|{name,isGranted}'
peekaboo list apps --json | jq '.data.applications[]|{name,bundleIdentifier}|select(.name|test("YourApp";"i"))'
peekaboo menu list --app $BID                                          # menu bar
peekaboo learn                                                         # full reference
peekaboo <cmd> --help                                                  # any subcommand
```

If a project keeps wanting the same recipe, *don't bake it into the task
runner* — recipes constrain the agent. Document the pattern in the project's
`AGENTS.md` instead and let the agent invoke `peekaboo` directly.
