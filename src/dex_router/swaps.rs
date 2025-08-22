use anchor_lang::prelude::*;
use std::str::FromStr;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

use crate::account_derivation::DerivedAccounts;
use crate::account_derivation::ProgramIds;
use crate::account_derivation::types::instruction_discriminators::{
    RAYDIUM_CPMM_SWAP_BASE_IN,
    RAYDIUM_CLMM_SWAP_V2,
    PUMPFUN_BUY,
    PUMPSWAP_BUY,
};
use crate::account_resolver::{
    RaydiumCpmmAccounts, RaydiumClmmAccounts, PumpfunAccounts, PumpswapAccounts,
};
use crate::dex_router::types::{DexSwap, SwapResult};
use crate::errors::ArbitrageError;

// 说明：本文件仅保留四个 DEX 的“原生 invoke”骨架实现。
// 后续将基于各自 IDL/文档补齐 discriminator、账户顺序与数据编码。

pub struct RaydiumCpmmSwap;

impl<'info> DexSwap<'info> for RaydiumCpmmSwap {
    type Accounts = RaydiumCpmmAccounts<'info>;

    fn execute_swap(
        _accounts: Self::Accounts,
        _derived: &DerivedAccounts,
        _remaining_accounts: &'info [AccountInfo<'info>],
        _payer: &AccountInfo<'info>,
        _token_program: &AccountInfo<'info>,
        _associated_token_program: &AccountInfo<'info>,
        _system_program: &AccountInfo<'info>,
        _user_input_account: &AccountInfo<'info>,
        _user_output_account: &AccountInfo<'info>,
        _amount_in: u64,
        _minimum_amount_out: u64,
    ) -> Result<SwapResult> {
        // Helper to find AccountInfo by key within remaining accounts
        fn find_ai<'a>(ais: &'a [AccountInfo<'a>], key: &Pubkey) -> Result<&'a AccountInfo<'a>> {
            for ai in ais {
                if ai.key() == *key { return Ok(ai); }
            }
            Err(ArbitrageError::AccountNotFound.into())
        }

        // Resolve Raydium authority from derived fixed addresses, then fetch AccountInfo from remaining_accounts
        let fixed = _derived.get_fixed_addresses().ok_or(ArbitrageError::AccountNotFound)?;
        let authority_ai = find_ai(_remaining_accounts, &fixed.raydium_cpmm_authority)?;

        // Build instruction data: discriminator + amount_in + minimum_amount_out
        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(RAYDIUM_CPMM_SWAP_BASE_IN);
        data.extend_from_slice(&_amount_in.to_le_bytes());
        data.extend_from_slice(&_minimum_amount_out.to_le_bytes());

        // Choose token program for both legs (Token vs Token2022 已由上层选择传入对应 program)
        let token_prog_ai = _token_program;

        // Accounts metas in expected order (参考 Raydium cp-swap swap_base_input)
        let metas = vec![
            AccountMeta::new_readonly(_payer.key(), true),
            AccountMeta::new_readonly(authority_ai.key(), false),
            AccountMeta::new_readonly(_accounts.amm_config.key(), false),
            AccountMeta::new(_accounts.pool_state.key(), false),
            AccountMeta::new(_user_input_account.key(), false),
            AccountMeta::new(_user_output_account.key(), false),
            AccountMeta::new(_accounts.token0_vault.key(), false),
            AccountMeta::new(_accounts.token1_vault.key(), false),
            AccountMeta::new_readonly(token_prog_ai.key(), false),
            AccountMeta::new_readonly(token_prog_ai.key(), false),
            AccountMeta::new_readonly(_accounts.input_mint.key(), false),
            AccountMeta::new_readonly(_accounts.output_mint.key(), false),
            AccountMeta::new(_accounts.observation_state.key(), false),
        ];

        // Build AccountInfos in the same order
        let account_infos: Vec<AccountInfo<'info>> = vec![
            _payer.clone(),
            authority_ai.clone(),
            _accounts.amm_config.clone(),
            _accounts.pool_state.clone(),
            _user_input_account.clone(),
            _user_output_account.clone(),
            _accounts.token0_vault.clone(),
            _accounts.token1_vault.clone(),
            token_prog_ai.clone(),
            token_prog_ai.clone(),
            _accounts.input_mint.clone(),
            _accounts.output_mint.clone(),
            _accounts.observation_state.clone(),
        ];

        // Program id for Raydium CPMM
        let program_ids = ProgramIds::default();
        let ix = Instruction { program_id: program_ids.raydium_cpmm, accounts: metas, data };

        // Invoke
        invoke(&ix, &account_infos)?;

        // Amount out: 由上层读取或另行计算。此处先返回占位，fee 设 0。
        Ok(SwapResult { amount_out: _minimum_amount_out, fee_amount: 0 })
    }
}

pub struct RaydiumClmmSwap;

