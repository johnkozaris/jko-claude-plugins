# Web Interact — API Reference

## CLI Invocation

```
web-interact [OPTIONS] [COMMAND]
```

### Global Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--browser <NAME>` | string | `"default"` | Named browser instance. Each name has separate state (cookies, localStorage, named pages). |
| `--connect [URL]` | optional URL | *(none)* | Connect to running Chrome via CDP. Without URL: auto-discovers on ports 9222-9229. With URL: connects to specific endpoint. |
| `--headless` | flag | `false` | Launch Chromium without visible window. No effect when `--connect` is used. |
| `--timeout <SECONDS>` | integer (min 1) | `30` | Maximum script execution time in seconds. |

### Subcommands

| Command | Description |
|---------|-------------|
| `run <FILE>` | Execute a JavaScript file |
| `install` | Install the embedded Patchright runtime and headless Chromium |
| `install-skill` | Interactively install the embedded web-interact skill into supported agent skill directories |
| `browsers` | List all managed browser instances with status and pages |
| `status` | Show daemon PID, uptime, socket path |
| `stop` | Stop daemon and all managed browsers |
| *(stdin)* | Read script from stdin (heredoc or pipe) |

### Invocation Patterns

```bash
# Heredoc (most common for AI agents)
web-interact --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com");
console.log(await page.title());
EOF

# File execution
web-interact run script.js

# Pipe
echo 'console.log("hello")' | web-interact --headless

# Connect to existing Chrome (auto-discover)
web-interact --connect <<'EOF'
const tabs = await browser.listPages();
console.log(JSON.stringify(tabs, null, 2));
EOF

# Connect to specific endpoint
web-interact --connect http://localhost:9222 <<'EOF'
// ...
EOF
```

---

## Sandbox Globals

### browser Object

The `browser` global is a `ScriptBrowserAPI` instance.

#### browser.getPage(nameOrId)

Get or create a page by name, or connect to an existing tab by its targetId.

- **Parameter:** `nameOrId: string` — A descriptive name (creates new page if not found) OR a hex targetId from `listPages()` (pattern: `/^[a-f0-9]{16,}$/i`)
- **Returns:** `Promise<Page>` — Full Playwright Page object
- **Behavior:** Named pages persist across script runs within the same `--browser` instance. If the name does not exist, a new tab opens and is registered under that name.

#### browser.newPage()

Create an anonymous page.

- **Returns:** `Promise<Page>`
- **Behavior:** Anonymous pages are automatically closed when the script ends. Use for throwaway operations.

#### browser.listPages()

List all tabs in the browser.

- **Returns:** `Promise<Array<{id: string, url: string, title: string, name: string | null}>>`
- **Note:** `id` is the Chrome DevTools Protocol `targetId`. `name` is null for tabs not created via `getPage()`.

#### browser.closePage(name)

Close and deregister a named page.

- **Parameter:** `name: string`
- **Returns:** `Promise<void>`

#### browser.getInteractiveElements(nameOrId, options?)

Return an indexed inventory of all interactive, visible elements on the page. Runs
instantly via JS injection — no Playwright timeouts, no retry loops.

- **Parameter:** `nameOrId: string` — Page name or targetId
- **Parameter:** `options: object` (optional)
  - `viewportOnly: boolean` (default: false) — Only elements within/near viewport
  - `maxElements: number` (default: 200) — Cap on elements returned
  - `maxTextLength: number` (default: 100) — Truncate text at this length
  - `checkOcclusion: boolean` (default: false) — Filter elements hidden behind others
- **Returns:** `Promise<{count, serialized, elements, scrollableAreas, viewport}>`

**Usage pattern — discover then interact:**
```javascript
const result = await browser.getInteractiveElements("main", {});
console.log(result.serialized);
// [1] button "Submit"
// [2] input[email] "Email" placeholder="user@example.com"
// [3] link "Sign in" href="/login"
// [4] select "Country" (190 options: US, UK, DE, FR, JP, ... 185 more)

// Find the element you want and use its selector:
const target = result.elements.find(e => e.name === "Submit");
await page.click(target.selector, { timeout: 2000 });
```

