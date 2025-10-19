# AC motor speed controller

This is a closed-loop AC motor speed controller.
It uses phase-angle control with a triac to regulate the motor's speed.
The firmware is written in Rust and runs on an ATtiny861A microcontroller.

Note that currently only 50 Hz mains is supported, but adding support for other mains frequencies is possible.

## Components

- `firmware`: The main controller firmware that runs on the ATtiny861A.
- `schematic`: The hardware design files in KiCad format.
- `debugtool`: A GTK4-based GUI application for debugging and monitoring the controller over a serial connection.
- `motmock`: A motor simulator for testing the controller. It runs on an ATmega8 and simulates the behavior of a motor.

## Hardware

The controller is based on the ATtiny861A microcontroller.
It uses a triac for phase-angle control of the AC motor.
The schematic directory contains the full hardware design.

## Software

The firmware and debugging tools are written in Rust.

The firmware implements a closed-loop PID controller to maintain the desired motor speed. It includes modules for:

- PID control
- Triac control
- Motor speed measurement
- Mains zero crossing detection and synchronization
- Temperature monitoring
- Safety monitoring and secondary shutoff path
- Serial communication for debugging

### Debug Tool

The debug tool is a GTK4 application that visualizes the controller's internal state.
It communicates with the firmware over a serial port and can display information such as:

- PID controller state
- Actual motor speed and setpoint
- Temperatures
- Safety monitoring state

## Building the Firmware

To build the firmware, you will need a `nightly` Rust toolchain.
Once the toolchain is set up, you can build the firmware by running `make` in the `firmware` directory:

```sh
cd firmware
make
```

## Running the Debug Tool

The debug tool can be run from the `debugtool` directory.
It takes an optional command-line argument to specify the serial port to use.

```sh
cd debugtool
cargo run -- /dev/ttyUSB0
```
