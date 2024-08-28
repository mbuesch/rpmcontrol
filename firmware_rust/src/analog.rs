use crate::{
    hw::Peripherals,
    mutex::{CriticalSection, MutexRefCell},
};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum AdcChannel {
    Setpoint,
    Vsense,
    ShuntDiff,
    ShuntHi,
}

impl AdcChannel {
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
    chan: AdcChannel,
    enabled: u8,
    running: bool,
    result: [u16; 4],
    ok: u8,
}

impl AdcInner {
    pub const fn new() -> Self {
        Self {
            chan: AdcChannel::Setpoint,
            enabled: 0,
            running: false,
            result: [0; 4],
            ok: 0,
        }
    }

    fn update_mux(&self, dp: &Peripherals) {
        match self.chan {
            AdcChannel::Setpoint => {
                dp.ADC.admux.write(|w| w.refs().vcc().mux().adc0());
            }
            AdcChannel::Vsense => {
                dp.ADC.admux.write(|w| w.refs().vcc().mux().adc1());
            }
            AdcChannel::ShuntDiff => {
                dp.ADC.admux.write(|w| w.refs().vcc().mux().adc4_adc3_20x());
            }
            AdcChannel::ShuntHi => {
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
        while !self.conversion_done(dp) {}

        //TODO offset compensation
    }

    pub fn run(&mut self, dp: &Peripherals) {
        if !self.is_enabled(self.chan) {
            self.ok &= !self.chan.mask();
            self.chan.select_next();
            self.running = false;
        }

        if self.running && self.is_enabled(self.chan) && self.conversion_done(dp) {
            self.result[self.chan as usize] = dp.ADC.adc.read().bits();
            self.ok |= self.chan.mask();
            self.chan.select_next();
            self.running = false;
        }

        if !self.running && self.is_enabled(self.chan) {
            self.update_mux(dp);
            self.start_conversion(dp);
            self.running = true;
        }
    }

    fn is_enabled(&self, chan: AdcChannel) -> bool {
        self.enabled & chan.mask() != 0
    }

    pub fn enable(&mut self, chan_mask: u8) {
        self.enabled = chan_mask;
    }

    pub fn get_result(&self, chan: AdcChannel) -> Option<u16> {
        if self.ok & chan.mask() == 0 {
            None
        } else {
            Some(self.result[chan as usize])
        }
    }
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

    pub fn enable(&self, cs: CriticalSection<'_>, chan_mask: u8) {
        self.inner.borrow_mut(cs).enable(chan_mask);
    }

    pub fn get_result(&self, cs: CriticalSection<'_>, chan: AdcChannel) -> Option<u16> {
        self.inner.borrow(cs).get_result(chan)
    }
}

pub struct Ac {}

#[derive(Clone)]
pub struct AcCapture {
    stamp: u8,
    flags: u8,
}

impl AcCapture {
    pub const FLAG_NEW: u8 = 0x01;
    pub const FLAG_LOST: u8 = 0x02;

    const fn new() -> Self {
        Self { stamp: 0, flags: 0 }
    }

    pub fn clone_and_reset(&mut self) -> Self {
        let ret = self.clone();
        self.flags = 0;
        ret
    }
}

pub static mut AC_CAPTURE: AcCapture = AcCapture::new();

#[avr_device::interrupt(attiny26)]
fn ANA_COMP() {
    // SAFETY: This interrupt shall not call into anything and not modify anything,
    //         except for the stored time stamp.
    //         The rest of the system safety depends on this. See main.rs.
    unsafe {
        AC_CAPTURE.stamp = 0; //TODO
        if AC_CAPTURE.flags == 0 {
            AC_CAPTURE.flags = AcCapture::FLAG_NEW;
        } else {
            AC_CAPTURE.flags = AcCapture::FLAG_NEW | AcCapture::FLAG_LOST;
        }
    }
}

// vim: ts=4 sw=4 expandtab
