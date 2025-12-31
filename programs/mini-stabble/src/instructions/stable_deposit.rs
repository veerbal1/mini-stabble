use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{
    constants::{AUTHORITY, POOL_VAULT, STABLE_POOL},
    errors::MiniStabbleError,
    state::StablePool,
};

#[derive(Accounts)]
pub struct StableDeposit<'info> {
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

    #[account(constraint = mint_a.key() != mint_b.key())]
    pub mint_a: Account<'info, Mint>,
    pub mint_b: Account<'info, Mint>,

    #[account(mut, constraint = lp_mint.key() == pool.lp_mint)]
    pub lp_mint: Account<'info, Mint>,

    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), mint_a.key().as_ref()], bump, token::mint = mint_a, token::authority = authority)]
    pub vault_token_a: Account<'info, TokenAccount>,

    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), mint_b.key().as_ref()], bump, token::mint = mint_b, token::authority = authority)]
    pub vault_token_b: Account<'info, TokenAccount>,

    #[account(mut, token::mint = mint_a, token::authority = user)]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(mut, token::mint = mint_b, token::authority = user)]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(init_if_needed, associated_token::mint = lp_mint, associated_token::authority = user, payer = user)]
    pub user_lp: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn handler(
    ctx: Context<StableDeposit>,
    max_amount_a: u64,
    max_amount_b: u64,
    lp_amount: u64,
) -> Result<()> {
    require!(max_amount_a > 0, MiniStabbleError::InvalidAmount);
    require!(max_amount_b > 0, MiniStabbleError::InvalidAmount);

    let pool = &mut ctx.accounts.pool;
    require!(pool.is_active, MiniStabbleError::PoolInActive);

    let lp_mint = &ctx.accounts.lp_mint;
    let token_a_mint = &ctx.accounts.mint_a;
    let token_b_mint = &ctx.accounts.mint_b;
    let token_a_index = pool
        .get_token_index(&token_a_mint.key())
        .ok_or(MiniStabbleError::InvalidMint)?;
    let token_b_index = pool
        .get_token_index(&token_b_mint.key())
        .ok_or(MiniStabbleError::InvalidMint)?;
    let scaled_max_amount_a = pool.tokens[token_a_index].scale_amount_up(max_amount_a);
    let scaled_max_amount_b = pool.tokens[token_b_index].scale_amount_up(max_amount_b);

    let (lp_to_mint, actual_amount_a_to_deposit, actual_amount_b_to_deposit) =
        if lp_mint.supply == 0 {
            let lp = ((scaled_max_amount_a as u128)
                .checked_mul(scaled_max_amount_b as u128)
                .ok_or(MiniStabbleError::MathOverflow)?)
            .isqrt();
            (u64::try_from(lp)?, scaled_max_amount_a, scaled_max_amount_b)
        } else {
            require!(lp_amount > 0, MiniStabbleError::InvalidAmount);
            let token_a_balance_scaled = pool.tokens[token_a_index].balance;
            let token_b_balance_scaled = pool.tokens[token_b_index].balance;

            let amount_a_to_deposit = (token_a_balance_scaled
                .checked_mul(lp_amount)
                .ok_or(MiniStabbleError::MathOverflow)?)
            .checked_div(lp_mint.supply)
            .ok_or(MiniStabbleError::MathOverflow)?;

            let amount_b_to_deposit = (token_b_balance_scaled
                .checked_mul(lp_amount)
                .ok_or(MiniStabbleError::MathOverflow)?)
            .checked_div(lp_mint.supply)
            .ok_or(MiniStabbleError::MathOverflow)?;

            (lp_amount, amount_a_to_deposit, amount_b_to_deposit)
        };

    require!(
        actual_amount_a_to_deposit <= scaled_max_amount_a,
        MiniStabbleError::SlippageExceeded
    );
    require!(
        actual_amount_b_to_deposit <= scaled_max_amount_b,
        MiniStabbleError::SlippageExceeded
    );

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_a.to_account_info(),
                to: ctx.accounts.vault_token_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        pool.tokens[token_a_index].scale_amount_down(actual_amount_a_to_deposit),
    )?;

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_b.to_account_info(),
                to: ctx.accounts.vault_token_b.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        pool.tokens[token_b_index].scale_amount_down(actual_amount_b_to_deposit),
    )?;

    let seeds = &[AUTHORITY, &[ctx.bumps.authority]];
    let signer_seeds = &[&seeds[..]];

    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.lp_mint.to_account_info(),
                to: ctx.accounts.user_lp.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
            signer_seeds,
        ),
        lp_to_mint,
    )?;

    pool.tokens[token_a_index].balance = pool.tokens[token_a_index]
        .balance
        .checked_add(actual_amount_a_to_deposit)
        .ok_or(MiniStabbleError::MathOverflow)?;

    pool.tokens[token_b_index].balance = pool.tokens[token_b_index]
        .balance
        .checked_add(actual_amount_b_to_deposit)
        .ok_or(MiniStabbleError::MathOverflow)?;
    Ok(())
}
