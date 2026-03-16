# Design Patterns for ESP32 Firmware

## Architecture Patterns

### Event-Driven Architecture (Recommended Default)

Most ESP32 applications are best structured as event-driven systems:

```cpp
// Central event bus using ESP-IDF event loop
ESP_EVENT_DEFINE_BASE(SENSOR_EVENT);
ESP_EVENT_DEFINE_BASE(NETWORK_EVENT);
ESP_EVENT_DEFINE_BASE(APP_EVENT);

enum {
    SENSOR_EVENT_NEW_READING = 0,
    SENSOR_EVENT_ERROR,
};

enum {
    NETWORK_EVENT_CONNECTED = 0,
    NETWORK_EVENT_DISCONNECTED,
    NETWORK_EVENT_DATA_SENT,
};

// Post events from any task
SensorReading reading = read_sensor();
esp_event_post(SENSOR_EVENT, SENSOR_EVENT_NEW_READING,
               &reading, sizeof(reading), pdMS_TO_TICKS(100));

// Handle events in dedicated tasks
void display_event_handler(void* arg, esp_event_base_t base,
                           int32_t id, void* data) {
    if (base == SENSOR_EVENT && id == SENSOR_EVENT_NEW_READING) {
        auto* reading = static_cast<SensorReading*>(data);
        update_display(reading->temperature, reading->humidity);
    }
}
```

**Advantages**: Decoupled components, easy to add new features, testable.

### Producer-Consumer with Queues

For data pipeline patterns (sensor -> process -> transmit):

```cpp
struct SensorData {
    float values[4];
    uint32_t timestamp;
    uint8_t sensor_id;
};

QueueHandle_t raw_data_queue;     // Sensor -> Processor
QueueHandle_t processed_queue;    // Processor -> Transmitter

void sensor_task(void*) {
    raw_data_queue = xQueueCreate(20, sizeof(SensorData));
    while (true) {
        SensorData data = sample_sensors();
        if (xQueueSend(raw_data_queue, &data, pdMS_TO_TICKS(100)) != pdTRUE) {
            ESP_LOGW("SENSOR", "Queue full, dropping reading");
        }
        vTaskDelay(pdMS_TO_TICKS(100));  // 10Hz sampling
    }
}

void processor_task(void*) {
    SensorData raw;
    while (true) {
        if (xQueueReceive(raw_data_queue, &raw, portMAX_DELAY)) {
            ProcessedData result = apply_filter(raw);
            xQueueSend(processed_queue, &result, pdMS_TO_TICKS(100));
        }
    }
}
```

### Supervisor Pattern

For production reliability -- monitor and restart failed components:

```cpp
class Supervisor {
    struct MonitoredTask {
        const char* name;
        TaskFunction_t fn;
        void* param;
        uint32_t stack;
        UBaseType_t priority;
        TaskHandle_t handle;
        TickType_t last_heartbeat;
        uint32_t restart_count;
    };

    std::array<MonitoredTask, 8> tasks_;
    size_t task_count_ = 0;

public:
    void register_task(const char* name, TaskFunction_t fn, void* param,
                       uint32_t stack, UBaseType_t priority) {
        auto& t = tasks_[task_count_++];
        t = { name, fn, param, stack, priority, nullptr, xTaskGetTickCount(), 0 };
        xTaskCreate(fn, name, stack, param, priority, &t.handle);
    }

    void heartbeat(TaskHandle_t task) {
        for (auto& t : tasks_) {
            if (t.handle == task) {
                t.last_heartbeat = xTaskGetTickCount();
                break;
            }
        }
    }

    void check() {
        TickType_t now = xTaskGetTickCount();
        for (auto& t : tasks_) {
            if (t.handle && (now - t.last_heartbeat) > pdMS_TO_TICKS(30000)) {
                ESP_LOGW("SUPERVISOR", "Task '%s' unresponsive, restarting (count: %u)",
                         t.name, ++t.restart_count);
                vTaskDelete(t.handle);
                xTaskCreate(t.fn, t.name, t.stack, t.param, t.priority, &t.handle);
                t.last_heartbeat = now;
            }
        }
    }
};
```

## Hardware Abstraction Layer (HAL)

### Three-Layer Architecture
```
Application Layer    (business logic, state machines, protocols)
     |
Driver Layer         (device-specific: BME280, SSD1306, DRV8825)
     |
HAL Layer            (platform abstraction: I2C, SPI, GPIO, Timer)
```

### HAL Interface Example
```cpp
// hal/i2c_interface.h -- abstract interface
class II2C {
public:
    virtual ~II2C() = default;
    virtual esp_err_t write(uint8_t addr, const uint8_t* data, size_t len) = 0;
    virtual esp_err_t read(uint8_t addr, uint8_t* data, size_t len) = 0;
    virtual esp_err_t write_read(uint8_t addr, const uint8_t* tx, size_t tx_len,
                                  uint8_t* rx, size_t rx_len) = 0;
};

// hal/esp_i2c.h -- ESP32 implementation
class EspI2C : public II2C {
    i2c_master_bus_handle_t bus_;
    SemaphoreHandle_t mutex_;
public:
    explicit EspI2C(i2c_port_t port, gpio_num_t sda, gpio_num_t scl, uint32_t freq);
    esp_err_t write(uint8_t addr, const uint8_t* data, size_t len) override;
    esp_err_t read(uint8_t addr, uint8_t* data, size_t len) override;
    esp_err_t write_read(uint8_t addr, const uint8_t* tx, size_t tx_len,
                          uint8_t* rx, size_t rx_len) override;
};

// hal/mock_i2c.h -- mock for testing
class MockI2C : public II2C {
    std::vector<uint8_t> response_data_;
public:
    void set_response(std::vector<uint8_t> data) { response_data_ = data; }
    esp_err_t write(uint8_t, const uint8_t*, size_t) override { return ESP_OK; }
    esp_err_t read(uint8_t, uint8_t* data, size_t len) override {
        memcpy(data, response_data_.data(), std::min(len, response_data_.size()));
        return ESP_OK;
    }
    // ...
};
```

