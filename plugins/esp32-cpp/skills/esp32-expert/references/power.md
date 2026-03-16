# ESP32 Power Management

## Sleep Modes

| Mode | CPU | WiFi/BT | RAM | RTC | Wake Time | Current |
|---|---|---|---|---|---|---|
| Active | On | On | On | On | - | 80-240mA |
| Modem Sleep | On | Off (periodic) | On | On | - | 20-30mA |
| Light Sleep | Paused | Off | On | On | ~1ms | 0.8mA |
| Deep Sleep | Off | Off | Off | On | ~10ms | 10-150uA |
| Hibernation | Off | Off | Off | Partial | ~10ms | 5uA |

## Deep Sleep

### Configuration
```cpp
// Wake on timer
esp_sleep_enable_timer_wakeup(60 * 1000000ULL);  // 60 seconds

// Wake on GPIO (RTC GPIO only)
esp_sleep_enable_ext0_wakeup(GPIO_NUM_33, 0);  // Wake on LOW

// Wake on multiple GPIOs
uint64_t gpio_mask = (1ULL << GPIO_NUM_32) | (1ULL << GPIO_NUM_33);
esp_sleep_enable_ext1_wakeup(gpio_mask, ESP_EXT1_WAKEUP_ANY_LOW);

// Wake on touch pad
esp_sleep_enable_touchpad_wakeup();

// Enter deep sleep
ESP_LOGI("SLEEP", "Entering deep sleep...");
esp_deep_sleep_start();
// Code after this line never executes -- device resets on wake
```

### Persisting Data Across Deep Sleep
```cpp
// RTC memory survives deep sleep (8KB)
RTC_DATA_ATTR static int boot_count = 0;
RTC_DATA_ATTR static float last_temperature = 0.0f;
RTC_DATA_ATTR static uint32_t accumulated_readings[100];

extern "C" void app_main(void) {
    boot_count++;
    ESP_LOGI("BOOT", "Boot count: %d", boot_count);

    // Check wake cause
    esp_sleep_wakeup_cause_t cause = esp_sleep_get_wakeup_cause();
    switch (cause) {
        case ESP_SLEEP_WAKEUP_TIMER:
            ESP_LOGI("WAKE", "Timer wake");
            break;
        case ESP_SLEEP_WAKEUP_EXT0:
            ESP_LOGI("WAKE", "GPIO wake");
            break;
        default:
            ESP_LOGI("WAKE", "Cold boot (power on / reset)");
            boot_count = 1;  // Reset on cold boot
    }
}
```

### Deep Sleep Best Practices

**DO**: Disconnect WiFi/BLE before entering deep sleep.
**DO**: Use RTC_DATA_ATTR for data that must survive deep sleep.
**DO**: Check wake cause on startup to differentiate cold boot from wake.
**DO**: Minimize time in active mode for battery applications.
**DO**: Use ULP coprocessor for simple monitoring during deep sleep.

**DON'T**: Keep WiFi connected for devices that only need periodic updates -- use deep sleep.
**DON'T**: Use more than 8KB of RTC memory.
**DON'T**: Forget that deep sleep resets the entire CPU -- all variables are lost except RTC.
**DON'T**: Assume GPIO state is maintained during deep sleep (it isn't, except RTC GPIOs with hold).

### GPIO Hold During Sleep
```cpp
// Hold GPIO state during deep sleep
gpio_set_level(GPIO_NUM_2, 1);  // Keep LED on during sleep
gpio_hold_en(GPIO_NUM_2);
gpio_deep_sleep_hold_en();

// On wake, release hold
gpio_hold_dis(GPIO_NUM_2);
```

## Light Sleep

```cpp
// Configure light sleep
esp_sleep_enable_timer_wakeup(100000);  // 100ms
esp_light_sleep_start();
// Execution continues here after wake
ESP_LOGI("WAKE", "Woke from light sleep");
```

Light sleep is useful for:
- Periodic sensor sampling at moderate rates (10-100Hz)
- Waiting for external events with fast response time
- Reducing power while maintaining RAM contents and task state

## WiFi Power Optimization

```cpp
// WiFi power save modes
esp_wifi_set_ps(WIFI_PS_MIN_MODEM);

// For minimum power: connect, send data, disconnect, deep sleep
void send_and_sleep() {
    wifi_connect();  // Blocking connect
    send_data();     // Send sensor data
    esp_wifi_disconnect();
    esp_wifi_stop();
    esp_deep_sleep_start();
}
```

### Battery-Powered WiFi Device Pattern
```
1. Wake from deep sleep
2. Read sensor data (fast -- before WiFi)
3. Connect WiFi
4. Send data (MQTT/HTTP)
5. Disconnect WiFi
6. Enter deep sleep
Total active time target: 2-5 seconds
```

## Current Measurement

**DO**: Use a current sense resistor (0.1 ohm) + oscilloscope for accurate measurements.
**DO**: Measure over complete duty cycles (sleep + wake).
**DON'T**: Trust USB power measurements -- USB cables have significant resistance.
**DON'T**: Measure only peak current -- average current over time determines battery life.

### Battery Life Estimation
```
Average current = (active_current * active_time + sleep_current * sleep_time) / total_time

Example: 5 seconds active at 100mA, 55 seconds deep sleep at 10uA
Avg = (100 * 5 + 0.01 * 55) / 60 = 8.34mA
2000mAh battery life = 2000 / 8.34 = 240 hours = 10 days
```

## ULP Coprocessor (ESP32, S2, S3)

The Ultra Low Power coprocessor runs during deep sleep:
- Can read GPIO, ADC
- Can wake the main CPU when conditions are met
- ESP32: FSM-based ULP, clocked from RTC_SLOW_CLK (~150kHz)
- ESP32-S2/S3: RISC-V ULP, clocked from RTC_SLOW_CLK (~17.5MHz max)
- Typical additional current draw: 100-150uA depending on workload

Use for: temperature threshold monitoring, motion detection, periodic ADC sampling during deep sleep.
