use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{
    constants::{AUTHORITY, POOL_VAULT, WEIGHT_POOL},
    errors::MiniStabbleError,
    math::{
        fixed::{ONE, SCALE},
        weighted::{calc_invariant, calc_lp_to_mint},
    },
    state::WeightedPool,
};

#[derive(Accounts)]
pub struct DepositUnbalanced<'info> {
    #[account(mut, seeds = [WEIGHT_POOL, pool.lp_mint.as_ref()], bump = pool.bump)]
    pub pool: Account<'info, WeightedPool>,

    #[account(mut)]
    pub user: Signer<'info>,

    // Mint Accounts
    #[account(mut, address = pool.lp_mint.key())]
    pub lp_mint: Account<'info, Mint>,
    #[account(constraint = token_a_mint.key() != token_b_mint.key())]
    pub token_a_mint: Account<'info, Mint>,
    pub token_b_mint: Account<'info, Mint>,

    #[account(mut, token::authority = user, token::mint = token_a_mint)]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(mut, token::authority = user, token::mint = token_b_mint)]
    pub user_token_b: Account<'info, TokenAccount>,

    // Vault Tokens
    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), token_a_mint.key().as_ref()], bump, token::authority = authority, token::mint = token_a_mint)]
    pub vault_token_a: Account<'info, TokenAccount>,

    #[account(mut, seeds=[POOL_VAULT, pool.key().as_ref(), token_b_mint.key().as_ref()], bump, token::authority = authority, token::mint = token_b_mint)]
    pub vault_token_b: Account<'info, TokenAccount>,

    // user lp account
    #[account(init_if_needed, associated_token::mint = lp_mint, associated_token::authority = user, payer = user)]
    pub user_lp: Account<'info, TokenAccount>,

    /// CHECK: Authority PDA used for signing
    #[account(seeds=[AUTHORITY], bump)]
    pub authority: UncheckedAccount<'info>,

    // Programs - token program. system program
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn handler(
    ctx: Context<DepositUnbalanced>,
    min_lp_amount: u64,
    input_amount_a: u64,
    input_amount_b: u64,
) -> Result<()> {
    require!(
        min_lp_amount > 0 && input_amount_a > 0 && input_amount_b > 0,
        MiniStabbleError::InvalidAmount
    );

    let pool = &mut ctx.accounts.pool;
    require!(pool.is_active, MiniStabbleError::PoolInActive);

    let token_a_mint = &ctx.accounts.token_a_mint;
    let token_b_mint = &ctx.accounts.token_b_mint;

    let lp = &ctx.accounts.lp_mint;

    let token_a_index = pool
        .get_token_index(&token_a_mint.key())
        .ok_or(MiniStabbleError::InvalidMint)?;

    let token_b_index = pool
        .get_token_index(&token_b_mint.key())
        .ok_or(MiniStabbleError::InvalidMint)?;

    // Get both amounts

    // Get current pool ratio.
    let scaled_input_amount_a = pool.tokens[token_a_index].scale_amount_up(input_amount_a);
    let scaled_input_amount_b = pool.tokens[token_b_index].scale_amount_up(input_amount_b);

    let vault_a_balance = pool.tokens[token_a_index].balance;
    let vault_b_balance = pool.tokens[token_b_index].balance;

    // then get deposits amount ratio

    let deposit_amount_ratio = ((scaled_input_amount_a as u128)
        .checked_mul(SCALE)
        .ok_or(MiniStabbleError::MathOverflow)?)
    .checked_div(scaled_input_amount_b as u128)
    .ok_or(MiniStabbleError::MathOverflow)?;

    let current_pool_ratio = ((vault_a_balance as u128)
        .checked_mul(SCALE)
        .ok_or(MiniStabbleError::MathOverflow)?)
    .checked_div(vault_b_balance as u128)
    .ok_or(MiniStabbleError::MathOverflow)?;

    // then check if deposit ratio is less than or greater than pool ratio
    let token_a_excess = deposit_amount_ratio > current_pool_ratio;

    let (excess_amount, balanced_portion_of_excess_token): (u128, u128) = if token_a_excess {
        // Input Token A is in excess
        let balanced = current_pool_ratio
            .checked_mul(scaled_input_amount_b as u128)
            .ok_or(MiniStabbleError::MathOverflow)?
            .checked_div(SCALE)
            .ok_or(MiniStabbleError::MathOverflow)?;

        let excess = (scaled_input_amount_a as u128)
            .checked_sub(balanced)
            .ok_or(MiniStabbleError::MathOverflow)?;

        (excess, balanced)
    } else {
        // Input Token B is in excess
        let balanced = (scaled_input_amount_a as u128)
            .checked_mul(SCALE)
            .ok_or(MiniStabbleError::MathOverflow)?
            .checked_div(current_pool_ratio as u128)
            .ok_or(MiniStabbleError::MathOverflow)?;

        let excess = (scaled_input_amount_b as u128)
            .checked_sub(balanced)
            .ok_or(MiniStabbleError::MathOverflow)?;

        (excess, balanced)
    };

    let num = (excess_amount as u128)
        .checked_mul(
            SCALE
                .checked_sub(pool.swap_fee as u128)
                .ok_or(MiniStabbleError::MathOverflow)? as u128,
        )
        .ok_or(MiniStabbleError::MathOverflow)?;

    let den = SCALE;

    let amount_after_fee = (num as u128)
        .checked_div(den)
        .ok_or(MiniStabbleError::MathOverflow)?;

    // total it.
    let (effective_deposit_amount_a_for_lp, effective_deposit_amount_b_for_lp): (u128, u128) =
        if token_a_excess {
            (
                balanced_portion_of_excess_token + amount_after_fee,
                scaled_input_amount_b as u128,
            )
        } else {
            (
                scaled_input_amount_a as u128,
                (balanced_portion_of_excess_token + amount_after_fee),
            )
        };

    let weight_a = pool.tokens[token_a_index].weight;
    let weight_b = pool.tokens[token_b_index].weight;

    // calculate new lp to mint based on new deposits (excluding fee amount)
    let old_k = calc_invariant(
        &[vault_a_balance as u128, vault_b_balance as u128],
        &[weight_a as u128, weight_b as u128],
    )?;

    let new_k = calc_invariant(
        &[
            (vault_a_balance as u128)
                .checked_add(effective_deposit_amount_a_for_lp)
                .ok_or(MiniStabbleError::MathOverflow)?,
            (vault_b_balance as u128)
                .checked_add(effective_deposit_amount_b_for_lp)
                .ok_or(MiniStabbleError::MathOverflow)?,
        ],
        &[weight_a as u128, weight_b as u128],
    )?;

    let lp_to_mint = calc_lp_to_mint(lp.supply as u128, new_k, old_k, ONE)?;

    require!(
        lp_to_mint >= min_lp_amount as u128,
        MiniStabbleError::SlippageExceeded
    );

    // deposit token a
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_a.to_account_info(),
                to: ctx.accounts.vault_token_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        input_amount_a,
    )?;

    // deposit token b
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_b.to_account_info(),
                to: ctx.accounts.vault_token_b.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        input_amount_b,
    )?;

    // mint LP tokens to user
    let authority_bump = ctx.bumps.authority;
    let authority_seeds = &[AUTHORITY, &[authority_bump]];
    let signer_seeds = &[&authority_seeds[..]];

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
        lp_to_mint as u64,
    )?;

    // update pool balance with new amount (scaled amounts since balances are stored scaled)
    let pool = &mut ctx.accounts.pool;

    pool.tokens[token_a_index].balance = pool.tokens[token_a_index]
        .balance
        .checked_add(scaled_input_amount_a)
        .ok_or(MiniStabbleError::MathOverflow)?;

    pool.tokens[token_b_index].balance = pool.tokens[token_b_index]
        .balance
        .checked_add(scaled_input_amount_b)
        .ok_or(MiniStabbleError::MathOverflow)?;

    Ok(())
}
