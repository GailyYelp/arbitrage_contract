use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const MANIFEST_MIN_ACCOUNTS: usize = 8;
pub const MANIFEST_SWAP_TAG: u8 = 4;
pub const MANIFEST_SWAP_DATA_LEN: usize = 19;

#[derive(Clone)]
pub struct ManifestSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub trader_base: &'info AccountInfo<'info>,
    pub trader_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub base_token_program: &'info AccountInfo<'info>,
    pub quote_token_program: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
}

pub fn manifest_swap<'info>(
    accounts: ManifestSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_in: bool,
) -> Result<SwapResult> {
    let output_account = if is_base_in {
        accounts.trader_quote
    } else {
        accounts.trader_base
    };
    let pre_out = read_token_amount(output_account)?;
    let mut data = Vec::with_capacity(MANIFEST_SWAP_DATA_LEN);
    data.push(MANIFEST_SWAP_TAG);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.push(u8::from(is_base_in));
    data.push(1); // exact-in

    let (metas, mut account_infos) = manifest_swap_accounts(&accounts);
    account_infos.push(accounts.program.clone());
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &account_infos,
    )?;

    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

fn manifest_swap_accounts<'info>(
    accounts: &ManifestSwapAccounts<'info>,
) -> (Vec<AccountMeta>, Vec<AccountInfo<'info>>) {
    let mut metas = vec![
        AccountMeta::new(accounts.payer.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new(accounts.trader_base.key(), false),
        AccountMeta::new(accounts.trader_quote.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.base_token_program.key(), false),
    ];
    let mut infos = vec![
        accounts.payer.clone(),
        accounts.market.clone(),
        accounts.system_program.clone(),
        accounts.trader_base.clone(),
        accounts.trader_quote.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.base_token_program.clone(),
    ];

    if accounts.base_token_program.key() == anchor_spl::token_2022::ID {
        metas.push(AccountMeta::new_readonly(accounts.base_mint.key(), false));
        infos.push(accounts.base_mint.clone());
    }
    if accounts.base_token_program.key() != accounts.quote_token_program.key() {
        metas.push(AccountMeta::new_readonly(
            accounts.quote_token_program.key(),
            false,
        ));
        infos.push(accounts.quote_token_program.clone());
    }
    if accounts.quote_token_program.key() == anchor_spl::token_2022::ID {
        metas.push(AccountMeta::new_readonly(accounts.quote_mint.key(), false));
        infos.push(accounts.quote_mint.clone());
    }

    (metas, infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(key: Pubkey, owner: Pubkey) -> &'static AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(owner));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        Box::leak(Box::new(AccountInfo::new(
            key, false, false, lamports, data, owner, false, 0,
        )))
    }

    #[test]
    fn account_layout_appends_only_required_token_2022_accounts() {
        let native = anchor_spl::token::ID;
        let token_2022 = anchor_spl::token_2022::ID;
        let accounts = ManifestSwapAccounts {
            program: account(Pubkey::new_unique(), native),
            payer: account(Pubkey::new_unique(), native),
            market: account(Pubkey::new_unique(), native),
            system_program: account(anchor_lang::system_program::ID, native),
            trader_base: account(Pubkey::new_unique(), native),
            trader_quote: account(Pubkey::new_unique(), native),
            base_vault: account(Pubkey::new_unique(), native),
            quote_vault: account(Pubkey::new_unique(), native),
            base_token_program: account(token_2022, native),
            quote_token_program: account(native, native),
            base_mint: account(Pubkey::new_unique(), token_2022),
            quote_mint: account(Pubkey::new_unique(), native),
        };
        let (metas, infos) = manifest_swap_accounts(&accounts);
        assert_eq!(metas.len(), infos.len());
        assert_eq!(metas.len(), 10);
        assert_eq!(metas[8].pubkey, accounts.base_mint.key());
        assert_eq!(metas[9].pubkey, native);
    }

    #[test]
    fn swap_data_layout_is_tag_two_u64s_and_two_bools() {
        let mut data = Vec::with_capacity(MANIFEST_SWAP_DATA_LEN);
        data.push(MANIFEST_SWAP_TAG);
        data.extend_from_slice(&12_u64.to_le_bytes());
        data.extend_from_slice(&9_u64.to_le_bytes());
        data.push(1);
        data.push(1);
        assert_eq!(data.len(), MANIFEST_SWAP_DATA_LEN);
        assert_eq!(data[0], 4);
        assert_eq!(&data[1..9], &12_u64.to_le_bytes());
        assert_eq!(&data[9..17], &9_u64.to_le_bytes());
        assert_eq!(&data[17..], &[1, 1]);
    }
}