**Serialization format:**
- `[N] tag "accessible name"` — index, HTML tag, computed accessible name
- `input[type]` — input type shown in brackets
- `placeholder=`, `href=` — key attributes inline
- `(N options: ...)` — select dropdown previews options
- `(format=YYYY-MM-DD)` — date/time format hints
- `(range 0-100 current=50)` — slider state
- `[checked]`, `[disabled]`, `[required]` — state indicators

**Detection heuristics** (12 checks): native interactive tags, ARIA roles,
contenteditable, inline event handlers, tabindex, ARIA state properties,
label wrappers, sized iframes, cursor:pointer fallback.

**Visibility filtering**: CSS display/visibility/opacity, bounding box, off-screen
honeypot detection, viewport bounds.

**When to use**: Before any interaction with an unfamiliar page. Instead of guessing
selectors and risking timeouts, call `getInteractiveElements` first to see what exists.

**Diff detection**: On the second and subsequent calls for the same page, elements that
are new since the last call are prefixed with `*`:
```
*[3] button "New Button"   ← appeared since last call
[4] input[text] "Name"     ← was there before
```

#### browser.waitForSettled(nameOrId, options?)

Wait for the page DOM to stop changing (MutationObserver-based). Useful before
`getInteractiveElements` on pages that load content dynamically.

- **Parameter:** `nameOrId: string` — Page name or targetId
- **Parameter:** `options: object` (optional)
  - `quietMs: number` (default: 500) — Milliseconds of no DOM mutations to consider settled
  - `timeout: number` (default: 5000) — Max wait time in milliseconds
- **Returns:** `Promise<{settled: boolean, elapsed: number, reason?: string}>`

```javascript
await browser.waitForSettled("main", { quietMs: 300, timeout: 3000 });
const result = await browser.getInteractiveElements("main", {});
```

#### browser.clickElement(nameOrId, selector, action?)

Click an element using its CSS selector via native DOM `click()`. Bypasses Playwright's
actionability checks and retry loop entirely — returns instantly.

- **Parameter:** `nameOrId: string` — Page name or targetId
- **Parameter:** `selector: string` — CSS selector (from `getInteractiveElements().elements[].selector`)
- **Parameter:** `action: string` (optional, default: `"click"`) — `"click"`, `"focus"`, or `"scrollIntoView"`
- **Returns:** `Promise<{success: boolean, tag?: string, text?: string, error?: string}>`

```javascript
const inv = await browser.getInteractiveElements("main", {});
const btn = inv.elements.find(e => e.name === "Submit");
if (btn) {
  await browser.clickElement("main", btn.selector);
}
```

#### browser.fillElement(nameOrId, selector, value)

Fill an input/textarea using its CSS selector via native DOM value setting. Uses the
native `HTMLInputElement.prototype.value` setter to bypass React/Vue controlled component
wrappers, then dispatches `input` and `change` events.

If the selector targets a `<label>`, automatically redirects to its associated form control.

- **Parameter:** `nameOrId: string` — Page name or targetId
- **Parameter:** `selector: string` — CSS selector
- **Parameter:** `value: string` — Value to fill
- **Returns:** `Promise<{success: boolean, tag?: string, value?: string, error?: string}>`

```javascript
const inv = await browser.getInteractiveElements("main", {});
const email = inv.elements.find(e => e.name === "Email");
if (email) {
  await browser.fillElement("main", email.selector, "user@example.com");
}
```

#### browser.selectOption(nameOrId, selector, value)

Select a dropdown option by value or visible text. Supports fuzzy matching (case-insensitive contains). Auto-redirects from `<label>` to `<select>`.

- **Returns:** `{success, selectedValue, selectedText}` or `{success: false, error, available}`

```javascript
const inv = await browser.getInteractiveElements("main", {});
const country = inv.elements.find(e => e.tag === "select");
if (country) await browser.selectOption("main", country.selector, "United Kingdom");
```

#### browser.checkElement(nameOrId, selector, checked?)

