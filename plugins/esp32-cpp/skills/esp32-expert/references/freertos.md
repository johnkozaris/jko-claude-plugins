# FreeRTOS Patterns & Rules for ESP32

## ESP-IDF FreeRTOS vs Vanilla FreeRTOS

ESP-IDF uses a modified FreeRTOS (based on v10.5.1) with SMP support on dual-core variants (ESP32, ESP32-S3, ESP32-P4). Key differences from vanilla FreeRTOS:

- `xTaskCreatePinnedToCore()` -- pin tasks to Core 0 (protocol CPU) or Core 1 (app CPU), or `tskNO_AFFINITY`
- Tasks can run on either core unless pinned
- WiFi/BLE stack runs on Core 0 -- avoid blocking Core 0 with long computations
- ESP-IDF uses 1ms tick rate by default (`configTICK_RATE_HZ = 1000` unless changed)
- Task Watchdog Timer (TWDT) monitors idle tasks and optionally user tasks
- Ring buffers are an ESP-IDF extension (not in vanilla FreeRTOS)
- Critical sections use **spinlocks + mutexes** (not just interrupt disable like vanilla)
- One idle task **per core** (Idle0 and Idle1) -- both must get CPU time
- Tick interrupt only runs on Core 0 -- Core 1 has no independent tick
- All FreeRTOS memory allocations mapped to ESP-IDF's `heap_caps` API (always internal RAM)
- Tasks using hardware `float` are **auto-pinned** to their current core (FPU context not saved across cores)
- `FreeRTOSConfig.h` is private -- configure via `idf.py menuconfig`, not direct edits
- `configASSERT()` must be defined during all development -- catches most common FreeRTOS errors
- ESP-IDF v5+ offers optional Amazon SMP FreeRTOS (`CONFIG_FREERTOS_SMP`) alongside the default IDF port

## Task Design Rules

### Stack Sizing

| Task Type | Minimum Stack | Recommended | Notes |
|---|---|---|---|
| Simple GPIO/sensor task | 2048 bytes | 2560 bytes | |
| I2C/SPI communication | 2560 bytes | 3072 bytes | |
| WiFi event processing task | 3584 bytes | 4096 bytes | Not the system event task itself |
| HTTP/TLS client | 8192 bytes | 10240 bytes | TLS handshake is very stack-heavy |
| JSON parsing (cJSON) | 4096 bytes | 6144 bytes | Proportional to nesting depth |
| Task with `printf`/logging | 3072 bytes | 4096 bytes | printf uses ~1.8KB internally |
| LVGL UI task | 6144 bytes | 8192 bytes | Depends on widget complexity |

**These are starting points. Always measure actual usage with `uxTaskGetStackHighWaterMark()`.**

**DO**: Measure in development, then set to measured + 25% margin.
**DO**: Monitor high-water marks periodically in debug builds.
**DON'T**: Guess stack sizes. Measure them.
**DON'T**: Allocate large buffers on the stack -- use static buffers or heap allocation.
**DON'T**: Trust these numbers blindly -- they vary with ESP-IDF version, compiler optimization, and library usage.

### Priority Assignment

```
Priority 0:  Idle task (system)
Priority 1:  Background tasks (logging, LED, non-critical monitoring)
Priority 2:  Normal application tasks (sensor reading, data processing)
Priority 3:  Communication tasks (WiFi event handling, MQTT, HTTP)
Priority 4:  Real-time tasks (motor control, fast sensor sampling)
Priority 5:  Time-critical tasks (hard real-time requirements)
Priority 24: Timer service task (configTIMER_TASK_PRIORITY, system)
```

**DO**: Keep most tasks at the same priority (2-3) unless there's a genuine timing requirement.
**DON'T**: Make everything high priority -- it defeats the scheduler and causes starvation.
**DON'T**: Use priority 0 for user tasks (reserved for idle).

### Task Creation Patterns

**DO** -- Static allocation for permanent tasks:
```cpp
StaticTask_t task_buffer;
StackType_t task_stack[4096];
xTaskCreateStatic(sensor_task, "sensor", 4096, nullptr, 2, task_stack, &task_buffer);
```

**DO** -- Pass context via parameter:
```cpp
struct SensorContext {
    i2c_port_t port;
    uint8_t address;
    QueueHandle_t data_queue;
};

void sensor_task(void* param) {
    auto* ctx = static_cast<SensorContext*>(param);
    // Use ctx->port, ctx->address, ctx->data_queue
}
```

**DON'T** -- Use global variables instead of task parameters.
**DON'T** -- Delete tasks that own resources without cleanup.
**DON'T** -- Create/delete tasks dynamically in a loop (fragmentation).

### C++ Task Pattern

