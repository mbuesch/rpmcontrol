// -*- coding: utf-8 -*-
// Copyright (C) 2025 Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{
    fixpt::Fixpt,
    hw::interrupt,
    mutex::{IrqCtx, MainInitCtx, Mutex},
    usi_uart::uart_tx_cs,
};
use core::cell::Cell;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Debug {
    Speedo,
    SpeedoStatus,
    Setpoint,
    PidY,
    MonDebounce,
}
const NRVALUES: usize = 5;

const INDEXSHIFT: usize = 2;
const INDEXMASK: u8 = (1 << INDEXSHIFT) - 1;

static VALUES: Mutex<[Cell<u16>; NRVALUES]> = Mutex::new([
    Cell::new(0),
    Cell::new(0),
    Cell::new(0),
    Cell::new(0),
    Cell::new(0),
]);
static INDEX: Mutex<Cell<u8>> = Mutex::new(Cell::new(0));

pub fn rx_complete_callback(_c: &IrqCtx, _data: u8) {
    //TODO
}

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

impl Debug {
    pub fn log_u16(&self, value: u16) {
        interrupt::free(|cs| {
            let id = *self as usize;
            let values = VALUES.borrow(cs);
            if id < values.len() {
                values[id].set(value);
            }
        });
    }

    pub fn log_u8(&self, value: u8) {
        self.log_u16(value.into())
    }

    pub fn log_fixpt(&self, value: Fixpt) {
        self.log_u16(value.to_q() as _);
    }
}

pub fn debug_init(c: &MainInitCtx) {
    uart_tx_cs(c.cs(), 0);
}

// vim: ts=4 sw=4 expandtab
