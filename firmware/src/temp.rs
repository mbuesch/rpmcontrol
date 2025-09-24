use crate::{
    debug::Debug,
    filter::Filter,
    fixpt::{Fixpt, fixpt},
    mutex::{MainCtx, MutexCell},
    shutoff::Shutoff,
};
use curveipo::Curve;

macro_rules! celsius {
    ($cel:literal) => {
        const { fixpt!($cel / 2) }
    };
}

const R1: Fixpt = fixpt!(10); // kOhms
const ADC_UREF: Fixpt = fixpt!(5); // volts
const ADC_MAX: u16 = 0x3FF;
const TEMP_LIMIT_HI: Fixpt = celsius!(100);
const TEMP_LIMIT_LO: Fixpt = celsius!(80);
const TEMP_FILTER_DIV: Fixpt = fixpt!(16);

const NTC_CURVE: Curve<Fixpt, (Fixpt, Fixpt), 7> = Curve::new([
    // (kOhms, double deg Celsius)
    (fixpt!(3321 / 10000), celsius!(145)),
    (fixpt!(5174 / 10000), celsius!(125)),
    (fixpt!(8400 / 10000), celsius!(105)),
    (fixpt!(1429 / 1000), celsius!(85)),
    (fixpt!(2565 / 1000), celsius!(65)),
    (fixpt!(4891 / 1000), celsius!(45)),
    (fixpt!(1000 / 100), celsius!(25)),
]);

const UC_CURVE: Curve<Fixpt, (Fixpt, Fixpt), 3> = Curve::new([
    // (adc / 8, double deg Celsius)
    (fixpt!(300 / 8), celsius!(25)),
    (fixpt!(370 / 8), celsius!(85)),
    (fixpt!(440 / 8), celsius!(145)),
]);

/// Convert motor temperature ADC to volts at ADC pin.
fn mot_adc_to_volts(adc: u16) -> Fixpt {
    let num = adc as i16 * ADC_UREF.to_int();
    let den = ADC_MAX as i16;
    Fixpt::from_fraction(num, den)
}

/// Convert motor temperature voltage to resistance of temperature sensor.
fn mot_volts_to_kohms(u2: Fixpt) -> Fixpt {
    (R1 * u2) / (ADC_UREF - u2)
}

/// Convert kOhms to degree double-Celsius.
fn mot_kohms_to_celsius_double(r2: Fixpt) -> Fixpt {
    NTC_CURVE.lin_inter(r2)
}

/// Convert microcontroller temp ADC to degree double-Celsius.
fn uc_adc_to_celsius_double(adc: u16) -> Fixpt {
    let adc = adc as i16;
    UC_CURVE.lin_inter(Fixpt::from_fraction(adc, 8))
}

pub struct TempAdc {
    /// Microcontroller temperature.
    pub uc: Option<u16>,
    /// Motor temperature.
    pub mot: Option<u16>,
}

pub struct Temp {
    shutoff: MutexCell<Shutoff>,
    filter_uc: Filter,
    filter_mot: Filter,
}

impl Temp {
    pub const fn new() -> Self {
        Self {
            shutoff: MutexCell::new(Shutoff::MachineShutoff),
            filter_uc: Filter::new(),
            filter_mot: Filter::new(),
        }
    }

    pub fn init(&self, m: &MainCtx<'_>) {
        self.shutoff.set(m, Shutoff::MachineRunning);
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
