use crate::errors::ArbitrageError;
use crate::instructions::types::append_remaining_accounts;
use crate::instructions::types::checked_balance_delta;
use crate::instructions::types::read_token_amount;
use crate::instructions::types::SwapResult;
use crate::protocal::pumpfun_amm::{
    fee_bps_to_u64, pumpfun_buy_effective_input_amount, simulate_swap_base_input,
    validate_total_fee_base_point,
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

// PumpSwap 指令选择器 (链上真实的discriminator)
pub const PUMPFUN_AMM_BUY_DISCRIMINATOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const PUMPFUN_AMM_SELL_DISCRIMINATOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
pub const PUMPFUN_SWAP_MIN_ACCOUNTS: usize = 7;

#[derive(Clone)]
pub struct PumpFunSwapAccounts<'info> {
    pub global_account: &'info AccountInfo<'info>, // 1. global
    pub fee_recipient: &'info AccountInfo<'info>,  // 2. fee_recipient
    pub mint: &'info AccountInfo<'info>,           // 3. mint
    pub pool_id: &'info AccountInfo<'info>,        // 4. pool_id
    pub token_vault0: &'info AccountInfo<'info>,   // 5. token_vault0
    pub user_token_account: &'info AccountInfo<'info>, // 6. 用户代币账户
    pub payer: &'info AccountInfo<'info>,          // 7. payer账户
    pub system_program: &'info AccountInfo<'info>, // 8. system program
    pub creator_vault: &'info AccountInfo<'info>,  // 9. creator vault
    pub token_program: &'info AccountInfo<'info>,  // 10. token_program
    pub event_authority: &'info AccountInfo<'info>, // 11. event authority
    pub program: &'info AccountInfo<'info>,        // 12. program
    pub remaining_accounts: Vec<AccountInfo<'info>>, // 13. remaining accounts (globalVolumeAccumulator + userVolumeAccumulator)
}

