// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

#![allow(unused_unsafe)]

use crate::{hw::mcu, system::SysPeriph};
use avr_context::{InitCtx, InitCtxCell};

pub trait PortOps {
    fn get(&self, bit: usize) -> bool;
    fn set(&self, bit: usize, value: bool);
    fn toggle(&self, bit: usize);
    fn output(&self, bit: usize);
    fn input(&self, bit: usize);
}

#[rustfmt::skip]
macro_rules! impl_port {
    (
        $struct:ident,
        $name:ident,
        $port:ident,
        $pin:ident,
        $ddr:ident,
        $bit0:ident,
        $bit1:ident,
        $bit2:ident,
        $bit3:ident,
        $bit4:ident,
        $bit5:ident,
        $bit6:ident,
        $bit7:ident
    ) => {
        #[allow(non_snake_case)]
        pub struct $struct {
            pub $name: mcu::$name,
        }

        // SAFETY: Is initialized when constructing the MainCtx.
        pub static $name: InitCtxCell<$struct> = unsafe { InitCtxCell::uninit() };

        impl PortOps for InitCtxCell<$struct> {
            #[inline(always)]
            #[allow(dead_code)]
            fn get(&self, bit: usize) -> bool {
                match bit {
                    0 => $name.$name.$pin().read().$bit0().bit(),
                    1 => $name.$name.$pin().read().$bit1().bit(),
                    2 => $name.$name.$pin().read().$bit2().bit(),
                    3 => $name.$name.$pin().read().$bit3().bit(),
                    4 => $name.$name.$pin().read().$bit4().bit(),
                    5 => $name.$name.$pin().read().$bit5().bit(),
                    6 => $name.$name.$pin().read().$bit6().bit(),
                    7 => $name.$name.$pin().read().$bit7().bit(),
                    _ => unreachable!(),
                }
            }

            #[inline(always)]
            #[allow(dead_code)]
            fn set(&self, bit: usize, value: bool) {
                match bit {
                    0 => $name.$name.$port().modify(|_, w| w.$bit0().bit(value)),
                    1 => $name.$name.$port().modify(|_, w| w.$bit1().bit(value)),
                    2 => $name.$name.$port().modify(|_, w| w.$bit2().bit(value)),
                    3 => $name.$name.$port().modify(|_, w| w.$bit3().bit(value)),
                    4 => $name.$name.$port().modify(|_, w| w.$bit4().bit(value)),
                    5 => $name.$name.$port().modify(|_, w| w.$bit5().bit(value)),
                    6 => $name.$name.$port().modify(|_, w| w.$bit6().bit(value)),
                    7 => $name.$name.$port().modify(|_, w| w.$bit7().bit(value)),
                    _ => unreachable!(),
                };
            }

            #[inline(always)]
            #[allow(dead_code)]
            fn toggle(&self, bit: usize) {
                match bit {
                    0 => $name.$name.$pin().modify(|_, w| w.$bit0().set_bit()),
                    1 => $name.$name.$pin().modify(|_, w| w.$bit1().set_bit()),
                    2 => $name.$name.$pin().modify(|_, w| w.$bit2().set_bit()),
                    3 => $name.$name.$pin().modify(|_, w| w.$bit3().set_bit()),
                    4 => $name.$name.$pin().modify(|_, w| w.$bit4().set_bit()),
                    5 => $name.$name.$pin().modify(|_, w| w.$bit5().set_bit()),
                    6 => $name.$name.$pin().modify(|_, w| w.$bit6().set_bit()),
                    7 => $name.$name.$pin().modify(|_, w| w.$bit7().set_bit()),
                    _ => unreachable!(),
                };
            }

            #[inline(always)]
            #[allow(dead_code)]
            fn output(&self, bit: usize) {
                match bit {
                    0 => $name.$name.$ddr().modify(|_, w| w.$bit0().set_bit()),
                    1 => $name.$name.$ddr().modify(|_, w| w.$bit1().set_bit()),
                    2 => $name.$name.$ddr().modify(|_, w| w.$bit2().set_bit()),
                    3 => $name.$name.$ddr().modify(|_, w| w.$bit3().set_bit()),
                    4 => $name.$name.$ddr().modify(|_, w| w.$bit4().set_bit()),
                    5 => $name.$name.$ddr().modify(|_, w| w.$bit5().set_bit()),
                    6 => $name.$name.$ddr().modify(|_, w| w.$bit6().set_bit()),
                    7 => $name.$name.$ddr().modify(|_, w| w.$bit7().set_bit()),
                    _ => unreachable!(),
                };
            }

            #[inline(always)]
            #[allow(dead_code)]
            fn input(&self, bit: usize) {
                match bit {
                    0 => $name.$name.$ddr().modify(|_, w| w.$bit0().clear_bit()),
                    1 => $name.$name.$ddr().modify(|_, w| w.$bit1().clear_bit()),
                    2 => $name.$name.$ddr().modify(|_, w| w.$bit2().clear_bit()),
                    3 => $name.$name.$ddr().modify(|_, w| w.$bit3().clear_bit()),
                    4 => $name.$name.$ddr().modify(|_, w| w.$bit4().clear_bit()),
                    5 => $name.$name.$ddr().modify(|_, w| w.$bit5().clear_bit()),
                    6 => $name.$name.$ddr().modify(|_, w| w.$bit6().clear_bit()),
                    7 => $name.$name.$ddr().modify(|_, w| w.$bit7().clear_bit()),
                    _ => unreachable!(),
                };
            }
        }
    };
}

