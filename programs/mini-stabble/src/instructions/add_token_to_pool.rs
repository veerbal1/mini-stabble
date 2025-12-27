use crate::{
    constants::{POOL_VAULT, WEIGHT_POOL},
    state::{PoolToken, WeightedPool},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
#[derive(Accounts)]
pub struct AddTokenToPool<'info> {
    #[account(
        mut,
        seeds = [WEIGHT_POOL, pool.lp_mint.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, WeightedPool>,

    /// Token mint being added
    pub token_mint: Account<'info, Mint>,

    /// Token vault (PDA) - created for this pool+token
    #[account(
        init,
        payer = payer,
        seeds = [POOL_VAULT, pool.key().as_ref(), token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = pool,  // Pool PDA owns the vault
    )]
    pub token_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<AddTokenToPool>,
    weight: u64,
    scaling_factor: u64,
    scaling_up: bool,
) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    let token_mint = &ctx.accounts.token_mint;
    let token_vault = &ctx.accounts.token_vault;

    // Create PoolToken entry
    let pool_token = PoolToken {
        mint: token_mint.key(),
        token_account: token_vault.key(),
        decimals: token_mint.decimals,
        scaling_up,
        scaling_factor,
        balance: 0,
        weight,
    };

    pool.tokens.push(pool_token);

    Ok(())
}
