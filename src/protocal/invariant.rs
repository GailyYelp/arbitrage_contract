use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const INVARIANT_SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
pub const INVARIANT_MIN_SQRT_PRICE: u128 = 15_258_932_000_000_000_000;
pub const INVARIANT_MAX_SQRT_PRICE: u128 = 65_535_383_934_512_647_000_000_000_000;
pub const INVARIANT_MIN_ACCOUNTS: usize = 7;

#[derive(Clone)]
pub struct InvariantAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub state: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub tickmap: &'info AccountInfo<'info>,
    pub account_x: &'info AccountInfo<'info>,
    pub account_y: &'info AccountInfo<'info>,
    pub reserve_x: &'info AccountInfo<'info>,
    pub reserve_y: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn invariant_swap<'info>(
    accounts: InvariantAccounts<'info>,
    amount_in: u64,
    x_to_y: bool,
) -> Result<SwapResult> {
    let output_token_account = if x_to_y {
        accounts.account_y
    } else {
        accounts.account_x
    };
    let pre_out = read_token_amount(output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.state.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new(accounts.tickmap.key(), false),
        AccountMeta::new(accounts.account_x.key(), false),
        AccountMeta::new(accounts.account_y.key(), false),
        AccountMeta::new(accounts.reserve_x.key(), false),
        AccountMeta::new(accounts.reserve_y.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let account_infos = vec![
        accounts.state.clone(),
        accounts.pool.clone(),
        accounts.tickmap.clone(),
        accounts.account_x.clone(),
        accounts.account_y.clone(),
        accounts.reserve_x.clone(),
        accounts.reserve_y.clone(),
        accounts.payer.clone(),
        accounts.authority.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: invariant_swap_data(amount_in, x_to_y),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn invariant_swap_data(amount_in: u64, x_to_y: bool) -> Vec<u8> {
    let sqrt_price_limit = if x_to_y {
        INVARIANT_MIN_SQRT_PRICE
    } else {
        INVARIANT_MAX_SQRT_PRICE
    };
    let mut data = Vec::with_capacity(34);
    data.extend_from_slice(&INVARIANT_SWAP_DISCRIMINATOR);
    data.push(u8::from(x_to_y));
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(1);
    data.extend_from_slice(&sqrt_price_limit.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_data_matches_official_borsh_layout() {
        let down = invariant_swap_data(1_000_000, true);
        assert_eq!(down.len(), 34);
        assert_eq!(&down[..8], &INVARIANT_SWAP_DISCRIMINATOR);
        assert_eq!(down[8], 1);
        assert_eq!(&down[9..17], &1_000_000_u64.to_le_bytes());
        assert_eq!(down[17], 1);
        assert_eq!(&down[18..34], &INVARIANT_MIN_SQRT_PRICE.to_le_bytes());

        let up = invariant_swap_data(2_000_000, false);
        assert_eq!(up[8], 0);
        assert_eq!(&up[18..34], &INVARIANT_MAX_SQRT_PRICE.to_le_bytes());
    }
}
