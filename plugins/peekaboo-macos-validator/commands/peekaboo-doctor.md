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

Peekaboo 3.x targets recent macOS. If this is < 13, warn the user.

### 3. Permissions

```bash
peekaboo list permissions --json | jq '.permissions[]|{name,isGranted}'
```

Both **Screen Recording** and **Accessibility** must be granted to the
process invoking peekaboo (your terminal, or the agent host). Grant them
in System Settings → Privacy & Security.

If either is missing, stop and instruct the user step-by-step.

### 4. Self-test: list a known app

```bash
peekaboo list apps --json \
  | jq '.data.applications[]|select(.name=="Finder")|{name,bundleIdentifier,processIdentifier}'
```

Finder is always running. If this returns nothing AND permissions look
granted, the agent likely lacks AX entitlement — re-launch the terminal
after granting Accessibility.

### 5. Snapshot self-test (optional but very informative)

```bash
peekaboo see --app Finder --json --annotate --path /tmp/doctor.png \
  > /tmp/doctor.json
ls -la /tmp/doctor.png /tmp/doctor_annotated.png
jq '.data | {snapshot_id, element_count, interactable_count}' /tmp/doctor.json
```

If `/tmp/doctor_annotated.png` exists and `element_count > 0`, you're
fully operational.

### 6. Cache hygiene

```bash
peekaboo clean --older-than 24
```

Each `see` call writes ~100–500 KB to `~/.peekaboo/snapshots/<UUID>/`.
Running `clean` weekly keeps that bounded.

### 7. Report

Print a final summary:

```
## Peekaboo Doctor

| Check                       | Status |
|-----------------------------|--------|
| Binary installed            | ✅ 3.0.0-beta4 |
| macOS version               | ✅ 14.x        |
| Screen Recording permission | ✅             |
| Accessibility permission    | ✅             |
| List apps                   | ✅ Finder      |
| Snapshot self-test          | ✅ 142 elements |

All systems go — `peekaboo` is ready.
```

If anything is ❌ or ⚠️, include the explicit fix command for that row.
