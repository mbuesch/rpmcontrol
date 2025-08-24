#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixpt(i16);

macro_rules! fixpt {
    ($numerator:literal / $denominator:literal) => {
        Fixpt::from_fraction($numerator, $denominator)
    };
    ($numerator:literal / $denominator:ident) => {
        Fixpt::from_fraction($numerator, $denominator)
    };
    ($numerator:ident / $denominator:literal) => {
        Fixpt::from_fraction($numerator, $denominator)
    };
    ($numerator:ident / $denominator:ident) => {
        Fixpt::from_fraction($numerator, $denominator)
    };
    ($numerator:literal) => {
        Fixpt::from_int($numerator)
    };
    ($numerator:ident) => {
        Fixpt::from_int($numerator)
    };
}
pub(crate) use fixpt;

impl Fixpt {
    pub const SHIFT: usize = 8;

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn from_int(int: i16) -> Self {
        Self(int << Self::SHIFT)
    }

    pub const fn from_parts(int: i16, frac: u16) -> Self {
        Self(int << Self::SHIFT | frac as i16)
    }

    pub const fn from_fraction(numerator: i16, denominator: i16) -> Self {
        let mut q: i32 = 1 << Self::SHIFT;
        q *= numerator as i32;
        q /= denominator as i32;
        Self(q as i16)
    }

    pub const fn to_int(self) -> i16 {
        self.0 >> Self::SHIFT
    }

    pub const fn to_q(self) -> i16 {
        self.0
    }

    pub const fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }

    pub const fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }

    #[inline(never)]
    pub const fn mul(self, other: Self) -> Self {
        Self(((self.0 as i32 * other.0 as i32) >> Self::SHIFT) as i16)
    }

    #[inline(never)]
    pub const fn div(self, other: Self) -> Self {
        let mut tmp: i32 = self.0 as i32;
        tmp <<= Self::SHIFT;
        tmp /= other.0 as i32;
        Self(tmp as i16)
    }

    #[inline(never)]
    pub const fn neg(self) -> Self {
        if self.0 == i16::MIN {
            Self(i16::MAX)
        } else {
            Self(-self.0)
        }
    }

    pub const fn abs(self) -> Self {
        if self.0 < 0 { self.neg() } else { self }
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
        Fixpt::add(self, other)
    }
}

impl core::ops::AddAssign for Fixpt {
    fn add_assign(&mut self, other: Self) {
        self.0 = (*self + other).0;
    }
}

impl core::ops::Sub for Fixpt {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Fixpt::sub(self, other)
    }
}

impl core::ops::SubAssign for Fixpt {
    fn sub_assign(&mut self, other: Self) {
        self.0 = (*self - other).0;
    }
}

impl core::ops::Mul for Fixpt {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Fixpt::mul(self, other)
    }
}

impl core::ops::MulAssign for Fixpt {
    fn mul_assign(&mut self, other: Self) {
        self.0 = (*self * other).0;
    }
}

impl core::ops::Div for Fixpt {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        Fixpt::div(self, other)
    }
}

impl core::ops::DivAssign for Fixpt {
    fn div_assign(&mut self, other: Self) {
        self.0 = (*self / other).0;
    }
}

impl core::ops::Neg for Fixpt {
    type Output = Self;

    fn neg(self) -> Self {
        Fixpt::neg(self)
    }
}

impl curveipo::CurvePoint<Fixpt> for (Fixpt, Fixpt) {
    fn x(&self) -> Fixpt {
        self.0
    }

    fn y(&self) -> Fixpt {
        self.1
    }
}

impl curveipo::CurveIpo for Fixpt {
    fn lin_inter(
        &self,
        left: &impl curveipo::CurvePoint<Self>,
        right: &impl curveipo::CurvePoint<Self>,
    ) -> Self {
        let dx = right.x() - left.x();
        let dy = right.y() - left.y();
        if dx == fixpt!(0) {
            left.y()
        } else {
            ((*self - left.x()) * (dy / dx)) + left.y()
        }
    }
}

// vim: ts=4 sw=4 expandtab
