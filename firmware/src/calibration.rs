// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

//! Calibration constants and tables.

use crate::{
    freq::Freq,
    mains::{MAINS_HALFWAVE_DUR, MAINS_HALFWAVE_DUR_MS},
    pid::PidParams,
    system::rpm,
    temp::celsius,
    timer::RelLargeTimestamp,
};
use avr_q::{Q7p8, Q15p8, q7p8, q15p8};
use curveipo::Curve;

/// Basic system parameters.
pub mod system {
    use super::*;

    /// Initial delay after startup, before any actuation or monitoring is active.
    pub const STARTUP_DELAY: RelLargeTimestamp = RelLargeTimestamp::from_millis(300);

    /// Nominal maximum motor RPM.
    pub const MAX_RPM: i16 = 24000;

    /// Maximum motor RPM that will trigger a hard triac inhibit.
    pub const MOT_SOFT_LIMIT: Freq = rpm!(MAX_RPM + 500);

    /// Maximum motor RPM that will trigger a monitoring fault.
    pub const MOT_HARD_LIMIT: Freq = rpm!(MAX_RPM + 1500);
}

/// RPM PID controller parameters.
pub mod rpm_pid {
    use super::system::MAX_RPM;
    use super::*;

    /// RPM PID parameters for normal operation.
    pub const RPMPID_PARAMS: PidParams = PidParams {
        kp: q7p8!(const 8 / 5),
        ki: q7p8!(const 3 / 32),
        kd: q7p8!(const 1 / 80),
    };

    /// RPM PID parameters for speedometer syncing.
    pub const RPMPID_PARAMS_SYNCING: PidParams = PidParams {
        kp: q7p8!(const 2 / 1),
        ki: q7p8!(const 0),
        kd: q7p8!(const 0),
    };

    /// Negative I-limit curve for the RPM PID controller.
    pub const RPMPID_ILIM_NEG: Curve<Q7p8, (Q7p8, Q7p8), 4> = Curve::new([
        // (speedo, I-limit)
        (rpm!(0).0, q7p8!(const 0)),
        (rpm!(1000).0, q7p8!(const 0)),
        (rpm!(1001).0, q7p8!(const -10)),
        (rpm!(MAX_RPM).0, q7p8!(const -10)),
    ]);

    /// Positive I-limit curve for the RPM PID controller.
    pub const RPMPID_ILIM_POS: Curve<Q7p8, (Q7p8, Q7p8), 4> = Curve::new([
        // (speedo, I-limit)
        (rpm!(0).0, q7p8!(const 0)),
        (rpm!(1000).0, q7p8!(const 0)),
        (rpm!(1001).0, q7p8!(const 80)),
        (rpm!(MAX_RPM).0, q7p8!(const 80)),
    ]);
}

/// Setpoint measurement and processing.
pub mod setpoint {
    use super::*;

    /// Minimum setpoint below which the triac will be shut off.
    pub const SP_MIN_CUTOFF: Freq = rpm!(300);

    /// The number of virtual steps in the setpoint.
    /// The setpoint potentiometer reading snaps to these descrete virtual steps.
    pub const SP_STEPS: i16 = 100;

    /// Setpoint below this threshold will enforce speedometer re-syncing.
    pub const SP_SYNC_THRES: Freq = rpm!(1000);
}

/// Speedometer measurement and filtering.
pub mod speedo {
    use super::*;

    /// High level speed filter divider.
    pub const SPEED_FILTER_DIV: Q15p8 = q15p8!(const 2);

    /// High level timeout.
    /// Maximum amount of time the system can live without a valid speedometer reading.
    pub const NO_SPEED_TIMEOUT: RelLargeTimestamp = RelLargeTimestamp::from_millis(100);

    /// Low level timeout.
    /// If no speedometer edge is detected for this long, reset the speedometer Ok count.
    pub const SPEEDO_LOWLEVEL_TIMEOUT: RelLargeTimestamp = RelLargeTimestamp::from_millis(50);

    /// Low level Ok counter threshold.
    /// Need at least this many valid speedometer edges in a row to consider the speed valid for the first time.
    pub const OK_THRES: u8 = 5;

    /// Low level filter shift.
    /// Filters the measured low level speedometer edge durations.
    pub const FILTER_SHIFT: u8 = 5;

    /// Physical layout.
    /// Number of speedometer edges per motor revolution.
    pub const SPEEDO_FACT: u32 = 4;

    /// Substitute speedometer value curve during syncing when the actual speedometer input is invalid.
    pub const SYNC_SPEEDO_SUBSTITUTE: Curve<Freq, (Freq, Freq), 2> = Curve::new([
        // (setpoint, speedo-substitute)
        (rpm!(0), rpm!(0)),
        (rpm!(1000), rpm!(800)),
    ]);
}

/// Mains zero crossing detection.
pub mod mains {
    use super::*;

    /// Mains frequency in Hz.
    pub const MAINS_HZ: u8 = 50;

    /// Next mains capture relative to the previous successful capture.
    /// This is used to filter out spurious captures due to electrical noise.
    /// Captures before this time has passed since the previous capture are ignored.
    pub const MAINS_NEXT_CAPTURE: RelLargeTimestamp =
        RelLargeTimestamp::from_micros(MAINS_HALFWAVE_DUR.to_micros() * 98 / 100);
}

