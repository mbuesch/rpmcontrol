// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use avr_context::{MainCtx, MainCtxCell};
use avr_q::{Q7p8, Q15p8, q7p8, q15p8};

/// A simple IIR filter for Q7.8 fixed-point values.
pub struct Filter {
    buf: MainCtxCell<Q15p8>,
    out: MainCtxCell<Q7p8>,
}

impl Filter {
    /// Create a new filter with zero initial state.
    pub const fn new() -> Self {
        Self {
            buf: MainCtxCell::new(q15p8!(const 0)),
            out: MainCtxCell::new(q7p8!(const 0)),
        }
    }

    /// Reset the filter state to zero.
    pub fn reset(&self, m: &MainCtx<'_>) {
        self.buf.set(m, q15p8!(const 0));
        self.out.set(m, q7p8!(const 0));
    }

    /// Run the filter with the given input and divisor.
    ///
    /// The divisor must be the same as the one used for the previous call,
    /// otherwise the filter will not work correctly.
    ///
    /// Returns the new output value of the filter.
    pub fn run(&self, m: &MainCtx<'_>, input: Q7p8, div: Q15p8) -> Q7p8 {
        let mut buf = self.buf.get(m);
        buf -= self.out.get(m).into();
        buf += input.into();
        self.buf.set(m, buf);

        let out = (buf / div).into();
        self.out.set(m, out);

        out
    }

    /// Get the current output value of the filter.
    pub fn get(&self, m: &MainCtx<'_>) -> Q7p8 {
        self.out.get(m)
    }
}

/// A simple IIR filter for i16 values.
pub struct FilterI16 {
    buf: MainCtxCell<i32>,
    out: MainCtxCell<i16>,
}

impl FilterI16 {
    /// Create a new filter with zero initial state.
    pub const fn new() -> Self {
        Self {
            buf: MainCtxCell::new(0),
            out: MainCtxCell::new(0),
        }
    }

    /// Reset the filter state to zero.
    #[allow(dead_code)]
    pub fn reset(&self, m: &MainCtx<'_>) {
        self.buf.set(m, 0);
        self.out.set(m, 0);
    }

    /// Set the filter state.
    pub fn set(&self, m: &MainCtx<'_>, value: i16, shift: u8) {
        self.buf.set(m, i32::from(value) << shift);
        self.out.set(m, value);
    }

    /// Run the filter with the given input and shift (divisor).
    ///
    /// The shift must be the same as the one used for the previous call,
    /// otherwise the filter will not work correctly.
    ///
    /// Returns the new output value of the filter.
    pub fn run(&self, m: &MainCtx<'_>, input: i16, shift: u8) -> i16 {
        let mut buf = self.buf.get(m);
        buf -= i32::from(self.out.get(m));
        buf += i32::from(input);
        self.buf.set(m, buf);

        let out = (buf >> shift).clamp(i16::MIN.into(), i16::MAX.into()) as _;
        self.out.set(m, out);

        out
    }

    /// Get the current output value of the filter.
    pub fn get(&self, m: &MainCtx<'_>) -> i16 {
        self.out.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
