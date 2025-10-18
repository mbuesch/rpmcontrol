use crate::raw::{Int24Raw, is_neg24, raw_max, raw_min, raw_minus_one, raw_zero};

fn to_i32(a: Int24Raw) -> i32 {
    if a.2 & 0x80 == 0 {
        i32::from_le_bytes([a.0, a.1, a.2, 0x00])
    } else {
        i32::from_le_bytes([a.0, a.1, a.2, 0xFF])
    }
}

fn from_i32(a: i32) -> Int24Raw {
    let a = a.to_le_bytes();
    (a[0], a[1], a[2])
}

pub fn asm_mulsat24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    let c = to_i32(a) as i64 * to_i32(b) as i64;
    if c > 0x7F_FFFF {
        from_i32(0x7F_FFFF)
    } else if c < -0x80_0000 {
        from_i32(-0x80_0000)
    } else {
        from_i32(c as i32)
    }
}

pub fn asm_divsat24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    if b == raw_zero() {
        if is_neg24(a) { raw_min() } else { raw_max() }
    } else if a == raw_min() && b == raw_minus_one() {
        raw_max()
    } else {
        from_i32(to_i32(a) / to_i32(b))
    }
}

pub fn asm_neg24(a: Int24Raw) -> Int24Raw {
    from_i32(-to_i32(a))
}

pub fn asm_shl24(a: Int24Raw, count: u8) -> Int24Raw {
    from_i32(to_i32(a) << count)
}

pub fn asm_shr24(a: Int24Raw, count: u8) -> Int24Raw {
    from_i32(to_i32(a) >> count)
}

pub fn asm_ge24(a: Int24Raw, b: Int24Raw) -> bool {
    to_i32(a) >= to_i32(b)
}

// vim: ts=4 sw=4 expandtab
