---
description: Check Peekaboo, jq, macOS compatibility, selected-runtime permissions, and capture/AX readiness
argument-hint: ""
allowed-tools:
  - Bash
user-invocable: true
---

# Peekaboo Doctor

Report prerequisites and actionable fixes without changing application state or
deleting artifacts.

1. Confirm `peekaboo` and `jq` exist. Print the installed Peekaboo version.
2. Confirm the supported macOS version.
3. Run `peekaboo permissions status --all-sources --json` and identify the
   selected runtime. Screen Recording and Accessibility are baseline;
   Event Synthesizing is required only for flows that use its input paths.
4. Run Bridge and daemon diagnostics. Do not assume the terminal owns grants and
   do not force `--no-remote` merely because the caller is an agent.
5. List visible applications and select one non-secure app for a capture/AX
   self-test. Do not assume Finder is present in every selected runtime.
6. Capture to a temporary doctor directory, persist JSON, and report only
   snapshot ID, element count, interactable count, and artifact paths.

A successful self-test proves capture and AX inspection, not keyboard,
coordinate, or every application action. Report those capabilities separately
from their permission state.

If permissions are missing, report the runtime that needs the grant and offer
the relevant Peekaboo permission command. Do not open settings or change grants
without the user's action.

Report managed snapshot age/size if useful, but offer `peekaboo clean` only
after confirmation. A health check must not delete prior evidence.
