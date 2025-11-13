// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use crate::{
    DP_TC0, DP_USI, debug,
    hw::interrupt,
    ports::{PORTB, PortOps as _},
};
use avr_context::{CriticalSection, InitCtx, IrqCtx, Mutex};
use core::cell::Cell;

const FCPU: u32 = 16_000_000;
const BAUD: u32 = 19_200;
const PORTB_BIT: usize = 1;
const TC0_PS: u32 = 8;
const TC0_OCR: u8 = (FCPU / (BAUD * TC0_PS)) as u8;

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

pub fn setup(c: &InitCtx) {
    DP_USI.initctx(c).usidr().write(|w| w.set(0xFF));
    //TODO enable PCINT
}

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
            let data = bit_rev(DP_USI.irqctx(c).usidr().read().bits());

            DP_TC0.irqctx(c).tccr0b().write(|w| w);

            DP_USI.irqctx(c).usicr().modify(|_, w| w.usioie().clear_bit());
            DP_USI.irqctx(c).usisr().modify(|_, w| w.usioif().set_bit());

            //TODO

            debug::rx_complete_callback(c, data);
        }
        Mode::Tx0 => {
            let data = TXDATA.borrow(cs).get();
            DP_USI.irqctx(c).usidr().write(|w| w.set((data << 3) | 0x07));
            DP_USI.irqctx(c).usisr().write(|w| {
                w.usicnt().set(16 - 6)
                 .usioif().set_bit()
            });

            mode.set(Mode::Tx1);
        }
        Mode::Tx1 => {
            DP_USI.irqctx(c).usidr().write(|w| w.set(0xFF));
            DP_USI.irqctx(c).usicr().modify(|_, w| w.usioie().clear_bit());
            DP_USI.irqctx(c).usisr().modify(|_, w| w.usioif().set_bit());

            DP_TC0.irqctx(c).tccr0b().write(|w| w);

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

            DP_TC0.cs(cs).tccr0b().write(|w| w);

            PORTB.set(PORTB_BIT, true);
            PORTB.output(PORTB_BIT);

            DP_USI.cs(cs).usidr().write(|w| w.set((data >> 2) | 0x80));
            DP_USI.cs(cs).usisr().write(|w| {
                w.usicnt().set(16 - 5)
                 .usioif().set_bit()
            });
            DP_USI.cs(cs).usicr().write(|w| {
                w.usioie().set_bit()
                 .usiwm().three_wire()
                 .usics().tc0()
            });
            DP_USI.cs(cs).usipp().write(|w| w);

            DP_TC0.cs(cs).tccr0a().write(|w| w.ctc0().set_bit());
            DP_TC0.cs(cs).tcnt0h().write(|w| w);
            DP_TC0.cs(cs).tcnt0l().write(|w| w);
            DP_TC0.cs(cs).ocr0a().write(|w| w.set(TC0_OCR));
            DP_TC0.cs(cs).tccr0b().write(|w| w.cs0().prescale_8());

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
