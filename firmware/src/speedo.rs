// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use crate::{
    analog::ac_capture_get,
    debug::Debug,
    timer::{LargeTimestamp, RelLargeTimestamp, TIMER_TICK_US, timer_get_large},
};
use avr_context::{MainCtx, MainCtxCell};
use avr_int24::I24;
use avr_q::{Q7p8, q7p8};
use derive_more as dm;

/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4;

/// Need at least this many valid speedometer edges in a row to consider the speed valid.
const OK_THRES: u8 = 4;

#[derive(Copy, Clone)]
pub struct MotorSpeed(Freq);

impl MotorSpeed {
    pub const fn as_freq(&self) -> Freq {
        self.0
    }

    pub fn from_freq(value: Freq) -> Self {
        Self(value)
    }

    pub fn from_period_dur(dur: RelLargeTimestamp) -> Self {
        let dur: i16 = dur.into();
        let dur = dur.min(i16::MAX / (Freq::FACT_HZ4 * 2)); // avoid mul overflow.
        let dur = dur.max(1); // avoid div by zero.

        // fact 2 to avoid rounding error.
        let num = (1_000_000 / (TIMER_TICK_US as u32 * (SPEEDO_FACT / 2))) as i16;
        let denom = dur * Freq::FACT_HZ4 * 2;

        Self::from_freq(Freq(q7p8!(num / denom)))
    }
}

pub struct Speedo {
    ok_count: MainCtxCell<u8>,
    prev_stamp: MainCtxCell<LargeTimestamp>,
    dur: [MainCtxCell<i16>; 4],
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            ok_count: MainCtxCell::new(0),
            prev_stamp: MainCtxCell::new(LargeTimestamp::new()),
            dur: [
                MainCtxCell::new(0),
                MainCtxCell::new(0),
                MainCtxCell::new(0),
                MainCtxCell::new(0),
            ],
        }
    }

    pub fn init(&self, m: &MainCtx<'_>, now: LargeTimestamp) {
        self.prev_stamp.set(m, now);
    }

    fn get_speed(&self, m: &MainCtx<'_>) -> Option<MotorSpeed> {
        if self.ok_count.get(m) >= OK_THRES {
            Some(MotorSpeed::from_period_dur(self.get_dur(m)))
        } else {
            None
        }
    }

    fn get_dur(&self, m: &MainCtx<'_>) -> RelLargeTimestamp {
        let a = I24::from_i16(self.dur[0].get(m));
        let b = I24::from_i16(self.dur[1].get(m));
        let c = I24::from_i16(self.dur[2].get(m));
        let d = I24::from_i16(self.dur[3].get(m));
        let dur = ((a + b + c + d) >> 2).to_i16();
        dur.into()
    }

    fn new_duration(&self, m: &MainCtx<'_>, dur: RelLargeTimestamp) {
        let dur: i16 = dur.into();
        self.dur[0].set(m, self.dur[1].get(m));
        self.dur[1].set(m, self.dur[2].get(m));
        self.dur[2].set(m, self.dur[3].get(m));
        self.dur[3].set(m, dur);
        self.ok_count.set(m, self.ok_count.get(m).saturating_add(1));
    }

    pub fn run(&self, m: &MainCtx<'_>) -> Option<MotorSpeed> {
        let now = timer_get_large();
        let prev_stamp = self.prev_stamp.get(m);
        if now < prev_stamp {
            // prev_stamp wrapped. Drop it.
            self.ok_count.set(m, 0);
        }

        while let Some(ac) = ac_capture_get() {
            if ac >= prev_stamp {
                let dur = ac - prev_stamp;
                self.new_duration(m, dur);
            } else {
                // prev_stamp wrapped.
                self.ok_count.set(m, 0);
            }
            self.prev_stamp.set(m, ac);
        }

        Debug::SpeedoStatus.log_u16(self.ok_count.get(m) as u16);

        self.get_speed(m)
    }
}

/// Frequency in 4-Hz. (Hz divided by 4)
#[repr(transparent)]
#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, dm::Add, dm::AddAssign, dm::Sub, dm::SubAssign,
)]
pub struct Freq(pub Q7p8);

impl Freq {
    pub const FACT_HZ4: i16 = 4;
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
