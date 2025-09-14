use crate::instructions::types::read_token_amount;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

// PumpSwap 指令选择器 (链上真实的discriminator)
pub const PUMPFUN_AMM_BUY_DISCRIMINATOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
pub const PUMPFUN_AMM_SELL_DISCRIMINATOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];

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
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = if direction == 0 {
        // sell
        accounts.payer.lamports()
    } else {
        // buy
        read_token_amount(accounts.user_token_account)?
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
    if direction == 0 {  // sell
        // buy: token_program 在 creator_vault 之前
        // sell: creator_vault 在 token_program 之前
        metas[8] = AccountMeta::new(accounts.creator_vault.key(), false);
        metas[9] = AccountMeta::new(accounts.token_program.key(), false);
        account_infos[8] = accounts.creator_vault.clone();
        account_infos[9] = accounts.token_program.clone();
    }

    // 动态补充剩余账户
    for ai in accounts.remaining_accounts {
        if ai.is_writable {
            metas.push(AccountMeta::new(ai.key(), false));
        } else {
            metas.push(AccountMeta::new_readonly(ai.key(), false));
        }
        account_infos.push(ai);
    }
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
        accounts.payer.lamports()
    } else {
        // buy
        read_token_amount(accounts.user_token_account)?
    };
    let amount_out = post_out.saturating_sub(pre_out);
    // TODO
    msg!("amount_out: {}", amount_out);
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}
