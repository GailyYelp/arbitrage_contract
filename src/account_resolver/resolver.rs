use anchor_lang::prelude::*;
use crate::state::{DexType, PathAccountMappingV2};
use crate::errors::ArbitrageError;
use super::accounts::*;
use crate::dex_router::types::{get_expected_account_count, constants};
use core::cmp::min;
use std::collections::HashSet;

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

    /// 按 V2 indices 解析 Raydium CPMM 所需账户（客户端 indices 顺序与常量对齐）
    pub fn resolve_raydium_cpmm_by_indices(
        &self,
        mapping: &PathAccountMappingV2,
    ) -> Result<RaydiumCpmmAccounts<'info>> {
        let idxs = &mapping.indices;
        if idxs.len() != 7 {
            return Err(ArbitrageError::InvalidAccountCount.into());
        }
        Ok(RaydiumCpmmAccounts {
            amm_config: self.ai(idxs[0])?,
            pool_state: self.ai(idxs[1])?,
            token0_vault: self.ai(idxs[2])?,
            token1_vault: self.ai(idxs[3])?,
            input_mint: self.ai(idxs[4])?,
            output_mint: self.ai(idxs[5])?,
            observation_state: self.ai(idxs[6])?,
        })
    }

    /// 按 V2 indices 解析 Raydium CLMM 所需账户（基础 11 个账户，tick arrays 单独追加到 remaining）
    pub fn resolve_raydium_clmm_by_indices(
        &self,
        mapping: &PathAccountMappingV2,
    ) -> Result<RaydiumClmmAccounts<'info>> {
        let idxs = &mapping.indices;
        if idxs.len() != 11 {
            return Err(ArbitrageError::InvalidAccountCount.into());
        }
        Ok(RaydiumClmmAccounts {
            clmm_program: self.ai(idxs[0])?,
            amm_config: self.ai(idxs[1])?,
            pool_state: self.ai(idxs[2])?,
            input_vault: self.ai(idxs[3])?,
            output_vault: self.ai(idxs[4])?,
            observation_state: self.ai(idxs[5])?,
            token_program: self.ai(idxs[6])?,
            token_program_2022: self.ai(idxs[7])?,
            memo_program: self.ai(idxs[8])?,
            input_vault_mint: self.ai(idxs[9])?,
            output_vault_mint: self.ai(idxs[10])?,
        })
    }

    /// 按 V2 indices 解析 Pumpfun Bonding Curve 所需账户
    pub fn resolve_pumpfun_by_indices(
        &self,
        mapping: &PathAccountMappingV2,
    ) -> Result<PumpfunAccounts<'info>> {
        let idxs = &mapping.indices;
        if idxs.len() < 3 || idxs.len() > 4 {
            msg!("[Resolver] PumpFun indices mismatch: expected 3..=4 got {}", idxs.len());
            msg!("[Resolver] indices={:?}", idxs);
            return Err(ArbitrageError::InvalidAccountCount.into());
        }
        let fee_recipient_opt = if idxs.len() >= 4 { Some(self.ai(idxs[3])?) } else { None };
        Ok(PumpfunAccounts {
            bonding_curve: self.ai(idxs[0])?,
            mint: self.ai(idxs[1])?,
            creator: self.ai(idxs[2])?,
            fee_recipient_opt,
        })
    }

    /// 按 V2 indices 解析 Pumpswap 所需账户
    pub fn resolve_pumpswap_by_indices(
        &self,
        mapping: &PathAccountMappingV2,
    ) -> Result<PumpswapAccounts<'info>> {
        let idxs = &mapping.indices;
        if idxs.len() < 4 || idxs.len() > 6 {
            msg!("[Resolver] PumpSwap indices mismatch: expected 4..=6 got {}", idxs.len());
            msg!("[Resolver] indices={:?}", idxs);
            return Err(ArbitrageError::InvalidAccountCount.into());
        }
        let fee_recipient_opt = if idxs.len() >= 5 { Some(self.ai(idxs[4])?) } else { None };
        let fee_recipient_ata_opt = if idxs.len() >= 6 { Some(self.ai(idxs[5])?) } else { None };
        Ok(PumpswapAccounts {
            pool_state: self.ai(idxs[0])?,
            base_mint: self.ai(idxs[1])?,
            quote_mint: self.ai(idxs[2])?,
            coin_creator: self.ai(idxs[3])?,
            fee_recipient_opt,
            fee_recipient_ata_opt,
        })
    }

    /// 校验 indices 数量与 DEX 期望一致，并输出关键日志（含 signer/writable 提示）
    ///
    /// V2 协议：indices 仅覆盖“固定最小集”，CLMM 的 tick arrays/extension 等动态账户
    /// 由客户端追加到全局账户表，并在 swaps 中按程序 owner 动态注入 CPI metas。
    pub fn validate_indices_for_dex(&self, mapping: &PathAccountMappingV2) -> Result<()> {
        let actual_len_u8 = mapping.indices.len() as u8;
        let expected = get_expected_account_count(mapping.dex_type);
        let total = self.remaining_accounts.len();

        // 数量校验（CLMM 基础 11 个）
        match mapping.dex_type {
            DexType::RaydiumClmm => {
                if actual_len_u8 != constants::RAYDIUM_CLMM_BASE_ACCOUNT_COUNT {
                    msg!(
                        "[Resolver] CLMM indices mismatch: expected {} got {}",
                        constants::RAYDIUM_CLMM_BASE_ACCOUNT_COUNT,
                        actual_len_u8
                    );
                    msg!("[Resolver] indices={:?}", mapping.indices);
                    return Err(ArbitrageError::InvalidAccountCount.into());
                }
            }
            DexType::PumpFunBondingCurve => {
                if !(3..=4).contains(&actual_len_u8) {
                    msg!("[Resolver] PumpFun indices mismatch: expected 3..=4 got {}", actual_len_u8);
                    msg!("[Resolver] indices={:?}", mapping.indices);
                    return Err(ArbitrageError::InvalidAccountCount.into());
                }
            }
            DexType::PumpSwap => {
                if !(4..=6).contains(&actual_len_u8) {
                    msg!("[Resolver] PumpSwap indices mismatch: expected 4..=6 got {}", actual_len_u8);
                    msg!("[Resolver] indices={:?}", mapping.indices);
                    return Err(ArbitrageError::InvalidAccountCount.into());
                }
            }
            _ => {
                if actual_len_u8 != expected {
                    msg!(
                        "[Resolver] {:?} indices mismatch: expected {} got {}",
                        mapping.dex_type,
                        expected,
                        actual_len_u8
                    );
                    msg!("[Resolver] indices={:?}", mapping.indices);
                    return Err(ArbitrageError::InvalidAccountCount.into());
                }
            }
        }

        // 越界与重复校验
        let mut seen: HashSet<u8> = HashSet::new();
        for (j, &idx) in mapping.indices.iter().enumerate() {
            let idx_usize = idx as usize;
            if idx_usize >= total {
                msg!(
                    "[Resolver] index out of bounds: j={} idx={} total_remaining={}",
                    j, idx, total
                );
                return Err(ArbitrageError::InvalidAccountIndex.into());
            }
            if !seen.insert(idx) {
                msg!("[Resolver] duplicated index detected: idx={}", idx);
                return Err(ArbitrageError::InvalidAccountIndex.into());
            }
        }

        // 角色顺序提示与 signer/writable 提示（仅日志）
        let roles = expected_roles(mapping.dex_type);
        let list_len = min(roles.len(), mapping.indices.len());
        for j in 0..list_len {
            let idx = mapping.indices[j] as usize;
            let ai = &self.remaining_accounts[idx];
            let role = roles[j];
            msg!(
                "[Resolver] role={} idx={} key={} W={} S={}",
                role,
                idx,
                ai.key(),
                ai.is_writable,
                ai.is_signer
            );
        }

        Ok(())
    }

    #[inline]
    fn ai(&self, idx: u8) -> Result<&'info AccountInfo<'info>> {
        self.remaining_accounts
            .get(idx as usize)
            .ok_or(ArbitrageError::InvalidAccountIndex.into())
    }
}

/// 期望的角色顺序（仅用于日志提示，帮助排查账户顺序问题）
fn expected_roles(dex_type: DexType) -> Vec<&'static str> {
    match dex_type {
        DexType::RaydiumCpmm => vec![
            "amm_config",
            "pool_state",
            "token0_vault",
            "token1_vault",
            "input_mint",
            "output_mint",
            "observation_state",
        ],
        DexType::RaydiumClmm => vec![
            "clmm_program",
            "amm_config",
            "pool_state",
            "input_vault",
            "output_vault",
            "observation_state",
            "token_program",
            "token_program_2022",
            "memo_program",
            "input_vault_mint",
            "output_vault_mint",
        ],
        DexType::PumpFunBondingCurve => vec![
            "bonding_curve",
            "mint",
            "creator",
        ],
        DexType::PumpSwap => vec![
            "pool_state",
            "base_mint",
            "quote_mint",
            "coin_creator",
        ],
    }
}