# C++ Guidelines for ESP32 Embedded Development

ESP-IDF v5.5+ compiles C++ with `-std=gnu++23` by default. Use modern C++ effectively within embedded constraints.

## The Zero-Overhead Principle

"What you don't use, you don't pay for. What you do use, you couldn't hand-code any better."

Templates, constexpr, RAII, references, and inline functions are FREE on ESP32. Use them. Virtual functions, RTTI, exceptions, and dynamic polymorphism have measurable costs -- use them deliberately.

## RAII: The Foundation

Every hardware resource MUST be managed with RAII in C++. No exceptions.

```cpp
// RAII for GPIO
class OutputPin {
public:
    explicit OutputPin(gpio_num_t pin) : pin_(pin) {
        gpio_config_t cfg = { .pin_bit_mask = 1ULL << pin, .mode = GPIO_MODE_OUTPUT };
        ESP_ERROR_CHECK(gpio_config(&cfg));
    }
    void set(bool level) { gpio_set_level(pin_, level); }
    ~OutputPin() { gpio_reset_pin(pin_); }
    OutputPin(const OutputPin&) = delete;
    OutputPin& operator=(const OutputPin&) = delete;
private:
    gpio_num_t pin_;
};

// RAII for I2C bus
class I2CMaster {
public:
    explicit I2CMaster(i2c_port_t port, const i2c_config_t& config) : port_(port) {
        ESP_ERROR_CHECK(i2c_param_config(port, &config));
        ESP_ERROR_CHECK(i2c_driver_install(port, config.mode, 0, 0, 0));
    }
    ~I2CMaster() { i2c_driver_delete(port_); }
    I2CMaster(const I2CMaster&) = delete;
    I2CMaster& operator=(const I2CMaster&) = delete;
private:
    i2c_port_t port_;
};

// RAII for FreeRTOS mutex
class MutexLock {
public:
    explicit MutexLock(SemaphoreHandle_t mutex, TickType_t timeout = pdMS_TO_TICKS(5000))
        : mutex_(mutex), acquired_(xSemaphoreTake(mutex, timeout) == pdTRUE) {}
    // Default 5s timeout -- never use portMAX_DELAY in production (hides deadlocks)
    ~MutexLock() { if (acquired_) xSemaphoreGive(mutex_); }
    explicit operator bool() const { return acquired_; }
    MutexLock(const MutexLock&) = delete;
    MutexLock& operator=(const MutexLock&) = delete;
private:
    SemaphoreHandle_t mutex_;
    bool acquired_;
};
```

## Modern C++ Features: Use or Avoid on ESP32

### USE Freely
- **constexpr**: Compile-time computation, zero runtime cost. Prefer over macros.
- **auto**: Reduces verbosity. Use for iterators, lambda captures, complex types.
- **enum class**: Type-safe enumerations. Replace `#define` constants.
- **std::array**: Stack-allocated, bounds-aware. Replace C arrays.
- **std::optional**: Explicit absence. Replace sentinel values (-1, nullptr).
- **std::string_view**: Non-owning string reference. Zero allocation.
- **Structured bindings**: `auto [err, value] = parse_sensor();`
- **if constexpr**: Compile-time branching for template code.
- **[[nodiscard]]**: Force callers to check return values (esp_err_t wrappers).
- **Lambda expressions**: For callbacks, functors, one-off operations.
- **Move semantics**: Transfer ownership without copying.
- **Templates**: Zero-cost type-safe abstractions.
- **Inline variables/functions**: Header-only libraries without ODR issues.
- **std::variant**: Type-safe unions. Replace raw unions.
- **Concepts (C++20)**: Constrain templates, better error messages.

### USE with Caution
- **std::string**: Allocates on heap. Fine for one-time setup, avoid in loops or ISRs.
- **std::vector**: Heap allocated. Pre-reserve capacity or use `std::array` + size.
- **std::function**: Heap allocates if capturing lambda is too large (~24-32 bytes). Use templates instead.
- **Virtual functions**: ~4-byte overhead per vtable pointer, indirect call overhead. Fine for top-level components, avoid in tight loops.
- **std::shared_ptr**: Atomic reference counting overhead. Use `std::unique_ptr` when ownership is clear.
- **std::map/std::set**: Tree-based, many small allocations. Use sorted `std::vector` or `std::array` for small collections.
- **Exceptions**: Enabled by default in ESP-IDF. Use for truly exceptional conditions, not control flow. Consider disabling in memory-constrained projects.

### AVOID
- **RTTI (typeid, dynamic_cast)**: Disabled by default. Large binary overhead. Use `enum class` + `switch` instead.
- **std::unordered_map**: Hash table with many allocations. Use sorted array for small sets.
- **throw in ISR context**: UB. ISRs must never throw.
- **Global constructors with side effects**: Initialization order is undefined across translation units.
- **Deep template recursion**: Increases compile time dramatically. Stay under 256 depth.

## C/C++ Interoperability (Critical for ESP-IDF)

ESP-IDF is a C library. All IDF headers use `extern "C"` guards, but your code must follow rules:

```cpp
// app_main MUST have C linkage
extern "C" void app_main(void) {
    // C++ code is fine here
    auto sensor = std::make_unique<BME280>(I2C_NUM_0, 0x76);
    sensor->init();
}
```

