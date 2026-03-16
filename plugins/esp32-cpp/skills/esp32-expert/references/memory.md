# ESP32 Memory Architecture & Management

## Memory Map Overview

### ESP32 (Original)
- **IRAM** (Instruction RAM): 128KB total, but ~32KB used by I-cache leaving ~96KB for user code (ISR handlers, time-critical functions). Actual usable IRAM depends on cache config.
- **DRAM** (Data RAM): 328KB total, ~160KB max static allocation, rest is heap
- **RTC FAST**: 8KB -- accessible by PRO CPU during deep sleep wake stub
- **RTC SLOW**: 8KB -- accessible by ULP coprocessor
- **Flash**: 4-16MB -- program code (XIP via cache), read-only data, filesystems
- **PSRAM** (optional): 2-8MB -- external SPI RAM, 3-10x slower than internal SRAM

### ESP32-S3
- **SRAM**: 512KB total (shared IRAM/DRAM, configurable split)
- **Flash**: 4-16MB (OPI flash support for faster access)
- **PSRAM**: Up to 32MB (OPI PSRAM -- significantly faster than ESP32's SPI PSRAM)

### ESP32-P4
- **Internal SRAM**: 768KB total (includes L2 cache + heap; cache is always active, not optional)
- **TCM RAM**: 8KB zero-wait-state tightly coupled memory
- **PSRAM**: Up to 32MB (Octal SPI, ~120MB/s bandwidth)
- **NO internal flash** -- external flash only

## Memory Attributes

```cpp
// Place function in IRAM (required for ISR handlers, flash-disabled code)
void IRAM_ATTR my_isr_handler(void* arg) { /* ... */ }

// Place data in DRAM (ensures it's not in flash, needed for DMA)
DRAM_ATTR static uint8_t dma_buffer[1024];

// Place data in RTC memory (survives deep sleep)
RTC_DATA_ATTR static int boot_count = 0;

// Place data in RTC FAST memory (accessible during wake stub)
RTC_FAST_ATTR static uint32_t wake_data;

// Place constant in DRAM instead of flash (for DMA or ISR access)
DRAM_ATTR static const char tag[] = "ISR";
```

**DO**: Mark ALL ISR handlers with `IRAM_ATTR` -- ISRs during flash operations cause crashes.
**DO**: Mark DMA buffers with `DRAM_ATTR` -- DMA cannot access flash.
**DO**: Use `RTC_DATA_ATTR` for data that must persist across deep sleep.
**DON'T**: Overuse `IRAM_ATTR` -- IRAM is scarce (128KB shared with cache).
**DON'T**: Put large constant arrays in IRAM -- use DRAM or flash.

## Heap Management

### ESP-IDF Multi-Heap Allocator

ESP-IDF manages multiple heaps with different capabilities:

```cpp
// Standard allocation (defaults to DRAM)
void* p = malloc(1024);

// Capability-based allocation
void* dma_buf = heap_caps_malloc(1024, MALLOC_CAP_DMA);
void* iram_buf = heap_caps_malloc(256, MALLOC_CAP_EXEC);  // Executable memory
void* psram_buf = heap_caps_malloc(65536, MALLOC_CAP_SPIRAM);  // PSRAM
void* fast_buf = heap_caps_malloc(512, MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT);

// Aligned allocation (needed for some peripherals)
void* aligned = heap_caps_aligned_alloc(16, 1024, MALLOC_CAP_DMA);
```

### Capability Flags

| Flag | Meaning | When to Use |
|---|---|---|
| `MALLOC_CAP_DMA` | DMA-accessible memory (DRAM) | SPI/I2S DMA buffers |
| `MALLOC_CAP_SPIRAM` | External PSRAM | Large buffers, framebuffers |
| `MALLOC_CAP_INTERNAL` | Internal memory only | Performance-critical data |
| `MALLOC_CAP_EXEC` | Executable (IRAM) | Dynamic code loading (rare) |
| `MALLOC_CAP_8BIT` | Byte-accessible | Most data |
| `MALLOC_CAP_32BIT` | Word-aligned access only | Some IRAM regions |
| `MALLOC_CAP_RTCRAM` | RTC memory | Deep sleep persistence |

### PSRAM (SPI RAM) Gotchas

**DO**: Enable PSRAM in menuconfig: `Component config -> ESP PSRAM`.
**DO**: Use `heap_caps_malloc(size, MALLOC_CAP_SPIRAM)` for large allocations (>4KB).
**DO**: Consider enabling "Make RAM allocatable using malloc()" for transparent PSRAM use.
**DO**: Use `CONFIG_SPIRAM_USE_MALLOC` with `CONFIG_SPIRAM_MALLOC_ALWAYSINTERNAL` threshold.

**DON'T**: Use PSRAM for DMA buffers -- DMA cannot access external memory (on most variants).
**DON'T**: Use PSRAM for time-critical data -- access is 3-10x slower than internal SRAM.
**DON'T**: Assume PSRAM is byte-accessible on ESP32 (original) -- it's 32-bit aligned through cache.
**DON'T**: Forget that PSRAM access goes through cache -- cache misses add latency.

### ESP32-S3 OPI PSRAM
- Octal SPI: ~120MB/s bandwidth vs ~18MB/s on standard SPI
- Still slower than internal SRAM for random access
- Good for large sequential data (audio buffers, framebuffers, ML models)

## Stack Management

### Task Stack Location
By default, task stacks are allocated from internal DRAM. For tasks that don't need fast stack access:

```cpp
// Allocate task stack in PSRAM (saves internal RAM)
// Requires CONFIG_SPIRAM_ALLOW_STACK_EXTERNAL_MEMORY=y
StaticTask_t task_buffer;
StackType_t* task_stack = (StackType_t*)heap_caps_malloc(8192, MALLOC_CAP_SPIRAM);
xTaskCreateStatic(my_task, "big_task", 8192, nullptr, 2, task_stack, &task_buffer);
```

**DO**: Put large-stack tasks (HTTP/TLS) on PSRAM if available.
**DON'T**: Put ISR-heavy or time-critical task stacks on PSRAM.

### Avoiding Stack Overflow

```cpp
// Check stack usage in development
void my_task(void* param) {
    while (true) {
        // ... work ...
        UBaseType_t watermark = uxTaskGetStackHighWaterMark(nullptr);
        ESP_LOGI("TASK", "Stack free: %u bytes", watermark * sizeof(StackType_t));
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}
```

**Common stack hogs:**
- Local arrays: `char buffer[2048]` consumes 2KB of stack per call
- `snprintf`/`printf` family: uses ~1KB of stack internally
- TLS/SSL operations: can use 4-8KB of stack
- JSON parsing (cJSON): proportional to document depth
- Recursive function calls: each frame adds to stack usage

**FIX**: Use `static` buffers, heap allocation, or pass buffers as parameters.

## Fragmentation Prevention

Long-running embedded systems (months/years) MUST avoid heap fragmentation:

1. **Allocate at startup, never free**: Permanent resources (task stacks, queues, buffers)
2. **Fixed-size pools**: For objects allocated/freed repeatedly
3. **Arena/bump allocators**: For request-scoped allocations
4. **Monitor free heap**: `esp_get_free_heap_size()` and `esp_get_minimum_free_heap_size()`

```cpp
// Monitor heap health periodically
void health_check_task(void* param) {
    while (true) {
        size_t free = esp_get_free_heap_size();
        size_t min_free = esp_get_minimum_free_heap_size();
        size_t largest = heap_caps_get_largest_free_block(MALLOC_CAP_8BIT);
        ESP_LOGI("HEAP", "Free: %u, Min: %u, Largest block: %u", free, min_free, largest);

        if (largest < 4096) {
            ESP_LOGW("HEAP", "Fragmentation detected! Largest block < 4KB");
        }
        vTaskDelay(pdMS_TO_TICKS(60000));
    }
}
```

## Memory Debugging Tools

### Heap Tracing
```cpp
#include "esp_heap_trace.h"

#define NUM_RECORDS 100
static heap_trace_record_t trace_records[NUM_RECORDS];

// Start tracing
heap_trace_init_standalone(trace_records, NUM_RECORDS);
heap_trace_start(HEAP_TRACE_LEAKS);

// ... code under test ...

heap_trace_stop();
heap_trace_dump();  // Prints allocations without matching free
```

### Heap Corruption Detection
Enable in menuconfig: `Component config -> Heap memory debugging`:
- **Basic** (default): Low overhead, detects gross corruption
- **Light impact**: Adds head/tail canary bytes (`0xABBA1234` / `0xBAAD5678`) -- detects buffer overrun/underrun
- **Comprehensive**: Fills freed memory with `0xFE` (detect use-after-free), new allocations with `0xCE` (detect uninitialized reads). Significant performance overhead -- development only.

### Additional Memory Attributes

```cpp
// __NOINIT_ATTR -- survives software reset (NOT deep sleep or power-on)
__NOINIT_ATTR static uint32_t reboot_reason;
// Use for crash diagnostics: write before reset, read after

// DMA_ATTR -- word-aligned in DRAM (shorthand for DMA-safe static buffers)
DMA_ATTR static uint8_t spi_buffer[1024];
```

### PSRAM Cache Bug (ESP32 Rev 0/1 ONLY)

A silicon bug in early ESP32 causes data corruption when PSRAM access coincides with certain interrupt patterns. ESP-IDF enables the workaround automatically (`-mfix-esp32-psram-cache-issue`). **PlatformIO users must add it manually** in `build_flags`. ESP32 rev 3.0+ is NOT affected. Check revision: `espefuse.py chip_id`.

Symptom: Random bytes in PSRAM silently flip to zero. Non-reproducible, appears as intermittent data corruption.

### ETL (Embedded Template Library) for No-Heap Containers

When `std::vector`/`std::string` heap allocation is unacceptable, use [ETL](https://www.etlcpp.com/):

```cpp
#include <etl/vector.h>
#include <etl/string.h>

etl::vector<int, 20> readings;     // Max 20 elements, zero heap
etl::string<64> device_name;       // Max 64 chars, zero heap

readings.push_back(42);
if (readings.full()) { /* handle */ }
```

Key ETL containers: `etl::vector<T,N>`, `etl::string<N>`, `etl::map<K,V,N>`, `etl::queue<T,N>`, `etl::circular_buffer<T,N>`, `etl::pool<T,N>`. All have compile-time fixed capacity, STL-compatible API, no heap, no RTTI, no exceptions required. MIT licensed, compatible with ESP-IDF.

### Memory Pool Pattern (Zero Fragmentation)

```cpp
template <typename T, size_t N>
class Pool {
    std::array<T, N> storage_{};
    std::array<T*, N> free_list_;
    size_t free_count_ = N;
public:
    Pool() { for (size_t i = 0; i < N; ++i) free_list_[i] = &storage_[i]; }
    T* alloc() { return free_count_ > 0 ? free_list_[--free_count_] : nullptr; }
    void free(T* p) { if (free_count_ < N) free_list_[free_count_++] = p; }
};
```

Use for: message objects, sensor readings, event structs -- anything allocated/freed repeatedly at a known peak count. O(1) alloc/free, zero fragmentation, zero external metadata.

### Memory Mapping Tips

| Data Type | Best Location | Why |
|---|---|---|
| ISR handlers | IRAM (IRAM_ATTR) | Must execute during flash ops |
| DMA buffers | Internal DRAM (MALLOC_CAP_DMA) | DMA controller requirement |
| WiFi/BLE buffers | Internal DRAM | Network stack requirement |
| Large data arrays | PSRAM | Saves internal memory |
| Lookup tables | Flash (const) | Read-only, saves RAM |
| FreeRTOS objects | Internal DRAM | Timing sensitive |
| Audio/video buffers | PSRAM | Large, sequential access |
| Deep sleep data | RTC memory (RTC_DATA_ATTR) | Survives power-off |
| Crash diagnostics | `__NOINIT_ATTR` | Survives software reset |
| Float arrays | DRAM only (`MALLOC_CAP_8BIT`) | FPU cannot access IRAM (AP-27) |
