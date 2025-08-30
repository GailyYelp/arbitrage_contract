use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

use crate::account_derivation::DerivedAccounts;
// use crate::account_derivation::ProgramIds;
use crate::account_derivation::types::instruction_discriminators::{
    RAYDIUM_CPMM_SWAP_BASE_IN,
    RAYDIUM_CLMM_SWAP_V2,
    PUMPFUN_BUY,
    PUMPFUN_SELL,
    PUMPSWAP_BUY,
};
use crate::account_resolver::{
    RaydiumCpmmAccounts, RaydiumClmmAccounts, PumpfunAccounts, PumpswapAccounts,
};
use crate::dex_router::types::{DexSwap, SwapResult};
use crate::errors::ArbitrageError;
use crate::account_derivation::types::{pda_utils, pda_seeds};

// 说明：本文件采用 Anchor+原生 invoke 的混合模式。
// 作用：按解析出的 DEX 账户，直接构造外部 DEX 指令（discriminator+data+metas），
// 利用 invoke 执行，前后读取用户输出 ATA 余额差以得到真实 amount_out，供链上滑点校验使用。

/// 读取 SPL Token(或Token-2022) 账户的 amount 字段（余额差法）
fn read_token_amount<'info>(ai: &AccountInfo<'info>) -> Result<u64> {
    // 至少包含 mint(32) + owner(32) + amount(u64) = 72 字节
    if ai.data_len() < 72 {
        return Err(ArbitrageError::InvalidAccount.into());
    }
    let data = ai.try_borrow_data()?;
    let mut amount_bytes = [0u8; 8];
    amount_bytes.copy_from_slice(&data[64..72]);
    Ok(u64::from_le_bytes(amount_bytes))
}

// 通用工具：在 remaining_accounts 中按 Pubkey 查找 AccountInfo
fn find_ai<'a>(ais: &'a [AccountInfo<'a>], key: &Pubkey) -> Result<&'a AccountInfo<'a>> {
    for ai in ais {
        if ai.key() == *key { return Ok(ai); }
    }
    Err(ArbitrageError::AccountNotFound.into())
}

// 通用工具：读取 token 账户的 mint（前 32 字节）
fn token_account_mint(ai: &AccountInfo) -> Option<Pubkey> {
    if ai.data_len() < 32 { return None; }
    if let Ok(data) = ai.try_borrow_data() {
        let mut mint_bytes = [0u8;32];
        mint_bytes.copy_from_slice(&data[0..32]);
        return Some(Pubkey::new_from_array(mint_bytes));
    }
    None
}

// 通用工具：判断某 AccountInfo 是否为指定 owner+mint 的 SPL(Token/2022) 账户
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

