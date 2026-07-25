# Electron validation gotchas

## Launch shape is project-specific

Inspect `package.json`, the build scripts, preload, BrowserWindow creation, and
runtime dependencies before launching. React, Zustand, sidecars, and
electron-vite are conditional project choices, not Electron defaults.

Use `E_CLI_ENTRY` and `E_CLI_BUILD_CMD` when discovery is ambiguous. Use
`E_CLI_NODE_ENV` for deliberate test or production-like behavior; do not assume
the caller's environment is ignored.

## Multiple windows

Electron exposes one CDP page per BrowserWindow. Run `e-cli pages`, then persist
selection with page index/URL or exclusions. Re-discover when a selected window
closes or the app changes its window topology.

## Runtime diagnostics

Typechecking does not catch packaged import failures, preload exposure errors,
or renderer crashes. Use `e-cli logs`, the compact snapshot, page title/root
state, and a screenshot only when pixels add evidence.

An interrupted launch can leave a lock or provisional state. e-cli removes a
lock owned by a dead PID and refuses to signal a process whose ready-session CDP
identity cannot be verified.

## Preload and eval

Find the actual `contextBridge.exposeInMainWorld` name in the project. Read-only
`eval` expressions can inspect bridge shape, title, or DOM counts. Bridge method
calls execute privileged IPC against the app's real profile and may mutate
state; do not invoke them unless the user requested that effect.

## Custom-rendered surfaces

Terminal, canvas, and WebGL pixels may be absent from AX. Prefer the component's
read-only buffer/state interface when the project exposes one; otherwise use
bounded visual evidence and verify focus before sending keyboard input.
