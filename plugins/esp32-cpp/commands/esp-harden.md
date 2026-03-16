---
description: Harden and inspect ESP32 firmware for field failures, crashes, memory, and security
allowed-tools:
  - Read
  - Glob
  - Grep
  - Edit
  - Write
  - Bash(idf.py:*)
  - Bash(pio:*)
argument-hint: "[file-or-directory]"
---

# ESP32 Firmware Hardening & Inspection

**First**: Use the esp32-expert skill for all guidelines and anti-patterns. Do NOT proceed without consulting the anti-patterns catalog and concurrency reference.

Think like an embedded reliability engineer conducting a field-readiness audit. Every finding must name the concrete failure mode it prevents.

## Process

1. **Detect framework** -- check for `platformio.ini` vs `CMakeLists.txt` with IDF includes
2. **Detect target variant** -- check sdkconfig or platformio.ini for target chip
3. **Anti-Pattern Scan** (from `references/anti-patterns.md`):
   - ISR functions missing `IRAM_ATTR` (AP-02)
   - `printf`/`ESP_LOG`/`malloc`/`new` in ISR context (AP-01)
   - `delay()` instead of `vTaskDelay` (AP-11)
   - Large local arrays in task functions >512 bytes (AP-03)
   - Shared I2C/SPI bus access without mutex (AP-05)
   - Unchecked `esp_err_t` return values (AP-06)
   - Hardcoded credentials (AP-16)
   - Missing TLS cert verification (AP-17)
   - `volatile` for thread synchronization (AP-09)
   - `xSemaphoreCreateBinary` for resource protection instead of Mutex (AP-14)
   - `malloc`/`new` in loops without free (AP-07)
   - DMA buffers not using `MALLOC_CAP_DMA` (AP-04)
   - Global constructors with side effects (AP-20)
   - `std::function` callbacks that may heap-allocate (AP-19)
   - No reconnection logic in WiFi/MQTT handlers (AP-08)

4. **Memory Safety Inspection**:
   - Task stack sizes vs typical requirements (check `references/freertos.md` table)
   - Heap usage patterns and fragmentation risk (AP-15)
   - PSRAM vs internal RAM placement correctness
   - DMA buffer alignment and capability requirements
   - Long-running allocation patterns (allocate+free cycles)

5. **Build Configuration Check**:
   - `sdkconfig.defaults` exists and committed; `sdkconfig` gitignored
   - Partition table appropriate (OTA, NVS, storage)
   - Compiler warnings enabled (`-Wall -Wextra`)
   - Debug vs release config separated
   - Correct target variant configured

6. **FreeRTOS Design Review**:
   - Task priorities make sense (no priority inversion setups)
   - Synchronization primitive selection correct (mutex vs semaphore vs queue)
   - Deadlock risk (multiple mutex acquisition)
   - Watchdog configuration and feeding patterns
   - Dual-core task pinning (if applicable)

7. **Security Check**:
   - No credentials in source code
   - TLS certificates verified
   - Secure boot planned for production
   - NVS encryption for sensitive data
   - OTA signed and verified

## Output Format

Group findings by severity, then by category.

For each finding:
1. Severity: **critical** / **important** / **warning**
2. Anti-pattern ID (e.g., AP-03) when applicable
3. File:line reference
4. WHY it's dangerous (concrete failure mode: crash, data loss, security breach)
5. Concrete fix with code snippet

End with:
- **Reliability Score**: 1-5 stars for production readiness
- **Top 5 Actions**: Most impactful fixes, prioritized
- **Memory Health**: Current state and recommendations

If $ARGUMENTS is provided, focus on those files/directories. Otherwise scan the full project.