### Driver Using HAL
```cpp
class BME280 {
    II2C& i2c_;
    uint8_t addr_;
public:
    BME280(II2C& i2c, uint8_t addr = 0x76) : i2c_(i2c), addr_(addr) {}

    esp_err_t init() {
        uint8_t chip_id;
        uint8_t reg = 0xD0;
        auto err = i2c_.write_read(addr_, &reg, 1, &chip_id, 1);
        if (err != ESP_OK) return err;
        if (chip_id != 0x60) return ESP_ERR_NOT_FOUND;
        return configure_sensor();
    }
    // Testable without hardware!
};
```

## Double-Buffer Pattern

For data acquisition where sampling and processing must not block each other:

```cpp
template <typename T, size_t N>
class DoubleBuffer {
    std::array<T, N> buffer_a_, buffer_b_;
    std::array<T, N>* write_buf_ = &buffer_a_;
    std::array<T, N>* read_buf_ = &buffer_b_;
    SemaphoreHandle_t swap_mutex_;
    size_t write_index_ = 0;

public:
    DoubleBuffer() { swap_mutex_ = xSemaphoreCreateMutex(); }

    bool write(const T& item) {
        if (write_index_ >= N) return false;
        (*write_buf_)[write_index_++] = item;
        return true;
    }

    // Swap buffers -- writer gets empty buffer, reader gets full buffer
    void swap() {
        MutexLock lock(swap_mutex_);
        std::swap(write_buf_, read_buf_);
        write_index_ = 0;
    }

    const std::array<T, N>& read_buffer() const { return *read_buf_; }
};
```

## Configuration Management

```cpp
// Configuration with defaults and NVS persistence
struct DeviceConfig {
    char wifi_ssid[32] = "";
    char wifi_pass[64] = "";
    char mqtt_broker[128] = "mqtt.example.com";
    uint16_t mqtt_port = 8883;
    uint32_t sample_interval_ms = 1000;
    uint8_t log_level = ESP_LOG_INFO;
    bool ota_enabled = true;

    esp_err_t load_from_nvs() {
        nvs_handle_t handle;
        esp_err_t err = nvs_open("config", NVS_READONLY, &handle);
        if (err != ESP_OK) return err;

        size_t len;
        len = sizeof(wifi_ssid); nvs_get_str(handle, "wifi_ssid", wifi_ssid, &len);
        len = sizeof(wifi_pass); nvs_get_str(handle, "wifi_pass", wifi_pass, &len);
        len = sizeof(mqtt_broker); nvs_get_str(handle, "mqtt_broker", mqtt_broker, &len);
        nvs_get_u16(handle, "mqtt_port", &mqtt_port);
        nvs_get_u32(handle, "sample_ms", &sample_interval_ms);

        nvs_close(handle);
        return ESP_OK;
    }

    esp_err_t save_to_nvs() {
        nvs_handle_t handle;
        esp_err_t err = nvs_open("config", NVS_READWRITE, &handle);
        if (err != ESP_OK) return err;

        nvs_set_str(handle, "wifi_ssid", wifi_ssid);
        nvs_set_str(handle, "wifi_pass", wifi_pass);
        nvs_set_str(handle, "mqtt_broker", mqtt_broker);
        nvs_set_u16(handle, "mqtt_port", mqtt_port);
        nvs_set_u32(handle, "sample_ms", sample_interval_ms);
        nvs_commit(handle);

        nvs_close(handle);
        return ESP_OK;
    }
};
```

## Component Interaction Patterns

### Preferred: Explicit Dependencies (Dependency Injection)
```cpp
// Application wiring in app_main
extern "C" void app_main(void) {
    auto i2c = std::make_unique<EspI2C>(I2C_NUM_0, GPIO_NUM_21, GPIO_NUM_22, 400000);
    auto sensor = std::make_unique<BME280>(*i2c);
    auto display = std::make_unique<SSD1306>(*i2c);
    auto wifi = std::make_unique<WiFiManager>();

    sensor->init();
    display->init();
    wifi->connect();

    // Pass dependencies to tasks
    xTaskCreate(sensor_task, "sensor", 4096, sensor.get(), 2, nullptr);
    xTaskCreate(display_task, "display", 4096, display.get(), 1, nullptr);

    // Keep main alive (or use vTaskStartScheduler equivalent)
    vTaskDelay(portMAX_DELAY);
}
```

### Avoid: Global Singletons
```cpp
// BAD: Hidden dependencies, untestable, initialization order issues
class SensorManager {
    static SensorManager& instance() {
        static SensorManager inst;  // When is this constructed?
        return inst;
    }
};

// GOOD: Explicit construction and ownership as shown above
```
