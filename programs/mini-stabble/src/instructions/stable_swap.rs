use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::{AUTHORITY, POOL_VAULT, STABLE_POOL},
    errors::MiniStabbleError,
    math::{
        fixed::{FixedMul, SCALE},
        stable::calc_out_given_in,
    },
    state::StablePool,
};

#[derive(Accounts)]
pub struct StableSwap<'info> {
    /// CHECK: Unchecked
    #[account(seeds = [AUTHORITY], bump)]
    pub authority: UncheckedAccount<'info>,

    /// Pool - derived from LP mint
    #[account(
            mut, 
            seeds = [STABLE_POOL, pool.lp_mint.key().as_ref()], 
            bump,
    )]
    pub pool: Account<'info, StablePool>,

    #[account(constraint = mint_in.key() != mint_out.key())]
    pub mint_in: Account<'info, Mint>,
    pub mint_out: Account<'info, Mint>,

    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), mint_in.key().as_ref()], bump, token::mint = mint_in, token::authority = authority)]
    pub vault_token_in: Account<'info, TokenAccount>,

    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), mint_out.key().as_ref()], bump, token::mint = mint_out, token::authority = authority)]
    pub vault_token_out: Account<'info, TokenAccount>,

    #[account(mut, token::mint = mint_in, token::authority = user)]
    pub user_token_in: Account<'info, TokenAccount>,

    #[account(mut, token::mint = mint_out, token::authority = user)]
    pub user_token_out: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<StableSwap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    // Check if pool is active
    require!(pool.is_active, MiniStabbleError::PoolInActive);
    require!(amount_in > 0, MiniStabbleError::InvalidAmount);
    require!(min_amount_out > 0, MiniStabbleError::InvalidAmount);

    let mint_in = &ctx.accounts.mint_in;
    let mint_out = &ctx.accounts.mint_out;
    let token_in_index = pool
        .get_token_index(&mint_in.key())
        .ok_or(MiniStabbleError::InvalidMint)?;
    let token_out_index = pool
        .get_token_index(&mint_out.key())
        .ok_or(MiniStabbleError::InvalidMint)?;

    let scaled_amount_in = pool.tokens[token_in_index].scale_amount_up(amount_in);

    let amp = pool.amp;

    let amount_out_scaled = calc_out_given_in(
        amp,
        &pool.get_balances(),
        token_in_index,
        token_out_index,
        scaled_amount_in,
    )
    .ok_or(MiniStabbleError::InvalidAmount)? as u128;

    // amount_out * (1 - fee/scale) -> amount_out * ((scale - fee)/scale)
    let scaled_amount_out_after_fee = u64::try_from(
        amount_out_scaled.mul_down(
            SCALE
                .checked_sub(pool.swap_fee as u128)
                .ok_or(MiniStabbleError::MathOverflow)?,
        )?,
    )?;

    require!(
        min_amount_out
            <= pool.tokens[token_out_index].scale_amount_down(scaled_amount_out_after_fee),
        MiniStabbleError::SlippageExceeded
    );

    // Amount In
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_in.to_account_info(),
                to: ctx.accounts.vault_token_in.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_in,
    )?;

    let seeds = [AUTHORITY, &[ctx.bumps.authority]];
    let signer_seeds = &[&seeds[..]];

    // Amount out
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_out.to_account_info(),
                to: ctx.accounts.user_token_out.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
            signer_seeds,
        ),
        pool.tokens[token_out_index].scale_amount_down(scaled_amount_out_after_fee),
    )?;

    // let amount_out_scaled = pool.tokens[token_out_index].scale_amount_down(scaled_amount)
    pool.tokens[token_in_index].balance = pool.tokens[token_in_index]
        .balance
        .checked_add(scaled_amount_in)
        .ok_or(MiniStabbleError::MathOverflow)?;

    pool.tokens[token_out_index].balance = pool.tokens[token_out_index]
        .balance
        .checked_sub(scaled_amount_out_after_fee)
        .ok_or(MiniStabbleError::MathOverflow)?;
    Ok(())
}
