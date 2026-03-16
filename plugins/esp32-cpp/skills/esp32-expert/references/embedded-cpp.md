# Embedded C++ Patterns for ESP32

## Static Polymorphism (CRTP)

Virtual functions cost ~4 bytes per vtable pointer and an indirect call. For hot paths, use CRTP:

```cpp
// Zero-cost abstraction for drivers
template <typename Derived>
class SensorBase {
public:
    float read() { return static_cast<Derived*>(this)->read_impl(); }
    void init() { static_cast<Derived*>(this)->init_impl(); }
};

class BME280 : public SensorBase<BME280> {
    friend class SensorBase<BME280>;
    float read_impl() { /* actual I2C read */ }
    void init_impl() { /* actual init */ }
};

// Compiler inlines everything -- zero overhead vs hand-written C
```

Use virtual functions for top-level components (few instances, rare calls). Use CRTP or templates for drivers in tight loops.

## Type-Safe Hardware Access

### Pin Configuration
```cpp
// BAD: Magic numbers
gpio_set_level(2, 1);
gpio_set_level(4, 0);

// GOOD: Type-safe pins
enum class Pin : gpio_num_t {
    LED = GPIO_NUM_2,
    RELAY = GPIO_NUM_4,
    BUTTON = GPIO_NUM_15,
    I2C_SDA = GPIO_NUM_21,
    I2C_SCL = GPIO_NUM_22,
};

constexpr gpio_num_t to_gpio(Pin p) { return static_cast<gpio_num_t>(p); }

gpio_set_level(to_gpio(Pin::LED), 1);
```

### Register Access
```cpp
// Type-safe register access (compile-time checked)
template <uint32_t Address, typename T = uint32_t>
struct Register {
    static volatile T& ref() {
        return *reinterpret_cast<volatile T*>(Address);
    }
    static T read() { return ref(); }
    static void write(T val) { ref() = val; }
    static void set_bits(T mask) { ref() |= mask; }
    static void clear_bits(T mask) { ref() &= ~mask; }
};
```

## Interrupt-Safe Patterns

### Deferred Processing
```cpp
class DeferredHandler {
public:
    DeferredHandler(const char* name, uint32_t stack, UBaseType_t priority) {
        sem_ = xSemaphoreCreateBinary();
        xTaskCreate(task_fn, name, stack, this, priority, &task_);
    }

    // Called from ISR
    void IRAM_ATTR trigger_from_isr() {
        BaseType_t woken = pdFALSE;
        xSemaphoreGiveFromISR(sem_, &woken);
        portYIELD_FROM_ISR(woken);
    }

protected:
    virtual void handle() = 0;

private:
    SemaphoreHandle_t sem_;
    TaskHandle_t task_;

    static void task_fn(void* arg) {
        auto* self = static_cast<DeferredHandler*>(arg);
        while (true) {
            if (xSemaphoreTake(self->sem_, portMAX_DELAY)) {
                self->handle();
            }
        }
    }
};
```

## State Machine Patterns

### Table-Driven State Machine
```cpp
enum class State { IDLE, CONNECTING, CONNECTED, ERROR };
enum class Event { START, CONNECTED, DISCONNECTED, TIMEOUT, RESET };

struct Transition {
    State current;
    Event event;
    State next;
    void (*action)();
};

constexpr Transition transitions[] = {
    { State::IDLE,       Event::START,        State::CONNECTING, &start_connect },
    { State::CONNECTING, Event::CONNECTED,    State::CONNECTED,  &on_connected },
    { State::CONNECTING, Event::TIMEOUT,      State::ERROR,      &on_timeout },
    { State::CONNECTED,  Event::DISCONNECTED, State::CONNECTING, &start_reconnect },
    { State::ERROR,      Event::RESET,        State::IDLE,       &reset_state },
};

class StateMachine {
    State state_ = State::IDLE;
public:
    void process(Event event) {
        for (const auto& t : transitions) {
            if (t.current == state_ && t.event == event) {
                ESP_LOGI("SM", "Transition: %d -> %d", (int)state_, (int)t.next);
                state_ = t.next;
                if (t.action) t.action();
                return;
            }
        }
        ESP_LOGW("SM", "No transition for state=%d event=%d", (int)state_, (int)event);
    }
};
```

### std::variant State Machine (Modern C++)
```cpp
struct Idle {};
struct Connecting { uint32_t retry_count; TickType_t started_at; };
struct Connected { uint32_t uptime_ms; };
struct Error { esp_err_t code; const char* message; };

using DeviceState = std::variant<Idle, Connecting, Connected, Error>;

// Pattern matching with std::visit
void log_state(const DeviceState& state) {
    std::visit([](auto&& s) {
        using T = std::decay_t<decltype(s)>;
        if constexpr (std::is_same_v<T, Idle>) ESP_LOGI("STATE", "Idle");
        else if constexpr (std::is_same_v<T, Connecting>) ESP_LOGI("STATE", "Connecting (retry %u)", s.retry_count);
        else if constexpr (std::is_same_v<T, Connected>) ESP_LOGI("STATE", "Connected (%u ms)", s.uptime_ms);
        else if constexpr (std::is_same_v<T, Error>) ESP_LOGE("STATE", "Error: %s", s.message);
    }, state);
}
```

