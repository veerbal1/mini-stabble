use bn::{
    safe_math::{CheckedDivCeil, CheckedMulDiv, Downcast},
    uint192, U192,
};

pub const AMP_PRECISION: u64 = 1_000;
pub const MIN_AMP: u64 = 1;
pub const MAX_AMP: u64 = 10_000;
pub const MAX_LOOP_LIMIT: u64 = 256;

// Convergence thresholds
pub const DEFAULT_INV_THRESHOLD: u64 = 100;
pub const BALANCE_THRESHOLD: u64 = 1;

#[inline(always)]
fn amp_precision_u192() -> U192 {
    uint192!(AMP_PRECISION)
}

/// Calculates the StableSwap invariant D using Newton-Raphson iteration.
/// Matches reference: libraries/math/src/stable_math.rs calc_invariant
pub fn calc_invariant(amp: u64, balances: &[u64]) -> Option<u64> {
    let sum: u64 = balances.iter().sum();

    if sum == 0 {
        return Some(0);
    }

    let num_tokens = balances.len();
    let num_tokens_u64 = num_tokens as u64;
    let num_tokens_u192 = uint192!(num_tokens_u64);

    let amp_times_total = amp.checked_mul(num_tokens_u64)?; // Ann

    let sum = uint192!(sum);
    let mut invariant = sum; // D

    // Precompute balances[i] * num_tokens
    let mut balances_times: Vec<U192> = Vec::with_capacity(balances.len());
    for &balance in balances.iter() {
        balances_times.push(uint192!(balance.checked_mul(num_tokens_u64)?));
    }

    for _ in 0..64 {
        let mut p = invariant;

        for &balance_times in balances_times.iter() {
            // (p * invariant) / (balances[i] * num_tokens)
            p = p.checked_mul_div_down(invariant, balance_times)?;
        }

        let prev_invariant = invariant;

        // Newton-Raphson formula:
        // D = (Ann * S / AMP_PRECISION + n * D_P) * D / ((Ann - AMP_PRECISION) * D / AMP_PRECISION + (n + 1) * D_P)
        invariant = (uint192!(amp_times_total)
            .checked_mul_div_down(sum, amp_precision_u192())?
            .checked_add(p.checked_mul(num_tokens_u192)?))?
        .checked_mul_div_down(
            invariant,
            uint192!(amp_times_total.checked_sub(AMP_PRECISION)?)
                .checked_mul_div_down(invariant, amp_precision_u192())?
                .checked_add(uint192!(num_tokens.saturating_add(1)).checked_mul(p)?)?,
        )?;

        let invariant_u64 = invariant.as_u64()?;
        let prev_invariant_u64 = prev_invariant.as_u64()?;

        if invariant_u64 > prev_invariant_u64 {
            if invariant_u64.saturating_sub(prev_invariant_u64) <= DEFAULT_INV_THRESHOLD {
                return Some(invariant_u64);
            }
        } else if prev_invariant_u64.saturating_sub(invariant_u64) <= DEFAULT_INV_THRESHOLD {
            return Some(invariant_u64);
        }
    }

    None
}

