# Anti-Patterns Catalog: ESP32 C++ / FreeRTOS / Embedded

Every anti-pattern listed here has caused real field failures, bricked devices, security breaches, or hours of debugging. Spot these patterns aggressively during code review.

## Critical: Will Crash or Corrupt (Fix Immediately)

### AP-01: Blocking in ISR

**BAD:**
```cpp
void IRAM_ATTR button_isr(void* arg) {
    ESP_LOGI("BTN", "Button pressed!");  // CRASH: ESP_LOG is not ISR-safe
    vTaskDelay(pdMS_TO_TICKS(50));       // CRASH: Cannot block in ISR
    xSemaphoreTake(mutex, portMAX_DELAY); // CRASH: Mutex not ISR-safe
}
```

**GOOD:**
```cpp
void IRAM_ATTR button_isr(void* arg) {
    BaseType_t woken = pdFALSE;
    xTaskNotifyFromISR(button_task, 1, eSetBits, &woken);
    portYIELD_FROM_ISR(woken);
}
```
**Consequence:** Watchdog reset, missed interrupts, undefined behavior. Device reboots in the field.

### AP-02: Missing IRAM_ATTR on ISR Handlers

**BAD:**
```cpp
void gpio_handler(void* arg) {  // Missing IRAM_ATTR!
    // This code is in flash. If flash is being written (OTA/NVS), CRASH.
}
```

**GOOD:**
```cpp
void IRAM_ATTR gpio_handler(void* arg) {
    // Safe: runs from IRAM, available even during flash operations
}
```
**Consequence:** Intermittent crashes during NVS writes or OTA updates. Extremely hard to reproduce.

### AP-03: Stack-Allocated Large Buffers in Tasks

**BAD:**
```cpp
void http_task(void* param) {
    char response[8192];  // 8KB on stack! Task stack is typically 4-8KB total
    char json[4096];      // Another 4KB!
    // ...
}
```

**GOOD:**
```cpp
void http_task(void* param) {
    auto* response = static_cast<char*>(heap_caps_malloc(8192, MALLOC_CAP_SPIRAM));
    if (!response) { ESP_LOGE("HTTP", "OOM"); return; }
    // Use response...
    free(response);
}
// Or use static buffers if only one instance exists
static char response_buf[8192];
```
**Consequence:** Stack overflow, Guru Meditation Error. Device reboot loop in the field.

### AP-04: DMA Buffer in PSRAM or Flash

**BAD:**
```cpp
// PSRAM buffer for SPI DMA -- WILL FAIL
uint8_t* buf = (uint8_t*)heap_caps_malloc(1024, MALLOC_CAP_SPIRAM);
spi_transaction_t t = { .tx_buffer = buf, .length = 1024 * 8 };
spi_device_transmit(spi, &t);  // DMA cannot access PSRAM!
```

**GOOD:**
```cpp
uint8_t* buf = (uint8_t*)heap_caps_malloc(1024, MALLOC_CAP_DMA);
// Or: DRAM_ATTR static uint8_t buf[1024];
```
**Consequence:** Silent data corruption, garbled SPI/I2S transfers, display glitches.

### AP-05: Shared Bus Without Mutex

**BAD:**
```cpp
// Task A: reads temperature sensor on I2C
void sensor_task(void*) {
    while (true) {
        i2c_master_transmit_receive(sensor_handle, &reg, 1, data, 2, 100);
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}

// Task B: updates display on SAME I2C bus
void display_task(void*) {
    while (true) {
        i2c_master_transmit(display_handle, frame, sizeof(frame), 100);
        vTaskDelay(pdMS_TO_TICKS(16));
    }
}
// If both tasks preempt each other mid-transaction: BUS CORRUPTION
```

**GOOD:**
```cpp
SemaphoreHandle_t i2c_mutex = xSemaphoreCreateMutex();

// Both tasks acquire mutex before I2C access
MutexLock lock(i2c_mutex, pdMS_TO_TICKS(1000));
if (lock) {
    i2c_master_transmit_receive(sensor_handle, &reg, 1, data, 2, 100);
}
```
**Consequence:** Garbled data, phantom sensor readings, display corruption, I2C bus lockup.

