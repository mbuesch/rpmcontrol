#![no_std]
#![no_main]

mod analog;
mod hw;
mod mutex;
mod system;

use crate::{
    hw::{ports_init, Peripherals},
    mutex::{fence, CriticalSection},
    system::System,
};
use panic_halt as _;

static SYSTEM: System = System::new();

#[avr_device::entry]
fn main() -> ! {
    // SAFETY: We never enable interrupts.
    let cs = unsafe { CriticalSection::new() };
    fence();

    let dp = Peripherals::take().unwrap();

    ports_init(&dp);
    SYSTEM.init(cs, &dp);

    loop {
        SYSTEM.run(cs, &dp);
    }
    //fence();
}

// vim: ts=4 sw=4 expandtab
