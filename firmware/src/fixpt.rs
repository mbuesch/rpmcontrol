use avr_int24::Int24;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Fixpt(i16);

macro_rules! fixpt {
    ($numerator:literal / $denominator:literal) => {
        const { $crate::fixpt::Fixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:literal / $denominator:ident) => {
        const { $crate::fixpt::Fixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:ident / $denominator:literal) => {
        const { $crate::fixpt::Fixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:ident / $denominator:ident) => {
        const { $crate::fixpt::Fixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:literal) => {
        const { $crate::fixpt::Fixpt::from_int($numerator) }
    };
    ($numerator:ident) => {
        const { $crate::fixpt::Fixpt::from_int($numerator) }
    };
}
pub(crate) use fixpt;

impl Fixpt {
    pub const SHIFT: usize = 8;

    pub const fn upgrade(&self) -> BigFixpt {
        BigFixpt(Int24::from_i16(self.to_q()))
    }

    pub const fn from_int(int: i16) -> Self {
        Self(int << Self::SHIFT)
    }

    pub const fn const_from_fraction(numerator: i16, denominator: i16) -> Self {
        Self(numerator).const_div(Self(denominator))
    }

    pub fn from_fraction(numerator: i16, denominator: i16) -> Self {
        Self(numerator) / Self(denominator)
    }

    pub const fn to_int(self) -> i16 {
        self.0 >> Self::SHIFT
    }

    pub const fn to_q(self) -> i16 {
        self.0
    }

    #[inline(never)]
    pub const fn add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    #[inline(never)]
    pub const fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    #[inline(never)]
    pub fn mul(self, other: Self) -> Self {
        const {
            assert!(Self::SHIFT == 8);
        }
        let a = Int24::from_i16(self.0);
        let b = Int24::from_i16(other.0);
        let c = (a * b).shr8();
        Self(c.to_i16())
    }

    #[inline(never)]
    pub fn div(self, other: Self) -> Self {
        const {
            assert!(Self::SHIFT == 8);
        }
        let a = Int24::from_i16(self.0);
        let b = Int24::from_i16(other.0);
        let c = a.shl8() / b;
        Self(c.to_i16())
    }

    pub const fn const_div(self, other: Self) -> Self {
        const {
            assert!(Self::SHIFT == 8);
        }
        let a = Int24::from_i16(self.0);
        let b = Int24::from_i16(other.0);
        let c = a.shl8().const_div(b);
        Self(c.to_i16())
    }

    #[inline(never)]
    pub const fn neg(self) -> Self {
        if self.0 == i16::MIN {
            Self(i16::MAX)
        } else {
            Self(-self.0)
        }
    }

    #[inline(never)]
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
    #[inline(never)]
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

macro_rules! big_fixpt {
    ($numerator:literal / $denominator:literal) => {
        const { $crate::fixpt::BigFixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:literal / $denominator:ident) => {
        const { $crate::fixpt::BigFixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:ident / $denominator:literal) => {
        const { $crate::fixpt::BigFixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:ident / $denominator:ident) => {
        const { $crate::fixpt::BigFixpt::const_from_fraction($numerator, $denominator) }
    };
    ($numerator:literal) => {
        const { $crate::fixpt::BigFixpt::from_int($numerator) }
    };
    ($numerator:ident) => {
        const { $crate::fixpt::BigFixpt::from_int($numerator) }
    };
}
pub(crate) use big_fixpt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BigFixpt(Int24);

impl BigFixpt {
    pub const SHIFT: usize = Fixpt::SHIFT;

    pub const fn downgrade(&self) -> Fixpt {
        Fixpt(self.0.to_i16())
    }

    pub const fn from_int(int: i16) -> Self {
        const {
            assert!(Self::SHIFT == 8);
        }
        Self(Int24::from_i16(int).shl8())
    }

    pub const fn const_from_fraction(numerator: i16, denominator: i16) -> Self {
        Self(Int24::from_i16(numerator)).const_div(Self(Int24::from_i16(denominator)))
    }

    pub fn from_fraction(numerator: i16, denominator: i16) -> Self {
        Self(Int24::from_i16(numerator)) / Self(Int24::from_i16(denominator))
    }

    pub fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }

    pub fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }

    #[inline(never)]
    pub fn mul(self, other: Self) -> Self {
        Self(Int24::from_i32(
            (self.0.to_i32() * other.0.to_i32()) >> Self::SHIFT,
        ))
    }

    #[inline(never)]
    pub fn div(self, other: Self) -> Self {
        self.const_div(other)
    }

    pub const fn const_div(self, other: Self) -> Self {
        let a = self.0.to_i32();
        let b = other.0.to_i32();
        let c = if b == 0 {
            if a < 0 { i32::MIN } else { i32::MAX }
        } else {
            (a << Self::SHIFT).saturating_div(b)
        };
        Self(Int24::from_i32(c))
    }
}

impl From<Fixpt> for BigFixpt {
    fn from(v: Fixpt) -> BigFixpt {
        v.upgrade()
    }
}

impl From<BigFixpt> for Fixpt {
    fn from(v: BigFixpt) -> Fixpt {
        v.downgrade()
    }
}

impl core::ops::Add for BigFixpt {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        BigFixpt::add(self, other)
    }
}

impl core::ops::AddAssign for BigFixpt {
    fn add_assign(&mut self, other: Self) {
        self.0 = (*self + other).0;
    }
}

impl core::ops::Sub for BigFixpt {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        BigFixpt::sub(self, other)
    }
}

impl core::ops::SubAssign for BigFixpt {
    fn sub_assign(&mut self, other: Self) {
        self.0 = (*self - other).0;
    }
}

impl core::ops::Mul for BigFixpt {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        BigFixpt::mul(self, other)
    }
}

impl core::ops::MulAssign for BigFixpt {
    fn mul_assign(&mut self, other: Self) {
        self.0 = (*self * other).0;
    }
}

impl core::ops::Div for BigFixpt {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        BigFixpt::div(self, other)
    }
}

impl core::ops::DivAssign for BigFixpt {
    fn div_assign(&mut self, other: Self) {
        self.0 = (*self / other).0;
    }
}

// vim: ts=4 sw=4 expandtab
