use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::ArbitrageParams;
use crate::account_resolver::AccountResolver;
use crate::account_derivation::{DerivedAccounts, ProgramIds};
use crate::dex_router::{DexRouter, DexAccounts};
use crate::errors::ArbitrageError;

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
    
    // 2. 初始化程序ID配置
    let program_ids = ProgramIds::default(); // TODO: 从参数或常量获取实际程序ID
    
    // 3. 推导所有需要的账户
    let mut derived_accounts = DerivedAccounts::new();
    derived_accounts.derive_for_path(&params.path_steps, &ctx.accounts.user.key(), &program_ids)?;
    
    // 4. 执行实际的套利路径
    let mut current_amount = params.input_amount;
    
    for (step_index, step) in params.path_steps.iter().enumerate() {
        msg!("Executing step {}: {:?} -> {:?} on {:?}", 
             step_index, step.input_mint, step.output_mint, step.dex_type);
        
        // 获取当前步骤的账户映射
        let mapping = &params.account_mappings[step_index];
        
        // 创建临时的账户解析器 
        let account_resolver = AccountResolver::new(ctx.remaining_accounts);
        
        // 验证账户映射
        account_resolver.validate_account_mapping(mapping)?;
        
        // 解析这一步需要的 DEX 账户
        let dex_accounts = match step.dex_type {
            crate::state::DexType::RaydiumCpmm => {
                DexAccounts::RaydiumCpmm(account_resolver.resolve_raydium_cpmm_accounts(mapping)?)
            }
            crate::state::DexType::RaydiumClmm => {
                DexAccounts::RaydiumClmm(account_resolver.resolve_raydium_clmm_accounts(mapping)?)
            }
            crate::state::DexType::PumpFunBondingCurve => {
                DexAccounts::Pumpfun(account_resolver.resolve_pumpfun_accounts(mapping)?)
            }
            crate::state::DexType::PumpSwap => {
                DexAccounts::Pumpswap(account_resolver.resolve_pumpswap_accounts(mapping)?)
            }
        };
        
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
        
        // 执行 DEX 交换
        let swap_result = DexRouter::execute_swap(
            step.dex_type,
            dex_accounts,
            &derived_accounts,
            user_input_account,
            user_output_account,
            current_amount,
            step.minimum_amount_out,
        )?;
        
        // 验证输出
        DexRouter::validate_swap_result(&swap_result, step.minimum_amount_out)?;
        
        current_amount = swap_result.amount_out;
        msg!("Step {} completed, output: {}", step_index, current_amount);
    }
    
    // 6. 验证最终利润
    require!(current_amount > params.input_amount, ArbitrageError::UnprofitableTrade);
    
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

