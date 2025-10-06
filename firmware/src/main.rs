#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(asm_experimental_arch)]

mod analog;
mod debounce;
mod debug;
mod filter;
mod fixpt;
mod history;
mod hw;
mod mains;
mod mon;
mod mutex;
mod pid;
mod pocheck;
mod ports;
mod ring;
mod shutoff;
mod speedo;
mod system;
mod temp;
mod timer;
mod triac;
mod usi_uart;

use crate::{
    debug::debug_init,
    hw::{Peripherals, interrupt, mcu},
    mutex::MainCtx,
    system::{SysPeriph, System},
    timer::timer_init,
};

static SYSTEM: System = System::new();

/// Cheaper Option::unwrap() alternative.
///
/// This is cheaper, because it doesn't call into the panic unwind path.
/// Therefore, it does not impose caller-saves overhead onto the calling function.
#[inline(always)]
pub fn unwrap_option<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => reset_system(),
    }
}

/// Cheaper Result::unwrap() alternative.
///
/// This is cheaper, because it doesn't call into the panic unwind path.
/// Therefore, it does not impose caller-saves overhead onto the calling function.
#[inline(always)]
#[allow(dead_code)]
pub fn unwrap_result<T, E>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(_) => reset_system(),
    }
}

/// Reset the system.
#[inline(always)]
#[allow(clippy::empty_loop)]
pub fn reset_system() -> ! {
    loop {
        // Wait for the watchdog timer to trigger and reset the system.
        // We don't need to disable interrupts here.
        // No interrupt will reset the watchdog timer.
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init3")]
/// Watchdog timer initialization.
///
/// # Safety
///
/// This naked function is run before main() from the .init3 section.
pub unsafe extern "C" fn wdt_init() {
    // Enable WDT with timeout 32.5 ms
    // This is a naked function, so we must return manually.
    core::arch::naked_asm!(
        "ldi r16, 0x10", // WDCE=1
        "out {WDTCR}, r16",
        "ldi r16, 0x19", // WDCE=1, WDE=1, WDP2=0, WDP1=0, WDP0=1
        "out {WDTCR}, r16",
        WDTCR = const 0x21,
    );
}

fn wdt_poke(_wp: &mcu::WDT) {
    avr_device::asm::wdr();
}

#[avr_device::entry]
fn main() -> ! {
    // SAFETY: We only call Peripherals::steal() once.
    let dp = unsafe { Peripherals::steal() };

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
        SYSTEM.run(&m, &sp);
        wdt_poke(&dp.WDT);
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    reset_system();
}

// vim: ts=4 sw=4 expandtab
