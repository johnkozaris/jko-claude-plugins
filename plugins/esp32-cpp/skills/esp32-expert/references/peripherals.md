# ESP32 Peripheral Driver Patterns

## General Rules

1. **Always check return values** -- `esp_err_t` is not optional
2. **Protect shared buses with mutexes** -- I2C and SPI buses shared across tasks MUST have a mutex
3. **Search the datasheet** for any unfamiliar IC -- timing requirements, voltage levels, register maps
4. **DMA buffers must be in internal DRAM** -- use `MALLOC_CAP_DMA` or `DRAM_ATTR`
5. **Pin assignment differs by variant** -- always check the pin multiplexing table for the target chip

## GPIO

### Configuration Pattern
```cpp
// Output
gpio_config_t out_cfg = {};
out_cfg.pin_bit_mask = (1ULL << GPIO_NUM_2) | (1ULL << GPIO_NUM_4);
out_cfg.mode = GPIO_MODE_OUTPUT;
out_cfg.pull_up_en = GPIO_PULLUP_DISABLE;
out_cfg.pull_down_en = GPIO_PULLDOWN_DISABLE;
ESP_ERROR_CHECK(gpio_config(&out_cfg));

// Input with interrupt
gpio_config_t in_cfg = {};
in_cfg.pin_bit_mask = (1ULL << GPIO_NUM_0);
in_cfg.mode = GPIO_MODE_INPUT;
in_cfg.pull_up_en = GPIO_PULLUP_ENABLE;
in_cfg.intr_type = GPIO_INTR_NEGEDGE;
ESP_ERROR_CHECK(gpio_config(&in_cfg));

gpio_install_isr_service(0);
gpio_isr_handler_add(GPIO_NUM_0, button_isr, nullptr);
```

### GPIO Gotchas

**DO**: Use `gpio_reset_pin()` when reconfiguring a pin that was used for something else.
**DO**: Check strapping pins -- GPIO0, GPIO2, GPIO12, GPIO15 on ESP32 affect boot mode.
**DO**: Add debouncing for mechanical switches (software timer or RC filter).
**DON'T**: Use GPIO6-11 on ESP32 -- they're connected to internal flash SPI.
**DON'T**: Draw more than 40mA per pin (absolute max) or 20mA recommended.
**DON'T**: Connect 5V signals directly -- ESP32 is 3.3V logic. Use level shifters.

### Strapping Pins by Variant

| Variant | Boot Strapping Pins | Notes |
|---|---|---|
| ESP32 | GPIO0, GPIO2, GPIO12, GPIO15 | GPIO12 controls flash voltage |
| ESP32-S2 | GPIO0, GPIO45, GPIO46 | GPIO46 controls boot mode |
| ESP32-S3 | GPIO0, GPIO3, GPIO45, GPIO46 | Similar to S2 |
| ESP32-C3 | GPIO2, GPIO8, GPIO9 | RISC-V boot mode |
| ESP32-C6 | GPIO8, GPIO9, GPIO15 | Check TRM for details |

## I2C

### ESP-IDF v5 New I2C Driver
```cpp
// IDF v5.x new driver (recommended)
i2c_master_bus_config_t bus_config = {};
bus_config.clk_source = I2C_CLK_SRC_DEFAULT;
bus_config.i2c_port = I2C_NUM_0;
bus_config.scl_io_num = GPIO_NUM_22;
bus_config.sda_io_num = GPIO_NUM_21;
bus_config.glitch_ignore_cnt = 7;
bus_config.flags.enable_internal_pullup = true;

i2c_master_bus_handle_t bus_handle;
ESP_ERROR_CHECK(i2c_new_master_bus(&bus_config, &bus_handle));

// Add device
i2c_device_config_t dev_config = {};
dev_config.dev_addr_length = I2C_ADDR_BIT_LEN_7;
dev_config.device_address = 0x76;  // BME280
dev_config.scl_speed_hz = 400000;

i2c_master_dev_handle_t dev_handle;
ESP_ERROR_CHECK(i2c_master_bus_add_device(bus_handle, &dev_config, &dev_handle));
```

### I2C Best Practices

**DO**: Use external pullup resistors (4.7K for 100kHz, 2.2K for 400kHz). Internal pullups are too weak for reliable operation.
**DO**: Implement bus recovery (clock stretching, bus reset) for stuck slaves.
**DO**: Protect shared I2C bus with a mutex when multiple tasks access it.
**DO**: Add timeouts to all I2C transactions.