Toggle a checkbox or radio button. Pass `true` to check, `false` to uncheck, or omit to toggle. Auto-redirects from `<label>` to `<input>`.

- **Returns:** `{success, type, checked}`

```javascript
const inv = await browser.getInteractiveElements("main", {});
const agree = inv.elements.find(e => e.name === "I agree to terms");
if (agree) await browser.checkElement("main", agree.selector, true);
```

**Complete discover-then-act pattern (zero timeout risk):**
```javascript
// 1. Wait for page to settle
await browser.waitForSettled("main", { quietMs: 300 });

// 2. Get inventory of interactive elements
const inv = await browser.getInteractiveElements("main", {});
console.log(inv.serialized); // LLM reads this

// 3. Fill and click using selectors from inventory
const name = inv.elements.find(e => e.name === "Name");
const submit = inv.elements.find(e => e.name === "Submit");
if (name) await browser.fillElement("main", name.selector, "John Doe");
if (submit) await browser.clickElement("main", submit.selector);
```

---

### Page Object (Playwright Page API)

Pages returned by `browser.getPage()` and `browser.newPage()` are full Playwright 1.58.2
Page objects exposed through the QuickJS sandbox bridge. All async methods return Promises.

#### Navigation

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `goto` | `goto(url: string, options?: {waitUntil?: 'load'\|'domcontentloaded'\|'networkidle', timeout?: number})` | `Promise<Response\|null>` | Navigate to URL |
| `goBack` | `goBack(options?)` | `Promise<Response\|null>` | Navigate back |
| `goForward` | `goForward(options?)` | `Promise<Response\|null>` | Navigate forward |
| `reload` | `reload(options?)` | `Promise<Response\|null>` | Reload page |
| `url` | `url()` | `string` | Current URL (synchronous getter) |
| `title` | `title()` | `Promise<string>` | Current page title |

#### AI Snapshots (web-interact extension — not standard Playwright)

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `snapshotForAI` | `snapshotForAI(options?: {track?: string, depth?: number, timeout?: number})` | `Promise<{full: string, incremental?: string}>` | ARIA accessibility tree snapshot |

`snapshotForAI()` is a **web-interact extension**, not part of the standard Playwright API.
Standard Playwright has `locator.ariaSnapshot()` which returns a plain string. The web-interact
version adds tracking/incremental support and is available on both Page and Locator objects.
On a Locator, it returns the same `{full, incremental?}` shape — always access `.full`.

**Options detail:**
- `track` — Tracking key for incremental snapshots. When provided, subsequent calls with the same key return an `incremental` field containing only changes since the last snapshot.
- `depth` — Limits accessibility tree traversal depth. Use for large/complex pages.
- `timeout` — Max milliseconds to wait for snapshot (default: 30000).

**Return value:**
- `full` — Complete YAML-like accessibility tree with semantic roles, labels, and ref markers.
- `incremental` — Only present when `track` was provided and a previous snapshot with that key exists. Contains only what changed.

#### Element Selection

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `locator` | `locator(selector: string)` | `Locator` | Create a Locator for chained actions |
| `getByRole` | `getByRole(role: string, options?: {name?: string\|RegExp})` | `Locator` | Find by ARIA role |
| `getByText` | `getByText(text: string\|RegExp, options?)` | `Locator` | Find by text content |
| `getByLabel` | `getByLabel(text: string\|RegExp, options?)` | `Locator` | Find by label text |
| `getByPlaceholder` | `getByPlaceholder(text: string\|RegExp, options?)` | `Locator` | Find by placeholder |
| `getByTestId` | `getByTestId(testId: string\|RegExp)` | `Locator` | Find by data-testid |

