use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;
pub mod errors;
pub mod account_resolver;
pub mod account_derivation;
pub mod dex_router;

pub use instructions::*;
pub use state::*;
pub use errors::*;
pub use account_resolver::*;
pub use account_derivation::*;
pub use dex_router::*;

declare_id!("7R2DEVjE6DQPsQYnLSaFwPQjp1RN5vR8BAfxNhociMxV");

#[program]
pub mod arbitrage_contract {
    use super::*;
    
    pub fn execute_arbitrage<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage<'info>>,
        params: ArbitrageParams,
    ) -> Result<()> {
        instructions::execute_arbitrage(ctx, params)
    }
}