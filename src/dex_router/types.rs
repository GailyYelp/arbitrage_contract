use anchor_lang::prelude::*;
use crate::account_resolver::{RaydiumCpmmAccounts, RaydiumClmmAccounts, PumpfunAccounts, PumpswapAccounts};
use crate::account_derivation::DerivedAccounts;
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
        derived: &DerivedAccounts,
        remaining_accounts: &'info [AccountInfo<'info>],
        payer: &AccountInfo<'info>,
        token_program: &AccountInfo<'info>,
        associated_token_program: &AccountInfo<'info>,
        system_program: &AccountInfo<'info>,
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
    // CPI/常量集版本号（用于日志与治理）
    pub const CPI_VERSION: u32 = 1;
    // 仅客户端传入账户数量（按 indices 协议）
    // Raydium CPMM：amm_config, pool_state, token0_vault, token1_vault, input_mint, output_mint, observation_state
    pub const RAYDIUM_CPMM_ACCOUNT_COUNT: u8 = 7;
    
    // Raydium CLMM（本实现按 11 个基础账户解析；tick arrays 动态由客户端另行传递到 remaining_accounts 但不在 indices 中计数）
    pub const RAYDIUM_CLMM_BASE_ACCOUNT_COUNT: u8 = 11;
    pub const RAYDIUM_CLMM_MAX_TICK_ARRAYS: u8 = 4; // 备用
    
    // PumpFun Bonding（bonding_curve, mint, creator）
    pub const PUMPFUN_ACCOUNT_COUNT: u8 = 3;
    
    // PumpSwap（pool_state, base_mint, quote_mint, coin_creator）
    pub const PUMPSWAP_ACCOUNT_COUNT: u8 = 4;
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