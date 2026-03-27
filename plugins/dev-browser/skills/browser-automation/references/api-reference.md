# dev-browser Complete API Reference

## CLI Invocation

```
dev-browser [OPTIONS] [COMMAND]
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
| `install` | Download Playwright Chromium |
| `install-skill` | Install the dev-browser skill to `~/.claude/skills/` |
| `browsers` | List all managed browser instances with status and pages |
| `status` | Show daemon PID, uptime, socket path |
| `stop` | Stop daemon and all managed browsers |
| *(stdin)* | Read script from stdin (heredoc or pipe) |

### Invocation Patterns

```bash
# Heredoc (most common for AI agents)
dev-browser --headless <<'EOF'
const page = await browser.getPage("main");
await page.goto("https://example.com");
console.log(await page.title());
EOF

# File execution
dev-browser run script.js

# Pipe
echo 'console.log("hello")' | dev-browser --headless

# Connect to existing Chrome (auto-discover)
dev-browser --connect <<'EOF'
const tabs = await browser.listPages();
console.log(JSON.stringify(tabs, null, 2));
EOF

# Connect to specific endpoint
dev-browser --connect http://localhost:9222 <<'EOF'
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

---

### Page Object (Playwright Page API)

Pages returned by `browser.getPage()` and `browser.newPage()` are full Playwright 1.52.0
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

#### AI Snapshots (dev-browser extension — not standard Playwright)

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `snapshotForAI` | `snapshotForAI(options?: {track?: string, depth?: number, timeout?: number})` | `Promise<{full: string, incremental?: string}>` | ARIA accessibility tree snapshot |

`snapshotForAI()` is a **dev-browser extension**, not part of the standard Playwright API.
Standard Playwright has `locator.ariaSnapshot()` which returns a plain string. The dev-browser
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
| `click` | `click(selector: string, options?)` | `Promise<void>` | Click element |
| `dblclick` | `dblclick(selector: string, options?)` | `Promise<void>` | Double-click |
| `fill` | `fill(selector: string, value: string)` | `Promise<void>` | Clear and fill input |
| `type` | `type(selector: string, text: string, options?)` | `Promise<void>` | Type character by character |
| `press` | `press(selector: string, key: string)` | `Promise<void>` | Press key (Enter, Tab, Escape, ArrowDown, etc.) |
| `selectOption` | `selectOption(selector: string, value: string\|string[])` | `Promise<string[]>` | Select dropdown option(s) by value |
| `check` | `check(selector: string)` | `Promise<void>` | Check checkbox |
| `uncheck` | `uncheck(selector: string)` | `Promise<void>` | Uncheck checkbox |
| `hover` | `hover(selector: string, options?)` | `Promise<void>` | Hover over element |
| `focus` | `focus(selector: string)` | `Promise<void>` | Focus element |
| `dragAndDrop` | `dragAndDrop(source: string, target: string)` | `Promise<void>` | Drag from source to target |

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

// Count
const count = await page.locator(".item").count();

// Get all text contents
const texts = await page.locator(".item").allTextContents();

// Locator-scoped snapshot
const snap = await page.locator("main").snapshotForAI();
```

---

### File I/O

All file operations are restricted to the temp directory under `DEV_BROWSER_HOME`.
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
| Dynamic `import()` | Only relative modules loaded from temp files under `DEV_BROWSER_HOME/tmp/` |
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
| Temp files | `DEV_BROWSER_HOME/tmp/` only |

---

## File System Layout

| Path | Description |
|------|-------------|
| `DEV_BROWSER_HOME/daemon.sock` | Unix domain socket (macOS/Linux) |
| `DEV_BROWSER_HOME/daemon.pid` | Daemon process ID |
| `DEV_BROWSER_HOME/browsers/{name}/browser-profile/` | Persistent browser profile per instance |
| `DEV_BROWSER_HOME/tmp/` | Sandboxed file I/O directory |
