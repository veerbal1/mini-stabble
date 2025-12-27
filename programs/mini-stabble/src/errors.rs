use anchor_lang::prelude::*;

#[error_code]
pub enum MiniStabbleError {
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

    #[msg("Token A mint should be less than Token B Mint")]
    MintOrderInvalid,

    #[msg("Invalid Token Weight")]
    InvalidWeight,
}
