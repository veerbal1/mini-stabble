use bn::{
    safe_math::{CheckedMulDiv, Downcast},
    uint192, U192,
};

pub const AMP_PRECISION: u64 = 1000;
pub const MIN_AMP: u64 = 1;
pub const MAX_AMP: u64 = 10_000;
pub const MAX_LOOP_LIMIT: u64 = 256;

/// Calculates the StableSwap invariant D using Newton-Raphson iteration.
pub fn calc_invariant(amplification: u64, balances: &Vec<u64>) -> Option<u64> {
    // An^n(x + y) + D - An²D - D³/(4xy) = 0
    let sum: u64 = balances.iter().sum();
    let num_of_tokens = balances.len() as u64;

    let amp_times_total = amplification.checked_mul(num_of_tokens)?;

    let sum_u192 = uint192!(sum);
    let mut invariant = sum_u192;
    todo!()
}
