use crate::instructions::types::read_token_amount;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

// buy discriminator
pub const PUMPFUN_AMM_BUY_DISCRIMINATOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
// sell discriminator
pub const PUMPFUN_AMM_SELL_DISCRIMINATOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];

#[derive(Clone)]
pub struct PumpFunAmmAccounts<'info> {
    pub pool_state: &'info AccountInfo<'info>, // 1. 池地址 (pool_id)
    pub payer: &'info AccountInfo<'info>,      // 2. payer账户
    pub global_config: &'info AccountInfo<'info>, // 3. global配置地址
    pub base_mint: &'info AccountInfo<'info>,  // 4. 基础代币mint
    pub quote_mint: &'info AccountInfo<'info>, // 5. 计价代币mint
    pub user_base_token_account: &'info AccountInfo<'info>, // 6. 用户基础代币账户
    pub user_quote_token_account: &'info AccountInfo<'info>, // 7. 用户计价代币账户
    pub pool_base_token_account: &'info AccountInfo<'info>, // 8. 池基础代币账户
    pub pool_quote_token_account: &'info AccountInfo<'info>, // 9. 池计价代币账户
    pub fee_recipient: &'info AccountInfo<'info>, // 10. fee_recipient
    pub fee_recipient_ata: &'info AccountInfo<'info>, // 11. fee_recipient_ata
    pub base_token_program: &'info AccountInfo<'info>, // 12. 基础代币program
    pub quote_token_program: &'info AccountInfo<'info>, // 13. 计价代币program
    pub system_program: &'info AccountInfo<'info>, // 14. system_program
    pub associated_token_program: &'info AccountInfo<'info>, // 15. associated_token_program
    pub event_authority: &'info AccountInfo<'info>, // 16. event_authority
    pub program: &'info AccountInfo<'info>,    // 17. program
    pub coin_creator_vault_ata: &'info AccountInfo<'info>, // 18. coin_creator_vault_ata
    pub coin_creator_vault_authority: &'info AccountInfo<'info>, // 19. coin_creator_vault_authority
    pub remaining_accounts: Vec<AccountInfo<'info>>, // 20. remaining accounts (globalVolumeAccumulator + userVolumeAccumulator)
}

pub fn pumpfun_amm_swap<'info>(
    accounts: PumpFunAmmAccounts<'info>,
    direction: u8, // 0: sell, 1: buy
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let  output_token_account = if direction == 0 {
        accounts.user_quote_token_account // sell
    } else {
        accounts.user_base_token_account // buy
    };
    let pre_out = read_token_amount(output_token_account)?;

    // 账户 metas（参照引擎构造顺序）
    let mut metas = vec![
        AccountMeta::new_readonly(accounts.pool_state.key(), false), // pool
        AccountMeta::new(accounts.payer.key(), true),                // user
        AccountMeta::new_readonly(accounts.global_config.key(), false), // global
        AccountMeta::new_readonly(accounts.base_mint.key(), false),  // base_mint
        AccountMeta::new_readonly(accounts.quote_mint.key(), false), // quote_mint
        AccountMeta::new(accounts.user_base_token_account.key(), false), // user_base_ata
        AccountMeta::new(accounts.user_quote_token_account.key(), false), // user_quote_ata
        AccountMeta::new(accounts.pool_base_token_account.key(), false), // pool_base_ata
        AccountMeta::new(accounts.pool_quote_token_account.key(), false), // pool_quote_ata
        AccountMeta::new_readonly(accounts.fee_recipient.key(), false), // fee_recipient
        AccountMeta::new(accounts.fee_recipient_ata.key(), false),   // fee_recipient_ata
        AccountMeta::new_readonly(accounts.base_token_program.key(), false), // base_token_program
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false), // quote_token_program
        AccountMeta::new_readonly(accounts.system_program.key(), false), // system_program
        AccountMeta::new_readonly(accounts.associated_token_program.key(), false), // associated_token_program
        AccountMeta::new_readonly(accounts.event_authority.key(), false), // event_authority
        AccountMeta::new_readonly(accounts.program.key(), false),         // amm_program
        AccountMeta::new(accounts.coin_creator_vault_ata.key(), false),   // creator_vault_ata
        AccountMeta::new_readonly(accounts.coin_creator_vault_authority.key(), false), // creator_vault_authority
    ];

    let mut account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.pool_state.clone(),
        accounts.payer.clone(),
        accounts.global_config.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.user_base_token_account.clone(),
        accounts.user_quote_token_account.clone(),
        accounts.pool_base_token_account.clone(),
        accounts.pool_quote_token_account.clone(),
        accounts.fee_recipient.clone(),
        accounts.fee_recipient_ata.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.system_program.clone(),
        accounts.associated_token_program.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
        accounts.coin_creator_vault_ata.clone(),
        accounts.coin_creator_vault_authority.clone(),
    ];

    // 动态补充：从 remaining_accounts 追加与 PumpFunAMM 程序相关且不在基础集中的账户（例如 global_volume_accumulator + user_volume_accumulator)
    for ai in accounts.remaining_accounts {
        if ai.is_writable {
            metas.push(AccountMeta::new(ai.key(), false));
        } else {
            metas.push(AccountMeta::new_readonly(ai.key(), false));
        }
        account_infos.push(ai);
    }
    account_infos.push(accounts.program.clone());

    // 构造 data
    let mut data = Vec::with_capacity(8 + 8 + 8);
    if direction == 0 { // sell
        data.extend_from_slice(PUMPFUN_AMM_SELL_DISCRIMINATOR);
        data.extend_from_slice(&amount_in.to_le_bytes()); // amount_in
        data.extend_from_slice(&minimum_amount_out.to_le_bytes()); // min_sol_output
    } else { // buy
        let minimum_amount_out2 = 45000000000_u64;
        data.extend_from_slice(PUMPFUN_AMM_BUY_DISCRIMINATOR);
        data.extend_from_slice(&minimum_amount_out2.to_le_bytes()); // amount_out
        // data.extend_from_slice(&minimum_amount_out.to_le_bytes()); // amount_out
        data.extend_from_slice(&amount_in.to_le_bytes()); // max_sol_cost
    }

    let program_id = accounts.program.key();
    let ix = Instruction {
        program_id,
        accounts: metas,
        data,
    };

    // Invoke
    invoke(&ix, &account_infos)?;

    let post_out = read_token_amount(output_token_account)?;
    let amount_out = post_out.saturating_sub(pre_out);
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

// pub fn simulate_pumpfun_amm_buy<'info>(
//     accounts: PumpFunSwapAccounts<'info>,
//     amount_in: u64,
//     minimum_amount_out: u64,
// ) -> Result<SwapResult> {
//     let pre_out = read_token_amount(accounts.user_token_account)?;
//     Ok(SwapResult {
//         amount_out: pre_out,
//         fee_amount: 0,
//     })
// }