impl_port!(
    PortA, PORTA, porta, pina, ddra, pa0, pa1, pa2, pa3, pa4, pa5, pa6, pa7
);
impl_port!(
    PortB, PORTB, portb, pinb, ddrb, pb0, pb1, pb2, pb3, pb4, pb5, pb6, pb7
);

fn pin_input(_bit: usize) -> u8 {
    0
}
fn pin_output(bit: usize) -> u8 {
    1 << bit
}
fn pin_low(_bit: usize) -> u8 {
    0
}
fn pin_high(bit: usize) -> u8 {
    1 << bit
}
fn pin_floating(_bit: usize) -> u8 {
    0
}
fn pin_pullup(bit: usize) -> u8 {
    1 << bit
}

impl PortA {
    pub fn setup(&self, _: &InitCtx) {
        // SAFETY: Called with interrupts disabled. Ensured by &InitCtx.
        unsafe {
            self.PORTA.porta().write(|w| {
                w.bits(
                    pin_floating(0) | // setpoint, single ended ADC
                    pin_floating(1) | // vsense
                    pin_low(2) | // DNC
                    pin_floating(3) | // AREF
                    pin_low(4) | // n_shutoff
                    pin_floating(5) | // motor temperature, single ended ADC
                    pin_floating(6) | // speedo, AD comparator pos
                    pin_floating(7), // speedo, AD comparator neg
                )
            });
            self.PORTA.ddra().write(|w| {
                w.bits(
                    pin_input(0) | // setpoint, single ended ADC
                    pin_input(1) | // vsense
                    pin_output(2) | // DNC
                    pin_input(3) | // AREF
                    pin_output(4) | // n_shutoff
                    pin_input(5) | // motor temperature, single ended ADC
                    pin_input(6) | // speedo, AD comparator pos
                    pin_input(7), // speedo, AD comparator neg
                )
            });
        }
    }
}

impl PortB {
    pub fn setup(&self, _: &InitCtx) {
        // SAFETY: Called with interrupts disabled. Ensured by &InitCtx.
        unsafe {
            self.PORTB.portb().write(|w| {
                w.bits(
                    pin_pullup(0) | // ISP MOSI + UART DI
                    pin_high(1) | // ISP MISO + UART DO
                    pin_low(2) | // ISP SCK
                    pin_high(3) | // trig, active low
                    pin_floating(4) | // XTAL1
                    pin_floating(5) | // XTAL2
                    pin_low(6) | // Debug
                    pin_floating(7), // RESET, Debug-Wire
                )
            });
            self.PORTB.ddrb().write(|w| {
                w.bits(
                    pin_input(0) | // ISP MOSI + UART DI
                    pin_output(1) | // ISP MISO + UART DO
                    pin_output(2) | // ISP SCK
                    pin_output(3) | // trig, active low
                    pin_input(4) | // XTAL1
                    pin_input(5) | // XTAL2
                    pin_output(6) | // Debug
                    pin_input(7), // RESET, Debug-Wire
                )
            });
        }
    }
}

#[rustfmt::skip]
pub fn setup_didr(sp: &SysPeriph) {
    sp.ADC.didr0().write(|w| {
        w.adc0d().set_bit() // PA0: setpoint ADC
         .adc1d().clear_bit()
         .adc2d().clear_bit()
         .arefd().clear_bit()
         .adc3d().clear_bit()
         .adc4d().clear_bit()
         .adc5d().set_bit() // PA6: speedo positive
         .adc6d().set_bit() // PA7: speedo positive
    });
    sp.ADC.didr1().write(|w| {
        w.adc7d().clear_bit()
         .adc8d().clear_bit()
         .adc9d().clear_bit()
         .adc10d().clear_bit()
    });
}

// vim: ts=4 sw=4 expandtab
