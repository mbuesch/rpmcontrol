use crate::{
    fixpt::{Fixpt, fixpt},
    hw::{Mutex, interrupt, mcu, nop3},
    mutex::{CriticalSection, IrqCtx, LazyMainInit, MainInitCtx},
    triac::triac_timer_interrupt,
};
use core::cell::Cell;

#[allow(non_snake_case)]
pub struct Dp {
    pub TC1: mcu::TC1,
}

// SAFETY: Is initialized when constructing the MainCtx.
pub static DP: LazyMainInit<Dp> = unsafe { LazyMainInit::uninit() };

static TIMER_UPPER: Mutex<Cell<u8>> = Mutex::new(Cell::new(0));

pub const TIMER_TICK_US: u8 = 16; // 16 us per tick.

impl Dp {
    #[rustfmt::skip]
    pub fn setup(&self, _c: &MainInitCtx) {
        // Timer 1 configuration:
        // CS: 256 -> 16 us per timer tick.
        DP.TC1.tc1h().write(|w| w.set(0));
        DP.TC1.tcnt1().write(|w| w.set(0));
        DP.TC1.tccr1a().write(|w| w);
        DP.TC1.tccr1c().write(|w| w);
        DP.TC1.tccr1d().write(|w| w);
        DP.TC1.tccr1e().write(|w| w);
        DP.TC1.ocr1a().write(|w| w.set(0xFF));
        DP.TC1.ocr1b().write(|w| w.set(0xFF));
        DP.TC1.ocr1c().write(|w| w.set(0xFF)); // TOP value
        DP.TC1.ocr1d().write(|w| w.set(0xFF));
        DP.TC1.dt1().write(|w| w);
        DP.TC1.tccr1b().write(|w| w.cs1().prescale_256());
    }
}

// SAFETY: This function may only do atomic-read-only accesses, because it's
//         called from all contexts, including interrupt context.
#[inline(always)]
pub fn timer_get() -> Timestamp {
    DP.TC1.tcnt1().read().bits().into()
}

#[inline(never)]
pub fn timer_get_large_cs(cs: CriticalSection<'_>) -> LargeTimestamp {
    let mut upper = TIMER_UPPER.borrow(cs).get();
    let mut lower = DP.TC1.tcnt1().read().bits();

    // Increment the upper part, if the lower part had an overflow.
    if DP.TC1.tifr().read().tov1().bit() {
        DP.TC1.tifr().write(|w| w.tov1().set_bit());
        lower = DP.TC1.tcnt1().read().bits();
        upper = upper.wrapping_add(1);
        TIMER_UPPER.borrow(cs).set(upper);
    }

    ((upper as u16) << 8 | lower as u16).into()
}

#[inline(never)]
pub fn timer_get_large() -> LargeTimestamp {
    interrupt::free(timer_get_large_cs)
}

// Wait for register write to synchronize to timer hardware.
#[inline(always)]
fn timer_sync_wait() {
    nop3();
}

