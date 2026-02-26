// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use avr_context::{MainCtx, MainCtxCell};
use core::ops::{Add, Sub};

pub struct Snap<T: Copy> {
    snapped: MainCtxCell<T>,
}

impl<T: Copy> Snap<T> {
    pub const fn new(snapped: T) -> Self {
        Self {
            snapped: MainCtxCell::new(snapped),
        }
    }
}

impl<T: Copy + PartialOrd + Add<Output = T> + Sub<Output = T>> Snap<T> {
    #[allow(clippy::if_same_then_else)]
    pub fn update(&self, m: &MainCtx<'_>, min: T, max: T, hyst: T, new: T) -> T {
        let mut snapped = self.snapped.get(m);

        if new >= max - hyst {
            snapped = max;
        } else if new <= min + hyst {
            snapped = min;
        } else if new > snapped && new - snapped > hyst {
            snapped = new;
        } else if new < snapped && snapped - new > hyst {
            snapped = new;
        }

        self.snapped.set(m, snapped);
        snapped
    }
}

// vim: ts=4 sw=4 expandtab
