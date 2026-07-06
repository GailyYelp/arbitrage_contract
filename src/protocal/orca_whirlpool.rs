use crate::instructions::types::read_token_amount;
use crate::instructions::types::token_balance_delta;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const ORCA_WHIRLPOOL_SWAP_V2_SELECTOR: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
pub const ORCA_WHIRLPOOL_MIN_ACCOUNTS: usize = 13;

#[derive(Clone)]
pub struct OrcaWhirlpoolAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_program_a: &'info AccountInfo<'info>,
    pub token_program_b: &'info AccountInfo<'info>,
    pub memo_program: &'info AccountInfo<'info>,
    pub whirlpool: &'info AccountInfo<'info>,
    pub token_mint_a: &'info AccountInfo<'info>,
    pub token_mint_b: &'info AccountInfo<'info>,
    pub token_owner_account_a: &'info AccountInfo<'info>,
    pub token_vault_a: &'info AccountInfo<'info>,
    pub token_owner_account_b: &'info AccountInfo<'info>,
    pub token_vault_b: &'info AccountInfo<'info>,
    pub tick_array_0: &'info AccountInfo<'info>,
    pub tick_array_1: &'info AccountInfo<'info>,
    pub tick_array_2: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
}

pub fn orca_whirlpool_swap<'info>(
    accounts: OrcaWhirlpoolAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    a_to_b: bool,
) -> Result<SwapResult> {
    let output_token_account = if a_to_b {
        accounts.token_owner_account_b
    } else {
        accounts.token_owner_account_a
    };
    let pre_out = read_token_amount(output_token_account)?;

    let metas = vec![
        AccountMeta::new_readonly(accounts.token_program_a.key(), false),
        AccountMeta::new_readonly(accounts.token_program_b.key(), false),
        AccountMeta::new_readonly(accounts.memo_program.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.whirlpool.key(), false),
        AccountMeta::new_readonly(accounts.token_mint_a.key(), false),
        AccountMeta::new_readonly(accounts.token_mint_b.key(), false),
        AccountMeta::new(accounts.token_owner_account_a.key(), false),
        AccountMeta::new(accounts.token_vault_a.key(), false),
        AccountMeta::new(accounts.token_owner_account_b.key(), false),
        AccountMeta::new(accounts.token_vault_b.key(), false),
        AccountMeta::new(accounts.tick_array_0.key(), false),
        AccountMeta::new(accounts.tick_array_1.key(), false),
        AccountMeta::new(accounts.tick_array_2.key(), false),
        AccountMeta::new(accounts.oracle.key(), false),
    ];

    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.token_program_a.clone(),
        accounts.token_program_b.clone(),
        accounts.memo_program.clone(),
        accounts.payer.clone(),
        accounts.whirlpool.clone(),
        accounts.token_mint_a.clone(),
        accounts.token_mint_b.clone(),
        accounts.token_owner_account_a.clone(),
        accounts.token_vault_a.clone(),
        accounts.token_owner_account_b.clone(),
        accounts.token_vault_b.clone(),
        accounts.tick_array_0.clone(),
        accounts.tick_array_1.clone(),
        accounts.tick_array_2.clone(),
        accounts.oracle.clone(),
        accounts.program.clone(),
    ];

    let mut data = Vec::with_capacity(43);
    data.extend_from_slice(ORCA_WHIRLPOOL_SWAP_V2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&0_u128.to_le_bytes());
    data.push(1_u8);
    data.push(u8::from(a_to_b));
    data.push(0_u8);

    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data,
    };

    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(output_token_account, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_v2_data_layout_matches_orca_builder_contract() {
        let mut data = Vec::new();
        data.extend_from_slice(ORCA_WHIRLPOOL_SWAP_V2_SELECTOR);
        data.extend_from_slice(&100_u64.to_le_bytes());
        data.extend_from_slice(&90_u64.to_le_bytes());
        data.extend_from_slice(&0_u128.to_le_bytes());
        data.push(1_u8);
        data.push(1_u8);
        data.push(0_u8);

        assert_eq!(data.len(), 43);
        assert_eq!(&data[0..8], ORCA_WHIRLPOOL_SWAP_V2_SELECTOR);
        assert_eq!(&data[8..16], &100_u64.to_le_bytes());
        assert_eq!(&data[16..24], &90_u64.to_le_bytes());
        assert_eq!(&data[24..40], &0_u128.to_le_bytes());
        assert_eq!(data[40], 1_u8);
        assert_eq!(data[41], 1_u8);
        assert_eq!(data[42], 0_u8);
    }
}