macro_rules! define_timer_interrupt {
    ($arm_fn:ident, $cancel_fn:ident, $irq_fn:ident, $handler_fn:path, $ocr:ident, $ocie:ident, $ocf:ident) => {
        pub fn $arm_fn(trigger_time: Timestamp) {
            interrupt::free(|_| {
                // Ensure it doesn't trigger right away by pushing OCR into the future.
                let now_ticks: u8 = timer_get().into();
                DP.TC1.tc1h().write(|w| w.set(0));
                DP.TC1.$ocr().write(|w| w.set(now_ticks.wrapping_add(0x7F)));
                timer_sync_wait();

                // Clear trigger flag and enable interrupt.
                DP.TC1.tifr().write(|w| w.$ocf().set_bit());
                DP.TC1.timsk().modify(|_, w| w.$ocie().set_bit());

                // Program the compare register.
                DP.TC1.tc1h().write(|w| w.set(0));
                DP.TC1.$ocr().write(|w| w.set(trigger_time.into()));
                timer_sync_wait();
                let now = timer_get();

                // Trigger is in the past and has not triggered, yet?
                if trigger_time <= now && !DP.TC1.tifr().read().$ocf().bit() {
                    loop {
                        // Enforce trigger now.
                        let trigger_time = timer_get() + RelTimestamp::from_ticks(1);
                        DP.TC1.tc1h().write(|w| w.set(0));
                        DP.TC1.$ocr().write(|w| w.set(trigger_time.into()));
                        timer_sync_wait();
                        let now = timer_get();

                        /* Is it going to trigger or did it trigger already? */
                        if trigger_time > now || DP.TC1.tifr().read().$ocf().bit() {
                            break; /* Done. IRQ is pending. */
                        }
                    }
                }
            });
        }

        pub fn $cancel_fn() {
            interrupt::free(|_| {
                DP.TC1.timsk().modify(|_, w| w.$ocie().clear_bit());
                DP.TC1.tifr().write(|w| w.$ocf().set_bit());
            });
        }

        pub fn $irq_fn(c: &IrqCtx<'_>) {
            DP.TC1.timsk().modify(|_, w| w.$ocie().clear_bit());
            DP.TC1.tifr().write(|w| w.$ocf().set_bit());
            let trig_time: Timestamp = DP.TC1.$ocr().read().bits().into();
            $handler_fn(c, trig_time);
        }
    };
}

define_timer_interrupt!(
    timer_interrupt_a_arm,
    timer_interrupt_a_cancel,
    irq_handler_timer1_compa,
    triac_timer_interrupt,
    ocr1a,
    ocie1a,
    ocf1a
);

macro_rules! impl_timestamp {
    ($rel:ident, $abs:ident, $reltype:ty, $abstype:ty) => {
        #[derive(PartialEq, Eq, Copy, Clone)]
        pub struct $abs(pub $abstype);

        impl $abs {
            #[inline]
            pub const fn new() -> Self {
                $abs(0)
            }

            #[inline]
            pub const fn from_ticks(ticks: $abstype) -> Self {
                $abs(ticks)
            }

            #[inline]
            pub const fn from_micros(us: u32) -> $abs {
                $abs((us / TIMER_TICK_US as u32) as $abstype)
            }

            #[inline]
            pub const fn from_millis(ms: u32) -> $abs {
                $abs(((ms * 1000) / TIMER_TICK_US as u32) as $abstype)
            }

            #[inline]
            pub const fn add(self, other: $rel) -> $abs {
                $abs(self.0.wrapping_add(other.0 as $abstype))
            }

            #[inline]
            pub const fn sub(self, other: $abs) -> $rel {
                $rel(self.0.wrapping_sub(other.0) as $reltype)
            }

            #[inline]
            pub const fn sub_rel(self, other: $rel) -> $abs {
                $abs(self.0.wrapping_sub(other.0 as $abstype))
            }
        }

        impl Default for $abs {
            #[inline]
            fn default() -> Self {
                Self::new()
            }
        }

        impl Ord for $abs {
            #[inline]
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                if self.0 == other.0 {
                    core::cmp::Ordering::Equal
                } else if self.0.wrapping_sub(other.0) & (1 << (<$abstype>::BITS - 1)) == 0 {
                    core::cmp::Ordering::Greater
                } else {
                    core::cmp::Ordering::Less
                }
            }
        }

        impl PartialOrd for $abs {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl core::ops::Add<$rel> for $abs {
            type Output = Self;

            #[inline]
            fn add(self, other: $rel) -> Self::Output {
                Self::add(self, other)
            }
        }

        impl core::ops::Sub for $abs {
            type Output = $rel;

            #[inline]
            fn sub(self, other: Self) -> Self::Output {
                Self::sub(self, other)
            }
        }

        impl core::ops::Sub<$rel> for $abs {
            type Output = $abs;

            #[inline]
            fn sub(self, other: $rel) -> Self::Output {
                Self::sub_rel(self, other)
            }
        }

        impl From<$abstype> for $abs {
            #[inline]
            fn from(stamp: $abstype) -> Self {
                $abs(stamp)
            }
        }

        impl From<$abs> for $abstype {
            #[inline]
            fn from(stamp: $abs) -> Self {
                stamp.0
            }
        }
    };
}

