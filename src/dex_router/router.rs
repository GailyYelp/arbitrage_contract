use anchor_lang::prelude::*;
use crate::account_derivation::DerivedAccounts;
use crate::state::DexType;
use super::swaps::*;
use super::types::*;

pub struct DexRouter;

impl DexRouter {
    pub fn execute_swap<'info>(
        dex_type: DexType,
        accounts: DexAccounts<'info>,
        derived: &DerivedAccounts,
        user_input_account: &AccountInfo<'info>,
        user_output_account: &AccountInfo<'info>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<SwapResult> {
        msg!("Routing {} swap: {} -> min {}", 
             match dex_type {
                 DexType::RaydiumCpmm => "Raydium CPMM",
                 DexType::RaydiumClmm => "Raydium CLMM", 
                 DexType::PumpFunBondingCurve => "PumpFun",
                 DexType::PumpSwap => "PumpSwap",
             },
             amount_in, 
             minimum_amount_out);
        
        // Dispatch to the appropriate DEX implementation
        match (dex_type, accounts) {
            (DexType::RaydiumCpmm, DexAccounts::RaydiumCpmm(cpmm_accounts)) => {
                RaydiumCpmmSwap::execute_swap(
                    cpmm_accounts,
                    derived,
                    user_input_account,
                    user_output_account,
                    amount_in,
                    minimum_amount_out,
                )
            }
            (DexType::RaydiumClmm, DexAccounts::RaydiumClmm(clmm_accounts)) => {
                RaydiumClmmSwap::execute_swap(
                    clmm_accounts,
                    derived,
                    user_input_account,
                    user_output_account,
                    amount_in,
                    minimum_amount_out,
                )
            }
            (DexType::PumpFunBondingCurve, DexAccounts::Pumpfun(pumpfun_accounts)) => {
                PumpfunSwap::execute_swap(
                    pumpfun_accounts,
                    derived,
                    user_input_account,
                    user_output_account,
                    amount_in,
                    minimum_amount_out,
                )
            }
            (DexType::PumpSwap, DexAccounts::Pumpswap(pumpswap_accounts)) => {
                PumpswapSwap::execute_swap(
                    pumpswap_accounts,
                    derived,
                    user_input_account,
                    user_output_account,
                    amount_in,
                    minimum_amount_out,
                )
            }
            // Mismatched DEX type and accounts
            _ => {
                msg!("DEX type and account type mismatch");
                Err(DexRouterError::InvalidAccountType.into())
            }
        }
    }
    
    /// Validates minimum output amount against actual result
    pub fn validate_swap_result(
        result: &SwapResult,
        minimum_amount_out: u64,
    ) -> Result<()> {
        if result.amount_out < minimum_amount_out {
            msg!(
                "Insufficient output amount: got {}, expected min {}",
                result.amount_out,
                minimum_amount_out
            );
            return Err(DexRouterError::InsufficientOutputAmount.into());
        }
        Ok(())
    }
    
    /// Gets DEX name for logging
    pub fn get_dex_name(dex_type: DexType) -> &'static str {
        match dex_type {
            DexType::RaydiumCpmm => "Raydium CPMM",
            DexType::RaydiumClmm => "Raydium CLMM",
            DexType::PumpFunBondingCurve => "PumpFun",
            DexType::PumpSwap => "PumpSwap",
        }
    }
}

#[error_code]
pub enum DexRouterError {
    #[msg("Invalid account type for DEX")]
    InvalidAccountType,
    #[msg("Swap execution failed")]
    SwapExecutionFailed,
    #[msg("Insufficient output amount")]
    InsufficientOutputAmount,
}