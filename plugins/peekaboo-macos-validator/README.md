# peekaboo-macos-validator

macOS app UI automation and visual critique via [Peekaboo](https://peekaboo.boo).
The macOS-native equivalent of Playwright + visual-regression review for
any SwiftUI / AppKit app — drive the UI, capture snapshots, and **read
the pixels inline** to write a real critique.

> **No external AI keys.** The agent (Claude Code / Copilot CLI) is the
> LLM. `peekaboo see` writes a PNG; the agent `Read`s it back and critiques
> it directly. No `peekaboo agent`, no `--analyze`, no API providers.

## What It Does

- **Snapshot** the AX tree + an annotated PNG in one call (`peekaboo see`)
- **Click** by `.accessibilityIdentifier`, by visible label, by element ID,
  or by raw coordinates
- **Type, hotkey, scroll, drag, paste** — full input surface
- **Drive menu bar items**, **system file/save dialogs**, and **window
  geometry** for responsive testing
- **Capture change-aware MP4 recordings** of animations and transitions
- **CRITIQUE every snapshot** — every UI change gets a scored visual
  critique pass before being claimed done

## Installation

```bash
# 1. Install peekaboo (one-time, Homebrew)
brew install steipete/tap/peekaboo

# 2. Grant permissions: System Settings → Privacy & Security
#    - Screen Recording        → your terminal / agent host
#    - Accessibility           → your terminal / agent host
#    Both are required.

# 3. Install this plugin
claude --plugin-dir /path/to/myclaudeskills/plugins/peekaboo-macos-validator
# or via the marketplace:
claude plugin install peekaboo-macos-validator@jko-claude-plugins

# 4. Verify
/peekaboo-macos-validator:peekaboo-doctor
```

A `SessionStart` hook prints a one-line install hint to stderr if the
binary is missing — silent on success.

## Commands

| Command                                      | Purpose                                                             |
|-----------------------------------------------|---------------------------------------------------------------------|
| `/peekaboo-macos-validator:peekaboo-doctor`   | Verify install, version, permissions; explicit fix-its              |
| `/peekaboo-macos-validator:validate-macos-app`| Launch app, walk each view, snapshot + critique each, report PASS/FAIL |

## Skill

**peekaboo** — teaches the agent the full workflow for any macOS app:
discover bundle ID → launch → snapshot → click by `.accessibilityIdentifier`
→ critique pixels inline → fix → re-verify. Activates when the user mentions
UI, screens, screenshots, layout, spacing, alignment, sidebar, panels,
clicking, typing, menus, file pickers, animations, or says "verify it
works", "see what it looks like", "show me the UI", "click X", "take a
screenshot", "critique the design".

The skill enforces a **mandatory critique pass** with a scored rubric
(visual hierarchy, alignment, spacing, contrast, copy clarity, AI-slop
tells) — it explicitly does not stop at "the button is clickable".

## Workflow at a glance

```bash
# 1. Build (your project's task runner)
just build                # or: xcodebuild build, swift build, …

# 2. Launch + snapshot
peekaboo app launch --bundle-id com.example.myapp --wait-until-ready
peekaboo see --app com.example.myapp --json --annotate \
  --path /tmp/k.png > /tmp/k.json

# 3. Read the PNG inline (the agent does this — that's the killer step)
#    `Read /tmp/k_annotated.png`  in the agent's tool surface

# 4. Click by accessibility identifier
SID=$(jq -r .data.snapshot_id /tmp/k.json)
ID=$(jq -r '.data.ui_elements[]|select(.identifier=="header.tab.settings").id' /tmp/k.json)
peekaboo click --on "$ID" --snapshot "$SID" --app com.example.myapp

# 5. Critique with the rubric, fix, re-verify
```

## Setting up your app for peekaboo

The skill works best when interactive views are tagged with stable
`.accessibilityIdentifier(...)` values:

```swift
Button("Settings") { showSettings = true }
    .accessibilityIdentifier("header.utility.settings")
```

Document the namespace conventions for your project in its `AGENTS.md`
or `CLAUDE.md` so additions stay consistent. Identifiers added for
peekaboo also make XCUITest queries simpler.

## Prerequisites

- macOS 13+ (Peekaboo 3.x)
- `peekaboo` ≥ 3.0 (`brew install steipete/tap/peekaboo`)
- Both **Screen Recording** and **Accessibility** permissions granted
- A buildable macOS app target

## Hook

| Event          | What it does                                                    |
|----------------|------------------------------------------------------------------|
| `SessionStart` | Silent if `peekaboo` is on PATH. Prints one-line install hint to stderr otherwise. Exits 0 either way (never blocks the session). |

## Pitfalls (lifted from the skill — full list inside)

- **Bundle ID vs display name**: the running process registers as
  `CFBundleName`, which often differs from the Xcode scheme. Always prefer
  `--bundle-id`. Cache the running display name in your `AGENTS.md`.
- **Composite views**: applying `.accessibilityIdentifier(...)` to a
  parent view broadcasts that identifier to multiple AX descendants. Tag
  the precise child instead, or use `.accessibilityElement(children: .contain)`.
- **Icon-only buttons**: `Image(systemName:)` inside a `Button` exposes no
  AX label. Always add `.accessibilityLabel("…")`.
- **Animation settle time**: SwiftUI `withAnimation(...)` durations need a
  matching `sleep 0.3–0.5` after triggering, or `peekaboo click --wait-for 1500`.
- **Snapshot cache growth**: `peekaboo clean --older-than 24` weekly.
- **Never call `--analyze`, `peekaboo agent`, `peekaboo analyze`** —
  they require external API keys and the agent is already an LLM.

## License

MIT — see [LICENSE](./LICENSE).
