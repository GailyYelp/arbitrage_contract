use anchor_lang::prelude::*;

use crate::errors::ArbitrageError;
use crate::instructions::accounts::{parse_route_accounts, validate_step_account_flags};
use crate::instructions::program_ids::{
    validate_step_fixed_accounts, validate_step_program_account,
};
use crate::instructions::types::{
    token_program_for_mint, validate_pumpfun_amm_semantic_accounts,
    validate_pumpfun_swap_semantic_accounts, validate_raydium_clmm_semantic_accounts,
    validate_raydium_cpmm_semantic_accounts, validate_raydium_launchpad_semantic_accounts,
    validate_raydium_pool_v4_authority, validate_raydium_pool_v4_semantic_accounts,
    validate_token_account_for_mint, validate_token_account_for_mint_and_authority,
};
use crate::protocal::{
    pumpfun_amm::{pumpfun_amm_swap, PumpFunAmmAccounts, PUMPFUN_AMM_MIN_ACCOUNTS},
    pumpfun_swap::{pumpfun_swap_swap, PumpFunSwapAccounts, PUMPFUN_SWAP_MIN_ACCOUNTS},
    raydium_clmm::{raydium_clmm_swap, RaydiumClmmAccounts, RAYDIUM_CLMM_MIN_ACCOUNTS},
    raydium_cpmm::{raydium_cpmm_swap, RaydiumCpmmAccounts, RAYDIUM_CPMM_MIN_ACCOUNTS},
    raydium_launchpad::{
        raydium_launchpad_swap, RaydiumLaunchpadAccounts, RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
    },
    raydium_pool_v4::{raydium_pool_v4_swap, RaydiumPoolV4Accounts, RAYDIUM_POOL_V4_MIN_ACCOUNTS},
};
use crate::state::Protocol;
use crate::state::SwapArbParams;
use crate::ExecuteArbitrage;

