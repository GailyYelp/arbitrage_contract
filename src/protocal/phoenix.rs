use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const PHOENIX_MIN_ACCOUNTS: usize = 8;
pub const PHOENIX_SWAP_TAG: u8 = 0;
pub const PHOENIX_IOC_VARIANT: u8 = 2;
pub const PHOENIX_DEFAULT_MATCH_LIMIT: u64 = 2_048;
pub const PHOENIX_SWAP_DATA_LEN: usize = 65;

#[derive(Clone)]
pub struct PhoenixSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub log_authority: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub trader_base: &'info AccountInfo<'info>,
    pub trader_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn phoenix_swap<'info>(
    accounts: PhoenixSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_in: bool,
    base_lot_size: u64,
    quote_lot_size: u64,
) -> Result<SwapResult> {
    let output_account = if is_base_in {
        accounts.trader_quote
    } else {
        accounts.trader_base
    };
    let pre_out = read_token_amount(output_account)?;
    let data = phoenix_swap_data(
        amount_in,
        minimum_amount_out,
        is_base_in,
        base_lot_size,
        quote_lot_size,
    )?;
    let (metas, account_infos) = phoenix_swap_accounts(&accounts);
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

fn phoenix_swap_data(
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_in: bool,
    base_lot_size: u64,
    quote_lot_size: u64,
) -> Result<Vec<u8>> {
    require!(base_lot_size > 0, ArbitrageError::InvalidAmount);
    require!(quote_lot_size > 0, ArbitrageError::InvalidAmount);
    let (side, num_base_lots, num_quote_lots, min_base_lots, min_quote_lots) = if is_base_in {
        (
            1_u8,
            floor_nonzero_lots(amount_in, base_lot_size)?,
            0,
            0,
            ceil_lots(minimum_amount_out, quote_lot_size)?,
        )
    } else {
        (
            0_u8,
            0,
            floor_nonzero_lots(amount_in, quote_lot_size)?,
            ceil_lots(minimum_amount_out, base_lot_size)?,
            0,
        )
    };

    let mut data = Vec::with_capacity(PHOENIX_SWAP_DATA_LEN);
    data.push(PHOENIX_SWAP_TAG);
    data.push(PHOENIX_IOC_VARIANT);
    data.push(side);
    data.push(0); // price_in_ticks: None
    data.extend_from_slice(&num_base_lots.to_le_bytes());
    data.extend_from_slice(&num_quote_lots.to_le_bytes());
    data.extend_from_slice(&min_base_lots.to_le_bytes());
    data.extend_from_slice(&min_quote_lots.to_le_bytes());
    data.push(1); // SelfTradeBehavior::CancelProvide
    data.push(1); // match_limit: Some
    data.extend_from_slice(&PHOENIX_DEFAULT_MATCH_LIMIT.to_le_bytes());
    data.extend_from_slice(&0_u128.to_le_bytes()); // client_order_id
    data.push(0); // use_only_deposited_funds: false
    data.push(0); // last_valid_slot: None
    data.push(0); // last_valid_unix_timestamp_in_seconds: None
    require!(
        data.len() == PHOENIX_SWAP_DATA_LEN,
        ArbitrageError::InvalidInstructionData
    );
    Ok(data)
}

fn phoenix_swap_accounts<'info>(
    accounts: &PhoenixSwapAccounts<'info>,
) -> (Vec<AccountMeta>, Vec<AccountInfo<'info>>) {
    (
        vec![
            AccountMeta::new_readonly(accounts.program.key(), false),
            AccountMeta::new_readonly(accounts.log_authority.key(), false),
            AccountMeta::new(accounts.market.key(), false),
            AccountMeta::new_readonly(accounts.payer.key(), true),
            AccountMeta::new(accounts.trader_base.key(), false),
            AccountMeta::new(accounts.trader_quote.key(), false),
            AccountMeta::new(accounts.base_vault.key(), false),
            AccountMeta::new(accounts.quote_vault.key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
        ],
        vec![
            accounts.program.clone(),
            accounts.log_authority.clone(),
            accounts.market.clone(),
            accounts.payer.clone(),
            accounts.trader_base.clone(),
            accounts.trader_quote.clone(),
            accounts.base_vault.clone(),
            accounts.quote_vault.clone(),
            accounts.token_program.clone(),
        ],
    )
}

fn floor_nonzero_lots(atoms: u64, atoms_per_lot: u64) -> Result<u64> {
    let lots = atoms
        .checked_div(atoms_per_lot)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(lots > 0, ArbitrageError::InvalidAmount);
    Ok(lots)
}

fn ceil_lots(atoms: u64, atoms_per_lot: u64) -> Result<u64> {
    atoms
        .checked_add(atoms_per_lot - 1)
        .and_then(|value| value.checked_div(atoms_per_lot))
        .ok_or_else(|| ArbitrageError::MathOverflow.into())
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
    fn swap_data_matches_phoenix_ioc_borsh_layout() {
        let buy = phoenix_swap_data(36, 29, false, 10, 2).expect("buy data");
        assert_eq!(buy.len(), PHOENIX_SWAP_DATA_LEN);
        assert_eq!(&buy[..4], &[0, 2, 0, 0]);
        assert_eq!(&buy[4..12], &0_u64.to_le_bytes());
        assert_eq!(&buy[12..20], &18_u64.to_le_bytes());
        assert_eq!(&buy[20..28], &3_u64.to_le_bytes());
        assert_eq!(&buy[36..46], &[1, 1, 0, 8, 0, 0, 0, 0, 0, 0]);

        let sell = phoenix_swap_data(30, 13, true, 10, 2).expect("sell data");
        assert_eq!(&sell[..4], &[0, 2, 1, 0]);
        assert_eq!(&sell[4..12], &3_u64.to_le_bytes());
        assert_eq!(&sell[28..36], &7_u64.to_le_bytes());
    }

    #[test]
    fn cpi_accounts_match_official_nine_account_order() {
        let token = anchor_spl::token::ID;
        let accounts = PhoenixSwapAccounts {
            program: account(Pubkey::new_unique(), token),
            log_authority: account(Pubkey::new_unique(), token),
            market: account(Pubkey::new_unique(), token),
            payer: account(Pubkey::new_unique(), token),
            trader_base: account(Pubkey::new_unique(), token),
            trader_quote: account(Pubkey::new_unique(), token),
            base_vault: account(Pubkey::new_unique(), token),
            quote_vault: account(Pubkey::new_unique(), token),
            token_program: account(token, token),
        };
        let (metas, infos) = phoenix_swap_accounts(&accounts);
        assert_eq!(metas.len(), 9);
        assert_eq!(infos.len(), 9);
        assert_eq!(metas[0].pubkey, accounts.program.key());
        assert_eq!(metas[3].pubkey, accounts.payer.key());
        assert!(metas[3].is_signer);
        assert_eq!(metas[8].pubkey, token);
    }
}
