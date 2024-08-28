#![no_std]
#![no_main]

mod analog;
mod hw;
mod mutex;
mod system;

use crate::{
    hw::{ports_init, Peripherals},
    system::System,
};

static SYSTEM: System = System::new();

use panic_halt as _;

/*
fn read_inputs(dp: &Peripherals) -> u8 {
    dp.PORTB.pinb.read().bits()
}

fn write_outputs(dp: &Peripherals) {
    dp.PORTA.porta.modify(|r, w| w.pa7().bit(!r.pa7().bit()));
}
*/

#[avr_device::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    ports_init(&dp);
    SYSTEM.init(&dp);
    loop {
        SYSTEM.run(&dp);
    }
}

// vim: ts=4 sw=4 expandtab