#### Interaction

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `click` | `click(selector, options?: {timeout?, force?, position?})` | `Promise<void>` | Click element. `force` skips actionability checks. |
| `dblclick` | `dblclick(selector, options?: {timeout?, force?})` | `Promise<void>` | Double-click |
| `fill` | `fill(selector, value, options?: {timeout?})` | `Promise<void>` | Clear and fill input |
| `type` | `type(selector, text, options?: {timeout?, delay?})` | `Promise<void>` | Type character by character |
| `press` | `press(selector, key, options?: {timeout?})` | `Promise<void>` | Press key (Enter, Tab, Escape, ArrowDown, etc.) |
| `selectOption` | `selectOption(selector, value\|values, options?: {timeout?})` | `Promise<string[]>` | Select dropdown option(s) by value |
| `check` | `check(selector, options?: {timeout?, force?})` | `Promise<void>` | Check checkbox |
| `uncheck` | `uncheck(selector, options?: {timeout?, force?})` | `Promise<void>` | Uncheck checkbox |
| `hover` | `hover(selector, options?: {timeout?, force?})` | `Promise<void>` | Hover over element |
| `focus` | `focus(selector, options?: {timeout?})` | `Promise<void>` | Focus element |
| `dragAndDrop` | `dragAndDrop(source, target, options?: {timeout?})` | `Promise<void>` | Drag from source to target |

**Per-action timeout:** All interaction methods accept `{ timeout: N }` in milliseconds.
Use shorter values for speculative interactions — the default action timeout can exceed
the script timeout, so a wrong selector in one action can kill the entire script:
```javascript
await page.click('.uncertain-selector', { timeout: 2000 });
await page.fill('input.maybe-here', 'value', { timeout: 2000 });
```
If the element is not found within the timeout, the action throws a specific error that
names the selector and the timeout — much more useful than a generic "Script timed out."

**Force click:** `page.click(selector, { force: true })` bypasses Playwright's
actionability checks (element must be visible, enabled, stable, not obscured). Use when
an element exists in the DOM but click() hangs due to overlays or other obstructions.
Alternative: `await page.evaluate(() => document.querySelector(sel)?.click())` for a
native DOM click that bypasses Playwright entirely.

#### Content Extraction

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `textContent` | `textContent(selector: string)` | `Promise<string\|null>` | Text content of element |
| `innerText` | `innerText(selector: string)` | `Promise<string>` | Visible text |
| `innerHTML` | `innerHTML(selector: string)` | `Promise<string>` | Inner HTML |
| `inputValue` | `inputValue(selector: string)` | `Promise<string>` | Value of input element |
| `getAttribute` | `getAttribute(selector: string, name: string)` | `Promise<string\|null>` | Get attribute value |
| `isVisible` | `isVisible(selector: string)` | `Promise<boolean>` | Check visibility |
| `isEnabled` | `isEnabled(selector: string)` | `Promise<boolean>` | Check if enabled |
| `isChecked` | `isChecked(selector: string)` | `Promise<boolean>` | Check if checked |

#### Inspection Helpers

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `consoleMessages` | `consoleMessages(options?: {filter?: 'all'\|'sinceNavigation'})` | `Promise<ConsoleMessage[]>` | Read buffered browser console messages |
| `pageErrors` | `pageErrors(options?: {filter?: 'all'\|'sinceNavigation'})` | `Promise<Error[]>` | Read buffered uncaught page errors |
| `requests` | `requests()` | `Promise<Request[]>` | Read tracked network requests for the current page |

#### JavaScript in Browser Context

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `evaluate` | `evaluate(fn: Function\|string, arg?)` | `Promise<any>` | Run plain JS in page context |
| `$eval` | `$eval(selector: string, fn: Function)` | `Promise<any>` | Run function on first match |
| `$$eval` | `$$eval(selector: string, fn: Function)` | `Promise<any>` | Run function on all matches |

**Critical:** `evaluate`, `$eval`, and `$$eval` run in the **browser context** (access to
`document`, `window`, DOM APIs). Write **plain JavaScript only** — no TypeScript.
The sandbox scope (`browser`, `console`, `writeFile`) is NOT available inside these calls.
Pass data via the second argument; return values must be JSON-serializable.

