# Build System Guide: ESP-IDF & PlatformIO

## Project Structure Dos & Don'ts

### ESP-IDF Project Layout

**DO:**
- Keep `main/` thin -- entry point + wiring only, push logic into `components/`
- One component per responsibility (`drivers/`, `protocol/`, `app_logic/`)
- Public headers in `include/`, private headers in `src/` or `priv_include/`
- Commit `sdkconfig.defaults`, gitignore `sdkconfig`
- Gitignore `build/` and `managed_components/`
- Commit `dependencies.lock` for reproducible CI builds
- Use `idf.py save-defconfig` to auto-generate minimal `sdkconfig.defaults`

**DON'T:**
- Put everything in `main/` -- it becomes unmaintainable past ~500 lines
- Commit `sdkconfig` -- it contains machine-specific paths and pollutes diffs
- Commit `build/` or `managed_components/` -- both are fully regenerable
- Edit `managed_components/` directly -- changes are overwritten on rebuild
- Put implementation in header files unless it's templates or inline

```
my_project/                     # GOOD layout
├── CMakeLists.txt              # 3 mandatory lines only
├── sdkconfig.defaults          # Committed: minimal config overrides
├── sdkconfig.defaults.esp32s3  # Per-variant overrides
├── partitions.csv              # Custom partition table
├── main/
│   ├── CMakeLists.txt
│   ├── idf_component.yml       # Managed dependencies
│   └── main.cpp                # Thin: init + task creation only
├── components/
│   ├── drivers/                # Hardware abstraction
│   │   ├── CMakeLists.txt
│   │   ├── include/drivers/    # Public API headers
│   │   └── src/                # Implementation
│   ├── app_logic/              # Business logic
│   │   ├── CMakeLists.txt
│   │   └── ...
│   └── ui/                     # LVGL screens (if applicable)
│       ├── CMakeLists.txt
│       └── ...
└── docs/                       # Hardware specs, pinouts
```

### PlatformIO Project Layout

**DO:**
- Put reusable modules in `lib/` with their own subfolder each
- Name files descriptively: `wifi_manager.cpp`, `sensor_handler.cpp`
- Use `lib_deps` for external libraries, never manual downloads
- Separate debug/release as different `[env:]` sections

**DON'T:**
- Put everything in `src/main.cpp` -- split by feature
- Mix `lib/` libraries with `src/` application code
- Use `lib_extra_dirs` pointing at individual libraries (point at parent directory)
- Forget `monitor_filters = esp32_exception_decoder` -- you need readable crash traces

```
my_project/                     # GOOD layout
├── platformio.ini
├── src/
│   └── main.cpp                # Thin entry point
├── include/
│   └── config.h                # Project-wide config
├── lib/
│   ├── WiFiManager/            # Each module in own folder
│   │   ├── WiFiManager.h
│   │   └── WiFiManager.cpp
│   └── SensorDriver/
│       ├── SensorDriver.h
│       └── SensorDriver.cpp
├── test/
│   ├── test_sensor/
│   └── test_wifi/
└── boards/                     # Custom board definitions
```

## CMakeLists.txt Dos & Don'ts

### Top-Level CMakeLists.txt

**DO:**
```cmake
# These 3 lines in this exact order. Nothing else needed.
cmake_minimum_required(VERSION 3.16)
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(my_project)
```

**DON'T:**
```cmake
# WRONG: project() before include()
cmake_minimum_required(VERSION 3.16)
project(my_project)                                    # TOO EARLY
include($ENV{IDF_PATH}/tools/cmake/project.cmake)     # TOO LATE

# WRONG: setting variables after project()
project(my_project)
set(EXTRA_COMPONENT_DIRS "components/external")        # HAS NO EFFECT
```

Variables go BETWEEN `cmake_minimum_required()` and `include()`:
```cmake
cmake_minimum_required(VERSION 3.16)
set(EXTRA_COMPONENT_DIRS "${CMAKE_CURRENT_LIST_DIR}/shared_components")
set(SDKCONFIG_DEFAULTS "sdkconfig.defaults;sdkconfig.defaults.prod")
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(my_project)
```

### Component CMakeLists.txt

**DO:**
```cmake
idf_component_register(
    SRCS "sensor.cpp" "sensor_hal.cpp"      # Explicit file list
    INCLUDE_DIRS "include"                   # Public headers
    PRIV_INCLUDE_DIRS "src"                  # Private headers
    REQUIRES driver                          # Public deps (in your headers)
    PRIV_REQUIRES esp_log nvs_flash          # Private deps (in your .cpp only)
)
```

