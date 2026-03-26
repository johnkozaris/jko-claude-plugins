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

## Quick Start

Check if dev-browser is available:

```bash
dev-browser status
```

If not installed, install it:

```bash
npm install -g dev-browser && dev-browser install
```

## Core Concepts

### Script Execution Model

Send scripts via Bash heredoc. Each script runs in an isolated QuickJS sandbox with these
globals: `browser`, `console`, `saveScreenshot`, `writeFile`, `readFile`, `setTimeout`.

```bash
dev-browser [--headless] [--connect [URL]] [--browser NAME] [--timeout SECONDS] <<'EOF'
// Script runs here. Top-level await is supported.
const page = await browser.getPage("main");
await page.goto("https://example.com");
console.log(await page.title());
EOF
```

### The One-Action-Per-Script Rule

Write small, focused scripts. Each script should do ONE thing: navigate, click, fill, or
read. End each script by logging the state needed for the next decision. This approach
uses fewer tokens and recovers better from errors than long multi-step scripts.

### Named Pages (Stateful Sessions)

Named pages persist across script runs within the same `--browser` instance:

```bash
# Script 1: Navigate (page persists after script ends)
dev-browser --headless <<'EOF'
const page = await browser.getPage("shop");
await page.goto("https://store.example.com");
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF

# Script 2: Interact with the SAME page (no re-navigation needed)
dev-browser --headless <<'EOF'
const page = await browser.getPage("shop");
await page.click('button:text("Add to Cart")');
console.log("Item added");
EOF
```

Use descriptive names: `"login"`, `"dashboard"`, `"checkout"` — not `"page1"`.

### Choosing a Mode

| Mode | Flag | When to use |
|------|------|-------------|
| **Connect** | `--connect` | **Default choice.** Attaches to user's Chrome (auto-discovers ports 9222-9229). Real fingerprint, bypasses bot detection, has user's cookies/auth. Launch Chrome with `--remote-debugging-port=9222` if not running. |
| **Headless** | `--headless` | Only for localhost, internal sites, or when user explicitly requests headless. No visible window. |
| **Headed** | *(no flag)* | When the user needs to watch the browser visually. |

**Prefer `--connect` over `--headless` for almost all browsing.** Headless Chromium has a
recognizable fingerprint that Cloudflare and similar services detect and block. Connect
mode uses the user's real Chrome, which passes bot detection automatically. Reserve
`--headless` only for localhost, internal sites, or when the user explicitly requests it.

To launch Chrome with remote debugging and connect (do this automatically when needed):
```bash
# Launch Chrome with debugging (runs in background, does not block)
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222 &

# Then connect (auto-discovers on ports 9222-9229)
dev-browser --connect <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com");
console.log(await page.title());
EOF
```

If Chrome is already running without debugging, ask the user to restart it with the flag,
or to enable debugging via `chrome://inspect/#remote-debugging`.

### Choosing an Approach for Unknown vs Known Pages

**Unknown page (first visit):** Use `snapshotForAI()` to discover the page structure, then
interact based on the ARIA accessibility tree output:

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://unknown-site.com");
const snap = await page.snapshotForAI();
console.log(snap.full);
EOF
```

**Known page (selectors already known):** Skip the snapshot. Use direct Playwright selectors
for speed — `page.click()`, `page.fill()`, `page.locator()`.

**Visual inspection needed:** Use `screenshot()` when layout, styling, or visual regressions
matter. `snapshotForAI()` returns structure, not appearance.

### fill() vs type()

- **`fill(selector, value)`** — Clears the field and sets the value instantly. Use for
  standard inputs where the final value matters.
- **`type(selector, text)`** — Types character by character with optional delay. Use for
  autocomplete inputs, search-as-you-type fields, or inputs that react to keystrokes.

### Output Patterns

Always log structured JSON so the output is parseable:

```javascript
console.log(JSON.stringify({ url: page.url(), title: await page.title(), status: "done" }));
```

- `console.log()` / `console.info()` → stdout (captured by Claude)
- `console.warn()` / `console.error()` → stderr

### Timeout Control

Default timeout is 30 seconds. Adjust with `--timeout`:
- `--timeout 10` — fast failure for simple checks
- `--timeout 60` — slow pages, large data extraction
- `--timeout 120` — multi-page pagination loops

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
| `page.snapshotForAI(options?)` | ARIA accessibility tree → `{full, incremental?}` *(dev-browser extension)* |
| `page.click(selector)` | Click element |
| `page.fill(selector, value)` | Clear and fill text input |
| `page.type(selector, text)` | Type character by character (for autocomplete) |
| `page.press(selector, key)` | Press key (Enter, Tab, Escape) |
| `page.selectOption(selector, value)` | Select dropdown option |
| `page.check(selector)` / `uncheck()` | Toggle checkbox |
| `page.getByRole(role, {name})` | Find element by ARIA role |
| `page.getByLabel(text)` | Find form element by its label |
| `page.locator(selector)` | Create locator for chained actions |
| `page.waitForSelector(selector)` | Wait for element to appear |
| `page.waitForURL(pattern)` | Wait for navigation (supports globs: `**/dashboard`) |
| `page.waitForResponse(url)` | Wait for a specific network response |
| `page.$$eval(selector, fn)` | Run function on all matching elements |
| `page.screenshot(options?)` | Capture screenshot buffer |
| `page.textContent(selector)` | Get text content of element |
| `page.setViewportSize({w, h})` | Set browser viewport dimensions |

Note: `page.evaluate(fn, args?)` runs **plain JavaScript** in the browser context. Use it
for DOM access (`document`, `window`) not available in the sandbox. Pass data via the second
argument; return values must be JSON-serializable. No TypeScript syntax inside evaluate.

### File I/O (sandboxed to `~/.dev-browser/tmp/`)

| Function | Description |
|----------|-------------|
| `saveScreenshot(buf, name)` | Save screenshot, returns file path |
| `writeFile(name, data)` | Write string to temp file, returns path |
| `readFile(name)` | Read temp file as UTF-8 |

For the complete API with all parameters and return types, consult
**`references/api-reference.md`**.

## Common Workflows

### Login to a Site

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("login");
await page.goto("https://app.example.com/login");
await page.fill('input[name="email"]', 'user@example.com');
await page.fill('input[name="password"]', 'password123');
await page.click('button[type="submit"]');
await page.waitForURL("**/dashboard");
console.log(JSON.stringify({ url: page.url(), title: await page.title() }));
EOF
```