### AP-06: Ignoring esp_err_t Return Values

**BAD:**
```cpp
i2c_driver_install(I2C_NUM_0, I2C_MODE_MASTER, 0, 0, 0);
gpio_config(&io_conf);
esp_wifi_start();
// If any of these fail, subsequent code crashes with no clue why
```

**GOOD:**
```cpp
ESP_ERROR_CHECK(i2c_driver_install(I2C_NUM_0, I2C_MODE_MASTER, 0, 0, 0));
// Or for recoverable errors:
esp_err_t err = esp_wifi_start();
if (err != ESP_OK) {
    ESP_LOGE("WIFI", "Start failed: %s", esp_err_to_name(err));
    enter_recovery_mode();
}
```
**Consequence:** Silent cascading failures. Device appears to work but subtly misbehaves.

## Important: Will Fail Eventually (Should Fix)

### AP-07: malloc/new in a Loop Without Free

**BAD:**
```cpp
void process_messages(void*) {
    while (true) {
        char* msg = (char*)malloc(256);
        receive_message(msg, 256);
        process(msg);
        // Forgot to free(msg)! Heap exhaustion in hours/days.
    }
}
```

**GOOD:**
```cpp
void process_messages(void*) {
    char msg[256];  // Stack or static buffer for fixed-size messages
    while (true) {
        receive_message(msg, sizeof(msg));
        process(msg);
    }
}
```
**Consequence:** Heap exhaustion after hours/days of operation. Device crashes in the field.

### AP-08: No Reconnection Logic

**BAD:**
```cpp
void wifi_event_handler(void* arg, esp_event_base_t base, int32_t id, void* data) {
    if (id == WIFI_EVENT_STA_DISCONNECTED) {
        ESP_LOGW("WIFI", "Disconnected");
        // Does nothing. Device is now offline PERMANENTLY.
    }
}
```

**GOOD:**
```cpp
if (id == WIFI_EVENT_STA_DISCONNECTED) {
    if (retry_count < MAX_RETRIES) {
        vTaskDelay(pdMS_TO_TICKS(backoff_ms));
        esp_wifi_connect();
        backoff_ms = std::min(backoff_ms * 2, 30000u);
        retry_count++;
    } else {
        ESP_LOGE("WIFI", "Max retries, entering AP mode for provisioning");
        start_provisioning_ap();
    }
}
```
**Consequence:** Device goes offline permanently after a transient WiFi issue.

### AP-09: Using volatile for Thread Synchronization

**BAD:**
```cpp
volatile bool data_ready = false;
volatile int shared_data = 0;

void producer(void*) {
    shared_data = 42;
    data_ready = true;  // No memory barrier! Consumer may see stale shared_data
}

void consumer(void*) {
    while (!data_ready) taskYIELD();
    int value = shared_data;  // May read 0, not 42!
}
```

**GOOD:**
```cpp
// Use FreeRTOS primitives (they include memory barriers)
QueueHandle_t data_queue = xQueueCreate(1, sizeof(int));

void producer(void*) {
    int data = 42;
    xQueueSend(data_queue, &data, portMAX_DELAY);
}

void consumer(void*) {
    int data;
    xQueueReceive(data_queue, &data, portMAX_DELAY);
    // data is guaranteed to be 42
}
```
**Consequence:** Data races, stale reads, heisenbugs that appear under load.

### AP-10: Watchdog Feeding Without Checking Work

**BAD:**
```cpp
void main_task(void*) {
    esp_task_wdt_add(nullptr);
    while (true) {
        esp_task_wdt_reset();  // Feed unconditionally at top
        if (!do_work()) {
            // Work failed but watchdog is happy -- device appears alive but broken
            continue;
        }
    }
}
```

**GOOD:**
```cpp
void main_task(void*) {
    esp_task_wdt_add(nullptr);
    while (true) {
        if (do_work()) {
            esp_task_wdt_reset();  // Only feed after successful work
        } else {
            ESP_LOGW("WDT", "Work failed, not feeding watchdog");
            // Watchdog will reset device if work keeps failing
        }
    }
}
```
**Consequence:** Device appears alive (watchdog happy) but is actually stuck. Silent failure.

