use std::cmp::max;

use crate::{
    constants::{AUTHORITY, POOL_VAULT, STABLE_POOL},
    errors::MiniStabbleError,
    math::{
        fixed::ONE_U64,
        stable::{AMP_PRECISION, MAX_AMP, MIN_AMP},
    },
    state::{PoolToken, StablePool},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
pub struct InitializeStablePool<'info> {
    /// CHECK: Unchecked
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
            seeds = [STABLE_POOL, lp_mint.key().as_ref()], 
            bump, 
            payer = payer, 
            space = StablePool::LEN
    )]
    pub pool: Account<'info, StablePool>,

    // Vault Tokens Mint
    #[account(constraint = token_mint_a.key() < token_mint_b.key() @ MiniStabbleError::MintOrderInvalid)]
    pub token_mint_a: Account<'info, Mint>,
    pub token_mint_b: Account<'info, Mint>,

    // Tokens
    #[account(init, seeds=[POOL_VAULT, pool.key().as_ref(), token_mint_a.key().as_ref()], bump, payer = payer, token::mint = token_mint_a, token::authority = authority)]
    pub vault_token_a: Account<'info, TokenAccount>,

    #[account(init, seeds=[POOL_VAULT, pool.key().as_ref(), token_mint_b.key().as_ref()], bump, payer = payer, token::mint = token_mint_b, token::authority = authority)]
    pub vault_token_b: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<InitializeStablePool>, swap_fee: u64, amp: u64) -> Result<()> {
    // 1. Validate AMP
    require!(amp >= MIN_AMP, MiniStabbleError::AmpTooLow);
    require!(amp <= MAX_AMP, MiniStabbleError::AmpTooHigh);

    // 2. Validate swap_fee
    require!(swap_fee < ONE_U64, MiniStabbleError::InvalidAmount);

    // 3. Create PoolToken structs
    let max_decimal = max(
        ctx.accounts.token_mint_a.decimals,
        ctx.accounts.token_mint_b.decimals,
    );
    let pool_token_a = PoolToken {
        mint: ctx.accounts.token_mint_a.key(),
        token_account: ctx.accounts.vault_token_a.key(),
        decimals: ctx.accounts.token_mint_a.decimals,
        scaling_factor: 10_u64.pow((max_decimal - ctx.accounts.token_mint_a.decimals) as u32),
        balance: ctx.accounts.vault_token_a.amount,
        weight: 0,
    };

    let pool_token_b = PoolToken {
        mint: ctx.accounts.token_mint_b.key(),
        token_account: ctx.accounts.vault_token_b.key(),
        decimals: ctx.accounts.token_mint_b.decimals,
        scaling_factor: 10_u64.pow((max_decimal - ctx.accounts.token_mint_b.decimals) as u32),
        balance: ctx.accounts.vault_token_b.amount,
        weight: 0,
    };

    // 4. Set pool fields
    let pool = &mut ctx.accounts.pool;
    pool.authority = ctx.accounts.authority.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.is_active = true;
    pool.invariant = 0;
    pool.swap_fee = swap_fee;
    pool.tokens = vec![pool_token_a, pool_token_b];
    pool.bump = ctx.bumps.pool;

    // AMP Specific
    pool.amp = amp
        .checked_mul(AMP_PRECISION)
        .ok_or(MiniStabbleError::MathOverflow)?;
    pool.amp_target = pool.amp;

    pool.amp_start_ts = 0;
    pool.amp_end_ts = 0;

    Ok(())
}