pub fn execute_arbitrage<'info>(
    ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage>,
    params: SwapArbParams,
) -> Result<()> {
    let route_accounts = parse_route_accounts(ctx.remaining_accounts, &params)?;
    let m = params.mints_count as usize;
    let remaining_accounts = ctx.remaining_accounts;
    let payer = route_accounts.payer;
    let system_program = route_accounts.system_program;
    let associated_token_program = route_accounts.associated_token_program;
    let token_program = route_accounts.token_program;
    let token_2022_program = route_accounts.token_2022_program;
    let mints = route_accounts.mints;
    let user_accounts = route_accounts.user_accounts;

    let mut current_amount = params.input_amount;
    for (i, step) in params.steps.iter().enumerate() {
        let direction = step.direction_value()?.as_u8();
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

        let in_mint_program_ai =
            token_program_for_mint(in_mint_ai, token_program, token_2022_program)?;
        let out_mint_program_ai =
            token_program_for_mint(out_mint_ai, token_program, token_2022_program)?;
        validate_token_account_for_mint_and_authority(
            user_in_ai,
            in_mint_ai,
            in_mint_program_ai,
            payer,
        )?;
        validate_token_account_for_mint_and_authority(
            user_out_ai,
            out_mint_ai,
            out_mint_program_ai,
            payer,
        )?;

        // 切分本步账户组
        let step_slice = &remaining_accounts[route_accounts.step_ranges[i].clone()];
        let program_account = step_slice
            .first()
            .ok_or(ArbitrageError::InvalidAccountCount)?;
        validate_step_account_flags(step.protocol, step_slice)?;
        validate_step_program_account(step.protocol, program_account)?;
        validate_step_fixed_accounts(step.protocol, step_slice)?;

        let result = match step.protocol {
            Protocol::RaydiumCPMM => {
                require!(
                    step_slice.len() >= RAYDIUM_CPMM_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    in_mint_ai,
                    in_mint_program_ai,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    out_mint_ai,
                    out_mint_program_ai,
                    &step_slice[1],
                )?;
                validate_raydium_cpmm_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;
                let account_infos = RaydiumCpmmAccounts {
                    program: &step_slice[0],
                    payer,
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
                raydium_cpmm_swap(account_infos, current_amount, step.min_output_amount)
            }
            Protocol::RaydiumCLMM => {
                require!(
                    step_slice.len() >= RAYDIUM_CLMM_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_token_account_for_mint(&step_slice[3], in_mint_ai, in_mint_program_ai)?;
                validate_token_account_for_mint(&step_slice[4], out_mint_ai, out_mint_program_ai)?;
                validate_raydium_clmm_semantic_accounts(step_slice, in_mint_ai, out_mint_ai)?;
                let account_infos = RaydiumClmmAccounts {
                    program: &step_slice[0],
                    payer,
                    amm_config: &step_slice[1],
                    pool_state: &step_slice[2],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    input_vault: &step_slice[3],
                    output_vault: &step_slice[4],
                    observation_state: &step_slice[5],
                    token_program,
                    token_program_2022: token_2022_program,
                    memo_program: &step_slice[6],
                    input_mint: in_mint_ai,
                    output_mint: out_mint_ai,
                    remaining_accounts: step_slice[7..].to_vec(),
                };
                raydium_clmm_swap(account_infos, current_amount, step.min_output_amount)
            }
            Protocol::RaydiumPoolV4 => {
                require!(
                    step_slice.len() >= RAYDIUM_POOL_V4_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                let (
                    coin_vault_mint,
                    pc_vault_mint,
                    coin_vault_token_program,
                    pc_vault_token_program,
                ) = if direction == 0 {
                    (
                        in_mint_ai,
                        out_mint_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                } else {
                    (
                        out_mint_ai,
                        in_mint_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    coin_vault_mint,
                    coin_vault_token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    pc_vault_mint,
                    pc_vault_token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[12],
                    coin_vault_mint,
                    coin_vault_token_program,
                    &step_slice[14],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[13],
                    pc_vault_mint,
                    pc_vault_token_program,
                    &step_slice[14],
                )?;
                validate_raydium_pool_v4_authority(&step_slice[0], &step_slice[1], &step_slice[2])?;
                validate_raydium_pool_v4_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let account_infos = RaydiumPoolV4Accounts {
                    program: &step_slice[0],
                    token_program,
                    pool_state: &step_slice[1],
                    amm_authority_info: &step_slice[2],
                    amm_open_orders: &step_slice[3],
                    amm_target_orders: &step_slice[4],
                    coin_vault: &step_slice[5],
                    pc_vault: &step_slice[6],
                    market_program: &step_slice[7],
                    market: &step_slice[8],
                    market_bids: &step_slice[9],
                    market_asks: &step_slice[10],
                    market_event_queue: &step_slice[11],
                    market_coin_vault: &step_slice[12],
                    market_pc_vault: &step_slice[13],
                    market_vault_signer: &step_slice[14],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    payer,
                };
                raydium_pool_v4_swap(account_infos, current_amount, step.min_output_amount)
            }
            Protocol::RaydiumLaunchPad => {
                require!(
                    step_slice.len() >= RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                let (
                    base_mint,
                    quote_mint,
                    user_base_token,
                    user_quote_token,
                    base_token_program,
                    quote_token_program,
                ) = if direction == 0 {
                    // sell/base -> quote
                    (
                        in_mint_ai,
                        out_mint_ai,
                        user_in_ai,
                        user_out_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                } else {
                    // buy/quote -> base
                    (
                        out_mint_ai,
                        in_mint_ai,
                        user_out_ai,
                        user_in_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    base_mint,
                    base_token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    quote_mint,
                    quote_token_program,
                    &step_slice[1],
                )?;
                validate_raydium_launchpad_semantic_accounts(step_slice, base_mint, quote_mint)?;
                let account_infos = RaydiumLaunchpadAccounts {
                    payer,
                    authority: &step_slice[1],
                    global_config: &step_slice[2],
                    platform_config: &step_slice[3],
                    pool_state: &step_slice[4],
                    user_base_token,
                    user_quote_token,
                    base_vault: &step_slice[5],
                    quote_vault: &step_slice[6],
                    base_mint,
                    quote_mint,
                    base_token_program,
                    quote_token_program,
                    event_authority: &step_slice[7],
                    program: &step_slice[0],
                    // system_program: &ctx.accounts.system_program.to_account_info(),
                    // observation_state: &step_slice[8],
                    // observation_state2: &step_slice[9],
                };
                raydium_launchpad_swap(
                    account_infos,
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }
            Protocol::PumpFunSwap => {
                require!(
                    step_slice.len() >= PUMPFUN_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                let (mint, user_token_account, token_program) = if direction == 1 {
                    (out_mint_ai, user_out_ai, out_mint_program_ai) // buy
                } else {
                    (in_mint_ai, user_in_ai, in_mint_program_ai) // sell
                };
                validate_token_account_for_mint(&step_slice[4], mint, token_program)?;
                validate_pumpfun_swap_semantic_accounts(
                    &step_slice[0],
                    payer,
                    step_slice,
                    direction,
                )?;

                let account_infos = PumpFunSwapAccounts {
                    program: &step_slice[0],
                    global_account: &step_slice[1],
                    fee_recipient: &step_slice[2],
                    mint,
                    pool_id: &step_slice[3],
                    token_vault0: &step_slice[4],
                    user_token_account,
                    payer,
                    system_program,
                    creator_vault: &step_slice[5],
                    token_program,
                    event_authority: &step_slice[6],
                    remaining_accounts: step_slice[7..].to_vec(),
                };
                pumpfun_swap_swap(account_infos, direction, current_amount, step.fee_rate)
            }
            Protocol::PumpFunAMM => {
                require!(
                    step_slice.len() >= PUMPFUN_AMM_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                // 建议顺序：[0 amm_program,1 global_config,2 pool_state,3 base_mint,4 quote_mint,5 coin_creator,6 event_authority,7 creator_vault_authority,8 creator_vault_ata,9 pool_base_ata,10 pool_quote_ata,(11 fee_recipient?),(12 fee_recipient_ata?)]
                let (
                    base_mint,
                    quote_mint,
                    user_base_token_account,
                    user_quote_token_account,
                    base_token_program,
                    quote_token_program,
                ) = if direction == 0 {
                    // sell
                    (
                        in_mint_ai,
                        out_mint_ai,
                        user_in_ai,
                        user_out_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                } else {
                    // buy
                    (
                        out_mint_ai,
                        in_mint_ai,
                        user_out_ai,
                        user_in_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                };

                validate_token_account_for_mint(&step_slice[3], base_mint, base_token_program)?;
                validate_token_account_for_mint(&step_slice[4], quote_mint, quote_token_program)?;
                validate_pumpfun_amm_semantic_accounts(
                    &step_slice[0],
                    payer,
                    associated_token_program,
                    quote_token_program,
                    base_mint,
                    quote_mint,
                    step_slice,
                )?;
                let account_infos = PumpFunAmmAccounts {
                    program: &step_slice[0],
                    pool_state: &step_slice[1],
                    payer,
                    global_config: &step_slice[2],
                    base_mint,
                    quote_mint,
                    user_base_token_account,
                    user_quote_token_account,
                    pool_base_token_account: &step_slice[3],
                    pool_quote_token_account: &step_slice[4],
                    fee_recipient: &step_slice[5],
                    fee_recipient_ata: &step_slice[6],
                    base_token_program,
                    quote_token_program,
                    system_program,
                    associated_token_program,
                    event_authority: &step_slice[7],
                    coin_creator_vault_ata: &step_slice[8],
                    coin_creator_vault_authority: &step_slice[9],
                    remaining_accounts: step_slice[10..].to_vec(),
                };
                pumpfun_amm_swap(account_infos, direction, current_amount, step.fee_rate)
            }
        }?;

        validate_step_min_output(result.amount_out, step.min_output_amount)?;

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

fn validate_step_min_output(amount_out: u64, min_output_amount: u64) -> Result<()> {
    require!(
        amount_out >= min_output_amount,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_min_output_accepts_equal_or_above_threshold() {
        assert!(validate_step_min_output(100, 100).is_ok());
        assert!(validate_step_min_output(101, 100).is_ok());
    }

    #[test]
    fn step_min_output_rejects_below_threshold() {
        let err = validate_step_min_output(99, 100).unwrap_err();
        assert_eq!(err, ArbitrageError::InsufficientOutputAmount.into());
    }
}
