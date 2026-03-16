# Testing Strategies for ESP32 C++ Projects

## Testing Pyramid for Embedded

```
        /  HIL Tests  \         Hardware-in-the-loop (real device)
       / Integration   \        On-device component tests
      / Host Unit Tests \       Native compilation, fast, no hardware
     / Static Analysis   \      cppcheck, clang-tidy, compiler warnings
    /___________________  \     Foundation: catches most bugs
```

## Host-Based Unit Testing

Compile and run tests on your development machine (no ESP32 needed):

### With ESP-IDF (pytest + Unity)
```cpp
// test/test_parser.cpp
#include "unity.h"
#include "my_parser.h"

TEST_CASE("Parse valid sensor data", "[parser]") {
    uint8_t data[] = {0x01, 0x02, 0x03, 0x04};
    auto result = parse_sensor_data(data, sizeof(data));
    TEST_ASSERT_TRUE(result.ok());
    TEST_ASSERT_FLOAT_WITHIN(0.1, 25.6, result.value.temperature);
}

TEST_CASE("Parse empty data returns error", "[parser]") {
    auto result = parse_sensor_data(nullptr, 0);
    TEST_ASSERT_FALSE(result.ok());
    TEST_ASSERT_EQUAL(ESP_ERR_INVALID_ARG, result.error);
}
```

### With PlatformIO (Unity)
```ini
# platformio.ini
[env:native]
platform = native
test_framework = unity
build_flags = -std=c++17

[env:esp32_test]
platform = espressif32
board = esp32dev
framework = espidf
test_framework = unity
```

```cpp
// test/test_ring_buffer/test_ring_buffer.cpp
#include <unity.h>
#include "ring_buffer.h"

void test_push_pop() {
    RingBuffer<int, 10> buf;
    TEST_ASSERT_TRUE(buf.push(42));
    int val;
    TEST_ASSERT_TRUE(buf.pop(val));
    TEST_ASSERT_EQUAL(42, val);
}

void test_full_buffer_rejects() {
    RingBuffer<int, 2> buf;
    TEST_ASSERT_TRUE(buf.push(1));
    TEST_ASSERT_TRUE(buf.push(2));
    TEST_ASSERT_FALSE(buf.push(3));  // Full!
}

int main() {
    UNITY_BEGIN();
    RUN_TEST(test_push_pop);
    RUN_TEST(test_full_buffer_rejects);
    return UNITY_END();
}
```

### Abstracting Hardware for Testability
```cpp
// Interface for testing
class ISensor {
public:
    virtual ~ISensor() = default;
    virtual esp_err_t init() = 0;
    virtual Result<float> read_temperature() = 0;
};

// Real implementation
class BME280 : public ISensor {
    i2c_port_t port_;
    uint8_t addr_;
public:
    BME280(i2c_port_t port, uint8_t addr) : port_(port), addr_(addr) {}
    esp_err_t init() override { /* real I2C init */ }
    Result<float> read_temperature() override { /* real I2C read */ }
};

// Mock for host tests
class MockSensor : public ISensor {
public:
    float next_temperature = 25.0f;
    esp_err_t next_error = ESP_OK;
    int read_count = 0;

    esp_err_t init() override { return ESP_OK; }
    Result<float> read_temperature() override {
        read_count++;
        if (next_error != ESP_OK) return Result<float>::fail(next_error);
        return Result<float>::success(next_temperature);
    }
};
```

## On-Device Testing

### ESP-IDF pytest Integration
```python
# pytest_my_test.py
import pytest

def test_sensor_reading(dut):
    dut.expect("Sensor initialized")
    dut.expect(r"Temperature: (\d+\.\d+)", timeout=10)
    temp = float(dut.match.group(1))
    assert 15.0 < temp < 45.0, f"Temperature out of range: {temp}"

def test_wifi_connection(dut):
    dut.expect("WiFi connected", timeout=30)
    dut.expect(r"IP: (\d+\.\d+\.\d+\.\d+)", timeout=10)
```

### Running Tests
```bash
# ESP-IDF
idf.py build
pytest --target esp32 --port /dev/ttyUSB0

# PlatformIO - on device
pio test -e esp32dev

# PlatformIO - host (native)
pio test -e native
```

## Static Analysis

### Compiler Warnings (First Line of Defense)
```cmake
# In CMakeLists.txt
target_compile_options(${COMPONENT_LIB} PRIVATE
    -Wall -Wextra -Wpedantic
    -Wshadow -Wconversion -Wsign-conversion
    -Wnull-dereference -Wdouble-promotion
    -Wformat=2 -Woverloaded-virtual
)
```

### cppcheck
```bash
cppcheck --enable=all --suppress=missingInclude \
    --std=c++20 --platform=unix32 \
    --inline-suppr \
    src/ components/
```

### clang-tidy
```yaml
# .clang-tidy
Checks: >
    -*,
    bugprone-*,
    cert-*,
    cppcoreguidelines-*,
    modernize-*,
    performance-*,
    readability-*,
    -modernize-use-trailing-return-type,
    -readability-magic-numbers,
    -cppcoreguidelines-avoid-magic-numbers
WarningsAsErrors: 'bugprone-*,cert-*'
```

### Key Static Analysis Checks for Embedded

| Check | Tool | Catches |
|---|---|---|
| Uninitialized variables | Compiler `-Wall` | UB, random behavior |
| Integer overflow | `-Wconversion` | Silent data corruption |
| Null dereference | cppcheck, `-Wnull-dereference` | Crashes |
| Buffer overflow | cppcheck, ASAN | Memory corruption |
| Unused variables | `-Wunused` | Dead code |
| Shadow variables | `-Wshadow` | Logic errors |
| Float promotion | `-Wdouble-promotion` | Wasted CPU on MCU without FPU64 |
| Format string | `-Wformat=2` | Crashes, security holes |

## CI/CD for Embedded Projects

### GitHub Actions Example
```yaml
name: ESP32 CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    container: espressif/idf:v5.3
    steps:
      - uses: actions/checkout@v4
      - name: Build for ESP32
        run: |
          . $IDF_PATH/export.sh
          idf.py set-target esp32
          idf.py build
      - name: Build for ESP32-S3
        run: |
          idf.py fullclean
          idf.py set-target esp32s3
          idf.py build
      - name: Run host tests
        run: |
          cd test
          cmake -B build && cmake --build build
          ./build/tests
      - name: Static analysis
        run: cppcheck --enable=all --error-exitcode=1 main/ components/
      - name: Size report
        run: idf.py size-components
```

## What to Test

### Must Test
- Protocol parsers (binary, JSON, custom)
- State machine transitions
- CRC/checksum calculations
- Configuration parsing
- Data conversion functions
- Business logic (independent of hardware)

### Should Test on Device
- WiFi connection and reconnection
- Peripheral communication (I2C, SPI, UART)
- Deep sleep and wake behavior
- OTA update flow
- Memory usage under load
- Long-running stability (soak tests)

### Hard to Test (Inspect Instead)
- ISR handlers (timing-dependent)
- DMA transfers
- Real-time deadlines
- Power consumption
- RF performance
