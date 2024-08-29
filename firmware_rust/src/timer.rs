use crate::{
    hw::mcu,
    mutex::{CriticalSection, MutexCell},
};

#[allow(non_snake_case)]
pub struct TimerPeriph {
    pub TC1: mcu::TC1,
}

pub static TIMER_PERIPH: MutexCell<Option<TimerPeriph>> = MutexCell::new(None);

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
pub fn timer_get(cs: CriticalSection) -> u8 {
    TIMER_PERIPH.as_ref_unwrap(cs).TC1.tcnt1.read().bits()
}

// vim: ts=4 sw=4 expandtab
