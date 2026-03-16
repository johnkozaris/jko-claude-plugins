# Debugging & Code Inspection for ESP32

## Logging System

### ESP-IDF Log Levels
```cpp
ESP_LOGE("TAG", "Error: %s", esp_err_to_name(err));    // Always shown
ESP_LOGW("TAG", "Warning: low memory %d", free_heap);   // Important warnings
ESP_LOGI("TAG", "Info: connected to %s", ssid);          // Normal operation
ESP_LOGD("TAG", "Debug: register value 0x%02x", reg);    // Development
ESP_LOGV("TAG", "Verbose: entering function");            // Detailed trace
```

### Log Configuration

Per-component log level in menuconfig:
```
Component config -> Log output -> Default log verbosity -> Info
```

At runtime:
```cpp
esp_log_level_set("WIFI", ESP_LOG_WARN);    // Only warnings and errors
esp_log_level_set("SENSOR", ESP_LOG_DEBUG); // Detailed for debugging
esp_log_level_set("*", ESP_LOG_INFO);       // Default for all
```

### Logging Best Practices

**DO**: Use meaningful tags matching component names.
**DO**: Include context in log messages (values, states, identifiers).
**DO**: Use `ESP_LOG_BUFFER_HEX_LEVEL()` for protocol debugging.
**DO**: Set production log level to WARN or ERROR to save flash and CPU.
**DO**: Use `CONFIG_LOG_TIMESTAMP_SOURCE_SYSTEM` for accurate timestamps.

**DON'T**: Log in ISRs -- use `ESP_DRAM_LOGE()` only for critical ISR debugging.
**DON'T**: Leave verbose logging in production -- wastes flash, UART bandwidth, CPU.
**DON'T**: Use `printf` -- always use `ESP_LOGx` macros (they can be compiled out).
**DON'T**: Log secrets, passwords, or tokens at any level.
**DON'T**: Log in tight loops -- serial output blocks and slows execution.

## Crash Analysis

### Guru Meditation Error (Panic Handler)
```
Guru Meditation Error: Core  0 panic'ed (LoadProhibited). Exception was unhandled.
Core 0 register dump:
PC      : 0x400d1234  PS      : 0x00060030  A0      : 0x800d5678
...
Backtrace: 0x400d1234:0x3ffb1234 0x400d5678:0x3ffb5678 0x400d9abc:0x3ffb9abc
```

### Interpreting Crash Dumps

1. **Exception type tells you what happened**:
   - `LoadProhibited` / `StoreProhibited`: Null pointer or invalid memory access
   - `InstrFetchProhibited`: Jumping to invalid code address (corrupted function pointer)
   - `IllegalInstruction`: Corrupted code or wrong architecture binary
   - `IntegerDivideByZero`: Division by zero
   - `Unhandled debug exception`: Watchpoint hit

2. **Decode the backtrace**:
   ```bash
   # ESP-IDF
   idf.py monitor  # Auto-decodes addresses to file:line

   # Manual decode
   xtensa-esp32-elf-addr2line -e build/my_project.elf 0x400d1234

   # PlatformIO with filter
   monitor_filters = esp32_exception_decoder
   ```

3. **Common crash causes**:
   - `0x00000000` in backtrace: Null function pointer call
   - `0x3ff.....` address: Peripheral register access issue
   - `0x400d....` address: Code in flash -- normal
   - `0x3ffb....` address: Stack in DRAM -- check stack

### Core Dump

Enable core dump to flash or UART for post-mortem analysis:

```bash
# In menuconfig:
# Component config -> Core dump -> Data destination -> Flash
# Component config -> Core dump -> Core dump data integrity check -> CRC32
```

```bash
# Retrieve and analyze core dump
idf.py coredump-info
idf.py coredump-debug  # Opens GDB with core dump
```

## JTAG Debugging

### Hardware Setup
- **ESP-PROG**: Espressif's official debug probe
- **ESP32-S3/C3/C6**: Built-in USB JTAG (no external probe needed!)
- **Generic FTDI FT2232H**: Works with OpenOCD

### OpenOCD Configuration
```bash
# ESP32 with ESP-PROG
openocd -f board/esp32-wrover-kit-3.3v.cfg

# ESP32-S3 USB JTAG (built-in)
openocd -f board/esp32s3-builtin.cfg

# ESP32-C3 USB JTAG (built-in)
openocd -f board/esp32c3-builtin.cfg
```

