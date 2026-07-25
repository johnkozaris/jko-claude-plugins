---
description: Investigate an ESP32 crash, reset, hang, peripheral failure, or long-uptime fault from hardware and runtime evidence
argument-hint: "<symptom or target>"
---

# ESP32 Debug

Invoke the `esp32-expert` skill and investigate `$ARGUMENTS`.

Follow the skill's target detection and evidence guidance. Decode addresses
against the exact firmware build when symbols are available. Form a small set
of competing causes and test them against the evidence; use current source,
datasheets, and Espressif documentation when target or SDK behavior matters.

Report the leading cause, evidence, competing explanations, smallest safe fix,
and a reproduction or soak-test plan. State clearly whether the result was only
compiled, simulated, or exercised on target hardware.