**DON'T:**
```cmake
# WRONG: SRC_DIRS breaks set_source_files_properties
idf_component_register(SRC_DIRS "src" INCLUDE_DIRS "include")

# WRONG: everything in REQUIRES (creates unnecessary coupling)
idf_component_register(
    SRCS "my_code.cpp"
    REQUIRES driver esp_log nvs_flash esp_wifi esp_event  # Most should be PRIV_REQUIRES
)

# WRONG: using CONFIG_xxx in REQUIRES (not loaded yet)
if(CONFIG_ENABLE_WIFI)                                     # BROKEN
    list(APPEND requires "esp_wifi")
endif()
idf_component_register(REQUIRES ${requires})               # deps resolved before sdkconfig
```

**Conditional sources (the right way):**
```cmake
set(srcs "core.cpp")
if(CONFIG_ENABLE_WIFI)
    list(APPEND srcs "wifi_driver.cpp")
endif()
idf_component_register(SRCS ${srcs} INCLUDE_DIRS "include" PRIV_REQUIRES esp_wifi)
```

### platformio.ini Dos & Don'ts

**DO:**
```ini
[env:esp32dev]
platform = espressif32
board = esp32dev
framework = espidf
monitor_speed = 115200
monitor_filters = esp32_exception_decoder    # Readable crash traces
build_flags =
    -Wall -Wextra                            # Warnings on
    -DCORE_DEBUG_LEVEL=3                     # Info logging
board_build.partitions = partitions.csv      # Custom partitions
```

**DON'T:**
```ini
# WRONG: build_flags appears twice (only last one applies)
build_flags = -Wall
build_flags = -DFOO                          # Overwrites -Wall!

# WRONG: string with spaces instead of multiline
build_flags = -Wall -Wextra -DFOO            # Works but fragile, hard to read

# WRONG: no monitor speed (defaults may not match Serial.begin())
[env:esp32dev]
platform = espressif32
board = esp32dev
# monitor_speed missing -- garbled output
```

**Removing default flags:**
```ini
build_unflags = -Os                          # Remove default size optimization
build_flags = -O2                            # Replace with speed optimization
```

## Modern CMake Dos & Don'ts

ESP-IDF wraps CMake heavily, but these modern CMake principles still apply:

**DO:**
- Use `target_compile_definitions()` instead of `add_definitions()` -- scoped to target
- Use `target_compile_options()` instead of `add_compile_options()` -- scoped to target
- Use `target_link_libraries()` with `PUBLIC`/`PRIVATE`/`INTERFACE` -- explicit visibility
- Use `CMAKE_CURRENT_LIST_DIR` for paths relative to the current CMakeLists.txt
- Use generator expressions (`$<BUILD_INTERFACE:...>`) for portable include paths
- Treat CMake files as code -- keep them clean, readable, no dead code

**DON'T:**
- Use `include_directories()` -- use `target_include_directories()` (scoped)
- Use `link_libraries()` -- use `target_link_libraries()` (scoped)
- Use `add_definitions(-DFOO)` -- use `target_compile_definitions(${COMPONENT_LIB} PRIVATE FOO)`
- Use `file(GLOB ...)` for source files -- explicit `SRCS` lists are rebuild-safe
- Use old ESP-IDF variables (`COMPONENT_ADD_LDFLAGS`, `COMPONENT_OBJS`) -- deprecated since v4
- Set `CMAKE_C_FLAGS` or `CMAKE_CXX_FLAGS` globally -- use per-target options

**ESP-IDF specific modern patterns:**
```cmake
# Per-component compile options (after idf_component_register)
target_compile_options(${COMPONENT_LIB} PRIVATE -Wno-unused-variable)

# Per-component defines
target_compile_definitions(${COMPONENT_LIB} PRIVATE MY_FEATURE=1)

# Per-file compile flags (requires explicit SRCS, not SRC_DIRS)
set_source_files_properties(legacy.c PROPERTIES COMPILE_FLAGS -Wno-error)

# Project-wide options (after project())
idf_build_set_property(COMPILE_OPTIONS "-Wall;-Wextra" APPEND)

# Trimming build to only needed components (faster builds)
set(COMPONENTS main my_driver my_sensor)  # Transitive deps auto-resolved
```

## Detecting the Build System

| Indicator | Framework |
|---|---|
| `platformio.ini` | PlatformIO |
| `CMakeLists.txt` + `main/CMakeLists.txt` + `sdkconfig` | ESP-IDF native |
| `platformio.ini` with `framework = espidf` | PlatformIO with IDF framework |
| `platformio.ini` with `framework = arduino` | PlatformIO with Arduino framework |

## ESP-IDF Native Build System

