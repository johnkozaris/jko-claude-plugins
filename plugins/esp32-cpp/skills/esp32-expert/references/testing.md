# Firmware verification

Use layers of evidence:

- host tests for pure parsers, state machines, and protocol logic;
- target tests for SDK, timing, memory-region, and peripheral behavior;
- hardware-in-the-loop for electrical integration and recovery;
- static analysis and compiler warnings for language-level defects;
- soak and fault-injection tests for long-uptime behavior.

Make hardware assumptions explicit in test metadata: board revision, chip,
flash/PSRAM size, SDK, configuration, attached peripherals, and power source.

Test failure paths that are difficult to reproduce manually: queue full,
allocation failure, malformed frame, bus stuck low, network loss, expired
credential, partial OTA, brownout, reboot during persistence, and repeated
sleep/wake.

A successful build proves only compilation. A simulator or host test cannot
prove ISR placement, DMA compatibility, radio behavior, or electrical timing.
State the strongest evidence actually obtained.
