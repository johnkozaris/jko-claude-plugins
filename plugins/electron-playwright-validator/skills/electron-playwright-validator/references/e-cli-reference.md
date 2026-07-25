# e-cli interface

Treat `e-cli --help` as the command source of truth. This reference explains
state, ownership, and environment behavior that help cannot show compactly.

## Requirements

- Node.js 18 or newer
- Electron in the target project's dependencies
- `@playwright/test` or `playwright-core` in that project

The script resolves modules from `E_CLI_PROJECT` or the current working
directory. It does not install dependencies.

## State and ownership

The project-local state file records PID, CDP port, launch status, private
stderr-log path, creation time, persisted page selection, and the exact
child-issued DevTools WebSocket identity. It is written atomically with mode
`0600`.

Ready sessions verify the live CDP identity before connecting or terminating a
process. Malformed or mismatched state is rejected rather than trusted. Log
deletion is restricted to e-cli-owned files in the OS temporary directory.

Add `.e-cli-state.json`, `.e-cli-launch.lock`, and `.e-cli-screenshot-*.png` to
the target project's ignore rules.

## Launch inputs

Use environment variables for host-portable configuration:

- `E_CLI_PROJECT`: Electron package root
- `E_CLI_ENTRY`: main-process entry override
- `E_CLI_BUILD_CMD`: explicit build command when the entry is absent
- `E_CLI_NODE_ENV`: environment override
- `E_CLI_STARTUP_TIMEOUT_MS`: startup bound
- `E_CLI_PAGE_INDEX`, `E_CLI_PAGE_URL`, `E_CLI_EXCLUDE_URL`: persisted window
  selection
- `E_CLI_USER_DATA_DIR`: explicit Electron profile
- `E_CLI_APP_ARGS`: JSON string array passed after the entry

Launch never invents an `npx` build. It uses an explicit build command or a
project build script with an identifiable lockfile/package manager.

`pages` lists renderer index, title, URL, and current selection. Use it before
persisting a page selector in a multi-window app.

## Evidence interfaces

- `snapshot` is compact by default; `--full` includes uninteresting AX nodes.
- `screenshot [path] --selector=<selector>` captures a page or one element.
- `wait` supports Playwright states, text, URL, or a trusted predicate.
- `logs [lines]` tails Electron/renderer stderr captured through Electron
  logging.
- `eval` executes arbitrary JavaScript with the renderer and preload bridge's
  privileges. Treat it as a trusted, side-effect-capable escape hatch.

`close` terminates the process group only after CDP ownership verification.
