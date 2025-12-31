use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{
    constants::{AUTHORITY, POOL_VAULT, WEIGHT_POOL},
    errors::MiniStabbleError,
    math::fixed::FixedDiv,
    state::WeightedPool,
};

#[derive(Accounts)]
pub struct Deposit<'info> {
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
    ctx: Context<Deposit>,
    lp_amount: u64,
    input_token_a_amount: u64,
    input_token_b_amount: u64,
) -> Result<()> {
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

    let vault_a_balance = pool.tokens[token_a_index].balance;
    let vault_b_balance = pool.tokens[token_b_index].balance;

    let lp_supply = lp.supply;

    let (lp_to_mint, token_a_required, token_b_required) = if lp_supply == 0 {
        require!(input_token_a_amount > 0, MiniStabbleError::InvalidAmount);
        require!(input_token_b_amount > 0, MiniStabbleError::InvalidAmount);

        let scaled_input_token_a_amount =
            pool.tokens[token_a_index].scale_amount_up(input_token_a_amount);
        let scaled_input_token_b_amount =
            pool.tokens[token_b_index].scale_amount_up(input_token_b_amount);

        // First deposit of the pool
        let amount_product = (scaled_input_token_a_amount as u128)
            .checked_mul(scaled_input_token_b_amount as u128)
            .ok_or(MiniStabbleError::MathOverflow)?;
        let lp_to_mint = u64::try_from(amount_product.isqrt())?;
        (
            lp_to_mint,
            scaled_input_token_a_amount,
            scaled_input_token_b_amount,
        )
    } else {
        require!(lp_amount > 0, MiniStabbleError::InvalidAmount);
        // Normal Deposit
        let token_a_required = u64::try_from(
            (lp_amount as u128)
                .checked_mul(vault_a_balance as u128)
                .ok_or(MiniStabbleError::MathOverflow)?
                .div_up(lp_supply as u128)?,
        )?;

        let token_b_required = u64::try_from(
            ((lp_amount as u128)
                .checked_mul(vault_b_balance as u128)
                .ok_or(MiniStabbleError::MathOverflow)?)
            .div_up(lp_supply as u128)?,
        )?;
        (lp_amount, token_a_required, token_b_required)
    };

    // Slippage check - compare actual transfer amounts (scaled down) to user's max
    require!(
        pool.tokens[token_a_index].scale_amount_down(token_a_required) <= input_token_a_amount,
        MiniStabbleError::SlippageExceeded
    );
    require!(
        pool.tokens[token_b_index].scale_amount_down(token_b_required) <= input_token_b_amount,
        MiniStabbleError::SlippageExceeded
    );

    // Transfer tokens - have lp_to_mint, token_a_required, token_b_required
    // Token 1
    let cpi_accounts_a = Transfer {
        from: ctx.accounts.user_token_a.to_account_info(),
        to: ctx.accounts.vault_token_a.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    token::transfer(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts_a),
        pool.tokens[token_a_index].scale_amount_down(token_a_required),
    )?;

    // Token 2
    let cpi_accounts_b = Transfer {
        from: ctx.accounts.user_token_b.to_account_info(),
        to: ctx.accounts.vault_token_b.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    token::transfer(
        CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts_b),
        pool.tokens[token_b_index].scale_amount_down(token_b_required),
    )?;

    // Mint
    let mint_accounts = MintTo {
        authority: ctx.accounts.authority.to_account_info(),
        to: ctx.accounts.user_lp.to_account_info(),
        mint: ctx.accounts.lp_mint.to_account_info(),
    };

    let seeds = &[AUTHORITY, &[ctx.bumps.authority]];
    let signer_seeds = &[&seeds[..]];

    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            mint_accounts,
            signer_seeds,
        ),
        lp_to_mint,
    )?;

    // Update pool state
    pool.tokens[token_a_index].balance = pool.tokens[token_a_index]
        .balance
        .checked_add(token_a_required)
        .ok_or(MiniStabbleError::MathOverflow)?;
    pool.tokens[token_b_index].balance = pool.tokens[token_b_index]
        .balance
        .checked_add(token_b_required)
        .ok_or(MiniStabbleError::MathOverflow)?;

    Ok(())
}
