# Peekaboo in agent runtimes

Use this reference for daemon/Bridge permissions, background delivery,
coordinate fallback, and snapshot lifecycle.

## Runtime and permissions

Peekaboo normally prefers a reusable daemon, then a capable Peekaboo.app
Bridge, then an operation-dependent local fallback. Permissions belong to the
process that performs the operation, not necessarily the terminal that started
the agent.

Start diagnosis with:

```bash
peekaboo permissions status --all-sources --json
peekaboo bridge status --verbose --json
peekaboo daemon status --json
```

Require Screen Recording for capture and Accessibility for AX inspection and
actions. Event Synthesizing is conditional on background keyboard input,
hotkeys, paste, coordinates, or synthetic fallback.

Do not default to `--no-remote`. A background process can report local grants
while capturing wallpaper or redacted pixels; inspect the selected runtime
before changing engines.

## Snapshot lifecycle

Estimate JSON size before running a command. Persist accessibility trees,
browser snapshots, capture results, and other likely-large responses, then
query only the needed fields. Element IDs are opaque: copy them from a fresh
result and never derive meaning from their shape.

Treat a snapshot as one observation/action transaction:

1. Persist the observation.
2. Extract the explicit snapshot and target IDs.
3. Perform one state-changing action.
4. Observe again before another ID-based action.

Rapid observations can briefly reuse an AX cache. If a just-mutated tree appears
unchanged, wait briefly and recapture instead of replaying a stale action.

## Background and coordinate behavior

Targeted click and keyboard actions normally use background delivery. Add
`--foreground` only when evidence shows the control needs the key window, a real
mouse event, a Space switch, or another foreground-only behavior.

Coordinates with an app/PID/window target are window-relative.
`--global-coords` means screen coordinates. Normalize and verify window geometry
before coordinate fallback.

## JSON and artifacts

Pass `--json` for automation and assert both the envelope's success field and a
product postcondition. Pass `--path` when a screenshot must be retained or
visually inspected; otherwise images can remain only in managed snapshot
storage.

Treat every screenshot as large. Track every task-created path and keep pixels
in the visual context described by `visual-verification.md`. After the reader
returns its compact report, terminate it with the host's kill/stop/remove
control and verify through worker listing that it is gone. Do not create a
reader when those controls are unavailable.

Delete temporary screenshots, JSON, traces, videos, contact sheets, and empty
artifact directories by exact path as soon as they are no longer needed and
again before the final response. Retain only requested artifacts or necessary
failure evidence. For transitions, prefer `capture action` so the child result
and evidence share one bounded capture.

Require Peekaboo 3.4 or newer. When behavior differs, check the installed
version, live command help, and current changelog before preserving a
version-specific workaround.

Primary documentation:

- https://peekaboo.sh/
- https://peekaboo.sh/automation.html
- https://peekaboo.sh/permissions.html
- https://peekaboo.sh/daemon.html
- https://peekaboo.sh/commands/see.html
- https://peekaboo.sh/commands/click.html
- https://peekaboo.sh/commands/capture.html
