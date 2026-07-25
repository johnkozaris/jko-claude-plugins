# Memory and DMA

ESP32 memory regions are capability-specific. Before allocating or moving a
buffer, inspect the API's alignment, lifetime, DMA, internal-memory, executable,
and cache requirements for the target chip.

PSRAM is useful capacity but is not interchangeable with internal RAM. DMA,
ISRs, flash-disabled windows, and some peripherals require specific capability
flags or internal memory. Check the actual allocation result and fail
observably.

Avoid repeated variable-size allocation in long-lived loops when the size
distribution can fragment the heap. Prefer ownership with deterministic
release, fixed buffers, pools, or bounded reuse where measurement shows risk.
Track minimum free heap and largest free block over soak tests rather than only
boot-time free bytes.

Large task-local buffers consume stack, not heap. Static buffers reduce
allocation but add shared-state and lifetime concerns. Choose placement from
access pattern and concurrency, not a universal rule.

For memory corruption, capture the first invalid access, heap-integrity result,
allocation timeline, and decoded backtrace. Later crashes are often secondary.

On legacy original-ESP32 silicon, verify whether the PSRAM cache workaround is
present in the actual compiler flags. ESP-IDF toolchains normally apply the
appropriate workaround for affected revisions; some PlatformIO configurations
have required `-mfix-esp32-psram-cache-issue` explicitly. Check the silicon
revision and generated build command before adding it--later revisions are not
affected and toolchain behavior changes.
