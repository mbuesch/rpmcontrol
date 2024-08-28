#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod analog;
mod hw;
mod mutex;
mod system;

use crate::{
    analog::AC_CAPTURE,
    hw::{interrupt, ports_init, Peripherals},
    mutex::{fence, CriticalSection},
    system::System,
};
use panic_halt as _;

static SYSTEM: System = System::new();

#[avr_device::entry]
fn main() -> ! {
    // SAFETY: Everything, except for the AC_CAPTURE access,
    //         can use this central critical section.
    //         We allow interruptions of `system_cs` by `ANA_COMP` ISR.
    let system_cs = unsafe { CriticalSection::new() };
    fence();

    let dp = Peripherals::take().unwrap();

    ports_init(&dp);
    SYSTEM.init(system_cs, &dp);

    unsafe {
        interrupt::enable();
    }
    loop {
        let ac_capture = interrupt::free(|_cs| {
            // SAFETY: Interrupts are disabled,
            //         therefore it is safe to access the analog comparator
            //         interrupt data.
            unsafe { AC_CAPTURE.clone_and_reset() }
        });

        SYSTEM.run(system_cs, &dp, ac_capture);
    }
    //fence();
}

// vim: ts=4 sw=4 expandtab
