---
name: electron-playwright-validator
description: >-
  Validate Electron desktop app UI using Playwright automation via the e-cli tool.
  Use when testing Electron app UI, validating after code changes, checking if tabs render,
  clicking through UI flows, taking screenshots for visual review, or debugging runtime
  import errors that typecheck doesn't catch. Works with any Electron project that has
  @playwright/test and electron in devDependencies. Use when the user asks "validate the
  electron app", "check the UI", "click through the app", "take a screenshot", "does the
  app render", "test the electron UI", "launch the app and check", "verify the app works",
  "is the app broken", or "debug a runtime error".
---

# Electron Playwright Validator

Validate Electron desktop apps by launching them with Chrome DevTools Protocol (CDP) and automating the UI via Playwright. The `e-cli` tool provides a persistent session model: launch the app once (~5s startup), then run fast CDP-connected commands (~200ms each) to inspect, click, screenshot, and validate.

## Core Workflow

Follow this sequence for every validation session:

1. **Launch** — `e-cli launch` starts the Electron app with CDP enabled. Auto-builds if `out/main/index.js` is missing.
2. **Snapshot** — `e-cli snapshot` dumps the full accessibility tree. This is how you "see" the UI.
3. **Interact** — Use `e-cli click`, `e-cli fill`, `e-cli press` to navigate and interact.
4. **Validate** — After each interaction, snapshot again to verify the UI updated correctly. Use `e-cli screenshot` + `Read` tool for visual confirmation.
5. **Close** — `e-cli close` terminates the app and cleans up state.

Always close the app when done. A lingering Electron process blocks subsequent launches.

## Reading the Accessibility Tree

The `e-cli snapshot` output is your primary tool for understanding the UI. Each line represents a node:

```
WebArea "Kodosi"
  navigation "Main"
    tab "Sessions" pressed=true
    tab "Rooms"
    tab "Friends"
    tab "Public"
  main
    heading "Sessions" level=1
    list "Session list"
      listitem "dev-server"
      listitem "build-watch"
```

Use this tree to:
- **Find elements** — scan for the role and name of what you want to interact with
- **Build selectors** — translate tree nodes into Playwright selectors
- **Verify state** — check `pressed`, `expanded`, `checked`, `disabled` attributes
- **Confirm navigation** — after clicking a tab, snapshot to see the new view content

## Selector Strategy

Prefer selectors in this order (most robust to least):

1. **Role selectors** — `role=tab[name="Sessions"]`, `role=button[name="New"]`
2. **Text selectors** — `text=Sessions`, `text=Create new session`
3. **Test ID selectors** — `[data-testid=session-list]`
4. **CSS selectors** — `.sidebar-nav button`, `#main-content h1`

Role selectors are best because they match the accessibility tree directly and survive CSS refactors. CSS selectors are fragile — use only as a last resort.

> *Consult [accessibility selectors reference](references/accessibility-selectors.md) for full selector syntax and patterns.*

## Multimodal Validation

Combine accessibility snapshots with screenshots for thorough validation:

```bash
e-cli snapshot              # structural check — are the right elements present?
e-cli screenshot            # visual check — does it look right?
```

After `e-cli screenshot`, use the `Read` tool on the PNG file to visually inspect the result. This catches issues that the accessibility tree cannot: broken layouts, missing styles, overlapping elements, blank screens.

## Timing and Async Content

Electron apps load content asynchronously. Handle this with:

- **`e-cli wait <selector>`** — wait for an element to become visible before interacting
- **Sequential snapshot-then-act** — always snapshot before clicking to confirm the target exists
- **Post-interaction delay** — if a click triggers an async operation (API call, animation), wait for the result element before snapshotting

```bash
e-cli wait "role=tab[name=Sessions]"      # wait for nav to render
e-cli click "role=tab[name=Rooms]"        # switch tab
e-cli wait "role=heading[name=Rooms]"     # wait for view to load
e-cli snapshot                             # verify the new view
```

> *Consult [electron gotchas reference](references/electron-gotchas.md) for timing issues, xterm quirks, and lazy loading.*

## When to Use e-cli vs pnpm test:e2e

| Scenario | Tool |
|----------|------|
| Exploring the UI after code changes | `e-cli` |
| Debugging a runtime import error | `e-cli` |
| Validating a specific view renders | `e-cli` |
| Interactive click-through of a flow | `e-cli` |
| Deterministic regression suite for CI | `pnpm test:e2e` |
| Testing IPC handler responses | `pnpm test:e2e` |

Use `e-cli` for **exploratory validation** — quick checks, visual inspection, debugging. Use `pnpm test:e2e` for **deterministic tests** that run in CI.

## Common Patterns

### Validate all tabs render

```bash
e-cli launch
e-cli snapshot                                    # check initial state
for tab in Sessions Rooms Friends Public; do
  e-cli click "role=tab[name=$tab]"
  e-cli snapshot                                  # verify view content
done
e-cli close
```

### Check if an error screen shows

```bash
e-cli launch
e-cli snapshot                                    # look for error roles/text
e-cli screenshot                                  # visual confirmation
e-cli close
```

### Evaluate renderer state

```bash
e-cli eval "Object.keys(window.api)"              # check preload bridge
e-cli eval "document.title"                       # check page title
e-cli eval "document.querySelectorAll('.error').length"  # count error elements
```

### Debug broken imports

If `pnpm typecheck` passes but the app shows a blank screen:

```bash
e-cli launch
e-cli eval "document.querySelector('#root')?.innerHTML?.substring(0, 200)"
e-cli screenshot
# Check the terminal output / DevTools console for import errors
e-cli close
```

## Validation Mindset

Running e-cli commands is not enough — you must think critically about what you see.

### UI Appearance
After every screenshot, evaluate it with a designer's eye. Do layouts, spacing, colors, and typography look correct? Does this screen feel consistent with the rest of the app? Would a real user notice something off — misalignment, clipped text, wrong font weight, broken dark-mode contrast? If something looks wrong, it is wrong.

### User Experience
Don't just verify elements exist. Click through the flow as a user would. Does the interaction sequence make sense? Are transitions smooth? Does the app respond where you expect it to? If a button exists but the flow around it is confusing, that's a bug.

### Test Depth
Happy-path validation is insufficient. Consider what a real user would encounter:
- Empty states — what happens with no data?
- Error states — what if something fails?
- Edge cases — rapid clicks, resize, long text, many items
- Navigation — can you get back? Does browser-back work?

If your validation only covers the obvious path, go deeper.

### Coherence
A feature can be technically correct but feel wrong in context. After validating, ask: does this change fit the existing app? Does it match the design language, information density, and interaction patterns of surrounding views? Flag anything that looks out of place, even if it "works."

## DO

- Always `e-cli snapshot` before interacting — confirms the element exists
- Always `e-cli close` when done — prevents port conflicts and zombie processes
- Use role selectors when possible — they match the accessibility tree directly
- Combine snapshot + screenshot for thorough validation
- Check `e-cli status` if launch says "Already running" unexpectedly

## DON'T

- Don't skip the snapshot step — clicking blind leads to flaky results
- Don't forget to close — a lingering process blocks the next launch
- Don't use CSS selectors for elements that have accessible names — prefer role selectors
- Don't assume instant rendering — use `e-cli wait` for async content
- Don't run `e-cli` in parallel — one session at a time per port

> *Consult [e-cli command reference](references/e-cli-reference.md) for full command documentation and examples.*
