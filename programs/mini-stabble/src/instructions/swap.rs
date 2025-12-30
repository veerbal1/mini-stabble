use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::{AUTHORITY, POOL_VAULT, WEIGHT_POOL},
    errors::MiniStabbleError,
    math::{
        fixed::{FixedComplement, FixedMul},
        weighted::calc_out_given_in,
    },
    state::WeightedPool,
};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(
        mut,
        seeds = [WEIGHT_POOL, pool.lp_mint.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, WeightedPool>,

    /// CHECK: Unchecked
    #[account(seeds = [AUTHORITY], bump)]
    pub authority: UncheckedAccount<'info>,

    #[account(constraint = mint_in.key() != mint_out.key())]
    pub mint_in: Account<'info, Mint>,
    pub mint_out: Account<'info, Mint>,

    #[account(mut, token::mint = mint_in, token::authority = user)]
    pub user_token_in: Account<'info, TokenAccount>,

    #[account(mut, token::mint = mint_out, token::authority = user)]
    pub user_token_out: Account<'info, TokenAccount>,

    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), mint_in.key().as_ref()], bump, constraint = vault_token_in.mint == mint_in.key(), token::authority = authority)]
    pub vault_token_in: Account<'info, TokenAccount>,

    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), mint_out.key().as_ref()], bump, constraint = vault_token_out.mint == mint_out.key(), token::authority = authority)]
    pub vault_token_out: Account<'info, TokenAccount>,

    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    // Step 1 starts
    require!(pool.is_active, MiniStabbleError::PoolInActive);

    let mint_in = ctx.accounts.mint_in.key();
    let mint_out = ctx.accounts.mint_out.key();

    let token_0_index = pool
        .get_token_index(&mint_in)
        .ok_or(MiniStabbleError::InvalidMint)?;
    let token_1_index = pool
        .get_token_index(&mint_out)
        .ok_or(MiniStabbleError::InvalidMint)?;

    require!(amount_in > 0, MiniStabbleError::InvalidAmount);
    require!(min_amount_out > 0, MiniStabbleError::InvalidAmount);
    // Step 1 ends

    // Step 2 starts
    let token_in_balance = pool.tokens[token_0_index].balance;
    let token_in_weight = pool.tokens[token_0_index].weight;
    let token_out_balance = pool.tokens[token_1_index].balance;
    let token_out_weight = pool.tokens[token_1_index].weight;

    let swap_fee = pool.swap_fee;
    // Step 2 ends

    // Step 3 starts - Calculate amount out
    let amount_out_without_fee = calc_out_given_in(
        token_in_balance.into(),
        token_in_weight.into(),
        token_out_balance.into(),
        token_out_weight.into(),
        amount_in.into(),
    )?;
    // Step 3 end - Calculate amount out

    // Step 4 starts - Apply fee
    let amount_out_after_fee = amount_out_without_fee.mul_down(swap_fee.complement() as u128)?;
    // Step 4 ends - Apply fee

    // Step 5 starts - Slippage check
    require!(
        amount_out_after_fee >= u128::from(min_amount_out),
        MiniStabbleError::SlippageExceeded
    );
    // Step 5 ends - Slippage Check

    // Step 6 starts -  Transfer Tokens
    let cpi_accounts_in = Transfer {
        from: ctx.accounts.user_token_in.to_account_info(),
        to: ctx.accounts.vault_token_in.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx_in = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts_in,
    );
    token::transfer(cpi_ctx_in, amount_in)?;

    let cpi_accounts_out = Transfer {
        from: ctx.accounts.vault_token_out.to_account_info(),
        to: ctx.accounts.user_token_out.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };

    let seeds = [AUTHORITY, &[ctx.bumps.authority]];
    let signer_seeds = &[&seeds[..]];

    let cpi_ctx_out = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts_out,
        signer_seeds,
    );
    
    let amount_out_u64 = amount_out_after_fee.try_into()?;
    token::transfer(cpi_ctx_out, amount_out_u64)?;
    // Step 6 ends

    // Step 7 - Update pool state
    pool.tokens[token_0_index].balance += amount_in;
    pool.tokens[token_1_index].balance -= amount_out_u64;
    Ok(())
}