### AP-11: `delay()` in Arduino Context Instead of vTaskDelay

**BAD (Arduino framework):**
```cpp
void loop() {
    read_sensors();
    delay(1000);  // Blocks the entire FreeRTOS scheduler! No other task runs.
}
```

**GOOD:**
```cpp
void loop() {
    read_sensors();
    vTaskDelay(pdMS_TO_TICKS(1000));  // Yields to other tasks
}
```
**Consequence:** Lower-priority tasks starved, WiFi stack blocked, watchdog resets.

### AP-12: printf/String Formatting in Production Firmware

**BAD:**
```cpp
void sensor_loop(void*) {
    while (true) {
        float temp = read_temp();
        char buf[64];
        snprintf(buf, sizeof(buf), "Temperature: %.2f", temp);
        ESP_LOGI("SENSOR", "%s", buf);
        // 1KB+ stack for printf, UART blocks CPU, wastes flash
    }
}
```

**GOOD:**
```cpp
// Set production log level to WARN
// In sdkconfig.defaults: CONFIG_LOG_DEFAULT_LEVEL_WARN=y
// Or at runtime:
esp_log_level_set("SENSOR", ESP_LOG_WARN);
```
**Consequence:** Wastes CPU (UART is slow), consumes stack, fills serial buffer.

### AP-13: Hard-Coded Pin Numbers Without Variant Guards

**BAD:**
```cpp
#define LED_PIN 2
#define SDA_PIN 21
#define SCL_PIN 22
// These pins are wrong on ESP32-C3, ESP32-S2, ESP32-P4...
```

**GOOD:**
```cpp
#if CONFIG_IDF_TARGET_ESP32
    constexpr gpio_num_t LED_PIN = GPIO_NUM_2;
    constexpr gpio_num_t SDA_PIN = GPIO_NUM_21;
    constexpr gpio_num_t SCL_PIN = GPIO_NUM_22;
#elif CONFIG_IDF_TARGET_ESP32S3
    constexpr gpio_num_t LED_PIN = GPIO_NUM_48;
    constexpr gpio_num_t SDA_PIN = GPIO_NUM_1;
    constexpr gpio_num_t SCL_PIN = GPIO_NUM_2;
#elif CONFIG_IDF_TARGET_ESP32C3
    constexpr gpio_num_t LED_PIN = GPIO_NUM_8;
    constexpr gpio_num_t SDA_PIN = GPIO_NUM_5;
    constexpr gpio_num_t SCL_PIN = GPIO_NUM_6;
#else
    #error "Define pins for this target"
#endif
```
**Consequence:** Code that works on one board silently fails on another variant.

### AP-14: Using Binary Semaphore for Resource Protection

**BAD:**
```cpp
SemaphoreHandle_t spi_lock = xSemaphoreCreateBinary();
xSemaphoreGive(spi_lock);  // Initialize to "available"
// Binary semaphore has NO priority inheritance!
```

**GOOD:**
```cpp
SemaphoreHandle_t spi_lock = xSemaphoreCreateMutex();
// Mutex has priority inheritance -- prevents priority inversion
```
**Consequence:** Priority inversion. High-priority task blocked by low-priority task holding the resource.

### AP-15: Heap Fragmentation Time Bomb

**BAD:**
```cpp
void process_request(void*) {
    while (true) {
        // Different-sized allocations over time
        char* small = (char*)malloc(32);
        char* big = (char*)malloc(2048);
        char* medium = (char*)malloc(256);
        free(small);
        process(big, medium);
        free(big);
        free(medium);
        // After weeks: malloc(2048) fails even with 10KB "free"
    }
}
```

**GOOD:**
```cpp
// Pre-allocate fixed-size pools or allocate once at startup
static char request_buffer[2048];
static char response_buffer[2048];

// Or use fixed-size block allocator
template <size_t BlockSize, size_t BlockCount>
class MemoryPool { /* ... */ };
```
**Consequence:** `malloc` returns `NULL` after days/weeks despite "available" heap. Device crashes.

