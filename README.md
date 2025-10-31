# Motor RPM controller - For AVR - In Rust

This project contains the firmware for a motor speed RPM controller based on phase angle control.

It is written in Rust for AVR.

## Features

- PID controller for motor RPM regulation.
- Triac control for AC motor speed adjustment.
- Speed measurement from a magnet based speedometer generator.
- Temperature sensing for motor and microcontroller.
- Safety monitoring and safety shutoff.

## Hardware

- **Microcontroller:** Atmel ATTiny861A
- Open Source PCB

## Building the Firmware

To build the firmware, you need the following tools:

- Rust AVR compiler: A `nightly` compiler is required.
- `avr-gcc` toolchain (for linking and binary processing: `avr-ld`, `avr-objcopy`, `avr-size`, etc.)
- [dwdebug](https://github.com/mbuesch/dwire-debug) or `avrdude` for flashing.

Once the toolchains are installed, you can build the firmware by running `make`:

```bash
cd firmware
make
```

This will build the project and create the firmware files in the
`firmware/target/avr-attiny861a/release/`
directory.
The final hex file for flashing is
`firmware/target/avr-attiny861a/release/rpmcontrol.post.hex`.

## Flashing the Firmware

The `Makefile` provides targets for flashing the firmware using `avrdude` (for ISP) or `dwdebug` (for debugWire).

### ISP Flashing

First, set the fuses (this only needs to be done once):

```bash
cd firmware
make isp-fuses
```

Then, flash the firmware:

```bash
cd firmware
make isp-flash
```

### debugWire Flashing

To flash the firmware using debugWire:

```bash
cd firmware
make dw-flash
```

## License

This project is dual-licensed under either of the following:

*   Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
*   MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
