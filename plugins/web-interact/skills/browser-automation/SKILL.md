---
name: Web Interact
description: >-
  Browser automation via the web-interact plugin. Use whenever the task requires opening a
  real browser to navigate, interact with, or read web content — as opposed to simple curl,
  WebFetch, or WebSearch calls. This includes development work (testing web apps, inspecting
  deployed sites, debugging frontend issues, verifying deploys, running smoke tests, visual
  regression checks) as well as everyday user tasks (checking email, reading social media,
  browsing communities, filling online forms, working with web apps like Office 365 or
  Google Docs, exploring websites, managing accounts, online shopping). Trigger on phrases
  like "open a page", "go to [url]", "click", "fill out a form", "take a screenshot",
  "scrape", "extract data", "log into", "test the website", "check if deploy works",
  "read my email", "look at my Twitter", "browse Reddit", "fill in this form online", or
  any request that implies navigating and acting within a browser. Also trigger when the
  user mentions "web-interact", "web-interact", "headless browser", or "browser automation".
  Use for any task where WebFetch/WebSearch are insufficient because the page requires
  JavaScript rendering, authentication, multi-step interaction, form submission, or visual
  inspection.
effort: high
---

# Web Interact — Browser Automation

web-interact is a browser automation plugin powered by the `web-interact` CLI. Scripts run
in a QuickJS WASM sandbox with Playwright Page API access plus CDP-based element discovery
and interaction. Pages persist between script runs via named handles, enabling incremental
multi-step workflows.

## Bootstrap: Always Do This First

**Before your first web-interact command in any session, read the help output.** The
SessionStart hook resolves `WEB_INTERACT_BIN` or `web-interact` on `PATH` and caches help
at `$WEB_INTERACT_HELP`. It contains the full LLM USAGE GUIDE with correct,
version-accurate patterns, sandbox constraints, and API reference.

```bash
# Read the cached help (populated by SessionStart hook — no network call needed)
cat "$WEB_INTERACT_HELP"
```

If the cache file is missing or stale, regenerate it:
`"${WEB_INTERACT_BIN:-web-interact}" --help > "$WEB_INTERACT_HELP"`

If the hook reports `WEB_INTERACT_AVAILABLE=false`, install or build the CLI and ensure
`web-interact` resolves on `PATH` (or set `WEB_INTERACT_BIN`).

If the hook reports `WEB_INTERACT_INSTALL_NEEDED=true` or
`WEB_INTERACT_RUNTIME_STATUS=install-required`, run:

```bash
web-interact install
```

CLI detection alone is not enough — `web-interact install` must have been run at least
once so the embedded daemon runtime is present.

The ~250 lines of help are cheaper than a single failed script cycle. Read them every time.

**The help aligns with this skill on most patterns.** In particular, the help says "Use
short timeouts (--timeout 10)" — follow that. If any help example uses `--timeout 30` or
higher for a simple action, ignore it and use `--timeout 10`. The Calling Conventions
below always take precedence over help examples.

## Calling Conventions (How to Invoke the web-interact CLI)

These rules govern how you call web-interact from the Bash tool. Violating them causes
timeouts, dead time, and cascading failures.

1. **Run web-interact synchronously (foreground).** You need the output immediately to
   decide the next step. Do NOT set `run_in_background: true`. Do NOT prefix with `sleep`.

2. **web-interact `--timeout` is the only timeout that matters.** When running foreground,
   the Bash tool simply waits for web-interact to finish and returns stdout. Set
   `--timeout 10` on web-interact for most scripts. Set the Bash tool `timeout: 30000`
   (30s) as a generous safety net — it should never fire during normal operation.

3. **NEVER `sleep`.** Not before web-interact commands (Claude's processing time provides
   natural 3-5s pacing). Not to poll output files (run foreground instead). Not between
   retries (fix the root cause instead). For within-script pacing, use
   `page.waitForTimeout()` inside the web-interact script.

4. **`--timeout 10` is the default.** Most actions take 2-5 seconds. If a script times out
   at 10s, the selector is wrong — fix the selector, don't increase the timeout.

5. **Replay working patterns exactly.** If `page.getByRole('textbox').nth(2)` worked, use
   it again. Do not switch to `[ref=eN]` locators, `text=` selectors, or `fill()` on
   elements where `keyboard.type()` worked.

6. **One action per script.** Do not combine `snapshotForAI()` + interaction in a single
   script — the snapshot alone can take 3-5 seconds. Either: discover in one script,
   interact in the next with stable selectors; or skip snapshot and use `getByRole()` /
   `evaluate()` directly.

