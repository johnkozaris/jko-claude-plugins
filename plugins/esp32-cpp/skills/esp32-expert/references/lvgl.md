# LVGL on ESP32: Performance, Thread Safety & Animation Patterns

LVGL is the most common GUI framework for ESP32. It is also the easiest to make slow, crashy, or visually broken. Every rule here has been proven in production on real ESP32 hardware.

## Critical Setup: LVGL Integration Layer

### esp_lvgl_port vs esp_lvgl_adapter

| Component | Status | Use When |
|---|---|---|
| `esp_lvgl_port` | Stable, widely used | Most ESP32 boards (S3, C3, original ESP32) |
| `esp_lvgl_adapter` | Newer, ESP-IoT-Solution | ESP32-P4, MIPI-DSI displays, advanced tear avoidance |

**DO NOT mix APIs** from these two components. Check which one your BSP uses.

### Initialization Order (Mandatory)

```cpp
// 1. Initialize LVGL (creates task, timer)
// 2. Initialize display hardware (LCD controller, SPI/RGB/MIPI)
// 3. Register display with LVGL
// 4. Initialize touch hardware (I2C to touch controller)
// 5. Register touch input with LVGL
// 6. Start LVGL task
// 7. THEN create UI widgets
```

Calling LVGL widget functions before the display is registered causes `NULL` pointer crashes.

## Thread Safety (THE #1 Source of LVGL Crashes)

### The Rule

**LVGL is NOT thread-safe.** All LVGL API calls (`lv_*`) must happen either:
1. Inside the LVGL task (the internal worker task), OR
2. Protected by the LVGL mutex lock

### Lock Patterns

**esp_lvgl_port:**
```cpp
lvgl_port_lock(0);  // 0 = wait forever
lv_label_set_text(label, "Hello");
lvgl_port_unlock();
```

**esp_lvgl_adapter (BSP):**
```cpp
bsp_display_lock(-1);  // -1 = wait forever (milliseconds, NOT ticks!)
lv_label_set_text(label, "Hello");
bsp_display_unlock();
```

### CRITICAL BUG: Lock Timeout Units

`bsp_display_lock()` accepts **milliseconds** (converted internally). `pdMS_TO_TICKS()` also converts ms to ticks. Wrapping one in the other double-converts:

```cpp
// BAD: pdMS_TO_TICKS(50) = 50 ticks (at 1kHz). Passed as "50 milliseconds" to
// bsp_display_lock, which internally converts again. Result: correct at 1kHz tick
// BUT wrong at other tick rates, and confusing/misleading in code review.
bsp_display_lock(pdMS_TO_TICKS(50));  // Fragile: only accidentally correct at 1kHz

// GOOD: Pass milliseconds directly -- clear, always correct
bsp_display_lock(50);  // 50 milliseconds, no double-conversion
```

### Cross-Task UI Updates (The Safe Pattern)

Event handlers, WiFi callbacks, and sensor tasks run on different cores/tasks than LVGL. Never call LVGL directly from them.

```cpp
// BAD: Direct LVGL call from event handler (runs on Core 0, LVGL on Core 1)
void wifi_event_handler(void* arg, esp_event_base_t base, int32_t id, void* data) {
    lv_label_set_text(status_label, "Connected");  // CRASH: thread-unsafe
}

// GOOD: Use lv_async_call with heap-copied data
void wifi_event_handler(void* arg, esp_event_base_t base, int32_t id, void* data) {
    auto* msg = new (std::nothrow) std::string("Connected");
    if (!msg) return;

    bsp_display_lock(50);
    bool ok = lv_async_call([](void* param) {
        auto* s = static_cast<std::string*>(param);
        lv_label_set_text(status_label, s->c_str());
        delete s;  // Always delete in callback
    }, msg);
    bsp_display_unlock();

    if (!ok) delete msg;  // Delete on failure path too!
}
```

### Backpressure for Async Calls

If events arrive faster than LVGL processes them, the async queue overflows:

```cpp
static std::atomic<int> pending_async{0};
constexpr int MAX_PENDING = 32;

bool schedule_ui(lv_async_cb_t cb, void* data) {
    if (pending_async.load() >= MAX_PENDING) return false;  // Drop
    pending_async.fetch_add(1);
    bsp_display_lock(50);
    bool ok = lv_async_call(cb, data);
    bsp_display_unlock();
    if (!ok) pending_async.fetch_sub(1);
    return ok;
}
```

