use crate::{
    fixpt::{Fixpt, fixpt},
    mutex::MainCtx,
};
use curveipo::Curve;

const R_1: Fixpt = fixpt!(10); // kOhms
const R_NTC_REF: Fixpt = fixpt!(10); // kOhms
const ADC_MAX: u16 = 0x3FF;

const NTC_CURVE: Curve<Fixpt, (Fixpt, Fixpt), 7> = Curve::new([
    // (kOhms, deg Celsius)
    (fixpt!(1000 / 100), fixpt!(25)),
    (fixpt!(4891 / 1000), fixpt!(45)),
    (fixpt!(2565 / 1000), fixpt!(65)),
    (fixpt!(1429 / 1000), fixpt!(85)),
    (fixpt!(8400 / 10000), fixpt!(105)),
    (fixpt!(5174 / 10000), fixpt!(125)),
    (fixpt!(3321 / 10000), fixpt!(145)),
]);

/// Convert a raw ADC value to NTC kOhms.
fn mot_adc_to_kohms(adc: u16) -> Fixpt {
    // Limit to avoid div by zero.
    let adc = adc.max(1);

    // R_NTC = R_1 * ((1023 / ADC) - 1)
    R_1 * (Fixpt::from_fraction(ADC_MAX as _, adc as _) - fixpt!(1))
}

/// Convert kOhms to degree Celsius.
fn mot_kohms_to_celsius(r_ntc: Fixpt) -> Fixpt {
    NTC_CURVE.lin_inter(r_ntc)
}

pub struct TempAdc {
    /// Microcontroller temperature.
    pub uc: Option<u16>,
    /// Motor temperature.
    pub mot: Option<u16>,
}

pub struct Temp {}

impl Temp {
    pub const fn new() -> Self {
        Self {}
    }

    pub fn run(&self, m: &MainCtx<'_>, temp_adc: TempAdc) {
        //TODO
    }
}

// vim: ts=4 sw=4 expandtab
