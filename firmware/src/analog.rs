use crate::{
    hw::interrupt,
    mutex::{IrqCtx, MainCtx, MutexCell},
    system::SysPeriph,
    timer::{LargeTimestamp, RelLargeTimestamp, timer_get_large_cs},
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
    settled: MutexCell<bool>,
    enabled: MutexCell<u8>,
    running: MutexCell<bool>,
    result: [MutexCell<u16>; 3],
    ok: MutexCell<u8>,
}

impl Adc {
    pub const fn new() -> Self {
        Self {
            chan: MutexCell::new(AdcChannel::Setpoint),
            settled: MutexCell::new(false),
            enabled: MutexCell::new(0),
            running: MutexCell::new(false),
            result: [MutexCell::new(0), MutexCell::new(0), MutexCell::new(0)],
            ok: MutexCell::new(0),
        }
    }

    #[rustfmt::skip]
    fn update_mux(&self, m: &MainCtx<'_>, sp: &SysPeriph) {
        match self.chan.get(m) {
            AdcChannel::Setpoint => {
                sp.ADC.admux().write(|w| {
                    w.refs().vcc().mux().adc0()
                });
            }
            AdcChannel::ShuntDiff => {
                sp.ADC.admux().write(|w| {
                    w.refs().vcc().mux().adc4_adc3_20x()
                });
            }
            AdcChannel::ShuntHi => {
                sp.ADC.admux().write(|w| {
                    w.refs().vcc().mux().adc4()
                });
            }
        }
        self.set_settled(m, false);
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
        let mut chan = self.chan.get(m);
        if !self.is_enabled(m, chan) {
            self.set_ok(m, chan, false);
            chan = self.select_next_chan(m);
            self.set_running(m, false);
        }

        if self.is_enabled(m, chan) && self.is_running(m) && self.conversion_done(m, sp) {
            if self.is_settled(m) {
                self.result[chan as usize].set(m, sp.ADC.adc().read().bits());
                self.set_ok(m, chan, true);
                chan = self.select_next_chan(m);
                self.set_running(m, false);
            } else {
                self.set_settled(m, true);
                self.start_conversion(m, sp);
            }
        }

        if self.is_enabled(m, chan) && !self.is_running(m) {
            self.update_mux(m, sp);
            self.start_conversion(m, sp);
            self.set_running(m, true);
        }
    }

    fn select_next_chan(&self, m: &MainCtx<'_>) -> AdcChannel {
        let next = self.chan.get(m).select_next();
        self.chan.set(m, next);
        next
    }

    fn is_running(&self, m: &MainCtx<'_>) -> bool {
        self.running.get(m)
    }

    fn set_running(&self, m: &MainCtx<'_>, running: bool) {
        self.running.set(m, running);
    }

    fn is_settled(&self, m: &MainCtx<'_>) -> bool {
        self.settled.get(m)
    }

    fn set_settled(&self, m: &MainCtx<'_>, settled: bool) {
        self.settled.set(m, settled);
    }

    fn is_enabled(&self, m: &MainCtx<'_>, chan: AdcChannel) -> bool {
        self.enabled.get(m) & chan.mask() != 0
    }

    pub fn enable(&self, m: &MainCtx<'_>, chan_mask: u8) {
        self.enabled.set(m, chan_mask);
    }

    fn set_ok(&self, m: &MainCtx<'_>, chan: AdcChannel, ok: bool) {
        if ok {
            self.ok.set(m, self.ok.get(m) | chan.mask());
        } else {
            self.ok.set(m, self.ok.get(m) & !chan.mask());
        }
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
            w.acie().set_bit() // Enable interrupt
             .aci().set_bit() // Clear interrupt flag
             .acis().on_rising_edge() // Interrupt on comparator output rising edge
             .acme().clear_bit() // No ADC mux
             .acbg().clear_bit() // no BG voltage
             .acd().clear_bit() // Enable AC
        });
        sp.AC.acsrb().write(|w| {
            w.hsel().set_bit() // Hysteresis select: on
             .hlev().set_bit() // Hysteresis level: 50 mV
             .acm().set(0) // Mux: Pos=AIN0, Neg=AIN1
        });
    }
}

#[derive(Clone)]
pub struct AcCapture {
    stamp: LargeTimestamp,
    new: bool,
}

impl AcCapture {
    const fn new() -> Self {
        Self {
            stamp: LargeTimestamp(0),
            new: false,
        }
    }

    pub fn is_new(&self) -> bool {
        self.new
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
const AC_CAPTURE_MINDIST: RelLargeTimestamp = RelLargeTimestamp::from_micros(100);

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
