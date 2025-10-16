#![no_std]
#![cfg_attr(target_arch = "avr", feature(asm_experimental_arch))]

pub use crate::raw::Int24Raw;
use crate::raw::{
    add24,
    conv::{i16_to_i24raw, i24raw_to_i16_sat, i24raw_to_i32, i32_to_i24raw_sat},
    div24, eq24, ge24, is_neg24, mul24, neg24, raw_zero, shl24, shl24_by8, shr24, shr24_by8, sub24,
};

#[cfg(not(target_arch = "avr"))]
mod asm_generic;
#[cfg(not(target_arch = "avr"))]
use asm_generic as asm;

#[cfg(target_arch = "avr")]
mod asm_avr;
#[cfg(target_arch = "avr")]
use asm_avr as asm;

mod raw;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(transparent)]
pub struct Int24(Int24Raw);

#[allow(clippy::should_implement_trait)]
impl Int24 {
    pub const fn zero() -> Self {
        Self(raw_zero())
    }

    pub const fn new() -> Self {
        Self::zero()
    }

    pub const fn from_raw(v: Int24Raw) -> Self {
        Self(v)
    }

    pub const fn from_i16(v: i16) -> Self {
        Self::from_raw(i16_to_i24raw(v))
    }

    pub const fn from_i32(v: i32) -> Self {
        Self(i32_to_i24raw_sat(v))
    }

    pub const fn to_i16(self) -> i16 {
        i24raw_to_i16_sat(self.0)
    }

    pub const fn to_i32(self) -> i32 {
        i24raw_to_i32(self.0)
    }

    #[inline(never)]
    pub fn add(self, other: Self) -> Self {
        Self::from_raw(add24(self.0, other.0))
    }

    pub const fn const_add(self, other: Self) -> Self {
        Self::from_i32(self.to_i32() + other.to_i32())
    }

    #[inline(never)]
    pub fn sub(self, other: Self) -> Self {
        Self::from_raw(sub24(self.0, other.0))
    }

    pub const fn const_sub(self, other: Self) -> Self {
        Self::from_i32(self.to_i32() - other.to_i32())
    }

    #[inline(never)]
    pub fn mul(self, other: Self) -> Self {
        Self::from_raw(mul24(self.0, other.0))
    }

    pub const fn const_mul(self, other: Self) -> Self {
        Self::from_i32(self.to_i32() * other.to_i32())
    }

    #[inline(never)]
    pub fn div(self, other: Self) -> Self {
        Self::from_raw(div24(self.0, other.0))
    }

    pub const fn const_div(self, other: Self) -> Self {
        Self::from_i32(self.to_i32() / other.to_i32())
    }

    #[inline(never)]
    pub fn neg(self) -> Self {
        Self(neg24(self.0))
    }

    pub const fn const_neg(self) -> Self {
        Self::from_i32(-self.to_i32())
    }

    #[inline(never)]
    pub fn abs(self) -> Self {
        if is_neg24(self.0) { self.neg() } else { self }
    }

    pub const fn const_abs(self) -> Self {
        if self.to_i32() < 0 {
            self.const_neg()
        } else {
            self
        }
    }

    pub const fn shl8(self) -> Self {
        Self(shl24_by8(self.0))
    }

    #[inline(never)]
    pub fn shl(self, count: u8) -> Self {
        Self(shl24(self.0, count))
    }

    pub const fn const_shl(self, count: u8) -> Self {
        Self::from_i32(self.to_i32() << count)
    }

    pub const fn shr8(self) -> Self {
        Self(shr24_by8(self.0))
    }

    #[inline(never)]
    pub fn shr(self, count: u8) -> Self {
        Self(shr24(self.0, count))
    }

    pub const fn const_shr(self, count: u8) -> Self {
        Self::from_i32(self.to_i32() >> count)
    }

