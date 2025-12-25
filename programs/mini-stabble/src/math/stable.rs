use anchor_lang::require;
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
    // An^n × S + D = An^n × D + D^(n+1) / (n^n × P)
    let sum: u64 = balances.iter().sum();
    if sum == 0 {
        return Some(0);
    }

    let num_tokens = balances.len() as u64;
    let ann = amplification.checked_mul(num_tokens)?;
    let sum_u192 = uint192!(sum);

    let initial = sum_u192;

    // D_new = (Ann × S + n × D_P) × D / ((Ann - 1) × D + (n + 1) × D_P)


    Some(0)
}
