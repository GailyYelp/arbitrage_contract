use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const WOOFI_SWAP_SELECTOR: &[u8; 8] = &[248, 198, 158, 145, 225, 117, 135, 200];
pub const WOOFI_SWAP_MIN_ACCOUNTS: usize = 15;

#[derive(Clone)]
pub struct WoofiSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub wooconfig: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub wooracle_from: &'info AccountInfo<'info>,
    pub woopool_from: &'info AccountInfo<'info>,
    pub token_owner_account_from: &'info AccountInfo<'info>,
    pub token_vault_from: &'info AccountInfo<'info>,
    pub price_update_from: &'info AccountInfo<'info>,
    pub wooracle_to: &'info AccountInfo<'info>,
    pub woopool_to: &'info AccountInfo<'info>,
    pub token_owner_account_to: &'info AccountInfo<'info>,
    pub token_vault_to: &'info AccountInfo<'info>,
    pub price_update_to: &'info AccountInfo<'info>,
    pub woopool_quote: &'info AccountInfo<'info>,
    pub quote_price_update: &'info AccountInfo<'info>,
    pub quote_token_vault: &'info AccountInfo<'info>,
    pub rebate_to: &'info AccountInfo<'info>,
}

pub fn woofi_swap<'info>(
    accounts: WoofiSwapAccounts<'info>,
    from_amount: u64,
    min_to_amount: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.token_owner_account_to)?;

    let metas = woofi_swap_account_metas(&accounts);
    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.wooconfig.clone(),
        accounts.token_program.clone(),
        accounts.payer.clone(),
        accounts.wooracle_from.clone(),
        accounts.woopool_from.clone(),
        accounts.token_owner_account_from.clone(),
        accounts.token_vault_from.clone(),
        accounts.price_update_from.clone(),
        accounts.wooracle_to.clone(),
        accounts.woopool_to.clone(),
        accounts.token_owner_account_to.clone(),
        accounts.token_vault_to.clone(),
        accounts.price_update_to.clone(),
        accounts.woopool_quote.clone(),
        accounts.quote_price_update.clone(),
        accounts.quote_token_vault.clone(),
        accounts.rebate_to.clone(),
        accounts.program.clone(),
    ];

    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: woofi_swap_data(from_amount, min_to_amount),
    };
    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(accounts.token_owner_account_to, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

fn woofi_swap_account_metas(accounts: &WoofiSwapAccounts<'_>) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(accounts.wooconfig.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.wooracle_from.key(), false),
        AccountMeta::new(accounts.woopool_from.key(), false),
        AccountMeta::new(accounts.token_owner_account_from.key(), false),
        AccountMeta::new(accounts.token_vault_from.key(), false),
        AccountMeta::new(accounts.price_update_from.key(), false),
        AccountMeta::new(accounts.wooracle_to.key(), false),
        AccountMeta::new(accounts.woopool_to.key(), false),
        AccountMeta::new(accounts.token_owner_account_to.key(), false),
        AccountMeta::new(accounts.token_vault_to.key(), false),
        AccountMeta::new(accounts.price_update_to.key(), false),
        AccountMeta::new(accounts.woopool_quote.key(), false),
        AccountMeta::new_readonly(accounts.quote_price_update.key(), false),
        AccountMeta::new(accounts.quote_token_vault.key(), false),
        AccountMeta::new_readonly(accounts.rebate_to.key(), false),
    ]
}

fn woofi_swap_data(from_amount: u64, min_to_amount: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(WOOFI_SWAP_SELECTOR);
    data.extend_from_slice(&u128::from(from_amount).to_le_bytes());
    data.extend_from_slice(&u128::from(min_to_amount).to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_data_layout_matches_generated_woofi_builder() {
        let data = woofi_swap_data(100, 90);

        assert_eq!(data.len(), 40);
        assert_eq!(&data[0..8], WOOFI_SWAP_SELECTOR);
        assert_eq!(&data[8..24], &100_u128.to_le_bytes());
        assert_eq!(&data[24..40], &90_u128.to_le_bytes());
    }
}
