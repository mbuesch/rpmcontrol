use crate::{
    hw::interrupt,
    mutex::{IrqCtx, MainCtx, MutexCell},
    system::SysPeriph,
    timer::{timer_get_large_cs, LargeTimestamp, RelLargeTimestamp},
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

    pub fn select_next(&self) -> AdcChannel {
        match self {
            Self::Setpoint => Self::ShuntDiff,
            Self::ShuntDiff => Self::ShuntHi,
            Self::ShuntHi => Self::Setpoint,
        }
    }
}

pub struct Adc {
    chan: MutexCell<AdcChannel>,
    enabled: MutexCell<u8>,
    running: MutexCell<bool>,
    result: [MutexCell<u16>; 3],
    ok: MutexCell<u8>,
}

impl Adc {
    pub const fn new() -> Self {
        Self {
            chan: MutexCell::new(AdcChannel::Setpoint),
            enabled: MutexCell::new(0),
            running: MutexCell::new(false),
            result: [MutexCell::new(0), MutexCell::new(0), MutexCell::new(0)],
            ok: MutexCell::new(0),
        }
    }

    fn update_mux(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        match self.chan.get(m) {
            AdcChannel::Setpoint => {
                sp.ADC.admux().write(|w| w.refs().vcc().mux().adc0());
            }
            AdcChannel::ShuntDiff => {
                sp.ADC
                    .admux()
                    .write(|w| w.refs().vcc().mux().adc4_adc3_20x());
            }
            AdcChannel::ShuntHi => {
                sp.ADC.admux().write(|w| w.refs().vcc().mux().adc4());
            }
        }

        //TODO settle time
    }

    #[rustfmt::skip]
    #[inline]
    fn start_conversion(&self, _m: &MainCtx<'_>, sp: &SysPeriph) {
        sp.ADC.adcsra().modify(|_, w| {
            w.adif().set_bit()
             .adsc().set_bit()
        });
    }

    #[inline]
    fn conversion_done(&self, _m: &MainCtx<'_>, sp: &SysPeriph) -> bool {
        sp.ADC.adcsra().read().adif().bit_is_set()
    }

    #[rustfmt::skip]
    pub fn init(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        sp.ADC.adcsra().write(|w| {
            w.adps().prescaler_128()
             .adie().clear_bit()
             .adif().set_bit()
             .adsc().clear_bit()
             .aden().set_bit()
        });

        self.update_mux(m, sp);
        self.start_conversion(m, sp);
        while !self.conversion_done(m, sp) {}

        //TODO offset compensation
    }

    pub fn run(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        let chan = self.chan.get(m);
        if !self.is_enabled(m, chan) {
            self.ok.set(m, self.ok.get(m) & !chan.mask());
            self.chan.set(m, chan.select_next());
            self.running.set(m, false);
        }

        let chan = self.chan.get(m);
        if self.running.get(m) && self.is_enabled(m, chan) && self.conversion_done(m, sp) {
            self.result[chan as usize].set(m, sp.ADC.adc().read().bits());
            self.ok.set(m, self.ok.get(m) | chan.mask());
            self.chan.set(m, self.chan.get(m).select_next());
            self.running.set(m, false);
        }

        let chan = self.chan.get(m);
        if !self.running.get(m) && self.is_enabled(m, chan) {
            self.update_mux(m, sp);
            self.start_conversion(m, sp);
            self.running.set(m, true);
        }
    }

    fn is_enabled(&self, m: &MainCtx<'_>, chan: AdcChannel) -> bool {
        self.enabled.get(m) & chan.mask() != 0
    }

    pub fn enable(&self, m: &MainCtx<'_>, chan_mask: u8) {
        self.enabled.set(m, chan_mask);
    }

    pub fn get_result(&self, m: &MainCtx<'_>, chan: AdcChannel) -> Option<u16> {
        if self.ok.get(m) & chan.mask() == 0 {
            None
        } else {
            Some(self.result[chan as usize].get(m))
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
        sp.AC.acsra().write(|w| {
            w.acie().set_bit()
             .aci().set_bit()
             .acis().on_toggle()
        });
    }
}

#[derive(Clone)]
pub struct AcCapture {
    stamp: LargeTimestamp,
    new: bool,
    rising: bool,
}

impl AcCapture {
    const fn new() -> Self {
        Self {
            stamp: LargeTimestamp(0),
            new: false,
            rising: false,
        }
    }

    pub fn is_new(&self) -> bool {
        self.new
    }

    pub fn is_rising(&self) -> bool {
        self.rising
    }

    pub fn stamp(&self) -> LargeTimestamp {
        self.stamp
    }

    pub fn clone_and_reset(&mut self) -> Self {
        let ret = self.clone();
        self.new = false;
        ret
    }
}

pub static mut AC_CAPTURE: AcCapture = AcCapture::new();

/// AC events closer than this to the previous valid event are ignored.
const AC_CAPTURE_MINDIST: RelLargeTimestamp = RelLargeTimestamp::from_micros(256);

/// Analog Comparator interrupt.
pub fn irq_handler_ana_comp(c: &IrqCtx) {
    // SAFETY: This interrupt shall not call into anything and not modify anything,
    //         except for timer and the stored time stamp.
    //         The rest of the system safety depends on this due to the system
    //         wide creation of the `system_cs` CriticalSection.
    //         See main.rs.

    let now = timer_get_large_cs(c.cs());

    // SAFETY: `AC_CAPTURE` is only accessed from here and
    //         from [ac_capture_get] with interrupts disabled.
    unsafe {
        if now >= AC_CAPTURE.stamp + AC_CAPTURE_MINDIST {
            if AC_CAPTURE.new {
                // ac_capture_get() has not been called frequently enough.
                //TODO?
            }
            AC_CAPTURE.stamp = now;
            AC_CAPTURE.new = true;
            AC_CAPTURE.rising = !AC_CAPTURE.rising;
        }
    }
}

#[allow(static_mut_refs)]
pub fn ac_capture_get() -> AcCapture {
    interrupt::free(|_cs| {
        // SAFETY: Interrupts are disabled.
        //         Therefore, it is safe to access the analog comparator
        //         interrupt data.
        //         See corresponding safety comment in `ANA_COMP` ISR.
        unsafe { AC_CAPTURE.clone_and_reset() }
    })
}

// vim: ts=4 sw=4 expandtab