7. **Use per-action timeouts for uncertain selectors:** `{ timeout: 2000 }`. The default
   Playwright action timeout exceeds the script timeout, so one wrong selector in a
   single action will kill the entire script with an unhelpful "timed out" error. With
   `{ timeout: 2000 }`, the action fails fast with a specific error that names the
   selector, leaving time to try alternatives or log diagnostics.

## Core Concepts

### Script Execution Model

Send scripts via Bash heredoc. Each script runs in an isolated QuickJS sandbox with these
globals: `browser`, `console`, `saveScreenshot`, `writeFile`, `readFile`, `resolveFilePath`,
`deleteFile`, `setTimeout`.

```bash
web-interact [--headless] [--connect [URL]] [--browser NAME] [--timeout SECONDS] <<'EOF'
// Script runs here. Top-level await is supported.
const page = await browser.getPage("main");
await page.goto("https://example.com");
console.log(await page.title());
EOF
```

### The One-Action-Per-Script Rule

Each script should do ONE thing: navigate, click, fill, or read. End each script by
logging the state needed for the next decision. If step 3 of 5 fails in a multi-step
script, you lose all context and must restart. Small scripts make failures easy to diagnose.

### Named Pages (Stateful Sessions)

Named pages persist across script runs within the same `--browser` instance:

```bash
# Script 1: Navigate
web-interact --headless --timeout 10 <<'EOF'
const page = await browser.getPage("shop");
await page.goto("https://store.example.com");
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF

# Script 2: Interact with the SAME page (no re-navigation needed)
web-interact --headless --timeout 10 <<'EOF'
const page = await browser.getPage("shop");
await page.click('button:text("Add to Cart")');
console.log("Item added");
EOF
```

### Choosing a Mode

| Mode | Flag | When to use |
|------|------|-------------|
| **Connect** | `--connect` | **External sites.** Attaches to user's Chrome. Real fingerprint, bypasses bot detection, has user's cookies/auth. |
| **Headed** | *(no flag)* | **Localhost/internal sites.** Launches a visible managed browser. Profiles persist under the active `WEB_INTERACT_HOME`. |
| **Headless** | `--headless` | CI/scripted jobs only, or when user explicitly requests no window. Uses whatever managed headless browser runtime the installed CLI provisions. |

**CRITICAL: Managed automation browsers are blocked by Google sign-in** (and other strict bot detection).
Google shows "This browser or app may not be secure." For Google services (Gmail, Sheets,
Drive, YouTube) you MUST use `--connect` mode with real Chrome.

### Avoiding Rate Limits and Bot Detection (Google, Cloudflare, etc.)

Even in `--connect` mode with real Chrome, **behavioral patterns** trigger bot detection.
Google and other services flag rapid-fire automated interactions — rapid page loads,
instant clicks, many requests with zero delay. The fix is pacing and human-like patterns.

**Pacing comes from within scripts, not from shell `sleep`.** Claude's own processing time
between tool calls (reading output, deciding, formulating the next call) provides 3-5
seconds of natural delay. Do NOT add `sleep` between web-interact commands — that
contradicts the "never sleep" calling convention. Instead, add `waitForTimeout()` INSIDE
scripts to simulate human reading time:
```javascript
await page.goto("https://mail.google.com");
await page.waitForTimeout(1500);  // simulate reading the page
await page.click('tr:first-child');
await page.waitForTimeout(1000);  // simulate reading the email
```

**Batch reads, minimize round trips.** Instead of 5 scripts each reading one piece of data,
write one script that extracts everything in a single `page.evaluate()`. Fewer connections
= fewer detection signals.

**Click links for in-site navigation** rather than `page.goto()` — clicking is more
human-like than direct URL jumps.

**If already blocked:** Stop. Do not retry in a loop — that escalates the block. Wait a
few minutes. The user may need to solve a CAPTCHA manually in the browser.

### Launching Chrome for Connect Mode (Chrome 136+)

Chrome 136+ silently ignores `--remote-debugging-port` on the default profile. You MUST
use `--user-data-dir` with a dedicated debug profile.

```bash
# Launch Chrome with a dedicated debug profile.
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome \
  --remote-debugging-port=9222 \
  --user-data-dir="$HOME/.chrome-debug-profile" \
  >/tmp/web-interact-chrome-debug.log 2>&1 &
```

