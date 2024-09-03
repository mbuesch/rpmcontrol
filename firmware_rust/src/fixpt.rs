#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixpt(i16);

impl Fixpt {
    pub const SHIFT: usize = 8;

    pub const fn new(int: i16) -> Self {
        Self(int << Self::SHIFT)
    }

    pub const fn from_parts(int: i16, frac: u16) -> Self {
        Self(int << Self::SHIFT | frac as i16)
    }
}

impl From<i16> for Fixpt {
    fn from(value: i16) -> Self {
        Self::new(value)
    }
}

impl core::ops::Add for Fixpt {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        (self.0 + other.0).into()
    }
}

impl core::ops::Sub for Fixpt {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        (self.0 - other.0).into()
    }
}

impl core::ops::Mul for Fixpt {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        ((self.0 * other.0) >> Self::SHIFT).into()
    }
}

impl core::ops::Div for Fixpt {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let mut tmp: i32 = self.0.into();
        tmp <<= Self::SHIFT;
        tmp /= other.0 as i32;
        (tmp as i16).into()
    }
}

// vim: ts=4 sw=4 expandtab
