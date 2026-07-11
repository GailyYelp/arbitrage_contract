use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const MERCURIAL_STABLE_SWAP_TAG: u8 = 4;
pub const MERCURIAL_STABLE_SWAP_MIN_ACCOUNTS: usize = 5;

#[derive(Clone)]
pub struct MercurialStableSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_a_reserve: &'info AccountInfo<'info>,
    pub token_b_reserve: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn mercurial_stable_swap<'info>(
    accounts: MercurialStableSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.token_a_reserve.key(), false),
        AccountMeta::new(accounts.token_b_reserve.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
    ];
    let account_infos = vec![
        accounts.pool.clone(),
        accounts.token_program.clone(),
        accounts.authority.clone(),
        accounts.payer.clone(),
        accounts.token_a_reserve.clone(),
        accounts.token_b_reserve.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: mercurial_exchange_data(amount_in, minimum_amount_out),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn mercurial_exchange_data(amount_in: u64, minimum_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(17);
    data.push(MERCURIAL_STABLE_SWAP_TAG);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_data_matches_official_layout() {
        let data = mercurial_exchange_data(12_914_249, 0);
        assert_eq!(data.len(), 17);
        assert_eq!(data[0], MERCURIAL_STABLE_SWAP_TAG);
        assert_eq!(&data[1..9], &12_914_249_u64.to_le_bytes());
        assert_eq!(&data[9..17], &0_u64.to_le_bytes());
    }
}
