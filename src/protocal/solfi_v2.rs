use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const SOLFI_V2_SWAP_SELECTOR: u8 = 7;
pub const SOLFI_V2_MIN_ACCOUNTS: usize = 11;

pub struct SolfiV2Accounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
    pub global_config: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub base_token_program: &'info AccountInfo<'info>,
    pub quote_token_program: &'info AccountInfo<'info>,
    pub instruction_sysvar: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn solfi_v2_swap<'info>(
    accounts: SolfiV2Accounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = account_metas(&accounts);
    let account_infos = vec![
        accounts.payer.clone(),
        accounts.market.clone(),
        accounts.oracle.clone(),
        accounts.global_config.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.instruction_sysvar.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(18);
    data.push(SOLFI_V2_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.push(direction);
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &account_infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn account_metas(accounts: &SolfiV2Accounts<'_>) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new_readonly(accounts.oracle.key(), false),
        AccountMeta::new_readonly(accounts.global_config.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(accounts.instruction_sysvar.key(), false),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_data_is_canonical_18_byte_layout() {
        let mut data = Vec::new();
        data.push(SOLFI_V2_SWAP_SELECTOR);
        data.extend_from_slice(&2_998_150_u64.to_le_bytes());
        data.extend_from_slice(&0_u64.to_le_bytes());
        data.push(1);
        assert_eq!(data.len(), 18);
        assert_eq!(data[0], 7);
        assert_eq!(data[17], 1);
    }
}
