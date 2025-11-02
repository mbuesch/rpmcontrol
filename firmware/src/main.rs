// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]

mod analog;
mod debounce;
mod debug;
mod exint;
mod filter;
mod history;
mod hw;
mod mains;
mod mon;
mod mon_pocheck;
mod mon_stack;
mod pid;
mod ports;
mod ring;
mod shutoff;
mod speedo;
mod system;
mod temp;
mod timer;
mod triac;
mod usi_uart;

use crate::{
    debug::debug_init,
    hw::{Peripherals, mcu},
    system::System,
};
use avr_context::{InitCtx, MainCtx, define_main};
use avr_device::asm::wdr;

static SYSTEM: System = System::new();

/// Reset the system.
#[inline(always)]
#[allow(clippy::empty_loop)]
pub fn reset_system() -> ! {
    loop {
        // Wait for the watchdog timer to trigger and reset the system.
        // We don't need to disable interrupts here.
        // No interrupt will reset the watchdog timer.
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init3")]
/// Watchdog timer initialization.
///
/// # Safety
///
/// This naked function is run before main() from the .init3 section.
pub unsafe extern "C" fn wdt_init() {
    // Enable WDT with timeout 32.5 ms
    core::arch::naked_asm!(
        "ldi r16, 0x10", // WDCE=1
        "out {WDTCR}, r16",
        "ldi r16, 0x19", // WDCE=1, WDE=1, WDP2=0, WDP1=0, WDP0=1
        "out {WDTCR}, r16",
        WDTCR = const 0x21,
    );
}

#[allow(non_snake_case)]
struct Dp {
    ADC: mcu::ADC,
}

#[inline(always)]
fn init(c: &InitCtx<'_>, dp: Peripherals) -> Dp {
    let porta = ports::PORTA.init(c, ports::PortA { PORTA: dp.PORTA });
    let portb = ports::PORTB.init(c, ports::PortB { PORTB: dp.PORTB });
    let exint = exint::EXINT.init(c, exint::ExInt { EXINT: dp.EXINT });
    let timer = timer::DP.init(c, timer::Dp { TC1: dp.TC1 });
    let usi_uart = usi_uart::DP.init(
        c,
        usi_uart::Dp {
            USI: dp.USI,
            TC0: dp.TC0,
        },
    );

    timer.setup(c);
    porta.setup(c);
    portb.setup(c);
    exint.setup(c);
    usi_uart.setup(c);
    debug_init(c);

    SYSTEM.init(c.main_ctx(), &dp.ADC, &dp.AC);

    Dp { ADC: dp.ADC }
}

#[inline(always)]
fn main(c: &MainCtx<'_>, dp: Dp) -> ! {
    loop {
        SYSTEM.run(c, &dp.ADC);
        wdr();
    }
}

define_main! {
    device: attiny861a,
    init: init,
    main: main,
    enable_interrupts: true,
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    reset_system();
}

// vim: ts=4 sw=4 expandtab