impl<'info> DexSwap<'info> for RaydiumClmmSwap {
    type Accounts = RaydiumClmmAccounts<'info>;

    fn execute_swap(
        _accounts: Self::Accounts,
        _derived: &DerivedAccounts,
        _remaining_accounts: &'info [AccountInfo<'info>],
        _payer: &AccountInfo<'info>,
        _token_program: &AccountInfo<'info>,
        _associated_token_program: &AccountInfo<'info>,
        _system_program: &AccountInfo<'info>,
        _user_input_account: &AccountInfo<'info>,
        _user_output_account: &AccountInfo<'info>,
        _amount_in: u64,
        _minimum_amount_out: u64,
    ) -> Result<SwapResult> {
        // Build instruction data
        let mut data = Vec::with_capacity(8 + 8 + 16 + 1);
        data.extend_from_slice(RAYDIUM_CLMM_SWAP_V2);
        data.extend_from_slice(&_amount_in.to_le_bytes());
        data.extend_from_slice(&_minimum_amount_out.to_le_bytes());
        data.extend_from_slice(&u128::MAX.to_le_bytes()); // sqrt_price_limit
        data.push(1); // is_base_input

        // Prefer token program from accounts
        let token_prog_ai = _accounts.token_program;

        let metas = vec![
            AccountMeta::new(_accounts.pool_state.key(), false),
            AccountMeta::new_readonly(_accounts.amm_config.key(), false),
            AccountMeta::new(_user_input_account.key(), false),
            AccountMeta::new(_user_output_account.key(), false),
            AccountMeta::new(_accounts.input_vault.key(), false),
            AccountMeta::new(_accounts.output_vault.key(), false),
            AccountMeta::new(_accounts.observation_state.key(), false),
            AccountMeta::new_readonly(token_prog_ai.key(), false),
        ];

        let program_id = _accounts.clmm_program.key();
        let ix = Instruction { program_id, accounts: metas, data };

        let account_infos: Vec<AccountInfo<'info>> = vec![
            _accounts.pool_state.clone(),
            _accounts.amm_config.clone(),
            _user_input_account.clone(),
            _user_output_account.clone(),
            _accounts.input_vault.clone(),
            _accounts.output_vault.clone(),
            _accounts.observation_state.clone(),
            token_prog_ai.clone(),
        ];

        invoke(&ix, &account_infos)?;
        Ok(SwapResult { amount_out: _minimum_amount_out, fee_amount: 0 })
    }
}

pub struct PumpfunSwap;

impl<'info> DexSwap<'info> for PumpfunSwap {
    type Accounts = PumpfunAccounts<'info>;