Then verify with `curl -s http://127.0.0.1:9222/json/version | head -1`. If the
endpoint is still unavailable, identify any stale debug Chrome PID with `ps` and stop
that specific PID via `kill <PID>` before relaunching. Common causes of failure:
- Missing `--user-data-dir` — Chrome 136+ silently ignores the port without it
- A stale debug-profile Chrome instance is still bound to the port

`~/.chrome-debug-profile` is the **standard debug profile**. Always reuse this exact path
so previous logins are preserved.

### Page Discovery Discipline

**Every new script starts from zero knowledge of the page.** Even if you interacted with
the same site seconds ago, the DOM may have changed. Follow this order:

0. **Use `browser.discover()` + act by index.** This is the primary workflow.
   `discover()` settles the page and returns an indexed inventory of all interactive
   elements — instantly, with no timeouts. Then `click`/`type`/`select`/`check` by index.
   ```javascript
   const els = await browser.discover("main");
   console.log(els.serialized);
   // [1] input "Name"
   // [2] input[email] "Email"
   // [3] button "Submit"
   await browser.type("main", 1, "John");    // type into [1]
   await browser.type("main", 2, "j@t.com"); // type into [2]
   await browser.click("main", 3);           // click [3]
   ```
   Uses real CDP mouse/keyboard events — works with React, Canvas, everything.

1. **For targeted checks, use `locator.count()`.** Returns immediately without waiting.

2. **For raw DOM inspection, use `page.evaluate()`.** One-line evaluate answers DOM
   questions instantly. Screenshots cost 3 round trips — use only for visual inspection.

2. **Use `page.evaluate()` for DOM questions, not screenshots.** A one-line evaluate
   (`document.querySelector('h2')?.textContent`) answers a DOM question instantly.
   Screenshots cost 3 round trips (take → save → Read tool) and only show visual layout.
   Use screenshots only when you need to see visual state (styling, layout, regressions).

3. **When selectors return wrong data, explore — don't screenshot.** Wrong data means the
   selectors don't match the current DOM. Run another quick evaluate to inspect what
   elements actually exist.

4. **Discover selectors at runtime, never hardcode third-party selectors.** Heavy web apps
   use obfuscated class names that change between deployments. Inspect structural patterns
   (ARIA roles, `data-*` attributes, tag hierarchy) rather than class names.

### Fail-Fast Interaction Patterns

Playwright actions (click, fill, type) use a **retry loop**: they repeatedly poll the DOM
for the element and check actionability (visible, stable, enabled, not obscured). If the
selector is wrong or the element doesn't exist, the action hangs for its full timeout
before failing. The default action timeout can exceed the script timeout, so a single bad
selector can kill the whole script with an unhelpful "Script timed out" error.

**1. Check existence before speculative interaction.**
`locator.count()` returns immediately without any retry loop — use it before any click on
an element that might not exist:
```javascript
const btn = page.getByRole('button', { name: 'Got it' });
if (await btn.count() > 0) await btn.click({ timeout: 2000 });
```

**2. Use per-action timeouts for uncertain selectors.**
When you're not 100% sure a selector is correct, add `{ timeout: 2000 }` so a wrong
selector fails in 2s with a specific error naming the selector:
```javascript
await page.click('.possibly-wrong-selector', { timeout: 2000 });
await page.fill('input[name="search"]', 'query', { timeout: 2000 });
```

**3. The `.catch(() => {})` trap — do NOT use this pattern.**
This looks safe but is deadly:
```javascript
// BAD: Playwright retries for the full action timeout before .catch() fires.
// With a 10s script timeout, the script dies before the catch ever runs.
await page.getByRole('button', { name: 'Got it' }).click().catch(() => {});
```
Use count-then-click instead (pattern 1 above).

**4. When `click()` hangs on an existing element: use evaluate for native click.**
Sometimes an element EXISTS in the DOM but fails Playwright's actionability checks
(obscured by overlay, needs special activation). The retry loop runs forever waiting for
the element to become "actionable". Bypass with a native DOM click:
```javascript
await page.evaluate(() => document.querySelector('.target')?.click());
```
Or use `{ force: true }` to skip Playwright's actionability checks:
```javascript
await page.click('.target', { force: true, timeout: 2000 });
```

