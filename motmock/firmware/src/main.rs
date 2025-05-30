#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]

use avr_device::atmega8::{PORTB, PORTC, PORTD, Peripherals, TC1, TC2};

/// # Safety
///
/// Must only be called during init with IRQs disabled.
pub unsafe fn ports_init(pb: &PORTB, pc: &PORTC, pd: &PORTD) {
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
    pb.portb().write(|w| {
        // SAFETY: We are running in init with IRQs disabled.
        unsafe {
            w.bits(
                pin_low(0) | // n/c
                pin_low(1) | // mot
                pin_low(2) | // n/c
                pin_low(3) | // ISP MOSI
                pin_low(4) | // ISP MISO
                pin_low(5), // ISP SCK
            )
        }
    });
    pb.ddrb().write(|w| {
        // SAFETY: We are running in init with IRQs disabled.
        unsafe {
            w.bits(
                pin_output(0) | // n/c
                pin_output(1) | // mot
                pin_output(2) | // n/c
                pin_output(3) | // ISP MOSI
                pin_output(4) | // ISP MISO
                pin_output(5), // ISP SCK
            )
        }
    });

    // PORTC
    pc.portc().write(|w| {
        // SAFETY: We are running in init with IRQs disabled.
        unsafe {
            w.bits(
                pin_low(0) | // 50hz
                pin_low(1) | // n/c
                pin_low(2) | // n/c
                pin_low(3) | // n/c
                pin_low(4) | // n/c
                pin_low(5) | // n/c
                pin_low(6) | // n/c
                pin_low(7), // n/c
            )
        }
    });
    pc.ddrc().write(|w| {
        // SAFETY: We are running in init with IRQs disabled.
        unsafe {
            w.bits(
                pin_output(0) | // 50hz
                pin_output(1) | // n/c
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
    pd.portd().write(|w| {
        // SAFETY: We are running in init with IRQs disabled.
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
    pd.ddrd().write(|w| {
        // SAFETY: We are running in init with IRQs disabled.
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

const TIMER1_DUTYMAX: u16 = 0xFF;

#[rustfmt::skip]
fn timer1_init(tc1: &TC1) {
    tc1.icr1().write(|w| w.set(TIMER1_DUTYMAX));
    tc1.ocr1a().write(|w| w.set(0x00));
    tc1.tccr1a().write(|w| {
        w.com1a().match_clear()
         .com1b().disconnected()
         .wgm1().set(2)
    });
    tc1.tccr1b().write(|w| {
        w.cs1().prescale_256()
         .wgm1().set(2)
    });
}

fn timer1_duty(tc1: &TC1, duty: u16) {
    tc1.ocr1a().write(|w| w.set(duty));
}

const TIMER2_MAX: u8 = 78;
const TIMER2_CUTOFF: u8 = 2;

#[rustfmt::skip]
fn timer2_init(tc2: &TC2) {
    // Timer2 init; ctc, 100 Hz.
    tc2.ocr2().write(|w| w.set(TIMER2_MAX));
    tc2.tccr2().write(|w| {
        w.cs2().prescale_1024()
         .wgm20().clear_bit()
         .wgm21().set_bit()
    });
}

fn timer2_event(tc2: &TC2) -> bool {
    let ocf = tc2.tifr().read().ocf2().bit();
    if ocf {
        tc2.tifr().write(|w| w.ocf2().set_bit());
    }
    ocf
}

fn timer2_value(tc2: &TC2) -> u8 {
    tc2.tcnt2().read().bits().min(TIMER2_MAX)
}

#[avr_device::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    // SAFETY: We are running in init with IRQs disabled.
    unsafe {
        ports_init(&dp.PORTB, &dp.PORTC, &dp.PORTD);
    }
    timer1_init(&dp.TC1);
    timer2_init(&dp.TC2);

    let mut hz50 = false;
    let mut in_trig = false;
    loop {
        let trig = dp.PORTD.pind().read().pd2().bit();
        if trig && !in_trig {
            let mut val: u8 = TIMER2_MAX - timer2_value(&dp.TC2);
            if val <= TIMER2_CUTOFF {
                val = 0;
            }
            let val: u16 = val.into();

            let duty = val * (TIMER1_DUTYMAX + 1) / (TIMER2_MAX as u16 + 1);

            timer1_duty(&dp.TC1, duty);
        }
        in_trig = trig;

        hz50 ^= timer2_event(&dp.TC2);
        dp.PORTC.portc().modify(|_, w| w.pc0().bit(hz50));
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// vim: ts=4 sw=4 expandtab
