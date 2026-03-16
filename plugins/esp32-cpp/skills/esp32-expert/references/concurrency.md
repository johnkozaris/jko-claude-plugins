# Concurrency, Deadlocks & Synchronization Patterns

When to use each primitive, when NOT to use it, and the anti-patterns that cause field failures.

## Primitive Selection Guide: When to Use / When NOT to Use

### Mutex (`xSemaphoreCreateMutex`)

**USE when:** Protecting a shared resource (I2C bus, SPI device, data structure, file).
**WHY:** Has priority inheritance -- prevents unbounded priority inversion.

**DO NOT use when:**
- Signaling from ISR (mutexes cannot be used in ISR context AT ALL)
- Simple event notification between tasks (use task notification instead -- 45% faster)
- Counting available resources (use counting semaphore)

**Rules:**
- Same task that takes MUST give (ownership semantics)
- Never hold across `vTaskDelay()` or any blocking call
- Use timeouts, never `portMAX_DELAY` in production
- Keep critical section as short as possible -- copy data out, release, then process

### Binary Semaphore (`xSemaphoreCreateBinary`)

**USE when:** ISR-to-task signaling (ISR gives, task takes).
**WHY:** ISR-safe, lightweight, latches the event.

**DO NOT use when:**
- Protecting shared resources (NO priority inheritance -- AP-14)
- Multiple events need counting (use counting semaphore)
- Single producer + single consumer simple signal (task notification is faster)

