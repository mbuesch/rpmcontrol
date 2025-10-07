use crate::{
    asm::{asm_div24, asm_ge24, asm_mul24, asm_neg24, asm_shl24, asm_shr24},
    raw::conv::{i24raw_to_i32, i32_to_i24raw_sat},
};

pub type Int24Raw = (u8, u8, u8);
pub const RAW_ZERO: Int24Raw = (0x00, 0x00, 0x00);
pub const RAW_MIN: Int24Raw = (0x00, 0x00, 0x80);
pub const RAW_MAX: Int24Raw = (0xFF, 0xFF, 0x7F);

#[inline(always)]
pub fn mul24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    let res = asm_mul24(a, b);
    //TODO sat
    res
}

#[inline(always)]
pub fn div24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    if b == RAW_ZERO {
        // Division by zero.
        if is_neg24(a) { RAW_MIN } else { RAW_MAX }
    } else {
        let res = asm_div24(a, b);
        //TODO sat
        res
    }
}

#[inline(always)]
pub fn add24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    // Use 32 bit arithmetic to detect and saturate overflow.
    i32_to_i24raw_sat(i24raw_to_i32(a) + i24raw_to_i32(b))
}

#[inline(always)]
pub fn sub24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    // Use 32 bit arithmetic to detect and saturate overflow.
    i32_to_i24raw_sat(i24raw_to_i32(a) - i24raw_to_i32(b))
}

#[inline(always)]
pub const fn is_neg24(a: Int24Raw) -> bool {
    a.2 & 0x80 != 0
}

#[inline(always)]
pub fn neg24(a: Int24Raw) -> Int24Raw {
    if a == RAW_MIN { RAW_MAX } else { asm_neg24(a) }
}

#[inline(always)]
pub const fn shl24_by8(a: Int24Raw) -> Int24Raw {
    (0x00, a.0, a.1)
}

#[inline(always)]
pub fn shl24(a: Int24Raw, count: u8) -> Int24Raw {
    asm_shl24(a, count)
}

#[inline(always)]
pub const fn shr24_by8(a: Int24Raw) -> Int24Raw {
    if is_neg24(a) {
        (a.1, a.2, 0xFF)
    } else {
        (a.1, a.2, 0x00)
    }
}

#[inline(always)]
pub fn shr24(a: Int24Raw, count: u8) -> Int24Raw {
    asm_shr24(a, count)
}

#[inline(always)]
pub fn eq24(a: Int24Raw, b: Int24Raw) -> bool {
    a == b
}

#[inline(always)]
pub fn ge24(a: Int24Raw, b: Int24Raw) -> bool {
    asm_ge24(a, b)
}

pub mod conv {
    use super::{Int24Raw, RAW_MAX, RAW_MIN, is_neg24};

    #[inline(never)]
    pub const fn i24raw_to_i32(v: Int24Raw) -> i32 {
        if is_neg24(v) {
            i32::from_le_bytes([v.0, v.1, v.2, 0xFF])
        } else {
            i32::from_le_bytes([v.0, v.1, v.2, 0x00])
        }
    }

    #[inline(never)]
    pub const fn i24raw_to_i16_sat(v: Int24Raw) -> i16 {
        if (v.2 == 0 && v.1 & 0x80 == 0) || (v.2 == 0xFF && v.1 & 0x80 != 0) {
            i16::from_le_bytes([v.0, v.1])
        } else if is_neg24(v) {
            i16::MIN // saturate
        } else {
            i16::MAX // saturate
        }
    }

    #[inline(never)]
    pub const fn i32_to_i24raw_sat(v: i32) -> Int24Raw {
        if v < -0x80_0000 {
            RAW_MIN // saturate
        } else if v > 0x7F_FFFF {
            RAW_MAX // saturate
        } else {
            let v = v.to_le_bytes();
            (v[0], v[1], v[2])
        }
    }

    #[inline(never)]
    pub const fn i16_to_i24raw(v: i16) -> Int24Raw {
        let v = v.to_le_bytes();
        if v[1] & 0x80 == 0 {
            (v[0], v[1], 0x00)
        } else {
            (v[0], v[1], 0xFF)
        }
    }
}

// vim: ts=4 sw=4 expandtab
