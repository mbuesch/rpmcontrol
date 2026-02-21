// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

//! Calibration constants and tables.

use crate::{freq::Freq, pid::PidParams, system::rpm, timer::RelLargeTimestamp};
use avr_q::{Q7p8, q7p8};
use curveipo::Curve;

/// Initial delay after startup, before any actuation or monitoring is active.
pub const STARTUP_DELAY: RelLargeTimestamp = RelLargeTimestamp::from_millis(300);

/// RPM PID parameters for normal operation.
pub const RPMPID_PARAMS: PidParams = PidParams {
    kp: q7p8!(const 5 / 1),
    ki: q7p8!(const 1 / 4),
    kd: q7p8!(const 0),
};

/// RPM PID parameters for speedometer syncing.
pub const RPMPID_PARAMS_SYNCING: PidParams = PidParams {
    kp: q7p8!(const 5 / 1),
    ki: q7p8!(const 0),
    kd: q7p8!(const 0),
};

/// Negative I-limit curve for the RPM PID controller.
pub const RPMPID_ILIM_NEG: Curve<Q7p8, (Q7p8, Q7p8), 4> = Curve::new([
    // (speedo, I-limit)
    (rpm!(0).0, q7p8!(const 0)),
    (rpm!(1000).0, q7p8!(const 0)),
    (rpm!(1001).0, q7p8!(const -99)),
    (rpm!(MAX_RPM).0, q7p8!(const -99)),
]);

/// Positive I-limit curve for the RPM PID controller.
pub const RPMPID_ILIM_POS: Curve<Q7p8, (Q7p8, Q7p8), 4> = Curve::new([
    // (speedo, I-limit)
    (rpm!(0).0, q7p8!(const 0)),
    (rpm!(1000).0, q7p8!(const 0)),
    (rpm!(1001).0, q7p8!(const 99)),
    (rpm!(MAX_RPM).0, q7p8!(const 99)),
]);

/// Substitute speedometer value curve for syncing, if the actual speedometer input is invalid.
pub const SYNC_SPEEDO_SUBSTITUTE: Curve<Freq, (Freq, Freq), 2> = Curve::new([
    // (setpoint, speedo-substitute)
    (rpm!(0), rpm!(0)),
    (rpm!(1000), rpm!(800)),
]);

/// Nominal maximum motor RPM.
pub const MAX_RPM: i16 = 24000;

/// Maximum motor RPM that will trigger a hard triac inhibit.
pub const MOT_SOFT_LIMIT: Freq = rpm!(MAX_RPM + 500);

/// Maximum motor RPM that will trigger a monitoring fault.
pub const MOT_HARD_LIMIT: Freq = rpm!(MAX_RPM + 1500);

/// Motor speed below this threshold will trigger speedometer re-syncing.
pub const RPM_SYNC_THRES: Freq = rpm!(1000);

/// Speedometer filter divider.
pub const SPEED_FILTER_DIV: Q7p8 = q7p8!(const 2 / 1);

/// Minimum setpoint below which the triac will be shut off.
pub const SP_MIN_CUTOFF: Freq = rpm!(300);

/// Temperature calibration constants and tables.
pub mod temp {
    use super::*;
    use crate::temp::celsius;

    /// High temperature limit for the motor, above which a shutoff will be triggered.
    pub const TEMP_LIMIT_HI: Q7p8 = celsius!(100);

    /// Low temperature limit for the motor, below which a shutoff will be released.
    pub const TEMP_LIMIT_LO: Q7p8 = celsius!(80);

    /// Temperature filter divider.
    pub const TEMP_FILTER_DIV: Q7p8 = q7p8!(const 16);

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

    /// Microcontroller temperature curve.
    pub const UC_CURVE: Curve<Q7p8, (Q7p8, Q7p8), 3> = Curve::new([
        // (adc / 8, double deg Celsius)
        (q7p8!(const 300 / 8), celsius!(25)),
        (q7p8!(const 370 / 8), celsius!(85)),
        (q7p8!(const 440 / 8), celsius!(145)),
    ]);
}
// vim: ts=4 sw=4 expandtab
