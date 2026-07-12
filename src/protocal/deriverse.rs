use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{read_token_amount, token_balance_delta, SwapResult},
};

#[cfg(any(not(feature = "devnet"), test))]
use crate::instructions::program_ids::DERIVERSE_PROGRAM_ID;
#[cfg(feature = "devnet")]
use crate::instructions::program_ids::DERIVERSE_PROGRAM_ID_DEVNET;

pub const DERIVERSE_STEP_ACCOUNTS: usize = 13;
const VERSION: u32 = 1;
const AUTHORITY_SEED: &[u8] = b"ndxnt";
const TOKEN_TAG: u32 = 4;
const INSTRUMENT_TAG: u32 = 7;
const MAPS_TAG: u32 = 10;
const CLIENT_INFOS_TAG: u32 = 12;
const BIDS_TREE_TAG: u32 = 14;
const ASKS_TREE_TAG: u32 = 15;
const BID_ORDERS_TAG: u32 = 16;
const ASK_ORDERS_TAG: u32 = 17;
const LINES_TAG: u32 = 18;
const SWAP_TAG: u8 = 26;
const TOKEN_STATE_LEN: usize = 88;
const INSTRUMENT_HEADER_LEN: usize = 1_064;
const SPOT_HEADER_LEN: usize = 24;
const SUSPENDED_FLAG: u32 = 0x20;
const EXPANDABLE_CANDLES_FLAG: u32 = 0x100;

pub struct DeriverseAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub asset_mint: &'info AccountInfo<'info>,
    pub currency_mint: &'info AccountInfo<'info>,
    pub asset_vault: &'info AccountInfo<'info>,
    pub currency_vault: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub side_tree: &'info AccountInfo<'info>,
    pub side_orders: &'info AccountInfo<'info>,
    pub lines: &'info AccountInfo<'info>,
    pub maps: &'info AccountInfo<'info>,
    pub client_infos: &'info AccountInfo<'info>,
    pub owner_asset: &'info AccountInfo<'info>,
    pub owner_currency: &'info AccountInfo<'info>,
    pub asset_token_program: &'info AccountInfo<'info>,
    pub currency_token_program: &'info AccountInfo<'info>,
}

