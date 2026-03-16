# ESP32 Security Guide

## Security Features by Variant

| Feature | ESP32 | S2 | S3 | C3 | C6 | P4 |
|---|---|---|---|---|---|---|
| Secure Boot v1 | Deprecated | - | - | - | - | - |
| Secure Boot v2 | Yes | Yes | Yes | Yes | Yes | Yes |
| Flash Encryption | Yes | Yes | Yes | Yes | Yes | Yes |
| Digital Signature | - | Yes | Yes | Yes | Yes | Yes |
| HMAC | - | Yes | Yes | Yes | Yes | Yes |
| World Controller | - | Yes | - | - | - | - |
| TEE/Privilege Sep. | - | - | - | - | - | Yes |

## Secure Boot

### When to Enable
- **Development**: Disabled (allows easy reflashing)
- **Production**: ALWAYS enabled
- **Note**: Enabling Secure Boot is ONE-WAY on most variants. Plan carefully.

### Setup
```bash
# In menuconfig:
# Security features -> Enable hardware Secure Boot in bootloader (v2)
# Security features -> Sign binaries during build

# Generate signing key (KEEP THIS SAFE - loss = bricked devices)
espsecure.py generate_signing_key --version 2 secure_boot_signing_key.pem
```

**DO**: Store signing keys in a hardware security module (HSM) or secure vault.
**DO**: Use different signing keys for development and production.
**DO**: Test secure boot in a separate environment before production deployment.

**DON'T**: Commit signing keys to git.
**DON'T**: Enable secure boot on development boards without understanding the implications.
**DON'T**: Lose the signing key -- devices become unbrickable.

## Flash Encryption

### Purpose
Encrypts flash contents so firmware cannot be read by physical access.

```bash
# In menuconfig:
# Security features -> Enable flash encryption on boot
# Mode: Development (allows reflashing) or Release (one-way)
```

**DO**: Use Development mode during testing, Release mode for production.
**DO**: Encrypt NVS partition separately for sensitive data.
**DON'T**: Store secrets in plaintext flash even with encryption -- defense in depth.

## Credential Storage

### NVS Encryption for Secrets
```cpp
// Initialize NVS with encryption
nvs_sec_cfg_t sec_cfg;
esp_err_t err = nvs_flash_read_security_cfg(nvs_key_partition, &sec_cfg);
if (err == ESP_ERR_NVS_KEYS_NOT_INITIALIZED) {
    nvs_flash_generate_keys(nvs_key_partition, &sec_cfg);
}
nvs_flash_secure_init_partition("nvs", &sec_cfg);

// Store credentials
nvs_handle_t handle;
nvs_open("credentials", NVS_READWRITE, &handle);
nvs_set_str(handle, "wifi_pass", password);
nvs_set_blob(handle, "tls_key", key_data, key_len);
nvs_commit(handle);
nvs_close(handle);
```

### What to Store Securely
- WiFi passwords
- API keys and tokens
- TLS client certificates and private keys
- Device identity secrets
- Encryption keys for application data

**DON'T**: Hardcode credentials in source code.
**DON'T**: Store secrets in `sdkconfig` -- it may be committed to git.
**DON'T**: Use `#define SECRET "value"` -- visible in binary.
**DON'T**: Log credentials at any log level.

## TLS/SSL Best Practices

```cpp
// ALWAYS verify server certificates
esp_tls_cfg_t tls_cfg = {};
tls_cfg.cacert_buf = server_root_ca;
tls_cfg.cacert_bytes = sizeof(server_root_ca);
tls_cfg.timeout_ms = 10000;
// Optional: client certificate for mutual TLS
tls_cfg.clientcert_buf = client_cert;
tls_cfg.clientcert_bytes = sizeof(client_cert);
tls_cfg.clientkey_buf = client_key;
tls_cfg.clientkey_bytes = sizeof(client_key);
```