### Project Structure
```
my_project/
├── CMakeLists.txt                # Top-level: include IDF cmake
├── sdkconfig                     # Generated by menuconfig (gitignore this)
├── sdkconfig.defaults            # Default config overrides (commit this)
├── sdkconfig.defaults.esp32s3    # Variant-specific defaults
├── partitions.csv                # Custom partition table
├── main/
│   ├── CMakeLists.txt            # Main component CMakeLists
│   ├── main.cpp                  # app_main entry point
│   ├── Kconfig.projbuild         # Project-level menuconfig options
│   └── idf_component.yml         # Component manager dependencies
├── components/
│   ├── drivers/
│   │   ├── CMakeLists.txt
│   │   ├── include/
│   │   │   └── drivers/
│   │   │       ├── bme280.h
│   │   │       └── display.h
│   │   └── src/
│   │       ├── bme280.cpp
│   │       └── display.cpp
│   └── app_logic/
│       ├── CMakeLists.txt
│       ├── include/
│       │   └── app_logic/
│       └── src/
└── managed_components/           # Downloaded by component manager (gitignore)
```

### Top-Level CMakeLists.txt
```cmake
cmake_minimum_required(VERSION 3.16)
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(my_project)
```

### Component CMakeLists.txt
```cmake
idf_component_register(
    SRCS "src/bme280.cpp" "src/display.cpp"
    INCLUDE_DIRS "include"
    REQUIRES driver i2c spi_master    # Public dependencies
    PRIV_REQUIRES esp_log             # Private dependencies
)

# Set C++ standard for this component (if different from project default)
# target_compile_options(${COMPONENT_LIB} PRIVATE -std=gnu++20)
```

### REQUIRES vs PRIV_REQUIRES

| Keyword | When to Use | Effect |
|---|---|---|
| `REQUIRES` | Header files include other component headers | Transitive: consumers also get the dependency |
| `PRIV_REQUIRES` | Only .cpp files use the dependency | Non-transitive: consumers don't see it |

**DO**: Use `PRIV_REQUIRES` by default. Only use `REQUIRES` when your public headers need it.
**DON'T**: Put everything in `REQUIRES` -- it creates unnecessary coupling and slows builds.
**DON'T**: Use `CONFIG_xxx` variables in `REQUIRES`/`PRIV_REQUIRES` -- deps are resolved before sdkconfig loads.

### sdkconfig.defaults Chaining (Multi-Config Builds)

```cmake
# Multiple defaults applied in order (later overrides earlier)
set(SDKCONFIG_DEFAULTS "sdkconfig.defaults;sdkconfig.defaults.release")
```

For each file, the build system also loads `<file>.<target>` automatically:
- `sdkconfig.defaults` + `sdkconfig.defaults.esp32s3` when target is esp32s3

### Common CMake Mistakes

1. **Setting variables after `project()`** -- `EXTRA_COMPONENT_DIRS`, `COMPONENTS` must be set BETWEEN `cmake_minimum_required()` and `include($ENV{IDF_PATH}/...)`
2. **`SRC_DIRS` with per-file flags** -- `set_source_files_properties()` does NOT work with `SRC_DIRS`. Use explicit `SRCS`
3. **Circular dependencies** -- Not natively supported. Workaround: `set_property(TARGET ${COMPONENT_LIB} APPEND PROPERTY LINK_INTERFACE_MULTIPLICITY 3)`. But prefer restructuring
4. **String syntax instead of CMake list** -- `set(EXTRA_COMPONENT_DIRS "path1 path2")` is WRONG in v5.0+. Use `set(EXTRA_COMPONENT_DIRS "path1" "path2")`

### sdkconfig.defaults Example
```ini
# sdkconfig.defaults -- commit this to version control
CONFIG_ESPTOOLPY_FLASHSIZE_4MB=y
CONFIG_PARTITION_TABLE_CUSTOM=y
CONFIG_PARTITION_TABLE_CUSTOM_FILENAME="partitions.csv"
CONFIG_ESP_TASK_WDT_TIMEOUT_S=10
CONFIG_FREERTOS_HZ=1000
CONFIG_ESP_DEFAULT_CPU_FREQ_MHZ_240=y
CONFIG_COMPILER_OPTIMIZATION_PERF=y

# C++ specific
CONFIG_COMPILER_CXX_EXCEPTIONS=y
CONFIG_COMPILER_CXX_RTTI=n
```

### Component Manager (idf_component.yml)
```yaml
dependencies:
  espressif/esp-idf-cxx: "^1.0"
  espressif/button: "^3.0"
  espressif/led_strip: "^2.4"
  idf:
    version: ">=5.0"
```

