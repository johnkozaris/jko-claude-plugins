# Power and field reliability

Power behavior is a hardware/software contract. Measure current in the target
board configuration and account for regulators, pull-ups, sensors, radios,
USB-UART bridges, and floating pins rather than quoting chip-only sleep figures.

Before deep sleep, define what state must persist, which wake sources are legal,
what code executes before flash is available, and how repeated wake failures
recover. Verify RTC-memory and wake-stub constraints for the target.

Brownouts, noisy supplies, radio transmit peaks, and peripheral inrush can look
like random firmware resets. Capture reset reason and rail behavior before
changing watchdog or retry logic.

Bound NVS and flash writes. Avoid write-on-every-loop or write-on-every-boot
patterns when values can be coalesced, wear-levelled, or persisted only after
meaningful change.

Run soak tests that include reconnects, time rollover, queue saturation,
allocation churn, sleep/wake cycles, OTA interruption, and peripheral faults.
Report what ran on real hardware and for how long.
