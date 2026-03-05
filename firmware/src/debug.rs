// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use crate::timer::RelLargeTimestamp;
use avr_context::{InitCtx, IrqCtx};
use avr_q::Q7p8;

#[cfg_attr(not(feature = "debug"), allow(dead_code))]
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Debug {
    Speedo,
    SpeedoStatus,
    Setpoint,
    PidY,
    MonDebounce,
    TempMot,
    TempUc,
    MaxRt,
    MinStack,
}
#[cfg_attr(not(feature = "debug"), allow(dead_code))]
const NRVALUES: usize = 9;

#[cfg_attr(not(feature = "debug"), allow(dead_code))]
#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn rx_complete_callback(c: &IrqCtx, data: u8) {
    #[cfg(feature = "debug")]
    inner::rx_complete_callback(c, data);
}

#[cfg_attr(not(feature = "debug"), allow(dead_code))]
#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn tx_complete_callback(c: &IrqCtx) {
    #[cfg(feature = "debug")]
    inner::tx_complete_callback(c);
}

impl Debug {
    #[cfg_attr(not(feature = "debug"), allow(unused_variables))]
    pub fn log_u16(&self, value: u16) {
        #[cfg(feature = "debug")]
        inner::log_u16(*self as u16, value);
    }

    #[cfg_attr(not(feature = "debug"), allow(dead_code))]
    pub fn log_i16(&self, value: i16) {
        self.log_u16(value as u16);
    }

    #[cfg_attr(not(feature = "debug"), allow(dead_code))]
    pub fn log_u8(&self, value: u8) {
        self.log_u16(value.into());
    }

    #[cfg_attr(not(feature = "debug"), allow(dead_code))]
    pub fn log_fixpt(&self, value: Q7p8) {
        self.log_u16(value.to_q() as _);
    }

    #[cfg_attr(not(feature = "debug"), allow(dead_code))]
    pub fn log_rel_large_timestamp(&self, value: RelLargeTimestamp) {
        self.log_i16(value.into());
    }
}

#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn setup(c: &InitCtx) {
    #[cfg(feature = "debug")]
    inner::setup(c);
}

#[cfg(feature = "debug")]
mod inner {
    use super::*;
    use crate::usi_uart::uart_tx_cs;
    use avr_context::{Mutex, with_cs};
    use core::cell::Cell;

    const INDEXSHIFT: usize = 2;
    const INDEXMASK: u8 = (1 << INDEXSHIFT) - 1;

    static VALUES: Mutex<[Cell<u16>; NRVALUES]> = Mutex::new([
        Cell::new(0),
        Cell::new(0),
        Cell::new(0),
        Cell::new(0),
        Cell::new(0),
        Cell::new(0),
        Cell::new(0),
        Cell::new(0),
        Cell::new(0),
    ]);
    static INDEX: Mutex<Cell<u8>> = Mutex::new(Cell::new(0));

    pub fn rx_complete_callback(_c: &IrqCtx, _data: u8) {}

    #[allow(clippy::collapsible_match)]
    pub fn tx_complete_callback(c: &IrqCtx) {
        let cs = c.cs();
        let index = INDEX.borrow(cs).get();
        let id = index >> INDEXSHIFT;
        let txindex = index & INDEXMASK;

        let value = if id < NRVALUES as u8 {
            VALUES.borrow(cs)[id as usize].get()
        } else {
            0xFFFF
        };

        match txindex {
            0 => {
                let data = if id < NRVALUES as u8 { id } else { 0xFF };
                if uart_tx_cs(cs, data) {
                    INDEX.borrow(cs).set(index + 1);
                }
            }
            1 => {
                if uart_tx_cs(cs, value as u8) {
                    INDEX.borrow(cs).set(index + 1);
                }
            }
            2 => {
                if uart_tx_cs(cs, (value >> 8) as u8) {
                    if id >= NRVALUES as u8 {
                        INDEX.borrow(cs).set(0);
                    } else {
                        INDEX.borrow(cs).set((id + 1) << INDEXSHIFT);
                    }
                }
            }
            _ => (),
        }
    }

    pub fn log_u16(id: u16, value: u16) {
        with_cs(|cs| {
            let id = id as usize;
            let values = VALUES.borrow(cs);
            if id < values.len() {
                values[id].set(value);
            }
        });
    }

    pub fn setup(c: &InitCtx) {
        uart_tx_cs(c.cs(), 0);
    }
}

// vim: ts=4 sw=4 expandtab
