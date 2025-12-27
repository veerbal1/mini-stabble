use crate::errors::MiniStabbleError;

pub const SCALE: u128 = 1_000_000_000;

pub const ONE: u128 = SCALE;
pub const TWO: u128 = 2 * SCALE;
pub const FOUR: u128 = 4 * SCALE;

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
    fn pow_down(self, exp: Self) -> Result<Self, MiniStabbleError> {
        match exp {
            0 => Ok(ONE),
            ONE => Ok(self),
            TWO => self.mul_down(self),
            FOUR => {
                // x^4 = (x^2)^2
                let squared = self.mul_down(self)?;
                squared.mul_down(squared)
            }
            _ => Err(MiniStabbleError::MathOverflow),
        }
    }

    fn pow_up(self, exp: Self) -> Result<Self, MiniStabbleError> {
        match exp {
            0 => Ok(ONE),
            ONE => Ok(self),
            TWO => self.mul_up(self),
            FOUR => {
                let squared = self.mul_up(self)?;
                squared.mul_up(squared)
            }
            _ => Err(MiniStabbleError::MathOverflow),
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
