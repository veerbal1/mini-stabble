use crate::{
    errors::MiniStabbleError,
    math::fixed::{FixedComplement, FixedDiv, FixedMul, FixedPow, ONE},
};

pub fn calc_invariant(balances: &[u128], weights: &[u128]) -> Result<u128, MiniStabbleError> {
    if balances.len() != weights.len() || balances.len() == 0 {
        return Err(MiniStabbleError::InvalidAmount);
    }

    let mut invariant = ONE;

    for (index, balance) in balances.iter().enumerate() {
        let result = balance.pow_down(weights[index])?;
        invariant = invariant.mul_down(result)?;
    }

    if invariant > 0 {
        return Ok(invariant);
    } else {
        return Err(MiniStabbleError::InvalidAmount);
    }
}

/// Calculate output amount given input amount for weighted pool swap.
///
/// Formula: amount_out = balance_out × (1 - (balance_in / (balance_in + amount_in))^(weight_in / weight_out))
///
/// ROUNDING STRATEGY (favor the pool, user receives LESS):
/// - Final result should be SMALLER → use mul_down at the end
/// - To make final result smaller, complement (1 - power) should be smaller
/// - To make complement smaller, power should be LARGER
/// - To make power larger with base < 1:
///   - base should be LARGER → use div_up
///   - exponent should be SMALLER → use div_down
/// - power itself → use pow_up
pub fn calc_out_given_in(
    balance_in: u128,
    weight_in: u128,
    balance_out: u128,
    weight_out: u128,
    amount_in: u128,
) -> Result<u128, MiniStabbleError> {
    // Step 1: base = balance_in / (balance_in + amount_in)
    // base < 1 always. Larger base → larger power → smaller complement → less output
    // Round UP to get larger base
    let base = balance_in.div_up(
        balance_in
            .checked_add(amount_in)
            .ok_or(MiniStabbleError::MathOverflow)?,
    )?;

    // Step 2: exponent = weight_in / weight_out
    // For base < 1: smaller exponent → larger power → smaller complement → less output
    // Round DOWN to get smaller exponent
    let exponent = weight_in.div_down(weight_out)?;

    // Step 3: power = base ^ exponent
    // Larger power → smaller complement → less output
    // Round UP to get larger power
    let power = base.pow_up(exponent)?;

    // Step 4: complement = 1 - power
    // No rounding choice here, just subtraction
    let complement = power.complement();

    // Step 5: amount_out = balance_out × complement
    // User receives this amount, round DOWN to give them less (favor pool)
    let amount_out = balance_out.mul_down(complement)?;

    Ok(amount_out)
}

// ROUNDING STRATEGY (favor pool, user PAYS MORE):
// amount_in should be LARGER
// amount_in will be larger when: mul_up AND complement larger
// complement (power - 1) larger when: power larger
// power larger when: pow_up, base larger, exponent larger
//   - For base > 1: larger exponent → larger power
//   - exponent larger → div_up
//   - base larger → div_up
pub fn calc_in_given_out(
    balance_in: u128,
    weight_in: u128,
    balance_out: u128,
    weight_out: u128,
    amount_out: u128,
) -> Result<u128, MiniStabbleError> {
    let base = balance_out.div_up(
        balance_out
            .checked_sub(amount_out)
            .ok_or(MiniStabbleError::MathOverflow)?,
    )?;

    let exponent = weight_out.div_up(weight_in)?;

    let power = base.pow_up(exponent)?;

    let complement = power
        .checked_sub(ONE)
        .ok_or(MiniStabbleError::MathOverflow)?;

    let amount_in = balance_in.mul_up(complement)?;

    return Ok(amount_in);
}

pub fn calc_lp_to_mint(
    lp_supply: u128,
    k_new: u128,
    k_old: u128,
    sum_of_weights: u128,
) -> Result<u128, MiniStabbleError> {
    // lp minted = lp supply * [((k_new / k_old) ^ sum of weights) - 1]
    let base: u128 = k_new
        .div_down(k_old)?;
    let base_pow = base.pow_down(sum_of_weights)?;

    let right = base_pow
        .checked_sub(ONE)
        .ok_or(MiniStabbleError::MathOverflow)?;

    let net_minted = lp_supply
        .mul_down(right)?;

    Ok(net_minted)
}
