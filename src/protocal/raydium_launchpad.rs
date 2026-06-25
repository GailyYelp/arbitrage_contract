use crate::instructions::types::read_token_amount;
use crate::instructions::types::token_balance_delta;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

// faea0d7bd59c13ec hex -> u8 array
pub const RAYDIUM_LAUNCHPAD_BUY_EXACT_IN_SELECTOR: &[u8; 8] =
    &[250, 234, 13, 189, 213, 46, 253, 81];
// 9527de9bd37c981a hex -> u8 array
pub const RAYDIUM_LAUNCHPAD_SELL_EXACT_IN_SELECTOR: &[u8; 8] =
    &[149, 39, 222, 155, 211, 124, 152, 26];
pub const RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS: usize = 8;

#[derive(Clone)]
pub struct RaydiumLaunchpadAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,            // 1. payer账户
    pub authority: &'info AccountInfo<'info>,        // 2. authority账户
    pub global_config: &'info AccountInfo<'info>,    // 3. global配置地址
    pub platform_config: &'info AccountInfo<'info>,  // 4. platform配置地址
    pub pool_state: &'info AccountInfo<'info>,       // 5. 池地址 (pool_id)
    pub user_base_token: &'info AccountInfo<'info>,  // 6. 用户基础代币账户
    pub user_quote_token: &'info AccountInfo<'info>, // 7. 用户计价代币账户
    pub base_vault: &'info AccountInfo<'info>,       // 8. 基础代币金库
    pub quote_vault: &'info AccountInfo<'info>,      // 9. 计价代币金库
    pub base_mint: &'info AccountInfo<'info>,        // 10. 基础代币mint
    pub quote_mint: &'info AccountInfo<'info>,       // 11. 计价代币mint
    pub base_token_program: &'info AccountInfo<'info>, // 12. 基础代币program
    pub quote_token_program: &'info AccountInfo<'info>, // 13. 计价代币program
    pub event_authority: &'info AccountInfo<'info>,  // 14. event authority账户
    pub program: &'info AccountInfo<'info>,          // 15. program账户
                                                     // pub system_program: &'info AccountInfo<'info>,   // 16. system program
                                                     // pub observation_state: &'info AccountInfo<'info>, // 17. observation账户
                                                     // pub observation_state2: &'info AccountInfo<'info>, // 18. observation账户
}

pub fn raydium_launchpad_swap<'info>(
    accounts: RaydiumLaunchpadAccounts<'info>,
    direction: u8, // 0: buy, 1: sell
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    // 读取执行前余额
    let output_token_account = if direction == 0 {
        accounts.user_base_token
    } else {
        accounts.user_quote_token
    };
    let pre_out = read_token_amount(output_token_account)?;

    // 构造 Raydium Launchpad 交换指令
    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.global_config.key(), false),
        AccountMeta::new_readonly(accounts.platform_config.key(), false),
        AccountMeta::new(accounts.pool_state.key(), false),
        AccountMeta::new(accounts.user_base_token.key(), false),
        AccountMeta::new(accounts.user_quote_token.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(accounts.event_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
        // AccountMeta::new_readonly(accounts.system_program.key(), false),
        // AccountMeta::new(accounts.observation_state.key(), false),
        // AccountMeta::new(accounts.observation_state2.key(), false),
    ];

    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.payer.clone(),
        accounts.authority.clone(),
        accounts.global_config.clone(),
        accounts.platform_config.clone(),
        accounts.pool_state.clone(),
        accounts.user_base_token.clone(),
        accounts.user_quote_token.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
        // accounts.system_program.clone(),
        // accounts.observation_state.clone(),
        // accounts.observation_state2.clone(),
        // Raydium Launchpad 程序账户
        accounts.program.clone(),
    ];

    // 指令数据 (根据Raydium程序规范构造)
    let share_fee_rate: u64 = 0;
    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(if direction == 0 {
        RAYDIUM_LAUNCHPAD_SELL_EXACT_IN_SELECTOR
    } else {
        RAYDIUM_LAUNCHPAD_BUY_EXACT_IN_SELECTOR
    });
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&share_fee_rate.to_le_bytes());

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
    let output_token_account = if direction == 0 {
        accounts.user_base_token
    } else {
        accounts.user_quote_token
    };
    let amount_out = token_balance_delta(output_token_account, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}
