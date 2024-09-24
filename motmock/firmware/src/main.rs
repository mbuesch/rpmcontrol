#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]
#![feature(asm_const)]

use avr_device::atmega8::{Peripherals, PORTB, PORTC, PORTD};

pub fn ports_init(pb: &PORTB, pc: &PORTC, pd: &PORTD) {
    fn pin_input(_bit: usize) -> u8 {
        0
    }
    fn pin_output(bit: usize) -> u8 {
        1 << bit
    }
    fn pin_low(_bit: usize) -> u8 {
        0
    }
    fn pin_floating(_bit: usize) -> u8 {
        0
    }

    // PORTB
    pb.portb.write(|w| {
        unsafe {
            w.bits(
                pin_low(0) | // n/c
                pin_low(1) | // n/c
                pin_low(2) | // n/c
                pin_low(3) | // ISP MOSI
                pin_low(4) | // ISP MISO
                pin_low(5), // ISP SCK
            )
        }
    });
    pb.ddrb.write(|w| {
        unsafe {
            w.bits(
                pin_output(0) | // n/c
                pin_output(1) | // n/c
                pin_output(2) | // n/c
                pin_output(3) | // ISP MOSI
                pin_output(4) | // ISP MISO
                pin_output(5), // ISP SCK
            )
        }
    });

    // PORTC
    pc.portc.write(|w| {
        unsafe {
            w.bits(
                pin_low(0) | // 50hz
                pin_low(1) | // mot
                pin_low(2) | // n/c
                pin_low(3) | // n/c
                pin_low(4) | // n/c
                pin_low(5) | // n/c
                pin_low(6) | // n/c
                pin_low(7), // n/c
            )
        }
    });
    pc.ddrc.write(|w| {
        unsafe {
            w.bits(
                pin_output(0) | // 50hz
                pin_output(1) | // mot
                pin_output(2) | // n/c
                pin_output(3) | // n/c
                pin_output(4) | // n/c
                pin_output(5) | // n/c
                pin_output(6) | // n/c
                pin_output(7), // n/c
            )
        }
    });

    // PORTD
    pd.portd.write(|w| {
        unsafe {
            w.bits(
                pin_low(0) | // n/c
                pin_low(1) | // n/c
                pin_floating(2) | // trig
                pin_low(3) | // n/c
                pin_low(4) | // n/c
                pin_low(5) | // n/c
                pin_low(6) | // n/c
                pin_low(7), // n/c
            )
        }
    });
    pd.ddrd.write(|w| {
        unsafe {
            w.bits(
                pin_output(0) | // n/c
                pin_output(1) | // n/c
                pin_input(2) |  // trig
                pin_output(3) | // n/c
                pin_output(4) | // n/c
                pin_output(5) | // n/c
                pin_output(6) | // n/c
                pin_output(7), // n/c
            )
        }
    });
}

#[avr_device::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    ports_init(&dp.PORTB, &dp.PORTC, &dp.PORTD);

    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// vim: ts=4 sw=4 expandtab