**DON'T**: Run I2C at 400kHz with long wires (>30cm). Drop to 100kHz.
**DON'T**: Use I2C for high-bandwidth data (>100KB/s). Use SPI instead.
**DON'T**: Connect multiple devices with the same address without a multiplexer (TCA9548A).
**DON'T**: Forget to check for ACK/NACK -- a NACK means the device isn't responding.

### I2C Bus Recovery
```cpp
esp_err_t i2c_bus_recover(gpio_num_t sda, gpio_num_t scl) {
    // If SDA is stuck low, clock SCL until SDA releases
    // MUST use open-drain: I2C lines are pulled high by external resistors
    gpio_set_direction(scl, GPIO_MODE_OUTPUT_OD);
    gpio_set_direction(sda, GPIO_MODE_INPUT_OUTPUT_OD);

    for (int i = 0; i < 9; i++) {
        gpio_set_level(scl, 0);
        esp_rom_delay_us(5);
        gpio_set_level(scl, 1);
        esp_rom_delay_us(5);
        if (gpio_get_level(sda)) break;  // SDA released
    }
    // Generate STOP condition
    gpio_set_direction(sda, GPIO_MODE_OUTPUT);
    gpio_set_level(sda, 0);
    esp_rom_delay_us(5);
    gpio_set_level(scl, 1);
    esp_rom_delay_us(5);
    gpio_set_level(sda, 1);
    return ESP_OK;
}
```

## SPI

### SPI Configuration
```cpp
spi_bus_config_t bus_cfg = {};
bus_cfg.mosi_io_num = GPIO_NUM_23;
bus_cfg.miso_io_num = GPIO_NUM_19;
bus_cfg.sclk_io_num = GPIO_NUM_18;
bus_cfg.quadwp_io_num = -1;
bus_cfg.quadhd_io_num = -1;
bus_cfg.max_transfer_sz = 4096;
ESP_ERROR_CHECK(spi_bus_initialize(SPI2_HOST, &bus_cfg, SPI_DMA_CH_AUTO));

spi_device_interface_config_t dev_cfg = {};
dev_cfg.clock_speed_hz = 10 * 1000 * 1000;  // 10MHz
dev_cfg.mode = 0;  // CPOL=0, CPHA=0
dev_cfg.spics_io_num = GPIO_NUM_5;
dev_cfg.queue_size = 7;

spi_device_handle_t spi_handle;
ESP_ERROR_CHECK(spi_bus_add_device(SPI2_HOST, &dev_cfg, &spi_handle));
```

### SPI Best Practices

**DO**: Use DMA for transfers > 64 bytes (`SPI_DMA_CH_AUTO`).
**DO**: Batch transactions with `spi_device_queue_trans()` for throughput.
**DO**: Check SPI mode (CPOL/CPHA) in the slave device datasheet.
**DO**: Use hardware CS when possible for timing accuracy.

**DON'T**: Use SPI1_HOST -- it's reserved for internal flash on most variants.
**DON'T**: Share SPI bus across tasks without proper locking (`spi_device_acquire_bus`).
**DON'T**: Exceed the clock speed specified in the slave device datasheet.
**DON'T**: Forget `max_transfer_sz` -- DMA needs this to allocate buffers.

### Multiple SPI Devices
```cpp
// Multiple devices on same bus (different CS pins)
spi_device_handle_t display, sd_card;
// ... add both to same SPI2_HOST with different spics_io_num

// Acquiring bus for burst transactions
spi_device_acquire_bus(display, portMAX_DELAY);
for (int i = 0; i < frame_count; i++) {
    spi_device_transmit(display, &transactions[i]);
}
spi_device_release_bus(display);
```

## UART

### UART Configuration
```cpp
uart_config_t uart_config = {};
uart_config.baud_rate = 115200;
uart_config.data_bits = UART_DATA_8_BITS;
uart_config.parity = UART_PARITY_DISABLE;
uart_config.stop_bits = UART_STOP_BITS_1;
uart_config.flow_ctrl = UART_HW_FLOWCTRL_DISABLE;
uart_config.source_clk = UART_SCLK_DEFAULT;

ESP_ERROR_CHECK(uart_driver_install(UART_NUM_1, 1024, 1024, 10, &uart_queue, 0));
ESP_ERROR_CHECK(uart_param_config(UART_NUM_1, &uart_config));
ESP_ERROR_CHECK(uart_set_pin(UART_NUM_1, TX_PIN, RX_PIN, -1, -1));
```

