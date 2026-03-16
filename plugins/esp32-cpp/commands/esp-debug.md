---
description: Help debug ESP32 crashes, hangs, and peripheral issues
allowed-tools:
  - Read
  - Glob
  - Grep
  - WebSearch
  - Bash(idf.py:*)
  - Bash(pio:*)
  - Bash(xtensa-*:*)
  - Bash(riscv32-*:*)
argument-hint: "<crash-log-or-symptom>"
---

# ESP32 Debug Assistant

**First**: Use the esp32-expert skill. → *Consult [debugging reference](references/debugging.md) for crash decoding, GDB commands, and the code inspection checklist.*

Think like a field support engineer with a crashed device and only a serial log to work with. Find the root cause, not just the symptom.

## Process

1. **Identify the problem type** from $ARGUMENTS:
   - **Crash log / Guru Meditation**: Decode the backtrace and register dump
   - **Watchdog timeout**: Identify which task is blocked and why
   - **Hang / freeze**: Check for deadlocks, infinite loops, blocked tasks
   - **Peripheral not working**: Check wiring, configuration, timing
   - **Memory issues**: Check heap, fragmentation, stack overflow
   - **WiFi/BLE issues**: Check events, reconnection, signal strength

2. **For crash logs**:
   - Identify the exception type (LoadProhibited, StoreProhibited, etc.)
   - Decode addresses to file:line using addr2line
   - Check for null pointer patterns (0x00000000)
   - Check if addresses are in IRAM, DRAM, flash, or peripheral space
   - Look at the backtrace to identify the call chain

3. **For hangs**:
   - Suggest enabling task runtime stats (`configGENERATE_RUN_TIME_STATS`)
   - Check for potential deadlock scenarios (multiple mutexes)
   - Check for tasks waiting on empty queues or unavailable semaphores
   - Check Core 0 vs Core 1 affinity issues

4. **For peripheral issues**:
   - Verify pin assignments match the target variant
   - Check if pins conflict with strapping pins or flash SPI
   - Verify voltage levels and pullups
   - Suggest bus scanning (I2C) or logic analyzer capture (SPI/UART)
   - Search for the device datasheet for timing requirements

5. **Provide the root cause** and a fix, not just a workaround.

## Output Format

```
## Diagnosis

### Symptom
[What the user reported]

### Root Cause
[Identified cause with evidence]

### Fix
[Concrete code changes or configuration changes]

### Prevention
[How to avoid this class of bug in the future]
```
