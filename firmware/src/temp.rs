use crate::{
    debug::Debug,
    filter::Filter,
    mutex::{MainCtx, MainCtxCell},
    shutoff::Shutoff,
};
use avr_q::{Q7p8, q7p8};
use curveipo::Curve;

macro_rules! celsius {
    ($cel:literal) => {
        const { q7p8!(const $cel / 2) }
    };
}

const R1: Q7p8 = q7p8!(const 10); // kOhms
const ADC_UREF: Q7p8 = q7p8!(const 5); // volts
const ADC_MAX: u16 = 0x3FF;
const TEMP_LIMIT_HI: Q7p8 = celsius!(100);
const TEMP_LIMIT_LO: Q7p8 = celsius!(80);
const TEMP_FILTER_DIV: Q7p8 = q7p8!(const 16);

const NTC_CURVE: Curve<Q7p8, (Q7p8, Q7p8), 7> = Curve::new([
    // (kOhms, double deg Celsius)
    (q7p8!(const 3321 / 10000), celsius!(145)),
    (q7p8!(const 5174 / 10000), celsius!(125)),
    (q7p8!(const 8400 / 10000), celsius!(105)),
    (q7p8!(const 1429 / 1000), celsius!(85)),
    (q7p8!(const 2565 / 1000), celsius!(65)),
    (q7p8!(const 4891 / 1000), celsius!(45)),
    (q7p8!(const 1000 / 100), celsius!(25)),
]);

const UC_CURVE: Curve<Q7p8, (Q7p8, Q7p8), 3> = Curve::new([
    // (adc / 8, double deg Celsius)
    (q7p8!(const 300 / 8), celsius!(25)),
    (q7p8!(const 370 / 8), celsius!(85)),
    (q7p8!(const 440 / 8), celsius!(145)),
]);

/// Convert motor temperature ADC to volts at ADC pin.
fn mot_adc_to_volts(adc: u16) -> Q7p8 {
    let num = adc as i16 * ADC_UREF.to_int();
    let den = ADC_MAX as i16;
    Q7p8::from_fraction(num, den)
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
    UC_CURVE.lin_inter(Q7p8::from_fraction(adc, 8))
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

    pub fn run(&self, m: &MainCtx<'_>, temp_adc: TempAdc) {
        let mut below_lo = false;
        let mut above_hi = false;

        if let Some(temp_mot) = temp_adc.mot {
            let temp_mot_volts = mot_adc_to_volts(temp_mot);
            let temp_mot_kohms = mot_volts_to_kohms(temp_mot_volts);
            let temp_mot_cel = mot_kohms_to_celsius_double(temp_mot_kohms);

            let temp_mot_cel = self.filter_mot.run(m, temp_mot_cel, TEMP_FILTER_DIV);

            if temp_mot_cel > TEMP_LIMIT_HI {
                above_hi = true;
            } else if temp_mot_cel < TEMP_LIMIT_LO {
                below_lo = true;
            }

            Debug::TempMot.log_fixpt(temp_mot_cel);
        }

        if let Some(temp_uc) = temp_adc.uc {
            let temp_uc_cel = uc_adc_to_celsius_double(temp_uc);

            let temp_uc_cel = self.filter_uc.run(m, temp_uc_cel, TEMP_FILTER_DIV);

            if temp_uc_cel > TEMP_LIMIT_HI {
                above_hi = true;
            } else if temp_uc_cel < TEMP_LIMIT_LO {
                below_lo = true;
            }

            Debug::TempUc.log_fixpt(temp_uc_cel);
        }

        if below_lo {
            self.shutoff.set(m, Shutoff::MachineRunning);
        }
        if above_hi {
            self.shutoff.set(m, Shutoff::MachineShutoff);
        }
    }

    pub fn get_shutoff(&self, m: &MainCtx<'_>) -> Shutoff {
        self.shutoff.get(m)
    }
}

// vim: ts=4 sw=4 expandtab
