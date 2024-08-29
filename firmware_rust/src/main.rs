#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod analog;
mod hw;
mod mutex;
mod system;
mod timer;

use crate::{
    analog::AC_CAPTURE,
    hw::{interrupt, mcu, ports_init, Peripherals},
    mutex::{fence, unwrap_option, CriticalSection},
    system::{SysPeriph, System},
    timer::{timer_init, TimerPeriph, TIMER_PERIPH},
};
use panic_halt as _;

static SYSTEM: System = System::new();

fn wdt_init(wp: &mcu::WDT) {
    //TODO
}

fn wdt_poke(_wp: &mcu::WDT) {
    avr_device::asm::wdr();
}

#[avr_device::entry]
fn main() -> ! {
    // SAFETY: Everything, except for the AC_CAPTURE access,
    //         can use this central critical section.
    //         We allow interruptions of `system_cs` by `ANA_COMP` ISR.
    let system_cs = unsafe { CriticalSection::new() };
    fence();

    let dp = unwrap_option(Peripherals::take());

    wdt_init(&dp.WDT);
    ports_init(&dp);

    let sp = SysPeriph {
        AC: dp.AC,
        ADC: dp.ADC,
        PORTA: dp.PORTA,
        PORTB: dp.PORTB,
    };
    let tp = TimerPeriph { TC1: dp.TC1 };

    timer_init(&tp);
    TIMER_PERIPH.replace(system_cs, Some(tp));
    SYSTEM.init(system_cs, &sp);

    unsafe { interrupt::enable() };
    loop {
        let ac_capture = interrupt::free(|_cs| {
            // SAFETY: Interrupts are disabled.
            //         Therefore, it is safe to access the analog comparator
            //         interrupt data.
            unsafe { AC_CAPTURE.clone_and_reset() }
        });

        SYSTEM.run(system_cs, &sp, ac_capture);
        wdt_poke(&dp.WDT);
    }
    //fence();
}

// vim: ts=4 sw=4 expandtab
