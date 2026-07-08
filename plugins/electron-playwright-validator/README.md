# electron-playwright-validator

Electron app UI automation and validation via Playwright CDP. Launch once, run fast commands to inspect, click, screenshot, and validate — all without a human in the loop.

## What It Does

Bridges the gap between Playwright's programmatic Electron API and CLI-driven automation. The `e-cli` tool manages a persistent Electron session via Chrome DevTools Protocol:

- **Launch** the Electron app with CDP enabled (~5s startup)
- **Snapshot** the accessibility tree to "see" the UI structure
- **Screenshot** for visual inspection via multimodal analysis
- **Click, fill, press, hover** to interact with UI elements
- **Evaluate** JavaScript in the renderer context
- **Wait** for async content before asserting

Session persistence means subsequent commands connect in ~200ms instead of re-launching.

## Installation

```bash
# As a plugin directory
claude --plugin-dir /path/to/myClaudeSkills/plugins/electron-playwright-validator

# Or install from the plugin collection
claude plugin install /path/to/myClaudeSkills
```

## Commands

| Command | Purpose |
|---------|---------|
| `/validate-electron` | Launch app, click through all tabs, report pass/fail |

## Skill

**electron-playwright-validator** — teaches Claude the full workflow for validating Electron app UI: launch, snapshot, interact, validate, close. Activates when asked to validate, check, test, or debug an Electron app's UI.

## CLI Tool: e-cli

The `bin/e-cli` script is **not** on PATH — invoke it via the plugin root: `"${CLAUDE_PLUGIN_ROOT}/bin/e-cli" <command>` (the skill instructs Claude to set `E_CLI="${CLAUDE_PLUGIN_ROOT}/bin/e-cli"` once per session). It resolves `electron` and `playwright` from the target project's own node_modules; nothing is installed globally.

| Command | Description |
|---------|-------------|
| `e-cli launch [--port=9222]` | Launch Electron with CDP |
| `e-cli snapshot` | Dump accessibility tree |
| `e-cli screenshot [path]` | Save PNG screenshot |
| `e-cli click <selector>` | Click element |
| `e-cli fill <selector> <value>` | Fill input field |
| `e-cli press <key>` | Keyboard press |
| `e-cli hover <selector>` | Hover over element |
| `e-cli text <selector>` | Print textContent |
| `e-cli eval <js>` | Evaluate JS in renderer |
| `e-cli wait <sel> [timeout]` | Wait for element visible |
| `e-cli close` | Close running app |
| `e-cli status` | Check if app is running |

## Prerequisites

The target Electron project must have in `devDependencies`:
- `electron`
- `@playwright/test` or `playwright-core`

## Hook

No active runtime hooks. Reserved for future command-based hooks.

## References

- **e-cli reference** — full command documentation with examples
- **accessibility selectors** — reading snapshot output, selector syntax, common patterns
- **electron gotchas** — timing issues, xterm quirks, lazy loading, preload bridge