macro_rules! impl_reltimestamp {
    ($rel:ident, $abs:ident, $reltype:ty, $abstype:ty) => {
        #[derive(PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
        pub struct $rel(pub $reltype);

        impl $rel {
            #[inline]
            pub const fn new() -> Self {
                $rel(0)
            }

            #[inline]
            pub const fn from_ticks(ticks: $reltype) -> Self {
                $rel(ticks)
            }

            #[inline(always)]
            pub const fn from_micros(us: i32) -> $rel {
                $rel((us / TIMER_TICK_US as i32) as $reltype)
            }

            #[inline(always)]
            pub const fn from_millis(ms: i32) -> $rel {
                $rel(((ms * 1000) / TIMER_TICK_US as i32) as $reltype)
            }

            #[inline]
            pub const fn add(self, other: $rel) -> $rel {
                $rel(self.0.wrapping_add(other.0))
            }

            #[inline]
            pub const fn sub(self, other: $rel) -> $rel {
                $rel(self.0.wrapping_sub(other.0))
            }

            #[inline]
            pub const fn div(&self, d: $reltype) -> $rel {
                $rel(self.0 / d)
            }
        }

        impl Default for $rel {
            #[inline]
            fn default() -> Self {
                Self::new()
            }
        }

        impl core::ops::Add<$rel> for $rel {
            type Output = Self;

            #[inline]
            fn add(self, other: $rel) -> Self::Output {
                Self::add(self, other)
            }
        }

        impl core::ops::Sub for $rel {
            type Output = $rel;

            #[inline]
            fn sub(self, other: Self) -> Self::Output {
                Self::sub(self, other)
            }
        }

        impl From<$reltype> for $rel {
            #[inline]
            fn from(relstamp: $reltype) -> Self {
                $rel(relstamp)
            }
        }

        impl From<$rel> for $reltype {
            #[inline]
            fn from(relstamp: $rel) -> Self {
                relstamp.0
            }
        }
    };
}

impl_timestamp!(RelTimestamp, Timestamp, i8, u8);
impl_timestamp!(RelLargeTimestamp, LargeTimestamp, i16, u16);

impl_reltimestamp!(RelTimestamp, Timestamp, i8, u8);
impl_reltimestamp!(RelLargeTimestamp, LargeTimestamp, i16, u16);

impl From<LargeTimestamp> for Timestamp {
    #[inline]
    fn from(stamp: LargeTimestamp) -> Timestamp {
        (stamp.0 as u8).into()
    }
}

impl RelLargeTimestamp {
    pub fn from_ms_fixpt(ms: Fixpt) -> RelLargeTimestamp {
        // We must convert `ms` milliseconds to a corresponding number of ticks.
        //
        // Basically, we want to do:
        //  let ticks = (ms * 1000) / TIMER_TICK_US;
        //
        // But we must avoid overflows and minimize rounding errors.
        //
        // assumptions:
        //  1000 / TIMER_TICK_US = 62.5
        //  We use a bias of 32 = 1 << 5.
        //
        // Therefore, we calculate:
        //
        //         ms * 62.5 * 32
        // ticks = --------------
        //              32
        //
        // But we split it up into a Fixpt calculation and the final bias shift.
        //
        // Fixpt calculation:
        //
        //         ms * 62.5
        // ticks = ---------
        //              32

        // The microseconds per tick value is embedded in the constants below.
        // See comment above.
        assert_eq!(TIMER_TICK_US, 16);

        // First part: Fixpt multiplication with bias.
        let fac = fixpt!(125 / 64); // 62.5 / 32
        let scaled = ms * fac;

        // Second part: Bias division.
        // Get the raw fixpt value and shift by 5.
        let ticks = scaled.to_q() >> (Fixpt::SHIFT - 5);

        Self::from_ticks(ticks)
    }
}

// vim: ts=4 sw=4 expandtab
