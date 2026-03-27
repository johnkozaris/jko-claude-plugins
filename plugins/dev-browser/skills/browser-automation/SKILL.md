---
name: Browser Automation (dev-browser)
description: >-
  This skill provides browser automation via the dev-browser CLI. It should be used whenever
  the task requires opening a real browser to navigate, interact with, or read web content —
  as opposed to simple curl, WebFetch, or WebSearch calls. This includes development work
  (testing web apps, inspecting deployed sites, debugging frontend issues, verifying deploys,
  running smoke tests, visual regression checks) as well as everyday user tasks (checking
  email, reading social media, browsing communities, filling online forms, working with web
  apps like Office 365 or Google Docs, exploring websites, managing accounts, online shopping).
  Trigger on phrases like "open a page", "go to [url]", "click", "fill out a form", "take a
  screenshot", "scrape", "extract data", "log into", "test the website", "check if deploy works",
  "read my email", "look at my Twitter", "browse Reddit", "fill in this form online", or any
  request that implies navigating and acting within a browser. Also trigger when the user
  mentions "dev-browser", "headless browser", or "browser automation". Use this skill for any
  task where WebFetch/WebSearch are insufficient because the page requires JavaScript rendering,
  authentication, multi-step interaction, form submission, or visual inspection.
effort: high
---

# Browser Automation with dev-browser

dev-browser is a CLI that controls browsers using sandboxed JavaScript scripts. Scripts run
in a QuickJS WASM sandbox (not Node.js) with Playwright Page API access plus dev-browser
extensions (notably `snapshotForAI()`). Pages persist between script runs via named handles,
enabling incremental multi-step workflows.

## Bootstrap: Always Do This First

**Before your first dev-browser command in any session, read the help output.** The
SessionStart hook resolves `DEV_BROWSER_BIN` or `dev-browser` on `PATH` and caches help
at `$DEV_BROWSER_HELP`. It contains the full LLM USAGE GUIDE with correct,
version-accurate patterns, sandbox constraints, and API reference.

```bash
# Read the cached help (populated by SessionStart hook — no network call needed)
cat "$DEV_BROWSER_HELP"
```

If the cache file is missing or stale, regenerate it:
`"${DEV_BROWSER_BIN:-dev-browser}" --help > "$DEV_BROWSER_HELP"`

If the hook reports `DEV_BROWSER_AVAILABLE=false`, install or build the CLI and ensure
`dev-browser` resolves on `PATH` (or set `DEV_BROWSER_BIN`).

If the hook reports `DEV_BROWSER_INSTALL_NEEDED=true` or
`DEV_BROWSER_RUNTIME_STATUS=install-required`, run:

```bash
dev-browser install
```

CLI detection alone is not enough — `dev-browser install` must have been run at least
once so the embedded daemon runtime is present.

The ~250 lines of help are cheaper than a single failed script cycle. Read them every time.

**The help aligns with this skill on most patterns.** In particular, the help says "Use
short timeouts (--timeout 10)" — follow that. If any help example uses `--timeout 30` or
higher for a simple action, ignore it and use `--timeout 10`. The Calling Conventions
below always take precedence over help examples.

## Calling Conventions (How to Invoke dev-browser from Claude Code)

These rules govern how you call dev-browser from the Bash tool. Violating them causes
timeouts, dead time, and cascading failures.

1. **Run dev-browser synchronously (foreground).** You need the output immediately to
   decide the next step. Do NOT set `run_in_background: true`. Do NOT prefix with `sleep`.

2. **dev-browser `--timeout` is the only timeout that matters.** When running foreground,
   the Bash tool simply waits for dev-browser to finish and returns stdout. Set
   `--timeout 10` on dev-browser for most scripts. Set the Bash tool `timeout: 30000`
   (30s) as a generous safety net — it should never fire during normal operation.

3. **NEVER `sleep`.** Not before dev-browser commands (Claude's processing time provides
   natural 3-5s pacing). Not to poll output files (run foreground instead). Not between
   retries (fix the root cause instead). For within-script pacing, use
   `page.waitForTimeout()` inside the dev-browser script.

4. **`--timeout 10` is the default.** Most actions take 2-5 seconds. If a script times out
   at 10s, the selector is wrong — fix the selector, don't increase the timeout.

5. **Replay working patterns exactly.** If `page.getByRole('textbox').nth(2)` worked, use
   it again. Do not switch to `[ref=eN]` locators, `text=` selectors, or `fill()` on
   elements where `keyboard.type()` worked.

6. **One action per script.** Do not combine `snapshotForAI()` + interaction in a single
   script — the snapshot alone can take 3-5 seconds. Either: discover in one script,
   interact in the next with stable selectors; or skip snapshot and use `getByRole()` /
   `evaluate()` directly.

## Core Concepts

### Script Execution Model

Send scripts via Bash heredoc. Each script runs in an isolated QuickJS sandbox with these
globals: `browser`, `console`, `saveScreenshot`, `writeFile`, `readFile`, `resolveFilePath`,
`deleteFile`, `setTimeout`.

