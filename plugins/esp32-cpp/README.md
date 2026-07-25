# ESP32 C++

Evidence-first firmware guidance for ESP32 projects using ESP-IDF, Arduino, or
PlatformIO.

The skill focuses on hardware and runtime facts a generic C++ review can miss:
execution context, chip capabilities, memory regions, DMA, FreeRTOS behavior,
peripherals, power, and long-uptime failure modes. Detailed references load only
when the project or failure report exposes a matching signal.

Where plugin commands are exposed, use `/esp32-cpp:esp-debug <symptom>` for an
explicit crash or field-failure investigation; otherwise request it directly.
