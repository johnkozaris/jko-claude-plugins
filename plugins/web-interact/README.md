# web-interact Plugin

Browser automation plugin for Claude Code, powered by the `web-interact` CLI.

## What It Does

Gives Claude Code full browser control: discover interactive elements, click, type, fill
forms, navigate pages, take screenshots, and automate web apps. Uses Chrome DevTools Protocol
for element detection (accessibility tree, DOMSnapshot, JS listener detection) and real
browser-level input events (CDP mouse/keyboard).

Works for standard web pages (discover → click/type by index) and visual content apps like
spreadsheets, design tools, and whiteboards (keyboard navigation + coordinate clicking).

The skill auto-triggers when Claude detects a task requiring browser interaction.

## Prerequisites

```bash
web-interact install
```

The `web-interact` CLI must be on `PATH` (or set `WEB_INTERACT_BIN`). Run `web-interact install`
once to provision the embedded runtime.

## Skills

### Web Interact (Browser Automation)

Auto-triggers when the task involves web interaction. Provides:
- `browser.discover()` — CDP-based interactive element detection
- `browser.click/type/select/check(index)` — act by index from discover
- `browser.clickAt(x, y)` / `browser.drag(x1, y1, x2, y2)` — coordinate actions
- Full Playwright Page API (`page.goto`, `page.evaluate`, `page.screenshot`)
- Error recovery, visual content area handling, screenshot best practices

### Reference Files (loaded on demand)

| File | Content |
|------|---------|
| `interactive-elements.md` | CDP discovery API, click/type/select/check, canvas patterns |
| `api-reference.md` | Full browser.*, page.* API with types |
| `workflow-patterns.md` | Login, forms, tabs, scraping, popups, connect mode |
| `error-recovery.md` | Failure catalog with recovery strategies |
| `large-pages.md` | Token efficiency, save-to-file patterns |
| `security.md` | Sandbox guarantees, prompt injection awareness |

## Commands

| Command | Description |
|---------|-------------|
| `/browse <url>` | Quick-navigate to URL, snapshot, and summarize |

## Hooks

- **SessionStart:** Detects the CLI, exports env vars, caches help output.
- **SessionEnd:** Stops the daemon to free resources.

## Modes

| Mode | Flag | Use case |
|------|------|----------|
| Headed | *(no flag)* | Default. Launches a visible Chrome session. |
| Headless | `--headless` | No visible window. For CI/scripted jobs. |
| Connect | `--connect` | Attach to your running Chrome (authenticated sessions). |

## License

MIT
