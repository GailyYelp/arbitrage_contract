use anchor_lang::prelude::*;
use crate::state::{DexType, PathAccountMapping};
use crate::errors::ArbitrageError;
use super::accounts::*;

pub struct AccountResolver<'info> {
    remaining_accounts: &'info [AccountInfo<'info>],
}

impl<'info> AccountResolver<'info> {
    pub fn new(
        remaining_accounts: &'info [AccountInfo<'info>],
    ) -> Self {
        Self {
            remaining_accounts,
        }
    }

    pub fn resolve_raydium_cpmm_accounts(
        &self,
        mapping: &PathAccountMapping,
    ) -> Result<RaydiumCpmmAccounts<'info>> {
        let start_idx = mapping.start_index as usize;
        
        if start_idx + 7 > self.remaining_accounts.len() {
            return Err(ArbitrageError::InsufficientAccounts.into());
        }

        Ok(RaydiumCpmmAccounts {
            amm_config: &self.remaining_accounts[start_idx],        // 1. AMM配置地址
            pool_state: &self.remaining_accounts[start_idx + 1],    // 2. 池地址 
            token0_vault: &self.remaining_accounts[start_idx + 2],  // 3. token0金库
            token1_vault: &self.remaining_accounts[start_idx + 3],  // 4. token1金库
            input_mint: &self.remaining_accounts[start_idx + 4],    // 5. 输入代币mint
            output_mint: &self.remaining_accounts[start_idx + 5],   // 6. 输出代币mint
            observation_state: &self.remaining_accounts[start_idx + 6], // 7. observation账户
        })
    }

    pub fn resolve_raydium_clmm_accounts(
        &self,
        mapping: &PathAccountMapping,
    ) -> Result<RaydiumClmmAccounts<'info>> {
        let start_idx = mapping.start_index as usize;
        
        if start_idx + 11 > self.remaining_accounts.len() {
            return Err(ArbitrageError::InsufficientAccounts.into());
        }

        Ok(RaydiumClmmAccounts {
            clmm_program: &self.remaining_accounts[start_idx],
            amm_config: &self.remaining_accounts[start_idx + 1],
            pool_state: &self.remaining_accounts[start_idx + 2],
            input_vault: &self.remaining_accounts[start_idx + 3],
            output_vault: &self.remaining_accounts[start_idx + 4],
            observation_state: &self.remaining_accounts[start_idx + 5],
            token_program: &self.remaining_accounts[start_idx + 6],
            token_program_2022: &self.remaining_accounts[start_idx + 7],
            memo_program: &self.remaining_accounts[start_idx + 8],
            input_vault_mint: &self.remaining_accounts[start_idx + 9],
            output_vault_mint: &self.remaining_accounts[start_idx + 10],
        })
    }

    pub fn resolve_pumpfun_accounts(
        &self,
        mapping: &PathAccountMapping,
    ) -> Result<PumpfunAccounts<'info>> {
        let start_idx = mapping.start_index as usize;
        
        if start_idx + 3 > self.remaining_accounts.len() {
            return Err(ArbitrageError::InsufficientAccounts.into());
        }

        Ok(PumpfunAccounts {
            bonding_curve: &self.remaining_accounts[start_idx],      // 1. bonding_curve地址
            mint: &self.remaining_accounts[start_idx + 1],           // 2. 代币mint
            creator: &self.remaining_accounts[start_idx + 2],        // 3. 创建者地址
        })
    }

    pub fn resolve_pumpswap_accounts(
        &self,
        mapping: &PathAccountMapping,
    ) -> Result<PumpswapAccounts<'info>> {
        let start_idx = mapping.start_index as usize;
        
        if start_idx + 4 > self.remaining_accounts.len() {
            return Err(ArbitrageError::InsufficientAccounts.into());
        }

        Ok(PumpswapAccounts {
            pool_state: &self.remaining_accounts[start_idx],         // 1. 池地址
            base_mint: &self.remaining_accounts[start_idx + 1],      // 2. 基础代币mint  
            quote_mint: &self.remaining_accounts[start_idx + 2],     // 3. 计价代币mint
            coin_creator: &self.remaining_accounts[start_idx + 3],   // 4. 代币创建者
        })
    }

    pub fn validate_account_mapping(&self, mapping: &PathAccountMapping) -> Result<()> {
        let start_idx = mapping.start_index as usize;
        let required_count = mapping.account_count as usize;
        
        if start_idx + required_count > self.remaining_accounts.len() {
            msg!("Account validation failed: need {} accounts starting at {}, but only have {} total", 
                 required_count, start_idx, self.remaining_accounts.len());
            return Err(ArbitrageError::InsufficientAccounts.into());
        }
        
        // Verify expected account counts for each DEX type (仅客户端传递的账户)
        let expected_count = match mapping.dex_type {
            DexType::RaydiumCpmm => 7,        // 对齐客户端 SmartAccountCollector
            DexType::RaydiumClmm => 8,        // 基础8个 + 动态tick_arrays
            DexType::PumpFunBondingCurve => 3, // 保持不变
            DexType::PumpSwap => 4,           // 保持不变
        };
        
        if required_count != expected_count {
            msg!("Account count mismatch for {:?}: expected {}, got {}", 
                 mapping.dex_type, expected_count, required_count);
            return Err(ArbitrageError::InvalidAccountCount.into());
        }
        
        Ok(())
    }
}