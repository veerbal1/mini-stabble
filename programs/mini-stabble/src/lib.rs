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
    ) -> Result<()> {
        instructions::initialize_weighted_pool::handler(ctx, swap_fee)?;
        Ok(())
    }

    pub fn add_token_to_pool(
        ctx: Context<AddTokenToPool>,
        weight: u64,
        scaling_factor: u64,
        scaling_up: bool,
    ) -> Result<()> {
        instructions::add_token_to_pool::handler(ctx, weight, scaling_factor, scaling_up)
    }
}

#[derive(Accounts)]
pub struct Initialize {}