### GDB Commands for ESP32

```gdb
# Connect to OpenOCD
target remote :3333

# FreeRTOS thread awareness
info threads              # List all FreeRTOS tasks
thread 3                  # Switch to task 3

# Memory examination
x/16xw 0x3FF44000        # Read GPIO registers
x/4xb &my_buffer         # Read buffer bytes

# Breakpoints
break main.cpp:42         # Line breakpoint
watch my_variable         # Hardware watchpoint (limited!)
break my_function if x>10 # Conditional breakpoint

# Stack inspection
bt                        # Backtrace current task
bt full                   # With local variables
info locals               # Local variables

# ESP32 specific
mon reset halt            # Reset and halt
mon esp32 smpbreak BreakIn BreakIn  # SMP breakpoint mode
```

**DO**: Use USB JTAG on S3/C3/C6 -- zero extra hardware needed.
**DO**: Set hardware watchpoints for memory corruption (limited to 2-4 on ESP32).
**DO**: Use `info threads` to see all FreeRTOS task states during debugging.

**DON'T**: Set too many software breakpoints in flash -- uses flash breakpoints (limited).
**DON'T**: Forget to halt all cores when debugging SMP code.

## Runtime Monitoring

### System Health Dashboard
```cpp
void system_health_task(void* param) {
    while (true) {
        // Heap
        ESP_LOGI("HEALTH", "Free heap: %u, min: %u, largest: %u",
            esp_get_free_heap_size(),
            esp_get_minimum_free_heap_size(),
            heap_caps_get_largest_free_block(MALLOC_CAP_8BIT));

        // Task stats
        char task_list[1024];
        vTaskList(task_list);  // Requires configUSE_TRACE_FACILITY
        ESP_LOGI("HEALTH", "Tasks:\n%s", task_list);

        // Runtime stats
        char runtime_stats[1024];
        vTaskGetRunTimeStats(runtime_stats);  // Requires configGENERATE_RUN_TIME_STATS
        ESP_LOGI("HEALTH", "Runtime:\n%s", runtime_stats);

        // WiFi
        wifi_ap_record_t ap_info;
        if (esp_wifi_sta_get_ap_info(&ap_info) == ESP_OK) {
            ESP_LOGI("HEALTH", "WiFi RSSI: %d", ap_info.rssi);
        }

        vTaskDelay(pdMS_TO_TICKS(30000));
    }
}
```

### Stack High-Water Mark Monitoring
```cpp
// Check all task stacks periodically
void check_task_stacks() {
    TaskStatus_t tasks[20];
    uint32_t total_runtime;
    UBaseType_t count = uxTaskGetSystemState(tasks, 20, &total_runtime);
    for (UBaseType_t i = 0; i < count; i++) {
        if (tasks[i].usStackHighWaterMark < 128) {
            ESP_LOGW("STACK", "Task '%s' low stack: %u words",
                tasks[i].pcTaskName, tasks[i].usStackHighWaterMark);
        }
    }
}
```

## Code Inspection Checklist

When reviewing ESP32 C++ code, check:

### Critical (Will Crash)
- [ ] All ISR handlers have `IRAM_ATTR`
- [ ] No `printf`/`ESP_LOG`/`malloc` in ISRs
- [ ] DMA buffers in internal DRAM (not PSRAM, not flash)
- [ ] Stack sizes measured, not guessed
- [ ] No large local arrays in FreeRTOS tasks
- [ ] Shared buses (I2C/SPI) protected by mutex

### Important (Will Fail Eventually)
- [ ] All `esp_err_t` return values checked
- [ ] Reconnection logic for WiFi/BLE/MQTT
- [ ] Watchdog configured and fed correctly
- [ ] Heap usage monitored (fragmentation detection)
- [ ] No memory leaks in repeated operations
- [ ] Proper cleanup on error paths (RAII)

### Build & Config
- [ ] `sdkconfig.defaults` committed, `sdkconfig` gitignored
- [ ] Partition table appropriate for project (OTA, NVS, storage)
- [ ] Compiler warnings enabled (`-Wall -Wextra`)
- [ ] Correct target variant configured
- [ ] Debug/release configurations separated

### Security
- [ ] No hardcoded credentials
- [ ] TLS certificates verified
- [ ] Secure boot planned for production
- [ ] NVS encryption for secrets
- [ ] UART download mode disabled in production