### UART Best Practices

**DO**: Use event-driven reading with `uart_driver_install` queue parameter.
**DO**: Size RX buffer to handle burst data (at least 2x expected message size).
**DO**: Implement framing protocol (start byte, length, CRC, end byte).
**DO**: Use pattern detection for AT-command style protocols.

**DON'T**: Use UART0 for application data on ESP32 -- it's the boot/debug UART.
**DON'T**: Busy-wait on `uart_read_bytes()` with `portMAX_DELAY` without a watchdog.
**DON'T**: Assume data arrives in complete packets -- UART is a byte stream.

## ADC

### ADC Calibration (Critical!)
```cpp
// IDF v5.x ADC oneshot driver
adc_oneshot_unit_handle_t adc_handle;
adc_oneshot_unit_init_cfg_t init_cfg = { .unit_id = ADC_UNIT_1 };
ESP_ERROR_CHECK(adc_oneshot_new_unit(&init_cfg, &adc_handle));

adc_oneshot_chan_cfg_t chan_cfg = {
    .atten = ADC_ATTEN_DB_12,  // 0-3.3V range
    .bitwidth = ADC_BITWIDTH_12,
};
ESP_ERROR_CHECK(adc_oneshot_config_channel(adc_handle, ADC_CHANNEL_0, &chan_cfg));

// ALWAYS calibrate for accurate voltage readings
adc_cali_handle_t cali_handle;
adc_cali_curve_fitting_config_t cali_cfg = {
    .unit_id = ADC_UNIT_1,
    .atten = ADC_ATTEN_DB_12,
    .bitwidth = ADC_BITWIDTH_12,
};
adc_cali_create_scheme_curve_fitting(&cali_cfg, &cali_handle);

// Read with calibration
int raw, voltage_mv;
adc_oneshot_read(adc_handle, ADC_CHANNEL_0, &raw);
adc_cali_raw_to_voltage(cali_handle, raw, &voltage_mv);
```

**DO**: Always use calibration -- raw ADC values vary 10-20% between chips.
**DO**: Average multiple readings for stable measurements.
**DON'T**: Use ADC2 while WiFi is active -- WiFi uses ADC2 internally on ESP32.
**DON'T**: Trust raw ADC readings for precision measurements without calibration.

## Timer/Counter

### General Purpose Timer (IDF v5)
```cpp
gptimer_handle_t timer;
gptimer_config_t timer_cfg = {
    .clk_src = GPTIMER_CLK_SRC_DEFAULT,
    .direction = GPTIMER_COUNT_UP,
    .resolution_hz = 1000000,  // 1MHz = 1us resolution
};
ESP_ERROR_CHECK(gptimer_new_timer(&timer_cfg, &timer));

gptimer_alarm_config_t alarm_cfg = {
    .alarm_count = 1000,  // 1ms alarm
    .reload_count = 0,
    .flags = { .auto_reload_on_alarm = true },
};
ESP_ERROR_CHECK(gptimer_set_alarm_action(timer, &alarm_cfg));

gptimer_event_callbacks_t cbs = { .on_alarm = timer_alarm_cb };
ESP_ERROR_CHECK(gptimer_register_event_callbacks(timer, &cbs, nullptr));
ESP_ERROR_CHECK(gptimer_enable(timer));
ESP_ERROR_CHECK(gptimer_start(timer));
```

## Datasheet Lookup Protocol

When encountering an unfamiliar device connected to the ESP32:

1. **Identify the part number** from the PCB silkscreen or schematic
2. **Search online**: `"<part-number> datasheet"` (e.g., "BME280 datasheet", "SSD1306 datasheet")
3. **Check these sections**:
   - Absolute maximum ratings (voltage, current)
   - Electrical characteristics (logic levels, timing)
   - Communication interface (I2C address, SPI mode, max clock)
   - Register map (for configuration and data reading)
   - Timing diagrams (setup/hold times, access patterns)
4. **Verify voltage compatibility** with ESP32 (3.3V logic)
5. **Check for existing ESP-IDF component**: search `components.espressif.com`
6. **Check for Arduino library**: often a good reference even when using ESP-IDF
