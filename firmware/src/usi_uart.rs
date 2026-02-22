// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use avr_context::{CriticalSection, InitCtx, IrqCtx};

#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn setup(c: &InitCtx) {
    #[cfg(feature = "debug")]
    inner::setup(c);
}

#[cfg_attr(not(feature = "debug"), allow(dead_code))]
#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn irq_handler_pcint(c: &IrqCtx) {
    #[cfg(feature = "debug")]
    inner::irq_handler_pcint(c);
}

#[cfg_attr(not(feature = "debug"), allow(dead_code))]
#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn irq_handler_usi_ovf(c: &IrqCtx) {
    #[cfg(feature = "debug")]
    inner::irq_handler_usi_ovf(c);
}

#[cfg_attr(not(feature = "debug"), allow(dead_code))]
#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn uart_tx_cs(cs: CriticalSection<'_>, data: u8) -> bool {
    #[cfg(feature = "debug")]
    let ret = inner::uart_tx_cs(cs, data);

    #[cfg(not(feature = "debug"))]
    let ret = false;

    ret
}

#[allow(dead_code)]
#[cfg_attr(not(feature = "debug"), allow(unused_variables))]
pub fn uart_tx(data: u8) -> bool {
    #[cfg(feature = "debug")]
    let ret = inner::uart_tx(data);

    #[cfg(not(feature = "debug"))]
    let ret = false;

    ret
}

#[cfg(feature = "debug")]
mod inner {
    use super::*;
    use crate::{
        DP_TC0, DP_USI, debug,
        hw::interrupt,
        ports::{PORTB, PortOps as _},
    };
    use avr_context::Mutex;
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
        let usi = DP_USI.as_ref_with_initctx(c);
        usi.usidr().write(|w| w.set(0xFF));
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
        let tc0 = DP_TC0.as_ref_with_irqctx(c);
        let usi = DP_USI.as_ref_with_irqctx(c);

        let mode = MODE.borrow(cs);
        match mode.get() {
            Mode::Rx => {
                let data = bit_rev(usi.usidr().read().bits());

                tc0.tccr0b().write(|w| w);

                usi.usicr().modify(|_, w| w.usioie().clear_bit());
                usi.usisr().modify(|_, w| w.usioif().set_bit());

                //TODO

                debug::rx_complete_callback(c, data);
            }
            Mode::Tx0 => {
                let data = TXDATA.borrow(cs).get();
                usi.usidr().write(|w| w.set((data << 3) | 0x07));
                usi.usisr().write(|w| {
                    w.usicnt().set(16 - 6)
                    .usioif().set_bit()
                });

                mode.set(Mode::Tx1);
            }
            Mode::Tx1 => {
                usi.usidr().write(|w| w.set(0xFF));
                usi.usicr().modify(|_, w| w.usioie().clear_bit());
                usi.usisr().modify(|_, w| w.usioif().set_bit());

                DP_TC0.as_ref_with_irqctx(c).tccr0b().write(|w| w);

                PORTB.set(c.cs(), PORTB_BIT, true);
                PORTB.input(c.cs(), PORTB_BIT);

                //TODO enable PCINT

                mode.set(Mode::Rx);
                debug::tx_complete_callback(c);
            }
        }
    }

    #[rustfmt::skip]
    pub fn uart_tx_cs(cs: CriticalSection<'_>, mut data: u8) -> bool {
        let tc0 = DP_TC0.as_ref_with_cs(cs);
        let usi = DP_USI.as_ref_with_cs(cs);

        let mode = MODE.borrow(cs);
        match mode.get() {
            Mode::Rx => {
                data = bit_rev(data);
                TXDATA.borrow(cs).set(data);

                tc0.tccr0b().write(|w| w);

                PORTB.set(cs, PORTB_BIT, true);
                PORTB.output(cs, PORTB_BIT);

                usi.usidr().write(|w| w.set((data >> 2) | 0x80));
                usi.usisr().write(|w| {
                    w.usicnt().set(16 - 5)
                    .usioif().set_bit()
                });
                usi.usicr().write(|w| {
                    w.usioie().set_bit()
                    .usiwm().three_wire()
                    .usics().tc0()
                });
                usi.usipp().write(|w| w);

                tc0.tccr0a().write(|w| w.ctc0().set_bit());
                tc0.tcnt0h().write(|w| w);
                tc0.tcnt0l().write(|w| w);
                tc0.ocr0a().write(|w| w.set(TC0_OCR));
                tc0.tccr0b().write(|w| w.cs0().prescale_8());

                mode.set(Mode::Tx0);
                true
            }
            Mode::Tx0 | Mode::Tx1 => false, // busy
        }
    }

    pub fn uart_tx(data: u8) -> bool {
        interrupt::free(|cs| uart_tx_cs(cs, data))
    }
}

// vim: ts=4 sw=4 expandtab
