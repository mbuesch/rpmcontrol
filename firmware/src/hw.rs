// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use avr_context::define_isr;

pub use attiny::{self as mcu, Peripherals};
pub use avr_device::attiny861a as attiny;
pub use avr_device::interrupt;

use crate::{
    analog::irq_handler_ana_comp, exint::irq_handler_pcint, timer::irq_handler_timer1_compa,
    usi_uart::irq_handler_usi_ovf,
};

define_isr!(attiny861a, PCINT, irq_handler_pcint);
define_isr!(attiny861a, TIMER1_COMPA, irq_handler_timer1_compa);
define_isr!(attiny861a, USI_OVF, irq_handler_usi_ovf);
define_isr!(attiny861a, ANA_COMP, irq_handler_ana_comp);

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