    fn execute_swap(
        _accounts: Self::Accounts,
        _derived: &DerivedAccounts,
        _remaining_accounts: &'info [AccountInfo<'info>],
        _payer: &AccountInfo<'info>,
        _token_program: &AccountInfo<'info>,
        _associated_token_program: &AccountInfo<'info>,
        _system_program: &AccountInfo<'info>,
        _user_input_account: &AccountInfo<'info>,
        _user_output_account: &AccountInfo<'info>,
        _amount_in: u64,
        _minimum_amount_out: u64,
    ) -> Result<SwapResult> {
        fn find_ai<'a>(ais: &'a [AccountInfo<'a>], key: &Pubkey) -> Result<&'a AccountInfo<'a>> {
            for ai in ais { if ai.key() == *key { return Ok(ai); } }
            Err(ArbitrageError::AccountNotFound.into())
        }

        // Fixed addresses
        let fixed = _derived.get_fixed_addresses().ok_or(ArbitrageError::AccountNotFound)?;
        let global_ai = find_ai(_remaining_accounts, &fixed.pumpfun_global_account)?;
        let fee_recipient_ai = find_ai(_remaining_accounts, &fixed.pumpfun_fee_recipient)?;
        let event_ai = find_ai(_remaining_accounts, &fixed.pumpfun_event_authority)?;

        // Try to locate associated accounts from remaining_accounts by scanning token accounts
        fn is_token_account_for(owner: &Pubkey, mint: &Pubkey, ai: &AccountInfo) -> bool {
            if ai.data_len() < 64 { return false; }
            if let Ok(data) = ai.try_borrow_data() {
                let mint_bytes = &data[0..32];
                let owner_bytes = &data[32..64];
                let mint_pk = Pubkey::new_from_array(mint_bytes.try_into().unwrap_or([0u8;32]));
                let owner_pk = Pubkey::new_from_array(owner_bytes.try_into().unwrap_or([0u8;32]));
                return &mint_pk == mint && &owner_pk == owner;
            }
            false
        }

        // bonding_curve ATA
        let associated_bonding_curve_ai = _remaining_accounts.iter()
            .find(|ai| is_token_account_for(&_accounts.bonding_curve.key(), &_accounts.mint.key(), ai))
            .ok_or(ArbitrageError::AccountNotFound)?;

        // user ATA
        let associated_user_ai = _remaining_accounts.iter()
            .find(|ai| is_token_account_for(&_payer.key(), &_accounts.mint.key(), ai))
            .ok_or(ArbitrageError::AccountNotFound)?;

        // Optional rent sysvar
        let rent_sysvar = Pubkey::from_str("SysvarRent111111111111111111111111111111111").unwrap();
        let maybe_rent = _remaining_accounts.iter().find(|ai| ai.key() == rent_sysvar);

        // Build data (use BUY by default)
        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(PUMPFUN_BUY);
        data.extend_from_slice(&_amount_in.to_le_bytes());
        data.extend_from_slice(&_minimum_amount_out.to_le_bytes());

        let mut metas = vec![
            AccountMeta::new_readonly(global_ai.key(), false),
            AccountMeta::new(fee_recipient_ai.key(), false),
            AccountMeta::new_readonly(_accounts.mint.key(), false),
            AccountMeta::new(_accounts.bonding_curve.key(), false),
            AccountMeta::new(associated_bonding_curve_ai.key(), false),
            AccountMeta::new(associated_user_ai.key(), false),
            AccountMeta::new(_payer.key(), true),
            AccountMeta::new_readonly(_system_program.key(), false),
            AccountMeta::new_readonly(_token_program.key(), false),
        ];
        if let Some(r) = maybe_rent { metas.push(AccountMeta::new_readonly(r.key(), false)); }
        metas.push(AccountMeta::new_readonly(event_ai.key(), false));

        let program_ids = ProgramIds::default();
        let ix = Instruction { program_id: program_ids.pumpfun, accounts: metas, data };

        let mut account_infos: Vec<AccountInfo<'info>> = vec![
            global_ai.clone(),
            fee_recipient_ai.clone(),
            _accounts.mint.clone(),
            _accounts.bonding_curve.clone(),
            associated_bonding_curve_ai.clone(),
            associated_user_ai.clone(),
            _payer.clone(),
            _system_program.clone(),
            _token_program.clone(),
        ];
        if let Some(r) = maybe_rent { account_infos.push(r.clone()); }
        account_infos.push(event_ai.clone());

        invoke(&ix, &account_infos)?;
        Ok(SwapResult { amount_out: _minimum_amount_out, fee_amount: 0 })
    }
}

pub struct PumpswapSwap;

impl<'info> DexSwap<'info> for PumpswapSwap {
    type Accounts = PumpswapAccounts<'info>;

    fn execute_swap(
        _accounts: Self::Accounts,
        _derived: &DerivedAccounts,
        _remaining_accounts: &'info [AccountInfo<'info>],
        _payer: &AccountInfo<'info>,
        _token_program: &AccountInfo<'info>,
        _associated_token_program: &AccountInfo<'info>,
        _system_program: &AccountInfo<'info>,
        _user_input_account: &AccountInfo<'info>,
        _user_output_account: &AccountInfo<'info>,
        _amount_in: u64,
        _minimum_amount_out: u64,
    ) -> Result<SwapResult> {
        fn find_ai<'a>(ais: &'a [AccountInfo<'a>], key: &Pubkey) -> Result<&'a AccountInfo<'a>> {
            for ai in ais { if ai.key() == *key { return Ok(ai); } }
            Err(ArbitrageError::AccountNotFound.into())
        }

        let fixed = _derived.get_fixed_addresses().ok_or(ArbitrageError::AccountNotFound)?;
        let global_cfg_ai = find_ai(_remaining_accounts, &fixed.pumpswap_global_config)?;
        let fee_recipient_ai = find_ai(_remaining_accounts, &fixed.pumpswap_fee_recipient)?;

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(PUMPSWAP_BUY);
        data.extend_from_slice(&_amount_in.to_le_bytes());
        data.extend_from_slice(&_minimum_amount_out.to_le_bytes());

        let metas = vec![
            AccountMeta::new_readonly(global_cfg_ai.key(), false),
            AccountMeta::new(fee_recipient_ai.key(), false),
            AccountMeta::new_readonly(_accounts.base_mint.key(), false),
            AccountMeta::new(_accounts.pool_state.key(), false),
            AccountMeta::new(_user_input_account.key(), false),
            AccountMeta::new(_user_output_account.key(), false),
        ];

        let account_infos: Vec<AccountInfo<'info>> = vec![
            global_cfg_ai.clone(),
            fee_recipient_ai.clone(),
            _accounts.base_mint.clone(),
            _accounts.pool_state.clone(),
            _user_input_account.clone(),
            _user_output_account.clone(),
        ];

        let program_id = fixed.pumpswap_amm_program;
        let ix = Instruction { program_id, accounts: metas, data };
        invoke(&ix, &account_infos)?;
        Ok(SwapResult { amount_out: _minimum_amount_out, fee_amount: 0 })
    }
}


