---
description: Run a Maestro flow against the booted iOS Simulator or Android emulator.
argument-hint: "[flow-file-or-dir]"
allowed-tools:
  - Read
  - Edit
  - Write
  - Glob
  - Bash
user-invocable: true
---

# /validate-mobile

Execute a Maestro flow. Handles simulator boot checks and produces human-readable output.

## Steps

1. **Locate the flow.** If the user passed a path, use it. Otherwise look for `**/Tests/maestro/*.yaml` or `**/maestro/*.yaml`. If multiple, ask which; if none, offer to scaffold a `launch.yaml` smoke flow.

2. **Ensure a simulator is booted.** `xcrun simctl list devices booted` — if empty, boot one the project commonly uses (check for a justfile target or existing test config hint).

3. **Run the flow.**

   ```bash
   maestro test <flow-path>
   ```

   For a full suite:

   ```bash
   maestro test --format junit --output report.xml <dir>
   ```

4. **On failure**, inspect the output:
   - Grab screenshots from `~/.maestro/tests/<run>/screenshots/`
   - Read the last N lines of the Maestro log for the specific step that failed
   - Suggest probable causes from the `mobile-flows-maestro` skill's flakiness section

5. **JAVA_HOME sanity check.** If Maestro errors with "Unable to locate a Java Runtime", make sure JAVA_HOME is exported:

   ```fish
   set -Ux JAVA_HOME /opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home
   ```

## Guardrails

- Do not modify flows in-place without showing the user the diff first
- If the flow targets a production app ID, confirm before running (unlikely but possible)

## When to defer to the skill

For writing new flows, debugging flakiness, handling OAuth/ASWebAuthenticationSession, GraalJS `runScript` constraints, or CI integration, invoke the `mobile-flows-maestro` skill directly.
