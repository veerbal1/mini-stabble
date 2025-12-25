use bn::{uint192, U192, safe_math::{CheckedMulDiv, Downcast}};

pub const AMP_PRECISION: u64 = 1000;
pub const MIN_AMP: u64 = 1;
pub const MAX_AMP: u64 = 10_000;
pub const MAX_LOOP_LIMIT: u64 = 256;

/// Calculates the StableSwap invariant D using Newton-Raphson iteration.
pub fn calc_invariant(amplification: u64, balances: &Vec<u64>) -> Option<u64> {
    // We'll fill this in together
    todo!()
}
