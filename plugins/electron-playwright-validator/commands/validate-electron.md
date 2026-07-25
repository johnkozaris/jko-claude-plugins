---
description: Launch an Electron app and validate requested or discovered product states through e-cli
argument-hint: "[flow or states to validate]"
allowed-tools:
  - Bash
  - Task
  - Read
user-invocable: true
---

# Validate Electron

Invoke the `electron-playwright-validator` skill and validate `$ARGUMENTS`.

Resolve `scripts/e-cli` from the active skill directory. In a full plugin host,
use
`${CLAUDE_PLUGIN_ROOT}/skills/electron-playwright-validator/scripts/e-cli`.
Fail clearly if neither path resolves; `node "$E_CLI"` is a mode-independent
fallback.

Follow the skill's observable-transaction and bounded-visual workflow. Do not
assume tabs, matching headings, one window, or that every view needs visiting.
Use `pages`, fresh snapshots, postcondition-aware waits, named screenshots, and
`logs` as the target app requires.

On failure, include the relevant structured state, diagnostics, and artifact
path. Close only through e-cli's ownership-verified cleanup and report cleanup
failure.
