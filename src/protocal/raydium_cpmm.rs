use crate::instructions::types::read_token_amount;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;


// Raydium CPMM SwapBaseIn 交易指令选择器
pub const RAYDIUM_CPMM_SWAP_BASE_IN_SELECTOR: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];

#[derive(Clone)]
pub struct RaydiumCpmmAccounts<'info> {
    pub program: &'info AccountInfo<'info>,    // 0. program账户
    pub payer: &'info AccountInfo<'info>,      // 1. payer账户
    pub authority: &'info AccountInfo<'info>,  // 2. authority账户
    pub amm_config: &'info AccountInfo<'info>, // 3. AMM配置地址
    pub pool_state: &'info AccountInfo<'info>, // 4. 池地址 (pool_id)
    pub input_token_account: &'info AccountInfo<'info>, // 5. 用户输入代币账户
    pub output_token_account: &'info AccountInfo<'info>, // 6. 用户输出代币账户
    pub input_vault: &'info AccountInfo<'info>, // 7. token0金库
    pub output_vault: &'info AccountInfo<'info>, // 8. token1金库
    pub input_token_program: &'info AccountInfo<'info>, // 9. 输入代币program
    pub output_token_program: &'info AccountInfo<'info>, // 10. 输出代币program
    pub input_mint: &'info AccountInfo<'info>, // 11. 输入代币mint
    pub output_mint: &'info AccountInfo<'info>, // 12. 输出代币mint
    pub observation_state: &'info AccountInfo<'info>, // 13. observation账户
}

pub fn raydium_cpmm_swap<'info>(
    accounts: RaydiumCpmmAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;

    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.amm_config.key(), false),
        AccountMeta::new(accounts.pool_state.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.input_vault.key(), false),
        AccountMeta::new(accounts.output_vault.key(), false),
        AccountMeta::new_readonly(accounts.input_token_program.key(), false),
        AccountMeta::new_readonly(accounts.output_token_program.key(), false),
        AccountMeta::new_readonly(accounts.input_mint.key(), false),
        AccountMeta::new_readonly(accounts.output_mint.key(), false),
        AccountMeta::new(accounts.observation_state.key(), false),
    ];

    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.payer.clone(),
        accounts.authority.clone(),
        accounts.amm_config.clone(),
        accounts.pool_state.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.input_vault.clone(),
        accounts.output_vault.clone(),
        accounts.input_token_program.clone(),
        accounts.output_token_program.clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.observation_state.clone(),
        // Raydium CPMM 程序账户（从 remaining_accounts 查找）
        accounts.program.clone(),
    ];

    // 指令数据 (根据Raydium程序规范构造)
    let mut data = Vec::with_capacity(8 + 8 + 8);
    data.extend_from_slice(RAYDIUM_CPMM_SWAP_BASE_IN_SELECTOR);
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
