# ESP32 C++ Expert Plugin

Expert firmware review and guidance for ESP32 and variants (ESP32-S2, S3, C3, C6, H2, P4) using ESP-IDF or PlatformIO.

## What It Does

A senior firmware engineer skill that reviews code for correctness, safety, performance, FreeRTOS patterns, and hardware interaction quality. Every finding explains WHY it matters -- what crash it prevents, what field failure it avoids.

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install esp32-cpp@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/esp32-cpp
```

## Commands

| Command | Purpose |
|---|---|
| `/esp-harden` | Scan for anti-patterns (34 patterns, AP-01 through AP-34) and inspect/fix |
| `/esp-debug` | Crash analysis, backtrace decoding, and root cause investigation |
| `/esp-optimize` | Speed, memory, power, or binary size optimization |
| `/esp-teach` | One-time: scan project, discover hardware, persist context to CLAUDE.md |

## Skill

The `esp32-expert` skill activates automatically when working with ESP32 firmware. It provides:

- Framework detection (ESP-IDF vs PlatformIO) and variant identification
- 4-layer thinking model (Hardware / RTOS / Application / C++)
- 16 reference files covering every embedded domain
- 34 anti-patterns catalog with BAD/GOOD code examples
- AI slop detection for embedded code

## Hook

Runs pattern checks on C/C++ file edits to catch dangerous anti-patterns (missing IRAM_ATTR, blocking in ISR, delay() misuse, hardcoded credentials).

## References

16 reference files organized by domain:

freertos, concurrency, memory, cpp-guidelines, embedded-cpp, peripherals, build-system, networking, security, power, debugging, testing, design-patterns, lvgl, libraries, anti-patterns

## License

MIT
