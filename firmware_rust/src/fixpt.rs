#[derive(Copy, Clone)]
pub struct Fixpt(i16);

impl Fixpt {
    const SHIFT: usize = 8;

    pub const fn new(value: i16) -> Self {
        Self(value << Self::SHIFT)
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
        //FIXME overflows a
        ((self.0 << Self::SHIFT) / other.0).into()
    }
}

// vim: ts=4 sw=4 expandtab
