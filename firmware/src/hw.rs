pub use attiny::{self as mcu, Peripherals};
pub use avr_device::attiny861a as attiny;
pub use avr_device::interrupt::{self, Mutex};

use crate::mutex::IrqCtx;

macro_rules! define_isr {
    ($name:ident, $handler:path) => {
        #[avr_device::interrupt(attiny861a)]
        fn $name() {
            // SAFETY: We are inside of an interrupt handler.
            // Therefore, it is safe to construct an `IrqCtx`.
            let c = unsafe { IrqCtx::new() };
            $handler(&c);
        }
    };
}

define_isr!(PCINT, crate::usi_uart::irq_handler_pcint);
define_isr!(TIMER1_COMPA, crate::timer::irq_handler_timer1_compa);
define_isr!(TIMER1_COMPB, crate::timer::irq_handler_timer1_compb);
define_isr!(USI_OVF, crate::usi_uart::irq_handler_usi_ovf);
define_isr!(ANA_COMP, crate::analog::irq_handler_ana_comp);

/// Do nothing for the duration of 3 CPU cycles.
#[inline(always)]
#[rustfmt::skip]
pub fn nop3() {
    unsafe {
        core::arch::asm!(
            "rjmp 1",
            "1: nop",
            options(preserves_flags)
        )
    }
}

// vim: ts=4 sw=4 expandtab
