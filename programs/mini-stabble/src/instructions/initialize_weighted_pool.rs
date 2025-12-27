use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::{
    constants::{AUTHORITY, WEIGHT_POOL},
    state::WeightedPool,
};

#[derive(Accounts)]
pub struct InitializeWeightedPool<'info> {
    #[account(seeds = [AUTHORITY], bump)]
    pub authority: UncheckedAccount<'info>,

    /// LP Mint - passed in as a new keypair by client
    #[account(
            init,
            payer = payer,
            mint::decimals = 9,
            mint::authority = authority,
        )]
    pub lp_mint: Account<'info, Mint>,

    /// Pool - derived from LP mint
    #[account(
        init, 
        seeds = [WEIGHT_POOL, lp_mint.key().as_ref()], 
        bump, 
        payer = payer, 
        space = WeightedPool::LEN
    )]
    pub pool: Account<'info, WeightedPool>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<InitializeWeightedPool>, swap_fee: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    
    pool.authority = ctx.accounts.authority.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.is_active = true;
    pool.invariant = 0;
    pool.swap_fee = swap_fee;
    pool.tokens = vec![];
    pool.bump = ctx.bumps.pool;
    
    Ok(())
}