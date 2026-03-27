# dev-browser Plugin

Browser automation plugin for Claude Code, powered by the `dev-browser` CLI.

## What It Does

Gives Claude Code full browser control: navigate pages, click buttons, fill forms, take screenshots, extract data, and test web applications. Scripts run in a QuickJS WASM sandbox with Playwright Page API access.

Works for both development tasks (testing deploys, debugging UI, visual regression) and everyday tasks (checking email, browsing websites, filling online forms, working with web apps).

The skill auto-triggers when Claude detects a task requiring browser interaction — no explicit command needed.

## Prerequisites

```bash
dev-browser install
```

This plugin depends on the `dev-browser` CLI contract, not a specific checkout path.
Make sure `dev-browser` resolves on `PATH`, or set `DEV_BROWSER_BIN` to the absolute
path of the CLI wrapper/binary. The SessionStart hook exports `DEV_BROWSER_BIN` and
caches `dev-browser --help` at `DEV_BROWSER_HELP` for the skill to read.

## Skills

### Browser Automation

Auto-triggers when the task involves web interaction. Provides:
- Full dev-browser API reference (browser.*, page.*, file I/O)
- Decision trees: when to snapshot vs use direct selectors
- Large page handling: extract to file, depth-limited snapshots
- Error recovery catalog with recovery strategies
- Security guidance for prompt injection awareness

### Reference Files (loaded on demand)

| File | Content |
|------|---------|
| `api-reference.md` | Complete CLI flags, browser.*, page.* API with types, events |
| `workflow-patterns.md` | Login, forms, tabs, scraping, popups, cookie banners, connect mode, pagination, responsive testing |
| `error-recovery.md` | 14 failure types with recovery strategies |
| `large-pages.md` | Token efficiency, save-to-file, section-scoped snapshots |
| `security.md` | Sandbox guarantees, browser-level risks, prompt injection, credentials |

## Commands

| Command | Description |
|---------|-------------|
| `/browse <url>` | Quick-navigate to URL, snapshot, and summarize |

## Hooks

- **SessionStart:** Detects dev-browser installation, sets `DEV_BROWSER_AVAILABLE` env var.
- **SessionEnd:** Stops the dev-browser daemon to free resources.

## Modes

| Mode | Flag | Use case |
|------|------|----------|
| Headless | `--headless` | Default. No visible browser window. |
| Connect | `--connect` | Attach to your running Chrome (authenticated sessions). |
| Headed | *(no flag)* | Watch the browser visually. |

## License

MIT