#### Waiting

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `waitForSelector` | `waitForSelector(selector: string, options?)` | `Promise<ElementHandle>` | Wait for element |
| `waitForURL` | `waitForURL(url: string\|RegExp\|Function)` | `Promise<void>` | Wait for navigation. Supports glob: `**/dashboard` |
| `waitForLoadState` | `waitForLoadState(state?: 'load'\|'domcontentloaded'\|'networkidle')` | `Promise<void>` | Wait for page load state |
| `waitForFunction` | `waitForFunction(fn: Function\|string, arg?, options?)` | `Promise<JSHandle>` | Wait for function to return truthy |
| `waitForTimeout` | `waitForTimeout(ms: number)` | `Promise<void>` | Hard wait (avoid if possible) |
| `waitForResponse` | `waitForResponse(url: string\|RegExp\|Function)` | `Promise<Response>` | Wait for a specific network response |
| `waitForEvent` | `waitForEvent(event: string, options?)` | `Promise<any>` | Wait for a page event (e.g., `"popup"`, `"download"`) |

#### Viewport and Emulation

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `setViewportSize` | `setViewportSize({width: number, height: number})` | `Promise<void>` | Set browser viewport dimensions |
| `viewportSize` | `viewportSize()` | `{width: number, height: number}\|null` | Get current viewport size |

```javascript
// Responsive testing
await page.setViewportSize({ width: 375, height: 812 }); // iPhone viewport
await page.setViewportSize({ width: 1920, height: 1080 }); // Desktop
```

#### Screenshots

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `screenshot` | `screenshot(options?: {fullPage?: boolean, type?: 'png'\|'jpeg', quality?: number})` | `Promise<Buffer>` | Capture screenshot buffer |

**Note:** The `path` option is stripped in the sandbox. Always use `saveScreenshot()` to persist:
```javascript
const path = await saveScreenshot(await page.screenshot(), "name.png");
```

Full-page screenshot: `page.screenshot({ fullPage: true })`
Element screenshot: `page.locator("header").screenshot()`

#### Frames

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `frame` | `frame(nameOrUrl: string)` | `Frame\|null` | Get frame by name or URL |
| `frames` | `frames()` | `Frame[]` | List all frames |
| `mainFrame` | `mainFrame()` | `Frame` | Get main frame |

Frames have the same API as Page for selectors, interaction, and content extraction.

#### Keyboard and Mouse

| Object | Method | Description |
|--------|--------|-------------|
| `page.keyboard` | `type(text)` | Type text |
| `page.keyboard` | `press(key)` | Press key |
| `page.keyboard` | `down(key)` / `up(key)` | Hold/release key |
| `page.mouse` | `click(x, y)` | Click at coordinates |
| `page.mouse` | `move(x, y)` | Move cursor |
| `page.mouse` | `wheel(deltaX, deltaY)` | Scroll |

#### Page Events

Register event handlers with `page.on(event, handler)`:

| Event | Handler Signature | Description |
|-------|------------------|-------------|
| `"dialog"` | `(dialog) => {}` | Browser alert/confirm/prompt. Call `dialog.accept()` or `dialog.dismiss()`. |
| `"popup"` | `(page) => {}` | New tab/window opened via `window.open()` or `target="_blank"`. Returns a Page object. |
| `"download"` | `(download) => {}` | File download triggered. Use `download.path()` to get the temp file path. |
| `"console"` | `(msg) => {}` | Browser console message. `msg.type()` returns `"log"`, `"error"`, etc. `msg.text()` returns content. |
| `"pageerror"` | `(error) => {}` | Uncaught JS exception in the page. `error.message` has the error text. |

```javascript
// Handle popups (new tabs opened by links)
const [popup] = await Promise.all([
  page.waitForEvent("popup"),
  page.click('a[target="_blank"]'),
]);
console.log("Popup URL:", popup.url());

// Capture browser console errors
page.on("console", msg => {
  if (msg.type() === "error") console.log("Browser error:", msg.text());
});
```

---

### Locator Object

Locators returned by `page.locator()`, `page.getByRole()`, etc. support chaining:

