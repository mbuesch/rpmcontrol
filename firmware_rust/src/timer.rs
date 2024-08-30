use crate::{
    hw::mcu,
    mutex::{CriticalSection, MutexCell},
};

#[allow(non_snake_case)]
pub struct TimerPeriph {
    pub TC1: mcu::TC1,
}

pub static TIMER_PERIPH: MutexCell<Option<TimerPeriph>> = MutexCell::new(None);
static TIMER_UPPER: MutexCell<u8> = MutexCell::new(0);

pub const TIMER_TICK_US: u8 = 8; // 8 us per tick.

pub fn timer_init(tp: &TimerPeriph) {
    // Timer 1 configuration:
    // CS: 128 -> 8 us per timer tick.
    // No compare match.
    // No pin toggle.
    tp.TC1.tcnt1.write(|w| w); // = 0
    tp.TC1.tccr1a.write(|w| w); // = 0
    tp.TC1.tccr1b.write(|w| w.cs1().running_clk_128());
}

// SAFETY: This function may only do atomic-read-only accesses, because it's
//         called from interrupt context.
pub fn timer_get(cs: CriticalSection) -> Timestamp {
    TIMER_PERIPH
        .as_ref_unwrap(cs)
        .TC1
        .tcnt1
        .read()
        .bits()
        .into()
}

pub fn timer_get_large(cs: CriticalSection) -> u16 {
    let tp = TIMER_PERIPH.as_ref_unwrap(cs);

    let mut upper = TIMER_UPPER.get(cs);
    let mut lower = tp.TC1.tcnt1.read().bits();

    // Increment the upper part, if the lower part had an overflow.
    if tp.TC1.tifr.read().tov1().bit() {
        tp.TC1.tifr.write(|w| w.tov1().set_bit());
        lower = tp.TC1.tcnt1.read().bits();
        upper = upper.wrapping_add(1);
        TIMER_UPPER.set(cs, upper);
    }

    (upper as u16) << 8 | lower as u16
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Timestamp(pub u8);

impl Ord for Timestamp {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.0 == other.0 {
            core::cmp::Ordering::Equal
        } else if self.0.wrapping_sub(other.0) & 0x80 == 0 {
            core::cmp::Ordering::Greater
        } else {
            core::cmp::Ordering::Less
        }
    }
}

impl PartialOrd for Timestamp {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl core::ops::Sub for Timestamp {
    type Output = u8;

    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        self.0 - other.0
    }
}

impl From<u8> for Timestamp {
    #[inline]
    fn from(stamp: u8) -> Self {
        Timestamp(stamp)
    }
}

impl From<Timestamp> for u8 {
    #[inline]
    fn from(stamp: Timestamp) -> Self {
        stamp.0
    }
}

// vim: ts=4 sw=4 expandtab
