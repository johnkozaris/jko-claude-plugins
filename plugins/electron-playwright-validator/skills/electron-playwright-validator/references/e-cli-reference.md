# e-cli Command Reference

`e-cli` is a CLI wrapper around Playwright's Electron automation API. It maintains a persistent session via Chrome DevTools Protocol (CDP), enabling fast sequential commands against a running Electron app.

## Prerequisites

The project must have these in `devDependencies`:
- `electron` — the Electron framework
- `@playwright/test` or `playwright-core` — Playwright automation library

The tool resolves these from the **project's own `node_modules`** using `createRequire(process.cwd())`. No global installs needed.

## State File

`e-cli` stores session state in `.e-cli-state.json` in the project root:

```json
{
  "pid": 12345,
  "port": 9222
}
```

Add `.e-cli-state.json` and `.e-cli-screenshot.png` to `.gitignore`.

## Commands

### `e-cli launch [--port=9222]`

Start the Electron app with CDP enabled.

**Behavior:**
1. Checks if an app is already running (reads state file, validates PID)
2. Resolves the project's `electron` binary
3. If `out/main/index.js` doesn't exist, runs `npx electron-vite build`
4. Spawns Electron with `--remote-debugging-port=<port>`
5. Waits for CDP to become available (polls `/json/version`)
6. Connects via Playwright, waits for the renderer page to reach `domcontentloaded`
7. Saves `{ pid, port }` to `.e-cli-state.json`

**Options:**
- `--port=<number>` — CDP port (default: 9222)

**Examples:**
```bash
e-cli launch                  # default port 9222
e-cli launch --port=9333      # custom port
```

**Output:**
```
Launching on CDP port 9222...
Ready (pid: 45678, port: 9222)
```

If already running:
```
Already running (pid: 45678, port: 9222)
```

---

### `e-cli snapshot`

Dump the full accessibility tree of the main renderer page as indented text.

**Output format:**
```
WebArea "Kodosi"
  navigation "Main"
    tab "Sessions" pressed=true
    tab "Rooms"
  main
    heading "Sessions" level=1
```

Each line shows: `role "name" [attributes]`. Indentation reflects the tree hierarchy.

**Attributes shown:** `value`, `checked`, `pressed`, `expanded`, `disabled`.

**Example:**
```bash
e-cli snapshot
```

---

### `e-cli screenshot [path]`

Save a PNG screenshot of the main renderer page.

**Arguments:**
- `path` (optional) — output file path. Default: `.e-cli-screenshot.png` in project root.

**Output:** Prints the absolute path to the saved file.

**Examples:**
```bash
e-cli screenshot                          # saves to .e-cli-screenshot.png
e-cli screenshot /tmp/app-state.png       # custom path
```

---

### `e-cli click <selector>`

Click an element matching the selector.

**Selector formats:**
- `role=tab[name="Sessions"]` — role selector (preferred)
- `text=Sessions` — text content match
- `[data-testid=session-list]` — attribute selector
- `.sidebar button` — CSS selector

**Examples:**
```bash
e-cli click 'role=tab[name="Rooms"]'
e-cli click 'text=Create Session'
e-cli click '[data-testid=new-session-btn]'
e-cli click '.header .menu-button'
```

---

### `e-cli fill <selector> <value>`

Clear an input field and type a new value.

**Arguments:**
- `selector` — element selector (first argument)
- `value` — text to type (remaining arguments, joined with spaces)

**Examples:**
```bash
e-cli fill '[data-testid=session-name]' "my-session"
e-cli fill 'role=textbox[name="Search"]' "hello world"
```

---

### `e-cli press <key>`

Send a keyboard event to the focused element.

**Key format:** Playwright key names — `Enter`, `Escape`, `Tab`, `Backspace`, `ArrowDown`, `Meta+n`, `Control+c`, `Shift+Tab`.

**Examples:**
```bash
e-cli press Enter
e-cli press Escape
e-cli press Meta+n
e-cli press Tab
```

---

### `e-cli hover <selector>`

Hover over an element to trigger tooltips, dropdown menus, or hover states.

**Examples:**
```bash
e-cli hover 'role=button[name="Settings"]'
e-cli hover '.user-avatar'
```

---

### `e-cli text <selector>`

Print the `textContent` of the first element matching the selector.

**Examples:**
```bash
e-cli text 'role=heading[level=1]'        # print the page heading
e-cli text '.status-bar'                  # print status bar content
e-cli text '[data-testid=error-message]'  # read an error message
```

---

### `e-cli eval <js>`

Evaluate a JavaScript expression in the renderer context and print the result as JSON.

**Arguments:** All remaining arguments are joined as a single JS expression.

**Examples:**
```bash
e-cli eval "document.title"
e-cli eval "Object.keys(window.api).length"
e-cli eval "document.querySelectorAll('.session-item').length"
e-cli eval "window.api.getSettings()"
```

**Output:** JSON-formatted result.

---

### `e-cli wait <selector> [timeout_ms]`

Wait for an element to become visible.

**Arguments:**
- `selector` — element selector
- `timeout_ms` (optional) — timeout in milliseconds (default: 10000)

**Examples:**
```bash
e-cli wait 'role=tab[name="Sessions"]'           # default 10s timeout
e-cli wait '[data-testid=session-list]' 30000     # 30s timeout
```

**Output:**
```
Visible: role=tab[name="Sessions"]
```

---

### `e-cli close`

Terminate the running Electron app and clean up the state file.

Sends `SIGTERM` to the stored PID and removes `.e-cli-state.json`.

**Example:**
```bash
e-cli close
```

**Output:**
```
Closed.
```

---

### `e-cli status`

Check if the Electron app is currently running.

Reads the state file and validates the PID is alive. Cleans up stale state if the process has died.

**Example:**
```bash
e-cli status
```

**Output (running):**
```
Running (pid: 45678, port: 9222)
```

**Output (not running):**
```
Not running.
```

Exit code: `0` if running, `1` if not.

---

## Error Messages

| Error | Cause | Fix |
|-------|-------|-----|
| `electron not found in project devDependencies` | No `electron` package | `pnpm add -D electron` |
| `@playwright/test or playwright-core not found` | No Playwright package | `pnpm add -D @playwright/test` |
| `Build failed — out/main/index.js not created` | electron-vite build error | Fix build errors, then retry |
| `CDP not available on port 9222 after 30s` | Electron crashed on startup | Check for main process errors |
| `No running app. Run: e-cli launch` | State file missing | Launch the app first |
| `App not running (stale state cleaned)` | Process died since launch | Launch again |
| `No renderer page found` | App started but no window opened | Check main process window creation |

## Page Selection

When the app has multiple windows (e.g., main app + agent intel window), `e-cli` targets the **main app window** by default. It filters out pages whose URL contains `intel.html`.