/// Temperature calibration constants and tables.
pub mod temp {
    use super::*;

    /// High temperature limit for the motor, above which a shutoff will be triggered.
    pub const TEMP_LIMIT_HI: Q7p8 = celsius!(100);

    /// Low temperature limit for the motor, below which a shutoff will be released.
    pub const TEMP_LIMIT_LO: Q7p8 = celsius!(80);

    /// Temperature filter divider.
    pub const TEMP_FILTER_DIV: Q15p8 = q15p8!(const 16);

    /// Motor NTC temperature curve.
    pub const NTC_CURVE: Curve<Q7p8, (Q7p8, Q7p8), 7> = Curve::new([
        // (kOhms, double deg Celsius)
        (q7p8!(const 3321 / 10000), celsius!(145)),
        (q7p8!(const 5174 / 10000), celsius!(125)),
        (q7p8!(const 8400 / 10000), celsius!(105)),
        (q7p8!(const 1429 / 1000), celsius!(85)),
        (q7p8!(const 2565 / 1000), celsius!(65)),
        (q7p8!(const 4891 / 1000), celsius!(45)),
        (q7p8!(const 1000 / 100), celsius!(25)),
    ]);

    /// Maximum kOhms for the motor NTC, above which a shutoff will immediately be triggered.
    pub const TEMP_MOT_KOHMS_LIM_HI: Q7p8 = q7p8!(const 120);

    /// Minimum kOhms for the motor NTC, below which a shutoff will immediately be triggered.
    pub const TEMP_MOT_KOHMS_LIM_LO: Q7p8 = q7p8!(const 1 / 10);

    /// Microcontroller temperature curve.
    pub const UC_CURVE: Curve<Q7p8, (Q7p8, Q7p8), 3> = Curve::new([
        // (adc / 8, double deg Celsius)
        (q7p8!(const 300 / 8), celsius!(25)),
        (q7p8!(const 370 / 8), celsius!(85)),
        (q7p8!(const 440 / 8), celsius!(145)),
    ]);
}

/// Monitoring constants and tables.
pub mod mon {
    use super::*;

    /// Distance between monitoring checks.
    pub const CHECK_DIST: RelLargeTimestamp = RelLargeTimestamp::from_millis(20);
    /// Immediate fault, if one actual monitoring distance is bigger than this.
    pub const CHECK_TIMEOUT: RelLargeTimestamp = RelLargeTimestamp::from_millis(100);

    /// Immediate fault, if mains zero crossing distance is bigger than this.
    pub const MAINS_ZERO_CROSSING_TIMEOUT: RelLargeTimestamp = RelLargeTimestamp::from_millis(100);

    /// Minimum amount of CPU stack space that must be free all the time.
    /// Immediate fault, if less stack space is free.
    pub const MIN_STACK_SPACE: u16 = 64;

    /// Maximum allowed main loop execution time.
    /// Immediate fault, if the main loop execution time is bigger than this.
    pub const MAX_MAIN_RT_LIMIT: RelLargeTimestamp = RelLargeTimestamp::from_millis(5);

    /// Setpoint history.
    /// Length = SP_HIST_DIST * SP_HIST_COUNT = 3 seconds
    pub const SP_HIST_DIST: RelLargeTimestamp = RelLargeTimestamp::from_micros(333333);
    /// Number if elements in the setpoint history.
    pub const SP_HIST_COUNT: usize = 9;

    /// Don't run monitoring, if the setpoint gradient in history is bigger than this.
    pub const SP_GRADIENT_THRES: Freq = rpm!(1000);

    /// Step size for one error event.
    pub const ERROR_DEBOUNCE_ERRSTEP: u8 = 3;
    /// Debounce limit to enter fault state.
    pub const ERROR_DEBOUNCE_LIMIT: u8 = 120;
    /// Sticky -> fault state cannot be healed.
    pub const ERROR_DEBOUNCE_STICKY: bool = true;

    /// Setpoint vs. speedometer deviation threshold that is considered to be an unexpected mismatch.
    pub const SPEEDO_TOLERANCE: Freq = rpm!(1000);
    /// Monitoring activation threshold for speedometer input.
    /// Monitoring is not active below this threshold.
    pub const MON_ACTIVE_THRES: Freq = rpm!(7500);
}

/// Monitoring Power-On-Check constants and tables.
pub mod mon_pocheck {
    use super::*;

    /// Duration of the `PoStatePart::Pre` part.
    pub const DUR_PRE: RelLargeTimestamp = RelLargeTimestamp::from_millis(50);

    /// Duration of the `PoStatePart::Check` part.
    pub const DUR_CHECK: RelLargeTimestamp = RelLargeTimestamp::from_millis(400);

    /// RPM below or equal to this limit are considered to be zero RPM.
    pub const RPM_ZERO_LIMIT: Freq = rpm!(5);

    /// Triac offset for the enabled-check.
    pub const TRIAC_TRIG_OFFS_ENABLED_MS: Q7p8 = MAINS_HALFWAVE_DUR_MS.const_div(q7p8!(const 10));

    /// Show state transitions on the debug pin?
    pub const DEBUG_PIN_ENA: bool = true;
}

// vim: ts=4 sw=4 expandtab
