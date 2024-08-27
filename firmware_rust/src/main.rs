#![no_std]
#![no_main]

use avr_device::attiny26::Peripherals;
use panic_halt as _;

fn read_inputs(dp: &Peripherals) -> u8 {
    dp.PORTB.pinb.read().bits()
}

fn write_outputs(dp: &Peripherals, x: u8) {
    dp.PORTB.portb.modify(|_, w| w.pb3().set_bit());
//    dp.PORTB.portb.modify(|r, w| w.bits(r.bits() | x));
}

#[avr_device::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    loop {
        let x = read_inputs(&dp);
        write_outputs(&dp, x);
    }
}

// vim: ts=4 sw=4 expandtab