```javascript
// Chain locators for precision
const row = page.locator("table tr").filter({ hasText: "John" });
const editBtn = row.locator("button:text('Edit')");
await editBtn.click();

// Nth element
await page.locator(".item").nth(0).click();

// First/last
await page.locator(".item").first().click();
await page.locator(".item").last().click();

// Count — returns IMMEDIATELY, no retry loop, no waiting
const count = await page.locator(".item").count();
// Use count() to check existence before interacting:
const btn = page.getByRole('button', { name: 'Submit' });
if (await btn.count() > 0) {
  await btn.click({ timeout: 2000 });
}

// Get all text contents
const texts = await page.locator(".item").allTextContents();

// Locator-scoped snapshot
const snap = await page.locator("main").snapshotForAI();
```

---

### File I/O

All file operations are restricted to the temp directory under `WEB_INTERACT_HOME`.
Security enforcements:
no path traversal (`..`), no symlinks, no absolute paths, null byte filtering, 0o600 permissions.

#### saveScreenshot(buffer, name)

- **Parameters:** `buffer: Buffer` (from `page.screenshot()`), `name: string` (filename)
- **Returns:** `Promise<string>` — Absolute file path
- **Subdirectories:** Allowed; parents are created automatically.
  ```javascript
  await saveScreenshot(buf, "session1/step3.png");
  ```

#### writeFile(name, data)

- **Parameters:** `name: string` (filename), `data: string` (content)
- **Returns:** `Promise<string>` — Absolute file path
- **Usage:** Save extracted data, HTML content, JSON results.
  ```javascript
  const path = await writeFile("results.json", JSON.stringify(data, null, 2));
  ```

#### readFile(name)

- **Parameters:** `name: string` (filename)
- **Returns:** `Promise<string>` — File content as UTF-8
- **Usage:** Read previously written temp files.
  ```javascript
  const data = JSON.parse(await readFile("results.json"));
  ```

---

### Console API

| Method | Output destination |
|--------|-------------------|
| `console.log(args...)` | stdout |
| `console.info(args...)` | stdout |
| `console.debug(args...)` | stdout |
| `console.warn(args...)` | stderr |
| `console.error(args...)` | stderr |

Arguments are formatted with `util.inspect()` (depth 6, compact mode 3). Use
`JSON.stringify()` for machine-readable output that Claude can parse.

---

## Unavailable in Sandbox

The QuickJS WASM sandbox explicitly does **not** provide:

| Feature | Alternative |
|---------|-------------|
| `require()` | Not available. All API is via globals. |
| Dynamic `import()` | Only relative modules loaded from temp files under `WEB_INTERACT_HOME/tmp/` |
| `fetch` / `XMLHttpRequest` | Use `page.goto()` to load URLs in the browser |
| `fs` / `path` / `os` / `process` | Use `writeFile()` / `readFile()` for temp I/O |
| `document` / `window` | Available inside `page.evaluate()` only |
| `Buffer` constructor | Polyfilled. `page.screenshot()` returns Buffer. |
| Playwright video recording | Stubbed — throws at runtime |
| `browserType.connect()` | Disabled. Browser ownership stays on host side. |
| `browserType.launchPersistentContext()` | Disabled. Use `--browser NAME` instead. |

---

## Resource Limits

| Resource | Limit |
|----------|-------|
| Memory | 512 MB (QuickJS heap) |
| Timeout | `--timeout` flag (default 30s) |
| Concurrent scripts | 1 per `--browser` name (serialized via lock) |
| Temp files | `WEB_INTERACT_HOME/tmp/` only |

---

## File System Layout

| Path | Description |
|------|-------------|
| `WEB_INTERACT_HOME/daemon.sock` | Unix domain socket (macOS/Linux) |
| `WEB_INTERACT_HOME/daemon.pid` | Daemon process ID |
| `WEB_INTERACT_HOME/browsers/{name}/browser-profile/` | Persistent browser profile per instance |
| `WEB_INTERACT_HOME/tmp/` | Sandboxed file I/O directory |

`web-interact install` must be run at least once so the embedded runtime under
`WEB_INTERACT_HOME` is available.
