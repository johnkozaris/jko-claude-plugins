# ESP32 Networking: WiFi, BLE & Protocols

## WiFi Critical Rules (From Official Espressif Guidance)

1. **Wait for `IP_EVENT_STA_GOT_IP` before creating sockets** -- NEVER `WIFI_EVENT_STA_CONNECTED` (DHCP not complete yet)
2. **Close ALL sockets on `WIFI_EVENT_STA_DISCONNECTED`** -- they are permanently invalid, even after reconnect
3. **Event handlers must NEVER block** -- defer heavy work to a task via notification or queue
4. **Always use `WIFI_INIT_CONFIG_DEFAULT()` macro** -- never manually construct `wifi_init_config_t`
5. **Initialization order is mandatory**: NVS -> netif -> event loop -> WiFi init -> register handlers -> WiFi start (AP-25)
6. **If IP changes (`ip_change=true`)**: close and recreate ALL sockets even though WiFi didn't drop

## WiFi Event-Driven Architecture

### Proper WiFi Initialization (IDF v5)
```cpp
// Event-driven WiFi with proper error handling
static void wifi_event_handler(void* arg, esp_event_base_t base,
                                int32_t event_id, void* event_data) {
    if (base == WIFI_EVENT) {
        switch (event_id) {
            case WIFI_EVENT_STA_START:
                esp_wifi_connect();
                break;
            case WIFI_EVENT_STA_DISCONNECTED: {
                auto* event = static_cast<wifi_event_sta_disconnected_t*>(event_data);
                ESP_LOGW("WIFI", "Disconnected, reason: %d", event->reason);
                // Exponential backoff reconnection
                vTaskDelay(pdMS_TO_TICKS(reconnect_delay_ms));
                reconnect_delay_ms = std::min(reconnect_delay_ms * 2, 30000u);
                esp_wifi_connect();
                break;
            }
        }
    } else if (base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        auto* event = static_cast<ip_event_got_ip_t*>(event_data);
        ESP_LOGI("WIFI", "Got IP: " IPSTR, IP2STR(&event->ip_info.ip));
        reconnect_delay_ms = 1000;  // Reset backoff
        xEventGroupSetBits(wifi_event_group, CONNECTED_BIT);
    }
}
```

### WiFi Rules

**DO**: Use event-driven patterns -- register handlers with `esp_event_handler_instance_register()`.
**DO**: Implement exponential backoff for reconnection (1s, 2s, 4s, 8s... max 30s).
**DO**: Use `xEventGroupWaitBits()` to wait for connection before starting network operations.
**DO**: Set hostname before connecting: `esp_netif_set_hostname(netif, "my-device")`.
**DO**: Store WiFi credentials in NVS with encryption.
**DO**: Handle DHCP failures -- fallback to static IP or AP mode for provisioning.

**DON'T**: Busy-loop checking `wifi_connected` flag -- use event groups or semaphores.
**DON'T**: Block the WiFi event handler with long operations -- defer to a task.
**DON'T**: Use `delay()` in the event handler -- it blocks the system event task.
**DON'T**: Hardcode WiFi credentials in source code.
**DON'T**: Forget that WiFi needs ~40KB of heap for the stack.
**DON'T**: Use ADC2 while WiFi is active on ESP32 (original).

### WiFi Power Save
```cpp
// Configure WiFi power save
esp_wifi_set_ps(WIFI_PS_MIN_MODEM);  // Balances power and latency
// WIFI_PS_NONE: No power save (lowest latency, highest power)
// WIFI_PS_MIN_MODEM: Modem sleep at minimum (good balance)
// WIFI_PS_MAX_MODEM: Maximum modem sleep (highest power savings, higher latency)
```

## BLE (NimBLE Recommended)

### NimBLE vs Bluedroid

| Feature | NimBLE | Bluedroid |
|---|---|---|
| Memory usage | ~50KB heap | ~100KB+ heap |
| Code size | Smaller | Larger |
| Performance | Good | Good |
| Features | BLE only | BLE + Classic BT |
| Recommendation | **Default choice** | Only if Classic BT needed |

Enable in menuconfig: `Component config -> Bluetooth -> Bluetooth controller -> BLE only` and `Host -> NimBLE`

### BLE Best Practices

**DO**: Use NimBLE unless Classic Bluetooth is required.
**DO**: Design GATT services with security in mind (bonding, encryption).
**DO**: Minimize connection interval for power savings.
**DO**: Use notifications instead of polling for real-time data.
**DO**: Handle disconnection gracefully -- re-advertise.

**DON'T**: Forget that BLE + WiFi coexistence requires careful timing.
**DON'T**: Send more data per connection event than MTU allows.
**DON'T**: Use 20ms connection interval for battery devices -- 100ms+ is usually fine.

## HTTP/HTTPS Client

```cpp
esp_http_client_config_t config = {};
config.url = "https://api.example.com/data";
config.cert_pem = server_cert;  // ALWAYS verify TLS certificates
config.timeout_ms = 10000;
config.buffer_size = 2048;
config.buffer_size_tx = 1024;

esp_http_client_handle_t client = esp_http_client_init(&config);
esp_err_t err = esp_http_client_perform(client);
if (err == ESP_OK) {
    int status = esp_http_client_get_status_code(client);
    int content_length = esp_http_client_get_content_length(client);
    ESP_LOGI("HTTP", "Status: %d, Content-Length: %d", status, content_length);
}
esp_http_client_cleanup(client);
```

**DO**: Set timeouts on all network operations.
**DO**: Verify TLS certificates in production (use `cert_pem`).
**DO**: Allocate HTTP task stack >= 8KB (TLS needs large stack).
**DON'T**: Use HTTP (unencrypted) for any sensitive data.
**DON'T**: Forget to call `esp_http_client_cleanup()` -- memory leak.

## MQTT

```cpp
esp_mqtt_client_config_t mqtt_cfg = {};
mqtt_cfg.broker.address.uri = "mqtts://broker.example.com:8883";
mqtt_cfg.broker.verification.certificate = server_cert;
mqtt_cfg.credentials.username = "device-001";
mqtt_cfg.credentials.authentication.password = "secure-token";
mqtt_cfg.session.keepalive = 30;
mqtt_cfg.network.reconnect_timeout_ms = 10000;
mqtt_cfg.buffer.size = 1024;

esp_mqtt_client_handle_t client = esp_mqtt_client_init(&mqtt_cfg);
esp_mqtt_client_register_event(client, ESP_EVENT_ANY_ID, mqtt_event_handler, nullptr);
esp_mqtt_client_start(client);
```

**DO**: Use MQTTS (TLS) in production.
**DO**: Implement QoS levels appropriately (QoS 1 for critical data, QoS 0 for telemetry).
**DO**: Use Last Will and Testament (LWT) for device availability.
**DO**: Buffer messages during disconnection and send on reconnect.

## Common Networking Anti-Patterns

| Anti-Pattern | Consequence | Fix |
|---|---|---|
| Blocking WiFi event handler | System event queue backs up, watchdog | Defer to task |
| No reconnection logic | Device offline permanently | Exponential backoff reconnect |
| Hardcoded credentials | Security breach, no field updates | NVS storage + provisioning |
| No TLS verification | MITM attacks | Always set `cert_pem` |
| Infinite retry without backoff | CPU waste, WiFi interference | Exponential backoff |
| HTTP without timeout | Task hangs permanently | Set `timeout_ms` |
| Large JSON parsing on stack | Stack overflow | Heap allocation or streaming parser |
| No DNS caching | Extra latency per request | Cache resolved IPs |