```bash
dev-browser [--headless] [--connect [URL]] [--browser NAME] [--timeout SECONDS] <<'EOF'
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
dev-browser --headless --timeout 10 <<'EOF'
const page = await browser.getPage("shop");
await page.goto("https://store.example.com");
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF

# Script 2: Interact with the SAME page (no re-navigation needed)
dev-browser --headless --timeout 10 <<'EOF'
const page = await browser.getPage("shop");
await page.click('button:text("Add to Cart")');
console.log("Item added");
EOF
```

### Choosing a Mode

| Mode | Flag | When to use |
|------|------|-------------|
| **Connect** | `--connect` | **External sites.** Attaches to user's Chrome. Real fingerprint, bypasses bot detection, has user's cookies/auth. |
| **Headed** | *(no flag)* | **Localhost/internal sites.** Launches a visible managed browser. Profiles persist under the active `DEV_BROWSER_HOME`. |
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
seconds of natural delay. Do NOT add `sleep` between dev-browser commands — that
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
  >/tmp/dev-browser-chrome-debug.log 2>&1 &
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

1. **Discover first, interact second.** Run a lightweight `page.evaluate()` to explore the
   DOM structure before attempting to interact. Never guess selectors.

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

### Tab Management in Connect Mode

When connecting to a user's Chrome, find the right tab, connect by ID, and bring it to
the foreground:

```bash
dev-browser --connect --timeout 10 <<'EOF'
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

### browser.* (dev-browser API)

| Method | Returns | Description |
|--------|---------|-------------|
| `browser.getPage(name)` | `Page` | Get or create a named page (persists across scripts) |
| `browser.newPage()` | `Page` | Create anonymous page (auto-closed when script ends) |
| `browser.listPages()` | `[{id, url, title, name}]` | List all tabs |
| `browser.closePage(name)` | `void` | Close a named page |

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

### File I/O (sandboxed to the temp dir under `DEV_BROWSER_HOME`)

| Function | Description |
|----------|-------------|
| `saveScreenshot(buf, name)` | Save screenshot, returns file path |
| `writeFile(name, data)` | Write string to temp file, returns path |
| `readFile(name)` | Read temp file as UTF-8 |

For the complete API, consult **`references/api-reference.md`**.

## Canvas-Based Web Apps (Google Sheets, Figma, etc.)

Canvas-rendered UIs have no DOM nodes to select. Use **keyboard navigation** and
**role-based selectors** for toolbar elements.

**Google Sheets pattern:**
- Navigate to cells via the Name Box: `page.getByRole('textbox').nth(2)` → click → `Meta+a`
  → type cell ref (e.g., `A1`) → `Enter`
- Type cell content with `page.keyboard.type(text, { delay: 5 })` — `fill()` does not work
- `Tab` = next column, `Enter` = next row
- Dismiss popups/sidebars first — they steal focus:
  ```javascript
  await page.getByRole('button', { name: 'Got it' }).click().catch(() => {});
  await page.getByRole('button', { name: 'Close' }).click().catch(() => {});
  ```

## Decision Tree

1. **Run `dev-browser --help` yet this session?** → No? Read it first.
2. **Google service or strict bot detection?** → MUST use `--connect` with real Chrome
3. **External website?** → Prefer `--connect`; fall back to headed
4. **Localhost or internal?** → Headed mode (no flags)
5. **First visit to this page?** → Discover structure with `evaluate()` or `snapshotForAI()`
6. **Heavy/complex SPA?** → Use `evaluate()` with targeted selectors, not `snapshotForAI()`
7. **Canvas-based app?** → Keyboard navigation + role selectors
8. **User expects to see the browser?** → `page.bringToFront()`
9. **Multiple tabs of same site?** → `browser.listPages()` to find the right one
10. **Selector returning wrong data?** → Explore DOM with `evaluate()`, don't screenshot
11. **Need visual state?** → `screenshot()` + `saveScreenshot()` + Read tool
12. **Popup/sidebar stealing focus?** → Dismiss first with `.click().catch(() => {})`
13. **Error occurred?** → Reconnect to same named page, explore with `evaluate()`

## Sandbox Limitations

Scripts run in QuickJS WASM, **NOT Node.js**. The following are unavailable:
- `require()`, `import()` — no module loading
- `fetch`, `WebSocket` — no direct network (use `page.goto()` instead)
- `fs`, `path`, `process`, `os` — no host access (use `writeFile`/`readFile`)
- `document`, `window` — not in sandbox scope (use `page.evaluate()` for DOM APIs)

Inside `page.evaluate()`, write **plain JavaScript only** — no TypeScript syntax.

## Additional Resources

For detailed documentation, consult these on demand:
- **`references/api-reference.md`** — Complete browser.*, page.*, file I/O API with types
- **`references/workflow-patterns.md`** — Login, forms, tabs, scraping, popups, iframes, cookie banners, connect mode, pagination
- **`references/error-recovery.md`** — Failure catalog with recovery strategies
- **`references/large-pages.md`** — Token efficiency, save-to-file patterns, section filtering
- **`references/security.md`** — Content boundaries, prompt injection awareness
