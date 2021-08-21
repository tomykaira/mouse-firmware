# mouse-firmware: Proof of work USB HID mouse implementation for pmw3360

## Setup

Install rust and stm32 environment.
Ref: [The embedded rust book](https://docs.rust-embedded.org/book/).

## Expected environment

- PMW3360 board that accepts 3.3V.
- STM32F103 MCU with SPI, USB, 3.3V. I use [bluepill](https://stm32-base.org/boards/STM32F103C8T6-Blue-Pill.html).

Circuit:

- Provide VCC and GND.
- Connect the same pins of MOSI, MISO, SCK.
- Connect NCS to GND.

## Build & run

Prepare openocd.

```
cargo.exe run --release
```

If you want semihosting (print debug, get panic messages), enable `semihosting` feature.

```
cargo.exe run --release --features semihosting
```

Then the mouse device will appear in the USB device list, and mouse cursor starts to move to right with constant speed.

You can control Y with PMW3360 sensor.

## Future work

Pull requests are very welcome.

- Support BURST mode
- Support MOT

## Thanks

eucalyn_ for a small pmw3360 dev board.
