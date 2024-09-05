use crate::{
    hw::interrupt,
    mutex::CriticalSection,
    system::SysPeriph,
    timer::{timer_get, Timestamp},
};

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum AdcChannel {
    Setpoint,
    ShuntDiff,
    ShuntHi,
}

impl AdcChannel {
    pub const fn mask(&self) -> u8 {
        1 << *self as usize
    }

    pub fn select_next(&mut self) {
        *self = match self {
            Self::Setpoint => Self::ShuntDiff,
            Self::ShuntDiff => Self::ShuntHi,
            Self::ShuntHi => Self::Setpoint,
        };
    }
}

pub struct Adc {
    chan: AdcChannel,
    enabled: u8,
    running: bool,
    result: [u16; 3],
    ok: u8,
}

impl Adc {
    pub const fn new() -> Self {
        Self {
            chan: AdcChannel::Setpoint,
            enabled: 0,
            running: false,
            result: [0; 3],
            ok: 0,
        }
    }

    fn update_mux(&self, sp: &SysPeriph) {
        match self.chan {
            AdcChannel::Setpoint => {
                sp.ADC.admux.write(|w| w.refs().vcc().mux().adc0());
            }
            AdcChannel::ShuntDiff => {
                sp.ADC.admux.write(|w| w.refs().vcc().mux().adc4_adc3_20x());
            }
            AdcChannel::ShuntHi => {
                sp.ADC.admux.write(|w| w.refs().vcc().mux().adc4());
            }
        }

        //TODO settle time
    }

    #[rustfmt::skip]
    #[inline]
    fn start_conversion(&mut self, sp: &SysPeriph) {
        sp.ADC.adcsr.modify(|_, w| {
            w.adif().set_bit()
             .adsc().set_bit()
        });
    }

    #[inline]
    fn conversion_done(&self, sp: &SysPeriph) -> bool {
        sp.ADC.adcsr.read().adif().bit_is_set()
    }

    #[rustfmt::skip]
    pub fn init(&mut self, sp: &SysPeriph) {
        sp.ADC.adcsr.write(|w| {
            w.adps().prescaler_128()
             .adie().clear_bit()
             .adfr().clear_bit()
             .adif().set_bit()
             .adsc().clear_bit()
             .aden().set_bit()
        });

        self.update_mux(sp);
        self.start_conversion(sp);
        while !self.conversion_done(sp) {}

        //TODO offset compensation
    }

    pub fn run(&mut self, sp: &SysPeriph) {
        if !self.is_enabled(self.chan) {
            self.ok &= !self.chan.mask();
            self.chan.select_next();
            self.running = false;
        }

        if self.running && self.is_enabled(self.chan) && self.conversion_done(sp) {
            self.result[self.chan as usize] = sp.ADC.adc.read().bits();
            self.ok |= self.chan.mask();
            self.chan.select_next();
            self.running = false;
        }

        if !self.running && self.is_enabled(self.chan) {
            self.update_mux(sp);
            self.start_conversion(sp);
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

pub struct Ac(());

impl Ac {
    pub const fn new() -> Self {
        Ac(())
    }

    #[rustfmt::skip]
    pub fn init(&self, sp: &SysPeriph) {
        sp.AC.acsr.write(|w| {
            w.acie().set_bit()
             .aci().set_bit()
             .acis().on_rising_edge()
        });
    }
}

#[derive(Clone)]
pub struct AcCapture {
    stamp: Timestamp,
    flags: u8,
}

impl AcCapture {
    pub const FLAG_NEW: u8 = 0x01;

    const fn new() -> Self {
        Self {
            stamp: Timestamp(0),
            flags: 0,
        }
    }

    pub fn is_new(&self) -> bool {
        self.flags & Self::FLAG_NEW != 0
    }

    pub fn stamp(&self) -> Timestamp {
        self.stamp
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
    //         except for timer and the stored time stamp.
    //         The rest of the system safety depends on this due to the system
    //         wide creation of the `system_cs` CriticalSection.
    //         See main.rs.

    // SAFETY: Creating a CS manually is safe, because
    //         we are in atomic interrupt context with interrupts disabled.
    let cs = unsafe { CriticalSection::new() };

    unsafe {
        if AC_CAPTURE.flags != 0 {
            // ac_capture_get() has not been called frequently enough.
            //TODO?
        }
        AC_CAPTURE.stamp = timer_get(cs);
        AC_CAPTURE.flags = AcCapture::FLAG_NEW;
    }
}

pub fn ac_capture_get() -> AcCapture {
    interrupt::free(|_cs| {
        // SAFETY: Interrupts are disabled.
        //         Therefore, it is safe to access the analog comparator
        //         interrupt data.
        unsafe { AC_CAPTURE.clone_and_reset() }
    })
}

// vim: ts=4 sw=4 expandtab