## Animation Performance (THE #2 Source of FPS Issues)

### The Frame Budget

LVGL redraws only dirty (invalidated) areas. The frame budget depends on:
- **Display interface**: SPI (~30fps max at 40MHz), RGB parallel (~60fps), MIPI-DSI (~60fps+)
- **Buffer config**: Single (slow), double (parallel render+flush), triple (tear-free)
- **Worker task timing**: `task_max_delay_ms` sets the FPS cap (15ms = ~67fps)

**If render time exceeds the frame budget, FPS drops. This is the fundamental animation performance issue.**

### What's CHEAP to Animate

| Operation | Cost | Why |
|---|---|---|
| Object position (`lv_obj_set_pos`) | Very low | Only old+new rect redrawn |
| Small object size (`lv_obj_set_size`) | Low | Delta area redrawn |
| Label text (`lv_label_set_text`) | Low | Text bounding box only |
| Color changes (solid fill) | Low | Simple fill, no blending |
| Arc rotation (`lv_arc_set_rotation`) | Moderate | Visible segment redrawn |
| Small object opacity | Moderate | Alpha blend on small area |

### What's EXPENSIVE to Animate (Avoid in Continuous Animations)

| Operation | Cost | Why |
|---|---|---|
| **Shadow properties** (width, opa, spread) | **EXTREME** | Shadow rendering cost scales with shadow_width and object area. Animated shadows recalculate every frame and bypass the shadow cache. Measured: shadow_width=40 on a 500px object can exceed 15ms render budget |
| Large object opacity | High | Entire bounding box redrawn with alpha blend |
| Large gradient backgrounds | High | RGB565 has only 32/64 color levels = visible banding |
| Border width animation | Medium but choppy | Only N discrete integer steps |
| Full-screen transitions | High | Entire display redrawn |
| Arc opacity on large arcs | High | Full arc redraw every step |

### Animation Anti-Patterns

**AP-LVGL-01: Animated Shadows**
```cpp
// BAD: Shadow recalculates Gaussian blur EVERY frame
lv_anim_t a;
lv_anim_set_values(&a, 0, 40);
lv_anim_set_exec_cb(&a, [](void* obj, int32_t v) {
    lv_obj_set_style_shadow_width((lv_obj_t*)obj, v, 0);  // Shadow recalculated every frame!
});
```
**Fix:** Use static shadows only. Enable `LV_SHADOW_CACHE_SIZE` (e.g., 64) to cache static shadows.

**AP-LVGL-02: Large Dirty Areas from Object Resize**
```cpp
// BAD: Resizing a 500px circle invalidates 500x500 = 250K pixel area
lv_anim_set_exec_cb(&a, [](void* obj, int32_t v) {
    lv_obj_set_size((lv_obj_t*)obj, v, v);  // 500px circle = huge dirty area
});

// GOOD: Move a small object across the screen instead
lv_anim_set_exec_cb(&a, [](void* obj, int32_t v) {
    lv_obj_set_x((lv_obj_t*)obj, v);  // 20px object = tiny dirty area
});
```

**AP-LVGL-03: Blocking Operations in LVGL Timer Callbacks**
```cpp
// BAD: HTTP request inside lv_timer callback (blocks LVGL for seconds)
void my_timer_cb(lv_timer_t* timer) {
    esp_http_client_perform(client);  // Blocks 100ms-5s!
    lv_label_set_text(label, response);
}

// GOOD: HTTP in separate task, update UI via lv_async_call
void http_task(void*) {
    esp_http_client_perform(client);
    char* copy = strdup(response);
    bsp_display_lock(50);
    lv_async_call([](void* p) {
        lv_label_set_text(label, (char*)p);
        free(p);
    }, copy);
    bsp_display_unlock();
}
```

**AP-LVGL-04: Using lv_timer for Animations Instead of lv_anim**
```cpp
// BAD: Manual position update in timer (no easing, no integration with LVGL timing)
void timer_cb(lv_timer_t* t) {
    static int x = 0;
    lv_obj_set_x(obj, x++);  // Linear, no easing, not synced with render
}

// GOOD: Use lv_anim (integrates with LVGL timer system, provides easing)
lv_anim_t a;
lv_anim_init(&a);
lv_anim_set_var(&a, obj);
lv_anim_set_values(&a, 0, 200);
lv_anim_set_time(&a, 1000);
lv_anim_set_path_cb(&a, lv_anim_path_ease_in_out);
lv_anim_set_exec_cb(&a, (lv_anim_exec_xcb_t)lv_obj_set_x);
lv_anim_start(&a);
```

