use crate::errors::MiniStabbleError;
use fixed::types::U34F30;
use fixed_exp::FixedPowF;

pub const SCALE: u128 = 1_000_000_000;

pub const ZERO: u128 = 0;
pub const ONE: u128 = SCALE;
pub const TWO: u128 = 2 * SCALE;
pub const THREE: u128 = 3 * SCALE;
pub const FOUR: u128 = 4 * SCALE;

// 1 << 30 = 1073741824 - represents 1.0 in U34F30 format (30 fractional bits)
pub const BITS_ONE: u64 = 1 << 30;

pub trait FixedMul {
    fn mul_down(self, other: Self) -> Result<Self, MiniStabbleError>
    where
        Self: Sized;

    fn mul_up(self, other: Self) -> Result<Self, MiniStabbleError>
    where
        Self: Sized;
}

pub trait FixedDiv {
    fn div_down(self, other: Self) -> Result<Self, MiniStabbleError>
    where
        Self: Sized;

    fn div_up(self, other: Self) -> Result<Self, MiniStabbleError>
    where
        Self: Sized;
}

pub trait FixedComplement {
    fn complement(self) -> Self;
}

impl FixedMul for u128 {
    fn mul_down(self, other: Self) -> Result<Self, MiniStabbleError> {
        // (self * other) / SCALE, rounded down
        self.checked_mul(other)
            .and_then(|v| v.checked_div(SCALE))
            .ok_or(MiniStabbleError::MathOverflow)
    }

    fn mul_up(self, other: Self) -> Result<Self, MiniStabbleError> {
        // (self * other + SCALE - 1) / SCALE, rounded up
        let product = self
            .checked_mul(other)
            .ok_or(MiniStabbleError::MathOverflow)?;

        // Round up: add (SCALE - 1) before dividing
        product
            .checked_add(SCALE - 1)
            .and_then(|v| v.checked_div(SCALE))
            .ok_or(MiniStabbleError::MathOverflow)
    }
}

impl FixedDiv for u128 {
    fn div_down(self, other: Self) -> Result<Self, MiniStabbleError> {
        if other == 0 {
            return Err(MiniStabbleError::DivideByZero);
        }
        // (self * SCALE) / other, rounded down
        self.checked_mul(SCALE)
            .and_then(|v| v.checked_div(other))
            .ok_or(MiniStabbleError::MathOverflow)
    }

    fn div_up(self, other: Self) -> Result<Self, MiniStabbleError> {
        if other == 0 {
            return Err(MiniStabbleError::DivideByZero);
        }
        // (self * SCALE + other - 1) / other, rounded up
        let numerator = self
            .checked_mul(SCALE)
            .ok_or(MiniStabbleError::MathOverflow)?;

        numerator
            .checked_add(other - 1)
            .and_then(|v| v.checked_div(other))
            .ok_or(MiniStabbleError::MathOverflow)
    }
}

impl FixedComplement for u128 {
    fn complement(self) -> Self {
        // 1.0 - self (saturating at 0)
        ONE.saturating_sub(self)
    }
}

pub trait FixedPow {
    fn pow_down(self, exp: Self) -> Result<Self, MiniStabbleError>
    where
        Self: Sized;
    fn pow_up(self, exp: Self) -> Result<Self, MiniStabbleError>
    where
        Self: Sized;
}

impl FixedPow for u128 {
    // Optimize for when y equals 1.0, 2.0, 3.0 or 4.0, as those are very simple to implement and occur often in
    // 50/50, 80/20 and 60/20/20 Weighted Pools

    fn pow_down(self, rhs: Self) -> Result<Self, MiniStabbleError> {
        match rhs {
            ZERO => Ok(ONE),
            ONE => Ok(self),
            TWO => self.mul_down(self),
            THREE => self.mul_down(self)?.mul_down(self),
            FOUR => {
                let square = self.mul_down(self)?;
                square.mul_down(square)
            }
            _ => {
                let base = U34F30::from_bits((self as u64).mul_down(BITS_ONE)?);
                let exp = U34F30::from_bits((rhs as u64).mul_down(BITS_ONE)?);
                Ok(base.powf(exp).ok_or(MiniStabbleError::MathOverflow)?.to_bits().div_down(BITS_ONE)? as u128)
            }
        }
    }

    fn pow_up(self, rhs: Self) -> Result<Self, MiniStabbleError> {
        match rhs {
            ZERO => Ok(ONE),
            ONE => Ok(self),
            TWO => self.mul_up(self),
            THREE => self.mul_up(self)?.mul_up(self),
            FOUR => {
                let square = self.mul_up(self)?;
                square.mul_up(square)
            }
            _ => {
                let base = U34F30::from_bits((self as u64).mul_up(BITS_ONE)?);
                let exp = U34F30::from_bits((rhs as u64).mul_up(BITS_ONE)?);
                Ok(base.powf(exp).ok_or(MiniStabbleError::MathOverflow)?.to_bits().div_up(BITS_ONE)? as u128)
            }
        }
    }
}

pub const ONE_U64: u64 = 1_000_000_000; // 10^9
impl FixedMul for u64 {
    fn mul_down(self, other: Self) -> Result<Self, MiniStabbleError> {
        (self as u128)
            .checked_mul(other as u128)
            .and_then(|v| v.checked_div(ONE_U64 as u128))
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(MiniStabbleError::MathOverflow)
    }
    fn mul_up(self, other: Self) -> Result<Self, MiniStabbleError> {
        let product = (self as u128)
            .checked_mul(other as u128)
            .ok_or(MiniStabbleError::MathOverflow)?;

        product
            .checked_add(ONE_U64 as u128 - 1)
            .and_then(|v| v.checked_div(ONE_U64 as u128))
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(MiniStabbleError::MathOverflow)
    }
}
impl FixedDiv for u64 {
    fn div_down(self, other: Self) -> Result<Self, MiniStabbleError> {
        if other == 0 {
            return Err(MiniStabbleError::DivideByZero);
        }
        (self as u128)
            .checked_mul(ONE_U64 as u128)
            .and_then(|v| v.checked_div(other as u128))
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(MiniStabbleError::MathOverflow)
    }
    fn div_up(self, other: Self) -> Result<Self, MiniStabbleError> {
        if other == 0 {
            return Err(MiniStabbleError::DivideByZero);
        }
        let numerator = (self as u128)
            .checked_mul(ONE_U64 as u128)
            .ok_or(MiniStabbleError::MathOverflow)?;

        numerator
            .checked_add(other as u128 - 1)
            .and_then(|v| v.checked_div(other as u128))
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(MiniStabbleError::MathOverflow)
    }
}
impl FixedComplement for u64 {
    fn complement(self) -> Self {
        ONE_U64.saturating_sub(self)
    }
}
