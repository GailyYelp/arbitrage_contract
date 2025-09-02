use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token_2022::Token2022;

use crate::account_derivation::{DerivedAccounts, ProgramIds};
use crate::account_resolver::{
    PumpfunAccounts, PumpswapAccounts, RaydiumClmmAccounts, RaydiumCpmmAccounts,
};
use crate::dex_router::{DexAccounts, DexRouter};
use crate::errors::ArbitrageError;
use crate::state::{DexType, V4ArbParams};

// ==============================================================================================
// 合约入口（V4 协议 - 分段账户 + 极简步骤元数据）
// 说明：
// - 仅定义入口签名与固定账户，保持与 V2 一致的通用程序账户；
// - 解析与路由将在后续步骤补充，实现与现有 DexRouter 保持一致；
// - 当前占位实现直接返回 Ok(())，不会影响现有逻辑。
// ==============================================================================================

#[derive(Accounts)]
pub struct ExecuteArbitrageV4<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    // 顺序按规范：system -> associated_token -> token -> token_2022
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
}

pub fn execute_arbitrage_v4<'info>(
    ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrageV4<'info>>,
    params: V4ArbParams,
) -> Result<()> {
    // 1) 基础校验与程序ID初始化（与 V2 一致）
    require!(params.mints_count > 1, ArbitrageError::InvalidAccountCount);
    require!(params.steps.len() as u8 == params.mints_count, ArbitrageError::InvalidAccountCount);

    let program_ids = ProgramIds::default();
    require!(ctx.accounts.system_program.key() == program_ids.system_program, ArbitrageError::InvalidAccount);
    require!(ctx.accounts.associated_token_program.key() == program_ids.associated_token_program, ArbitrageError::InvalidAccount);
    require!(ctx.accounts.token_program.key() == program_ids.token_program, ArbitrageError::InvalidAccount);
    require!(ctx.accounts.token_2022_program.key() == program_ids.token_2022_program, ArbitrageError::InvalidAccount);

    let mut derived_accounts = DerivedAccounts::new();
    derived_accounts.initialize(&program_ids)?;

    // 2) 三段切分：mints[M]、user_accounts[M]、step_accounts（剩余）
    let rai = ctx.remaining_accounts;
    let m = params.mints_count as usize;
    require!(rai.len() >= 2 * m, ArbitrageError::InvalidAccountCount);
    // 校验 step_accounts 总长度是否与 steps_meta 对齐
    let declared_steps_len: usize = params
        .steps
        .iter()
        .fold(0usize, |acc, s| acc.saturating_add(s.accounts_len as usize));
    let actual_steps_len = rai.len().saturating_sub(2 * m);
    require!(
        actual_steps_len == declared_steps_len,
        ArbitrageError::InvalidAccountCount
    );
    let mints = &rai[0..m];
    let user_accounts = &rai[m..2 * m];

    // 3) 逐步推导闭环的输入/输出 mint 与用户账户，并切分每步账户组，随后路由执行
    let mut current_amount = params.input_amount;
    let mut cursor = 2 * m;
    for (i, step) in params.steps.iter().enumerate() {
        // 闭环：第 i 步 (mints[i], mints[(i+1)%M])
        let in_idx = i;
        let out_idx = (i + 1) % m;
        let user_in_ai = user_accounts
            .get(in_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;
        let user_out_ai = user_accounts
            .get(out_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;
        let in_mint_ai = mints
            .get(in_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;
        let out_mint_ai = mints
            .get(out_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;

        // 用户 SPL 账户快速校验（mint/owner/program）
        validate_user_token_account(
            user_in_ai,
            &in_mint_ai.key(),
            &ctx.accounts.user.key(),
            &program_ids,
        )?;
        validate_user_token_account(
            user_out_ai,
            &out_mint_ai.key(),
            &ctx.accounts.user.key(),
            &program_ids,
        )?;

        // 切分本步账户组
        let step_len = step.accounts_len as usize;
        require!(cursor + step_len <= rai.len(), ArbitrageError::InvalidAccountCount);
        let step_slice = &rai[cursor..cursor + step_len];
        cursor += step_len;
        // 按 dex_type 将 step_slice 映射为 DexAccounts 并路由
        let result = match step.dex_type {
            DexType::RaydiumCpmm => {
                // 期望顺序：[0 program_id, 1 amm_config, 2 pool_state, 3 token0_vault, 4 token1_vault, 5 observation, 6 authority]
                // 精简且稳妥：要求显式携带 authority 并固定在 index=6
                require!(step_len >= 7, ArbitrageError::InvalidAccountCount);
                // 程序账户可执行校验
                require!(step_slice[0].executable, ArbitrageError::InvalidAccount);
                // authority 必须为 PDA（非可执行且无数据）
                require!(!step_slice[6].executable, ArbitrageError::InvalidAccount);
                require!(step_slice[6].data_len() == 0, ArbitrageError::InvalidAccount);
                let acc = RaydiumCpmmAccounts {
                    amm_config: &step_slice[1],
                    pool_state: &step_slice[2],
                    token0_vault: &step_slice[3],
                    token1_vault: &step_slice[4],
                    input_mint: in_mint_ai,
                    output_mint: out_mint_ai,
                    observation_state: &step_slice[5],
                };
                let res = DexRouter::execute_swap(
                    DexType::RaydiumCpmm,
                    DexAccounts::RaydiumCpmm(acc),
                    &derived_accounts,
                    step_slice,
                    &ctx.accounts.user.to_account_info(),
                    &ctx.accounts.token_program.to_account_info(),
                    &ctx.accounts.associated_token_program.to_account_info(),
                    &ctx.accounts.system_program.to_account_info(),
                    user_in_ai,
                    user_out_ai,
                    current_amount,
                    0, // 本版不使用每步 min_out
                );
                res
            }
            DexType::RaydiumClmm => {
                // 期望顺序：[0 clmm_program,1 amm_config,2 pool_state,3 input_vault,4 output_vault,5 observation,6 token_program,7 token_2022,8 memo,9 input_vault_mint,10 output_vault_mint, (11 tick_ext?), (后续 tick_arrays...)]
                require!(step_len >= 11, ArbitrageError::InvalidAccountCount);
                // 程序账户可执行校验
                require!(step_slice[0].executable, ArbitrageError::InvalidAccount);
                let acc = RaydiumClmmAccounts {
                    clmm_program: &step_slice[0],
                    amm_config: &step_slice[1],
                    pool_state: &step_slice[2],
                    input_vault: &step_slice[3],
                    output_vault: &step_slice[4],
                    observation_state: &step_slice[5],
                    token_program: &step_slice[6],
                    token_program_2022: &step_slice[7],
                    memo_program: &step_slice[8],
                    input_vault_mint: &step_slice[9],
                    output_vault_mint: &step_slice[10],
                };
                let res = DexRouter::execute_swap(
                    DexType::RaydiumClmm,
                    DexAccounts::RaydiumClmm(acc),
                    &derived_accounts,
                    step_slice,
                    &ctx.accounts.user.to_account_info(),
                    &ctx.accounts.token_program.to_account_info(),
                    &ctx.accounts.associated_token_program.to_account_info(),
                    &ctx.accounts.system_program.to_account_info(),
                    user_in_ai,
                    user_out_ai,
                    current_amount,
                    0,
                );
                res
            }
            DexType::PumpFunBondingCurve => {
                // 建议顺序：[0 program,1 global,2 bonding_curve,3 mint,4 creator,5 event_authority,6 associated_bonding_curve, (7 fee_recipient?),(其他可选...)]
                require!(step_len >= 5, ArbitrageError::InvalidAccountCount);
                // 程序账户可执行校验
                require!(step_slice[0].executable, ArbitrageError::InvalidAccount);
                let fee_opt = if step_len >= 8 { Some(&step_slice[7]) } else { None };
                let acc = PumpfunAccounts {
                    bonding_curve: &step_slice[2],
                    mint: &step_slice[3],
                    creator: &step_slice[4],
                    fee_recipient_opt: fee_opt,
                };
                let res = DexRouter::execute_swap(
                    DexType::PumpFunBondingCurve,
                    DexAccounts::Pumpfun(acc),
                    &derived_accounts,
                    step_slice,
                    &ctx.accounts.user.to_account_info(),
                    &ctx.accounts.token_program.to_account_info(),
                    &ctx.accounts.associated_token_program.to_account_info(),
                    &ctx.accounts.system_program.to_account_info(),
                    user_in_ai,
                    user_out_ai,
                    current_amount,
                    0,
                );
                res
            }
            DexType::PumpSwap => {
                // 建议顺序：[0 amm_program,1 global_config,2 pool_state,3 base_mint,4 quote_mint,5 coin_creator,6 event_authority,7 creator_vault_authority,8 creator_vault_ata,9 pool_base_ata,10 pool_quote_ata,(11 fee_recipient?),(12 fee_recipient_ata?)]
                require!(step_len >= 6, ArbitrageError::InvalidAccountCount);
                // 程序账户可执行校验
                require!(step_slice[0].executable, ArbitrageError::InvalidAccount);
                let fee_opt = if step_len >= 12 { Some(&step_slice[11]) } else { None };
                let fee_ata_opt = if step_len >= 13 { Some(&step_slice[12]) } else { None };
                let acc = PumpswapAccounts {
                    pool_state: &step_slice[2],
                    base_mint: &step_slice[3],
                    quote_mint: &step_slice[4],
                    coin_creator: &step_slice[5],
                    fee_recipient_opt: fee_opt,
                    fee_recipient_ata_opt: fee_ata_opt,
                };
                let res = DexRouter::execute_swap(
                    DexType::PumpSwap,
                    DexAccounts::Pumpswap(acc),
                    &derived_accounts,
                    step_slice,
                    &ctx.accounts.user.to_account_info(),
                    &ctx.accounts.token_program.to_account_info(),
                    &ctx.accounts.associated_token_program.to_account_info(),
                    &ctx.accounts.system_program.to_account_info(),
                    user_in_ai,
                    user_out_ai,
                    current_amount,
                    0,
                );
                res
            }
        }?;

        // 更新运行金额（余额差法结果）
        current_amount = result.amount_out;
    }

    // 4) 终局利润阈值
    require!(
        current_amount >= params
            .input_amount
            .saturating_add(params.min_profit_lamports),
        ArbitrageError::InsufficientProfit
    );
    Ok(())
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
    let mint_pk = Pubkey::new_from_array(mint_bytes.try_into().unwrap_or([0u8; 32]));
    let owner_pk = Pubkey::new_from_array(owner_bytes.try_into().unwrap_or([0u8; 32]));

    require!(mint_pk == *expected_mint, ArbitrageError::InvalidTokenMint);
    require!(owner_pk == *expected_owner, ArbitrageError::InvalidAccount);

    // 额外健壮性检查：mint/owner 不应为默认零地址
    require!(mint_pk != Pubkey::default(), ArbitrageError::InvalidAccount);
    require!(owner_pk != Pubkey::default(), ArbitrageError::InvalidAccount);

    Ok(())
}