### Take a Screenshot

```bash
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com");
const path = await saveScreenshot(await page.screenshot(), "capture.png");
console.log(path);
EOF
```

Then use `Read` tool on the returned path to view the image.

### Extract Data to File (Large Pages)

When page content is too large for the context window, extract to a file and use Read/Grep:

```bash
dev-browser --headless --timeout 60 <<'EOF'
const page = await browser.getPage("data");
await page.goto("https://example.com/large-table");
await page.waitForSelector("table");
const rows = await page.$$eval("table tbody tr", trs =>
  trs.map(tr => Array.from(tr.cells).map(c => c.textContent?.trim() ?? ""))
);
const path = await writeFile("extracted.json", JSON.stringify(rows, null, 2));
console.log(path);
EOF
```

Then: `Read` the file at the returned path, or `Grep` it for specific data.

For detailed workflow patterns covering forms, tabs, pagination, popups, cookie banners,
iframes, keyboard control, and connect mode, consult **`references/workflow-patterns.md`**.

## Decision Tree

1. **Need to interact with a webpage?** → Use dev-browser
2. **First time on this page?** → `snapshotForAI()` to discover structure
3. **Know the selectors?** → Direct Playwright calls (skip snapshot)
4. **Need authenticated session?** → `--connect` to attach to user's Chrome
5. **Page too large for context?** → `$$eval` to extract specific data → `writeFile()` → Read/Grep
6. **Need visual state?** → `screenshot()` + `saveScreenshot()`
7. **Multiple isolated sessions?** → `--browser name1`, `--browser name2`
8. **Page has iframes?** → `page.frameLocator()` — see `references/workflow-patterns.md`
9. **Page shows dialog/alert?** → `page.on("dialog", ...)` — see `references/workflow-patterns.md`
10. **Link opens new tab/popup?** → `page.waitForEvent("popup")` — see `references/workflow-patterns.md`
11. **Cookie consent banner blocking?** → Dismiss first — see `references/workflow-patterns.md`
12. **External website?** → Prefer `--connect` mode (bypasses bot detection automatically)
13. **Error occurred?** → Reconnect to same named page, screenshot for debug — see `references/error-recovery.md`

## Sandbox Limitations

Scripts run in QuickJS WASM, **NOT Node.js**. The following are unavailable:
- `require()`, `import()` — no module loading
- `fetch`, `WebSocket` — no direct network (use `page.goto()` instead)
- `fs`, `path`, `process`, `os` — no host access (use `writeFile`/`readFile`)
- `document`, `window` — not in sandbox scope (use `page.evaluate()` for DOM APIs)
- Playwright tracing, video recording, HAR routing — stubbed, throw at runtime

Inside `page.evaluate()`, write **plain JavaScript only** — no TypeScript syntax.

## Additional Resources

### Reference Files

For detailed documentation, consult these on demand:
- **`references/api-reference.md`** — Complete browser.*, page.*, file I/O API with types
- **`references/workflow-patterns.md`** — Login, forms, tabs, scraping, popups, iframes, cookie banners, connect mode, pagination
- **`references/error-recovery.md`** — Failure catalog with recovery strategies
- **`references/large-pages.md`** — Token efficiency, save-to-file patterns, section filtering
- **`references/security.md`** — Content boundaries, prompt injection awareness
