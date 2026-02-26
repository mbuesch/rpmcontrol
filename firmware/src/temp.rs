// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

use crate::{
    calibration::temp::{
        NTC_CURVE, TEMP_FILTER_DIV, TEMP_LIMIT_HI, TEMP_LIMIT_LO, TEMP_MOT_KOHMS_LIM_HI,
        TEMP_MOT_KOHMS_LIM_LO, UC_CURVE,
    },
    debug::Debug,
    filter::Filter,
    shutoff::Shutoff,
    timer::LargeTimestamp,
};
use avr_context::{MainCtx, MainCtxCell};
use avr_q::{Q7p8, q7p8};

macro_rules! celsius {
    ($cel:literal) => {
const { q7p8!(const $cel / 2) }
    };
}
pub(crate) use celsius;

const R1: Q7p8 = q7p8!(const 10); // kOhms
const ADC_UREF: Q7p8 = q7p8!(const 5); // volts
const ADC_MAX: u16 = 0x3FF;

/// Convert motor temperature ADC to volts at ADC pin.
fn mot_adc_to_volts(adc: u16) -> Q7p8 {
    let num = adc as i16 * ADC_UREF.to_int() as i16;
    let den = ADC_MAX as i16;
    q7p8!(num / den)
}

/// Convert motor temperature voltage to resistance of temperature sensor.
fn mot_volts_to_kohms(u2: Q7p8) -> Q7p8 {
    (R1 * u2) / (ADC_UREF - u2)
}

/// Convert kOhms to degree double-Celsius.
fn mot_kohms_to_celsius_double(r2: Q7p8) -> Q7p8 {
    NTC_CURVE.lin_inter(r2)
}

/// Convert microcontroller temp ADC to degree double-Celsius.
fn uc_adc_to_celsius_double(adc: u16) -> Q7p8 {
    let adc = adc as i16;
    UC_CURVE.lin_inter(q7p8!(adc / 8))
}

pub struct TempAdc {
    /// Microcontroller temperature.
    pub uc: Option<u16>,
    /// Motor temperature.
    pub mot: Option<u16>,
}

pub struct Temp {
    shutoff: MainCtxCell<Shutoff>,
    filter_uc: Filter,
    filter_mot: Filter,
}

impl Temp {
    pub const fn new() -> Self {
        Self {
            shutoff: MainCtxCell::new(Shutoff::MachineShutoff),
            filter_uc: Filter::new(),
            filter_mot: Filter::new(),
        }
    }

    pub fn init(&self, _m: &MainCtx<'_>, _now: LargeTimestamp) {
        // Nothing to do.
    }

    pub fn run(&self, m: &MainCtx<'_>, temp_adc: TempAdc) {
        let mut must_shutoff = false;
        let mut may_restart = true;

        if let Some(temp_mot) = temp_adc.mot {
            let temp_mot_volts = mot_adc_to_volts(temp_mot);
            let temp_mot_kohms = mot_volts_to_kohms(temp_mot_volts);
            let temp_mot_cel;

            if temp_mot_kohms >= TEMP_MOT_KOHMS_LIM_HI || temp_mot_kohms <= TEMP_MOT_KOHMS_LIM_LO {
                must_shutoff = true;
                temp_mot_cel = celsius!(-20);
            } else {
                temp_mot_cel = self.filter_mot.run(
                    m,
                    mot_kohms_to_celsius_double(temp_mot_kohms),
                    TEMP_FILTER_DIV,
                );

                if temp_mot_cel > TEMP_LIMIT_HI {
                    must_shutoff = true;
                }
                if temp_mot_cel >= TEMP_LIMIT_LO {
                    may_restart = false;
                }
            }

            Debug::TempMot.log_fixpt(temp_mot_cel);
        } else {
            may_restart = false;
        }

        if let Some(temp_uc) = temp_adc.uc {
            let temp_uc_cel = uc_adc_to_celsius_double(temp_uc);

            let temp_uc_cel = self.filter_uc.run(m, temp_uc_cel, TEMP_FILTER_DIV);

            if temp_uc_cel > TEMP_LIMIT_HI {
                must_shutoff = true;
            }
            if temp_uc_cel >= TEMP_LIMIT_LO {
                may_restart = false;
            }

            Debug::TempUc.log_fixpt(temp_uc_cel);
        } else {
            may_restart = false;
        }

        if must_shutoff {
            self.shutoff.set(m, Shutoff::MachineShutoff);
        } else if may_restart {
            self.shutoff.set(m, Shutoff::MachineRunning);
        }
    }

    pub fn get_shutoff(&self, m: &MainCtx<'_>) -> Shutoff {
        self.shutoff.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
