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
    hw::{Peripherals, interrupt},
    system::System,
};
use avr_context::{InitCtx, MainCtx};
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

struct Init {
    porta: ports::PortA,
    portb: ports::PortB,
    exint: exint::ExInt,
    timer: timer::Dp,
    usi: usi_uart::Dp,
}

fn initialize(c: &InitCtx, init: Init) {
    let porta = ports::PORTA.init(c, init.porta);
    let portb = ports::PORTB.init(c, init.portb);
    let exint = exint::EXINT.init(c, init.exint);
    let timer = timer::DP.init(c, init.timer);
    let usi_uart = usi_uart::DP.init(c, init.usi);

    timer.setup(c);
    porta.setup(c);
    portb.setup(c);
    exint.setup(c);
    usi_uart.setup(c);
    debug_init(c);
}

#[avr_device::entry]
fn main() -> ! {
    // SAFETY: We only call Peripherals::steal() once.
    let dp = unsafe { Peripherals::steal() };

    let init = Init {
        porta: ports::PortA { PORTA: dp.PORTA },
        portb: ports::PortB { PORTB: dp.PORTB },
        exint: exint::ExInt { EXINT: dp.EXINT },
        timer: timer::Dp { TC1: dp.TC1 },
        usi: usi_uart::Dp {
            USI: dp.USI,
            TC0: dp.TC0,
        },
    };

    // SAFETY:
    // This is the context handle for the main() function.
    // Holding a reference to this object proves that the holder
    // is running in main() context.
    let m = unsafe { MainCtx::new_with_init(initialize, init) };

    SYSTEM.init(&m, &dp.ADC, &dp.AC);

    // SAFETY:
    // This must be after construction of MainCtx
    // and after initialization of static InitCtx variables.
    unsafe { interrupt::enable() };

    loop {
        SYSTEM.run(&m, &dp.ADC);
        wdr();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    reset_system();
}

// vim: ts=4 sw=4 expandtab