pub fn pumpfun_swap_swap<'info>(
    accounts: PumpFunSwapAccounts<'info>,
    direction: u8, // 0: buy, 1: sell
    amount_in: u64,
    total_fee_base_point: u16,
) -> Result<SwapResult> {
    let total_fee_base_point = fee_bps_to_u64(total_fee_base_point);
    validate_total_fee_base_point(total_fee_base_point)?;

    let pre_out = if direction == 0 {
        // sell
        accounts.payer.try_lamports()?
    } else {
        // buy
        read_token_amount(accounts.user_token_account)?
    };

    // 若为买入且未传 token_amount，则基于池状态与全局费率模拟可买到的最小 token 数量
    let minimum_amount_out = if direction != 0 {
        simulate_pumpfun_swap_buy_amount_by_input(
            accounts.pool_id,
            amount_in,
            total_fee_base_point,
        )?
    } else {
        0_u64
    };

    let mut metas = vec![
        AccountMeta::new_readonly(accounts.global_account.key(), false),
        AccountMeta::new(accounts.fee_recipient.key(), false),
        AccountMeta::new_readonly(accounts.mint.key(), false),
        AccountMeta::new(accounts.pool_id.key(), false),
        AccountMeta::new(accounts.token_vault0.key(), false),
        AccountMeta::new(accounts.user_token_account.key(), false),
        AccountMeta::new(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new(accounts.creator_vault.key(), false),
        AccountMeta::new_readonly(accounts.event_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
    ];

    let mut account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.global_account.clone(),
        accounts.fee_recipient.clone(),
        accounts.mint.clone(),
        accounts.pool_id.clone(),
        accounts.token_vault0.clone(),
        accounts.user_token_account.clone(),
        accounts.payer.clone(),
        accounts.system_program.clone(),
        accounts.token_program.clone(),
        accounts.creator_vault.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
    ];
    if direction == 0 {
        // sell
        // buy: token_program 在 creator_vault 之前
        // sell: creator_vault 在 token_program 之前
        metas[8] = AccountMeta::new(accounts.creator_vault.key(), false);
        metas[9] = AccountMeta::new_readonly(accounts.token_program.key(), false);
        account_infos[8] = accounts.creator_vault.clone();
        account_infos[9] = accounts.token_program.clone();
    }

    // 动态补充剩余账户
    append_remaining_accounts(&mut metas, &mut account_infos, accounts.remaining_accounts);
    account_infos.push(accounts.program.clone());

    // 构造 data 与账户顺序（严格按 BUY/SELL 对齐）
    let mut data = Vec::with_capacity(8 + 8 + 8);
    // mint --> sol_mint == sell == 0
    // sol_mint --> mint == buy == 1
    if direction == 0 {
        // SELL: data = [SELL, token_amount, min_sol_output] → 使用 amount_in 作为 token_amount，min_out 保持
        data.extend_from_slice(PUMPFUN_AMM_SELL_DISCRIMINATOR);
        data.extend_from_slice(&amount_in.to_le_bytes()); // token_amount
        data.extend_from_slice(&minimum_amount_out.to_le_bytes()); // min_sol_output
    } else {
        // BUY: data = [BUY, token_amount, max_sol_cost] → 使用 min_out 作为 token_amount，上界用 amount_in
        data.extend_from_slice(PUMPFUN_AMM_BUY_DISCRIMINATOR);
        data.extend_from_slice(&minimum_amount_out.to_le_bytes()); // token_amount
        data.extend_from_slice(&amount_in.to_le_bytes()); // max_sol_cost
    };

    // Instruction
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data,
    };

    // Invoke
    invoke(&ix, &account_infos)?;

    // 读取执行后余额并计算真实产出
    let post_out = if direction == 0 {
        // sell
        accounts.payer.try_lamports()?
    } else {
        // buy
        read_token_amount(accounts.user_token_account)?
    };
    let amount_out = checked_balance_delta(pre_out, post_out)?;
    // msg!(
    //     "amount_out: {} pre_out: {} post_out: {}",
    //     amount_out,
    //     pre_out,
    //     post_out
    // );
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

// ================================================================
// 模拟：PumpFun Swap 买入（SOL -> Token），返回可买到的 token_amount
// 说明：
// - 使用 bonding curve：k = vSOL * vTOKEN
// - 费用在输入侧扣除：in' = in - ceil(in*fee_bps/10000) - ceil(in*creator_fee_bps/10000)
// - new_vSOL = vSOL + in'；new_vTOKEN = ceil(k / new_vSOL)
// - token_out = min(vTOKEN - new_vTOKEN, real_token_reserves)
fn simulate_pumpfun_swap_buy_amount_by_input<'info>(
    pool_account: &'info AccountInfo<'info>,
    max_sol_in: u64,
    total_fee_base_point: u64,
) -> Result<u64> {
    if max_sol_in == 0 {
        return Ok(0);
    }

    // 读取池账户数据（含 8 字节 discriminator）
    // 对齐 PumpFunSwapPoolState:
    // virtual_token_reserves[0..8]
    // virtual_sol_reserves  [8..16]
    // real_token_reserves   [16..24]
    // real_sol_reserves     [24..32]
    // token_total_supply    [32..40]
    // complete              [40]
    // creator               [41..73]
    let pool_data = pool_account.try_borrow_data()?;
    // 需要读取到 complete(1B) 与 creator(32B)，creator 起始偏移为 41（相对结构），所以至少 8+73 字节
    require!(pool_data.len() >= 8 + 73, ArbitrageError::InvalidAccount);

    let virtual_token_reserves = read_pool_u64(&pool_data, 8)?;
    let virtual_sol_reserves = read_pool_u64(&pool_data, 16)?;
    let effective_sol_in = pumpfun_buy_effective_input_amount(max_sol_in)?;
    let token_amount_out = simulate_swap_base_input(
        virtual_token_reserves,
        virtual_sol_reserves,
        total_fee_base_point,
        effective_sol_in,
    )?;
    Ok(token_amount_out)
}

fn read_pool_u64(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}
