# Motor RPM controller - For AVR - In Rust

This project contains the firmware for a motor speed RPM controller based on phase angle control.

It is written in Rust for AVR.

## Features

- PID controller for motor RPM regulation.
- Triac control for AC motor speed adjustment.
- Speed measurement from a magnet-based speedometer generator.
- Temperature sensing for motor and microcontroller.
- Safety monitoring and safety shutoff.

## Restrictions

- The system is currently restricted to 50 Hz mains frequency.
  This limit can be lifted, if needed.
  Please open an issue if you need support for 60 Hz mains.

## Hardware

- **Microcontroller:** Atmel ATTiny861A
- Open Source PCB

## Building the Firmware

To build the firmware, you need the following tools:

- Rust AVR compiler: A `nightly` compiler is required.
- [avr-libc](https://github.com/avrdudes/avr-libc) AVR-specific C library.
- `avr-gcc` + `avr-binutils` toolchain (for linking and binary processing: `avr-ld`, `avr-objcopy`, `avr-size`, etc.)
- [avr-postprocess](https://github.com/mbuesch/avr-postprocess) for post-processing the compiled AVR machine code.
- [avra](https://github.com/Ro5bert/avra) for assembling AVR assembly code.
- Gnu `make`.
- [avrdude](https://github.com/avrdudes/avrdude) and [dwdebug](https://github.com/mbuesch/dwire-debug) (optional) for flashing.

Once the toolchain is installed, you can build the firmware by running `make`:

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

## Safety

Note that the AVR microcontroller is **not** a safety-certified controller.
Therefore, this project should not be used in any safety-critical application.

This project implements many safety features that should make it practically safe to use in many applications.
However, the safety features are not guaranteed to be sufficient for all applications and all failure modes and the usual functional safety standards are **not** followed.

**YOU ARE RESPONSIBLE FOR THE SAFETY OF YOUR APPLICATION.**

If you think there is a safety issue with the project, then do not use it.

## License

This project is dual-licensed under either of the following:

*   Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
*   MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
