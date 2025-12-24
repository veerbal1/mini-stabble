use crate::{
    errors::MiniStabbleError,
    math::fixed::{FixedMul, FixedPow, ONE},
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
