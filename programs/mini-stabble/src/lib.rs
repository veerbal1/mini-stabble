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
}

#[derive(Accounts)]
pub struct Initialize {}
