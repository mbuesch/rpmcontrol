// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

pub struct Filter {
    buf: u32,
}

impl Filter {
    pub const fn new() -> Self {
        Self { buf: 0 }
    }

    pub fn run(&mut self, input: u16, shift: u8) -> u16 {
        let mut buf = self.buf;
        buf -= buf >> shift;
        buf += input as u32;
        self.buf = buf;
        let out = buf >> shift;
        out.clamp(u16::MIN as u32, u16::MAX as u32) as u16
    }
}

// vim: ts=4 sw=4 expandtab
