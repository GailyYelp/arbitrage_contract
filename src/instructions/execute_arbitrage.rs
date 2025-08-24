use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::{ArbitrageParams, PathAccountMappingV2, DexType};
use crate::account_resolver::AccountResolver;
use crate::account_derivation::{DerivedAccounts, ProgramIds};
use crate::dex_router::{DexRouter, DexAccounts};
use crate::errors::ArbitrageError;
use anchor_lang::solana_program as _; // ensure linkage

// ==============================================================================================
// 合约入口（V2 indices 协议）
// 作用：
// - 按客户端提供的全局账户表 + indices 解析每步 DEX 账户；
// - 每步读取真实 amount_out（余额差）、校验 minimum_amount_out；
// - 终局校验 min_profit_lamports，原子回滚失败路径；
// - Token/Token-2022 的用户 ATA 做 owner/mint/program 快速校验；
// - 打印 CPI_VERSION 和 remaining_accounts 快照（len/hash）用于双端排错；
// - 调用前统一初始化 DerivedAccounts（固定地址/系统程序）。
// ==============================================================================================

#[derive(Accounts)]
pub struct ExecuteArbitrage<'info> {
    #[account(mut)]
    pub user: Signer<'info>,   
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn execute_arbitrage<'info>(
    ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage<'info>>,
    params: ArbitrageParams,
) -> Result<()> {
    // 1. 验证参数
    require!(!params.path_steps.is_empty(), ArbitrageError::PathTooShort);
    require!(params.path_steps.len() <= 10, ArbitrageError::PathTooLong);
    require!(params.input_amount > 0, ArbitrageError::InvalidAmount);
    require!(params.account_mappings_v2.len() == params.path_steps.len(), ArbitrageError::InvalidAccountCount);
    
    // 2. 初始化程序ID配置
    let program_ids = ProgramIds::default(); // 与客户端常量保持一致
    msg!("[CPI_VERSION] {}", crate::dex_router::types::constants::CPI_VERSION);
    msg!(
        "[PROGRAM_IDS] token={} token22={} assoc_token={} system={} cpmm={} clmm={} pumpfun={} pumpswap={}",
        program_ids.token_program,
        program_ids.token_2022_program,
        program_ids.associated_token_program,
        program_ids.system_program,
        program_ids.raydium_cpmm,
        program_ids.raydium_clmm,
        program_ids.pumpfun,
        program_ids.pumpswap
    );
    // 基础系统程序一致性校验（不影响 Token/Token-2022 选择）
    require!(
        ctx.accounts.associated_token_program.key() == program_ids.associated_token_program,
        ArbitrageError::InvalidAccount
    );
    require!(
        ctx.accounts.system_program.key() == program_ids.system_program,
        ArbitrageError::InvalidAccount
    );
    
    // 3. 推导所有需要的账户
    let mut derived_accounts = DerivedAccounts::new();
    // 初始化固定地址与系统程序表
    derived_accounts.initialize(&program_ids)?;
    derived_accounts.derive_for_path(
        &params.path_steps,
        &ctx.accounts.user.key(),
        &program_ids,
        ctx.remaining_accounts,
    )?;
    
    // 4. 执行实际的套利路径
    let mut current_amount = params.input_amount;
    // 可选：账户表快照已移除（减少日志噪音）
    
    for (step_index, step) in params.path_steps.iter().enumerate() {
        msg!("Executing step {}: {:?} -> {:?} on {:?}", 
             step_index, step.input_mint, step.output_mint, step.dex_type);
        msg!(
            "Step {} inputs: amount_in={}, min_out={}",
            step_index,
            current_amount,
            step.minimum_amount_out
        );
        
        // 获取当前步骤的账户映射（V2 indices 协议）
        let mapping: &PathAccountMappingV2 = &params.account_mappings_v2[step_index];
        
        // 创建临时的账户解析器 
        let account_resolver = AccountResolver::new(ctx.remaining_accounts);
        
        // 验证账户映射（indices 数量）
        account_resolver.validate_indices_for_dex(mapping)?;
        
        // 解析这一步需要的 DEX 账户
        let dex_accounts = match step.dex_type {
            DexType::RaydiumCpmm => {
                DexAccounts::RaydiumCpmm(account_resolver.resolve_raydium_cpmm_by_indices(mapping)?)
            }
            DexType::RaydiumClmm => {
                DexAccounts::RaydiumClmm(account_resolver.resolve_raydium_clmm_by_indices(mapping)?)
            }
            DexType::PumpFunBondingCurve => {
                DexAccounts::Pumpfun(account_resolver.resolve_pumpfun_by_indices(mapping)?)
            }
            DexType::PumpSwap => {
                DexAccounts::Pumpswap(account_resolver.resolve_pumpswap_by_indices(mapping)?)
            }
        };

        // 版本治理辅助日志：打印关键账户指纹（长度 + 头8字节），用于多版本池/配置识别
        match step.dex_type {
            DexType::RaydiumCpmm => {
                let pool_idx = mapping.indices[1] as usize; // pool_state
                let cfg_idx = mapping.indices[0] as usize;  // amm_config
                if let Some(ai) = ctx.remaining_accounts.get(pool_idx) {
                    log_account_fingerprint(ai, "CPMM.pool_state");
                }
                if let Some(ai) = ctx.remaining_accounts.get(cfg_idx) {
                    log_account_fingerprint(ai, "CPMM.amm_config");
                }
            }
            DexType::RaydiumClmm => {
                let pool_idx = mapping.indices[2] as usize; // pool_state
                let cfg_idx = mapping.indices[1] as usize;  // amm_config
                if let Some(ai) = ctx.remaining_accounts.get(pool_idx) {
                    log_account_fingerprint(ai, "CLMM.pool_state");
                }
                if let Some(ai) = ctx.remaining_accounts.get(cfg_idx) {
                    log_account_fingerprint(ai, "CLMM.amm_config");
                }
            }
            DexType::PumpFunBondingCurve => {
                let bc_idx = mapping.indices[0] as usize; // bonding_curve
                if let Some(ai) = ctx.remaining_accounts.get(bc_idx) {
                    log_account_fingerprint(ai, "PumpFun.bonding_curve");
                }
            }
            DexType::PumpSwap => {
                let pool_idx = mapping.indices[0] as usize; // pool_state
                if let Some(ai) = ctx.remaining_accounts.get(pool_idx) {
                    log_account_fingerprint(ai, "PumpSwap.pool_state");
                }
            }
        }
        
        // 获取用户的输入输出账户地址
        let user_input_account_key = derived_accounts.get_user_token_account(&step.input_mint)
            .ok_or(ArbitrageError::MissingTokenAccount)?;
        let user_output_account_key = derived_accounts.get_user_token_account(&step.output_mint)
            .ok_or(ArbitrageError::MissingTokenAccount)?;
        
        // 从remaining_accounts中找到对应的AccountInfo
        // 注意：用户的代币账户应该在remaining_accounts的末尾部分
        // 这需要客户端按约定放置：DEX账户在前，用户代币账户在后
        let user_input_account = find_account_info(ctx.remaining_accounts, user_input_account_key)?;
        let user_output_account = find_account_info(ctx.remaining_accounts, user_output_account_key)?;

        // 安全校验：用户 ATA 的 owner/mint/program 是否符合预期
        validate_user_token_account(
            user_input_account,
            &step.input_mint,
            &ctx.accounts.user.key(),
            &program_ids,
        )?;
        validate_user_token_account(
            user_output_account,
            &step.output_mint,
            &ctx.accounts.user.key(),
            &program_ids,
        )?;
        
        // 执行 DEX 交换
        let swap_result = DexRouter::execute_swap(
            step.dex_type,
            dex_accounts,
            &derived_accounts,
            ctx.remaining_accounts,
            &ctx.accounts.user.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            &ctx.accounts.associated_token_program.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            user_input_account,
            user_output_account,
            current_amount,
            step.minimum_amount_out,
        )?;
        
        // 验证输出
        DexRouter::validate_swap_result(&swap_result, step.minimum_amount_out)?;
        
        current_amount = swap_result.amount_out;
        msg!(
            "Step {} completed: amount_out={} -> new_running_amount={}",
            step_index,
            swap_result.amount_out,
            current_amount
        );
    }
    
    // 6. 验证最终利润
    require!(
        current_amount >= params.input_amount.saturating_add(params.min_profit_lamports),
        ArbitrageError::InsufficientProfit
    );
    let profit = current_amount - params.input_amount;
    msg!("Arbitrage completed successfully. Profit: {}", profit);
    
    Ok(())
}

