# Common ESP32 Libraries & Components

## ESP Component Registry

Search: [components.espressif.com](https://components.espressif.com)

Install: `idf.py add-dependency "namespace/component^version"`

PlatformIO: add to `lib_deps` in `platformio.ini`, or use `idf_component.yml` for IDF framework.

## Display & GUI

| Component | Purpose | Interface | Notes |
|---|---|---|---|
| `lvgl/lvgl` | GUI framework (v8 or v9) | - | See `references/lvgl.md` for full guide |
| `espressif/esp_lvgl_port` | LVGL integration (most boards) | - | Handles task, mutex, display registration |
| `espressif/esp_lvgl_adapter` | LVGL integration (P4, advanced) | - | Tear avoidance, DMA2D, newer |
| `espressif/esp_lcd` | LCD panel abstraction | SPI/RGB/MIPI | Base for all display drivers |
| `esp_lcd_ili9341` | ILI9341 display driver | SPI | 240x320, common CYD boards |
| `esp_lcd_st7789` | ST7789 display driver | SPI | 240x240/135x240, T-Display |
| `esp_lcd_gc9a01` | GC9A01 round display | SPI | 240x240 circular |
| `esp_lcd_ssd1306` | SSD1306 OLED | I2C/SPI | 128x64/128x32 monochrome |
| `esp_lcd_jd9365` | JD9365 MIPI-DSI | MIPI | 800x800, ESP32-P4 |

### Display Anti-Patterns

**DON'T**: Use blocking SPI transactions for display -- always use DMA.
**DON'T**: Call `lv_disp_flush_ready()` before DMA completion -- causes tearing.
**DON'T**: Allocate display buffers in PSRAM for SPI displays -- too slow, CPU stalls.
**DO**: Use double buffering with DMA for parallel render + flush.
**DO**: Call `lv_disp_flush_ready()` in the DMA completion ISR callback.

## Touch

| Component | Controller | Interface | Notes |
|---|---|---|---|
| `esp_lcd_touch_gt911` | GT911 (capacitive) | I2C | Multi-touch, common on larger displays |
| `esp_lcd_touch_ft5x06` | FT5x06/FT6x06 | I2C | Common on CYD boards |
| `esp_lcd_touch_cst816s` | CST816S | I2C | Common on small round displays |
| `esp_lcd_touch_xpt2046` | XPT2046 (resistive) | SPI | Older/cheaper boards |

### Touch Best Practices

**DO**: Configure `swap_xy`, `mirror_x`, `mirror_y` to match display rotation.
**DO**: Use interrupt-driven reading (not polling) for lower latency.
**DO**: Debounce touch events in firmware -- capacitive touch can be noisy.
**DON'T**: Share I2C bus between touch and other devices without mutex.
**DON'T**: Forget to check touch I2C address -- GT911 can be 0x5D or 0x14 depending on reset timing.

## WiFi & Networking

| Component | Purpose | Notes |
|---|---|---|
| `esp_wifi` (built-in) | WiFi STA/AP/AP+STA | See `references/networking.md` |
| `esp_wifi_remote` | WiFi via companion chip | ESP32-P4 → ESP32-C6 bridge |
| `esp_hosted` | WiFi/BLE transport bridge | SDIO/SPI between P4 and C6 |
| `espressif/mdns` (built-in) | mDNS service discovery | Zero-config networking |
| `espressif/esp_http_client` | HTTP/HTTPS client | Supports chunked, TLS, redirect |
| `espressif/esp_http_server` | HTTP server | REST APIs, WebSocket |
| `espressif/esp_mqtt` | MQTT 3.1.1 / 5.0 client | QoS 0/1/2, TLS, LWT |
| `espressif/esp_websocket_client` | WebSocket client | Async, TLS |

### mDNS Patterns

```cpp
// Service discovery (find a server on the local network)
mdns_init();
mdns_hostname_set("my-device");
mdns_result_t* results = NULL;
esp_err_t err = mdns_query_ptr("_http", "_tcp", 5000, 10, &results);
if (err == ESP_OK && results) {
    ESP_LOGI(TAG, "Found: %s:%d", results->hostname, results->port);
    mdns_query_results_free(results);
}

// Service advertisement
mdns_service_add("my-sensor", "_http", "_tcp", 80, NULL, 0);
```

**DO**: Set a timeout on `mdns_query_*` calls (default blocks for the full duration).
**DO**: Implement fallback to static IP if mDNS fails.
**DO**: Call `mdns_free_results()` after processing query results.
**DO**: Set hostname BEFORE starting WiFi for reliable advertisement.
**DON'T**: Rely solely on mDNS in production -- routers can block multicast.
**DON'T**: Use mDNS with WiFi power save enabled -- responses become unreliable. Call `esp_wifi_set_ps(WIFI_PS_NONE)` if mDNS is critical.
**DON'T**: Assume mDNS works on all networks -- enterprise networks and some consumer routers block mDNS multicast.
**KNOWN ISSUE**: mDNS can stop responding after minutes on some ESP-IDF versions. If mDNS is critical, implement timeout + fallback to static IP.
**KNOWN ISSUE**: Malformed mDNS packets from some Android apps can crash ESP32. Test on your target IDF version.

### esp_http_client Dos & Don'ts

**DO**: Set `timeout_ms` on all requests -- default can hang forever.
**DO**: Allocate 8-12KB stack for HTTP tasks (TLS handshake is stack-heavy).
**DO**: Always call `esp_http_client_cleanup()` -- even on error paths (memory leak).
**DO**: Use chunked encoding for streaming responses.
**DON'T**: Parse large JSON responses on stack -- use heap or streaming parser.
**DON'T**: Skip TLS cert verification in production (`cert_pem` must be set).

### esp_mqtt Dos & Don'ts

**DO**: Use MQTTS (TLS) in production -- unencrypted MQTT is trivially interceptable.
**DO**: Implement Last Will and Testament (LWT) for device availability monitoring.
**DO**: Buffer messages during WiFi disconnect and publish on reconnect.
**DO**: Set `keepalive` appropriately (30-60s typical).
**DON'T**: Publish large messages (>4KB) without checking return value.
**DON'T**: Use QoS 2 for telemetry data -- QoS 0 or 1 is sufficient and lighter.
**DON'T**: Block in the MQTT event handler -- defer heavy work to a task.

### PubSubClient (Arduino) Dos & Don'ts

**DO**: Call `client.loop()` frequently -- if not called, MQTT keepalive fails and connection drops.
**DO**: Disable WiFi sleep (`WiFi.setSleep(false)`) -- PubSubClient is unreliable with WiFi power save.
**DO**: Increase `MQTT_MAX_PACKET_SIZE` in `PubSubClient.h` for large payloads (default 256 bytes).
**DON'T**: Block the main loop for more than a few seconds -- `client.loop()` must run regularly.
**DON'T**: Use PubSubClient with ESP-IDF framework -- use `esp_mqtt` instead.
**DON'T**: Redefine `MQTT_KEEPALIVE` in your sketch -- it must be changed in `PubSubClient.h`.
**KNOWN ISSUE**: Message delivery delays on ESP32. Often caused by WiFi power save -- disable with `esp_wifi_set_ps(WIFI_PS_NONE)` as diagnostic step.

## Serial Communication

| Component | Purpose | Notes |
|---|---|---|
| `driver/uart` (built-in) | UART driver | Ring buffer, event queue, DMA |
| `driver/usb_serial_jtag` | USB Serial/JTAG | ESP32-S3/C3/C6 built-in USB |
| `espressif/usb_host_cdc_acm` | USB CDC host | ESP32-S3/P4 USB OTG |
| `tinyusb` (via ESP-IDF) | USB device stack | CDC, HID, MSC, MIDI |

### UART Patterns

```cpp
// Event-driven UART (recommended over polling)
QueueHandle_t uart_queue;
uart_driver_install(UART_NUM_1, 1024, 1024, 10, &uart_queue, 0);

// Process events in a task
uart_event_t event;
while (xQueueReceive(uart_queue, &event, portMAX_DELAY)) {
    switch (event.type) {
        case UART_DATA:
            uart_read_bytes(UART_NUM_1, buf, event.size, pdMS_TO_TICKS(100));
            process(buf, event.size);
            break;
        case UART_FIFO_OVF:
        case UART_BUFFER_FULL:
            uart_flush_input(UART_NUM_1);
            xQueueReset(uart_queue);
            break;
    }
}
```

**DON'T**: Use UART0 for application data on ESP32 -- it's the boot/debug UART.
**DON'T**: Assume data arrives in complete packets -- UART is a byte stream. Implement framing.

## Audio

| Component | Purpose | Notes |
|---|---|---|
| `espressif/es8311` | ES8311 codec driver | Common on dev boards |
| `espressif/es7210` | ES7210 4-channel ADC | Mic arrays |
| `espressif/esp_codec_dev` | Codec abstraction | Wraps various codecs |
| `driver/i2s` (built-in) | I2S driver | Audio data streaming |
| `esp_audio_player` | Audio player | MP3, WAV, FLAC |

### Audio Anti-Patterns

**DON'T**: Change audio sample rate while playback is active -- race condition with the I2S task.
**DON'T**: Use small I2S DMA buffers -- causes underflow and clicks/pops.
**DO**: Double-buffer audio data in PSRAM (large, sequential access = PSRAM-friendly).
**DO**: Use a dedicated high-priority task for audio playback.

## Storage

| Component | Purpose | Notes |
|---|---|---|
| `nvs_flash` (built-in) | Key-value storage | Config, credentials, small data |
| `spiffs` (built-in) | SPI Flash File System | Read-heavy, no directories |
| `littlefs` | LittleFS | Better wear leveling than SPIFFS |
| `fatfs` (built-in) | FAT filesystem | SD cards, wear-leveled flash |
| `espressif/esp_encrypted_img` | Encrypted images | OTA + flash encryption |

### NVS Best Practices
**DO**: Read before write -- only write if value changed (flash wear).
**DO**: Use `nvs_flash_secure_init()` for credentials.
**DON'T**: Store large blobs (>4KB) in NVS -- use SPIFFS/LittleFS.
**DON'T**: Open/close NVS handles in loops -- open once, use, close.

## Sensors & Peripherals (PlatformIO Arduino Libraries)

| Library | Purpose | Interface |
|---|---|---|
| `adafruit/Adafruit BME280 Library` | BME280 temp/humidity/pressure | I2C/SPI |
| `adafruit/Adafruit BME680 Library` | BME680 air quality | I2C/SPI |
| `adafruit/Adafruit NeoPixel` | WS2812/SK6812 LEDs | GPIO (RMT) |
| `knolleary/PubSubClient` | MQTT client (Arduino) | WiFi |
| `bblanchon/ArduinoJson` | JSON parser/generator | - |
| `h2zero/NimBLE-Arduino` | BLE (NimBLE stack) | BLE |
| `lovyan03/LovyanGFX` | Display driver (Arduino) | SPI/RGB/I2C |
| `bodmer/TFT_eSPI` | Display driver (Arduino) | SPI |

### ArduinoJson Dos & Don'ts

**DO**: Use v7 (`^7.0.0`) -- `DynamicJsonDocument` is deprecated, use `JsonDocument`.
**DO**: Use a custom PSRAM allocator for large documents: `heap_caps_malloc(size, MALLOC_CAP_SPIRAM)`.
**DO**: Use `measureJson()` to check size before serializing to a fixed buffer.
**DO**: Prefer `std::string` over Arduino `String` for serialization output on ESP32.
**DON'T**: Use v5 -- completely outdated API.
**DON'T**: Allocate oversized `JsonDocument` -- it consumes heap proportional to capacity, not content.
**DON'T**: Use `serializeJson()` with Arduino `String` on ESP32-S3 -- known crash on some Core 3.x versions. Use `std::string` or `char[]`.
**KNOWN BUG**: WiFi disconnects when large JsonDocuments consume most of the heap -- size documents carefully.

### NimBLE-Arduino Dos & Don'ts

**DO**: Always check for `nullptr` after `getService()` / `getCharacteristic()` -- crash if service not found.
**DO**: Use NimBLE over Bluedroid -- ~50KB less heap, better for LVGL coexistence.
**DO**: Release BLE memory after provisioning if BLE no longer needed.
**DO**: Set connection interval >= 100ms for battery devices.
**DON'T**: Combine NimBLE + WiFi + LVGL on ESP32 without PSRAM -- heap exhaustion.
**DON'T**: Use NimBLE + ESP-NOW simultaneously on ESP32-C3 -- ESP-NOW stops receiving.
**DON'T**: Rapid connect/disconnect cycles in production -- causes NimBLE crashes under BLE traffic.
**KNOWN BUG**: Connection failures after IDF version update -- test NimBLE with each IDF upgrade.

### TFT_eSPI vs LovyanGFX (Arduino Display Drivers)

| Feature | TFT_eSPI | LovyanGFX |
|---|---|---|
| Setup | `User_Setup.h` (compile-time) | Runtime config |
| DMA | Yes | Yes |
| Sprite support | Good | Better |
| Rotation | Good | Better (runtime) |
| Maintenance | Active | Active |
| LVGL integration | Manual flush_cb | Manual flush_cb |

**DO**: Choose ONE display driver -- never both.
**DO**: Enable DMA in whichever driver you use.
**DON'T**: Use TFT_eSPI user setup files from other projects without verifying pin assignments.
**DON'T**: Mix TFT_eSPI direct drawing with LVGL -- LVGL must own the display exclusively.

### cJSON (ESP-IDF Built-in) Dos & Don'ts

**DO**: Use `cJSON_CreateObject()` and always check for `NULL` return.
**DO**: Call `cJSON_Delete(root)` on the root object to free the entire tree.
**DO**: Use `cJSON_PrintUnformatted()` for smaller output (no whitespace).
**DON'T**: Forget to `cJSON_Delete()` -- each `cJSON_Parse()` allocates heap.
**DON'T**: Use cJSON for large documents on ESP32 -- every key/value is a separate `malloc`. Fragments heap rapidly.
**DON'T**: Parse untrusted JSON without size limits -- deeply nested JSON can overflow stack.
**PREFER**: `ArduinoJson` or a streaming parser for complex JSON on constrained devices.

## Image & Font Libraries

| Library | Purpose | Notes |
|---|---|---|
| `libpng` | PNG decoding | CPU-intensive; use for static assets |
| `libjpeg-turbo` (via IDF) | JPEG decode | Hardware JPEG on P4 |
| LVGL built-in image decoder | BIN/PNG/JPEG in LVGL | Use `lv_image_set_src()` |
| LVGL Font Converter | Custom fonts | Web tool: lvgl.io/tools/fontconverter |
| FreeType (via LVGL) | Runtime font rendering | Higher quality, more RAM |

### Image & Font Dos & Don'ts

**DO**: Convert images to LVGL binary format offline (LVGL Image Converter tool) -- zero runtime decode.
**DO**: Use BPP=4 for fonts unless anti-aliasing quality demands BPP=8.
**DO**: Limit font glyph ranges to what you actually use (Latin-1 + symbols, not full Unicode).
**DO**: Store large images in SPIFFS/LittleFS and load on demand (not all in RAM).
**DO**: Use hardware JPEG decoder on ESP32-P4 (`esp_jpeg` component).
**DO**: Pre-decode PNG/JPEG at startup and keep `lv_image_dsc_t` in PSRAM for reuse.
**DON'T**: Decode PNG/JPEG on every frame -- decode once, cache the result.
**DON'T**: Include unused fonts in the build -- each Montserrat size adds 20-100KB to flash.
**DON'T**: Use FreeType for simple static UIs -- it consumes significant heap. Use pre-converted bitmap fonts.
**DON'T**: Open/close image files in LVGL timer callbacks -- filesystem I/O blocks the LVGL task.
**KNOWN ISSUE**: LVGL TJPGD decoder leaks memory on JPEG decode failure (fixed in v9.3.1+).
**KNOWN ISSUE**: `lv_image_cache_drop()` fails if widgets still reference the image. Pre-convert to binary format instead.

### Storage Dos & Don'ts

**SPIFFS**: Good for read-heavy workloads, no directories, ~75% usable space, degrades with many small writes.
**LittleFS**: Better wear leveling, directory support, power-loss resilient. **Preferred over SPIFFS for new projects.**
**FAT**: Required for SD cards. Use wear leveling component for internal flash.

**DO**: Use LittleFS for new projects (better wear leveling than SPIFFS).
**DO**: Mount filesystem AFTER NVS init (NVS provides wear-leveling data).
**DO**: Check free space before writing -- full filesystem can corrupt files.
**DON'T**: Open files in LVGL timer callbacks or ISRs -- filesystem I/O is blocking.
**DON'T**: Use FAT on internal flash without wear leveling -- flash sectors will fail.
**DON'T**: Store frequently-updated data (>1 write/minute) on flash -- use RAM + periodic flush.

## Protocol Libraries

| Component | Purpose | Notes |
|---|---|---|
| `espressif/esp_tls` | TLS wrapper (mbedTLS) | Used by HTTP, MQTT, WebSocket |
| `espressif/cbor` | CBOR encoding/decoding | Compact binary, IoT-friendly |
| `protobuf-c` | Protocol Buffers for C | Structured data serialization |
| `cJSON` (built-in) | JSON parser | Simple but heap-heavy |
| `driver/twai` (built-in) | CAN bus (TWAI) | Automotive/industrial |
| `espressif/esp_now` (built-in) | ESP-NOW P2P | Connectionless, low latency |

### Protocol Dos & Don'ts

**esp_tls**: Allocate 8-12KB stack for any task doing TLS. Set `timeout_ms`. Always verify certificates in production.
**cJSON**: See cJSON section above. Each parse allocates many small heap blocks -- fragments fast.
**TWAI (CAN)**: Use acceptance filter to reduce CPU load. Don't forget termination resistor (120 ohm).
**ESP-NOW**: Max 250 bytes per packet. Encrypt with PMK/LMK for security. Doesn't work if WiFi is in AP+STA mode on some firmware versions.
