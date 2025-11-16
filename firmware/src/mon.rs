// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use crate::{
    debounce::Debounce,
    debug::Debug,
    history::History,
    shutoff::Shutoff,
    system::{MOT_HARD_LIMIT, rpm},
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large},
};
use avr_atomic::AvrAtomic;
use avr_context::{MainCtx, MainCtxCell};
use avr_q::{Q7p8, q7p8};
use avr_stack::estimate_unused_stack_space;

/// Distance between monitoring checks.
const CHECK_DIST: RelLargeTimestamp = RelLargeTimestamp::from_millis(20);
/// Immediate fault, if one actual monitoring distance is bigger than this.
const CHECK_TIMEOUT: RelLargeTimestamp = RelLargeTimestamp::from_millis(100);

/// Immediate fault, if mains zero crossing distance is bigger than this.
const MAINS_ZERO_CROSSING_TIMEOUT: RelLargeTimestamp = RelLargeTimestamp::from_millis(100);

/// Minimum amount of CPU stack space that must be free all the time.
/// Immediate fault, if less stack space is free.
const MIN_STACK_SPACE: u16 = 64;

/// Setpoint history.
/// Length = SP_HIST_DIST * SP_HIST_COUNT = 3 seconds
const SP_HIST_DIST: RelLargeTimestamp = RelLargeTimestamp::from_micros(333333);
/// Number if elements in the setpoint history.
const SP_HIST_COUNT: usize = 9;

/// Don't run monitoring, if the setpoint gradient in history is bigger than this.
const SP_GRADIENT_THRES: Q7p8 = rpm!(1000);

/// Step size for one error event.
const ERROR_DEBOUNCE_ERRSTEP: u8 = 3;
/// Debounce limit to enter fault state.
const ERROR_DEBOUNCE_LIMIT: u8 = 120;
/// Sticky -> fault state cannot be healed.
const ERROR_DEBOUNCE_STICKY: bool = true;

/// Setpoint vs. speedometer deviation threshold that is considered to be an unexpected mismatch.
const SPEEDO_TOLERANCE: Q7p8 = rpm!(1000);
/// Monitoring activation threshold for speedometer input.
/// Monitoring is not active below this threshold.
const MON_ACTIVE_THRES: Q7p8 = rpm!(7500);

static ANALOG_FAILURE: AvrAtomic<bool> = AvrAtomic::new();

pub struct Mon {
    prev_check: MainCtxCell<LargeTimestamp>,
    prev_mains_90deg: MainCtxCell<LargeTimestamp>,
    prev_sp: MainCtxCell<LargeTimestamp>,
    error_deb: Debounce<ERROR_DEBOUNCE_ERRSTEP, ERROR_DEBOUNCE_LIMIT, ERROR_DEBOUNCE_STICKY>,
    sp_hist: History<Q7p8, SP_HIST_COUNT>,
}

impl Mon {
    pub const fn new() -> Self {
        Self {
            prev_check: MainCtxCell::new(LargeTimestamp::new()),
            prev_mains_90deg: MainCtxCell::new(LargeTimestamp::new()),
            prev_sp: MainCtxCell::new(LargeTimestamp::new()),
            error_deb: Debounce::new(),
            sp_hist: History::new([
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
                MainCtxCell::new(q7p8!(const 0)),
            ]),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>, now: LargeTimestamp) {
        self.prev_check.set(m, now);
        self.prev_mains_90deg.set(m, now);
        self.prev_sp.set(m, now);
    }

    pub fn check(
        &self,
        m: &MainCtx<'_>,
        setpoint: Q7p8,
        speedo_hz: Q7p8,
        mains_90deg: bool,
    ) -> Shutoff {
        let now = timer_get_large();

        // If we just had a mains 90deg crossing, remember the time stamp.
        if mains_90deg {
            self.prev_mains_90deg.set(m, now);
        }
        // Check if the distance between mains 90deg crossings is too big.
        let mains_90deg_dist_failure =
            now > self.prev_mains_90deg.get(m) + MAINS_ZERO_CROSSING_TIMEOUT;

        // Put the next setpoint sample into the history buffer.
        let next_sp = self.prev_sp.get(m) + SP_HIST_DIST;
        if now >= next_sp {
            self.prev_sp.set(m, next_sp);
            self.sp_hist.push_back(m, setpoint);
        }

        // Check if the distance between monitoring checks is too big.
        let prev_check = self.prev_check.get(m);
        let mon_check_dist_failure = now > prev_check + CHECK_TIMEOUT;

        // Check if we need to do the monitoring checks now.
        let next_check = prev_check + CHECK_DIST;
        if now >= next_check {
            self.prev_check.set(m, next_check);

            // If the motor speed is above the hard limit, then we have a problem.
            if speedo_hz >= MOT_HARD_LIMIT {
                self.error_deb.error(m);
            } else {
                // The motor speed is inside of the allowed range.

                // Get the setpoint gradient between
                // current setpoint and oldest setpoint from history buffer.
                let sp_grad = (setpoint - self.sp_hist.oldest(m)).abs();

                // Only do the monitoring checks,
                // if the setpoint didn't change much recently.
                if sp_grad <= SP_GRADIENT_THRES {
                    // Check if we are above the monitoring activation RPM threshold.
                    if speedo_hz >= MON_ACTIVE_THRES {
                        // Get the difference between measured speed and speed setpoint.
                        let diff = (speedo_hz - setpoint).abs();

                        // If the speed difference is above a threshold,
                        // we might have an error.
                        // Debounce the error.
                        if diff > SPEEDO_TOLERANCE {
                            self.error_deb.error(m);
                        } else {
                            self.error_deb.ok(m);
                        }
                    } else {
                        // We are below the monitoring activation threshold.
                        // The machine is running with slow speed.
                        // Assume everything is fine.
                        self.error_deb.ok(m);
                    }
                } else {
                    // We just wait until the user stopped changing the setpoint.
                }
            }
        }

        // Check if stack usage was too large.
        let unused_stack_bytes = estimate_unused_stack_space();
        let stack_failure = unused_stack_bytes < MIN_STACK_SPACE;

        // Analog value processing failed.
        let analog_failure = ANALOG_FAILURE.load();

        // Raise an immediate error without debouncing on certain hard failures.
        if stack_failure || mon_check_dist_failure || analog_failure || mains_90deg_dist_failure {
            self.error_deb.error_no_debounce(m);
        }

        Debug::MinStack.log_u16(unused_stack_bytes);
        Debug::MonDebounce.log_u8(self.error_deb.count(m));

        if self.error_deb.is_ok(m) {
            Shutoff::MachineRunning
        } else {
            Shutoff::MachineShutoff
        }
    }
}

pub fn mon_report_analog_failure() {
    ANALOG_FAILURE.store(true);
}

// vim: ts=4 sw=4 expandtab
