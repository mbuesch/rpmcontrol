pub use avr_device::{
    attiny26::{self as mcu, Peripherals},
    interrupt::{self, Mutex},
};

pub fn ports_init(dp: &Peripherals) {
    fn pin_input(_bit: usize) -> u8 {
        0
    }
    fn pin_output(bit: usize) -> u8 {
        1 << bit
    }
    fn pin_low(_bit: usize) -> u8 {
        0
    }
    fn pin_high(bit: usize) -> u8 {
        1 << bit
    }
    fn pin_floating(_bit: usize) -> u8 {
        0
    }

    // PORTA
    dp.PORTA.porta.write(|w| {
        w.bits(
            pin_floating(0) | // setpoint, single ended ADC
            pin_floating(1) | // vsense, single ended ADC
            pin_low(2) | // DNC
            pin_floating(3) | // AREF
            pin_floating(4) | // shunt_lo, differential ADC
            pin_floating(5) | // shunt_hi, differential ADC + single ended ADC
            pin_floating(6) | // speedo, AD comparator pos
            pin_floating(7), // speedoref, AD comparator neg
        )
    });
    dp.PORTA.ddra.write(|w| {
        w.bits(
            pin_input(0) | // setpoint, single ended ADC
            pin_input(1) | // vsense, single ended ADC
            pin_output(2) | // DNC
            pin_input(3) | // AREF
            pin_input(4) | // shunt_lo, differential ADC
            pin_input(5) | // shunt_hi, differential ADC + single ended ADC
            pin_input(6) | // speedo, AD comparator pos
            pin_input(7), // speedoref, AD comparator neg
        )
    });

    // PORTB
    dp.PORTB.portb.write(|w| {
        w.bits(
            pin_low(0) | // ISP MOSI
            pin_low(1) | // ISP MISO
            pin_low(2) | // ISP SCK
            pin_high(3) | // trig, active low
            pin_floating(4) | // XTAL1
            pin_floating(5) | // XTAL2
            pin_low(6) | // Debug
            pin_floating(7), // RESET, active low
        )
    });
    dp.PORTB.ddrb.write(|w| {
        w.bits(
            pin_output(0) | // ISP MOSI
            pin_output(1) | // ISP MISO
            pin_output(2) | // ISP SCK
            pin_output(3) | // trig, active low
            pin_input(4) | // XTAL1
            pin_input(5) | // XTAL2
            pin_output(6) | // Debug
            pin_input(7), // RESET, active low
        )
    });
}

// vim: ts=4 sw=4 expandtab