```cpp
class Task {
public:
    void start(const char* name, uint32_t stack, UBaseType_t priority, BaseType_t core = tskNO_AFFINITY) {
        xTaskCreatePinnedToCore(task_trampoline, name, stack, this, priority, &handle_, core);
    }
    virtual ~Task() { if (handle_) vTaskDelete(handle_); }
protected:
    virtual void run() = 0;
private:
    TaskHandle_t handle_ = nullptr;
    static void task_trampoline(void* param) {
        static_cast<Task*>(param)->run();
        vTaskDelete(nullptr);  // Self-delete if run() returns
    }
};
```

**CRITICAL**: Never call virtual functions from the constructor. The trampoline pattern avoids this.

## Synchronization Rules

### Mutex vs Semaphore vs Queue

| Need | Use | Why |
|---|---|---|
| Protect shared resource | `xSemaphoreCreateMutex()` | Has priority inheritance |
| Signal event from ISR | `xSemaphoreCreateBinary()` | ISR-safe give, no inheritance needed |
| Count resources | `xSemaphoreCreateCounting()` | Track available slots/items |
| Pass data between tasks | `xQueueCreate()` | Copies data, inherently thread-safe |
| Signal multiple conditions | `xEventGroupCreate()` | Bitmask of events, wait for any/all |
| Lightweight signal | `xTaskNotify()` | Fastest, no kernel object needed |
| One-to-one data stream | `xStreamBufferCreate()` | Byte stream, single reader/writer |

### Mutex Rules

**DO**: Always use mutexes (not binary semaphores) for resource protection -- they have priority inheritance.
**DO**: Keep critical sections as short as possible. Acquire late, release early.
**DO**: Use the same lock ordering everywhere to prevent deadlocks.
**DO**: Use `xSemaphoreTake(mutex, pdMS_TO_TICKS(1000))` with a timeout, not `portMAX_DELAY`, to detect deadlocks.

**DON'T**: Take a mutex in an ISR -- use `xSemaphoreGiveFromISR()` with binary semaphores instead.
**DON'T**: Hold a mutex across a `vTaskDelay()` or any blocking call.
**DON'T**: Nest mutexes without a strict ordering protocol.
**DON'T**: Use recursive mutexes unless absolutely necessary (design smell).

### Queue Rules

**DO**: Size queues based on burst rate, not average rate. If a sensor produces 10 readings before the consumer runs, queue size >= 10.
**DO**: Use `xQueueSendFromISR()` (not `xQueueSend()`) in interrupt handlers.
**DO**: Check return values -- `errQUEUE_FULL` means you're losing data.

**DON'T**: Put large structures in queues (they copy data). Send pointers to statically allocated buffers instead.
**DON'T**: Use a queue of size 1 as a mutex -- use an actual mutex.

### ISR Rules (CRITICAL)

1. **Keep ISRs short** -- set a flag or give a semaphore, do processing in a task.
2. **Only use `FromISR` API variants** -- `xQueueSendFromISR`, `xSemaphoreGiveFromISR`, `xTaskNotifyFromISR`.
3. **Yield if needed** -- check `pxHigherPriorityTaskWoken` and call `portYIELD_FROM_ISR()`.
4. **Mark ISR functions** with `IRAM_ATTR` on ESP32 -- ISRs must be in IRAM, not flash.
5. **No `printf`, no `ESP_LOG*`, no `malloc`** in ISRs -- they are not ISR-safe.
6. **No floating point** in ISRs on Xtensa ESP32 (FPU context not saved by default).

```cpp
void IRAM_ATTR gpio_isr_handler(void* arg) {
    BaseType_t higher_woken = pdFALSE;
    auto* ctx = static_cast<TaskContext*>(arg);
    xTaskNotifyFromISR(ctx->task_handle, GPIO_EVENT_BIT, eSetBits, &higher_woken);
    portYIELD_FROM_ISR(higher_woken);
}
```

## Timer Patterns

**DO**: Use software timers for periodic non-critical operations (LED blink, status updates).
**DO**: Use hardware timers (via `gptimer` API in IDF v5) for precise timing.
**DON'T**: Do heavy processing in timer callbacks -- they run in the timer service task context.
**DON'T**: Block in timer callbacks -- they share one task with all other timers.

## Task Watchdog (TWDT)

```cpp
// Subscribe a task to TWDT
esp_task_wdt_add(nullptr);  // Add current task

// In main loop -- must feed regularly
while (true) {
    do_work();
    esp_task_wdt_reset();  // Feed the watchdog
    vTaskDelay(pdMS_TO_TICKS(100));
}
```

**DO**: Subscribe long-running tasks to TWDT.
**DO**: Configure TWDT timeout appropriate to task cycle time (typically 5-30 seconds).
**DON'T**: Disable TWDT in production -- it's your field recovery mechanism.
**DON'T**: Feed the watchdog unconditionally at the top of the loop -- only after confirming work completed.

