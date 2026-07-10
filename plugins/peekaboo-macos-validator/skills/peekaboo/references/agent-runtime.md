# Peekaboo in agent runtimes

Use this reference when Peekaboo runs under Claude Code, Copilot CLI, SSH,
LaunchAgents, or another subprocess-driven agent.

## Runtime and permission model

Peekaboo commands normally prefer a warm reusable daemon, then a capable
Peekaboo.app Bridge, then an operation-dependent local fallback. Screen
Recording, Accessibility, and Event Synthesizing grants belong to the process
that actually performs the operation, not necessarily the terminal that
started the agent.

Start diagnosis with:

```bash
peekaboo permissions status --all-sources --json
peekaboo bridge status --verbose --json
peekaboo daemon status --json
```

Require Screen Recording for capture and Accessibility for this skill's AX
inspection/actions. Event Synthesizing is optional for AX-only clicks but
required for background keyboard input, hotkeys, key presses, paste,
coordinate clicks, and synthetic click fallback.

Do not default to `--no-remote`. Background launchd/SSH agent sessions can
have apparently valid TCC state yet capture only wallpaper or redacted pixels.
Use local execution with an explicit capture engine only as a diagnostic when
the caller is in the active Aqua session.

## Observation and action hierarchy

Use the cheapest reliable observation:

1. `peekaboo inspect-ui --app-target "$BID" --json` for AX metadata only.
2. `peekaboo see --app "$BID" --json --annotate --path ...` when pixels,
   layout, or an auditable artifact matter.
3. `peekaboo image ...` for custom-rendered/canvas content where AX adds no
   value.

Use the most semantic reliable action:

1. `set-value` for a settable non-secure field when replacement semantics are
   correct.
2. `click --on "$ID" --snapshot "$SID"` for normal button activation.
3. `perform-action` for a specific supported AX action such as `AXShowMenu`,
   `AXIncrement`, or `AXDecrement`.
4. A label/query when no stable identifier exists.
5. Coordinates only for custom-drawn or AX-invisible surfaces.

Public 2026 Claude Code projects independently converge on this pattern:
AX setters/actions are faster and avoid focus theft; screenshots remain
valuable for discovery, visual critique, and fallback. Coordinate-map projects
also normalize and verify window geometry before replaying a coordinate and
pause before irreversible actions.

## Snapshot lifecycle

Element IDs are opaque. Copy them exactly from a fresh `see` or `inspect-ui`
result; never generate or parse meaning from their shape.

Treat one snapshot as one observation-action transaction:

1. Observe and persist JSON.
2. Extract the explicit `snapshot_id` and element ID.
3. Perform one state-changing action.
4. Observe again before the next ID-based action.

Mutating commands advance snapshot invalidation barriers. Avoid implicit
`latest` state in multi-command agent sessions, especially when several
Peekaboo clients or concurrent observations may be active.

Rapid repeated `see` calls can reuse a short-lived AX cache. If a just-mutated
tree appears unchanged, wait briefly and recapture rather than retrying the
same stale ID.

## Background and foreground delivery

Targeted `click`, `type`, `press`, `hotkey`, and `paste` use background
delivery by default. This is ideal for agent runtimes because it avoids
stealing the user's focus.

Add `--foreground` only when:

- the field accepts input only in the key window;
- a real mouse event is required;
- the action intentionally switches Spaces or brings a window forward;
- a double-click is required; or
- a background action fails with an explicit unsupported-delivery error.

With `--app`, `--pid`, or window flags, `--coords x,y` is relative to the
target window. `--global-coords` forces screen coordinates. Never mix the two
mental models.

For Peekaboo 3.8.0 and older, avoid trusting background positional clicks:
the 3.8.1 changelog contains the fix for top-left false success and truthful
foreground focus. Until 3.8.1 is installed, prefer ID/query actions or use
foreground coordinates and verify the result.

## JSON and artifacts

Always pass `--json` for automation and inspect both the envelope's `success`
field and the expected postcondition. Keep screenshots in a known directory:

```bash
ARTIFACT_DIR="${PEEKABOO_ARTIFACT_DIR:-$PWD/.artifacts/peekaboo}"
mkdir -p "$ARTIFACT_DIR"
```

When `see --json` omits `--path`, images remain in managed snapshot storage
and direct screenshot path fields can be empty. Pass `--path` whenever the
agent must `view` or retain the PNG.

For transitions or flaky flows, capture the action itself:

```bash
peekaboo capture action --app "$BID" --duration-limit 10 \
  --post-roll-ms 800 --path "$ARTIFACT_DIR/action" --json -- \
  <child command>
```

`capture action` propagates child failure/timeout and validates frames,
`contact.png`, and `metadata.json`. Parse the child exit details and inspect
the contact sheet or MP4; do not treat artifact creation alone as product
success.

## Security

- `set-value` rejects secure/password fields. Keep credentials out of shell
  arguments, logs, screenshots, and recordings.
- Peekaboo 3.6+ masks secure typing in visualizer events, but the safest
  validator flow still delegates credential entry to the user.
- Require confirmation before payment, deletion, sending, publishing, or any
  other irreversible action.

## 2026 compatibility notes

- **3.4.0** added `inspect-ui` and `capture action`.
- **3.5.3-3.5.4** hardened exact-window background actions, snapshot
  invalidation, opaque IDs, and explicit `see` artifact behavior.
- **3.6.0** fixed visual feedback routing and secure-typing masking.
- **3.7.0-3.7.1** bounded `capture action` child cleanup and reduced MCP image
  payloads.
- **3.8.0** changed Peekaboo.app's signing team. macOS may request one-time
  protected-data permission confirmation; the CLI retains legacy signing
  compatibility.
- **3.8.1** was unreleased on 2026-07-10. Do not assume its coordinate-click,
  stale-snapshot, daemon-start, and truthful window-resize fixes until the
  installed release includes them.

Require Peekaboo 3.4 or newer for this skill and recommend the latest stable
release. If behavior differs, check `peekaboo --version`, live
`peekaboo <command> --help`, and the current docs before adding a workaround.

## Primary sources

- Current docs: https://peekaboo.sh/
- Automation and delivery: https://peekaboo.sh/automation.html
- Permissions and agent-host caveats: https://peekaboo.sh/permissions.html
- Daemon and Bridge: https://peekaboo.sh/daemon.html and
  https://peekaboo.sh/bridge-host.html
- `see`: https://peekaboo.sh/commands/see.html
- `click`: https://peekaboo.sh/commands/click.html
- Semantic actions: https://peekaboo.sh/commands/set-value.html and
  https://peekaboo.sh/commands/perform-action.html
- Capture evidence: https://peekaboo.sh/commands/capture.html
- Release history: https://github.com/openclaw/Peekaboo/releases and
  https://github.com/openclaw/Peekaboo/blob/main/CHANGELOG.md
- Claude Code AX-first example: https://github.com/fletcherholt/ghost
- Claude Code coordinate fallback example:
  https://github.com/XiaoChu-1208/inner-coordinates
- Agent no-op false-success report:
  https://github.com/openclaw/Peekaboo/issues/245