### Custom Partition Table
```csv
# partitions.csv
# Name,    Type, SubType, Offset,  Size,    Flags
nvs,       data, nvs,     0x9000,  0x6000,
phy_init,  data, phy,     0xf000,  0x1000,
factory,   app,  factory, 0x10000, 0x1E0000,
ota_0,     app,  ota_0,   0x1F0000,0x1E0000,
ota_1,     app,  ota_1,   0x3D0000,0x1E0000,
nvs_key,   data, nvs_keys,0x5B0000,0x1000,
storage,   data, spiffs,  0x5B1000,0x4F000,
```

### Build Commands
```bash
# Configure (menuconfig)
idf.py menuconfig

# Build
idf.py build

# Flash and monitor
idf.py -p /dev/ttyUSB0 flash monitor

# Build for specific target
idf.py set-target esp32s3
idf.py build

# Clean build
idf.py fullclean

# Size analysis
idf.py size
idf.py size-components
idf.py size-files
```

## PlatformIO Build System

### Project Structure
```
my_project/
├── platformio.ini                # Main configuration
├── src/
│   └── main.cpp                  # Entry point
├── include/
│   └── config.h                  # Project headers
├── lib/
│   ├── BME280/                   # Local libraries
│   │   ├── BME280.h
│   │   └── BME280.cpp
│   └── Display/
│       ├── Display.h
│       └── Display.cpp
├── test/
│   └── test_main.cpp
├── boards/
│   └── custom_board.json         # Custom board definitions
└── sdkconfig.defaults            # ESP-IDF config (if framework = espidf)
```

### CRITICAL: PlatformIO Platform Status (2025-2026)

The official `platform = espressif32` is stuck at Arduino Core 2.x / ESP-IDF 4.x. For Arduino Core 3.x with ESP-IDF 5.x, use the community **pioarduino** fork:

```ini
; Use pioarduino for latest chip support (C6, H2, P4) and IDF 5.x
platform = https://github.com/pioarduino/platform-espressif32/releases/download/stable/platform-espressif32.zip
```

### platformio.ini Configuration
```ini
[env:esp32dev]
platform = espressif32
board = esp32dev
framework = espidf                ; or arduino, or both
monitor_speed = 115200
monitor_filters = esp32_exception_decoder

; Build flags
build_flags =
    -DCORE_DEBUG_LEVEL=4
    -std=gnu++23
    -Wall -Wextra -Werror

; Upload settings
upload_speed = 921600
upload_port = /dev/ttyUSB0

; ESP-IDF specific
board_build.partitions = partitions.csv

[env:esp32s3]
platform = espressif32
board = esp32-s3-devkitc-1
framework = espidf
board_build.flash_mode = qio
board_build.psram = enabled
build_flags =
    -DBOARD_HAS_PSRAM
    -DCORE_DEBUG_LEVEL=3

[env:esp32c3]
platform = espressif32
board = esp32-c3-devkitm-1
framework = espidf
```

### PlatformIO Best Practices

**DO**: Use `lib_deps` for external libraries.
**DO**: Set `monitor_filters = esp32_exception_decoder` for readable crash traces.
**DO**: Use multi-environment configs for different boards.
**DO**: Set explicit `platform_packages` for reproducible builds.

**DON'T**: Mix Arduino and ESP-IDF frameworks unless you understand the implications.
**DON'T**: Use `lib_extra_dirs` for project-internal code -- put it in `lib/`.
**DON'T**: Forget `board_build.partitions` when using custom partition tables.

## Compiler Flags for Embedded C++

### Recommended Warning Flags
```ini
# In platformio.ini build_flags or CMakeLists.txt
-Wall -Wextra -Wpedantic
-Wshadow                    # Warn on variable shadowing
-Wconversion                # Warn on implicit narrowing conversions
-Wsign-conversion           # Warn on sign conversion
-Wnull-dereference          # Warn on null pointer dereference
-Wdouble-promotion          # Warn on implicit float->double promotion
-Wformat=2                  # Stricter format string checking
-Woverloaded-virtual        # Warn on virtual function hiding
-Wnon-virtual-dtor          # Warn on classes with virtual functions but no virtual dtor
```

### Optimization Levels
| Flag | When | Effect |
|---|---|---|
| `-Og` | Debug builds | Best debugging experience, minimal optimization |
| `-O2` | Release builds | Good balance of speed and size |
| `-Os` | Size-constrained | Optimize for size (default in ESP-IDF) |
| `-flto` | Release | Link-time optimization, smaller binary, slower build |

### Debug vs Release Configuration

In ESP-IDF menuconfig:
- `Component config -> Compiler options -> Optimization Level`
- `Debug` for development, `Release/Size` for production

In PlatformIO:
```ini
[env:debug]
build_type = debug
build_flags = -DDEBUG -Og -g3

[env:release]
build_type = release
build_flags = -DNDEBUG -Os -flto
```
