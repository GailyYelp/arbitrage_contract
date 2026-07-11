use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const METEORA_DAMM_V1_SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const METEORA_DAMM_V1_MIN_ACCOUNTS: usize = 13;

#[derive(Clone)]
pub struct MeteoraDammV1Accounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub a_vault: &'info AccountInfo<'info>,
    pub b_vault: &'info AccountInfo<'info>,
    pub a_token_vault: &'info AccountInfo<'info>,
    pub b_token_vault: &'info AccountInfo<'info>,
    pub a_vault_lp_mint: &'info AccountInfo<'info>,
    pub b_vault_lp_mint: &'info AccountInfo<'info>,
    pub a_vault_lp: &'info AccountInfo<'info>,
    pub b_vault_lp: &'info AccountInfo<'info>,
    pub protocol_token_fee: &'info AccountInfo<'info>,
    pub vault_program: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn meteora_damm_v1_swap<'info>(
    accounts: MeteoraDammV1Accounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;

    let metas = vec![
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.a_vault.key(), false),
        AccountMeta::new(accounts.b_vault.key(), false),
        AccountMeta::new(accounts.a_token_vault.key(), false),
        AccountMeta::new(accounts.b_token_vault.key(), false),
        AccountMeta::new(accounts.a_vault_lp_mint.key(), false),
        AccountMeta::new(accounts.b_vault_lp_mint.key(), false),
        AccountMeta::new(accounts.a_vault_lp.key(), false),
        AccountMeta::new(accounts.b_vault_lp.key(), false),
        AccountMeta::new(accounts.protocol_token_fee.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.vault_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];

    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.pool.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.a_vault.clone(),
        accounts.b_vault.clone(),
        accounts.a_token_vault.clone(),
        accounts.b_token_vault.clone(),
        accounts.a_vault_lp_mint.clone(),
        accounts.b_vault_lp_mint.clone(),
        accounts.a_vault_lp.clone(),
        accounts.b_vault_lp.clone(),
        accounts.protocol_token_fee.clone(),
        accounts.payer.clone(),
        accounts.vault_program.clone(),
        accounts.token_program.clone(),
    ];

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(METEORA_DAMM_V1_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());

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
    fn swap_data_layout_matches_meteora_damm_v1_builder_contract() {
        let mut data = Vec::new();
        data.extend_from_slice(METEORA_DAMM_V1_SWAP_SELECTOR);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&90_u64.to_le_bytes());

        assert_eq!(data.len(), 24);
        assert_eq!(&data[0..8], METEORA_DAMM_V1_SWAP_SELECTOR);
        assert_eq!(&data[8..16], &100_u64.to_le_bytes());
        assert_eq!(&data[16..24], &90_u64.to_le_bytes());
    }
}
