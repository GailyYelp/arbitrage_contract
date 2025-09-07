use crate::instructions::types::read_token_amount;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

// Raydium CLMM SwapBaseIn 交易指令选择器
pub const RAYDIUM_CLMM_SWAP_V2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];

#[derive(Clone)]
pub struct RaydiumClmmAccounts<'info> {
    pub program: &'info AccountInfo<'info>,    // 0. program账户
    pub payer: &'info AccountInfo<'info>,      // 1. payer账户
    pub amm_config: &'info AccountInfo<'info>, // 2. AMM配置地址
    pub pool_state: &'info AccountInfo<'info>, // 3. 池地址 (pool_id)
    pub input_token_account: &'info AccountInfo<'info>, // 4. 用户输入代币账户
    pub output_token_account: &'info AccountInfo<'info>, // 5. 用户输出代币账户
    pub input_vault: &'info AccountInfo<'info>, // 6. token0金库
    pub output_vault: &'info AccountInfo<'info>, // 7. token1金库
    pub observation_state: &'info AccountInfo<'info>, // 8. observation账户
    pub token_program: &'info AccountInfo<'info>, // 9. 代币program
    pub token_program_2022: &'info AccountInfo<'info>, // 10. 代币program 2022
    pub memo_program: &'info AccountInfo<'info>, // 11. memo program
    pub input_mint: &'info AccountInfo<'info>, // 12. 输入代币mint
    pub output_mint: &'info AccountInfo<'info>, // 13. 输出代币mint
    // 可选扩展：indices 可追加  tick_array_extension_opt(可选) + tick_arrays（若提供则优先使用）
    pub remaining_accounts: Vec<AccountInfo<'info>>,
}

pub fn raydium_clmm_swap<'info>(
    accounts: RaydiumClmmAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_input: bool,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;

    let mut metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.amm_config.key(), false),
        AccountMeta::new(accounts.pool_state.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.input_vault.key(), false),
        AccountMeta::new(accounts.output_vault.key(), false),
        AccountMeta::new(accounts.observation_state.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program_2022.key(), false),
        AccountMeta::new_readonly(accounts.memo_program.key(), false),
        AccountMeta::new_readonly(accounts.input_mint.key(), false),
        AccountMeta::new_readonly(accounts.output_mint.key(), false),
    ];

    // 先构建基础 account_infos
    let mut account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.payer.clone(),
        accounts.amm_config.clone(),
        accounts.pool_state.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.input_vault.clone(),
        accounts.output_vault.clone(),
        accounts.observation_state.clone(),
        accounts.token_program.clone(),
        accounts.token_program_2022.clone(),
        accounts.memo_program.clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
    ];

    // 动态补充：从 remaining_accounts 追加与 CLMM 程序相关且不在基础集中的账户（例如 extension / tick arrays)
    for ai in accounts.remaining_accounts {
        metas.push(AccountMeta::new(ai.key(), false));
        account_infos.push(ai);
    }
    // CLMM 程序账户
    account_infos.push(accounts.program.clone());

    // Build instruction data
    let mut data = Vec::with_capacity(8 + 8 + 16 + 1);
    data.extend_from_slice(RAYDIUM_CLMM_SWAP_V2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&0u128.to_le_bytes()); // sqrt_price_limit
    data.push(if is_base_input { 1 } else { 0 }); // is_base_input

    // Instruction
    let program_id = accounts.program.key();
    let ix = Instruction {
        program_id,
        accounts: metas.clone(),
        data,
    };

    // Invoke
    invoke(&ix, &account_infos)?;

    let post_out = read_token_amount(accounts.output_token_account)?;
    let amount_out = post_out.saturating_sub(pre_out);
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}
