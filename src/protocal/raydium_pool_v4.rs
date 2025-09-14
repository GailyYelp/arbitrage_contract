use crate::instructions::types::read_token_amount;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const RAYDIUM_POOL_V4_SWAP_BASE_IN_SELECTOR: &[u8; 1] = &[9];

#[derive(Clone)]
pub struct RaydiumPoolV4Accounts<'info> {
    pub program: &'info AccountInfo<'info>,       // 0. program账户
    pub token_program: &'info AccountInfo<'info>, // 1. tokenProgram
    pub pool_state: &'info AccountInfo<'info>,    // 2. 池地址 (pool_id)
    pub amm_authority_info: &'info AccountInfo<'info>, // 3. amm_authority_info
    pub coin_vault: &'info AccountInfo<'info>,    // 4. token0金库
    pub pc_vault: &'info AccountInfo<'info>,      // 5. token1金库
    pub input_token_account: &'info AccountInfo<'info>, // 6. 输入代币
    pub output_token_account: &'info AccountInfo<'info>, // 7. 输出代币
    pub payer: &'info AccountInfo<'info>,         // 8. payer
}

pub fn raydium_pool_v4_swap<'info>(
    accounts: RaydiumPoolV4Accounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;

    let metas = vec![
        AccountMeta::new_readonly(accounts.token_program.key(), false), // 0: tokenProgram
        AccountMeta::new(accounts.pool_state.key(), false),             // 1: pool_info
        AccountMeta::new_readonly(accounts.amm_authority_info.key(), false), // 2: amm_authority_info
        AccountMeta::new(accounts.pool_state.key(), false), // 3: amm_open_orders_info
        AccountMeta::new(accounts.pool_state.key(), false), // 4: amm_open_orders_info
        AccountMeta::new(accounts.coin_vault.key(), false), // 5: amm_coin_vault_info
        AccountMeta::new(accounts.pc_vault.key(), false),   // 6: amm_pc_vault_info
        AccountMeta::new_readonly(accounts.pool_state.key(), false), // 7: market_program_info
        AccountMeta::new(accounts.pool_state.key(), false), // 8: market_info
        AccountMeta::new(accounts.pool_state.key(), false), // 9: market_bids_info
        AccountMeta::new(accounts.pool_state.key(), false), // 10: market_asks_info
        AccountMeta::new(accounts.pool_state.key(), false), // 11: market_event_queue_info
        AccountMeta::new(accounts.pool_state.key(), false), // 12: market_coin_vault_info
        AccountMeta::new(accounts.pool_state.key(), false), // 13: market_pc_vault_info
        AccountMeta::new_readonly(accounts.pool_state.key(), false), // 14: market_vault_signer
        AccountMeta::new(accounts.input_token_account.key(), false), // 15: user_source_info
        AccountMeta::new(accounts.output_token_account.key(), false), // 16: user_destination_info
        AccountMeta::new_readonly(accounts.payer.key(), true), // 17: user_source_owner
    ];

    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.token_program.clone(),
        accounts.pool_state.clone(),
        accounts.amm_authority_info.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.coin_vault.clone(),
        accounts.pc_vault.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.pool_state.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.payer.clone(),
        // Raydium Pool V4 程序账户
        accounts.program.clone(),
    ];

    // 指令数据 (根据Raydium程序规范构造)
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(RAYDIUM_POOL_V4_SWAP_BASE_IN_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());

    // Instruction
    let program_id = accounts.program.key();
    let ix = Instruction {
        program_id,
        accounts: metas,
        data,
    };

    // Invoke
    invoke(&ix, &account_infos)?;

    // 读取执行后余额并计算真实产出
    let post_out = read_token_amount(accounts.output_token_account)?;
    let amount_out = post_out.saturating_sub(pre_out);
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}