    #[inline(never)]
    pub fn cmp(self, other: Self) -> core::cmp::Ordering {
        if eq24(self.0, other.0) {
            core::cmp::Ordering::Equal
        } else if ge24(self.0, other.0) {
            core::cmp::Ordering::Greater
        } else {
            core::cmp::Ordering::Less
        }
    }

    pub const fn const_cmp(self, other: Self) -> core::cmp::Ordering {
        if self.to_i32() == other.to_i32() {
            core::cmp::Ordering::Equal
        } else if self.to_i32() >= other.to_i32() {
            core::cmp::Ordering::Greater
        } else {
            core::cmp::Ordering::Less
        }
    }
}

impl Default for Int24 {
    fn default() -> Self {
        Self::new()
    }
}

impl core::cmp::Ord for Int24 {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Self::cmp(*self, *other)
    }
}

impl core::cmp::PartialOrd for Int24 {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl core::ops::Add for Int24 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::add(self, other)
    }
}

impl core::ops::AddAssign for Int24 {
    fn add_assign(&mut self, other: Self) {
        self.0 = (*self + other).0;
    }
}

impl core::ops::Sub for Int24 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::sub(self, other)
    }
}

impl core::ops::SubAssign for Int24 {
    fn sub_assign(&mut self, other: Self) {
        self.0 = (*self - other).0;
    }
}

impl core::ops::Mul for Int24 {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::mul(self, other)
    }
}

impl core::ops::MulAssign for Int24 {
    fn mul_assign(&mut self, other: Self) {
        self.0 = (*self * other).0;
    }
}

impl core::ops::Div for Int24 {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Self::div(self, other)
    }
}

impl core::ops::DivAssign for Int24 {
    fn div_assign(&mut self, other: Self) {
        self.0 = (*self / other).0;
    }
}

impl core::ops::Neg for Int24 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::neg(self)
    }
}

impl core::ops::Shl<u8> for Int24 {
    type Output = Self;

    fn shl(self, other: u8) -> Self {
        Self::shl(self, other)
    }
}

impl core::ops::ShlAssign<u8> for Int24 {
    fn shl_assign(&mut self, other: u8) {
        self.0 = (*self << other).0;
    }
}

impl core::ops::Shr<u8> for Int24 {
    type Output = Self;

    fn shr(self, other: u8) -> Self {
        Self::shr(self, other)
    }
}

