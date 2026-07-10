---
description: Health check for peekaboo — verifies install, version, permissions, and prints actionable next steps
argument-hint: ""
allowed-tools:
  - Bash
user-invocable: true
---

# Peekaboo Doctor

Verify that `peekaboo` is correctly installed and permitted to drive
macOS apps. Prints a status table and explicit next steps when something
is missing.

This is the verbose counterpart to the silent `SessionStart` hook.

## Steps

### 1. Binary present?

```bash
if ! command -v peekaboo >/dev/null 2>&1; then
  echo "❌ peekaboo not installed."
  echo "   Install: brew install steipete/tap/peekaboo"
  exit 1
fi
peekaboo --version
```

### 2. macOS version

```bash
sw_vers -productVersion
```

Released Peekaboo CLI builds require macOS 15.0+. If this is < 15, stop with
an unsupported-platform error.

Parse the installed Peekaboo version printed above. Require >= 3.4 for this
plugin and recommend the latest stable release. If Homebrew reports Peekaboo
as outdated, print `brew upgrade steipete/tap/peekaboo`; do not upgrade
without the user's approval.

### 3. Permissions

```bash
peekaboo permissions status --all-sources --json \
  | jq '.data | {
      selectedSource,
      sources: [.sources[] | {
        source, displayName, isSelected,
        permissions: [.permissions[] | {name,isRequired,isGranted}]
      }]
    }'
```

**Screen Recording** and **Accessibility** must be granted to the selected
runtime. **Event Synthesizing** is also required when the flow uses background
typing, hotkeys, key presses, paste, coordinate clicks, or synthetic fallback.

If either required permission is missing, stop and run:

```bash
peekaboo permissions grant
```

For missing Event Synthesizing, run:

```bash
peekaboo permissions request-event-synthesizing
```

Do not assume the terminal owns the grant. In Claude Code, Copilot CLI, SSH,
or launchd sessions, the selected source can be a daemon or Peekaboo.app
Bridge.

### 4. Runtime host diagnostics

```bash
peekaboo bridge status --verbose --json
peekaboo daemon status --json
```

Report the selected host and why. A stopped daemon is informational because
normal commands can auto-start it. Do not force `--no-remote` merely because
the agent is a subprocess; background sessions should prefer a permissioned
Bridge/daemon.

### 5. Self-test: list a known app

```bash
peekaboo list apps --json \
  | jq '.data.applications[]|select(.name=="Finder")|{name,bundleIdentifier,processIdentifier}'
```

Finder is always running. If this returns nothing AND permissions look
granted, the selected runtime likely lacks AX entitlement. Restart that
runtime after granting Accessibility.

### 6. Snapshot self-test (optional but very informative)

```bash
ARTIFACT_DIR="${PEEKABOO_ARTIFACT_DIR:-${TMPDIR:-/tmp}/peekaboo-doctor}"
mkdir -p "$ARTIFACT_DIR"
peekaboo see --app Finder --json --annotate \
  --path "$ARTIFACT_DIR/doctor.png" > "$ARTIFACT_DIR/doctor.json"
ls -la "$ARTIFACT_DIR/doctor.png" "$ARTIFACT_DIR/doctor_annotated.png"
jq '.data | {snapshot_id, element_count, interactable_count}' \
  "$ARTIFACT_DIR/doctor.json"
```

If `doctor_annotated.png` exists and `element_count > 0`, you're
fully operational.

### 7. Cache hygiene

```bash
peekaboo clean --older-than 24
```

Each `see` call writes ~100–500 KB to `~/.peekaboo/snapshots/<UUID>/`.
Running `clean` weekly keeps that bounded.

### 8. Report

Print a final summary:

```
## Peekaboo Doctor

| Check                       | Status |
|-----------------------------|--------|
| Binary installed            | ✅ <version>   |
| macOS version               | ✅ 15.x        |
| Selected runtime            | ✅ daemon      |
| Screen Recording permission | ✅             |
| Accessibility permission    | ✅             |
| Event Synthesizing          | ✅             |
| List apps                   | ✅ Finder      |
| Snapshot self-test          | ✅ 142 elements |

All systems go — `peekaboo` is ready.
```

If anything is ❌ or ⚠️, include the explicit fix command for that row.
