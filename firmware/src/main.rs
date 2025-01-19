#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]
#![feature(asm_const)]

mod analog;
mod debug;
mod fixpt;
mod hw;
mod mains;
mod mutex;
mod pi;
mod ports;
mod speedo;
mod system;
mod timer;
mod triac;
mod usi_uart;

use crate::{
    analog::ac_capture_get,
    hw::{interrupt, mcu, Peripherals},
    mutex::{unwrap_option, MainCtx},
    system::{SysPeriph, System},
    timer::timer_init,
    debug::debug_init,
};

static SYSTEM: System = System::new();

fn wdt_init() {
    // SAFETY: The asm code only accesses the WDT registers
    //         which are not accessed from anywhere else in the program.
    unsafe {
        // Enable WDT with timeout 32.5 ms
        core::arch::asm!(
            "ldi {tmp}, 0x10", // WDCE=1
            "out {WDTCR}, {tmp}",
            "ldi {tmp}, 0x19", // WDCE=1, WDE=1, WDP2=0, WDP1=0, WDP0=1
            "out {WDTCR}, {tmp}",
            tmp = out(reg_upper) _,
            WDTCR = const 0x21,
            options(nostack, preserves_flags)
        );
    }
}

fn wdt_poke(_wp: &mcu::WDT) {
    avr_device::asm::wdr();
}

#[avr_device::entry]
fn main() -> ! {
    wdt_init();

    let dp = unwrap_option(Peripherals::take());

    let sp = SysPeriph {
        AC: dp.AC,
        ADC: dp.ADC,
    };

    let porta_dp = ports::PortA { PORTA: dp.PORTA };
    let portb_dp = ports::PortB { PORTB: dp.PORTB };
    let timer_dp = timer::Dp { TC1: dp.TC1 };
    let usi_dp = usi_uart::Dp {
        USI: dp.USI,
        TC0: dp.TC0,
    };

    let init_static_vars = |ctx| {
        let porta = ports::PORTA.init(ctx, porta_dp);
        let portb = ports::PORTB.init(ctx, portb_dp);
        timer::DP.init(ctx, timer_dp);
        let usi_uart = usi_uart::DP.init(ctx, usi_dp);

        porta.setup(ctx);
        portb.setup(ctx);
        usi_uart.setup(ctx);
        debug_init(ctx);
    };

    // SAFETY:
    // This is the context handle for the main() function.
    // Holding a reference to this object proves that the holder
    // is running in main() context.
    let m = unsafe { MainCtx::new_with_init(init_static_vars) };

    timer_init(&m);
    SYSTEM.init(&m, &sp);

    // SAFETY: This must be after construction of MainCtx
    //         and after initialization of static MainInit variables.
    unsafe { interrupt::enable() };

    loop {
        let ac_capture = ac_capture_get();
        SYSTEM.run(&m, &sp, ac_capture);
        wdt_poke(&dp.WDT);
    }
}

// vim: ts=4 sw=4 expandtab
