#!/bin/sh
set -e
export AVR_CPU_FREQUENCY_HZ=16000000
cargo build --target avr-attiny26.json -Z build-std=core --release
avr-objcopy -R.eeprom -O ihex target/avr-attiny26/release/rpmcontrol.elf target/avr-attiny26/release/rpmcontrol.hex
avr-objdump --disassemble target/avr-attiny26/release/rpmcontrol.elf > target/avr-attiny26/release/rpmcontrol.elf.dasm
avr-size  --format=SysV target/avr-attiny26/release/rpmcontrol.elf