**DO**: Pin server certificates or use a trusted CA bundle.
**DO**: Use mbedTLS (default in ESP-IDF) -- it's well-tested for embedded.
**DO**: Allocate 8-12KB stack for tasks doing TLS operations.
**DO**: Set connection timeouts.

**DON'T**: Skip certificate verification (`skip_common_name_check`, `common_name = "*"`).
**DON'T**: Use self-signed certificates without pinning in production.
**DON'T**: Ignore certificate expiration -- devices may run for years.

## Common Security Anti-Patterns

| Anti-Pattern | Risk | Fix |
|---|---|---|
| No secure boot | Firmware replacement attacks | Enable Secure Boot v2 |
| No flash encryption | Firmware reverse engineering | Enable flash encryption |
| Hardcoded credentials | Credential extraction from binary | NVS encrypted storage |
| No TLS cert verification | Man-in-the-middle attacks | Pin certificates or use CA bundle |
| Debug UART enabled in production | Physical access = full control | Disable UART download mode |
| Default WiFi AP password | Network access to all devices | Unique per-device credentials |
| OTA without signing | Malicious firmware injection | Sign OTA images |
| Logging secrets | Credential exposure via serial | Never log sensitive data |
| No anti-rollback | Downgrade to vulnerable firmware | Enable anti-rollback counter |
| Predictable device IDs | Enumeration attacks | Use crypto-random IDs |

## OTA Security

```cpp
// Secure OTA update
esp_https_ota_config_t ota_config = {};
ota_config.http_config = &http_config;  // With cert verification!

esp_err_t ret = esp_https_ota(&ota_config);
if (ret == ESP_OK) {
    ESP_LOGI("OTA", "Update successful, restarting...");
    esp_restart();
} else {
    ESP_LOGE("OTA", "Update failed: %s", esp_err_to_name(ret));
    // Rollback will happen automatically if app doesn't confirm
}

// After successful boot, confirm the update
esp_ota_mark_app_valid_cancel_rollback();
```

**DO**: Always use HTTPS for OTA.
**DO**: Verify OTA image signature.
**DO**: Implement rollback on boot failure.
**DO**: Use anti-rollback to prevent downgrade attacks.
**DO**: Confirm valid boot to prevent automatic rollback.

## Known ESP32 Security Issues

Check Espressif's security advisory page regularly and update ESP-IDF.

- **ESP32 Bluetooth HCI**: Undocumented vendor-specific HCI commands exist. Low risk (requires local BT access), but audit any BT-exposed device.
- **BluFi provisioning**: Memory safety issues in the reference app. If you copied BluFi example code, review it.
- **BT AVRCP OOB read**: Fixed in ESP-IDF 5.5.1/5.4.3/5.3.4. Update if using Bluetooth.
- **Fault injection**: Power analysis + fault injection can bypass Secure Boot on some revisions. Physical security matters for high-value targets.

**Rule**: Reference/example code copied into production inherits all its bugs. Always review vendor examples before shipping.

### OWASP IoT Top 10 Mapping for ESP32 Projects

| OWASP IoT Risk | ESP32 Relevance | Check |
|---|---|---|
| Weak/default passwords | NVS credentials, AP mode password | Unique per-device, never default |
| Insecure network services | Open MQTT, Telnet, HTTP admin | TLS everywhere, disable unused ports |
| Insecure ecosystem interfaces | Cloud API keys, mobile app | Per-device identity, TLS cert pinning |
| Lack of secure update | OTA without signature | Sign + verify + anti-rollback |
| Use of insecure components | Outdated ESP-IDF, old libraries | Track CVEs, update regularly |
| Insufficient privacy protection | Sensor data in plaintext flash | NVS encryption, TLS transport |
| Insecure data transfer | HTTP, unencrypted MQTT | TLS for all network traffic |
| Lack of device management | No remote config or monitoring | OTA + health telemetry |
| Insecure default settings | Debug UART enabled, no secure boot | Production security checklist |
| Lack of physical hardening | JTAG exposed, flash readable | Disable JTAG in eFuse, flash encryption |
