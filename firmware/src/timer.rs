use crate::{
    hw::mcu,
    mutex::{AnyCtx, LazyMainInit, MainCtx, MutexCell},
};

#[allow(non_snake_case)]
pub struct Dp {
    pub TC1: mcu::TC1,
}

// SAFETY: Is initialized when constructing the MainCtx.
pub static DP: LazyMainInit<Dp> = unsafe { LazyMainInit::uninit() };

static TIMER_UPPER: MutexCell<u8> = MutexCell::new(0);

pub const TIMER_TICK_US: u8 = 16; // 16 us per tick.

#[rustfmt::skip]
pub fn timer_init(m: &MainCtx) {
    // Timer 1 configuration:
    // CS: 256 -> 16 us per timer tick.
    DP.deref(&m).TC1.tc1h.write(|w| w);
    DP.deref(&m).TC1.tcnt1.write(|w| w);
    DP.deref(&m).TC1.tccr1a.write(|w| w);
    DP.deref(&m).TC1.tccr1c.write(|w| w);
    DP.deref(&m).TC1.tccr1d.write(|w| w);
    DP.deref(&m).TC1.tccr1e.write(|w| w);
    DP.deref(&m).TC1.ocr1a.write(|w| w.bits(0xFF));
    DP.deref(&m).TC1.ocr1b.write(|w| w.bits(0xFF));
    DP.deref(&m).TC1.ocr1c.write(|w| w.bits(0xFF)); // TOP value
    DP.deref(&m).TC1.ocr1d.write(|w| w.bits(0xFF));
    DP.deref(&m).TC1.dt1.write(|w| w);
    DP.deref(&m).TC1.tccr1b.write(|w| w.cs1().prescale_256());
}

// SAFETY: This function may only do atomic-read-only accesses, because it's
//         called from all contexts, including interrupt context.
#[inline(always)]
pub fn timer_get(a: &AnyCtx) -> Timestamp {
    // SAFETY: This function only does atomic peripheral read-only accesses.
    //         Therefore, it is safe to pretend to be the main context, even
    //         if we were actually called from irq context.
    let m = unsafe { a.to_main_ctx() };

    DP.deref(&m).TC1.tcnt1.read().bits().into()
}

#[inline(never)]
pub fn timer_get_large(m: &MainCtx) -> LargeTimestamp {
    let mut upper = TIMER_UPPER.get(m);
    let mut lower = DP.deref(m).TC1.tcnt1.read().bits();

    // Increment the upper part, if the lower part had an overflow.
    if DP.deref(m).TC1.tifr.read().tov1().bit() {
        DP.deref(m).TC1.tifr.write(|w| w.tov1().set_bit());
        lower = DP.deref(m).TC1.tcnt1.read().bits();
        upper = upper.wrapping_add(1);
        TIMER_UPPER.set(m, upper);
    }

    ((upper as u16) << 8 | lower as u16).into()
}

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
                self.0.wrapping_add(other.0 as $abstype).into()
            }
        }

        impl core::ops::Sub for $abs {
            type Output = $rel;

            #[inline]
            fn sub(self, other: Self) -> Self::Output {
                (self.0.wrapping_sub(other.0) as $reltype).into()
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

            #[inline]
            pub const fn from_micros(us: i32) -> $rel {
                $rel((us / TIMER_TICK_US as i32) as $reltype)
            }

            #[inline]
            pub const fn from_millis(ms: i32) -> $rel {
                $rel(((ms * 1000) / TIMER_TICK_US as i32) as $reltype)
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

// vim: ts=4 sw=4 expandtab
