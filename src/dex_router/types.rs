use anchor_lang::prelude::*;
use crate::account_resolver::{RaydiumCpmmAccounts, RaydiumClmmAccounts, PumpfunAccounts, PumpswapAccounts};
use crate::state::DexType;

/// Swap result containing output amount and fees
#[derive(Debug, Clone)]
pub struct SwapResult {
    pub amount_out: u64,
    pub fee_amount: u64,
}

/// Generic trait for DEX swap operations
pub trait DexSwap<'info> {
    type Accounts;
    
    fn execute_swap(
        accounts: Self::Accounts,
        user_input_account: &AccountInfo<'info>,
        user_output_account: &AccountInfo<'info>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<SwapResult>;
}

#[derive(Clone)]
pub enum DexAccounts<'info> {
    RaydiumCpmm(RaydiumCpmmAccounts<'info>),
    RaydiumClmm(RaydiumClmmAccounts<'info>),
    Pumpfun(PumpfunAccounts<'info>),
    Pumpswap(PumpswapAccounts<'info>),
}

/// DEX-specific account requirements and constants
pub mod constants {
    // Raydium CPMM requires 12 accounts (from raydium-cpi-example)
    pub const RAYDIUM_CPMM_ACCOUNT_COUNT: u8 = 12;
    
    // Raydium CLMM base accounts + variable tick arrays 
    pub const RAYDIUM_CLMM_BASE_ACCOUNT_COUNT: u8 = 12;
    pub const RAYDIUM_CLMM_MAX_TICK_ARRAYS: u8 = 4;
    
    // PumpFun requires 8 accounts (estimated)
    pub const PUMPFUN_ACCOUNT_COUNT: u8 = 8;
    
    // PumpSwap requires 8 accounts (estimated)
    pub const PUMPSWAP_ACCOUNT_COUNT: u8 = 8;
}

/// Helper function to get expected account count for a DEX type
pub fn get_expected_account_count(dex_type: DexType) -> u8 {
    match dex_type {
        DexType::RaydiumCpmm => constants::RAYDIUM_CPMM_ACCOUNT_COUNT,
        DexType::RaydiumClmm => constants::RAYDIUM_CLMM_BASE_ACCOUNT_COUNT,
        DexType::PumpFunBondingCurve => constants::PUMPFUN_ACCOUNT_COUNT,
        DexType::PumpSwap => constants::PUMPSWAP_ACCOUNT_COUNT,
    }
}

/// Validation helper for account counts
pub fn validate_account_count(dex_type: DexType, actual_count: u8) -> Result<()> {
    let expected = get_expected_account_count(dex_type);
    match dex_type {
        DexType::RaydiumClmm => {
            // CLMM allows variable tick arrays
            if actual_count < expected || actual_count > expected + constants::RAYDIUM_CLMM_MAX_TICK_ARRAYS {
                return Err(error!(crate::errors::ArbitrageError::InvalidAccountCount));
            }
        }
        _ => {
            if actual_count != expected {
                return Err(error!(crate::errors::ArbitrageError::InvalidAccountCount));
            }
        }
    }
    Ok(())
}