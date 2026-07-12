use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const CREMA_CLMM_SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
pub const CREMA_CLMM_MIN_SQRT_PRICE_X64: u128 = 4_295_048_016;
pub const CREMA_CLMM_MAX_SQRT_PRICE_X64: u128 = 79_226_673_515_401_279_992_447_579_055;
pub const CREMA_CLMM_FIXED_STEP_ACCOUNTS: usize = 7;
pub const CREMA_CLMM_MIN_STEP_ACCOUNTS: usize = 8;
pub const CREMA_CLMM_MAX_STEP_ACCOUNTS: usize = 10;

#[derive(Clone)]
pub struct CremaClmmAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub clmm_config: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub token_a_mint: &'info AccountInfo<'info>,
    pub token_b_mint: &'info AccountInfo<'info>,
    pub account_a: &'info AccountInfo<'info>,
    pub account_b: &'info AccountInfo<'info>,
    pub token_a_vault: &'info AccountInfo<'info>,
    pub token_b_vault: &'info AccountInfo<'info>,
    pub tick_array_map: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn crema_clmm_swap<'info>(
    accounts: CremaClmmAccounts<'info>,
    tick_arrays: &'info [AccountInfo<'info>],
    amount_in: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Result<SwapResult> {
    require!(
        (1..=3).contains(&tick_arrays.len()),
        ArbitrageError::InvalidAccountCount
    );
    let output_token_account = if a_to_b {
        accounts.account_b
    } else {
        accounts.account_a
    };
    let pre_out = read_token_amount(output_token_account)?;
    let mut metas = vec![
        AccountMeta::new_readonly(accounts.clmm_config.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.token_a_mint.key(), false),
        AccountMeta::new_readonly(accounts.token_b_mint.key(), false),
        AccountMeta::new(accounts.account_a.key(), false),
        AccountMeta::new(accounts.account_b.key(), false),
        AccountMeta::new(accounts.token_a_vault.key(), false),
        AccountMeta::new(accounts.token_b_vault.key(), false),
        AccountMeta::new(accounts.tick_array_map.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    metas.extend(
        tick_arrays
            .iter()
            .map(|account| AccountMeta::new(account.key(), false)),
    );
    let mut account_infos = vec![
        accounts.clmm_config.clone(),
        accounts.pool.clone(),
        accounts.token_a_mint.clone(),
        accounts.token_b_mint.clone(),
        accounts.account_a.clone(),
        accounts.account_b.clone(),
        accounts.token_a_vault.clone(),
        accounts.token_b_vault.clone(),
        accounts.tick_array_map.clone(),
        accounts.payer.clone(),
        accounts.token_program.clone(),
    ];
    account_infos.extend(tick_arrays.iter().cloned());
    account_infos.push(accounts.program.clone());
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: crema_clmm_swap_data(amount_in, min_amount_out, a_to_b),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn crema_clmm_swap_data(amount_in: u64, min_amount_out: u64, a_to_b: bool) -> Vec<u8> {
    let sqrt_price_limit = if a_to_b {
        CREMA_CLMM_MIN_SQRT_PRICE_X64
    } else {
        CREMA_CLMM_MAX_SQRT_PRICE_X64
    };
    let mut data = Vec::with_capacity(42);
    data.extend_from_slice(&CREMA_CLMM_SWAP_DISCRIMINATOR);
    data.push(u8::from(a_to_b));
    data.push(1);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_data_matches_published_crema_borsh_layout() {
        let data = crema_clmm_swap_data(85_000_000, 827_000_000, false);
        assert_eq!(data.len(), 42);
        assert_eq!(&data[..8], &CREMA_CLMM_SWAP_DISCRIMINATOR);
        assert_eq!(data[8], 0);
        assert_eq!(data[9], 1);
        assert_eq!(&data[10..18], &85_000_000_u64.to_le_bytes());
        assert_eq!(&data[18..26], &827_000_000_u64.to_le_bytes());
        assert_eq!(&data[26..42], &CREMA_CLMM_MAX_SQRT_PRICE_X64.to_le_bytes());
    }
}
