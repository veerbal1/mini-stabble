use anchor_lang::prelude::*;

/// Struct representing a single token in the pool
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, InitSpace)]
pub struct PoolToken {
    /// Mint address of the token
    pub mint: Pubkey,

    /// Token account for the token
    pub token_account: Pubkey,

    /// Number of decimals for this token's mint
    pub decimals: u8,

    /// Factor by which amounts are scaled for calculations
    pub scaling_factor: u64,

    /// The current balance of the token held by the pool (on-chain units)
    pub balance: u64,

    /// The weight of the token within the pool (for weighted pools)
    pub weight: u64,
}

impl PoolToken {
    pub fn scale_amount_up(&self, raw_amount: u64) -> u64 {
        raw_amount.checked_mul(self.scaling_factor).unwrap()
    }

    pub fn scale_amount_down(&self, scaled_amount: u64) -> u64 {
        scaled_amount.checked_div(self.scaling_factor).unwrap()
    }
}

#[account]
#[derive(InitSpace)]
pub struct WeightedPool {
    /// PDA that signs for token transfers
    pub authority: Pubkey,

    /// LP token mint for this pool
    pub lp_mint: Pubkey,

    /// Whether trading is enabled
    pub is_active: bool,

    /// Cached invariant value
    pub invariant: u64,

    /// Swap fee (e.g., 3_000_000 = 0.3% with SCALE = 1e9)
    pub swap_fee: u64,

    /// Token metadata
    #[max_len(8)]
    pub tokens: Vec<PoolToken>,

    /// PDA bump seed
    pub bump: u8,
}

impl WeightedPool {
    pub fn get_token_index(&self, mint: &Pubkey) -> Option<usize> {
        self.tokens.iter().position(|t| t.mint == *mint)
    }

    pub fn get_balances(&self) -> Vec<u64> {
        self.tokens.iter().map(|t| t.balance).collect()
    }

    pub fn get_weights(&self) -> Vec<u64> {
        self.tokens.iter().map(|t| t.weight).collect()
    }

    pub const LEN: usize = 8 + Self::INIT_SPACE;
}

#[account]
#[derive(InitSpace)]
pub struct StablePool {
    pub authority: Pubkey,
    pub lp_mint: Pubkey,
    pub is_active: bool,
    pub invariant: u64,
    pub swap_fee: u64,

    /// Current amplification factor
    pub amp: u64,

    /// Target amp (for ramping)
    pub amp_target: u64,

    /// Ramp start timestamp
    pub amp_start_ts: i64,

    /// Ramp end timestamp  
    pub amp_end_ts: i64,

    #[max_len(8)]
    pub tokens: Vec<PoolToken>,
    pub bump: u8,
}

impl StablePool {
    pub fn get_token_index(&self, mint: &Pubkey) -> Option<usize> {
        self.tokens.iter().position(|t| t.mint == *mint)
    }

    pub fn get_balances(&self) -> Vec<u64> {
        self.tokens.iter().map(|t| t.balance).collect()
    }

    pub fn get_current_amp(&self) -> u64 {
        self.amp
    }

    pub const LEN: usize = 8 + Self::INIT_SPACE;
}
