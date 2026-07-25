# FreeRTOS runtime and concurrency

Identify the execution context before judging an operation: task, ISR, timer
callback, Wi-Fi/BLE event loop, high-resolution timer, or startup/shutdown code.

ISRs must do bounded, ISR-safe work and defer the rest. Verify IRAM/flash-cache
requirements against the chip and SDK API in use. A function callable from a
task is not automatically legal from an ISR.

With `ESP_INTR_FLAG_IRAM`, the entire reachable call path and accessed data must
remain available while flash cache is disabled; `IRAM_ATTR` on only the
top-level ISR is insufficient. The panic signature "Cache disabled but cached
memory region accessed" points at placement rather than ISR business logic.

Choose synchronization by ownership. Queues and task notifications fit message
delivery; mutexes protect short shared critical sections and preserve priority
inheritance where supported; event groups represent bits of state. Bound queue
capacity and wait time.

On multicore targets, disabling interrupts or entering a local critical section
may protect only one core. Use the SDK's SMP-safe primitive and inspect lock
ordering before treating a watchdog timeout as a need for a longer timeout.

Measure stack high-water marks on realistic paths, including TLS, logging, and
error handling. Increasing a stack can hide recursion, large local buffers, or
the wrong task boundary.

ESP-IDF task-creation stack sizes are expressed in bytes, unlike upstream
FreeRTOS ports that commonly use `StackType_t` units. In Arduino-ESP32,
`setup()` and `loop()` execute inside FreeRTOS tasks rather than on bare metal;
inspect the core version and task configuration before assuming stack or core
affinity.
