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
pub fn calc_invariant(amp: u64, balances: &[u64]) -> Option<u128> {
    if balances.is_empty() {
        return None;
    }
    if balances.iter().any(|&val| val == 0) {
        return None;
    }

    let sum: u128 = balances.iter().map(|&val| val as u128).sum();

    // Initial D
    let mut d = sum;
    // Newton's formula
    // d_new = (Ann × sum + n × D_P) × D / ((Ann - 1) × D + (n + 1) × D_P)

    let n = balances.len() as u128;
    let ann: u128 = n.checked_mul(amp as u128)?;
    let amp_precision = AMP_PRECISION as u128;

    for _ in 0..255 {
        // Calculate D_P (protection term)
        let mut d_p = d;
        for balance in balances {
            d_p = d_p
                .checked_mul(d)?
                .checked_div(n.checked_mul(*balance as u128)?)?;
        }
        // numerator = (an × sum + n × d_p) × d
        let numerator = (ann
            .checked_mul(sum)?
            .checked_div(amp_precision)?
            .checked_add(n.checked_mul(d_p)?)?)
        .checked_mul(d)?;

        // denominator = (an - 1) × d + (n + 1) × d_p
        let denominator = ((ann.checked_sub(amp_precision)?).checked_mul(d)?)
            .checked_div(amp_precision)?
            .checked_add(n.checked_add(1)?.checked_mul(d_p)?)?;

        // Calculate D_new using the formula
        let d_new = numerator.checked_div(denominator)?;

        // Check if D_new is close enough to D (converged)
        if (d_new).abs_diff(d) <= 1 {
            return Some(d_new);
        }
        // If not, set D = D_new and repeat
        d = d_new;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_calc_invariant_basic() {
        let amp = 5_000_000; // A = 5000 scaled by 1000
        let balances = vec![40_000_000_000_000_000_u64, 60_000_000_000_000_000_u64];

        let result = calc_invariant(amp, &balances);

        assert!(result.is_some(), "calc_invariant should return Some");

        let d = result.unwrap();
        println!("D = {}", d);

        // Expected value from reference implementation
        let expected = 99999583421855646_u128;

        // Should be very close (within 1% for now)
        let diff = if d > expected {
            d - expected
        } else {
            expected - d
        };
        let tolerance = expected / 100; // 1% tolerance

        assert!(
            diff <= tolerance,
            "D should be close to expected. Got {}, expected {}",
            d,
            expected
        );
    }
}