## Dual-Core Specific Rules (ESP32, S3, P4)

- WiFi/BLE callbacks execute on Core 0 -- never block them
- Pin ISR-heavy tasks to Core 1 to avoid contention with WiFi stack
- Use `xSemaphoreCreateMutex()` (not critical sections) for cross-core synchronization
- `portENTER_CRITICAL()` on ESP-IDF disables interrupts on BOTH cores -- use sparingly
- `portENTER_CRITICAL_SAFE()` works in both task and ISR context (ESP-IDF extension)
- Prefer task notifications over queues for simple signaling (faster, less overhead)

## Common FreeRTOS Bugs on ESP32

| Bug | Symptom | Fix |
|---|---|---|
| Stack overflow | Guru Meditation, LoadProhibited | Increase stack, check high-water mark |
| Priority inversion | High-priority task starved | Use mutex (has inheritance), not binary semaphore |
| Deadlock | Two tasks hang permanently | Enforce lock ordering, use timeouts |
| ISR too long | Watchdog reset, missed events | Move processing to deferred task |
| Blocking on Core 0 | WiFi disconnects, BLE fails | Pin blocking tasks to Core 1 |
| vTaskDelay(0) | Does nothing (doesn't yield) | Use `taskYIELD()` or `vTaskDelay(1)` |
| Queue full ignored | Silent data loss | Always check return value, add overflow counter |
| Mutex in ISR | Undefined behavior, crash | Use binary semaphore or task notification from ISR |
| Float auto-pinning | Core imbalance, unexpected affinity | Explicitly pin tasks that use float |
| `vTaskDelay` for periodic work | Cumulative timing drift | Use `vTaskDelayUntil()` with absolute reference |
| `pdMS_TO_TICKS(1/portTICK_PERIOD_MS)` | Integer division truncates to 0 | Always use `pdMS_TO_TICKS()` macro |
| Deleting tasks externally | Resource leak, heap corruption | Signal task to self-delete with `vTaskDelete(NULL)` |
| Event group manual clear race | Two tasks enter critical region | Use `xClearOnExit = pdTRUE` (atomic test-and-clear) |
| Stack variable passed to task param | Corrupted after scheduler starts | Use static/heap-allocated params |

## Task Design Philosophy

**Event-driven is the foundation.** Polling tasks consume 100% CPU, starve the idle task, and defeat the RTOS. The canonical task:

```cpp
while (true) {
    wait_for_event();   // Blocks here -- consumes zero CPU
    process_event();    // Runs only when needed
}
```

**Rules:**
- A task function must NEVER return -- call `vTaskDelete(NULL)` if done
- Use `vTaskDelayUntil()` (not `vTaskDelay()`) for periodic tasks
- Use `pdMS_TO_TICKS()` everywhere -- never hardcode tick counts
- Multiple instances of the same task function are valid
- The idle task frees memory from deleted tasks -- if starved, memory leaks silently
- Standard `printf()` uses `malloc` internally and needs ~1.8KB stack. Use `ESP_LOGx` macros

## Heap Scheme Selection (Official Guidance)

ESP-IDF uses its own multi-heap allocator, but understanding FreeRTOS heap schemes matters for portability:

| Scheme | Free? | Coalescing | Best For |
|---|---|---|---|
| heap_1 | No | N/A | Allocate-once-at-startup; safety-critical |
| heap_2 | Yes | No | **Avoid** -- fragmentation guaranteed |
| heap_3 | Yes | libc | When stdlib malloc is already in use |
| heap_4 | Yes | Yes | General purpose (recommended) |
| heap_5 | Yes | Yes | Non-contiguous RAM regions |

**ESP-IDF note:** ESP-IDF wraps its own TLSF-based allocator, not vanilla FreeRTOS heaps. But the principles apply: never use `malloc()`/`free()` directly (not thread-safe); use `pvPortMalloc()`/`vPortFree()` or ESP-IDF's `heap_caps_malloc()`/`free()`.

## Task Notification Patterns (Lightweight Alternative)

Task notifications are 45% faster than binary semaphores and use zero extra RAM (built into each task's TCB):

```cpp
// ISR-to-task signaling (replaces binary semaphore)
void IRAM_ATTR sensor_isr(void* arg) {
    BaseType_t woken = pdFALSE;
    vTaskNotifyGiveFromISR(sensor_task_handle, &woken);
    portYIELD_FROM_ISR(woken);
}

void sensor_task(void*) {
    while (true) {
        ulTaskNotifyTake(pdTRUE, portMAX_DELAY);  // pdTRUE = clear to 0 (binary mode)
        process_sensor_data();
    }
}
```

**When to use:** Single producer, single consumer. Replaces binary/counting semaphore.
**When NOT to use:** Multiple tasks waiting on same event (use event group), broadcasting (use event group), complex data passing (use queue).
