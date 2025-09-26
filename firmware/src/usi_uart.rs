// -*- coding: utf-8 -*-
// Copyright (C) 2025 Michael Büsch <m@bues.ch>
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::{
    debug,
    hw::{interrupt, mcu},
    mutex::{CriticalSection, IrqCtx, LazyMainInit, MainInitCtx, Mutex},
    ports::PORTB,
};
use core::cell::Cell;

const FCPU: u32 = 16_000_000;
const BAUD: u32 = 19_200;
const PORTB_BIT: usize = 1;
const TC0_PS: u32 = 8;
const TC0_OCR: u8 = (FCPU / (BAUD * TC0_PS)) as u8;

#[allow(non_snake_case)]
pub struct Dp {
    pub USI: mcu::USI,
    pub TC0: mcu::TC0,
}

// SAFETY: Is initialized when constructing the MainCtx.
pub static DP: LazyMainInit<Dp> = unsafe { LazyMainInit::uninit() };

fn bit_rev(mut data: u8) -> u8 {
    data = (data & 0xF0) >> 4 | (data & 0x0F) << 4;
    data = (data & 0xCC) >> 2 | (data & 0x33) << 2;
    data = (data & 0xAA) >> 1 | (data & 0x55) << 1;
    data
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Rx,
    Tx0,
    Tx1,
}

static MODE: Mutex<Cell<Mode>> = Mutex::new(Cell::new(Mode::Rx));
static TXDATA: Mutex<Cell<u8>> = Mutex::new(Cell::new(0));

impl Dp {
    pub fn setup(&self, _c: &MainInitCtx) {
        self.USI.usidr().write(|w| w.set(0xFF));
        //TODO enable PCINT
    }
}

#[allow(unused)]
pub fn irq_handler_pcint(c: &IrqCtx) {
    let cs = c.cs();
    let mode = MODE.borrow(cs);
    match mode.get() {
        Mode::Rx => {
            //TODO
        }
        Mode::Tx0 | Mode::Tx1 => (),
    }
}

#[rustfmt::skip]
pub fn irq_handler_usi_ovf(c: &IrqCtx) {
    let cs = c.cs();
    let mode = MODE.borrow(cs);
    match mode.get() {
        Mode::Rx => {
            let data = bit_rev(DP.USI.usidr().read().bits());

            DP.TC0.tccr0b().write(|w| w);

            DP.USI.usicr().modify(|_, w| w.usioie().clear_bit());
            DP.USI.usisr().modify(|_, w| w.usioif().set_bit());

            //TODO

            debug::rx_complete_callback(c, data);
        }
        Mode::Tx0 => {
            let data = TXDATA.borrow(cs).get();
            DP.USI.usidr().write(|w| w.set((data << 3) | 0x07));
            DP.USI.usisr().write(|w| {
                w.usicnt().set(16 - 6)
                 .usioif().set_bit()
            });

            mode.set(Mode::Tx1);
        }
        Mode::Tx1 => {
            DP.USI.usidr().write(|w| w.set(0xFF));
            DP.USI.usicr().modify(|_, w| w.usioie().clear_bit());
            DP.USI.usisr().modify(|_, w| w.usioif().set_bit());

            DP.TC0.tccr0b().write(|w| w);

            PORTB.set(PORTB_BIT, true);
            PORTB.input(PORTB_BIT);

            //TODO enable PCINT

            mode.set(Mode::Rx);
            debug::tx_complete_callback(c);
        }
    }
}

#[rustfmt::skip]
pub fn uart_tx_cs(cs: CriticalSection<'_>, mut data: u8) -> bool {
    let mode = MODE.borrow(cs);
    match mode.get() {
        Mode::Rx => {
            data = bit_rev(data);
            TXDATA.borrow(cs).set(data);

            DP.TC0.tccr0b().write(|w| w);

            PORTB.set(PORTB_BIT, true);
            PORTB.output(PORTB_BIT);

            DP.USI.usidr().write(|w| w.set((data >> 2) | 0x80));
            DP.USI.usisr().write(|w| {
                w.usicnt().set(16 - 5)
                 .usioif().set_bit()
            });
            DP.USI.usicr().write(|w| {
                w.usioie().set_bit()
                 .usiwm().three_wire()
                 .usics().tc0()
            });
            DP.USI.usipp().write(|w| w);

            DP.TC0.tccr0a().write(|w| w.ctc0().set_bit());
            DP.TC0.tcnt0h().write(|w| w);
            DP.TC0.tcnt0l().write(|w| w);
            DP.TC0.ocr0a().write(|w| w.set(TC0_OCR));
            DP.TC0.tccr0b().write(|w| w.cs0().prescale_8());

            mode.set(Mode::Tx0);
            true
        }
        Mode::Tx0 | Mode::Tx1 => false, // busy
    }
}

#[allow(dead_code)]
pub fn uart_tx(data: u8) -> bool {
    interrupt::free(|cs| uart_tx_cs(cs, data))
}

// vim: ts=4 sw=4 expandtab