### Designated Initializers
C++ designated initializers are more restrictive than C:

```cpp
// ESP-IDF C style (works in C, NOT in C++23):
gpio_config_t cfg = {
    .pin_bit_mask = (1ULL << GPIO_NUM_2),
    .mode = GPIO_MODE_OUTPUT,
    // C++ requires ALL fields in order, no skipping
};

// C++23 safe approach:
gpio_config_t cfg = {};  // Zero-initialize everything
cfg.pin_bit_mask = (1ULL << GPIO_NUM_2);
cfg.mode = GPIO_MODE_OUTPUT;
cfg.pull_up_en = GPIO_PULLUP_DISABLE;
cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
cfg.intr_type = GPIO_INTR_DISABLE;
```

## constexpr for Embedded

Move computation to compile time. This is FREE -- costs zero cycles at runtime:

```cpp
// Pin configuration at compile time
constexpr gpio_num_t LED_PIN = GPIO_NUM_2;
constexpr uint64_t LED_PIN_MASK = 1ULL << LED_PIN;

// Lookup tables at compile time
constexpr std::array<uint8_t, 256> crc8_table = [] {
    std::array<uint8_t, 256> table{};
    for (int i = 0; i < 256; ++i) {
        uint8_t crc = i;
        for (int j = 0; j < 8; ++j)
            crc = (crc & 0x80) ? (crc << 1) ^ 0x07 : crc << 1;
        table[i] = crc;
    }
    return table;
}();

// Baud rate calculation at compile time
constexpr uint32_t calculate_divider(uint32_t clock, uint32_t baud) {
    return (clock + baud / 2) / baud;
}
static_assert(calculate_divider(80'000'000, 115200) == 694);
```

## Error Handling Strategy

ESP-IDF uses `esp_err_t` (integer error codes). Wrap them for C++ safety:

```cpp
// [[nodiscard]] wrapper -- compiler warns if return value ignored
[[nodiscard]] esp_err_t init_wifi();

// Or wrap in a helper that checks
inline void check(esp_err_t err, const char* msg = "ESP error") {
    if (err != ESP_OK) {
        ESP_LOGE("CHECK", "%s: %s", msg, esp_err_to_name(err));
        // In production: handle gracefully. In development: abort.
        assert(err == ESP_OK);
    }
}

// Usage
check(i2c_driver_install(port, mode, 0, 0, 0), "I2C install failed");
```

## Type Safety Rules

**DO**: Use `enum class` instead of `#define` or bare `enum` for constants.
**DO**: Use strong typedefs/newtypes for physical quantities:
```cpp
struct Celsius { float value; };
struct Pressure { float value; };  // Can't accidentally mix them
```
**DO**: Use `std::byte` instead of `uint8_t` for raw byte buffers (when semantics are "data", not "number").
**DO**: Use `static_cast` explicitly -- never C-style casts in C++.
**DON'T**: Use `reinterpret_cast` except for hardware register access and serialization.
**DON'T**: Use `#define` for constants -- use `constexpr` or `enum class`.
**DON'T**: Pass `bool` parameters -- use `enum class` for readability at call sites.

## Rule of Zero / Five

Prefer the Rule of Zero: don't write special member functions unless managing a resource directly.

```cpp
// Rule of Zero -- compiler generates everything correctly
struct SensorReading {
    float temperature;
    float humidity;
    uint32_t timestamp;
};

// Rule of Five -- only when directly managing a raw resource
class DMABuffer {
public:
    explicit DMABuffer(size_t size)
        : data_(static_cast<uint8_t*>(heap_caps_malloc(size, MALLOC_CAP_DMA)))
        , size_(size) {
        if (!data_) ESP_LOGE("DMA", "Allocation failed");
    }
    ~DMABuffer() { free(data_); }
    DMABuffer(const DMABuffer&) = delete;
    DMABuffer& operator=(const DMABuffer&) = delete;
    DMABuffer(DMABuffer&& other) noexcept : data_(other.data_), size_(other.size_) {
        other.data_ = nullptr;
        other.size_ = 0;
    }
    DMABuffer& operator=(DMABuffer&& other) noexcept {
        if (this != &other) { free(data_); data_ = other.data_; size_ = other.size_; other.data_ = nullptr; other.size_ = 0; }
        return *this;
    }
private:
    uint8_t* data_;
    size_t size_;
};
```

## Key Rules from Industry Experts

| Rule | Why |
|---|---|
| Prefer `constexpr` over `const` | Compile-time guarantees, zero runtime cost |
| Never use `volatile` for synchronization | `volatile` is for hardware registers, not thread safety |
| Prefer scoped enums over unscoped | Prevent implicit conversions, name pollution |
| Use `[[nodiscard]]` on error-returning functions | Prevent ignored errors |
| Avoid `std::endl`, use `'\n'` | `std::endl` flushes, 10-100x slower |
| Initialize all variables at declaration | Prevent UB from uninitialized reads |
| Use `nullptr`, never `NULL` or `0` | Type safety |
| Prefer `using` over `typedef` | Templates, readability |
| Make interfaces hard to use incorrectly | Prevent bugs at compile time |
| Prefer composition over inheritance | Simpler, more flexible |