/// Calculates the balance of a token given the invariant and all other balances.
/// This is the core function for swap calculations.
/// Matches reference: get_token_balance_given_invariant_n_all_other_balances
fn get_token_balance_given_invariant_and_others(
    amp: u64,
    balances: &[u64],
    invariant: u64,
    token_index: usize,
) -> Option<u64> {
    let num_tokens = balances.len() as u64;
    let amp_times_total = uint192!(amp.checked_mul(num_tokens)?);

    let invariant = uint192!(invariant);

    // Calculate sum and product of ALL balances (including token_index for now)
    let mut sum = balances[0];
    let mut p = uint192!(balances[0].checked_mul(num_tokens)?);

    for i in 1..balances.len() {
        let p_i = uint192!(balances[i].checked_mul(num_tokens)?);
        p = p.checked_mul_div_down(p_i, invariant)?;
        sum = sum.checked_add(balances[i])?;
    }

    // Remove the balance at token_index from sum
    let balance = balances[token_index];
    sum = sum.saturating_sub(balance);
    let sum = uint192!(sum);

    let invariant_2 = invariant.checked_mul(invariant)?;

    // c = D² * AMP_PRECISION / (Ann * P) * balance
    // We multiply by balance to "remove" it from P
    let c = invariant_2
        .checked_mul_div_up(amp_precision_u192(), amp_times_total.checked_mul(p)?)?
        .checked_mul(uint192!(balance))?;

    // b = D * AMP_PRECISION / Ann + sum
    let b = invariant
        .checked_mul_div_down(amp_precision_u192(), amp_times_total)?
        .checked_add(sum)?;

    // Initial approximation: (D² + c) / (D + b)
    let mut token_balance = invariant_2
        .checked_add(c)?
        .checked_div_up(invariant.checked_add(b)?)?;

    // Newton-Raphson iteration: y = (y² + c) / (2y + b - D)
    for _ in 0..64 {
        let prev_token_balance = token_balance;

        token_balance = token_balance
            .checked_mul(token_balance)?
            .checked_add(c)?
            .checked_div_up(
                (token_balance << 1)
                    .checked_add(b)?
                    .checked_sub(invariant)?,
            )?;

        let token_balance_u64 = token_balance.as_u64()?;
        let prev_token_balance_u64 = prev_token_balance.as_u64()?;

        if token_balance_u64 > prev_token_balance_u64 {
            if token_balance_u64.saturating_sub(prev_token_balance_u64) <= BALANCE_THRESHOLD {
                return Some(token_balance_u64);
            }
        } else if prev_token_balance_u64.saturating_sub(token_balance_u64) <= BALANCE_THRESHOLD {
            return Some(token_balance_u64);
        }
    }

    None
}

/// Calculates how many tokens can be taken out of a pool if `amount_in` are sent.
pub fn calc_out_given_in(
    amp: u64,
    balances: &[u64],
    token_index_in: usize,
    token_index_out: usize,
    amount_in: u64,
) -> Option<u64> {
    // Calculate invariant first
    let invariant = calc_invariant(amp, balances)?;

    // Create new balances with amount_in added
    let mut new_balances = balances.to_vec();
    new_balances[token_index_in] = new_balances[token_index_in].checked_add(amount_in)?;

    let balance_out = balances[token_index_out];

    // Calculate what the output token balance should be
    let final_balance_out = get_token_balance_given_invariant_and_others(
        amp,
        &new_balances,
        invariant,
        token_index_out,
    )?;

    // Output = current_balance - final_balance - 1 (for rounding protection)
    balance_out.checked_sub(final_balance_out)?.checked_sub(1)
}

