// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use crate::{
    analog::ac_capture_get,
    debug::Debug,
    filter::FilterI16,
    freq::Freq,
    timer::{LargeTimestamp, RelLargeTimestamp, TIMER_TICK_US, timer_get_large},
};
use avr_context::{MainCtx, MainCtxCell};
use avr_q::q15p8;

/// 4 speedometer edges per motor revolution
const SPEEDO_FACT: u32 = 4;

/// Need at least this many valid speedometer edges in a row to consider the speed valid.
const OK_THRES: u8 = 5;

/// If no speedometer edge is detected for this long, consider the speed invalid.
const TIMEOUT: RelLargeTimestamp = RelLargeTimestamp::from_millis(50);

/// Speedometer filter shift.
const FILTER_SHIFT: u8 = 4;

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
        const DUR_LIM: i16 =
            ((i16::MAX as i32 * Freq::FACT_DEN as i32) / (Freq::FACT_NUM as i32)) as i16;

        let dur: i16 = dur.into();
        let dur = dur.min(DUR_LIM);
        let dur = dur.max(1); // avoid div by zero.

        let num = (1_000_000 / (TIMER_TICK_US as u32 * SPEEDO_FACT)) as i16;
        let denom = dur;

        let freq = q15p8!(num / denom) / Freq::FACT.to_q15p8();

        Self::from_freq(Freq(freq.to_q7p8()))
    }
}

pub struct Speedo {
    ok_count: MainCtxCell<u8>,
    prev_stamp: MainCtxCell<LargeTimestamp>,
    dur_filter: FilterI16,
}

impl Speedo {
    pub const fn new() -> Self {
        Self {
            ok_count: MainCtxCell::new(0),
            prev_stamp: MainCtxCell::new(LargeTimestamp::new()),
            dur_filter: FilterI16::new(),
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
        self.dur_filter.get(m).into()
    }

    fn new_duration(&self, m: &MainCtx<'_>, dur: RelLargeTimestamp) {
        // First real duration?
        if self.ok_count.get(m) <= 1 {
            // Just store.
            self.dur_filter.set(m, dur.into(), FILTER_SHIFT);
        } else {
            // Filter duration.
            self.dur_filter.run(m, dur.into(), FILTER_SHIFT);
        }
        self.inc_ok(m);
    }

    fn inc_ok(&self, m: &MainCtx<'_>) {
        self.ok_count.set(m, self.ok_count.get(m).saturating_add(1));
    }

    pub fn run(&self, m: &MainCtx<'_>) -> Option<MotorSpeed> {
        let mut prev_stamp = self.prev_stamp.get(m);

        // Process all new AC captures.
        while let Some(ac) = ac_capture_get() {
            // prev_stamp is invalid?
            if self.ok_count.get(m) == 0 {
                // first edge, just store prev_stamp and increment ok_count.
                self.inc_ok(m);
            } else {
                // ac stamp is valid?
                if ac >= prev_stamp {
                    let dur = ac - prev_stamp;
                    self.new_duration(m, dur);
                } else {
                    // invalid stamp.
                    self.ok_count.set(m, 0);
                }
            }
            prev_stamp = ac;
        }

        // Check if prev_stamp is too old.
        let now = timer_get_large();
        if now - prev_stamp >= TIMEOUT {
            self.ok_count.set(m, 0);
        }

        self.prev_stamp.set(m, prev_stamp);

        Debug::SpeedoStatus.log_u16(self.ok_count.get(m) as u16);

        self.get_speed(m)
    }
}

// vim: ts=4 sw=4 expandtab
