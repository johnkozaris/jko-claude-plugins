# Electron Gotchas

Common pitfalls when automating Electron apps with `e-cli` and Playwright CDP.

## Timing and Async Loading

### Lazy-loaded views

Many Electron apps use `React.lazy()` (or equivalent code-splitting) for view loading. When you click a tab, the view component loads asynchronously. The accessibility tree will briefly show a skeleton/loading state before the actual content appears.

**Pattern:** Always wait for a landmark element in the new view before snapshotting:
```bash
e-cli click 'role=tab[name="Rooms"]'
e-cli wait 'role=heading[name="Rooms"]' 5000
e-cli snapshot
```

### Initial app startup

The Electron app goes through several phases:
1. Main process starts (~1s)
2. BrowserWindow created, preload script runs (~1s)
3. Renderer loads HTML, React hydrates (~2-3s)
4. `window.api` bridge becomes available
5. Zustand stores initialize, data loads from sidecar

`e-cli launch` waits for step 3 (domcontentloaded). Steps 4-5 may still be in progress. If you need to interact with fully-loaded state, add a wait:
```bash
e-cli launch
e-cli wait 'role=navigation' 15000    # wait for nav to render
```

### Store hydration

State stores (Zustand/Redux/etc.) are often populated asynchronously via IPC events from the main process or a backend sidecar. UI elements that depend on store data may not render immediately.

If the backing service is not available (common in test/validation scenarios), some features will be empty or show fallback states. This is expected — validate the UI chrome, not the data.

## xterm.js Terminal

### Terminal content not in accessibility tree

xterm.js renders terminal content on a `<canvas>` element. Canvas content does **not** appear in the accessibility tree. `e-cli snapshot` will show the terminal container but not its text content.

To inspect terminal content:
```bash
# Check if terminal element exists
e-cli wait '.xterm' 5000

# Read terminal content via its buffer API
e-cli eval "document.querySelector('.xterm')?.textContent"

# Or check the underlying buffer
e-cli eval "document.querySelector('.xterm-screen')?.textContent?.substring(0, 500)"
```

### Terminal focus

xterm.js captures keyboard events aggressively. If a terminal is focused, `e-cli press` events will go to the terminal, not the app UI. Click outside the terminal first:
```bash
e-cli click 'role=navigation'     # move focus out of terminal
e-cli press 'Meta+n'              # now the app shortcut works
```

## Preload Bridge (window.api)

### Checking bridge availability

Preload scripts conventionally expose a bridge object (often `window.api` — grep the project's preload script for the actual `contextBridge.exposeInMainWorld` name). Verify it's loaded:
```bash
e-cli eval "typeof window.api"                    # should be "object"
e-cli eval "Object.keys(window.api ?? {}).length" # > 0 when the bridge loaded
```

### IPC calls in eval

You can invoke IPC methods directly:
```bash
e-cli eval "window.api.getSettings()"
e-cli eval "window.api.getPlatformInfo()"
```

But be cautious — some methods require the sidecar to be running and will throw or return errors without it.

## Multiple Windows

### Main vs auxiliary windows

Apps with multiple BrowserWindows (splash screens, settings panels, tool palettes) expose one CDP page per window. By default `e-cli` picks the first page. To skip auxiliary windows, set `E_CLI_EXCLUDE_URL` to comma-separated URL substrings before launching:

```bash
E_CLI_EXCLUDE_URL="splash.html,settings.html" "$E_CLI" launch
```

### Window not found

If `e-cli snapshot` reports "No renderer page found", the main window may not have opened yet or may have been closed. Check:
```bash
e-cli status                    # is the process running?
e-cli eval "1 + 1"              # basic connectivity check
```

## CDP Connection

### Port conflicts

`e-cli` refuses to launch when the requested port is already occupied, preventing it from attaching to an unrelated browser. Choose another port:
```bash
e-cli launch --port=9333
```

### Stale state

If `e-cli` reports the app is running but it's actually dead (force-killed, crashed), clean up manually:
```bash
e-cli close                     # cleans state even if process is dead
e-cli launch                    # fresh start
```

### Connection refused

If commands fail with "connection refused" after a successful launch, the Electron process may have crashed. Check:
```bash
e-cli status                    # validates PID is alive
e-cli close                     # clean up
e-cli launch                    # restart
```

## Build Issues

### Main entry not found

`e-cli launch` resolves the main entry from package.json `"main"` (falling back to `out/main/index.js`) and auto-builds if it is missing — using `E_CLI_BUILD_CMD` if set, else the project's own `build` script via its package manager, else `npx electron-vite build`. If the build fails (TypeScript errors, missing deps), the launch fails with the build error.

Fix build errors first by running the project's build directly (e.g. `pnpm build`), then `e-cli launch` again.

### Stale build

`e-cli launch` does NOT rebuild if the main entry already exists. If you've made changes and want to test the latest code:
```bash
"$E_CLI" close                        # stop current app
pnpm build                            # rebuild (or the project's build command)
"$E_CLI" launch                       # launch fresh
```

## NODE_ENV

`e-cli launch` sets `NODE_ENV=test`. This may affect:
- Feature flags gated on environment
- Logging levels
- Mock data vs real data
- Sidecar connection behavior

If you need production-like behavior, set the env explicitly before launch or modify the e-cli script.