/// Calculates how many tokens must be sent to get `amount_out`.
pub fn calc_in_given_out(
    amp: u64,
    balances: &[u64],
    token_index_in: usize,
    token_index_out: usize,
    amount_out: u64,
) -> Option<u64> {
    // Calculate invariant first
    let invariant = calc_invariant(amp, balances)?;

    // Create new balances with amount_out subtracted
    let mut new_balances = balances.to_vec();
    new_balances[token_index_out] = new_balances[token_index_out].checked_sub(amount_out)?;

    let balance_in = balances[token_index_in];

    // Calculate what the input token balance should be
    let final_balance_in = get_token_balance_given_invariant_and_others(
        amp,
        &new_balances,
        invariant,
        token_index_in,
    )?;

    // Input = final_balance - current_balance + 1 (for rounding protection)
    final_balance_in.checked_sub(balance_in)?.checked_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_invariant_basic() {
        // Test case from reference: amp=5_000_000, balances=[40M, 60M]
        let amp = 5_000_000;
        let balances = vec![40_000_000_000_000_000_u64, 60_000_000_000_000_000_u64];

        let result = calc_invariant(amp, &balances);

        assert!(result.is_some(), "calc_invariant should return Some");

        let d = result.unwrap();
        println!("D = {}", d);

        // Expected value from reference implementation
        let expected = 99999583421855646_u64;

        assert_eq!(
            d, expected,
            "D should match reference. Got {}, expected {}",
            d, expected
        );
    }

    #[test]
    fn test_calc_out_given_in() {
        // Test case from reference
        let amp = 5_000_000;
        let balances = vec![894_520_800_000_000_u64, 467_581_800_000_000_u64];

        let invariant = calc_invariant(amp, &balances).unwrap();
        println!("D = {}", invariant);

        // Swap 1 trillion units of token 0 for token 1
        let amount_in = 1_000_000_000_000_u64;
        let result = calc_out_given_in(amp, &balances, 0, 1, amount_in);

        println!("calc_out_given_in result: {:?}", result);

        let amount_out = result.unwrap();
        println!("Swapping {} token0 -> {} token1", amount_in, amount_out);

        // Reference expects: 999845351779 (nearly 1:1 for stables)
        let expected = 999845351779_u64;
        assert_eq!(
            amount_out, expected,
            "Output should match reference. Got {}, expected {}",
            amount_out, expected
        );
    }

    #[test]
    fn test_calc_out_given_in_smaller_amounts() {
        let amp = 5_000_000;
        let balances = vec![894_520_800_000_000_u64, 467_581_800_000_000_u64];

        // Test with 1 billion
        let amount_in = 1_000_000_000_u64;
        let amount_out = calc_out_given_in(amp, &balances, 0, 1, amount_in).unwrap();
        println!("1B swap: {} -> {}", amount_in, amount_out);
        assert_eq!(amount_out, 999845869, "1B swap should match reference");

        // Test with 1 million
        let amount_in = 1_000_000_u64;
        let amount_out = calc_out_given_in(amp, &balances, 0, 1, amount_in).unwrap();
        println!("1M swap: {} -> {}", amount_in, amount_out);
        assert_eq!(amount_out, 999845, "1M swap should match reference");
    }

    #[test]
    fn test_calc_in_given_out() {
        let amp = 5_000_000;
        let balances = vec![894_520_800_000_000_u64, 467_581_800_000_000_u64];

        // Want 100 billion units of token 1
        let amount_out = 100_000_000_000_u64;
        let result = calc_in_given_out(amp, &balances, 0, 1, amount_out);

        println!("calc_in_given_out result: {:?}", result);

        let amount_in = result.unwrap();
        println!("To get {} token1, need {} token0", amount_out, amount_in);

        // Should need roughly the same amount in (near 1:1 for stables)
        assert!(amount_in > 0, "Should need some input");
        // For high-amp stableswap, input should be close to output
        assert!(
            amount_in < amount_out * 2,
            "Input shouldn't be more than 2x output for stableswap"
        );
    }

    #[test]
    fn test_calc_invariant_three_tokens() {
        // Test case from reference: 3 tokens
        let amp = 750_000;
        let balances = vec![
            40_000_000_000_000_000_u64,
            50_000_000_000_000_000_u64,
            60_000_000_000_000_000_u64,
        ];

        let invariant = calc_invariant(amp, &balances).unwrap();
        let expected = 149997226126050479_u64;

        assert_eq!(
            invariant, expected,
            "3-token invariant should match reference. Got {}, expected {}",
            invariant, expected
        );
    }

    #[test]
    fn test_calc_invariant_four_tokens() {
        // Test case from reference: 4 tokens
        let amp = 150_000;
        let balances = vec![
            40_000_000_000_000_000_u64,
            50_000_000_000_000_000_u64,
            60_000_000_000_000_000_u64,
            70_000_000_000_000_000_u64,
        ];

        let invariant = calc_invariant(amp, &balances).unwrap();
        let expected = 219967475585041316_u64;

        assert_eq!(
            invariant, expected,
            "4-token invariant should match reference. Got {}, expected {}",
            invariant, expected
        );
    }
}
