use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const STABBLE_SWAP_V2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const STABBLE_SWAP_MIN_ACCOUNTS: usize = 12;

#[derive(Clone)]
pub struct StabbleSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub mint_in: &'info AccountInfo<'info>,
    pub mint_out: &'info AccountInfo<'info>,
    pub user_token_in: &'info AccountInfo<'info>,
    pub user_token_out: &'info AccountInfo<'info>,
    pub vault_token_in: &'info AccountInfo<'info>,
    pub vault_token_out: &'info AccountInfo<'info>,
    pub beneficiary_token_out: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub withdraw_authority: &'info AccountInfo<'info>,
    pub vault: &'info AccountInfo<'info>,
    pub vault_authority: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub token_2022_program: &'info AccountInfo<'info>,
}

pub fn stabble_swap_v2<'info>(
    accounts: StabbleSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.user_token_out)?;

    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.mint_in.key(), false),
        AccountMeta::new_readonly(accounts.mint_out.key(), false),
        AccountMeta::new(accounts.user_token_in.key(), false),
        AccountMeta::new(accounts.user_token_out.key(), false),
        AccountMeta::new(accounts.vault_token_in.key(), false),
        AccountMeta::new(accounts.vault_token_out.key(), false),
        AccountMeta::new(accounts.beneficiary_token_out.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.withdraw_authority.key(), false),
        AccountMeta::new_readonly(accounts.vault.key(), false),
        AccountMeta::new_readonly(accounts.vault_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_2022_program.key(), false),
    ];
    let account_infos = vec![
        accounts.payer.clone(),
        accounts.mint_in.clone(),
        accounts.mint_out.clone(),
        accounts.user_token_in.clone(),
        accounts.user_token_out.clone(),
        accounts.vault_token_in.clone(),
        accounts.vault_token_out.clone(),
        accounts.beneficiary_token_out.clone(),
        accounts.pool.clone(),
        accounts.withdraw_authority.clone(),
        accounts.vault.clone(),
        accounts.vault_authority.clone(),
        accounts.program.clone(),
        accounts.token_program.clone(),
        accounts.token_2022_program.clone(),
    ];

    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: stabble_swap_v2_data(amount_in, minimum_amount_out),
    };
    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(accounts.user_token_out, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

fn stabble_swap_v2_data(amount_in: u64, minimum_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(STABBLE_SWAP_V2_SELECTOR);
    data.push(1);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_v2_data_layout_matches_stabble_builder_contract() {
        let data = stabble_swap_v2_data(100, 90);

        assert_eq!(data.len(), 25);
        assert_eq!(&data[0..8], STABBLE_SWAP_V2_SELECTOR);
        assert_eq!(data[8], 1);
        assert_eq!(&data[9..17], &100_u64.to_le_bytes());
        assert_eq!(&data[17..25], &90_u64.to_le_bytes());
    }
}
