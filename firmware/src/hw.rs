// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use avr_context::IrqCtx;

pub use attiny::{self as mcu, Peripherals};
pub use avr_device::attiny861a as attiny;
pub use avr_device::interrupt;

macro_rules! define_isr {
    ($name:ident, $handler:path) => {
        #[avr_device::interrupt(attiny861a)]
        fn $name() {
            // SAFETY: We are inside of an interrupt handler.
            // Therefore, it is safe to construct an `IrqCtx`.
            let c = unsafe { IrqCtx::new() };
            $handler(&c);
        }
    };
}

define_isr!(PCINT, crate::exint::irq_handler_pcint);
define_isr!(TIMER1_COMPA, crate::timer::irq_handler_timer1_compa);
define_isr!(USI_OVF, crate::usi_uart::irq_handler_usi_ovf);
define_isr!(ANA_COMP, crate::analog::irq_handler_ana_comp);

/// Do nothing for the duration of 3 CPU cycles.
#[inline(always)]
#[rustfmt::skip]
pub fn nop3() {
    // SAFETY: Asm block doesn't access anything.
    unsafe {
        core::arch::asm!(
            "rjmp 1",
            "1: nop",
            options(preserves_flags)
        )
    }
}

// vim: ts=4 sw=4 expandtab
