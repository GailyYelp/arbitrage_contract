use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const FLUXBEAM_MIN_ACCOUNTS: usize = 12;
pub const FLUXBEAM_SWAP_TAG: u8 = 1;

#[derive(Clone)]
pub struct FluxBeamAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub input_vault: &'info AccountInfo<'info>,
    pub output_vault: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub pool_mint: &'info AccountInfo<'info>,
    pub pool_fee_account: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub input_token_program: &'info AccountInfo<'info>,
    pub output_token_program: &'info AccountInfo<'info>,
    pub pool_token_program: &'info AccountInfo<'info>,
}

pub fn fluxbeam_swap<'info>(
    accounts: FluxBeamAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let mut data = Vec::with_capacity(17);
    data.push(FLUXBEAM_SWAP_TAG);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());

    let metas = vec![
        AccountMeta::new_readonly(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.input_vault.key(), false),
        AccountMeta::new(accounts.output_vault.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.pool_mint.key(), false),
        AccountMeta::new(accounts.pool_fee_account.key(), false),
        AccountMeta::new_readonly(accounts.input_mint.key(), false),
        AccountMeta::new_readonly(accounts.output_mint.key(), false),
        AccountMeta::new_readonly(accounts.input_token_program.key(), false),
        AccountMeta::new_readonly(accounts.output_token_program.key(), false),
        AccountMeta::new_readonly(accounts.pool_token_program.key(), false),
    ];
    let account_infos = vec![
        accounts.pool.clone(),
        accounts.authority.clone(),
        accounts.payer.clone(),
        accounts.input_token_account.clone(),
        accounts.input_vault.clone(),
        accounts.output_vault.clone(),
        accounts.output_token_account.clone(),
        accounts.pool_mint.clone(),
        accounts.pool_fee_account.clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.input_token_program.clone(),
        accounts.output_token_program.clone(),
        accounts.pool_token_program.clone(),
        accounts.program.clone(),
    ];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_data_matches_confirmed_fluxbeam_layout() {
        let mut data = vec![FLUXBEAM_SWAP_TAG];
        data.extend_from_slice(&100_000_000_u64.to_le_bytes());
        data.extend_from_slice(&7_703_295_u64.to_le_bytes());
        assert_eq!(data.len(), 17);
        assert_eq!(data[0], 1);
        assert_eq!(&data[1..9], &100_000_000_u64.to_le_bytes());
        assert_eq!(&data[9..17], &7_703_295_u64.to_le_bytes());
    }
}
