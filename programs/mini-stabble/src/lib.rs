use anchor_lang::prelude::*;
use instructions::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod state;

declare_id!("FURtuxyXWgpnETkNho8PL6mpuRh9mCnVsWgUY14JzusX");

#[program]
pub mod mini_stabble {
    use super::*;

    pub fn initialize_weighted_pool(
        ctx: Context<InitializeWeightedPool>,
        swap_fee: u64,
        only_token_a_weight: u64,
    ) -> Result<()> {
        instructions::initialize_weighted_pool::handler(ctx, swap_fee, only_token_a_weight)?;
        Ok(())
    }

    pub fn swap(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
        instructions::swap::handler(ctx, amount_in, min_amount_out)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        lp_amount: u64,
        input_token_a_amount: u64,
        input_token_b_amount: u64,
    ) -> Result<()> {
        instructions::deposit::handler(ctx, lp_amount, input_token_a_amount, input_token_b_amount)
    }

    pub fn deposit_unbalanced(
        ctx: Context<DepositUnbalanced>,
        min_lp_amount: u64,
        input_amount_a: u64,
        input_amount_b: u64,
    ) -> Result<()> {
        instructions::deposit_unbalanced::handler(
            ctx,
            min_lp_amount,
            input_amount_a,
            input_amount_b,
        )
    }

    pub fn initialize_stable_pool(
        ctx: Context<InitializeStablePool>,
        swap_fee: u64,
        amp: u64,
    ) -> Result<()> {
        instructions::initialize_stable_pool::handler(ctx, swap_fee, amp)
    }

    pub fn stable_deposit(
        ctx: Context<StableDeposit>,
        max_amount_a: u64,
        max_amount_b: u64,
        lp_amount: u64,
    ) -> Result<()> {
        instructions::stable_deposit::handler(ctx, max_amount_a, max_amount_b, lp_amount)
    }
}
