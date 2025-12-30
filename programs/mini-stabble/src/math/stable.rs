use crate::math::fixed::{FixedComplement, FixedDiv, FixedMul, ONE_U64};
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
    let n = balances.len() as u64;
    let sum: u64 = balances.iter().sum();

    if sum == 0 {
        return Some(0);
    }

    let ann = uint192!(amp.checked_mul(n)?);
    let sum_u192 = uint192!(sum);
    let n_u192 = uint192!(n);
    let mut d = uint192!(sum);

    for _ in 0..MAX_LOOP_LIMIT {
        let mut dp = d;
        for &balance in balances.iter() {
            dp = dp.checked_mul_div_down(d, n_u192.checked_mul(uint192!(balance))?)?;
        }

        // d_new = (Ann * S + n * D_P * AMP_PRECISION) * D / ((Ann - AMP_PRECISION) * D + (n + 1) * D_P * AMP_PRECISION)
        let amp_prec = uint192!(AMP_PRECISION);

        let num = ann
            .checked_mul(sum_u192)?
            .checked_add(n_u192.checked_mul(dp)?.checked_mul(amp_prec)?)?;

        let den = ann
            .checked_sub(amp_prec)?
            .checked_mul(d)?
            .checked_add(n_u192.checked_add(uint192!(1))?.checked_mul(dp)?.checked_mul(amp_prec)?)?;

        let d_new = num.checked_mul(d)?.checked_div(den)?;

        let diff = if d_new > d {
            d_new.checked_sub(d)?
        } else {
            d.checked_sub(d_new)?
        };
        if diff <= uint192!(DEFAULT_INV_THRESHOLD) {
            return d_new.as_u64();
        }
        d = d_new;
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

/// Calculates LP tokens for deposit (simple, no fees - for proportional deposits)
pub fn calc_lp_tokens_for_deposit_simple(
    amp: u64,
    balances: &[u64],
    amounts_in: &[u64],
    lp_supply: u64,
) -> Option<u64> {
    let current_d = calc_invariant(amp, balances)?;

    let mut new_balances = Vec::with_capacity(balances.len());
    for i in 0..balances.len() {
        new_balances.push(balances[i].checked_add(amounts_in[i])?);
    }

    let new_d = calc_invariant(amp, &new_balances)?;

    let lp_out = (lp_supply as u128)
        .checked_mul(new_d as u128)?
        .checked_div(current_d as u128)?
        .checked_sub(lp_supply as u128)?;

    u64::try_from(lp_out).ok()
}

/// Calculates LP tokens for imbalanced deposit (with swap fees)
/// Ring 2.10: LP token calculations for Stable pools
pub fn calc_lp_tokens_for_deposit_with_fee(
    amp: u64,
    balances: &[u64],
    amounts_in: &[u64],
    lp_supply: u64,
    current_invariant: u64,
    swap_fee: u64, // e.g., 3_000_000 = 0.3%
) -> Option<u64> {
    // Step 1: Calculate sum of all balances (for computing weights)
    let sum: u64 = balances.iter().sum();

    // Step 2: Calculate balance ratios and weighted average (ideal ratio)
    let mut balance_ratios = Vec::with_capacity(balances.len());
    let mut ideal_ratio: u64 = 0;

    for i in 0..balances.len() {
        // ratio = (balance + amount_in) / balance
        let new_balance = balances[i].checked_add(amounts_in[i])?;
        let ratio = new_balance.div_down(balances[i]).ok()?;
        balance_ratios.push(ratio);

        // weight = balance / sum
        let weight = balances[i].div_down(sum).ok()?;

        // ideal_ratio += ratio * weight
        ideal_ratio = ideal_ratio.checked_add(ratio.mul_down(weight).ok()?)?;
    }

    // Step 3: Calculate fee-adjusted amounts
    let mut new_balances = Vec::with_capacity(balances.len());

    for i in 0..balances.len() {
        let amount_in_without_fee;

        if balance_ratios[i] > ideal_ratio {
            // This token has excess deposit → taxable portion
            let non_taxable = balances[i]
                .mul_down(ideal_ratio.saturating_sub(ONE_U64))
                .ok()?;
            let taxable = amounts_in[i].saturating_sub(non_taxable);

            // Apply fee: taxable * (1 - swap_fee) + non_taxable
            amount_in_without_fee = taxable
                .mul_down(swap_fee.complement())
                .ok()?
                .checked_add(non_taxable)?;
        } else {
            // Below ideal ratio → no fee
            amount_in_without_fee = amounts_in[i];
        }

        new_balances.push(balances[i].checked_add(amount_in_without_fee)?);
    }

    // Step 4: Calculate new invariant with fee-adjusted balances
    let new_invariant = calc_invariant(amp, &new_balances)?;

    // Step 5: LP tokens = supply × (new_d / old_d - 1)
    let ratio = new_invariant.div_down(current_invariant).ok()?;

    if ratio > ONE_U64 {
        lp_supply.mul_down(ratio.saturating_sub(ONE_U64)).ok()
    } else {
        Some(0)
    }
}

/// Calculates tokens out when burning LP tokens (single-sided withdraw)
/// Ring 2.10: LP token calculations for Stable pools
pub fn calc_token_out_for_lp_burn(
    amp: u64,
    balances: &[u64],
    token_index: usize,
    lp_amount_in: u64,
    lp_supply: u64,
    current_invariant: u64,
    swap_fee: u64,
) -> Option<u64> {
    // Step 1: Calculate new invariant after burning LP
    // new_invariant = current_invariant × (supply - lp_burn) / supply
    let new_invariant = (current_invariant as u128)
        .checked_mul(lp_supply.checked_sub(lp_amount_in)? as u128)?
        .checked_div(lp_supply as u128)?;
    let new_invariant = u64::try_from(new_invariant).ok()?;

    let balance = balances[token_index];

    // Step 2: Calculate what the token balance should be at new invariant
    let new_balance =
        get_token_balance_given_invariant_and_others(amp, balances, new_invariant, token_index)?;

    // Step 3: Raw amount out (before fees)
    let amount_out_without_fee = balance.checked_sub(new_balance)?;

    // Step 4: Apply fees on the taxable portion
    let sum: u64 = balances.iter().sum();
    let current_weight = balance.div_down(sum).ok()?;
    let taxable_percentage = current_weight.complement();

    let taxable_amount = amount_out_without_fee.mul_up(taxable_percentage).ok()?;
    let non_taxable_amount = amount_out_without_fee.saturating_sub(taxable_amount);

    // Final amount = taxable * (1 - fee) + non_taxable
    taxable_amount
        .mul_down(swap_fee.complement())
        .ok()?
        .checked_add(non_taxable_amount)
}

/// Calculates proportional token amounts for a balanced withdraw
/// Ring 2.11: Proportional liquidity math
pub fn calc_tokens_out_proportional(
    balances: &[u64],
    lp_amount_in: u64,
    lp_supply: u64,
) -> Option<Vec<u64>> {
    let mut amounts_out = Vec::with_capacity(balances.len());

    for &balance in balances {
        // amount_out = balance × lp_amount / lp_supply
        let amount = (balance as u128)
            .checked_mul(lp_amount_in as u128)?
            .checked_div(lp_supply as u128)?;
        amounts_out.push(u64::try_from(amount).ok()?);
    }

    Some(amounts_out)
}

/// Calculates the required token amounts for a proportional deposit
/// Ring 2.11: Proportional liquidity math  
pub fn calc_tokens_in_proportional(
    balances: &[u64],
    lp_amount_out: u64,
    lp_supply: u64,
) -> Option<Vec<u64>> {
    let mut amounts_in = Vec::with_capacity(balances.len());

    for &balance in balances {
        // amount_in = balance × lp_amount / lp_supply (round up to be safe)
        let amount = (balance as u128)
            .checked_mul(lp_amount_out as u128)?
            .checked_add(lp_supply as u128 - 1)? // round up
            .checked_div(lp_supply as u128)?;
        amounts_in.push(u64::try_from(amount).ok()?);
    }

    Some(amounts_in)
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

    #[test]
    fn test_calc_lp_tokens_for_deposit_simple() {
        let amp = 5_000_000;
        let balances = vec![1_000_000_000_000_u64, 1_000_000_000_000_u64]; // 1T each
        let lp_supply = 2_000_000_000_000_u64; // 2T LP tokens

        // Balanced deposit: 100B each token
        let amounts_in = vec![100_000_000_000_u64, 100_000_000_000_u64];

        let lp_out =
            calc_lp_tokens_for_deposit_simple(amp, &balances, &amounts_in, lp_supply).unwrap();

        println!("Balanced deposit: depositing 100B + 100B");
        println!("LP tokens received: {}", lp_out);

        // Should be ~10% of supply since we added 10% more tokens
        // Expected: ~200B LP tokens (10% of 2T)
        assert!(
            lp_out > 190_000_000_000,
            "Should get roughly 10% of LP supply"
        );
        assert!(
            lp_out < 210_000_000_000,
            "Should get roughly 10% of LP supply"
        );
    }

    #[test]
    fn test_calc_lp_tokens_with_fee_imbalanced() {
        let amp = 5_000_000;
        let balances = vec![1_000_000_000_000_u64, 1_000_000_000_000_u64];
        let lp_supply = 2_000_000_000_000_u64;
        let swap_fee = 3_000_000_u64; // 0.3%

        let current_invariant = calc_invariant(amp, &balances).unwrap();

        // Imbalanced deposit: 200B of token 0, 0 of token 1
        let amounts_in = vec![200_000_000_000_u64, 0_u64];

        let lp_out_with_fee = calc_lp_tokens_for_deposit_with_fee(
            amp,
            &balances,
            &amounts_in,
            lp_supply,
            current_invariant,
            swap_fee,
        )
        .unwrap();

        let lp_out_simple =
            calc_lp_tokens_for_deposit_simple(amp, &balances, &amounts_in, lp_supply).unwrap();

        println!("Imbalanced deposit: 200B + 0");
        println!("LP with fee: {}", lp_out_with_fee);
        println!("LP without fee: {}", lp_out_simple);

        // With fee should be less than without fee
        assert!(
            lp_out_with_fee < lp_out_simple,
            "Fee should reduce LP tokens"
        );
        assert!(lp_out_with_fee > 0, "Should still get some LP tokens");
    }

    #[test]
    fn test_calc_proportional_withdraw() {
        let balances = vec![1_000_000_000_000_u64, 2_000_000_000_000_u64];
        let lp_supply = 3_000_000_000_000_u64;

        // Withdraw 10% of LP
        let lp_amount = 300_000_000_000_u64;

        let amounts_out = calc_tokens_out_proportional(&balances, lp_amount, lp_supply).unwrap();

        println!("Proportional withdraw: {} LP", lp_amount);
        println!("Token 0 out: {}", amounts_out[0]);
        println!("Token 1 out: {}", amounts_out[1]);

        // Should get 10% of each token
        assert_eq!(amounts_out[0], 100_000_000_000, "Should get 10% of token 0");
        assert_eq!(amounts_out[1], 200_000_000_000, "Should get 10% of token 1");
    }

    #[test]
    fn test_calc_proportional_deposit() {
        let balances = vec![1_000_000_000_000_u64, 2_000_000_000_000_u64];
        let lp_supply = 3_000_000_000_000_u64;

        // Want 10% more LP tokens
        let lp_amount = 300_000_000_000_u64;

        let amounts_in = calc_tokens_in_proportional(&balances, lp_amount, lp_supply).unwrap();

        println!("Proportional deposit for {} LP", lp_amount);
        println!("Token 0 needed: {}", amounts_in[0]);
        println!("Token 1 needed: {}", amounts_in[1]);

        // Should need 10% of each token (rounded up)
        assert!(
            amounts_in[0] >= 100_000_000_000,
            "Should need ~10% of token 0"
        );
        assert!(
            amounts_in[1] >= 200_000_000_000,
            "Should need ~10% of token 1"
        );
    }
}
