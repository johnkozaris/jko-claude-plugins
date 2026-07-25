# Peripherals

Start with the exact part number, board schematic, voltage levels, pins,
pull-ups, bus speed, and target-chip peripheral matrix. Read the datasheet and
errata when electrical or timing behavior matters.

Give each bus a clear owner or serialize transactions with the correct
primitive. A timeout can indicate a held line, missing pull-up, wrong mode,
clock stretching, another task using the bus, an unpowered device, or a driver
state problem; increasing the timeout distinguishes none of them.

For SPI and display paths, verify DMA-capable buffer placement, transaction
lifetime, chip-select behavior, and transfer-size limits. For UART, bound input,
handle partial frames, and define overflow recovery. For GPIO, check boot
strapping, input-only pins, wake behavior, and external circuitry.

ADC, touch, radio, and low-power peripheral interactions vary by chip. Do not
copy limitations from the original ESP32 to a newer RISC-V or S-series target
without checking current documentation.

Record a logic-analyzer trace or register-level evidence when software logs
cannot distinguish protocol from electrical failure.
