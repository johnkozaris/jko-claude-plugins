# Displays and LVGL

Use this reference only when the project actually uses LVGL or a comparable
framebuffer/display pipeline.

Identify which task owns LVGL calls and how other tasks hand it updates. LVGL
APIs are generally not safe to call concurrently without the integration's
chosen serialization mechanism.

Verify display-buffer size, color format, DMA capability, transfer completion,
cache behavior, and whether single/double buffering matches the driver. Tearing,
partial updates, and corrupted colors often originate at that boundary rather
than in widget code.

Avoid recreating entire object trees for periodic value changes. Update owned
objects, release screens and resources deliberately, and monitor heap behavior
across repeated navigation.

Measure render time, flush time, task occupancy, and missed frames before
reducing animation or refresh rate. Keep UI work bounded so it cannot starve
network, input, or watchdog-sensitive tasks.

Check the installed LVGL major version and integration layer before suggesting
API names; lifecycle and driver interfaces change between releases.