impl core::ops::ShrAssign<u8> for Int24 {
    fn shr_assign(&mut self, other: u8) {
        self.0 = (*self >> other).0;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_conv_i16() {
        let a = 0x1234;
        let b = Int24::from_i16(a).to_i16();
        assert_eq!(a, b);

        let a = -0x1234;
        let b = Int24::from_i16(a).to_i16();
        assert_eq!(a, b);

        let a = 0x123456;
        let b = Int24::from_i32(a).to_i16();
        assert_eq!(b as u16, 0x7FFF);

        let a = -0x123456;
        let b = Int24::from_i32(a).to_i16();
        assert_eq!(b, -0x8000);
        assert_eq!(b as u16, 0x8000);

        let mut a = 0x0000_8000_u32;
        loop {
            let b = Int24::from_i32(a as i32).to_i16();
            assert_eq!(b as u16, 0x7FFF);
            if a == 0x4000_0000_u32 {
                break;
            }
            a <<= 1;
        }

        let mut a = 0xFFFF_8000_u32;
        loop {
            let b = Int24::from_i32(a as i32).to_i16();
            assert_eq!(b as u16, 0x8000);
            if a == 0x8000_0000_u32 {
                break;
            }
            a <<= 1;
        }
    }

    #[test]
    fn test_conv_i32() {
        let a = 0x123456;
        let b = Int24::from_i32(a).to_i32();
        assert_eq!(a, b);

        let a = -0x123456;
        let b = Int24::from_i32(a).to_i32();
        assert_eq!(a, b);

        let a = 0x12345678;
        let b = Int24::from_i32(a).to_i32();
        assert_eq!(b as u32, 0x007F_FFFF);

        let a = -0x12345678;
        let b = Int24::from_i32(a).to_i32();
        assert_eq!(b, -0x800000);
        assert_eq!(b as u32, 0xFF80_0000);

        let mut a = 0x0080_0000_u32;
        loop {
            let b = Int24::from_i32(a as i32).to_i32();
            assert_eq!(b as u32, 0x007F_FFFF);
            if a == 0x4000_0000_u32 {
                break;
            }
            a <<= 1;
        }

        let mut a = 0xFF80_0000_u32;
        loop {
            let b = Int24::from_i32(a as i32).to_i32();
            assert_eq!(b as u32, 0xFF80_0000);
            if a == 0x8000_0000_u32 {
                break;
            }
            a <<= 1;
        }
    }

    #[test]
    fn test_add() {
        let a = Int24::from_i32(1000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(2010);
        assert_eq!(a + b, c);
        assert_eq!(a.const_add(b), c);

        let a = Int24::from_i32(1000);
        let b = Int24::from_i32(-1010);
        let c = Int24::from_i32(-10);
        assert_eq!(a + b, c);
        assert_eq!(a.const_add(b), c);

        let a = Int24::from_i32(-1000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(10);
        assert_eq!(a + b, c);
        assert_eq!(a.const_add(b), c);

        let a = Int24::from_i32(0x7F_FFFF - 1);
        let b = Int24::from_i32(2);
        let c = Int24::from_i32(0x7F_FFFF);
        assert_eq!(a + b, c);
        assert_eq!(a.const_add(b), c);

        let a = Int24::from_i32(-0x80_0000 + 1);
        let b = Int24::from_i32(-2);
        let c = Int24::from_i32(-0x80_0000);
        assert_eq!(a + b, c);
        assert_eq!(a.const_add(b), c);
    }

    #[test]
    fn test_sub() {
        let a = Int24::from_i32(1000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(-10);
        assert_eq!(a - b, c);
        assert_eq!(a.const_sub(b), c);

        let a = Int24::from_i32(1000);
        let b = Int24::from_i32(-1010);
        let c = Int24::from_i32(2010);
        assert_eq!(a - b, c);
        assert_eq!(a.const_sub(b), c);

        let a = Int24::from_i32(-1000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(-2010);
        assert_eq!(a - b, c);
        assert_eq!(a.const_sub(b), c);

        let a = Int24::from_i32(-0x80_0000 + 1);
        let b = Int24::from_i32(2);
        let c = Int24::from_i32(-0x80_0000);
        assert_eq!(a - b, c);
        assert_eq!(a.const_sub(b), c);

        let a = Int24::from_i32(0x7F_FFFF - 1);
        let b = Int24::from_i32(-2);
        let c = Int24::from_i32(0x7F_FFFF);
        assert_eq!(a - b, c);
        assert_eq!(a.const_sub(b), c);
    }

    #[test]
    fn test_mul() {
        let a = Int24::from_i32(1000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(1010000);
        assert_eq!(a * b, c);
        assert_eq!(a.const_mul(b), c);

        let a = Int24::from_i32(1000);
        let b = Int24::from_i32(-1010);
        let c = Int24::from_i32(-1010000);
        assert_eq!(a * b, c);
        assert_eq!(a.const_mul(b), c);

        let a = Int24::from_i32(-1000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(-1010000);
        assert_eq!(a * b, c);
        assert_eq!(a.const_mul(b), c);

        //TODO sat
    }

    #[test]
    fn test_div() {
        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(99);
        assert_eq!(a / b, c);
        assert_eq!(a.const_div(b), c);

        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(-1010);
        let c = Int24::from_i32(-99);
        assert_eq!(a / b, c);
        assert_eq!(a.const_div(b), c);

        let a = Int24::from_i32(-100000);
        let b = Int24::from_i32(1010);
        let c = Int24::from_i32(-99);
        assert_eq!(a / b, c);
        assert_eq!(a.const_div(b), c);

        let a = Int24::from_i32(-0x80_0000);
        let b = Int24::from_i32(-1);
        let c = Int24::from_i32(0x7F_FFFF); // sat
        assert_eq!(a / b, c);
        assert_eq!(a.const_div(b), c);
    }

    #[test]
    fn test_neg() {
        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(-100000);
        assert_eq!(-a, b);
        assert_eq!(a.const_neg(), b);

        let a = Int24::from_i32(-100000);
        let b = Int24::from_i32(100000);
        assert_eq!(-a, b);
        assert_eq!(a.const_neg(), b);

        let a = Int24::from_i32(0x7F_FFFF);
        let b = Int24::from_i32(-0x7F_FFFF);
        assert_eq!(-a, b);
        assert_eq!(a.const_neg(), b);

        let a = Int24::from_i32(-0x7F_FFFF);
        let b = Int24::from_i32(0x7F_FFFF);
        assert_eq!(-a, b);
        assert_eq!(a.const_neg(), b);

        let a = Int24::from_i32(-0x80_0000);
        let b = Int24::from_i32(0x7F_FFFF); // saturated
        assert_eq!(-a, b);
        assert_eq!(a.const_neg(), b);
    }

    #[test]
    fn test_abs() {
        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(100000);
        assert_eq!(a.abs(), b);
        assert_eq!(a.const_abs(), b);

        let a = Int24::from_i32(-100000);
        let b = Int24::from_i32(100000);
        assert_eq!(a.abs(), b);
        assert_eq!(a.const_abs(), b);

        let a = Int24::from_i32(0x7F_FFFF);
        let b = Int24::from_i32(0x7F_FFFF);
        assert_eq!(a.abs(), b);
        assert_eq!(a.const_abs(), b);

        let a = Int24::from_i32(-0x7F_FFFF);
        let b = Int24::from_i32(0x7F_FFFF);
        assert_eq!(a.abs(), b);
        assert_eq!(a.const_abs(), b);

        let a = Int24::from_i32(-0x80_0000);
        let b = Int24::from_i32(0x7F_FFFF); // saturated
        assert_eq!(a.abs(), b);
        assert_eq!(a.const_abs(), b);
    }

    #[test]
    fn test_shl() {
        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(400000);
        assert_eq!(a << 2, b);
        assert_eq!(a.const_shl(2), b);

        let a = Int24::from_i32(1000);
        let b = Int24::from_i32(256000);
        assert_eq!(a.shl8(), b);
    }

    #[test]
    fn test_shr() {
        let a = Int24::from_i32(400000);
        let b = Int24::from_i32(100000);
        assert_eq!(a >> 2, b);
        assert_eq!(a.const_shr(2), b);

        let a = Int24::from_i32(256000);
        let b = Int24::from_i32(1000);
        assert_eq!(a.shr8(), b);
    }

    #[test]
    fn test_cmp() {
        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(100000);
        assert_eq!(a, b);
        assert_eq!(a.const_cmp(b), core::cmp::Ordering::Equal);

        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(100001);
        assert_ne!(a, b);
        assert_eq!(a.const_cmp(b), core::cmp::Ordering::Less);

        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(100000);
        assert!(a <= b);
        assert_eq!(a.const_cmp(b), core::cmp::Ordering::Equal);

        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(100001);
        assert!(a < b);
        assert_eq!(a.const_cmp(b), core::cmp::Ordering::Less);

        let a = Int24::from_i32(100000);
        let b = Int24::from_i32(100000);
        assert!(a >= b);
        assert_eq!(a.const_cmp(b), core::cmp::Ordering::Equal);

        let a = Int24::from_i32(100001);
        let b = Int24::from_i32(100000);
        assert!(a > b);
        assert_eq!(a.const_cmp(b), core::cmp::Ordering::Greater);
    }
}

// vim: ts=4 sw=4 expandtab
