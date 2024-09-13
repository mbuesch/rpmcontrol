#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixpt(i16);

macro_rules! fixpt {
    ($numerator:literal / $denominator:literal) => {
        Fixpt::from_decimal($numerator, $denominator)
    };
    ($numerator:literal / $denominator:ident) => {
        Fixpt::from_decimal($numerator, $denominator)
    };
    ($numerator:ident / $denominator:literal) => {
        Fixpt::from_decimal($numerator, $denominator)
    };
    ($numerator:ident / $denominator:ident) => {
        Fixpt::from_decimal($numerator, $denominator)
    };
}
pub(crate) use fixpt;

impl Fixpt {
    pub const SHIFT: usize = 8;

    pub const fn from_int(int: i16) -> Self {
        Self(int << Self::SHIFT)
    }

    #[allow(dead_code)]
    pub const fn from_parts(int: i16, frac: u16) -> Self {
        Self(int << Self::SHIFT | frac as i16)
    }

    pub const fn from_decimal(numerator: i16, denominator: i16) -> Self {
        let mut q: i32 = 1 << Self::SHIFT;
        q *= numerator as i32;
        q /= denominator as i32;
        Self(q as i16)
    }

    pub const fn to_int(self) -> i16 {
        self.0 >> Self::SHIFT
    }
}

impl From<u8> for Fixpt {
    fn from(value: u8) -> Self {
        Self::from_int(value.into())
    }
}

impl From<i8> for Fixpt {
    fn from(value: i8) -> Self {
        Self::from_int(value.into())
    }
}

impl From<i16> for Fixpt {
    fn from(value: i16) -> Self {
        Self::from_int(value)
    }
}

impl core::ops::Add for Fixpt {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl core::ops::Sub for Fixpt {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl core::ops::Mul for Fixpt {
    type Output = Self;

    #[inline(never)]
    fn mul(self, other: Self) -> Self {
        Self(((self.0 as i32 * other.0 as i32) >> Self::SHIFT) as i16)
    }
}

/*
impl core::ops::Div for Fixpt {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let mut tmp: i32 = self.0.into();
        tmp <<= Self::SHIFT;
        tmp /= other.0 as i32;
        Self(tmp as i16)
    }
}
*/

impl core::ops::Neg for Fixpt {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

// vim: ts=4 sw=4 expandtab
