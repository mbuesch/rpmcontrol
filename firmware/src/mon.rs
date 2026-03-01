// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use crate::{
    calibration::{
        mon::{
            ACCELERATION_GRADIENT_LO_THRES, CHECK_DIST, CHECK_TIMEOUT, ERROR_DEBOUNCE_ERRSTEP,
            ERROR_DEBOUNCE_LIMIT, ERROR_DEBOUNCE_STICKY, HIST_COUNT, HIST_DIST,
            MAINS_ZERO_CROSSING_TIMEOUT, MAX_MAIN_RT_LIMIT, MIN_STACK_SPACE, MON_ACTIVE_THRES,
            SP_GRADIENT_THRES, SPEEDO_TOLERANCE,
        },
        system::MOT_HARD_LIMIT,
    },
    debounce::Debounce,
    debug::Debug,
    freq::Freq,
    history::History,
    shutoff::Shutoff,
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large},
};
use avr_atomic::AvrAtomic;
use avr_context::{MainCtx, MainCtxCell};
use avr_q::q7p8;
use avr_stack::estimate_unused_stack_space;

static ANALOG_FAILURE: AvrAtomic<bool> = AvrAtomic::new();

/// RPM controller state for monitoring.
#[derive(Copy, Clone)]
struct MonControllerState {
    setpoint: Freq,
    speedo: Freq,
}

// Hard monitoring failures.
#[derive(Clone, Default)]
struct MonHardFailures {
    mains_90deg_dist_failure: bool,
    mon_check_dist_failure: bool,
    stack_failure: bool,
    max_main_rt_failure: bool,
    analog_failure: bool,
}

/// Monitoring system.
pub struct Mon {
    prev_check: MainCtxCell<LargeTimestamp>,
    prev_mains_90deg: MainCtxCell<LargeTimestamp>,
    prev_hist: MainCtxCell<LargeTimestamp>,
    error_deb: Debounce<ERROR_DEBOUNCE_ERRSTEP, ERROR_DEBOUNCE_LIMIT, ERROR_DEBOUNCE_STICKY>,
    hist: History<MonControllerState, HIST_COUNT>,
    prev_main_rt_stamp: MainCtxCell<LargeTimestamp>,
    max_main_rt: MainCtxCell<RelLargeTimestamp>,
}

impl Mon {
    /// Create a new monitoring system.
    pub const fn new() -> Self {
        Self {
            prev_check: MainCtxCell::new(LargeTimestamp::new()),
            prev_mains_90deg: MainCtxCell::new(LargeTimestamp::new()),
            prev_hist: MainCtxCell::new(LargeTimestamp::new()),
            error_deb: Debounce::new(),
            hist: History::new(MainCtxCell::new_array(MonControllerState {
                setpoint: Freq(q7p8!(const 0)),
                speedo: Freq(q7p8!(const 0)),
            })),
            prev_main_rt_stamp: MainCtxCell::new(LargeTimestamp::new()),
            max_main_rt: MainCtxCell::new(RelLargeTimestamp::new()),
        }
    }

    /// Initialize the monitoring system.
    pub fn init(&self, m: &MainCtx<'_>, now: LargeTimestamp) {
        self.prev_check.set(m, now);
        self.prev_mains_90deg.set(m, now);
        self.prev_hist.set(m, now);
    }

    /// Check distance between monitoring checks and return whether main checks should be performed.
    fn mon_distance_check(
        &self,
        m: &MainCtx<'_>,
        now: LargeTimestamp,
        hard_failures: &mut MonHardFailures,
    ) -> bool {
        let prev_check = self.prev_check.get(m);

        // Check if the distance between monitoring checks is too big.
        hard_failures.mon_check_dist_failure = now > prev_check + CHECK_TIMEOUT;

        // Check if we need to do the main monitoring checks now.
        let next_main_check = prev_check + CHECK_DIST;
        let main_checks_now = now >= next_main_check;
        if main_checks_now {
            self.prev_check.set(m, next_main_check);
        }

        main_checks_now
    }

    /// Update the monitoring history buffer with the current setpoint and speedometer values.
    fn mon_update_history(&self, m: &MainCtx<'_>, now: LargeTimestamp, entry: &MonControllerState) {
        // Put the next setpoint sample into the history buffer.
        let next_hist = self.prev_hist.get(m) + HIST_DIST;
        if now >= next_hist {
            self.prev_hist.set(m, next_hist);
            self.hist.push_back(m, *entry);
        }
    }

    /// Check the distance between mains zero crossings.
    fn mon_mains_90deg(
        &self,
        m: &MainCtx<'_>,
        now: LargeTimestamp,
        mains_90deg: bool,
        hard_failures: &mut MonHardFailures,
    ) {
        // If we just had a mains 90deg crossing, remember the time stamp.
        if mains_90deg {
            self.prev_mains_90deg.set(m, now);
        }

        // Check if the distance between mains 90deg crossings is too big.
        hard_failures.mains_90deg_dist_failure =
            now > self.prev_mains_90deg.get(m) + MAINS_ZERO_CROSSING_TIMEOUT;
    }

    /// Check the CPU stack usage.
    fn mon_check_stack_usage(&self, _m: &MainCtx<'_>, hard_failures: &mut MonHardFailures) {
        let unused_stack_bytes = estimate_unused_stack_space();

        // Check if stack usage was too large.
        hard_failures.stack_failure = unused_stack_bytes < MIN_STACK_SPACE;

        Debug::MinStack.log_u16(unused_stack_bytes);
    }

