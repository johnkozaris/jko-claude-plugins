---
name: peekaboo
description: >-
  Provides runtime observation and interaction for native macOS interfaces
  through accessibility state and screenshots. Use when the task depends on
  visible or interactive state in a running SwiftUI/AppKit app: what is
  rendered, focused, selected, enabled, reachable through menus/windows/dialogs,
  or experienced across a UI workflow. Tasks about source, configuration, data,
  logs, processes, or app APIs do not need this skill unless their result must
  be verified in the interface. Not for Electron/web apps or iOS simulators.
homepage: https://peekaboo.sh
metadata:
  os: ["darwin"]
  requires:
    bins: ["peekaboo", "jq"]
    permissions: ["Screen Recording", "Accessibility"]
  install:
    - kind: brew
      formula: steipete/tap/peekaboo
---

# Peekaboo

Use Peekaboo when the unresolved evidence or action lives in the running native
interface: whether a workflow works, what state it reached, and whether that
state looks intentional.

## Preserve the driver's context

Text is cheaper than pixels, but an accessibility tree is not free. Persist
structured JSON and query only the fields needed for the next decision rather
than loading whole trees into the conversation. Some hosts retain and resend
every image loaded into a conversation, so a long capture-inspect loop can
consume the context window even when each screenshot is small.

Prefer structured inspection until pixels are necessary. When visual judgment
is required, save artifacts to disk and inspect them through a bounded,
disposable visual context when the host provides one. Return only the text
finding to the driving conversation. A visual task may compare a small related
set such as before/after states; the invariant is bounded isolation, not one
particular sub-agent choreography. If isolation is unavailable, inspect only
the smallest necessary image or crop and avoid accumulating further images in
the same long-running context.

Read `references/agent-runtime.md` for host-specific options and
`references/visual-verification.md` for the visual-reader contract.

## Start from live capabilities

Check the installed CLI and selected runtime before guessing flags:

```bash
peekaboo --version
peekaboo permissions status --all-sources --json
peekaboo <command> --help
```

Screen Recording belongs to the process performing capture; Accessibility
belongs to the process inspecting or acting on UI. Event Synthesizing is needed
for background keyboard input and coordinate/synthetic fallback. In agent,
SSH, or launchd sessions the selected process may be the daemon or Peekaboo.app
Bridge rather than the terminal.

## Observe, act, verify

1. Identify the app by bundle ID when possible.
2. Observe with the cheapest source that answers the question.
3. Resolve the target from fresh structured state.
4. Perform one meaningful action.
5. Observe again and assert a product postcondition.
6. Inspect pixels only when layout, rendering, or visual quality matters.
7. Fix and repeat until both behavior and appearance satisfy the request.

Use `inspect-ui --json` for accessibility state without pixels. Use
`see --json --path ...` when a screenshot or auditable snapshot is required.
Use a raw window image only for custom rendering that accessibility cannot
represent.

Treat snapshot and element IDs as opaque. Copy them from the latest result and
do not infer or generate them. Any state-changing action can invalidate the
previous snapshot, so observe again before another ID-based action.

## Choose semantic actions

Prefer direct accessibility behavior over synthesized input:

1. `set-value` for a settable non-secure field.
2. ID-based click for normal activation.
3. `perform-action` for a supported accessibility action.
4. A label or query when no stable identifier exists.
5. Coordinates only for custom-drawn or accessibility-invisible content.

Use native commands for app lifecycle, menus, file dialogs, windows, clipboard,
and deep links rather than rebuilding those flows with coordinate clicks.
Preserve background delivery unless the app explicitly requires focus or real
pointer input.

## Recover from evidence

- On a stale snapshot or missing element, observe again and resolve the target.
- On a truncated accessibility tree, increase bounds only enough to reach the
  needed surface.
- On blank, wallpaper-only, or redacted capture, inspect the selected runtime,
  daemon, Bridge, and permission state before changing capture engines.
- When accessibility cannot expose custom rendering, normalize the target
  window and make one verified coordinate fallback.
- Treat a successful command with an unchanged postcondition as failure, not as
  proof the feature worked.

## Validate the product, not the command

A click succeeding does not prove the resulting state is correct. Check the
fresh accessibility tree or application output for the expected behavior, then
use visual evidence for hierarchy, alignment, spacing, contrast, copy,
overflow, focus, empty/loading/error states, and consistency with the product.

Use `capture action` for transitions or intermittent failures so action result
and visual evidence share one bounded capture. Keep artifact names tied to the
state under test and record paths for failures.

Instrument repeated targets with stable accessibility identifiers. Prefer a
small documented namespace and tag the precise actionable child rather than a
composite parent.

## Safety

Keep credentials and private content out of shell arguments, logs,
screenshots, and recordings. Let the user mediate secure fields. Require
confirmation before payments, deletion, publishing, sending, account changes,
or other irreversible actions.

Use Peekaboo for interactive product validation and XCUITest for durable
regression gates. Do not replace deterministic tests with an agent-only flow.
