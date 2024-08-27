#![no_std]
#![no_main]

use avr_device::attiny26::Peripherals;
use attiny_hal::delay::Delay;
use embedded_hal::delay::DelayNs;
use panic_halt as _;

type Fcpu = attiny_hal::clock::MHz16;

fn read_inputs(dp: &Peripherals) -> u8 {
    dp.PORTB.pinb.read().bits()
}

fn write_outputs(dp: &Peripherals) {
    dp.PORTA.porta.modify(|r, w| w.pa7().bit(!r.pa7().bit()));
    Delay::<Fcpu>::new().delay_ms(10);
}

#[avr_device::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    dp.PORTA.ddra.modify(|_, w| w.pa7().set_bit());
    loop {
//        let x = read_inputs(&dp);
        write_outputs(&dp);
    }
}

// vim: ts=4 sw=4 expandtab