/// Helper function to find AccountInfo by public key in remaining_accounts
fn find_account_info<'info>(
    remaining_accounts: &'info [AccountInfo<'info>], 
    target_key: &Pubkey
) -> Result<&'info AccountInfo<'info>> {
    for account in remaining_accounts {
        if account.key() == *target_key {
            return Ok(account);
        }
    }
    Err(ArbitrageError::AccountNotFound.into())
}

/// 校验用户 SPL Token 账户是否与期望的 mint/owner 对齐，且为受支持的 token program
fn validate_user_token_account<'info>(
    token_ai: &AccountInfo<'info>,
    expected_mint: &Pubkey,
    expected_owner: &Pubkey,
    program_ids: &ProgramIds,
) -> Result<()> {
    // 校验 program（Token 或 Token-2022）
    let is_token = token_ai.owner == &program_ids.token_program;
    let is_token22 = token_ai.owner == &program_ids.token_2022_program;
    require!(is_token || is_token22, ArbitrageError::InvalidAccount);

    // 读取 token account 基础数据（至少 72 字节：mint(32)+owner(32)+amount(8)）
    let len = token_ai.data_len();
    if len < 72 {
        return Err(ArbitrageError::InvalidAccount.into());
    }
    let data = token_ai.try_borrow_data()?;
    let mint_bytes = &data[0..32];
    let owner_bytes = &data[32..64];
    let mint_pk = Pubkey::new_from_array(mint_bytes.try_into().unwrap_or([0u8;32]));
    let owner_pk = Pubkey::new_from_array(owner_bytes.try_into().unwrap_or([0u8;32]));

    require!(mint_pk == *expected_mint, ArbitrageError::InvalidTokenMint);
    require!(owner_pk == *expected_owner, ArbitrageError::InvalidAccount);

    // 额外健壮性检查：mint/owner 不应为默认零地址
    require!(mint_pk != Pubkey::default(), ArbitrageError::InvalidAccount);
    require!(owner_pk != Pubkey::default(), ArbitrageError::InvalidAccount);

    // 观测日志：便于链上对齐问题排查
    msg!(
        "[ATA] program={} len={} mint={} owner={} (expected_mint={} expected_owner={})",
        if is_token { program_ids.token_program } else { program_ids.token_2022_program },
        len,
        mint_pk,
        owner_pk,
        expected_mint,
        expected_owner
    );
    Ok(())
}

// bytes_to_hex/compute_accounts_table_snapshot 已移除

/// 打印账户指纹（长度 + 前8字节十六进制），用于多版本池/配置识别
fn log_account_fingerprint<'info>(ai: &AccountInfo<'info>, label: &str) {
    let len = ai.data_len();
    let head8 = if let Ok(data) = ai.try_borrow_data() {
        let n = core::cmp::min(8, data.len());
        // 简化：直接打印前 8 字节的十进制数组，避免 hex 工具
        let mut s = String::from("[");
        for (i, b) in data[0..n].iter().enumerate() {
            if i > 0 { s.push(','); }
            s.push_str(&b.to_string());
        }
        s.push(']');
        s
    } else {
        String::from("")
    };
    msg!("[FINGERPRINT] {} len={} head8={}", label, len, head8);
}

