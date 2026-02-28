// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use avr_q::{Q7p8, q7p8};
use derive_more as dm;

/// Frequency with internal representation in Q7.8 format.
/// The actual frequency in Hz is `freq.0 * Freq::FACT`.
#[repr(transparent)]
#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign,
)]
pub struct Freq(pub Q7p8);

const FREQ_FACT_NUM: i16 = Freq::FACT_NUM;
const FREQ_FACT_DEN: i16 = Freq::FACT_DEN;

impl Freq {
    pub const FACT_NUM: i16 = 10;
    pub const FACT_DEN: i16 = 3;
    pub const FACT: Q7p8 = q7p8!(const FREQ_FACT_NUM / FREQ_FACT_DEN);
}

// Project to inner.
impl curveipo::CurvePoint<Freq> for (Freq, Freq) {
    #[inline(always)]
    fn x(&self) -> Freq {
        self.0
    }

    #[inline(always)]
    fn y(&self) -> Freq {
        self.1
    }
}

// Project to inner.
impl curveipo::CurveIpo for Freq {
    #[inline(always)]
    fn lin_inter(
        &self,
        left: &impl curveipo::CurvePoint<Self>,
        right: &impl curveipo::CurvePoint<Self>,
    ) -> Self {
        let left = (left.x().0, left.y().0);
        let right = (right.x().0, right.y().0);
        Self(self.0.lin_inter(&left, &right))
    }
}

// vim: ts=4 sw=4 expandtab
