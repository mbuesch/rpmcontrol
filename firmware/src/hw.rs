// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use avr_context::define_isr;

pub use avr_device::{attiny861a as mcu, interrupt};

use crate::{
    analog::irq_handler_ana_comp, exint::irq_handler_pcint, timer::irq_handler_timer1_compa,
    usi_uart::irq_handler_usi_ovf,
};

define_isr! {
    device: attiny861a,
    interrupt: PCINT,
    isr: irq_handler_pcint,
}
define_isr! {
    device: attiny861a,
    interrupt: TIMER1_COMPA,
    isr: irq_handler_timer1_compa,
}
define_isr! {
    device: attiny861a,
    interrupt: USI_OVF,
    isr: irq_handler_usi_ovf,
}
define_isr! {
    device: attiny861a,
    interrupt: ANA_COMP,
    isr: irq_handler_ana_comp,
}

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
