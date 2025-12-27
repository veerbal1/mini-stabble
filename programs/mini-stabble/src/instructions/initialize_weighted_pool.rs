use std::cmp::max;

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    constants::{AUTHORITY, POOL_VAULT, WEIGHT_POOL}, errors::MiniStabbleError, math::fixed::{ONE_U64}, state::{PoolToken, WeightedPool}
};

#[derive(Accounts)]
pub struct InitializeWeightedPool<'info> {
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
        seeds = [WEIGHT_POOL, lp_mint.key().as_ref()], 
        bump, 
        payer = payer, 
        space = WeightedPool::LEN
    )]
    pub pool: Account<'info, WeightedPool>,

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

pub fn handler(ctx: Context<InitializeWeightedPool>, swap_fee: u64, only_token_a_weight: u64,) -> Result<()> {
    let pool: &mut Account<'_, WeightedPool> = &mut ctx.accounts.pool;
    
    require!(only_token_a_weight < ONE_U64, MiniStabbleError::InvalidWeight);
    require!(only_token_a_weight > 0, MiniStabbleError::InvalidWeight);
    require!(swap_fee < ONE_U64, MiniStabbleError::InvalidAmount);

    let max_decimal = max(ctx.accounts.token_mint_a.decimals, ctx.accounts.token_mint_b.decimals);

    let pool_token_a = PoolToken {
        mint: ctx.accounts.token_mint_a.key(),
        token_account: ctx.accounts.vault_token_a.key(),
        decimals: ctx.accounts.token_mint_a.decimals,
        scaling_factor: 10_u64.pow((max_decimal - ctx.accounts.token_mint_a.decimals) as u32),
        balance: ctx.accounts.vault_token_a.amount,
        weight: only_token_a_weight
    };

    let pool_token_b = PoolToken {
        mint: ctx.accounts.token_mint_b.key(),
        token_account: ctx.accounts.vault_token_b.key(),
        decimals: ctx.accounts.token_mint_b.decimals,
        scaling_factor: 10_u64.pow((max_decimal - ctx.accounts.token_mint_b.decimals) as u32),
        balance: ctx.accounts.vault_token_b.amount,
        weight: ONE_U64.checked_sub(only_token_a_weight).unwrap()
    };

    pool.authority = ctx.accounts.authority.key();
    pool.lp_mint = ctx.accounts.lp_mint.key();
    pool.is_active = true;
    pool.invariant = 0;
    pool.swap_fee = swap_fee;
    pool.tokens = vec![pool_token_a, pool_token_b];
    pool.bump = ctx.bumps.pool;
    
    Ok(())
}