use anchor_lang::prelude::*;

#[error_code]
pub enum StabbleError {
    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Divide by zero")]
    DivideByZero,

    #[msg("Invalid token amount")]
    InvalidAmount,

    #[msg("Slippage exceeded")]
    SlippageExceeded,

    #[msg("No profitable arbitrage opportunity")]
    NoProfitableArbitrage,
}
