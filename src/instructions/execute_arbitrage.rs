use anchor_lang::prelude::*;

use crate::errors::ArbitrageError;
use crate::ExecuteArbitrage;
use crate::protocal::{
    pumpfun_amm::pumpfun_amm_swap, pumpfun_amm::PumpFunAmmAccounts,
    pumpfun_swap::pumpfun_swap_swap, pumpfun_swap::PumpFunSwapAccounts,
    raydium_clmm::raydium_clmm_swap, raydium_clmm::RaydiumClmmAccounts,
    raydium_cpmm::raydium_cpmm_swap, raydium_cpmm::RaydiumCpmmAccounts,
    raydium_launchpad::raydium_launchpad_swap,
    raydium_launchpad::RaydiumLaunchpadAccounts, raydium_pool_v4::raydium_pool_v4_swap,
    raydium_pool_v4::RaydiumPoolV4Accounts,
};
use crate::state::Protocol;
use crate::state::SwapArbParams;

// ==============================================================================================
// 合约入口（协议 - 分段账户 + 极简步骤元数据）
// 说明：
// - 仅定义入口签名与固定账户，保持与一致的通用程序账户；
// - 解析与路由将在后续步骤补充，实现与现有 DexRouter 保持一致；
// - 当前占位实现直接返回 Ok(())，不会影响现有逻辑。
// ==============================================================================================

