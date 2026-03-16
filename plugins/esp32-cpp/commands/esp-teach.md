---
description: One-time project setup -- discover hardware, find datasheets, persist context to CLAUDE.md
allowed-tools:
  - Read
  - Glob
  - Grep
  - WebSearch
  - WebFetch
  - Write
  - Edit
  - Bash(idf.py:*)
  - Bash(pio:*)
  - AskUserQuestion
argument-hint: "[optional: specific hardware question]"
---

# ESP32 Project Teach

One-time project onboarding that establishes persistent hardware context for all future sessions.

**First**: Use the esp32-expert skill for domain knowledge. → *Consult [peripherals reference](references/peripherals.md) for datasheet lookup protocol and [libraries reference](references/libraries.md) for component identification.*

Explore the codebase, discover hardware, search online for datasheets, ask only what you couldn't infer, and persist everything to CLAUDE.md. Future sessions start with full hardware knowledge -- no re-discovery needed.

## Step 1: Explore the Project

Before asking any questions, scan the project thoroughly to discover what you can:

### Build System & Target
- Check for `platformio.ini` vs `CMakeLists.txt` with IDF includes
- Read `sdkconfig.defaults` or `sdkconfig` for target chip, flash size, PSRAM, CPU freq
- Read `platformio.ini` for board, framework, build flags
- Check `idf_component.yml` / `managed_components/` for dependencies
- Identify: ESP-IDF version, C++ standard, LVGL version (if present)

### Hardware Discovery
- Read `CLAUDE.md`, `README.md`, any `docs/` files for hardware description
- Search for pin definitions: `GPIO_NUM_`, `#define.*PIN`, board config files
- Search for I2C addresses: `0x` patterns in driver init code
- Search for SPI device configs: `spi_device_interface_config_t`
- Search for display driver: `esp_lcd`, `TFT_eSPI`, `LovyanGFX`, LVGL display init
- Search for touch driver: `esp_lcd_touch`, touch I2C addresses
- Search for audio: `i2s_`, `es8311`, `es7210`, codec references
- Search for sensors: BME280, BMP280, DHT, IMU, ADC usage patterns
- Identify BSP component (if any) and what hardware it abstracts

### Software Patterns
- How is LVGL integrated (if present)? Task core, mutex pattern, flush callback
- FreeRTOS task structure: what tasks exist, priorities, core pinning
- Communication: WiFi, BLE, MQTT, HTTP, WebSocket, ESP-NOW
- Storage: NVS, SPIFFS, LittleFS, SD card
- Error handling pattern: ESP_ERROR_CHECK vs manual checks
- Logging: tag conventions, log levels

Note everything you learned and what remains unclear.

## Step 2: Search for Datasheets

For every hardware component discovered (MCU, display, touch controller, sensors, codecs, communication modules):

1. **Search online**: `"<part-number> datasheet"` for each IC/module found
2. **Extract key specs**: voltage, interface, I2C address, SPI mode, timing constraints
3. **Search for existing drivers**: `"<part-number> ESP-IDF component"` or `"<part-number> ESP32 library"`
4. **Note any errata or known issues** from the datasheets

Build a hardware reference table with what you found.

## Step 3: Ask Targeted Questions

Ask the user ONLY about things you could NOT infer from the codebase. Use the AskUserQuestion tool. Example questions (skip any already answered by code):

### Hardware
- What board/module is this? (if not documented)
- Are there peripherals connected that aren't in the code yet?
- Any specific power constraints? (battery, solar, PoE, USB)
- Any environmental constraints? (temperature range, outdoor, vibration)

### Project Purpose
- What does this device do in one sentence?
- What's the deployment scenario? (single prototype, 10 units, mass production)
- What's the expected uptime? (always-on, periodic wake, interactive)

### Constraints
- Any strict timing requirements? (real-time control, sampling rates)
- Memory constraints you've hit or worried about?
- Any known issues or bugs you're fighting?

Skip questions where the answer is already clear from the codebase exploration.

## Step 4: Write Hardware Context to CLAUDE.md

Synthesize findings into a `## Hardware Context` section. Write or update the project's `CLAUDE.md`:

```markdown
## Hardware Context

### Board
[Board name, MCU variant, clock speed, memory (flash + PSRAM)]

### Peripherals
| Component | Part Number | Interface | Address/Config | Datasheet |
|---|---|---|---|---|
| Display | [e.g., ILI9341] | SPI (40MHz, mode 0) | CS=GPIO5 | [link] |
| Touch | [e.g., GT911] | I2C (0x5D) | INT=GPIO4 | [link] |
| Sensor | [e.g., BME280] | I2C (0x76) | | [link] |

### Pin Map
[Key GPIO assignments discovered, organized by function]

### Build System
- Framework: [ESP-IDF v5.x / PlatformIO + Arduino / PlatformIO + ESP-IDF]
- Target: [esp32 / esp32s3 / esp32c3 / esp32p4]
- Key sdkconfig: [flash size, PSRAM, CPU freq, optimization level]

### Key Patterns
- LVGL: [version, integration layer, task core, mutex API]
- FreeRTOS: [task structure summary, priority scheme]
- Networking: [WiFi/BLE/MQTT setup, reconnection strategy]

### Datasheet Quick Reference
[Links to all datasheets found, organized by component]

### Known Constraints
[Power budget, memory limits, timing requirements, known hardware issues]

### ESP32 Skill References
For firmware review and best practices, this project uses the esp32-expert skill:
- Anti-patterns: `references/anti-patterns.md` (34 patterns to avoid)
- FreeRTOS rules: `references/freertos.md`
- LVGL performance: `references/lvgl.md` (if LVGL is used)
- Memory management: `references/memory.md`
- Build system: `references/build-system.md`
```

Write this section to the project's `CLAUDE.md`. If the file exists, append or update the Hardware Context section. Do NOT overwrite existing content.

## Step 5: Confirm

Summarize what was discovered:
- Hardware components found (with datasheet links)
- Build system and target identified
- Key patterns documented
- Any gaps or items to investigate further

Confirm that CLAUDE.md has been updated and future sessions will have full hardware context.

Remember: A well-documented hardware context saves hours of re-discovery across every future session. Datasheets are the ground truth -- signal timing, voltage levels, and register maps are not optional reading. Document once, benefit forever.
