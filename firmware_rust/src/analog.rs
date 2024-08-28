use crate::{
    hw::Peripherals,
    mutex::{CriticalSection, MutexRefCell},
};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Channel {
    Setpoint,
    Vsense,
    ShuntDiff,
    ShuntHi,
}

impl Channel {
    pub const fn mask(&self) -> u8 {
        1 << *self as usize
    }

    pub fn select_next(&mut self) {
        *self = match self {
            Self::Setpoint => Self::Vsense,
            Self::Vsense => Self::ShuntDiff,
            Self::ShuntDiff => Self::ShuntHi,
            Self::ShuntHi => Self::Setpoint,
        };
    }
}

struct AdcInner {
    chan: Channel,
    enabled: u8,
    running: bool,
    result: [u16; 4],
    ok: u8,
}

impl AdcInner {
    pub const fn new() -> Self {
        Self {
            chan: Channel::Setpoint,
            enabled: 0,
            running: false,
            result: [0; 4],
            ok: 0,
        }
    }

    fn update_mux(&self, dp: &Peripherals) {
        match self.chan {
            Channel::Setpoint => {
                dp.ADC.admux.write(|w| w.refs().vcc().mux().adc0());
            }
            Channel::Vsense => {
                dp.ADC.admux.write(|w| w.refs().vcc().mux().adc1());
            }
            Channel::ShuntDiff => {
                dp.ADC.admux.write(|w| w.refs().vcc().mux().adc4_adc3_20x());
            }
            Channel::ShuntHi => {
                dp.ADC.admux.write(|w| w.refs().vcc().mux().adc4());
            }
        }

        //TODO settle time
    }

    #[rustfmt::skip]
    #[inline]
    fn start_conversion(&mut self, dp: &Peripherals) {
        dp.ADC.adcsr.modify(|_, w| {
            w.adif().set_bit()
             .adsc().set_bit()
        });
        self.running = true;
    }

    #[inline]
    fn conversion_done(&self, dp: &Peripherals) -> bool {
        dp.ADC.adcsr.read().adif().bit_is_set()
    }

    #[rustfmt::skip]
    pub fn init(&mut self, dp: &Peripherals) {
        dp.ADC.adcsr.write(|w| {
            w.adps().prescaler_128()
             .adie().clear_bit()
             .adfr().clear_bit()
             .adif().set_bit()
             .adsc().clear_bit()
             .aden().set_bit()
        });
        self.update_mux(dp);
        self.start_conversion(dp);
        //TODO offset compensation
    }

    pub fn run(&mut self, dp: &Peripherals) {
        if self.enabled & self.chan.mask() == 0 {
            self.ok &= !self.chan.mask();
            self.chan.select_next();
            self.running = false;
        }

        if self.running && (self.enabled & self.chan.mask()) != 0 && self.conversion_done(dp) {
            self.running = false;
            let result = dp.ADC.adc.read().bits();
            self.result[self.chan as usize] = result;
            self.ok |= self.chan.mask();
            self.chan.select_next();
        }

        if !self.running && (self.enabled & self.chan.mask()) != 0 {
            self.update_mux(dp);
            self.start_conversion(dp);
        }
    }

    //TODO enable/disable
}

pub struct Adc {
    inner: MutexRefCell<AdcInner>,
}

impl Adc {
    pub const fn new() -> Self {
        Self {
            inner: MutexRefCell::new(AdcInner::new()),
        }
    }

    pub fn init(&self, cs: CriticalSection<'_>, dp: &Peripherals) {
        self.inner.borrow_mut(cs).init(dp);
    }

    pub fn run(&self, cs: CriticalSection<'_>, dp: &Peripherals) {
        self.inner.borrow_mut(cs).run(dp);
    }
}

// vim: ts=4 sw=4 expandtab