pub fn execute_arbitrage<'info>(
    ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage>,
    params: SwapArbParams,
) -> Result<()> {
    // 1) 基础校验
    require!(params.mints_count > 1, ArbitrageError::InvalidAccountCount);
    require!(
        params.steps.len() as u8 == params.mints_count,
        ArbitrageError::InvalidAccountCount
    );

    // 校验 remaining_accounts 长度是否与 steps_meta 对齐
    let remaining_accounts_len = ctx.remaining_accounts.len();
    let m = params.mints_count as usize;
    require!(
        remaining_accounts_len >= 2 * m,
        ArbitrageError::InvalidAccountCount
    );
    // 校验 step_accounts 总长度是否与 steps_meta 对齐
    let declared_steps_len: usize = params
        .steps
        .iter()
        .fold(0usize, |acc, s| acc.saturating_add(s.accounts_len as usize));
    let actual_steps_len = remaining_accounts_len.saturating_sub(2 * m);
    require!(
        actual_steps_len == declared_steps_len,
        ArbitrageError::InvalidAccountCount
    );

    // 2) 三段切分：mints[M]、user_accounts[M]、step_accounts（剩余）
    let remaining_accounts = ctx.remaining_accounts;
    // payer
    let payer = &remaining_accounts[0];
    // program accounts
    let system_program = &remaining_accounts[1];
    let associated_token_program = &remaining_accounts[2];
    let token_program = &remaining_accounts[3];
    let token_2022_program = &remaining_accounts[4];
    // mints
    let mints = &remaining_accounts[5..m+5];
    // user_accounts
    let user_accounts = &remaining_accounts[m+5..2 * m+5];

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

        let in_mint_program_ai = if in_mint_ai.owner.key() == token_program.key() {
            token_program
        } else {
            token_2022_program
        };
        let out_mint_program_ai = if out_mint_ai.owner.key() == token_program.key() {
            token_program
        } else {
            token_2022_program
        };

        // 切分本步账户组
        let step_len = step.accounts_len as usize;
        let step_slice = &remaining_accounts[cursor..cursor + step_len];
        cursor += step_len;

        // 执行单步 swap
        let result = // 按 dex_type 将 step_slice 映射为 DexAccounts 并路由
        match step.protocol {
            Protocol::RaydiumCPMM => {
                require!(step_slice.len() >= 7, ArbitrageError::InvalidAccountCount);
                let account_infos = RaydiumCpmmAccounts {
                    program: &step_slice[0],
                    payer: payer,
                    authority: &step_slice[1],
                    amm_config: &step_slice[2],
                    pool_state: &step_slice[3],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    input_vault: &step_slice[4],
                    output_vault: &step_slice[5],
                    input_token_program: in_mint_program_ai,
                    output_token_program: out_mint_program_ai,
                    input_mint: in_mint_ai,
                    output_mint: out_mint_ai,
                    observation_state: &step_slice[6],
                };
                raydium_cpmm_swap(account_infos, current_amount, 0)
            }
            Protocol::RaydiumCLMM => {
                require!(step_slice.len() >= 8, ArbitrageError::InvalidAccountCount);
                let account_infos = RaydiumClmmAccounts {
                    program: &step_slice[0],
                    payer: payer,
                    amm_config: &step_slice[1],
                    pool_state: &step_slice[2],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    input_vault: &step_slice[3],
                    output_vault: &step_slice[4],
                    observation_state: &step_slice[5],
                    token_program: token_program,
                    token_program_2022: token_2022_program,
                    memo_program: &step_slice[6],
                    input_mint: &in_mint_ai,
                    output_mint: &out_mint_ai,
                    remaining_accounts: step_slice[7..].to_vec(),
                };
                let is_base_input = step.direction == 0;
                raydium_clmm_swap(account_infos, current_amount, 0, is_base_input)
            }
            Protocol::RaydiumPoolV4 => {
                require!(step_slice.len() >= 5, ArbitrageError::InvalidAccountCount);
                let account_infos = RaydiumPoolV4Accounts {
                    program: &step_slice[0],
                    token_program: token_program,
                    pool_state: &step_slice[1],
                    amm_authority_info: &step_slice[2],
                    coin_vault: &step_slice[3],
                    pc_vault: &step_slice[4],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    payer: payer,
                };
                raydium_pool_v4_swap(account_infos, current_amount, 0)
            }
            Protocol::RaydiumLaunchPad => {
                require!(step_slice.len() >= 8, ArbitrageError::InvalidAccountCount);
                let (
                    base_mint,
                    quote_mint,
                    user_base_token,
                    user_quote_token,
                    base_token_program,
                    quote_token_program,
                ) = if step.direction == 0 {
                    // buy
                    (
                        out_mint_ai,
                        in_mint_ai,
                        user_out_ai,
                        user_in_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                } else {
                    // sell
                    (
                        in_mint_ai,
                        out_mint_ai,
                        user_in_ai,
                        user_out_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                };
                let account_infos = RaydiumLaunchpadAccounts {
                    payer: payer,
                    authority: &step_slice[1],
                    global_config: &step_slice[2],
                    platform_config: &step_slice[3],
                    pool_state: &step_slice[4],
                    user_base_token: user_base_token,
                    user_quote_token: user_quote_token,
                    base_vault: &step_slice[5],
                    quote_vault: &step_slice[6],
                    base_mint: base_mint,
                    quote_mint: quote_mint,
                    base_token_program: base_token_program,
                    quote_token_program: quote_token_program,
                    event_authority: &step_slice[7],
                    program: &step_slice[0],
                    // system_program: &ctx.accounts.system_program.to_account_info(),
                    // observation_state: &step_slice[8],
                    // observation_state2: &step_slice[9],
                };
                raydium_launchpad_swap(account_infos, step.direction, current_amount, 0)
            }
            Protocol::PumpFunSwap => {
                require!(step_slice.len() >= 7, ArbitrageError::InvalidAccountCount);
                let (mint, user_token_account, token_program) = if step.direction == 0 {
                    (out_mint_ai, user_out_ai, out_mint_program_ai) // buy
                } else {
                    (in_mint_ai, user_in_ai, in_mint_program_ai) // sell
                };
    
                let account_infos = PumpFunSwapAccounts {
                    program: &step_slice[0],
                    global_account: &step_slice[1],
                    fee_recipient: &step_slice[2],
                    mint: mint,
                    pool_id: &step_slice[3],
                    token_vault0: &step_slice[4],
                    user_token_account: user_token_account,
                    payer: payer,
                    system_program: system_program,
                    creator_vault: &step_slice[5],
                    token_program: token_program,
                    event_authority: &step_slice[6],
                    remaining_accounts: step_slice[7..].to_vec(),
                };
                pumpfun_swap_swap(account_infos, step.direction, current_amount, 0)
            }
            Protocol::PumpFunAMM => {
                require!(step_slice.len() >= 10, ArbitrageError::InvalidAccountCount);
                // 建议顺序：[0 amm_program,1 global_config,2 pool_state,3 base_mint,4 quote_mint,5 coin_creator,6 event_authority,7 creator_vault_authority,8 creator_vault_ata,9 pool_base_ata,10 pool_quote_ata,(11 fee_recipient?),(12 fee_recipient_ata?)]
                let (
                    base_mint,
                    quote_mint,
                    user_base_token_account,
                    user_quote_token_account,
                    base_token_program,
                    quote_token_program,
                ) = if step.direction == 0 {
                    // buy
                    (
                        out_mint_ai,
                        in_mint_ai,
                        user_out_ai,
                        user_in_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                } else {
                    // sell
                    (
                        in_mint_ai,
                        out_mint_ai,
                        user_in_ai,
                        user_out_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                };
    
                let account_infos = PumpFunAmmAccounts {
                    program: &step_slice[0],
                    pool_state: &step_slice[1],
                    payer: payer,
                    global_config: &step_slice[2],
                    base_mint: base_mint,
                    quote_mint: quote_mint,
                    user_base_token_account: user_base_token_account,
                    user_quote_token_account: user_quote_token_account,
                    pool_base_token_account: &step_slice[3],
                    pool_quote_token_account: &step_slice[4],
                    fee_recipient: &step_slice[5],
                    fee_recipient_ata: &step_slice[6],
                    base_token_program: base_token_program,
                    quote_token_program: quote_token_program,
                    system_program: system_program,
                    associated_token_program: associated_token_program,
                    event_authority: &step_slice[7],
                    coin_creator_vault_ata: &step_slice[8],
                    coin_creator_vault_authority: &step_slice[9],
                    remaining_accounts: step_slice[10..].to_vec(),
                };
                pumpfun_amm_swap(account_infos, step.direction, current_amount, 0)
            }
        }?;

        // 更新运行金额（余额差法结果）
        current_amount = result.amount_out;
    }

    // 4) 终局利润阈值
    require!(
        current_amount
            >= params
                .input_amount
                .saturating_add(params.min_profit_lamports),
        ArbitrageError::InsufficientProfit
    );
    Ok(())
}
