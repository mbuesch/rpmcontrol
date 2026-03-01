// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use avr_context::{MainCtx, MainCtxCell};
use avr_q::{Q7p8, Q15p8, q7p8, q15p8};

pub struct Filter {
    buf: MainCtxCell<Q15p8>,
    out: MainCtxCell<Q7p8>,
}

impl Filter {
    pub const fn new() -> Self {
        Self {
            buf: MainCtxCell::new(q15p8!(const 0)),
            out: MainCtxCell::new(q7p8!(const 0)),
        }
    }

    pub fn reset(&self, m: &MainCtx<'_>) {
        self.buf.set(m, q15p8!(const 0));
        self.out.set(m, q7p8!(const 0));
    }

    pub fn run(&self, m: &MainCtx<'_>, input: Q7p8, div: Q15p8) -> Q7p8 {
        let mut buf = self.buf.get(m);
        buf -= self.out.get(m).into();
        buf += input.into();
        self.buf.set(m, buf);

        let out = (buf / div).into();
        self.out.set(m, out);

        out
    }

    pub fn get(&self, m: &MainCtx<'_>) -> Q7p8 {
        self.out.get(m)
    }
}

pub struct FilterI16 {
    buf: MainCtxCell<i32>,
    out: MainCtxCell<i16>,
}

impl FilterI16 {
    pub const fn new() -> Self {
        Self {
            buf: MainCtxCell::new(0),
            out: MainCtxCell::new(0),
        }
    }

    pub fn set(&self, m: &MainCtx<'_>, value: i16, shift: u8) {
        self.buf.set(m, i32::from(value) << shift);
        self.out.set(m, value);
    }

    pub fn run(&self, m: &MainCtx<'_>, input: i16, shift: u8) -> i16 {
        let mut buf = self.buf.get(m);
        buf -= i32::from(self.out.get(m));
        buf += i32::from(input);
        self.buf.set(m, buf);

        let out = (buf >> shift).clamp(i16::MIN.into(), i16::MAX.into()) as _;
        self.out.set(m, out);

        out
    }

    pub fn get(&self, m: &MainCtx<'_>) -> i16 {
        self.out.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