## Security Anti-Patterns

### AP-16: Hardcoded Credentials

**BAD:**
```cpp
#define WIFI_SSID "MyNetwork"
#define WIFI_PASS "SuperSecret123"
#define API_KEY "sk-1234567890abcdef"
```

**GOOD:**
```cpp
// Store in NVS, provision via BLE/AP mode
nvs_get_str(handle, "wifi_ssid", ssid, &len);
```
**Consequence:** Credentials extractable from firmware binary. Security breach.

### AP-17: No TLS Certificate Verification

**BAD:**
```cpp
esp_http_client_config_t config = {};
config.url = "https://api.example.com";
// No cert_pem = no verification = accepts ANY certificate
```
**Consequence:** Man-in-the-middle attack. Attacker intercepts all data.

### AP-18: OTA Without Signature Verification

**BAD:**
```cpp
esp_http_client_config_t ota_cfg = {};
ota_cfg.url = "http://server/firmware.bin";  // HTTP, not HTTPS!
// No signature check. Anyone who can DNS-spoof can flash arbitrary firmware.
```
**Consequence:** Malicious firmware injection. Complete device compromise.

## C++ Specific Anti-Patterns

### AP-19: std::function for Callbacks (Hidden Heap Allocation)

**BAD:**
```cpp
using Callback = std::function<void(int)>;
void set_callback(Callback cb);  // Allocates if lambda captures > ~24 bytes
```

**GOOD:**
```cpp
template <typename F>
void set_callback(F&& cb);  // Zero overhead, inlined
// Or C-style function pointer + void* context
```
**Consequence:** Heap allocation per callback, fragmentation in long-running systems.

### AP-20: Global Constructors with Side Effects

**BAD:**
```cpp
// File scope -- construction order undefined across translation units!
I2CMaster i2c_bus(I2C_NUM_0, config);  // Calls i2c_driver_install at file scope
BME280 sensor(i2c_bus, 0x76);           // Depends on i2c_bus being ready
```

**GOOD:**
```cpp
// Construct in app_main with explicit order
extern "C" void app_main(void) {
    auto i2c_bus = std::make_unique<I2CMaster>(I2C_NUM_0, config);
    auto sensor = std::make_unique<BME280>(*i2c_bus, 0x76);
}
```
**Consequence:** Undefined initialization order = random crashes on some builds/compilers.

### AP-21: Using C-Style Casts

**BAD:**
```cpp
void* param = (void*)&my_struct;     // C cast hides errors
int value = (int)float_value;         // Truncation hidden
uint8_t* p = (uint8_t*)big_struct;   // Byte access -- reinterpret_cast
```

**GOOD:**
```cpp
void* param = static_cast<void*>(&my_struct);
int value = static_cast<int>(float_value);
auto* p = reinterpret_cast<uint8_t*>(&big_struct);
```
**Consequence:** Hides dangerous conversions that C++ casts make explicit and searchable.

### AP-22: Unprotected Shared State Between Tasks

**BAD:**
```cpp
struct SharedState {
    float temperature;
    float humidity;
    uint32_t timestamp;
    bool valid;
};
SharedState state;  // Read and written from multiple tasks with no protection

void sensor_task(void*) {
    state.temperature = read_temp();
    state.humidity = read_hum();
    state.timestamp = esp_timer_get_time();
    state.valid = true;  // Consumer may read partially-updated struct
}
```

**GOOD:**
```cpp
SemaphoreHandle_t state_mutex = xSemaphoreCreateMutex();

void sensor_task(void*) {
    SharedState local;
    local.temperature = read_temp();
    local.humidity = read_hum();
    local.timestamp = esp_timer_get_time();
    local.valid = true;
    // Atomic update
    MutexLock lock(state_mutex);
    if (lock) state = local;
}
```
**Consequence:** Torn reads -- consumer sees temperature from one sample, humidity from another.

