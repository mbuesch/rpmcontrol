use crate::raw::Int24Raw;
use core::arch::asm;

#[inline(always)]
pub fn asm_mulsat24(a: Int24Raw, mut b: Int24Raw) -> Int24Raw {
    unsafe {
        asm!(
            // any operand is zero?
            "   cp {a0}, __zero_reg__",
            "   cpc {a1}, __zero_reg__",
            "   cpc {a2}, __zero_reg__",
            "   breq 80f",
            "   cp {b0}, __zero_reg__",
            "   cpc {b1}, __zero_reg__",
            "   cpc {b2}, __zero_reg__",
            "   breq 80f",

            // handle multiplicand == MIN
            "   cp {a0}, __zero_reg__",
            "   cpc {a1}, __zero_reg__",
            "   ldi {t}, 0x80",
            "   cpc {a2}, {t}",
            "   brne 10f",
            "   sbrs {b2}, 7",          // multiplier is negative?
            "   rjmp 60f",              // saturate to neg min
            "   rjmp 70f",              // saturate to pos max
            "10:",

            // multiplication logic

            "   ldi {t}, 24",           // loop counter
            "   sub {p3}, {p3}",        // clear upper product and carry
            "   sub {p4}, {p4}",
            "   sub {p5}, {p5}",

            "1: brcc 2f",
            "   add {p3}, {a0}",
            "   adc {p4}, {a1}",
            "   adc {p5}, {a2}",

            "2: sbrs {b0}, 0",
            "   rjmp 3f",
            "   sub {p3}, {a0}",
            "   sbc {p4}, {a1}",
            "   sbc {p5}, {a2}",

            "3: asr {p5}",
            "   ror {p4}",
            "   ror {p3}",
            "   ror {b2}",
            "   ror {b1}",
            "   ror {b0}",

            "   dec {t}",
            "   brne 1b",               // loop counter != 0?

            // check if saturation is needed
            "   cp {p3}, __zero_reg__", // product high all bits cleared?
            "   cpc {p4}, __zero_reg__",
            "   cpc {p5}, __zero_reg__",
            "   breq 90f",
            "   ldi {t}, 0xFF",         // product high all bits set?
            "   cp {p3}, {t}",
            "   cpc {p4}, {t}",
            "   cpc {p5}, {t}",
            "   breq 90f",
            "   sbrs {p5}, 7",          // saturate pos or neg
            "   rjmp 70f",

            // saturate to negative min
            "60:",
            "   mov {b0}, __zero_reg__",
            "   mov {b1}, __zero_reg__",
            "   ldi {b2}, 0x80",
            "   rjmp 90f",

            // saturate to positive max
            "70:",
            "   ldi {b1}, 0xFF",
            "   mov {b0}, {b1}",
            "   ldi {b2}, 0x7F",
            "   rjmp 90f",

            // zero result
            "80:",
            "   clr {b0}",
            "   clr {b1}",
            "   clr {b2}",

            "90:",

            a0 = in(reg) a.0,           // multiplicand
            a1 = in(reg) a.1,
            a2 = in(reg) a.2,

            b0 = inout(reg) b.0,        // multiplier and product low
            b1 = inout(reg_upper) b.1,
            b2 = inout(reg_upper) b.2,
            p3 = out(reg) _,            // product high
            p4 = out(reg) _,
            p5 = out(reg) _,

            t = out(reg_upper) _,

            options(pure, nomem, nostack),
        );
    }
    b
}

