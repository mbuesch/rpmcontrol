use crate::raw::Int24Raw;

fn to_i32(a: Int24Raw) -> i32 {
    if a[2] & 0x80 == 0 {
        i32::from_le_bytes([a[0], a[1], a[2], 0x00])
    } else {
        i32::from_le_bytes([a[0], a[1], a[2], 0xFF])
    }
}

fn from_i32(a: i32) -> Int24Raw {
    let a = a.to_le_bytes();
    [a[0], a[1], a[2]]
}

pub fn asm_mul24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    from_i32(to_i32(a) * to_i32(b))
}

pub fn asm_div24(a: Int24Raw, b: Int24Raw) -> Int24Raw {
    from_i32(to_i32(a) / to_i32(b))
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
