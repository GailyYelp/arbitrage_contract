use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const SENCHA_SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const SENCHA_SWAP_MIN_ACCOUNTS: usize = 6;

#[derive(Clone)]
pub struct SenchaSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub input_reserve: &'info AccountInfo<'info>,
    pub input_fees: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub output_reserve: &'info AccountInfo<'info>,
    pub output_fees: &'info AccountInfo<'info>,
}

pub fn sencha_swap<'info>(
    accounts: SenchaSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.input_reserve.key(), false),
        AccountMeta::new(accounts.input_fees.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.output_reserve.key(), false),
        AccountMeta::new(accounts.output_fees.key(), false),
    ];
    let account_infos = vec![
        accounts.token_program.clone(),
        accounts.pool.clone(),
        accounts.payer.clone(),
        accounts.input_token_account.clone(),
        accounts.input_reserve.clone(),
        accounts.input_fees.clone(),
        accounts.output_token_account.clone(),
        accounts.output_reserve.clone(),
        accounts.output_fees.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: sencha_swap_data(amount_in, minimum_amount_out),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn sencha_swap_data(amount_in: u64, minimum_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(SENCHA_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_data_matches_verified_idl() {
        let data = sencha_swap_data(29_478, 4_147_016);
        assert_eq!(data.len(), 24);
        assert_eq!(&data[..8], SENCHA_SWAP_SELECTOR);
        assert_eq!(&data[8..16], &29_478_u64.to_le_bytes());
        assert_eq!(&data[16..], &4_147_016_u64.to_le_bytes());
    }
}