**Rules:**
- Any task/ISR can give -- no ownership
- Starts in "taken" state -- must give once before first take succeeds
- Multiple rapid gives collapse to one (only latches, doesn't count)

### Counting Semaphore (`xSemaphoreCreateCounting`)

**USE when:** Managing a pool of N identical resources, or counting events that arrive faster than consumed.
**WHY:** Tracks count; each give increments, each take decrements.

**DO NOT use when:**
- Protecting a single resource (use mutex)
- Simple 1-shot signaling (use binary semaphore or task notification)

### Queue (`xQueueCreate`)

**USE when:** Passing data between tasks or from ISR to task.
**WHY:** Copies data, inherently thread-safe, provides backpressure.

**DO NOT use when:**
- Just signaling an event with no data (use task notification or semaphore)
- Passing large structs by value (pass pointer instead -- queue copies the data)

**Rules:**
- Size queue for burst rate, not average rate
- Always check return value -- `errQUEUE_FULL` means data loss
- Use `FromISR` variants in ISRs
- For large data: queue a pointer to a statically/pool-allocated buffer

### Event Group (`xEventGroupCreate`)

**USE when:** Task must wait on multiple independent events (OR/AND logic), or multiple tasks must synchronize at a barrier.
**WHY:** Bitmask of events, flexible wait conditions, broadcast capability.

**DO NOT use when:**
- Simple 1-to-1 signaling (overkill -- use task notification)
- Passing data (use queue)
- ISR context (uses `FromISR` variant which defers to timer daemon task -- higher latency)

**Rules:**
- Use `xClearOnExit = pdTRUE` for atomic test-and-clear (avoids race -- AP in anti-patterns)
- For multi-task rendezvous: use `xEventGroupSync()`, NOT separate set + wait
- 24 usable bits (when `configUSE_16_BIT_TICKS = 0`)

### Task Notification (`xTaskNotify` / `xTaskNotifyGive`)

**USE when:** Lightweight 1-to-1 signaling, replacing binary/counting semaphore in common cases.
**WHY:** 45% faster than semaphores, zero extra RAM (built into TCB).

**DO NOT use when:**
- Multiple tasks need to wait on the same event
- Broadcasting to several tasks
- Complex data passing (32 bits max)

**Rules:**
- Only ONE task can wait on a notification slot
- Multiple rapid gives: use `eIncrement` action to count, not `eSetBits` (AP-15 in FreeRTOS anti-patterns -- events collapse)
- FreeRTOS 10.4+ supports per-task notification arrays (multiple slots)

### Critical Section (`portENTER_CRITICAL` / `taskENTER_CRITICAL`)

**USE when:** Very short atomic operations (few instructions) where ISR access must also be excluded.

**DO NOT use when:**
- Anything longer than a few microseconds (degrades interrupt latency)
- Task-to-task synchronization without ISR involvement (use mutex)
- Cross-core exclusion on SMP (see below)

**SMP WARNING:** On dual-core ESP32, `taskENTER_CRITICAL()` only protects the calling core. Use `portENTER_CRITICAL(&spinlock)` with a `portMUX_TYPE` for cross-core exclusion. Keep spinlock-held time absolute minimum -- other core spins burning CPU.

### Stream/Message Buffer (`xStreamBufferCreate` / `xMessageBufferCreate`)

**USE when:** Continuous byte stream or variable-length messages, single producer + single consumer.
**WHY:** Lower overhead than queue for byte-oriented data.

**DO NOT use when:**
- Multiple producers or multiple consumers (undefined behavior!)
- Fixed-size structured data (queue is simpler)

## Deadlock: Causes, Prevention, Detection

### The Four Conditions for Deadlock

ALL four must be present simultaneously:
1. **Mutual exclusion** -- resources cannot be shared
2. **Hold and wait** -- task holds one resource while waiting for another
3. **No preemption** -- resources cannot be forcibly taken
4. **Circular wait** -- A waits for B, B waits for A

**Break ANY one condition to prevent deadlock.**

### Prevention Strategies (Ranked by Effectiveness)

**1. Avoid nested locks entirely (best)**
Redesign so no task ever holds two locks simultaneously. Copy data under one lock, release, acquire the next.

**2. Enforce global lock ordering**
Assign a hierarchy number to every mutex. Every task acquires in ascending order. This eliminates circular wait.

```cpp
// Lock ordering: I2C (1) < SPI (2) < UART (3) < WiFi (4)
// ALWAYS acquire in this order, never reverse

void task_needing_both(void) {
    xSemaphoreTake(i2c_mutex, timeout);   // 1 first
    xSemaphoreTake(spi_mutex, timeout);   // 2 second
    // ... work ...
    xSemaphoreGive(spi_mutex);            // release in reverse
    xSemaphoreGive(i2c_mutex);
}
```

**3. Use timeouts on ALL lock acquisitions**
Never use `portMAX_DELAY` in production. A timeout allows detection and recovery:

```cpp
if (xSemaphoreTake(mutex, pdMS_TO_TICKS(1000)) != pdTRUE) {
    ESP_LOGE("DEADLOCK", "Mutex timeout -- possible deadlock");
    // Release any held resources, retry, or reset
}
```

**4. Server Task pattern (eliminate shared state)**
Instead of multiple tasks sharing a resource with mutexes, create one task that owns the resource exclusively. Other tasks communicate via queues:

```cpp
// Server task owns the I2C bus exclusively
void i2c_server_task(void*) {
    I2CRequest req;
    while (xQueueReceive(i2c_request_queue, &req, portMAX_DELAY)) {
        esp_err_t result = i2c_master_transmit_receive(...);
        xQueueSend(req.response_queue, &result, portMAX_DELAY);
    }
}
// No mutex needed -- only one task touches I2C hardware
```

**5. Try-lock with backoff**
```cpp
bool acquire_both(SemaphoreHandle_t a, SemaphoreHandle_t b) {
    if (xSemaphoreTake(a, pdMS_TO_TICKS(100)) == pdTRUE) {
        if (xSemaphoreTake(b, pdMS_TO_TICKS(100)) == pdTRUE) {
            return true;  // Got both
        }
        xSemaphoreGive(a);  // Release first, try again later
    }
    return false;
}
```

### Why This Matters

A high-priority task blocks on a resource held by a low-priority task. A medium-priority task preempts the low-priority holder, starving the high-priority task indefinitely. Watchdog fires, device resets. This happens in production with binary semaphores because they have no priority inheritance.

**Fix:** Always use `xSemaphoreCreateMutex()` for resource protection. Mutexes have priority inheritance built in.

## Race Conditions

### The Fundamental Pattern
```
Task A reads shared_var → (preemption) → Task B writes shared_var → Task A writes stale value
```

### Common Race Scenarios on ESP32

**1. Read-modify-write on shared variable**
Even `counter++` is not atomic -- compiles to load/add/store. On dual-core ESP32, both cores can execute the sequence simultaneously.

**2. Checking then acting without lock**
```cpp
// BAD: TOCTOU (Time-of-Check-Time-of-Use)
if (buffer_available()) {    // Another task may fill buffer between check and write
    write_to_buffer(data);
}
```

**3. Partially-updated multi-field struct**
A task reads `state.temperature` from one sample and `state.humidity` from another because the writer was preempted mid-update. Fix: copy entire struct under mutex.

**4. ISR + task accessing same variable**
ISR writes, task reads. Without `volatile`, compiler caches. Even with `volatile`, multi-byte access is not atomic on 8/16-bit values on 32-bit MCU (or 64-bit values on 32-bit MCU). Fix: disable interrupts for the read, or use `std::atomic`.

### Lock-Free Patterns (When Locks Are Too Expensive)

**Single-writer single-reader ring buffer:** No mutex needed if only one task writes and one task reads. The write index is only modified by the writer; the read index by the reader. Works because single-word reads/writes are atomic on 32-bit ARM/Xtensa.

**Double-read consistency check:**
```cpp
uint32_t read_consistent(volatile uint32_t* high, volatile uint32_t* low) {
    uint32_t h1, h2, l;
    do {
        h1 = *high;
        l = *low;
        h2 = *high;
    } while (h1 != h2);  // Retry if high word changed during read
    return (h1 << 16) | l;
}
```

**Sequence lock (seqlock):** Writer increments a sequence counter before and after update. Reader checks counter before and after read -- if odd or changed, retry. Zero writer blocking.

## Priority Inversion

### Bounded vs Unbounded

- **Bounded:** High-priority task waits only as long as low-priority task holds the lock. Annoying but quantifiable.
- **Unbounded:** Medium-priority task preempts lock-holder, blocking high-priority task indefinitely. This is the dangerous form.

### Prevention on ESP32

| Mechanism | How | When |
|---|---|---|
| Mutex priority inheritance | Automatic: FreeRTOS raises lock-holder to waiter's priority | Default for `xSemaphoreCreateMutex()` |
| Priority ceiling | Set resource's priority ceiling; task elevates when acquiring | Manual implementation needed |
| Server task | Resource owner is always highest-priority for that resource | Design-level solution |
| Task pinning | Pin conflicting tasks to same core | Reduces SMP contention |

## Dual-Core ESP32 Specific Concurrency Rules

1. **WiFi/BLE callbacks run on Core 0** -- never hold locks when WiFi/BLE might need them
2. **Pin ISR-heavy tasks to Core 1** to avoid contention with protocol stack
3. **`portENTER_CRITICAL(&mux)`** is the ONLY safe cross-core critical section (uses spinlock)
4. **`taskDISABLE_INTERRUPTS()`** only affects the calling core -- NOT safe for SMP exclusion
5. **Float auto-pinning**: task using `float` is silently pinned to current core. Plan for this.
6. **`std::atomic<T>`** is safe for cross-core atomics on ESP32 (uses hardware compare-and-swap)
7. **FreeRTOS mutexes** are SMP-safe on ESP-IDF (internally use spinlocks)
8. **Priority-based exclusion is BROKEN on SMP**: a high-priority task on Core 0 does NOT prevent a lower-priority task on Core 1 from running. Always use explicit synchronization.

## Anti-Pattern Quick Reference

| Anti-Pattern | Consequence | Fix |
|---|---|---|
| Binary semaphore for resource protection | Priority inversion | Use mutex |
| `portMAX_DELAY` on all locks | Unrecoverable deadlock | Use finite timeouts |
| Inconsistent lock ordering | Deadlock after weeks | Document and enforce order |
| Holding lock across blocking call | Starvation, deadlock | Copy-and-release pattern |
| `volatile` for synchronization | No memory barriers, races | Use FreeRTOS primitives or `std::atomic` |
| `taskDISABLE_INTERRUPTS` on SMP | Only protects one core | Use `portENTER_CRITICAL(&mux)` |
| Multiple tasks same `spi_device_handle_t` | DMA corruption | Mutex or bus acquire |
| Checking-then-acting without lock | TOCTOU race | Atomic check-and-act under lock |
| Polling with zero timeout | 100% CPU, starves others | Block with timeout |
| Event group manual clear | Race window | `xClearOnExit = pdTRUE` |
| Recursive mutex (design smell) | Masks poor decomposition | Restructure code |
| `xTaskResumeFromISR` for signaling | Events silently lost | Use semaphore or notification |