pub fn deriverse_swap<'info>(
    accounts: DeriverseAccounts<'info>,
    instrument_id: u32,
    amount_in: u64,
    minimum_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidPath);
    let amount = i64::try_from(amount_in).map_err(|_| ArbitrageError::InvalidAmount)?;
    let minimum_amount_out =
        i64::try_from(minimum_amount_out).map_err(|_| ArbitrageError::InvalidAmount)?;
    let output_account = if direction == 0 {
        accounts.owner_currency
    } else {
        accounts.owner_asset
    };
    let pre_out = read_token_amount(output_account)?;

    let metas = vec![
        AccountMeta::new(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.asset_mint.key(), false),
        AccountMeta::new_readonly(accounts.currency_mint.key(), false),
        AccountMeta::new(accounts.asset_vault.key(), false),
        AccountMeta::new(accounts.currency_vault.key(), false),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.side_tree.key(), false),
        AccountMeta::new(accounts.side_orders.key(), false),
        AccountMeta::new(accounts.lines.key(), false),
        AccountMeta::new(accounts.maps.key(), false),
        AccountMeta::new(accounts.client_infos.key(), false),
        AccountMeta::new(accounts.owner_asset.key(), false),
        AccountMeta::new(accounts.owner_currency.key(), false),
        AccountMeta::new_readonly(accounts.asset_token_program.key(), false),
        AccountMeta::new_readonly(accounts.currency_token_program.key(), false),
    ];
    let account_infos = vec![
        accounts.payer.clone(),
        accounts.asset_mint.clone(),
        accounts.currency_mint.clone(),
        accounts.asset_vault.clone(),
        accounts.currency_vault.clone(),
        accounts.market.clone(),
        accounts.side_tree.clone(),
        accounts.side_orders.clone(),
        accounts.lines.clone(),
        accounts.maps.clone(),
        accounts.client_infos.clone(),
        accounts.owner_asset.clone(),
        accounts.owner_currency.clone(),
        accounts.asset_token_program.clone(),
        accounts.currency_token_program.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(32);
    data.push(SWAP_TAG);
    data.push(direction);
    data.extend_from_slice(&0_u16.to_le_bytes());
    data.extend_from_slice(&instrument_id.to_le_bytes());
    data.extend_from_slice(&0_i64.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
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

pub fn validate_deriverse_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<u32> {
    require!(
        step.len() == DERIVERSE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidPath);
    let program_id = deriverse_program_id();
    require_keys_eq!(step[0].key(), program_id, ArbitrageError::InvalidAccount);
    require_keys_eq!(*step[5].owner, program_id, ArbitrageError::InvalidAccount);

    let market = step[5].try_borrow_data()?;
    validate_discriminator(&market, INSTRUMENT_TAG, INSTRUMENT_HEADER_LEN)?;
    let instrument_id = u32_at(&market, 8)?;
    let asset_token_id = u32_at(&market, 12)?;
    let currency_token_id = u32_at(&market, 16)?;
    let asset_decimals =
        u8::try_from(u32_at(&market, 20)?).map_err(|_| ArbitrageError::InvalidAccount)?;
    let currency_decimals =
        u8::try_from(u32_at(&market, 24)?).map_err(|_| ArbitrageError::InvalidAccount)?;
    let mask = u32_at(&market, 28)?;
    let last_price = i64_at(&market, 32)?;
    let best_bid = i64_at(&market, 48)?;
    let best_ask = i64_at(&market, 56)?;
    let asset_tokens = i64_at(&market, 144)?;
    let currency_tokens = i64_at(&market, 152)?;
    let pool_supply = i64_at(&market, 160)?;
    let maps = pubkey_at(&market, 352)?;
    let asset_mint = pubkey_at(&market, 416)?;
    let currency_mint = pubkey_at(&market, 448)?;
    let decimal_factor = i64_at(&market, 800)?;
    require!(
        asset_mint != currency_mint
            && (last_price > 0 || best_bid > 0 || best_ask > 0)
            && asset_tokens > 0
            && currency_tokens > 0
            && pool_supply > 0
            && decimal_factor > 0
            && mask & (SUSPENDED_FLAG | EXPANDABLE_CANDLES_FLAG) == 0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        spot_address(
            INSTRUMENT_TAG,
            asset_token_id,
            currency_token_id,
            &program_id
        ),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        maps,
        spot_address(MAPS_TAG, asset_token_id, currency_token_id, &program_id),
        ArbitrageError::InvalidAccount
    );
    drop(market);

    let (expected_input_mint, expected_output_mint) = if direction == 0 {
        (asset_mint, currency_mint)
    } else {
        (currency_mint, asset_mint)
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output_mint,
        ArbitrageError::InvalidAccount
    );
    let (asset_mint_info, currency_mint_info, asset_program, currency_program) = if direction == 0 {
        (
            input_mint,
            output_mint,
            input_token_program,
            output_token_program,
        )
    } else {
        (
            output_mint,
            input_mint,
            output_token_program,
            input_token_program,
        )
    };
    require_keys_eq!(
        step[11].key(),
        asset_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[12].key(),
        currency_program.key(),
        ArbitrageError::InvalidAccount
    );
    validate_mint_decimals(asset_mint_info, asset_decimals)?;
    validate_mint_decimals(currency_mint_info, currency_decimals)?;

    validate_token_state(
        &step[1],
        asset_mint,
        step[3].key(),
        asset_token_id,
        asset_decimals,
        &program_id,
    )?;
    validate_token_state(
        &step[2],
        currency_mint,
        step[4].key(),
        currency_token_id,
        currency_decimals,
        &program_id,
    )?;
    let (tree_tag, order_tag) = if direction == 0 {
        (BIDS_TREE_TAG, BID_ORDERS_TAG)
    } else {
        (ASKS_TREE_TAG, ASK_ORDERS_TAG)
    };
    for (index, tag) in [
        (6, tree_tag),
        (7, order_tag),
        (8, LINES_TAG),
        (9, MAPS_TAG),
        (10, CLIENT_INFOS_TAG),
    ] {
        validate_spot_account(
            &step[index],
            tag,
            instrument_id,
            asset_token_id,
            currency_token_id,
            &program_id,
        )?;
    }
    require_keys_eq!(step[9].key(), maps, ArbitrageError::InvalidAccount);
    Ok(instrument_id)
}

fn validate_token_state(
    account: &AccountInfo,
    mint: Pubkey,
    vault: Pubkey,
    token_id: u32,
    decimals: u8,
    program_id: &Pubkey,
) -> Result<()> {
    require_keys_eq!(*account.owner, *program_id, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        account.key(),
        token_state_address(&mint, program_id),
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    validate_discriminator(&data, TOKEN_TAG, TOKEN_STATE_LEN)?;
    require_keys_eq!(pubkey_at(&data, 8)?, mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(pubkey_at(&data, 40)?, vault, ArbitrageError::InvalidAccount);
    require!(
        u32_at(&data, 72)? == token_id,
        ArbitrageError::InvalidAccount
    );
    require!(
        u32_at(&data, 76)? & 0xff == u32::from(decimals),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_spot_account(
    account: &AccountInfo,
    tag: u32,
    instrument_id: u32,
    asset_token_id: u32,
    currency_token_id: u32,
    program_id: &Pubkey,
) -> Result<()> {
    require_keys_eq!(*account.owner, *program_id, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        account.key(),
        spot_address(tag, asset_token_id, currency_token_id, program_id),
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    validate_discriminator(&data, tag, SPOT_HEADER_LEN)?;
    require!(
        u32_at(&data, 8)? == instrument_id
            && u32_at(&data, 16)? == asset_token_id
            && u32_at(&data, 20)? == currency_token_id,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_mint_decimals(mint: &AccountInfo, expected: u8) -> Result<()> {
    let data = mint.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[44] == expected,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_discriminator(data: &[u8], tag: u32, min_len: usize) -> Result<()> {
    require!(
        data.len() >= min_len && u32_at(data, 0)? == tag && u32_at(data, 4)? == VERSION,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn authority(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[AUTHORITY_SEED], program_id).0
}

fn token_state_address(mint: &Pubkey, program_id: &Pubkey) -> Pubkey {
    let mint_bytes = mint.to_bytes();
    let mut seed = [0_u8; 32];
    seed[..28].copy_from_slice(&mint_bytes[..28]);
    seed[28..].copy_from_slice(&VERSION.to_le_bytes());
    Pubkey::find_program_address(&[&seed, authority(program_id).as_ref()], program_id).0
}

fn spot_address(tag: u32, asset_id: u32, currency_id: u32, program_id: &Pubkey) -> Pubkey {
    let mut seed = [0_u8; 16];
    seed[..4].copy_from_slice(&VERSION.to_le_bytes());
    seed[4..8].copy_from_slice(&tag.to_le_bytes());
    seed[8..12].copy_from_slice(&asset_id.to_le_bytes());
    seed[12..].copy_from_slice(&currency_id.to_le_bytes());
    Pubkey::find_program_address(&[&seed, authority(program_id).as_ref()], program_id).0
}

fn deriverse_program_id() -> Pubkey {
    #[cfg(feature = "devnet")]
    {
        DERIVERSE_PROGRAM_ID_DEVNET
    }
    #[cfg(not(feature = "devnet"))]
    {
        DERIVERSE_PROGRAM_ID
    }
}

fn pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes: [u8; 32] = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset.checked_add(4).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(bytes))
}

fn i64_at(data: &[u8], offset: usize) -> Result<i64> {
    let bytes: [u8; 8] = data
        .get(offset..offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(i64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_pinned_mainnet_authority_and_sol_usdc_market() {
        let program = DERIVERSE_PROGRAM_ID;
        assert_eq!(
            authority(&program),
            anchor_lang::pubkey!("5QLXhNtkVUEtCyzZ6C1iAavT2x6c6t6LdJcSwVnqWEii")
        );
        assert_eq!(
            spot_address(INSTRUMENT_TAG, 2, 1, &program),
            anchor_lang::pubkey!("8Wk2L1yDovBJifCN1o86X7g7pDcqLau39m6tEsJ9Sheh")
        );
    }

    #[test]
    fn swap_payload_matches_observed_32_byte_layout() {
        let mut data = Vec::with_capacity(32);
        data.push(SWAP_TAG);
        data.push(1);
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&0_i64.to_le_bytes());
        data.extend_from_slice(&20_731_167_i64.to_le_bytes());
        data.extend_from_slice(&0_i64.to_le_bytes());
        assert_eq!(data.len(), 32);
        assert_eq!(data, hex_literal_observed_payload());
    }

    fn hex_literal_observed_payload() -> Vec<u8> {
        vec![
            0x1a, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x1f, 0x55, 0x3c, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ]
    }
}