## ESP32-Specific Anti-Patterns (Hardware/Silicon)

### AP-23: ADC2 Pins While WiFi Is Active

**BAD:**
```cpp
// GPIO27 = ADC2_CHANNEL_7 -- shared with WiFi RF on ESP32 (original)
adc2_get_raw(ADC2_CHANNEL_7, ADC_WIDTH_BIT_12, &raw);
// Returns garbage (0 or 4095) or crashes when WiFi is active
```

**GOOD:**
```cpp
// Use ADC1 exclusively when WiFi is active (GPIO32-39)
adc1_config_channel_atten(ADC1_CHANNEL_6, ADC_ATTEN_DB_12); // GPIO34
int raw = adc1_get_raw(ADC1_CHANNEL_6);
```
**Consequence:** Garbage ADC readings (0 or 4095 constantly) or `ESP_ERR_TIMEOUT`. The WiFi driver and ADC2 share hardware resources; when WiFi is active, the driver cannot acquire ADC2. This is a driver-level resource conflict on ESP32, improved but not eliminated on S2/S3. ADC2 channels: GPIO0, 2, 4, 12-15, 25-27.

### AP-24: GPIO Strapping Pins for General I/O

**BAD:**
```cpp
// GPIO12 drives a relay -- at power-on, relay may pull GPIO12 HIGH
// Boot ROM samples GPIO12 → selects 1.8V flash voltage → IMMEDIATE CRASH
gpio_set_direction(GPIO_NUM_12, GPIO_MODE_OUTPUT);
```

**GOOD:**
```cpp
// Avoid GPIO0, GPIO2, GPIO12, GPIO15 for outputs that could be driven at boot
// If unavoidable: add series 10K resistor so internal pull wins at boot
// For GPIO12: burn VDD_SDIO eFuse to 3.3V permanently:
//   esptool.py burn_efuse XPD_SDIO_TIEH
// Use safe pins: GPIO4, GPIO18, GPIO19, GPIO21, GPIO22, GPIO23
gpio_set_direction(GPIO_NUM_21, GPIO_MODE_OUTPUT);
```
**Consequence:** Device fails to boot, enters download mode, or selects wrong flash voltage causing brick. Per-variant strapping pins: ESP32 (GPIO0/2/12/15), ESP32-S3 (GPIO0/3/45/46), ESP32-C3 (GPIO2/8/9).

### AP-25: Wrong Initialization Order

**BAD:**
```cpp
void app_main(void) {
    esp_wifi_init(&cfg);  // NVS not initialized → RF calibration data missing
    esp_wifi_start();     // Event loop not created → events lost silently
}
```

**GOOD:**
```cpp
void app_main(void) {
    // MANDATORY order -- each step depends on the previous
    ESP_ERROR_CHECK(nvs_flash_init());           // 1. NVS first (WiFi needs it)
    ESP_ERROR_CHECK(esp_netif_init());           // 2. TCP/IP stack
    ESP_ERROR_CHECK(esp_event_loop_create_default()); // 3. Event loop
    esp_netif_create_default_wifi_sta();          // 4. Network interface
    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));         // 5. WiFi driver
    // 6. Register handlers BEFORE esp_wifi_start()
    esp_event_handler_instance_register(WIFI_EVENT, ESP_EVENT_ANY_ID, &handler, NULL, NULL);
    ESP_ERROR_CHECK(esp_wifi_start());            // 7. Start
}
```
**Consequence:** "Failed to load RF calibration data", events silently lost, panic on `esp_wifi_init()`. The most common ESP32 beginner bug per Espressif forums.

### AP-26: Opening Sockets Before IP_EVENT_STA_GOT_IP

**BAD:**
```cpp
void wifi_handler(void* arg, esp_event_base_t base, int32_t id, void* data) {
    if (id == WIFI_EVENT_STA_CONNECTED) {
        // WRONG: DHCP not complete yet! Socket will fail.
        int sock = socket(AF_INET, SOCK_STREAM, 0);
        connect(sock, ...);
    }
}
```

