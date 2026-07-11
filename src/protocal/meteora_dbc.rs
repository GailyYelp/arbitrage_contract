use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const METEORA_DBC_SWAP2_SELECTOR: &[u8; 8] = &[65, 75, 63, 76, 235, 91, 91, 136];
pub const METEORA_DBC_MIN_ACCOUNTS: usize = 7;
pub const METEORA_DBC_EXACT_IN_MODE: u8 = 0;

#[derive(Clone)]
pub struct MeteoraDbcAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub pool_authority: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub token_base_program: &'info AccountInfo<'info>,
    pub token_quote_program: &'info AccountInfo<'info>,
    pub referral_token_account: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
}

pub fn meteora_dbc_swap<'info>(
    accounts: MeteoraDbcAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;

    let metas = vec![
        AccountMeta::new_readonly(accounts.pool_authority.key(), false),
        AccountMeta::new_readonly(accounts.config.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.token_base_program.key(), false),
        AccountMeta::new_readonly(accounts.token_quote_program.key(), false),
        AccountMeta::new_readonly(accounts.referral_token_account.key(), false),
        AccountMeta::new_readonly(accounts.event_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
    ];

    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.pool_authority.clone(),
        accounts.config.clone(),
        accounts.pool.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.payer.clone(),
        accounts.token_base_program.clone(),
        accounts.token_quote_program.clone(),
        accounts.referral_token_account.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
    ];

    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(METEORA_DBC_SWAP2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.push(METEORA_DBC_EXACT_IN_MODE);

    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data,
    };

    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(accounts.output_token_account, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap2_data_layout_matches_meteora_dbc_contract() {
        let mut data = Vec::new();
        data.extend_from_slice(METEORA_DBC_SWAP2_SELECTOR);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&90_u64.to_le_bytes());
        data.push(METEORA_DBC_EXACT_IN_MODE);

        assert_eq!(data.len(), 25);
        assert_eq!(&data[0..8], METEORA_DBC_SWAP2_SELECTOR);
        assert_eq!(&data[8..16], &100_u64.to_le_bytes());
        assert_eq!(&data[16..24], &90_u64.to_le_bytes());
        assert_eq!(data[24], METEORA_DBC_EXACT_IN_MODE);
    }
}
