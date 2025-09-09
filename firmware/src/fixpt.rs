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

    pub const fn upgrade(&self) -> BigFixpt {
        let v = self.0.to_le_bytes();
        if v[1] & 0x80 == 0 {
            BigFixpt([v[0], v[1], 0x00])
        } else {
            BigFixpt([v[0], v[1], 0xFF])
        }
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn from_int(int: i16) -> Self {
        Self(int << Self::SHIFT)
    }

    pub const fn from_parts(int: i16, frac: u16) -> Self {
        Self(int << Self::SHIFT | frac as i16)
    }

    #[inline(never)]
    pub const fn from_fraction(numerator: i16, denominator: i16) -> Self {
        let mut q: i32 = 1 << Self::SHIFT;
        q *= numerator as i32;
        q /= denominator as i32;
        Self::from_q_sat(q)
    }

    #[inline(never)]
    const fn from_q_sat(v: i32) -> Self {
        if v < i16::MIN as i32 {
            Self(i16::MIN)
        } else if v > i16::MAX as i32 {
            Self(i16::MAX)
        } else {
            Self(v as i16)
        }
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
    pub const fn mul(self, other: Self) -> Self {
        let prod = (self.0 as i32 * other.0 as i32) >> Self::SHIFT;
        Self::from_q_sat(prod)
    }

    #[inline(never)]
    pub const fn div(self, other: Self) -> Self {
        let mut tmp = self.0 as i32;
        tmp <<= Self::SHIFT;
        tmp /= other.0 as i32;
        Self::from_q_sat(tmp)
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BigFixpt([u8; 3]);

impl BigFixpt {
    pub const SHIFT: usize = Fixpt::SHIFT;

    #[inline(never)]
    pub const fn downgrade(&self) -> Fixpt {
        Fixpt::from_q_sat(self.to_q())
    }

    #[inline(never)]
    const fn to_q(self) -> i32 {
        if self.0[2] & 0x80 == 0 {
            i32::from_le_bytes([self.0[0], self.0[1], self.0[2], 0x00])
        } else {
            i32::from_le_bytes([self.0[0], self.0[1], self.0[2], 0xFF])
        }
    }

    #[inline(never)]
    const fn from_q_sat(q: i32) -> Self {
        if q < -0x80_0000 {
            Self([0x00, 0x00, 0x80])
        } else if q > 0x7F_FFFF {
            Self([0xFF, 0xFF, 0x7F])
        } else {
            let q = q.to_le_bytes();
            Self([q[0], q[1], q[2]])
        }
    }

    #[inline(never)]
    pub const fn add(self, other: Self) -> Self {
        Self::from_q_sat(self.to_q() + other.to_q())
    }

    #[inline(never)]
    pub const fn sub(self, other: Self) -> Self {
        Self::from_q_sat(self.to_q() - other.to_q())
    }

    #[inline(never)]
    pub const fn mul(self, other: Self) -> Self {
        let prod = (self.to_q() * other.to_q()) >> Self::SHIFT;
        Self::from_q_sat(prod)
    }

    #[inline(never)]
    pub const fn div(self, other: Self) -> Self {
        let mut tmp = self.to_q();
        tmp <<= Self::SHIFT;
        tmp /= other.to_q();
        Self::from_q_sat(tmp)
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
