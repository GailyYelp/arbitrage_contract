use anchor_lang::{
    prelude::{AccountInfo, Result},
    solana_program::{instruction::AccountMeta, program::invoke},
};

use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const LIFINITY_AMM_V2_MIN_ACCOUNTS: usize = 13;
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

pub struct LifinityAmmV2SwapAccounts<'info> {
    pub program: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub amm: AccountInfo<'info>,
    pub user_transfer_authority: AccountInfo<'info>,
    pub source_info: AccountInfo<'info>,
    pub destination_info: AccountInfo<'info>,
    pub swap_source: AccountInfo<'info>,
    pub swap_destination: AccountInfo<'info>,
    pub pool_mint: AccountInfo<'info>,
    pub fee_account: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub oracle_main_account: AccountInfo<'info>,
    pub oracle_sub_account: AccountInfo<'info>,
    pub oracle_pc_account: AccountInfo<'info>,
}

pub fn lifinity_amm_v2_swap<'info>(
    amount_in: u64,
    minimum_amount_out: u64,
    accounts: LifinityAmmV2SwapAccounts<'info>,
) -> Result<SwapResult> {
    let before = read_token_amount(&accounts.destination_info)?;
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    let metas = vec![
        AccountMeta::new_readonly(*accounts.authority.key, false),
        AccountMeta::new(*accounts.amm.key, false),
        AccountMeta::new_readonly(*accounts.user_transfer_authority.key, true),
        AccountMeta::new(*accounts.source_info.key, false),
        AccountMeta::new(*accounts.destination_info.key, false),
        AccountMeta::new(*accounts.swap_source.key, false),
        AccountMeta::new(*accounts.swap_destination.key, false),
        AccountMeta::new(*accounts.pool_mint.key, false),
        AccountMeta::new(*accounts.fee_account.key, false),
        AccountMeta::new_readonly(*accounts.token_program.key, false),
        AccountMeta::new_readonly(*accounts.oracle_main_account.key, false),
        AccountMeta::new_readonly(*accounts.oracle_sub_account.key, false),
        AccountMeta::new_readonly(*accounts.oracle_pc_account.key, false),
    ];
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: *accounts.program.key,
        accounts: metas,
        data,
    };
    invoke(
        &ix,
        &[
            accounts.authority,
            accounts.amm,
            accounts.user_transfer_authority,
            accounts.source_info,
            accounts.destination_info.clone(),
            accounts.swap_source,
            accounts.swap_destination,
            accounts.pool_mint,
            accounts.fee_account,
            accounts.token_program,
            accounts.oracle_main_account,
            accounts.oracle_sub_account,
            accounts.oracle_pc_account,
            accounts.program,
        ],
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(&accounts.destination_info, before)?,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_data_matches_anchor_discriminator_and_two_u64_args() {
        let amount_in = 123_u64;
        let minimum = 45_u64;
        let mut data = Vec::new();
        data.extend_from_slice(&SWAP_DISCRIMINATOR);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&minimum.to_le_bytes());

        assert_eq!(data.len(), 24);
        assert_eq!(&data[..8], &SWAP_DISCRIMINATOR);
        assert_eq!(&data[8..16], &amount_in.to_le_bytes());
        assert_eq!(&data[16..24], &minimum.to_le_bytes());
    }
}
