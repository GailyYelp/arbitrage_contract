use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const SABER_STABLE_SWAP_TAG: u8 = 1;
pub const SABER_STABLE_SWAP_MIN_ACCOUNTS: usize = 7;

#[derive(Clone)]
pub struct SaberStableSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub input_reserve: &'info AccountInfo<'info>,
    pub output_reserve: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub output_admin_fees: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn saber_stable_swap<'info>(
    accounts: SaberStableSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.input_reserve.key(), false),
        AccountMeta::new(accounts.output_reserve.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.output_admin_fees.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let account_infos = vec![
        accounts.pool.clone(),
        accounts.authority.clone(),
        accounts.payer.clone(),
        accounts.input_token_account.clone(),
        accounts.input_reserve.clone(),
        accounts.output_reserve.clone(),
        accounts.output_token_account.clone(),
        accounts.output_admin_fees.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: saber_stable_swap_data(amount_in, minimum_amount_out),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn saber_stable_swap_data(amount_in: u64, minimum_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(17);
    data.push(SABER_STABLE_SWAP_TAG);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_data_matches_official_layout() {
        let data = saber_stable_swap_data(4_950_000, 4_945_794);
        assert_eq!(data.len(), 17);
        assert_eq!(data[0], SABER_STABLE_SWAP_TAG);
        assert_eq!(&data[1..9], &4_950_000_u64.to_le_bytes());
        assert_eq!(&data[9..17], &4_945_794_u64.to_le_bytes());
    }
}
