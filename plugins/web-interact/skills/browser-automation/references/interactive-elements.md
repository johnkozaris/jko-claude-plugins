# Interactive Elements API

Zero-timeout element discovery and interaction for browser automation.
Adapted from [browser-use](https://github.com/browser-use/browser-use),
[Stagehand](https://github.com/browserbase/stagehand), and
[Playwright MCP](https://github.com/microsoft/playwright-mcp).

## The Problem

Playwright's `click()`, `fill()`, and similar actions use a **retry loop** that polls
the DOM repeatedly. If the selector is wrong, the action hangs for its full timeout
(up to 30 seconds) before failing. This wastes time and produces unhelpful errors.

## The Solution: Discover-Then-Act

Instead of guessing selectors, take an inventory of what exists first:

```javascript
// Step 1: Wait for page to settle
await browser.waitForSettled("main", { quietMs: 300 });

// Step 2: Get all interactive elements (instant — no timeouts)
const inv = await browser.getInteractiveElements("main", {});
console.log(inv.serialized);
// [1] button "Submit"
// [2] input[email] "Email" placeholder="user@example.com"
// [3] link "Sign in" href="/login"

// Step 3: Interact using selectors from the inventory
const btn = inv.elements.find(e => e.name === "Submit");
if (btn) await browser.clickElement("main", btn.selector);
```

Every step is instant. No Playwright retry loops. No timeouts.

## API Reference

### browser.getInteractiveElements(nameOrId, options?)

Returns an indexed inventory of all interactive, visible elements on the page.

**Options:**
- `viewportOnly: boolean` (default: false) — Only elements within/near the viewport
- `maxElements: number` (default: 200) — Cap on returned elements
- `maxTextLength: number` (default: 100) — Truncate text at this length
- `checkOcclusion: boolean` (default: false) — Filter elements hidden behind others
- `detectListeners: boolean` (default: true) — Use CDP to detect JS event listeners

**Returns:**
```javascript
{
  count: 12,
  serialized: "[1] button \"Submit\"\n[2] input[email] ...",
  elements: [
    {
      index: 1,
      tag: "button",
      role: "button",
      name: "Submit",
      selector: "button#submit-btn",    // CSS selector
      xpath: "/html/body/form/button",  // Absolute XPath
      attrs: { type: "submit", id: "submit-btn" }
    },
    // ...
  ],
  scrollableAreas: [
    { selector: "#content", hiddenPixels: 2400, hiddenPages: 3.2 }
  ],
  viewport: { width: 1280, height: 720, scrollX: 0, scrollY: 0 }
}
```

**Serialized format for LLM consumption:**
```
[1] button "Submit"
[2] input[email] "Email" placeholder="user@example.com"
[3] link "Sign in" href="/login"
[4] select "Country" (190 options: US, UK, DE, FR, JP, ... 185 more)
[5] input[date] "Birthday" (format=YYYY-MM-DD)
[6] checkbox "Remember me" [checked]
[7] input[range] "Volume" (range 0-100 current=50)
*[8] button "New Button"   ← appeared since last call (marked with *)

--- Scrollable areas ---
  #content (~3.2 pages hidden below)
```

### browser.waitForSettled(nameOrId, options?)

Waits for the DOM to stop changing using a MutationObserver.

**Options:**
- `quietMs: number` (default: 500) — Milliseconds of no mutations to consider settled
- `timeout: number` (default: 5000) — Maximum wait time

**Returns:** `{ settled: boolean, elapsed: number, reason?: string }`

### browser.clickElement(nameOrId, selector, action?)

Native DOM click — bypasses Playwright's actionability checks entirely.

- `selector: string` — CSS selector from `getInteractiveElements`
- `action: string` (default: "click") — `"click"`, `"focus"`, or `"scrollIntoView"`

**Returns:** `{ success: boolean, tag?: string, text?: string, error?: string }`

### browser.fillElement(nameOrId, selector, value)

Native value setter — works with React/Vue controlled components.

If the selector targets a `<label>`, automatically redirects to its associated input.
Uses the native `HTMLInputElement.prototype.value` setter to bypass framework wrappers,
then dispatches `input` and `change` events.

- `selector: string` — CSS selector from `getInteractiveElements`
- `value: string` — Value to fill

**Returns:** `{ success: boolean, tag?: string, value?: string, error?: string }`

## Detection Heuristics (12 checks)

In priority order:

1. **CDP JS event listeners** — React onClick, Vue @click, Angular (click) via
   `Runtime.evaluate` with `includeCommandLineAPI`
2. **Native interactive HTML tags** — `a[href]`, `button`, `input`, `select`, `textarea`,
   `details`, `summary`
3. **ARIA widget roles** — button, link, checkbox, textbox, combobox, slider, tab,
   menuitem, switch, searchbox, treeitem, etc.
4. **`contenteditable`** — editable elements
5. **Inline event handlers** — onclick, onmousedown, ontouchstart, onpointerdown
6. **`tabindex`** — keyboard-focusable elements (positive values)
7. **ARIA state properties** — aria-expanded, aria-checked, aria-pressed, aria-selected
   (their presence implies an interactive widget)
8. **Label wrappers** — `<label>` and `<span>` wrapping form controls (Ant Design, Bootstrap)
9. **Sized iframes** — iframe/frame with dimensions > 100x100
10. **CSS cursor: pointer** — filtered to avoid noise from large container divs
11. **Shadow DOM** — traverses open shadow roots via TreeWalker
12. **Keyboard shortcuts** — surfaces `aria-keyshortcuts` and `accesskey` attributes

## Visibility Pipeline

Elements must pass ALL checks:

1. `Element.checkVisibility()` (modern API, Chrome 105+)
2. `getComputedStyle()`: display, visibility, opacity
3. `getBoundingClientRect()`: width > 0, height > 0
4. Off-screen honeypot filter (left/top < -100)
5. Viewport bounds (when `viewportOnly` enabled)

## Honeypot Filtering

Form inputs are checked for bot-trap patterns:
- Hidden via CSS (display:none, visibility:hidden, opacity:0)
- Zero dimensions (0x0 bounding box)
- Far off-screen positioning (left: -9999px)
- `aria-hidden="true"` on inputs
- Common honeypot field names (honeypot, hp_field, trap, etc.)

## Diff Detection

On repeated calls for the same page, elements new since the last call are prefixed
with `*` in the serialized output:

```
[1] button "Submit"          ← was here before
*[2] button "Cancel"         ← appeared since last call
[3] input[text] "Name"      ← was here before
```

## Bounding Box Deduplication

"Propagating parents" like `<a>`, `<button>`, `[role=button]`, `[role=combobox]` absorb
their children. If a child element is >95% contained within an interactive parent's
bounding box, the child is skipped to avoid redundant entries like:
```
[1] button "Submit"         ← kept
    span "Submit"           ← skipped (inside the button)
    svg icon                ← skipped (inside the button)
```

## When to Use What

| Situation | Approach |
|-----------|----------|
| **First visit to a page** | `getInteractiveElements()` — see everything clickable |
| **Need to fill a form** | `getInteractiveElements()` → `fillElement()` for each field |
| **Need full page structure** | `snapshotForAI()` for the accessibility tree |
| **Known stable selector** | Direct Playwright `click()`/`fill()` with `{ timeout: 2000 }` |
| **Canvas/WebGL app** | `snapshotForAI()` or keyboard navigation |
| **After an action** | Re-run `getInteractiveElements()` to see what changed (diff detection) |