## Fixed-Size Containers

When `std::vector` heap allocation is unacceptable:

```cpp
// Fixed-capacity ring buffer (no heap)
template <typename T, size_t N>
class RingBuffer {
    std::array<T, N> buf_{};
    size_t head_ = 0, tail_ = 0, count_ = 0;
public:
    bool push(const T& item) {
        if (count_ >= N) return false;
        buf_[head_] = item;
        head_ = (head_ + 1) % N;
        ++count_;
        return true;
    }
    bool pop(T& item) {
        if (count_ == 0) return false;
        item = buf_[tail_];
        tail_ = (tail_ + 1) % N;
        --count_;
        return true;
    }
    size_t size() const { return count_; }
    bool empty() const { return count_ == 0; }
    bool full() const { return count_ >= N; }
};
```

## Callback Patterns (Avoiding std::function Overhead)

```cpp
// Template callback (zero overhead, no heap)
template <typename Callback>
void on_timer_expire(uint32_t period_ms, Callback&& cb) {
    // Timer setup with cb as the action
}

// Function pointer + context (C-compatible, used by ESP-IDF)
struct CallbackContext {
    void (*fn)(void* arg);
    void* arg;
};

// Type-erased callback with small buffer optimization
template <size_t BufferSize = 32>
class Delegate {
    alignas(void*) uint8_t buffer_[BufferSize];
    void (*invoker_)(void*) = nullptr;
    void (*destructor_)(void*) = nullptr;
public:
    template <typename F>
    Delegate(F&& f) {
        static_assert(sizeof(F) <= BufferSize, "Callable too large for inline storage");
        new (buffer_) F(std::forward<F>(f));
        invoker_ = [](void* buf) { (*static_cast<F*>(buf))(); };
        destructor_ = [](void* buf) { static_cast<F*>(buf)->~F(); };
    }
    void operator()() { if (invoker_) invoker_(buffer_); }
    ~Delegate() { if (destructor_) destructor_(buffer_); }
};
```

## Compile-Time Configuration

```cpp
// Board configuration as compile-time constants
namespace board {
    constexpr gpio_num_t LED = GPIO_NUM_2;
    constexpr gpio_num_t BUTTON = GPIO_NUM_0;
    constexpr i2c_port_t I2C_PORT = I2C_NUM_0;
    constexpr gpio_num_t I2C_SDA = GPIO_NUM_21;
    constexpr gpio_num_t I2C_SCL = GPIO_NUM_22;
    constexpr uint32_t I2C_FREQ = 400000;

    // Variant-specific configuration
    #if CONFIG_IDF_TARGET_ESP32
        constexpr int CPU_FREQ_MHZ = 240;
        constexpr size_t INTERNAL_RAM_KB = 328;
    #elif CONFIG_IDF_TARGET_ESP32S3
        constexpr int CPU_FREQ_MHZ = 240;
        constexpr size_t INTERNAL_RAM_KB = 512;
    #elif CONFIG_IDF_TARGET_ESP32C3
        constexpr int CPU_FREQ_MHZ = 160;
        constexpr size_t INTERNAL_RAM_KB = 400;
    #elif CONFIG_IDF_TARGET_ESP32P4
        constexpr int CPU_FREQ_MHZ = 400;  // Max; typical boards run at 360MHz
        constexpr size_t INTERNAL_RAM_KB = 768;  // Includes L2 cache
    #endif
}
```

## volatile: Correct Usage

`volatile` is ONLY for:
1. Memory-mapped hardware registers
2. Variables modified by ISRs and read by tasks (with caveats)
3. Variables modified by DMA

`volatile` is NOT for:
- Thread synchronization (use mutexes, atomics, or FreeRTOS primitives)
- Preventing compiler optimizations you don't understand

```cpp
// CORRECT: ISR flag
volatile bool isr_flag = false;  // Modified in ISR, read in task
// BUT: For anything more complex, use xTaskNotify or a queue

// CORRECT: Hardware register
volatile uint32_t* const GPIO_OUT_REG = reinterpret_cast<volatile uint32_t*>(0x3FF44004);

// WRONG: Thread synchronization
volatile int shared_counter = 0;  // NOT thread-safe! Use std::atomic<int>
```

## Error Propagation Without Exceptions

```cpp
// Result type pattern (inspired by Rust)
template <typename T>
struct Result {
    T value;
    esp_err_t error;

    bool ok() const { return error == ESP_OK; }
    explicit operator bool() const { return ok(); }

    static Result success(T val) { return {val, ESP_OK}; }
    static Result fail(esp_err_t err) { return {T{}, err}; }
};

// Usage
Result<float> read_temperature() {
    uint8_t data[2];
    esp_err_t err = i2c_read(I2C_NUM_0, SENSOR_ADDR, data, 2);
    if (err != ESP_OK) return Result<float>::fail(err);
    float temp = (data[0] << 8 | data[1]) * 0.01f;
    return Result<float>::success(temp);
}

auto result = read_temperature();
if (result) {
    ESP_LOGI("TEMP", "%.2f C", result.value);
} else {
    ESP_LOGE("TEMP", "Read failed: %s", esp_err_to_name(result.error));
}
```
