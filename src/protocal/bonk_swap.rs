use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const BONK_SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
pub const BONK_SWAP_MIN_ACCOUNTS: usize = 10;

#[derive(Clone)]
pub struct BonkSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub state: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub token_x: &'info AccountInfo<'info>,
    pub token_y: &'info AccountInfo<'info>,
    pub pool_x_account: &'info AccountInfo<'info>,
    pub pool_y_account: &'info AccountInfo<'info>,
    pub swapper_x_account: &'info AccountInfo<'info>,
    pub swapper_y_account: &'info AccountInfo<'info>,
    pub swapper: &'info AccountInfo<'info>,
    pub referrer_x_account: &'info AccountInfo<'info>,
    pub referrer_y_account: &'info AccountInfo<'info>,
    pub referrer: &'info AccountInfo<'info>,
    pub program_authority: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub rent: &'info AccountInfo<'info>,
}

pub fn bonk_swap<'info>(
    accounts: BonkSwapAccounts<'info>,
    amount_in: u64,
    x_to_y: bool,
    pool_price: u128,
) -> Result<SwapResult> {
    let output_token_account = if x_to_y {
        accounts.swapper_y_account
    } else {
        accounts.swapper_x_account
    };
    let pre_out = read_token_amount(output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.state.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.token_x.key(), false),
        AccountMeta::new_readonly(accounts.token_y.key(), false),
        AccountMeta::new(accounts.pool_x_account.key(), false),
        AccountMeta::new(accounts.pool_y_account.key(), false),
        AccountMeta::new(accounts.swapper_x_account.key(), false),
        AccountMeta::new(accounts.swapper_y_account.key(), false),
        AccountMeta::new(accounts.swapper.key(), true),
        AccountMeta::new(accounts.referrer_x_account.key(), false),
        AccountMeta::new(accounts.referrer_y_account.key(), false),
        AccountMeta::new(accounts.referrer.key(), false),
        AccountMeta::new_readonly(accounts.program_authority.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(accounts.rent.key(), false),
    ];
    let account_infos = vec![
        accounts.state.clone(),
        accounts.pool.clone(),
        accounts.token_x.clone(),
        accounts.token_y.clone(),
        accounts.pool_x_account.clone(),
        accounts.pool_y_account.clone(),
        accounts.swapper_x_account.clone(),
        accounts.swapper_y_account.clone(),
        accounts.swapper.clone(),
        accounts.referrer_x_account.clone(),
        accounts.referrer_y_account.clone(),
        accounts.referrer.clone(),
        accounts.program_authority.clone(),
        accounts.system_program.clone(),
        accounts.token_program.clone(),
        accounts.associated_token_program.clone(),
        accounts.rent.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: bonk_swap_data(amount_in, x_to_y, pool_price)?,
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn bonk_swap_data(amount_in: u64, x_to_y: bool, pool_price: u128) -> Result<Vec<u8>> {
    let price_limit = if x_to_y {
        0
    } else {
        pool_price
            .checked_mul(2)
            .ok_or(crate::errors::ArbitrageError::MathOverflow)?
    };
    let mut data = Vec::with_capacity(33);
    data.extend_from_slice(&BONK_SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&price_limit.to_le_bytes());
    data.push(u8::from(x_to_y));
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_data_matches_official_borsh_layout() {
        let x_to_y = bonk_swap_data(1_000_000, true, 2_000_000_000_000).unwrap();
        assert_eq!(x_to_y.len(), 33);
        assert_eq!(&x_to_y[..8], &BONK_SWAP_DISCRIMINATOR);
        assert_eq!(&x_to_y[8..16], &1_000_000_u64.to_le_bytes());
        assert_eq!(&x_to_y[16..32], &0_u128.to_le_bytes());
        assert_eq!(x_to_y[32], 1);

        let y_to_x = bonk_swap_data(2_000_000, false, 3_000_000_000_000).unwrap();
        assert_eq!(&y_to_x[16..32], &6_000_000_000_000_u128.to_le_bytes());
        assert_eq!(y_to_x[32], 0);
    }
}
