use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{constants::WEIGHT_POOL, state::WeightedPool};

#[derive(Accounts)]
#[instruction(mint_in: Pubkey, mint_out: Pubkey)]
pub struct Swap<'info> {
    #[account(
        mut,
        seeds = [WEIGHT_POOL, pool.lp_mint.as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, WeightedPool>,

    #[account(mut)]
    pub user_token_in: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_token_out: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_token_in: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_token_out: Account<'info, TokenAccount>,

    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}
