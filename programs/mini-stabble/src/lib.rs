use anchor_lang::prelude::*;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod state;

declare_id!("FURtuxyXWgpnETkNho8PL6mpuRh9mCnVsWgUY14JzusX");

#[program]
pub mod mini_stabble {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
