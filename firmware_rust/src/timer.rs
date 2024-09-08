use crate::{
    hw::mcu,
    mutex::{CriticalSection, MutexCell},
};

#[allow(non_snake_case)]
pub struct TimerPeriph {
    pub TC0: mcu::TC0,
}

pub static TIMER_PERIPH: MutexCell<Option<TimerPeriph>> = MutexCell::new(None);
static TIMER_UPPER: MutexCell<u8> = MutexCell::new(0);

pub const TIMER_TICK_US: u8 = 16; // 16 us per tick.

pub fn timer_init(tp: &TimerPeriph) {
    // Timer 0 configuration:
    // CS: 256 -> 16 us per timer tick.
    tp.TC0.tccr0.write(|w| w.cs0().running_clk_256());
}

// SAFETY: This function may only do atomic-read-only accesses, because it's
//         called from all contexts, including interrupt context.
#[rustfmt::skip]
pub fn timer_get(cs: CriticalSection) -> Timestamp {
    TIMER_PERIPH.as_ref_unwrap(cs).TC0.tcnt0.read().bits().into()
}

pub fn timer_get_large(cs: CriticalSection) -> LargeTimestamp {
    let tp = TIMER_PERIPH.as_ref_unwrap(cs);

    let mut upper = TIMER_UPPER.get(cs);
    let mut lower = tp.TC0.tcnt0.read().bits();

    // Increment the upper part, if the lower part had an overflow.
    if tp.TC0.tifr.read().tov0().bit() {
        tp.TC0.tifr.write(|w| w.tov0().set_bit());
        lower = tp.TC0.tcnt0.read().bits();
        upper = upper.wrapping_add(1);
        TIMER_UPPER.set(cs, upper);
    }

    ((upper as u16) << 8 | lower as u16).into()
}

macro_rules! impl_timestamp {
    ($rel:ident, $abs:ident, $type:ty) => {
        #[derive(PartialEq, Eq, Copy, Clone)]
        pub struct $abs(pub $type);

        impl $abs {
            #[inline]
            pub const fn new() -> Self {
                $abs(0)
            }

            #[inline]
            pub const fn from_ticks(ticks: $type) -> Self {
                $abs(ticks)
            }

            #[inline]
            pub const fn from_micros(us: u32) -> $abs {
                $abs((us / TIMER_TICK_US as u32) as $type)
            }

            #[inline]
            pub const fn from_millis(ms: u32) -> $abs {
                $abs(((ms * 1000) / TIMER_TICK_US as u32) as $type)
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
                } else if self.0.wrapping_sub(other.0) & (1 << (<$type>::BITS - 1)) == 0 {
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
                self.0.wrapping_add(other.0 as $type).into()
            }
        }

        impl core::ops::Sub for $abs {
            type Output = $type;

            #[inline]
            fn sub(self, other: Self) -> Self::Output {
                self.0.wrapping_sub(other.0)
            }
        }

        impl From<$type> for $abs {
            #[inline]
            fn from(stamp: $type) -> Self {
                $abs(stamp)
            }
        }

        impl From<$abs> for $type {
            #[inline]
            fn from(stamp: $abs) -> Self {
                stamp.0
            }
        }
    };
}

macro_rules! impl_reltimestamp {
    ($rel:ident, $abs:ident, $type:ty) => {
        #[derive(PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
        pub struct $rel(pub $type);

        impl $rel {
            #[inline]
            pub const fn new() -> Self {
                $rel(0)
            }

            #[inline]
            pub const fn from_ticks(ticks: $type) -> Self {
                $rel(ticks)
            }

            #[inline]
            pub const fn from_micros(us: i32) -> $rel {
                $rel((us / TIMER_TICK_US as i32) as $type)
            }

            #[inline]
            pub const fn from_millis(ms: i32) -> $rel {
                $rel(((ms * 1000) / TIMER_TICK_US as i32) as $type)
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
                self.0.wrapping_add(other.0).into()
            }
        }

        impl core::ops::Sub for $rel {
            type Output = $rel;

            #[inline]
            fn sub(self, other: Self) -> Self::Output {
                self.0.wrapping_sub(other.0).into()
            }
        }

        impl From<$type> for $rel {
            #[inline]
            fn from(relstamp: $type) -> Self {
                $rel(relstamp)
            }
        }

        impl From<$rel> for $type {
            #[inline]
            fn from(relstamp: $rel) -> Self {
                relstamp.0
            }
        }
    };
}

impl_timestamp!(RelTimestamp, Timestamp, u8);
impl_timestamp!(RelLargeTimestamp, LargeTimestamp, u16);

impl_reltimestamp!(RelTimestamp, Timestamp, i8);
impl_reltimestamp!(RelLargeTimestamp, LargeTimestamp, i16);

impl From<LargeTimestamp> for Timestamp {
    #[inline]
    fn from(stamp: LargeTimestamp) -> Timestamp {
        (stamp.0 as u8).into()
    }
}

// vim: ts=4 sw=4 expandtab
