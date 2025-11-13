// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use crate::DP_EXINT;
use avr_context::{InitCtx, IrqCtx};

const PCINT_ENA_0: bool = false;
const PCINT_ENA_1: bool = true; // PA1: Mains vsense.
const PCINT_ENA_2: bool = false;
const PCINT_ENA_3: bool = false;
const PCINT_ENA_4: bool = false;
const PCINT_ENA_5: bool = false;
const PCINT_ENA_6: bool = false;
const PCINT_ENA_7: bool = false;
const PCINT_ENA_8: bool = false;
const PCINT_ENA_9: bool = false;
const PCINT_ENA_10: bool = false;
const PCINT_ENA_11: bool = false;
const PCINT_ENA_12: bool = false;
const PCINT_ENA_13: bool = false;
const PCINT_ENA_14: bool = false;
const PCINT_ENA_15: bool = false;

#[allow(clippy::identity_op)]
pub fn setup(c: &InitCtx) {
    DP_EXINT.initctx(c).pcmsk0().write(|w| {
        w.set(
            ((PCINT_ENA_0 as u8) << 0)
                | ((PCINT_ENA_1 as u8) << 1)
                | ((PCINT_ENA_2 as u8) << 2)
                | ((PCINT_ENA_3 as u8) << 3)
                | ((PCINT_ENA_4 as u8) << 4)
                | ((PCINT_ENA_5 as u8) << 5)
                | ((PCINT_ENA_6 as u8) << 6)
                | ((PCINT_ENA_7 as u8) << 7),
        )
    });
    DP_EXINT.initctx(c).pcmsk1().write(|w| {
        w.set(
            ((PCINT_ENA_8 as u8) << 0)
                | ((PCINT_ENA_9 as u8) << 1)
                | ((PCINT_ENA_10 as u8) << 2)
                | ((PCINT_ENA_11 as u8) << 3)
                | ((PCINT_ENA_12 as u8) << 4)
                | ((PCINT_ENA_13 as u8) << 5)
                | ((PCINT_ENA_14 as u8) << 6)
                | ((PCINT_ENA_15 as u8) << 7),
        )
    });
    DP_EXINT.initctx(c).gifr().write(|w| w.pcif().set_bit());
    DP_EXINT.initctx(c).gimsk().write(|w| w.pcie().set(0x3));
}

pub fn irq_handler_pcint(c: &IrqCtx) {
    crate::mains::irq_handler_pcint(c);
    crate::usi_uart::irq_handler_pcint(c);
}

// vim: ts=4 sw=4 expandtab
