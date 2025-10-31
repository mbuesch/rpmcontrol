// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]

use crate::filter::Filter;
use avr_device::atmega8::{PORTB, PORTC, PORTD, Peripherals, TC1, TC2};

mod filter;

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
        w.cs1().prescale_64()
         .wgm1().set(2)
    });
}

fn timer1_duty(tc1: &TC1, duty: u16) {
    tc1.ocr1a().write(|w| w.set(duty));
    if tc1.tcnt1().read().bits() > duty {
        tc1.tcnt1().write(|w| w.set(0));
    }
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

    let mut triggered = false;
    let mut duty_filter = Filter::new();
    const DUTY_FILTER_SHIFT: u8 = 2;
    loop {
        // Simulated mains zero crossing.
        let zerocrossing = timer2_event(&dp.TC2);

        // 50 Hz output.
        dp.PORTC
            .portc()
            .modify(|r, w| w.pc0().bit(r.pc0().bit() ^ zerocrossing));

        // Triac trigger input.
        if zerocrossing {
            if !triggered {
                // No trigger -> Turn PWM off.
                let duty = duty_filter.run(0, DUTY_FILTER_SHIFT);
                timer1_duty(&dp.TC1, duty);
            }
            triggered = false;
        }
        let trig = !dp.PORTD.pind().read().pd2().bit();
        if trig && !triggered {
            triggered = true;

            // Output PWM proportionally to triac trigger time.
            let mut val: u8 = TIMER2_MAX - timer2_value(&dp.TC2);
            if val <= TIMER2_CUTOFF {
                val = 0;
            }
            let val: u16 = val.into();

            let duty = val * (TIMER1_DUTYMAX + 1) / (TIMER2_MAX as u16 + 1);
            let duty = (duty * 3) / 4; // reduce
            let duty = duty_filter.run(duty, DUTY_FILTER_SHIFT);

            timer1_duty(&dp.TC1, duty);
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// vim: ts=4 sw=4 expandtab
