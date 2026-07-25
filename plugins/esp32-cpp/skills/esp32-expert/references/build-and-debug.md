# Build and debugging

Use the project's framework and task runner. Do not mix `idf.py` instructions
into a PlatformIO workflow or assume Arduino APIs in a native ESP-IDF component.

Preserve the exact ELF and map file for the firmware under test. Decode panic
addresses and backtraces against that build; an address decoded against a later
binary is misleading evidence.

Capture reset reason, panic text, exception registers, task/watchdog name,
stack high-water data, heap state, and the events immediately before failure.
Separate boot loops, brownout, watchdog, abort, memory corruption, and explicit
restart before forming a theory.

Distinguish the task watchdog ("Task watchdog got triggered", usually a
starved/non-yielding task) from the interrupt watchdog ("Interrupt wdt timeout
on CPU0/1", usually interrupts disabled or a critical section running too
long). They have different causes and fixes.

Use core dumps, GDB/JTAG, heap tracing, and application tracing when logs cannot
observe the failure without perturbing it. Keep debug instrumentation bounded
and removable.

Treat `sdkconfig`, partition tables, linker configuration, component
dependencies, and generated build metadata as part of the program. Verify
current ESP-IDF or PlatformIO documentation when flags or component APIs differ
from installed versions.

Two ESP-IDF CMake ordering traps are easy to miss:

- Set `EXTRA_COMPONENT_DIRS` and `COMPONENTS` before `project(...)`; placing
  them before the IDF project-CMake include is conventional but not required.
  Assignment after `project()` is too late for component discovery.
- `set_source_files_properties()` can silently miss sources discovered through
  `SRC_DIRS`. Call it after `idf_component_register()` with absolute paths such
  as `${CMAKE_CURRENT_SOURCE_DIR}/...`, or verify the generated compile command
  contains the intended property.