#[inline(always)]
#[allow(unused_assignments)]
pub fn asm_divsat24(mut a: Int24Raw, mut b: Int24Raw) -> Int24Raw {
    unsafe {
        asm!(
            // check division by zero
            "   cp {b0}, __zero_reg__",
            "   cpc {b1}, __zero_reg__",
            "   cpc {b2}, __zero_reg__",
            "   brne 10f",
            "   rjmp 50f",
            "10:",

            // saturate MIN/-1
            "   ldi {t}, 0xFF",
            "   cp {b0}, {t}",
            "   cpc {b1}, {t}",
            "   cpc {b2}, {t}",
            "   cpc {a0}, __zero_reg__",
            "   cpc {a1}, __zero_reg__",
            "   ldi {t}, 0x80",
            "   cpc {a2}, {t}",
            "   breq 70f",

            // store the result sign in SREG.T
            "   clt",
            "   mov {t}, {a2}",
            "   eor {t}, {b2}",
            "   sbrc {t}, 7",
            "   set",

            // a = abs(a)
            "   sbrs {a2}, 7",
            "   rjmp 20f",
            "   com {a2}",              // negate
            "   com {a1}",
            "   neg {a0}",
            "   sbci {a1}, 0xFF",
            "   sbci {a2}, 0xFF",
            "   sbrs {a2}, 7",
            "   rjmp 20f",
            "   ldi {a1}, 0xFF",        // saturate to max
            "   mov {a0}, {a1}",
            "   ldi {a2}, 0x7F",
            "20:",

            // b = abs(b)
            "   sbrs {b2}, 7",
            "   rjmp 30f",
            "   com {b2}",              // negate
            "   com {b1}",
            "   neg {b0}",
            "   sbci {b1}, 0xFF",
            "   sbci {b2}, 0xFF",
            "   sbrs {b2}, 7",
            "   rjmp 30f",
            "   ldi {b1}, 0xFF",        // saturate to max
            "   mov {b0}, {b1}",
            "   ldi {b2}, 0x7F",
            "30:",

            // division logic

            "   ldi {t}, 25",           // loop counter

            "   sub {rem0}, {rem0}",    // remainder = 0 and carry = 0
            "   sub {rem1}, {rem1}",
            "   sub {rem2}, {rem2}",

            "40:",
            "   rol {a0}",              // (dividend << 1) + carry
            "   rol {a1}",
            "   rol {a2}",

            "   dec {t}",
            "   breq 80f",              // loop counter == 0?

            "   rol {rem0}",            // (remainder << 1) + dividend.23
            "   rol {rem1}",
            "   rol {rem2}",

            "   sub {rem0}, {b0}",      // remainder -= divisor
            "   sbc {rem1}, {b1}",
            "   sbc {rem2}, {b2}",
            "   brcs 41f",              // remainder was less than divisor?
            "   sec",                   // result lsb = 1
            "   rjmp 40b",
            "41:",
            "   add {rem0}, {b0}",
            "   adc {rem1}, {b1}",
            "   adc {rem2}, {b2}",
            "   clc",                   // result lsb = 0
            "   rjmp 40b",

            // handle division by zero
            "50:",
            "   sbrs {a2}, 7",
            "   rjmp 70f",

            // saturate to negative min
            "60:",
            "   mov {a0}, __zero_reg__",
            "   mov {a1}, __zero_reg__",
            "   ldi {a2}, 0x80",
            "   rjmp 90f",

            // saturate to positive max
            "70:",
            "   ldi {a1}, 0xFF",
            "   mov {a0}, {a1}",
            "   ldi {a2}, 0x7F",
            "   rjmp 90f",

            // adjust the result sign according to SREG.T
            "80:",
            "   brtc 90f",
            "   com {a2}",              // negate
            "   com {a1}",
            "   neg {a0}",
            "   sbci {a1}, 0xFF",
            "   sbci {a2}, 0xFF",

            "90:",

            rem0 = out(reg) _,          // remainder
            rem1 = out(reg) _,
            rem2 = out(reg) _,

            b0 = inout(reg) b.0,        // divisor
            b1 = inout(reg_upper) b.1,
            b2 = inout(reg_upper) b.2,

            a0 = inout(reg) a.0,        // dividend and quotient
            a1 = inout(reg_upper) a.1,
            a2 = inout(reg_upper) a.2,

            t = out(reg_upper) _,       // temporary and loop counter

            options(pure, nomem, nostack),
        );
    }
    a
}

#[inline(always)]
pub fn asm_negsat24(mut a: Int24Raw) -> Int24Raw {
    unsafe {
        asm!(
            "   mov {t}, {a2}",

            "   com {a2}",
            "   com {a1}",
            "   neg {a0}",
            "   sbci {a1}, 0xFF",
            "   sbci {a2}, 0xFF",

            "   and {t}, {a2}",
            "   sbrs {t}, 7",
            "   rjmp 1f",
            "   ldi {a1}, 0xFF",
            "   mov {a0}, {a1}",
            "   ldi {a2}, 0x7F",
            "1:",

            a0 = inout(reg) a.0,
            a1 = inout(reg_upper) a.1,
            a2 = inout(reg_upper) a.2,

            t = out(reg) _,

            options(pure, nomem, nostack),
        );
    }
    a
}

#[inline(always)]
#[allow(unused_assignments)]
pub fn asm_shl24(mut a: Int24Raw, mut count: u8) -> Int24Raw {
    unsafe {
        asm!(
            "   and {count}, {count}",
            "   breq 2f",
            "1: lsl {a0}",
            "   rol {a1}",
            "   rol {a2}",
            "   dec {count}",
            "   brne 1b",
            "2:",

            a0 = inout(reg) a.0,
            a1 = inout(reg) a.1,
            a2 = inout(reg) a.2,
            count = inout(reg) count,

            options(pure, nomem, nostack),
        );
    }
    a
}

#[inline(always)]
#[allow(unused_assignments)]
pub fn asm_shr24(mut a: Int24Raw, mut count: u8) -> Int24Raw {
    unsafe {
        asm!(
            "   and {count}, {count}",
            "   breq 2f",
            "1: asr {a2}",
            "   ror {a1}",
            "   ror {a0}",
            "   dec {count}",
            "   brne 1b",
            "2:",

            a0 = inout(reg) a.0,
            a1 = inout(reg) a.1,
            a2 = inout(reg) a.2,
            count = inout(reg) count,

            options(pure, nomem, nostack),
        );
    }
    a
}

#[inline(always)]
pub fn asm_ge24(a: Int24Raw, b: Int24Raw) -> bool {
    let mut c: u8;
    unsafe {
        asm!(
            "   cp {a0}, {b0}",
            "   cpc {a1}, {b1}",
            "   cpc {a2}, {b2}",
            "   in {c}, __SREG__",
            "   andi {c}, 0x10",

            a0 = in(reg) a.0,
            a1 = in(reg) a.1,
            a2 = in(reg) a.2,

            b0 = in(reg) b.0,
            b1 = in(reg) b.1,
            b2 = in(reg) b.2,

            c = out(reg_upper) c,

            options(pure, nomem, nostack),
        );
    }
    c == 0
}

// vim: ts=4 sw=4 expandtab