**GOOD:**
```cpp
void ip_handler(void* arg, esp_event_base_t base, int32_t id, void* data) {
    if (id == IP_EVENT_STA_GOT_IP) {
        // NOW safe to create sockets -- IP is assigned
        xTaskCreate(tcp_client_task, "tcp", 8192, NULL, 5, NULL);
    }
}
// On WIFI_EVENT_STA_DISCONNECTED: close ALL existing sockets immediately
// They are permanently invalid after disconnect, even after reconnect
```
**Consequence:** Connection failures, zombie sockets that block forever. Sockets created before IP assignment are invalid. Sockets from a previous connection are permanently broken after disconnect.

### AP-27: MALLOC_CAP_32BIT for Floating-Point Variables

**BAD:**
```cpp
// MALLOC_CAP_32BIT may return IRAM -- FPU cannot access IRAM on ESP32
float* data = (float*)heap_caps_malloc(1024 * sizeof(float), MALLOC_CAP_32BIT);
data[0] = 3.14f;  // CRASH: LoadStoreError -- FPU instructions can't reach IRAM
```

**GOOD:**
```cpp
// MALLOC_CAP_8BIT | MALLOC_CAP_INTERNAL guarantees DRAM (FPU-accessible)
float* data = (float*)heap_caps_malloc(1024 * sizeof(float),
    MALLOC_CAP_8BIT | MALLOC_CAP_INTERNAL);
```
**Consequence:** Fatal `LoadStoreError` exception. The ESP32 Xtensa FPU can only access DRAM, not IRAM. This is a documented hardware limitation.

### AP-28: taskDISABLE_INTERRUPTS for SMP Mutual Exclusion

**BAD:**
```cpp
// On dual-core ESP32: disabling interrupts on Core 0 does NOTHING to Core 1
taskDISABLE_INTERRUPTS();
shared_counter++;  // Core 1 can still read/write shared_counter simultaneously
taskENABLE_INTERRUPTS();
```

**GOOD:**
```cpp
// Use spinlock for true cross-core mutual exclusion
static portMUX_TYPE lock = portMUX_INITIALIZER_UNLOCKED;
portENTER_CRITICAL(&lock);  // Acquires spinlock across BOTH cores
shared_counter++;
portEXIT_CRITICAL(&lock);

// Or use a FreeRTOS mutex for longer operations
xSemaphoreTake(mutex, portMAX_DELAY);
shared_counter++;
xSemaphoreGive(mutex);
```
**Consequence:** Data race on dual-core ESP32/S3/P4. Works correctly on single-core ESP32-C3/C6 but breaks silently when ported to dual-core.

### AP-29: NVS Writes on Every Boot Cycle

**BAD:**
```cpp
void app_main(void) {
    nvs_open("config", NVS_READWRITE, &handle);
    nvs_set_str(handle, "ssid", WIFI_SSID);  // Written every single boot
    nvs_set_str(handle, "pass", WIFI_PASS);  // Flash rated ~100K write cycles
    nvs_commit(handle);
}
```

**GOOD:**
```cpp
char stored[64] = {};
size_t len = sizeof(stored);
esp_err_t err = nvs_get_str(handle, "ssid", stored, &len);
// Only write if value changed or was never set
if (err == ESP_ERR_NVS_NOT_FOUND || strcmp(stored, WIFI_SSID) != 0) {
    nvs_set_str(handle, "ssid", WIFI_SSID);
    nvs_commit(handle);
}
```
**Consequence:** Flash wear-out after months/years. Device that boots 50x/day reaches flash end-of-life in ~5 years. Silent NVS data corruption precedes total failure.

### AP-30: Wake Stub Calling Flash-Resident Code

**BAD:**
```cpp
void esp_wake_deep_sleep(void) {
    ESP_LOGI(TAG, "Woke up");  // ESP_LOGI is in flash → crash (cache not mapped yet)
    configure_gpio();           // Function in flash → LoadProhibited
}
```