// 通用工具：在 remaining_accounts 中查找 owner+mint 对应的 token 账户
fn find_ata<'a>(ais: &'a [AccountInfo<'a>], owner: &Pubkey, mint: &Pubkey) -> Option<&'a AccountInfo<'a>> {
    for ai in ais {
        if is_token_account_for(owner, mint, ai) { return Some(ai); }
    }
    None
}

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
        let pre_out = read_token_amount(_user_output_account)?;

        // Resolve Raydium authority from derived fixed addresses, then fetch AccountInfo from remaining_accounts
        let fixed = _derived.get_fixed_addresses().ok_or(ArbitrageError::AccountNotFound)?;
        let authority_ai = find_ai(_remaining_accounts, &fixed.raydium_cpmm_authority)?;
        // Program account (required by invoke): derive from state owner to兼容不同网络
        let cpmm_program_id = *_accounts.amm_config.owner;
        let cpmm_program_ai = find_ai(_remaining_accounts, &cpmm_program_id)?;
        // 安全校验：仅要求可执行，具体 program_id 由客户端传入并与状态账户 owner 一致
        require!(cpmm_program_ai.executable, ArbitrageError::InvalidAccount);
        msg!("[CPMM] program_id={} ok", cpmm_program_ai.key());

        // Build instruction data: discriminator + amount_in + minimum_amount_out
        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(RAYDIUM_CPMM_SWAP_BASE_IN);
        data.extend_from_slice(&_amount_in.to_le_bytes());
        data.extend_from_slice(&_minimum_amount_out.to_le_bytes());

        // 为输入/输出 mint 分别选择 Token/Token-2022 程序（根据 mint.owner 动态判定）
        let input_prog_pk = *_accounts.input_mint.owner;
        let output_prog_pk = *_accounts.output_mint.owner;
        let token_prog_tokenkeg = _token_program.key();
        let input_token_prog_ai = if input_prog_pk == token_prog_tokenkeg {
            _token_program.clone()
        } else {
            // 例如 Token-2022 程序需从 remaining_accounts 定位
            find_ai(_remaining_accounts, &input_prog_pk)?.clone()
        };
        let output_token_prog_ai = if output_prog_pk == token_prog_tokenkeg {
            _token_program.clone()
        } else {
            find_ai(_remaining_accounts, &output_prog_pk)?.clone()
        };

        // 根据输入 mint 动态选择 input_vault/output_vault，确保与 input_token_mint/output_token_mint 一致
        let input_mint_key = _accounts.input_mint.key();
        let token0_mint = token_account_mint(&_accounts.token0_vault).ok_or(ArbitrageError::InvalidTokenMint)?;
        let token1_mint = token_account_mint(&_accounts.token1_vault).ok_or(ArbitrageError::InvalidTokenMint)?;
        let (input_vault_ai, output_vault_ai) = if token0_mint == input_mint_key {
            (_accounts.token0_vault.clone(), _accounts.token1_vault.clone())
        } else if token1_mint == input_mint_key {
            (_accounts.token1_vault.clone(), _accounts.token0_vault.clone())
        } else {
            return Err(ArbitrageError::InvalidTokenMint.into());
        };

        // Accounts metas in expected order (参考 Raydium cp-swap swap_base_input)
        let metas = vec![
            AccountMeta::new_readonly(_payer.key(), true),
            AccountMeta::new_readonly(authority_ai.key(), false),
            AccountMeta::new_readonly(_accounts.amm_config.key(), false),
            AccountMeta::new(_accounts.pool_state.key(), false),
            AccountMeta::new(_user_input_account.key(), false),
            AccountMeta::new(_user_output_account.key(), false),
            AccountMeta::new(input_vault_ai.key(), false),
            AccountMeta::new(output_vault_ai.key(), false),
            AccountMeta::new_readonly(input_token_prog_ai.key(), false),
            AccountMeta::new_readonly(output_token_prog_ai.key(), false),
            AccountMeta::new_readonly(_accounts.input_mint.key(), false),
            AccountMeta::new_readonly(_accounts.output_mint.key(), false),
            AccountMeta::new(_accounts.observation_state.key(), false),
        ];

        let account_infos: Vec<AccountInfo<'info>> = vec![
            _payer.clone(),
            authority_ai.clone(),
            _accounts.amm_config.clone(),
            _accounts.pool_state.clone(),
            _user_input_account.clone(),
            _user_output_account.clone(),
            input_vault_ai.clone(),
            output_vault_ai.clone(),
            input_token_prog_ai.clone(),
            output_token_prog_ai.clone(),
            _accounts.input_mint.clone(),
            _accounts.output_mint.clone(),
            _accounts.observation_state.clone(),
           // Raydium CPMM 程序账户（从 remaining_accounts 查找）
            cpmm_program_ai.clone(),
        ];

        // Program id for Raydium CPMM（使用状态账户的 owner 推导出的程序ID）
        let program_id = cpmm_program_id;
        let ix = Instruction { program_id, accounts: metas, data };

        // Invoke
        invoke(&ix, &account_infos)?;

        // 读取执行后余额并计算真实产出
        let post_out = read_token_amount(_user_output_account)?;
        let amount_out = post_out.saturating_sub(pre_out);
        Ok(SwapResult { amount_out, fee_amount: 0 })
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
        let pre_out = read_token_amount(_user_output_account)?;
        // Build instruction data
        let mut data = Vec::with_capacity(8 + 8 + 16 + 1);
        data.extend_from_slice(RAYDIUM_CLMM_SWAP_V2);
        data.extend_from_slice(&_amount_in.to_le_bytes());
        data.extend_from_slice(&_minimum_amount_out.to_le_bytes());
        data.extend_from_slice(&u128::MAX.to_le_bytes()); // sqrt_price_limit
        data.push(1); // is_base_input

        // Prefer token program from accounts
        let token_prog_ai = _accounts.token_program;

        let mut metas = vec![
            AccountMeta::new_readonly(_payer.key(), true),
            AccountMeta::new_readonly(_accounts.amm_config.key(), false),
            AccountMeta::new(_accounts.pool_state.key(), false),
            AccountMeta::new(_user_input_account.key(), false),
            AccountMeta::new(_user_output_account.key(), false),
            AccountMeta::new(_accounts.input_vault.key(), false),
            AccountMeta::new(_accounts.output_vault.key(), false),
            AccountMeta::new(_accounts.observation_state.key(), false),
            AccountMeta::new_readonly(token_prog_ai.key(), false),
            AccountMeta::new_readonly(_accounts.token_program_2022.key(), false),
            AccountMeta::new_readonly(_accounts.memo_program.key(), false),
            AccountMeta::new_readonly(_accounts.input_vault_mint.key(), false),
            AccountMeta::new_readonly(_accounts.output_vault_mint.key(), false),
        ];

        // 安全校验：CLMM 程序账户（来自 indices）必须为可执行程序
        require!(_accounts.clmm_program.executable, ArbitrageError::InvalidAccount);
        msg!("[CLMM] program_id={} ok", _accounts.clmm_program.key());
        // 先构建基础 account_infos
        let mut account_infos: Vec<AccountInfo<'info>> = vec![
            _payer.clone(),
            _accounts.amm_config.clone(),
            _accounts.pool_state.clone(),
            _user_input_account.clone(),
            _user_output_account.clone(),
            _accounts.input_vault.clone(),
            _accounts.output_vault.clone(),
            _accounts.observation_state.clone(),
            token_prog_ai.clone(),
            _accounts.token_program_2022.clone(),
            _accounts.memo_program.clone(),
            _accounts.input_vault_mint.clone(),
            _accounts.output_vault_mint.clone(),
            // CLMM 程序账户
            _accounts.clmm_program.clone(),
        ];

        // 动态补充：从 remaining_accounts 追加与 CLMM 程序相关且不在基础集中的账户（例如 tick arrays/extension）
        let clmm_program_id = _accounts.clmm_program.key();
        use std::collections::HashSet as _HashSet;
        let mut base_keys: _HashSet<Pubkey> = _HashSet::new();
        for ai in account_infos.iter() { base_keys.insert(ai.key()); }
        for ai in _remaining_accounts.iter() {
            if ai.owner != &clmm_program_id { continue; }
            if base_keys.contains(&ai.key()) { continue; }
            // 与引擎对齐：tick arrays 与扩展在引擎侧以可写形式传递
            metas.push(AccountMeta::new(ai.key(), false));
            account_infos.push(ai.clone());
            base_keys.insert(ai.key());
        }

        let program_id = clmm_program_id;
        let ix = Instruction { program_id, accounts: metas.clone(), data };

        // account_infos 已在上方构建并包含动态追加

        invoke(&ix, &account_infos)?;
        let post_out = read_token_amount(_user_output_account)?;
        let amount_out = post_out.saturating_sub(pre_out);
        Ok(SwapResult { amount_out, fee_amount: 0 })
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
        let pre_out = read_token_amount(_user_output_account)?;

        // 先确定 pumpfun 程序ID（来自入参账户 owner）
        let pumpfun_program_id = *_accounts.bonding_curve.owner;
        // Fixed addresses（从配置加载；若 devnet 值不同，需由配置覆盖并在全局表提供对应账户）
        let fixed = _derived.get_fixed_addresses().ok_or(ArbitrageError::AccountNotFound)?;
        // 优先用 PDA 派生并在全局表定位（global 与 event_authority），失败再回退到固定地址
        let (global_key, _) = Pubkey::find_program_address(&[pda_seeds::PUMPFUN_GLOBAL], &pumpfun_program_id);
        let global_ai = match find_ai(_remaining_accounts, &global_key) {
            Ok(ai) => ai,
            Err(_) => find_ai(_remaining_accounts, &fixed.pumpfun_global_account)?,
        };
        let (event_key, _) = Pubkey::find_program_address(&[pda_seeds::PUMPFUN_EVENT_AUTHORITY], &pumpfun_program_id);
        let event_ai = match find_ai(_remaining_accounts, &event_key) {
            Ok(ai) => ai,
            Err(_) => find_ai(_remaining_accounts, &fixed.pumpfun_event_authority)?,
        };
        // fee_recipient：若可选索引提供则优先，否则回退到固定地址
        let fee_recipient_ai = if let Some(fr) = _accounts.fee_recipient_opt { fr } else { find_ai(_remaining_accounts, &fixed.pumpfun_fee_recipient)? };

        // 通过扫描 token 账户数据在全局表定位所需账户
        let associated_bonding_curve_ai = _remaining_accounts.iter()
            .find(|ai| is_token_account_for(&_accounts.bonding_curve.key(), &_accounts.mint.key(), ai))
            .ok_or(ArbitrageError::AccountNotFound)?;
        let associated_user_ai = _remaining_accounts.iter()
            .find(|ai| is_token_account_for(&_payer.key(), &_accounts.mint.key(), ai))
            .ok_or(ArbitrageError::AccountNotFound)?;

        // 追加：creator_vault（PDA）使用“传入的 pumpfun 程序”派生，兼容不同网络
        let creator_key = _accounts.creator.key();
        let expected_creator_vault = pda_utils::derive_pumpfun_creator_vault(&creator_key, &pumpfun_program_id)
            .map_err(|_| ArbitrageError::AccountNotFound)?;
        let creator_vault_ai = find_ai(_remaining_accounts, &expected_creator_vault)?;

        // 基于输入/输出账户的 mint 与 WSOL 判断买/卖方向
        let in_mint = token_account_mint(_user_input_account).ok_or(ArbitrageError::InvalidAccount)?;
        let out_mint = token_account_mint(_user_output_account).ok_or(ArbitrageError::InvalidAccount)?;
        let wsol = fixed.wrapped_sol_mint;
        let is_buy = in_mint == wsol; // 用 SOL 买代币
        let is_sell = out_mint == wsol; // 卖代币换 SOL

        // volume accumulators（仅买入路径尽力追加，不阻塞）
        let (maybe_gva_ai, maybe_uva_ai) = if is_buy {
            let maybe_gva_key = pda_utils::derive_pumpfun_global_volume_accumulator(&pumpfun_program_id).ok();
            let maybe_uva_key = pda_utils::derive_pumpfun_user_volume_accumulator(&_payer.key(), &pumpfun_program_id).ok();
            (
                if let Some(k) = maybe_gva_key { _remaining_accounts.iter().find(|ai| ai.key() == k) } else { None },
                if let Some(k) = maybe_uva_key { _remaining_accounts.iter().find(|ai| ai.key() == k) } else { None },
            )
        } else { (None, None) };

        // 构造 data 与账户顺序（严格按 BUY/SELL 对齐）
        let (data, metas): (Vec<u8>, Vec<AccountMeta>) = if is_buy {
            // BUY: data = [BUY, token_amount, max_sol_cost] → 使用 min_out 作为 token_amount，上界用 amount_in
            let mut data = Vec::with_capacity(8 + 8 + 8);
            data.extend_from_slice(PUMPFUN_BUY);
            data.extend_from_slice(&_minimum_amount_out.to_le_bytes()); // token_amount
            data.extend_from_slice(&_amount_in.to_le_bytes());          // max_sol_cost

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
                AccountMeta::new(creator_vault_ai.key(), false),
                AccountMeta::new_readonly(event_ai.key(), false),
            ];
            if let Some(gva) = maybe_gva_ai { metas.push(AccountMeta::new(gva.key(), false)); }
            if let Some(uva) = maybe_uva_ai { metas.push(AccountMeta::new(uva.key(), false)); }
            (data, metas)
        } else if is_sell {
            // SELL: data = [SELL, token_amount, min_sol_output] → 使用 amount_in 作为 token_amount，min_out 保持
            let mut data = Vec::with_capacity(8 + 8 + 8);
            data.extend_from_slice(PUMPFUN_SELL);
            data.extend_from_slice(&_amount_in.to_le_bytes());          // token_amount
            data.extend_from_slice(&_minimum_amount_out.to_le_bytes());  // min_sol_output

            // 注意 sell 的账户顺序：creator_vault 在 token_program 之前
            let metas = vec![
                AccountMeta::new_readonly(global_ai.key(), false),
                AccountMeta::new(fee_recipient_ai.key(), false),
                AccountMeta::new_readonly(_accounts.mint.key(), false),
                AccountMeta::new(_accounts.bonding_curve.key(), false),
                AccountMeta::new(associated_bonding_curve_ai.key(), false),
                AccountMeta::new(associated_user_ai.key(), false),
                AccountMeta::new(_payer.key(), true),
                AccountMeta::new_readonly(_system_program.key(), false),
                AccountMeta::new(creator_vault_ai.key(), false),
                AccountMeta::new_readonly(_token_program.key(), false),
                AccountMeta::new_readonly(event_ai.key(), false),
            ];
            (data, metas)
        } else {
            // 既不是买也不是卖（非 SOL 对），视为无效
            return Err(ArbitrageError::InvalidAccount.into());
        };

        let ix = Instruction { program_id: pumpfun_program_id, accounts: metas, data };

        let mut account_infos: Vec<AccountInfo<'info>> = vec![
            global_ai.clone(),
            fee_recipient_ai.clone(),
            _accounts.mint.clone(),
            _accounts.bonding_curve.clone(),
            associated_bonding_curve_ai.clone(),
            associated_user_ai.clone(),
            _payer.clone(),
            _system_program.clone(),
        ];
        if is_sell {
            // sell: creator_vault 在 token_program 之前
            account_infos.push(creator_vault_ai.clone());
            account_infos.push(_token_program.clone());
        } else {
            // buy: token_program 在 creator_vault 之前
            account_infos.push(_token_program.clone());
            account_infos.push(creator_vault_ai.clone());
        }
        account_infos.push(event_ai.clone());
        if let Some(gva) = maybe_gva_ai { account_infos.push(gva.clone()); }
        if let Some(uva) = maybe_uva_ai { account_infos.push(uva.clone()); }

        // Pumpfun 程序账户（invoke 需要程序 AccountInfo）：仅校验可执行
        let pumpfun_program_ai = find_ai(_remaining_accounts, &pumpfun_program_id)?;
        require!(pumpfun_program_ai.executable, ArbitrageError::InvalidAccount);
        msg!("[PumpFun] program_id={} ok", pumpfun_program_ai.key());
        account_infos.push(pumpfun_program_ai.clone());

        invoke(&ix, &account_infos)?;
        let post_out = read_token_amount(_user_output_account)?;
        let amount_out = post_out.saturating_sub(pre_out);
        Ok(SwapResult { amount_out, fee_amount: 0 })
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
        let pre_out = read_token_amount(_user_output_account)?;
        let fixed = _derived.get_fixed_addresses().ok_or(ArbitrageError::AccountNotFound)?;

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(PUMPSWAP_BUY);
        data.extend_from_slice(&_amount_in.to_le_bytes());
        data.extend_from_slice(&_minimum_amount_out.to_le_bytes());

        // 解析用户与池两侧 ATAs（根据 mint 判定 input/output 的归属）
        let base_mint = _accounts.base_mint.key();
        let quote_mint = _accounts.quote_mint.key();
        let (user_base_ata_ai, user_quote_ata_ai) = match (token_account_mint(_user_input_account), token_account_mint(_user_output_account)) {
            (Some(m0), Some(_m1)) => {
                let a = if m0 == base_mint { _user_input_account } else { _user_output_account };
                let b = if m0 == base_mint { _user_output_account } else { _user_input_account };
                (a, b)
            }
            _ => (_user_input_account, _user_output_account),
        };

        // 期望地址（用于在 remaining_accounts 中查找）：pool 两侧、fee_recipient_ata、creator_vault_*、event_authority、amm_program
        let pool_key = _accounts.pool_state.key();
        
        // AMM 程序账户：仅校验可执行；兼容不同网络的程序ID
        let amm_program_ai = match find_ai(_remaining_accounts, &fixed.pumpswap_amm_program) {
            Ok(ai) => ai,
            Err(_) => {
                // 若配置中的固定ID未找到，则在 remaining_accounts 中寻找任一可执行账户作为 AMM 程序（宽松）
                let mut found: Option<&AccountInfo> = None;
                for ai in _remaining_accounts.iter() {
                    if ai.executable { found = Some(ai); break; }
                }
                found.ok_or(ArbitrageError::AccountNotFound)?
            }
        };
        require!(amm_program_ai.executable, ArbitrageError::InvalidAccount);
        // derive global_config 与 event_authority PDA 并在 remaining_accounts 中定位（失败回退 fixed）
        let amm_pid = amm_program_ai.key();
        let (global_cfg_key, _) = Pubkey::find_program_address(&[pda_seeds::PUMPSWAP_GLOBAL_CONFIG], &amm_pid);
        let global_cfg_ai = match find_ai(_remaining_accounts, &global_cfg_key) {
            Ok(ai) => ai,
            Err(_) => find_ai(_remaining_accounts, &fixed.pumpswap_global_config)?,
        };
        let (event_auth_key, _) = Pubkey::find_program_address(&[pda_seeds::PUMPSWAP_EVENT_AUTHORITY], &amm_pid);
        let event_authority_ai = match find_ai(_remaining_accounts, &event_auth_key) {
            Ok(ai) => ai,
            Err(_) => find_ai(_remaining_accounts, &fixed.pumpswap_event_authority)?,
        };
        // fee_recipient 及其 ATA：若可选索引提供则优先，否则回退 fixed/扫描
        let fee_recipient_ai = if let Some(fr) = _accounts.fee_recipient_opt { fr } else { find_ai(_remaining_accounts, &fixed.pumpswap_fee_recipient)? };
        let fee_recipient_key = fee_recipient_ai.key();
        // creator_vault 派生
        let creator_key = _accounts.coin_creator.key();
        let creator_vault_authority_key = crate::account_derivation::types::pda_utils::derive_pumpswap_creator_vault(&creator_key, &amm_pid)
            .map_err(|_| ArbitrageError::AccountNotFound)?;
        let creator_vault_authority_ai = find_ai(_remaining_accounts, &creator_vault_authority_key)?;
        // 查找池/fee/creator 的 ATAs（通过 owner+mint 扫描找到 AccountInfo）
        let pool_base_ata_ai = find_ata(_remaining_accounts, &pool_key, &base_mint).ok_or(ArbitrageError::AccountNotFound)?;
        let pool_quote_ata_ai = find_ata(_remaining_accounts, &pool_key, &quote_mint).ok_or(ArbitrageError::AccountNotFound)?;
        let fee_recipient_ata_ai = if let Some(fra) = _accounts.fee_recipient_ata_opt { fra } else { find_ata(_remaining_accounts, &fee_recipient_key, &quote_mint).ok_or(ArbitrageError::AccountNotFound)? };
        let creator_vault_ata_ai = find_ata(_remaining_accounts, &creator_vault_authority_key, &quote_mint).ok_or(ArbitrageError::AccountNotFound)?;

        // 账户 metas（参照引擎构造顺序）
        let metas = vec![
            AccountMeta::new_readonly(_accounts.pool_state.key(), false), // pool
            AccountMeta::new(_payer.key(), true),                          // user
            AccountMeta::new_readonly(global_cfg_ai.key(), false),        // global
            AccountMeta::new_readonly(_accounts.base_mint.key(), false),  // base_mint
            AccountMeta::new_readonly(_accounts.quote_mint.key(), false), // quote_mint
            AccountMeta::new(user_base_ata_ai.key(), false),              // user_base_ata
            AccountMeta::new(user_quote_ata_ai.key(), false),             // user_quote_ata
            AccountMeta::new(pool_base_ata_ai.key(), false),              // pool_base_ata
            AccountMeta::new(pool_quote_ata_ai.key(), false),             // pool_quote_ata
            AccountMeta::new_readonly(fee_recipient_ai.key(), false),     // fee_recipient
            AccountMeta::new(fee_recipient_ata_ai.key(), false),          // fee_recipient_ata
            AccountMeta::new_readonly(_token_program.key(), false),       // base_token_program
            AccountMeta::new_readonly(_token_program.key(), false),       // quote_token_program
            AccountMeta::new_readonly(_system_program.key(), false),      // system_program
            AccountMeta::new_readonly(_associated_token_program.key(), false), // associated_token_program
            AccountMeta::new_readonly(event_authority_ai.key(), false),   // event_authority
            AccountMeta::new_readonly(amm_program_ai.key(), false),       // amm_program
            AccountMeta::new(creator_vault_ata_ai.key(), false),          // creator_vault_ata
            AccountMeta::new_readonly(creator_vault_authority_ai.key(), false), // creator_vault_authority
        ];

        let account_infos: Vec<AccountInfo<'info>> = vec![
            _accounts.pool_state.clone(),
            _payer.clone(),
            global_cfg_ai.clone(),
            _accounts.base_mint.clone(),
            _accounts.quote_mint.clone(),
            user_base_ata_ai.clone(),
            user_quote_ata_ai.clone(),
            pool_base_ata_ai.clone(),
            pool_quote_ata_ai.clone(),
            fee_recipient_ai.clone(),
            fee_recipient_ata_ai.clone(),
            _token_program.clone(),
            _token_program.clone(),
            _system_program.clone(),
            _associated_token_program.clone(),
            event_authority_ai.clone(),
            amm_program_ai.clone(),
            creator_vault_ata_ai.clone(),
            creator_vault_authority_ai.clone(),
        ];
        msg!("[PumpSwap] program_id={} ok", amm_program_ai.key());
        let program_id = amm_program_ai.key();
        let ix = Instruction { program_id, accounts: metas, data };
        invoke(&ix, &account_infos)?;
        let post_out = read_token_amount(_user_output_account)?;
        let amount_out = post_out.saturating_sub(pre_out);
        Ok(SwapResult { amount_out, fee_amount: 0 })
    }
}


