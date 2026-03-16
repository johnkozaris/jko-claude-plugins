# AI Code Slop in ESP32 Firmware

LLMs generate embedded code that compiles but fails in the field. AI models lack hardware awareness -- they don't understand flash cache, DMA constraints, interrupt latency, or RTOS scheduling. If the code looks like it came from a generic C++ tutorial pasted into an ESP32 project, it probably has these tells.

## Framework Confusion (the #1 tell)

- `delay()` instead of `vTaskDelay(pdMS_TO_TICKS())` -- Arduino habit in ESP-IDF code
- `Serial.println()` instead of `ESP_LOGI()` -- wrong logging API for the framework
- `analogRead()` instead of `adc1_get_raw()` / `adc_oneshot_read()` -- Arduino API in IDF
- `WiFi.begin()` in an ESP-IDF project -- completely wrong API layer
- Mixing `#include <Arduino.h>` with ESP-IDF component code

## Missing Hardware Awareness

- ISR handlers without `IRAM_ATTR` -- AI doesn't know about flash cache
- No `volatile` on ISR-shared variables -- AI treats it like regular multithreading
- `char buffer[4096]` on the stack inside a FreeRTOS task -- AI doesn't think about stack limits
- DMA buffers allocated from PSRAM or stack -- AI doesn't understand DMA constraints
- `printf` / `std::cout` in production firmware -- AI adds debugging everywhere

## Concurrency Naivety

- `portMAX_DELAY` on every mutex/queue operation -- AI defaults to "wait forever"
- `xSemaphoreCreateBinary()` to protect shared resources (should be `Mutex`)
- No mutex on shared I2C/SPI bus -- AI doesn't consider multi-task access
- `volatile bool flag` for task synchronization -- AI confuses volatile with thread-safe
- `xSemaphoreGive()` called from ISR instead of `xSemaphoreGiveFromISR()`

## Hallucinated APIs (AI invents functions that don't exist)

- Mixing ESP-IDF v4 APIs with v5 code (`i2c_master_write_slave()` doesn't exist in v5)
- Calling functions from wrong component (`esp_wifi_connect()` without prior `esp_wifi_init()`)
- Invented configuration structs with wrong field names
- Using deprecated APIs (`driver/adc.h` instead of `esp_adc/adc_oneshot.h`)
- Hallucinated component names in `idf_component.yml` (packages that don't exist on the ESP Component Registry)
- Wrong `#include` paths that moved between IDF versions (`esp_event_loop_init()` is gone in v5)

## Outdated Training Data Patterns (AI learned from old tutorials)

- Old I2C driver API (`i2c_param_config` + `i2c_driver_install`) instead of v5's `i2c_new_master_bus`
- Old timer API (`timer_group_init`) instead of v5's `gptimer_new_timer`
- Old ADC API (`adc1_config_width` + `adc1_get_raw`) instead of v5's `adc_oneshot` driver
- `esp_event_loop_init()` (removed) instead of `esp_event_loop_create_default()`
- `register_component()` in CMake instead of `idf_component_register()`
- Code patterns from popular but outdated Arduino tutorials with ESP-IDF v3-era conventions

## Generic C++ Patterns That Don't Belong in Embedded

- `std::string` concatenation in a loop (heap fragmentation)
- `std::function<>` for callbacks (hidden heap allocation)
- `std::map` / `std::unordered_map` for small lookups (use sorted array)
- `try/catch` blocks everywhere (unnecessary overhead if exceptions are disabled)
- `#include <iostream>` (adds ~200KB to binary)
- `new` without corresponding `delete` or RAII wrapper

## Configuration Ignorance

- Hardcoded `GPIO_NUM_2` for LED on every board variant
- No `sdkconfig.defaults` -- everything in `sdkconfig` (not portable)
- `CONFIG_FREERTOS_HZ` assumed to be 1000 without checking
- Pin numbers that conflict with strapping pins (GPIO0, GPIO12)
- WiFi code on ESP32-H2 or ESP32-P4 (no WiFi radio on these chips)

## Happy-Path-Only Code (AI skips the hard parts)

- Init functions with no cleanup-on-failure path (resource leak on partial init)
- `ESP_ERROR_CHECK()` wrapping runtime calls that should handle errors gracefully
- No reconnection logic after WiFi disconnect -- just connects once
- No watchdog feeding -- AI doesn't know the device needs field recovery
- Missing `extern "C"` on `app_main()` in `.cpp` files -- linker error
- C-style designated initializers that don't compile under C++23 (out-of-order, nested)
- `while(!connected) { delay(100); }` blocking poll instead of event-driven pattern
- Generates the sensor read but forgets calibration (ADC without `adc_cali_raw_to_voltage`)

## Structural Tells (code that works on desktop but not on MCU)

- One massive `app_main()` function doing everything instead of task decomposition
- Global `std::vector` growing unbounded from sensor readings (OOM in hours)
- `xTaskCreate` for every operation instead of persistent tasks with queues
- Comments that describe what the code SHOULD do, not what it DOES (hallucinated intent)
- Copy-pasted boilerplate from different projects with conflicting conventions
