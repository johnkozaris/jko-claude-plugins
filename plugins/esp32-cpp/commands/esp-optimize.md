---
description: Optimize ESP32 firmware for performance, memory, power, or binary size
allowed-tools:
  - Read
  - Glob
  - Grep
  - Edit
  - Write
  - Bash(idf.py:*)
  - Bash(pio:*)
argument-hint: "<target: speed|memory|power|size> [file-or-directory]"
---

# ESP32 Firmware Optimization

**First**: Use the esp32-expert skill. → *Consult [memory reference](references/memory.md) for heap/PSRAM/IRAM placement, [LVGL reference](references/lvgl.md) for display performance, and [power reference](references/power.md) for sleep mode design.*

Optimization without measurement is guesswork. Profile first, then target the bottleneck. On ESP32, the bottleneck is almost always memory bandwidth (PSRAM), flash cache misses, or DMA contention -- not CPU speed.

## Parse Target from $ARGUMENTS

$1 should be one of: `speed`, `memory`, `power`, `size`. Default to a general review if not specified.

## Speed Optimization

1. **IRAM placement**: Move critical functions to IRAM with `IRAM_ATTR`
2. **DMA usage**: Ensure SPI/I2S/ADC use DMA for large transfers
3. **Cache optimization**: Align data structures, avoid PSRAM for hot paths
4. **Task priority tuning**: Ensure real-time tasks have appropriate priority
5. **Compiler flags**: `-O2` or `-Ofast` for performance-critical components
6. **Algorithm selection**: Replace O(n^2) with O(n log n) where applicable
7. **Interrupt latency**: Minimize ISR processing, use deferred handlers
8. **Flash access**: Reduce flash reads (cache-friendly code layout)

## Memory Optimization

1. **Heap analysis**: Run `idf.py size-components` to identify large consumers
2. **Stack tuning**: Measure with `uxTaskGetStackHighWaterMark()`, reduce to actual + 25%
3. **Flash strings**: Use `PROGMEM` (Arduino) or ensure const strings stay in flash
4. **PSRAM offloading**: Move large buffers to PSRAM (`MALLOC_CAP_SPIRAM`)
5. **Component pruning**: Disable unused ESP-IDF components in menuconfig
6. **Static allocation**: Replace dynamic allocation with static for permanent objects
7. **Shared buffers**: Reuse buffers across non-concurrent operations

## Power Optimization

1. **Sleep mode selection**: Deep sleep for >1s idle, light sleep for 10ms-1s
2. **WiFi power save**: Enable modem sleep, use duty cycling
3. **Peripheral shutdown**: Disable unused peripherals before sleep
4. **CPU frequency**: Reduce to 80MHz when processing demand is low
5. **GPIO hold**: Use `gpio_hold_en()` to maintain state during sleep
6. **Batch operations**: Wake, do all work, sleep (minimize active time)
7. **ULP usage**: Offload monitoring to ULP coprocessor during deep sleep

## Binary Size Optimization

1. **Compiler flags**: `-Os` (optimize for size), `-flto` (link-time optimization)
2. **Disable unused features**: RTTI, exceptions, unused log levels
3. **Component pruning**: Remove unused ESP-IDF components
4. **Symbol stripping**: Strip debug symbols in release builds
5. **String deduplication**: Use common string constants
6. **Log level reduction**: Set production log level to WARN/ERROR

## Output Format

For each optimization found:
1. Category (speed/memory/power/size)
2. Current state and measured/estimated impact
3. Specific change to make
4. Trade-offs (e.g., "saves 8KB RAM but adds 2ms latency")
