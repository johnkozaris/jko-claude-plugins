# Electron Gotchas

Common pitfalls when automating Electron apps with `e-cli` and Playwright CDP.

## Timing and Async Loading

### Lazy-loaded views

Kodosi (and many Electron apps) uses `React.lazy()` with `createResettableLazyComponent` for view loading. When you click a tab, the view component loads asynchronously. The accessibility tree will briefly show a skeleton/loading state before the actual content appears.

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

Runtime stores (`auth`, `runtime`, `catalog`, etc.) are populated asynchronously from the Rust sidecar via IPC events. UI elements that depend on store data may not render immediately.

If the sidecar is not available (common in test/validation scenarios), some features will be empty or show fallback states. This is expected — validate the UI chrome, not the data.

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

The preload script exposes `window.api` via `contextBridge`. Verify it's loaded:
```bash
e-cli eval "typeof window.api"                    # should be "object"
e-cli eval "Object.keys(window.api).length"       # should be ~70
```

### IPC calls in eval

You can invoke IPC methods directly:
```bash
e-cli eval "window.api.getSettings()"
e-cli eval "window.api.getPlatformInfo()"
```

But be cautious — some methods require the sidecar to be running and will throw or return errors without it.

## Multiple Windows

### Main vs Intel window

Kodosi has two windows:
- **Main window** — `index.html` → AppShell (the primary UI)
- **Intel window** — `intel.html` → IntelWindowShell (agent intelligence)

`e-cli` automatically targets the main window by filtering out pages with `intel.html` in their URL. You don't need to specify which window.

### Window not found

If `e-cli snapshot` reports "No renderer page found", the main window may not have opened yet or may have been closed. Check:
```bash
e-cli status                    # is the process running?
e-cli eval "1 + 1"              # basic connectivity check
```

## CDP Connection

### Port conflicts

If port 9222 is already in use (another Electron instance, Chrome DevTools), launch with a different port:
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

### out/main/index.js not found

`e-cli launch` auto-builds via `npx electron-vite build` if the output is missing. But if the build fails (TypeScript errors, missing deps), the launch will fail with a clear message.

Fix build errors first:
```bash
npx electron-vite build         # see the actual error
# fix the issue
e-cli launch                    # try again
```

### Stale build

`e-cli launch` does NOT rebuild if `out/main/index.js` already exists. If you've made changes and want to test the latest code:
```bash
e-cli close                           # stop current app
npx electron-vite build               # rebuild
e-cli launch                          # launch fresh
```

Or delete the output:
```bash
rm -rf out/
e-cli launch                          # will auto-build
```

## NODE_ENV

`e-cli launch` sets `NODE_ENV=test`. This may affect:
- Feature flags gated on environment
- Logging levels
- Mock data vs real data
- Sidecar connection behavior

If you need production-like behavior, set the env explicitly before launch or modify the e-cli script.