**5. Discover the correct ARIA role before using `getByRole()`.**
Using the wrong role (e.g., `combobox` when it's actually a plain `input`) causes the
retry loop to search for an element that will never match. Discover the actual role first:
```javascript
const role = await page.evaluate(() =>
  document.querySelector('input.search-box')?.getAttribute('role') ?? 'no role attr'
);
console.log('Actual role:', role);
```

### Tab Management in Connect Mode

When connecting to a user's Chrome, find the right tab, connect by ID, and bring it to
the foreground:

```bash
web-interact --connect --timeout 10 <<'EOF'
const pages = await browser.listPages();
const target = pages.find(p => p.url.includes("example.com"));
const page = await browser.getPage(target.id);
await page.bringToFront();
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF
```

**Always call `page.bringToFront()`** — without it, interactions happen on a background tab
and the user sees nothing. Watch for duplicate tabs of the same site.

### Opening New Tabs from a Connected Page

```javascript
await page.evaluate(() => { window.open("https://example.com", "_blank"); });
await page.waitForTimeout(3000);
const updated = await browser.listPages();
const newTab = updated.find(p => p.url.includes("example.com"));
```

### fill() vs type()

- **`fill(selector, value)`** — Clears and sets instantly. Standard inputs.
- **`type(selector, text)`** — Types character by character. Autocomplete fields, canvas
  inputs, search-as-you-type. Use `{ delay: 5 }` for reliability.

### Output Patterns

Always log structured JSON so the output is parseable:

```javascript
console.log(JSON.stringify({ url: page.url(), title: await page.title(), status: "done" }));
```

### Stale Refs and snapshotForAI() Budget

ARIA `[ref=eN]` identifiers from `snapshotForAI()` are valid **only within the same
script run**. For cross-script targeting, use stable selectors:
- `page.getByRole('button', { name: 'Submit' })` — ARIA role + accessible name
- `page.getByLabel('Email')` — form labels
- `page.getByRole('textbox').nth(N)` — positional role selectors
- `page.locator('input[name="email"]')` — CSS selectors

**Do not combine `snapshotForAI()` with interactions in a short-timeout script.**
`snapshotForAI()` can take 3-5 seconds on complex pages, eating most of a `--timeout 10`
budget. Either: (a) use snapshot in a read-only discovery script, then interact in the
next script using stable selectors, or (b) skip snapshot entirely and use `getByRole()`
/ `evaluate()` for discovery and interaction in one script.

### Browser Instances

Each `--browser NAME` gets its own isolated instance (separate cookies, localStorage,
named pages). Default name is `"default"`.

## API Quick Reference

### browser.* (web-interact API)

| Method | Returns | Description |
|--------|---------|-------------|
| `browser.getPage(name)` | `Page` | Get or create a named page (persists across scripts) |
| `browser.newPage()` | `Page` | Create anonymous page (auto-closed when script ends) |
| `browser.listPages()` | `[{id, url, title, name}]` | List all tabs |
| `browser.closePage(name)` | `void` | Close a named page |
| `browser.discover(name, opts?)` | `{count, serialized, elements[]}` | **Settle + detect all interactive elements.** Returns indexed list. Primary entry point. |
| `browser.click(name, index)` | `{success, method}` | **CDP mouse click** on element [N] from discover(). Real browser events. |
| `browser.type(name, index, text)` | `{success, method}` | **CDP keyboard type** into element [N]. Real keyDown/char/keyUp events. |
| `browser.select(name, index, value)` | `{success, selectedText}` | Select dropdown option on element [N]. Fuzzy text matching. |
| `browser.check(name, index, checked?)` | `{success, checked}` | Toggle checkbox/radio on element [N]. |
| `browser.clickAt(name, x, y, opts?)` | `{success, x, y}` | **CDP click at viewport coordinates.** For canvas/WebGL apps. |
| `browser.drag(name, x1, y1, x2, y2)` | `{success}` | **CDP drag** between coordinates. For drawing, sliders, canvas interaction. |
| `browser.waitForSettled(name, opts?)` | `{settled, elapsed}` | Wait for network + DOM to settle. Called automatically by discover(). |
| `browser.getInteractiveElements(name, opts?)` | `{count, serialized, elements[]}` | Low-level: detect without settling. Prefer discover(). |
| `browser.clickElement(name, selector)` | `{success}` | Low-level: JS click by CSS selector. Prefer click(name, index). |
| `browser.fillElement(name, selector, value)` | `{success}` | Low-level: JS fill by selector. Prefer type(name, index, text). |

### Key page.* Methods (Playwright API)

| Method | Description |
|--------|-------------|
| `page.goto(url, options?)` | Navigate to URL |
| `page.snapshotForAI(options?)` | ARIA accessibility tree → `{full, incremental?}` |
| `page.click(selector)` | Click element |
| `page.fill(selector, value)` | Clear and fill text input |
| `page.type(selector, text)` | Type character by character |
| `page.press(selector, key)` | Press key (Enter, Tab, Escape) |
| `page.selectOption(selector, value)` | Select dropdown option |
| `page.check(selector)` / `uncheck()` | Toggle checkbox |
| `page.getByRole(role, {name})` | Find element by ARIA role |
| `page.getByLabel(text)` | Find form element by its label |
| `page.locator(selector)` | Create locator for chained actions |
| `page.waitForSelector(selector)` | Wait for element to appear |
| `page.waitForURL(pattern)` | Wait for navigation (supports globs) |
| `page.evaluate(fn, args?)` | Run plain JS in browser context |
| `page.screenshot(options?)` | Capture screenshot buffer |
| `page.bringToFront()` | Bring tab to foreground |
| `page.keyboard.type(text, opts?)` | Type via keyboard (for canvas apps) |
| `page.keyboard.press(key)` | Press a key |
| `page.setViewportSize({w, h})` | Set browser viewport dimensions |

### File I/O (sandboxed to the temp dir under `WEB_INTERACT_HOME`)

| Function | Description |
|----------|-------------|
| `saveScreenshot(buf, name)` | Save screenshot, returns file path |
| `writeFile(name, data)` | Write string to temp file, returns path |
| `readFile(name)` | Read temp file as UTF-8 |

For the complete API, consult **`references/api-reference.md`**.

## Visual Content Areas (canvas elements)

When `discover()` includes an element like `canvas (1200x800) — visual content area`,
the app renders content visually. `discover()` finds the toolbar but not the content.

**How to work with it:**
- **Toolbar:** `browser.click(index)` works normally for buttons, menus, sidebars
- **Content:** `page.keyboard` for typing/navigation, `browser.clickAt(x, y)` for
  clicking specific positions, `browser.drag(x1, y1, x2, y2)` for drawing/dragging
- **Overlays:** dismiss onboarding panels first — `discover()` finds Close buttons
- **Verify:** screenshot after every action. Navigate to target area BEFORE screenshotting.

**The loop:** discover → dismiss overlays → act → screenshot → verify → fix if wrong

For detailed API docs, see **`references/interactive-elements.md`**.

## Decision Tree

1. **Run `web-interact --help` yet this session?** → No? Read it first.
2. **Google service or strict bot detection?** → MUST use `--connect` with real Chrome
3. **External website?** → Prefer `--connect`; fall back to headed
4. **Localhost or internal?** → Headed mode (no flags)
5. **First visit to a page?** → `browser.discover()` — see all interactive elements instantly
6. **Standard page (buttons, forms, links)?** → `browser.click/type/select/check(index)` by index from discover
7. **discover() shows only toolbar/menus, no content?** → Keyboard navigation + `browser.clickAt(x,y)` for content area + screenshots to verify
8. **Overlay/sidebar blocking?** → `discover()` to find Close/Dismiss button → `browser.click(index)` to dismiss
9. **Action didn't work?** → Screenshot to see what happened → discover again → fix and retry
10. **User expects to see the browser?** → `page.bringToFront()`
11. **Multiple tabs of same site?** → `browser.listPages()` to find the right one
12. **Need visual state?** → Navigate to target area first → `page.screenshot()` + `saveScreenshot()` + Read tool
13. **Error from click/type?** → Read the error message — it tells you what the element is and suggests the right action

## Sandbox Limitations

Scripts run in QuickJS WASM, **NOT Node.js**. The following are unavailable:
- `require()`, `import()` — no module loading
- `fetch`, `WebSocket` — no direct network (use `page.goto()` instead)
- `fs`, `path`, `process`, `os` — no host access (use `writeFile`/`readFile`)
- `document`, `window` — not in sandbox scope (use `page.evaluate()` for DOM APIs)

Inside `page.evaluate()`, write **plain JavaScript only** — no TypeScript syntax.

## Additional Resources

For detailed documentation, consult these on demand:
- **`references/interactive-elements.md`** — Interactive element discovery API (getInteractiveElements, clickElement, fillElement, waitForSettled)
- **`references/api-reference.md`** — Complete browser.*, page.*, file I/O API with types
- **`references/workflow-patterns.md`** — Login, forms, tabs, scraping, popups, iframes, cookie banners, connect mode, pagination
- **`references/error-recovery.md`** — Failure catalog with recovery strategies
- **`references/large-pages.md`** — Token efficiency, save-to-file patterns, section filtering
- **`references/security.md`** — Content boundaries, prompt injection awareness