    /// Check the main loop execution time.
    fn mon_check_main_runtime(&self, m: &MainCtx<'_>, hard_failures: &mut MonHardFailures) {
        // Check if the main loop execution time was too long.
        hard_failures.max_main_rt_failure = self.max_main_rt.get(m) > MAX_MAIN_RT_LIMIT;
    }

    /// Check for analog failures.
    fn mon_check_analog_failure(&self, _m: &MainCtx<'_>, hard_failures: &mut MonHardFailures) {
        // Analog value processing failed.
        hard_failures.analog_failure = ANALOG_FAILURE.load();
    }

    /// Do the main periodic monitoring checks.
    fn mon_main_checks(&self, m: &MainCtx<'_>, ctrl_state: &MonControllerState) {
        // If the motor speed is above the hard limit, then we have a major problem.
        if ctrl_state.speedo >= MOT_HARD_LIMIT && cfg!(feature = "monitoring") {
            // We already know that we have an error.
            // Do not run the remaining checks.
            self.error_deb.error(m);
            return;
        }
        // The motor speed is not above the hard limit or the hard limit check is disabled.

        // Get the oldest entry from the history buffer.
        let oldest_hist_entry = self.hist.oldest(m);

        // Get the setpoint gradient between
        // the oldest setpoint from history buffer and the current setpoint.
        let sp_grad = ctrl_state.setpoint - oldest_hist_entry.setpoint;

        // Get the speedometer gradient between
        // the oldest speedometer value from history buffer and the current speedometer value.
        let speedo_grad = ctrl_state.speedo - oldest_hist_entry.speedo;

        // Only do the monitoring checks,
        // if the setpoint didn't change much recently.
        if sp_grad.abs() > SP_GRADIENT_THRES {
            // We just wait until the user stopped changing the setpoint.
            // Do not run the monitoring checks and do *not* debounce to Ok here,
            // because we don't know the state of the system until the setpoint has settled.
            return;
        }

        if speedo_grad < ACCELERATION_GRADIENT_LO_THRES {
            // The machine is decelerating.
            // We cannot actively control the deceleration, because the machine does not have actively controlled braking.
            // Therefore, don't run the monitoring checks during deceleration.
            return;
        }

        // Check if we are above the monitoring activation RPM threshold.
        if ctrl_state.speedo >= MON_ACTIVE_THRES {
            // Get the absolute difference between measured speed and speed setpoint.
            let diff = (ctrl_state.speedo - ctrl_state.setpoint).abs();

            // If the speed difference is above a threshold,
            // we might have an error.
            // Debounce the error.
            if diff > SPEEDO_TOLERANCE {
                if cfg!(feature = "monitoring") {
                    self.error_deb.error(m);
                }
            } else {
                self.error_deb.ok(m);
            }
        } else {
            // We are below the monitoring activation threshold.
            // The machine is running with slow speed.
            // Assume everything is fine.
            self.error_deb.ok(m);
        }
    }

    /// Do the monitoring checks and return the shutoff state.
    pub fn check(
        &self,
        m: &MainCtx<'_>,
        setpoint: Freq,
        speedo: Freq,
        mains_90deg: bool,
    ) -> Shutoff {
        let mut hard_failures = MonHardFailures::default();
        let ctrl_state = MonControllerState { setpoint, speedo };
        let now = timer_get_large();

        // Update monitoring history.
        self.mon_update_history(m, now, &ctrl_state);

        // Monitoring distance.
        let main_checks_now = self.mon_distance_check(m, now, &mut hard_failures);

        // Check if we need to do the main monitoring checks now.
        if main_checks_now {
            self.mon_main_checks(m, &ctrl_state);
        }

        // Run the remaining hard failure checks.
        self.mon_mains_90deg(m, now, mains_90deg, &mut hard_failures);
        self.mon_check_stack_usage(m, &mut hard_failures);
        self.mon_check_main_runtime(m, &mut hard_failures);
        self.mon_check_analog_failure(m, &mut hard_failures);

        // Do we have any hard failure?
        if hard_failures.stack_failure
            || hard_failures.mon_check_dist_failure
            || hard_failures.analog_failure
            || hard_failures.mains_90deg_dist_failure
            || hard_failures.max_main_rt_failure
        {
            // We have a hard failure.
            // Raise an immediate and permanent error without debouncing.
            self.error_deb.error_no_debounce(m);
        }

        Debug::MonDebounce.log_u8(self.error_deb.count(m));

        if self.error_deb.is_ok(m) {
            Shutoff::MachineRunning
        } else {
            Shutoff::MachineShutoff
        }
    }

    /// Measure the main loop runtime.
    pub fn meas_main_runtime(&self, m: &MainCtx<'_>) {
        let now = timer_get_large();

        let runtime = now - self.prev_main_rt_stamp.get(m);
        self.prev_main_rt_stamp.set(m, now);

        let max_main_rt = self.max_main_rt.get(m).max(runtime);
        self.max_main_rt.set(m, max_main_rt);

        Debug::MaxRt.log_rel_large_timestamp(max_main_rt);
    }
}

/// Report an analog failure to the monitoring system.
pub fn mon_report_analog_failure() {
    ANALOG_FAILURE.store(true);
}

// vim: ts=4 sw=4 expandtab
