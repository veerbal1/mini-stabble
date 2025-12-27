use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct PoolToken {
    pub mint: Pubkey,
    pub decimals: u8,
    pub scaling_up: bool,
    pub scaling_factor: u64,
    pub balance: u64,
    pub weight: u64,
}

impl PoolToken {
    pub fn scale_amount_up(&self, raw_amount: u64) -> u64 {
        if self.scaling_up {
            raw_amount.checked_mul(self.scaling_factor).unwrap()
        } else {
            raw_amount.checked_div(self.scaling_factor).unwrap()
        }
    }

    pub fn scale_amount_down(&self, scaled_amount: u64) -> u64 {
        if self.scaling_up {
            scaled_amount.checked_div(self.scaling_factor).unwrap()
        } else {
            scaled_amount.checked_mul(self.scaling_factor).unwrap()
        }
    }
}
