# maestro-mobile-validator

iOS and Android mobile app validation via Maestro flows. Covers flow authoring, simulator management, GraalJS scripting constraints, permission handling, flakiness debugging, and CI patterns.

## What It Does

- **Maestro flows** for UI validation — YAML-based tap/assert/wait sequences
- **Simulator management** — boot, erase, screenshot, biometrics via simctl
- **CI integration** — GitHub Actions patterns, JUnit output, retry strategies

## Installation

```bash
claude --plugin-dir /path/to/myClaudeSkills/plugins/maestro-mobile-validator
```

## Commands

| Command | Purpose |
|---------|---------|
| `/validate-mobile` | Run a Maestro flow against the booted iOS Simulator or Android emulator |

## Skill

**mobile-flows-maestro** — teaches Claude how to author, run, and debug Maestro flows for iOS and Android apps. Activates when asked about Maestro, mobile testing, iOS Simulator flows, or flaky mobile UI tests.

## Hook

No active runtime hooks. Reserved for future command-based hooks.

## References

- **maestro-gotchas** — flakiness debugging, CI patterns, simctl cookbook, Android specifics
