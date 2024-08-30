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
//         called from interrupt context.
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
    ($name:ident, $type:ty) => {

        #[derive(PartialEq, Eq, Copy, Clone)]
        pub struct $name(pub $type);

        impl Ord for $name {
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

        impl PartialOrd for $name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl core::ops::Sub for $name {
            type Output = $type;

            #[inline]
            fn sub(self, other: Self) -> Self::Output {
                self.0 - other.0
            }
        }

        impl From<$type> for $name {
            #[inline]
            fn from(stamp: $type) -> Self {
                $name(stamp)
            }
        }

        impl From<$name> for $type {
            #[inline]
            fn from(stamp: $name) -> Self {
                stamp.0
            }
        }
    }
}

impl_timestamp!(Timestamp, u8);
impl_timestamp!(LargeTimestamp, u16);

// vim: ts=4 sw=4 expandtab
