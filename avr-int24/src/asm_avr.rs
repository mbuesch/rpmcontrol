use crate::raw::{Int24Raw, abs24, is_neg24};
use core::arch::asm;

#[inline(always)]
pub fn asm_mul24(mut a: Int24Raw, b: Int24Raw) -> Int24Raw {
    unsafe {
        asm!(
            "mov {r0_save}, r0",
            "rcall __mulpsi3",
            "mov r0, {r0_save}",

            inout("r22") a.0,
            inout("r23") a.1,
            inout("r24") a.2,

            in("r18") b.0,
            in("r19") b.1,
            in("r20") b.2,

            out("r21") _, // clobbered by __mulpsi3
            r0_save = out(reg) _, // clobbered by __mulpsi3

            options(pure, nomem),
        );
    }
    a
}

#[inline(always)]
pub fn asm_div24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    let res_neg = is_neg24(a) ^ is_neg24(b);
    let mut a = abs24(a);
    let b = abs24(b);

    unsafe {
        asm!(
            "   ldi {loop}, 25",        // loop counter

            "   sub {rem0}, {rem0}",    // remainder = 0 and carry = 0
            "   sub {rem1}, {rem1}",
            "   sub {rem2}, {rem2}",

            "1: rol {a0}",              // (dividend << 1) + carry
            "   rol {a1}",
            "   rol {a2}",

            "   dec {loop}",
            "   breq 3f",               // loop counter == 0?

            "   rol {rem0}",            // (remainder << 1) + dividend.23
            "   rol {rem1}",
            "   rol {rem2}",

            "   sub {rem0}, {b0}",      // remainder -= divisor
            "   sbc {rem1}, {b1}",
            "   sbc {rem2}, {b2}",
            "   brcs 2f",               // remainder was less than divisor?
            "   sec",                   // result lsb = 1
            "   rjmp 1b",
            "2: add {rem0}, {b0}",
            "   adc {rem1}, {b1}",
            "   adc {rem2}, {b2}",
            "   clc",                   // result lsb = 0
            "   rjmp 1b",

            "3:",

            rem0 = out(reg) _,          // remainder
            rem1 = out(reg) _,
            rem2 = out(reg) _,

            b0 = in(reg) b.0,           // divisor
            b1 = in(reg) b.1,
            b2 = in(reg) b.2,

            a0 = inout(reg) a.0,        // dividend and quotient
            a1 = inout(reg) a.1,
            a2 = inout(reg) a.2,

            loop = out(reg_upper) _,    // loop counter

            options(pure, nomem),
        );
    }

    if res_neg { asm_neg24(a) } else { a }
}

#[inline(always)]
pub fn asm_neg24(mut a: Int24Raw) -> Int24Raw {
    unsafe {
        asm!(
            "com {a2}",
            "com {a1}",
            "neg {a0}",
            "sbci {a1}, 0xFF",
            "sbci {a2}, 0xFF",

            a0 = inout(reg) a.0,
            a1 = inout(reg_upper) a.1,
            a2 = inout(reg_upper) a.2,

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
            "and {count}, {count}",
            "breq 2f",
            "1: lsl {a0}",
            "rol {a1}",
            "rol {a2}",
            "dec {count}",
            "brne 1b",
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
            "and {count}, {count}",
            "breq 2f",
            "1: asr {a2}",
            "ror {a1}",
            "ror {a0}",
            "dec {count}",
            "brne 1b",
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
pub fn asm_ge24(mut a: Int24Raw, b: Int24Raw) -> bool {
    unsafe {
        asm!(
            "cp {a0}, {b0}",
            "cpc {a1}, {b1}",
            "cpc {a2}, {b2}",
            "ldi {a0}, 1",
            "brge 1f",
            "clr {a0}",
            "1:",

            a0 = inout(reg) a.0,
            a1 = in(reg) a.1,
            a2 = in(reg) a.2,

            b0 = in(reg) b.0,
            b1 = in(reg) b.1,
            b2 = in(reg) b.2,

            options(pure, nomem, nostack),
        );
    }
    a.0 != 0
}

// vim: ts=4 sw=4 expandtab