**AP-LVGL-05: LV_USE_OS = LV_OS_FREERTOS Causing 100% CPU or Hangs**

Reported on ESP32-S3 (GitHub issue #8813, #6414): setting `LV_USE_OS` to `LV_OS_FREERTOS` can cause `lv_timer_handler` to hang in `lv_thread_sync_wait` or burn 100% CPU. Use `LV_OS_NONE` and manage the mutex externally (via `esp_lvgl_port` or `esp_lvgl_adapter`).

**AP-LVGL-06: Animation Tick Source Mismatch**

Using `lv_tick_set_cb()` with `xTaskGetTickCount` causes perceived lag because FreeRTOS tick resolution (1ms at 1000Hz) creates timing jitter in animations. Use `esp_timer_get_time() / 1000` for microsecond-accurate animation timing.

**AP-LVGL-07: PSRAM Frame Buffers on SPI Displays**

PSRAM is 3-10x slower than internal SRAM for random access. Placing frame buffers in PSRAM on SPI displays creates a bottleneck: CPU stalls waiting for PSRAM data while trying to fill the SPI DMA buffer. Internal SRAM buffers are critical for SPI displays. PSRAM is acceptable for MIPI-DSI/RGB displays where DMA2D handles the copy.

## Display Buffer Configuration

### Buffer Strategies

| Strategy | Buffers | Size | FPS | RAM Cost |
|---|---|---|---|---|
| Single small | 1 | 10-20 lines | Low | Lowest |
| Double small | 2 | 10-20 lines | Medium | Low |
| Double 1/10th | 2 | H_RES * V_RES/10 | Good | Medium |
| Double full | 2 | Full frame | Best | Highest |
| Triple partial | 3 | 50-line strips | Best (tear-free) | High |

**SPI displays:** Double buffer 1/10th screen in internal SRAM + DMA. `lv_disp_flush_ready()` called in DMA completion ISR.
**RGB parallel:** Double full-frame in PSRAM. Direct mode recommended.
**MIPI-DSI:** Triple partial with DMA2D (ESP32-P4). Hardware-synced tear avoidance.

### Flush Callback Pattern (SPI)

```cpp
static void flush_cb(lv_display_t* disp, const lv_area_t* area, uint8_t* px_map) {
    uint32_t size = lv_area_get_width(area) * lv_area_get_height(area) * 2;  // RGB565
    // Set display window
    esp_lcd_panel_draw_bitmap(panel, area->x1, area->y1, area->x2 + 1, area->y2 + 1, px_map);
    // DO NOT call lv_disp_flush_ready here! Call it in DMA completion callback.
}

// DMA completion ISR/callback
static bool on_trans_done(esp_lcd_panel_io_handle_t io, void* user_data, void* event_data) {
    lv_display_flush_ready((lv_display_t*)user_data);  // Signal LVGL: safe to render next frame
    return false;
}
```

**Calling `lv_disp_flush_ready()` in `flush_cb` before DMA completes** means LVGL starts rendering into the buffer while DMA is still reading it = torn frames.

## Performance Optimization Checklist

### Hardware-Level (Highest Impact)

| Setting | Impact | How |
|---|---|---|
| CPU frequency | **Critical** | `CONFIG_ESP_DEFAULT_CPU_FREQ_MHZ=240` (or 400 for P4) |
| Flash frequency + mode | **High** | 80MHz QIO or higher |
| Frame buffers in internal SRAM | **High** | `MALLOC_CAP_DMA` for SPI; PSRAM ok for RGB/MIPI |
| Compiler `-O2` | **High** | Significantly faster than `-Os` for LVGL rendering (measure on your target; `-Os` saves flash at the cost of render speed) |
| SPI bus speed | **High** | Push to 40-80MHz (verify with your display controller) |
| LVGL critical code in IRAM | **Medium** | `CONFIG_LV_ATTRIBUTE_FAST_MEM_USE_IRAM=y` |
| LVGL task on Core 1 | **Medium** | Avoids WiFi cache thrash on Core 0 |
| DMA2D/PPA for buffer copies | **Medium** | ESP32-P4 only |
| Draw buffer alignment | **Medium** | `CONFIG_LV_DRAW_BUF_ALIGN=64` for PPA (P4) |
| Dual-core rendering | **Low-Medium** | `CONFIG_LV_DRAW_SW_DRAW_UNIT_CNT=2` |

### Software-Level (Design Decisions)

| Practice | Impact |
|---|---|
| Animate position of small objects, not properties of large objects | **Critical** |
| Never animate shadow properties | **Critical** |
| Use `lv_anim` not manual timers | **High** |
| Keep dirty areas small | **High** |
| Use solid colors instead of gradients (RGB565 banding) | **Medium** |
| Limit font sizes and glyph ranges | **Medium** |
| Use `lv_obj_set_style_clip_corner` for round displays | **Medium** |
| Pre-render complex graphics as images | **Medium** |

## Common LVGL + ESP32 Libraries

### Display Drivers

| Display Type | ESP-IDF Component | Interface |
|---|---|---|
| ILI9341/9342 | `esp_lcd_ili9341` | SPI |
| ST7789 | `esp_lcd_st7789` | SPI |
| ST7796 | `esp_lcd_st7796` | SPI/RGB |
| SSD1306 (OLED) | `esp_lcd_ssd1306` | I2C/SPI |
| GC9A01 (round) | `esp_lcd_gc9a01` | SPI |
| JD9365 | `esp_lcd_jd9365` | MIPI-DSI (P4) |
| EK79007 | `esp_lcd_ek79007` | MIPI-DSI (P4) |

### Touch Controllers

| Controller | Component | Interface |
|---|---|---|
| GT911 | `esp_lcd_touch_gt911` | I2C |
| FT5x06/FT6x06 | `esp_lcd_touch_ft5x06` | I2C |
| CST816S | `esp_lcd_touch_cst816s` | I2C |
| XPT2046 | `esp_lcd_touch_xpt2046` | SPI (resistive) |

### Touch Gotchas

- **Coordinate transform**: `swap_xy`, `mirror_x`, `mirror_y` flags in touch config must match display rotation
- **I2C address conflicts**: GT911 can be 0x5D or 0x14 depending on reset timing
- **Touch + display on same SPI**: use different CS pins, mutex the bus
- **Touch interrupt**: prefer interrupt-driven over polling for lower latency and CPU usage

## Screen & Object Lifecycle (Memory Leak Hotspot)

### Screen Transition Rules

**DO**: Use `lv_scr_load_anim(new_scr, anim, time, delay, true)` -- `auto_del = true` auto-frees old screen.
**DO**: Update labels/values in-place (`lv_label_set_text`) instead of recreating entire screens.
**DO**: Use `lv_obj_del_async(obj)` when deleting objects from callbacks (avoids deleting self mid-execution).
**DO**: Monitor LVGL memory with `lv_mem_monitor()` during development.

**DON'T**: Manually `lv_obj_del()` a screen while an animated transition is still in progress -- crash.
**DON'T**: Use `lv_obj_clean()` + recreate pattern for periodic updates (temperature display, etc.) -- memory leak every cycle.
**DON'T**: Recreate image objects -- reuse with `lv_image_set_src()` instead. Each recreate leaks decoder cache.
**DON'T**: Create/destroy screens in a loop -- each cycle fragments LVGL's internal heap.

### Object Deletion Timing

```cpp
// BAD: Delete screen A while transition A→B animation is running
lv_scr_load_anim(screen_b, LV_SCR_LOAD_ANIM_FADE_ON, 500, 0, true);
lv_obj_del(screen_a);  // CRASH: animation still references screen_a

// BAD: Transition B→C while A→B animation not finished
lv_scr_load_anim(screen_b, LV_SCR_LOAD_ANIM_FADE_ON, 500, 0, true);
// 200ms later (animation still running):
lv_scr_load(screen_c);  // Old screen deleted mid-animation = crash

// GOOD: Let auto_del handle cleanup, or wait for animation to finish
lv_scr_load_anim(screen_b, LV_SCR_LOAD_ANIM_FADE_ON, 500, 0, true);
// screen_a auto-deleted after 500ms animation completes
```

### Image Caching Problems

- LVGL image cache can exhaust memory when decoding many PNG/JPEG images
- `lv_image_cache_drop(NULL)` may fail if entries are still referenced by active widgets
- JPEG decoder (TJPGD) has a known memory leak on decode failure (fixed in LVGL 9.3.1+)
- **Pattern**: Decode images once at startup, keep decoded `lv_image_dsc_t` in PSRAM, reference them

```cpp
// BAD: Reopen/decode PNG every time it's displayed
lv_image_set_src(img, "S:icon.png");  // Decodes from filesystem every call if not cached

// GOOD: Use pre-converted binary images (LVGL Image Converter tool)
LV_IMAGE_DECLARE(icon_img);  // Compiled into flash as C array
lv_image_set_src(img, &icon_img);  // Zero runtime decode, instant
```

## Display Tearing & Flickering Fixes

### SPI Displays

| Problem | Cause | Fix |
|---|---|---|
| Tearing during updates | Buffer swapped before DMA completes | Call `lv_disp_flush_ready` in DMA ISR, not in flush_cb |
| Flickering with WiFi active | WiFi DMA interrupts delay LCD DMA | Pin LVGL to Core 1, WiFi to Core 0 |
| Slow after rotation | Pixel access pattern changes, SPI throughput drops | Adjust SPI clock, use partial refresh |
| PSRAM buffer stalls | SPI DMA can't access PSRAM directly | Use internal SRAM for display buffers |

### RGB Parallel Displays

| Problem | Cause | Fix |
|---|---|---|
| Glitches/lines | PSRAM bandwidth contention with LCD DMA | Use bounce buffers in internal SRAM |
| Tearing | No VSync synchronization | Enable `on_vsync` callback for buffer swap |
| Frame drops under WiFi load | Shared memory bus saturated | Reduce display resolution or color depth |

### MIPI-DSI Displays (ESP32-P4)

| Problem | Cause | Fix |
|---|---|---|
| Tearing | Buffer swap not synced to VSync | Use `ESP_LV_ADAPTER_TEAR_AVOID_MODE_TRIPLE_PARTIAL` |
| Slow triple-buffer copy | Large dirty areas amplify 1MB+ framebuffer copies | Keep animated areas small |
| PPA alignment errors | Draw buffer not 64-byte aligned | `CONFIG_LV_DRAW_BUF_ALIGN=64` |

## BLE + LVGL Coexistence

BLE (especially Bluedroid) consumes 100KB+ heap. Combined with LVGL on ESP32 (520KB total SRAM), heap exhaustion is common.

**DO**: Use NimBLE instead of Bluedroid (~50KB less heap).
**DO**: Initialize BLE BEFORE LVGL display buffers (let BLE claim its heap first).
**DO**: Release BLE memory after provisioning if BLE is no longer needed.
**DON'T**: Enable Classic Bluetooth + LVGL on ESP32 without PSRAM -- not enough heap.

## Known LVGL 9 Bugs (Check Before Debugging)

| Bug | Version | Symptom | Workaround |
|---|---|---|---|
| Animation freeze on tick jump | v9.3.0+ (#9278) | Animations freeze if system tick jumps significantly | Ensure monotonic tick source |
| TJPGD memory leak on failure | v9.3.0 (#8579) | JPEG decode failure leaks memory | Update to 9.3.1+ |
| Direct mode double buffer glitch | v9.x (#6545) | Bar animations glitch with direct mode + double buffer | Use partial mode |
| `lv_timer_handler` hang with FreeRTOS OS | v9.x (#6414) | Hang in `lv_thread_sync_wait` | Use `LV_OS_NONE` + external mutex |
| 100% CPU with `LV_OS_FREERTOS` | v9.x (#8813) | CPU pegged even when idle | Use `LV_OS_NONE` + external mutex |
| Image cache entries not droppable | v9.0 (#5861) | Cache full but entries still referenced | Pre-convert to binary, avoid runtime decode |

## LVGL 9 Breaking Changes from v8

- `lv_meter` and `lv_gauge` removed -- use `lv_arc` + `lv_scale` + `lv_label`
- `lv_disp_drv_t` → `lv_display_t` (display driver API completely reworked)
- `lv_indev_drv_t` → `lv_indev_t` (input driver API reworked)
- Buffer allocation: `lv_disp_draw_buf_init()` removed -- buffers passed to display creation
- `lv_task_handler()` → `lv_timer_handler()`
- Color: `lv_color_t` → native types (`lv_color16_t`, etc.)
- Memory: `lv_mem_alloc/free` → standard `malloc/free` (or custom)
- Font converter output format changed -- regenerate all custom fonts