**GOOD:**
```cpp
RTC_IRAM_ATTR void esp_wake_deep_sleep(void) {
    esp_default_wake_deep_sleep();  // Required (especially on rev 0 silicon)
    // Only ROM functions and RTC-resident code are safe here
    REG_WRITE(GPIO_OUT_W1TS_REG, BIT(LED_PIN));  // Direct register write -- safe
}
```
**Consequence:** `LoadProhibited` crash on every wake from deep sleep. Flash is not mapped when the wake stub runs. All code and data must be in RTC memory or ROM.

### AP-31: PSRAM Without Cache Workaround on ESP32 Rev 0/1

**BAD:**
```cpp
// ESP32 rev 0/1 has a silicon bug: PSRAM access + interrupt can corrupt data
// Missing: -mfix-esp32-psram-cache-issue compiler flag
// Result: random bytes in PSRAM silently flip to zero
```

**GOOD:**
```cpp
// ESP-IDF handles this automatically when PSRAM is enabled in menuconfig
// PlatformIO users MUST add manually:
// build_flags = -mfix-esp32-psram-cache-issue
// ESP32 rev 3.0+ is NOT affected
// Check revision: espefuse.py chip_id
```
**Consequence:** Random, non-reproducible data corruption in PSRAM. Bytes silently become zero. Appears as intermittent sensor glitches, garbled images, or corrupted data structures. Months of debugging if the workaround is missing.

### AP-32: vTaskDelay for Periodic Tasks (Cumulative Drift)

**BAD:**
```cpp
void control_loop(void*) {
    while (true) {
        read_sensor();
        compute_pid();
        vTaskDelay(pdMS_TO_TICKS(10));  // 10ms AFTER work completes → period drifts
    }
}
```

**GOOD:**
```cpp
void control_loop(void*) {
    TickType_t last_wake = xTaskGetTickCount();
    while (true) {
        read_sensor();
        compute_pid();
        vTaskDelayUntil(&last_wake, pdMS_TO_TICKS(10));  // Absolute 10ms period
    }
}
```
**Consequence:** Period = work_time + delay_time, growing under load. A 10ms control loop becomes 12ms, then 15ms. Motor control jitters, sensor sampling drifts, protocol timing violations.

### AP-33: ESP_ERROR_CHECK in Runtime Recovery Paths

**BAD:**
```cpp
while (true) {
    ESP_ERROR_CHECK(esp_mqtt_client_publish(client, topic, msg, 0, 1, 0));
    // If WiFi is briefly down, ESP_ERROR_CHECK calls abort() → device reboots
}
```

**GOOD:**
```cpp
esp_err_t ret = esp_mqtt_client_publish(client, topic, msg, 0, 1, 0);
if (ret != ESP_OK) {
    ESP_LOGW("MQTT", "Publish failed: %s, will retry", esp_err_to_name(ret));
    // Queue for retry, don't abort
}
```
**Consequence:** Device reboots on transient network errors. `ESP_ERROR_CHECK` calls `abort()` on failure -- appropriate for init, catastrophic for runtime. Use `ESP_RETURN_ON_ERROR` or explicit checks in runtime paths.

### AP-34: Bitfields for Hardware Register Mapping

**BAD:**
```cpp
struct ControlReg {
    uint8_t enable : 1;   // Bit order is implementation-defined
    uint8_t mode   : 2;   // Padding is implementation-defined
    uint8_t speed  : 3;   // Different compiler = different layout
};
volatile ControlReg* ctrl = reinterpret_cast<ControlReg*>(0x40020000);
ctrl->enable = 1;  // May write to wrong bit depending on compiler
```

**GOOD:**
```cpp
constexpr uint8_t CTRL_ENABLE = (1u << 0);
constexpr uint8_t CTRL_MODE_MASK = (0x3u << 1);
volatile uint8_t* ctrl = reinterpret_cast<volatile uint8_t*>(0x40020000);
*ctrl |= CTRL_ENABLE;  // Always correct regardless of compiler
```
**Consequence:** Registers mapped to wrong bits on different compilers or compiler versions. Bit order, padding, and signedness of bitfields are all implementation-defined in C/C++. The same struct can produce different register access on GCC vs Clang vs IAR. Use explicit shifts and masks for hardware registers.
