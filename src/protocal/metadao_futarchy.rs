use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const METADAO_FUTARCHY_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("FUTARELBfJfQ8RDGhg1wdhddq1odMAJUePHFuBYfUxKq");
pub const METADAO_FUTARCHY_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("DGEympSS4qLvdr9r3uGHTfACdN8snShk4iGdJtZPxuBC");
pub const METADAO_FUTARCHY_STEP_ACCOUNTS: usize = 5;
pub const METADAO_FUTARCHY_SPOT_SWAP_DISCRIMINATOR: [u8; 8] = [167, 97, 12, 231, 237, 78, 166, 251];

const DAO_DISCRIMINATOR: [u8; 8] = [163, 9, 47, 31, 52, 85, 197, 49];
const SPOT_POOL_TAG_OFFSET: usize = 8;
const QUOTE_RESERVE_OFFSET: usize = 109;
const BASE_RESERVE_OFFSET: usize = 117;
const QUOTE_PROTOCOL_FEE_OFFSET: usize = 125;
const BASE_PROTOCOL_FEE_OFFSET: usize = 133;
const TOTAL_LIQUIDITY_OFFSET: usize = 141;
const BASE_MINT_OFFSET: usize = 157;
const QUOTE_MINT_OFFSET: usize = 189;
const BASE_VAULT_OFFSET: usize = 221;
const QUOTE_VAULT_OFFSET: usize = 253;
const NONCE_OFFSET: usize = 285;
const DAO_CREATOR_OFFSET: usize = 293;
const PDA_BUMP_OFFSET: usize = 325;

#[derive(Clone)]
pub struct MetaDaoFutarchyAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub dao: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
}

pub fn metadao_futarchy_swap<'info>(
    accounts: MetaDaoFutarchyAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let output_account = if direction == 0 {
        accounts.user_base
    } else {
        accounts.user_quote
    };
    let pre_out = read_token_amount(output_account)?;
    let metas = vec![
        AccountMeta::new(accounts.dao.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.event_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
    ];
    let infos = vec![
        accounts.dao.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.payer.clone(),
        accounts.token_program.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: spot_swap_data(amount_in, min_amount_out, direction).to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_metadao_futarchy_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == METADAO_FUTARCHY_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[0].key(),
        METADAO_FUTARCHY_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        METADAO_FUTARCHY_EVENT_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *in_mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *out_mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );

    let data = step[1].try_borrow_data()?;
    require!(
        data.starts_with(&DAO_DISCRIMINATOR)
            && data.get(SPOT_POOL_TAG_OFFSET) == Some(&0)
            && data.len() > PDA_BUMP_OFFSET,
        ArbitrageError::InvalidAccount
    );
    let quote_reserve = read_u64(&data, QUOTE_RESERVE_OFFSET)?;
    let base_reserve = read_u64(&data, BASE_RESERVE_OFFSET)?;
    let quote_fee = read_u64(&data, QUOTE_PROTOCOL_FEE_OFFSET)?;
    let base_fee = read_u64(&data, BASE_PROTOCOL_FEE_OFFSET)?;
    require!(
        quote_reserve > 0 && base_reserve > 0 && read_u128(&data, TOTAL_LIQUIDITY_OFFSET)? > 0,
        ArbitrageError::InvalidAccount
    );
    let base_mint = read_pubkey(&data, BASE_MINT_OFFSET)?;
    let quote_mint = read_pubkey(&data, QUOTE_MINT_OFFSET)?;
    require!(base_mint != quote_mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&data, BASE_VAULT_OFFSET)?,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, QUOTE_VAULT_OFFSET)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    let nonce = read_u64(&data, NONCE_OFFSET)?;
    let creator = read_pubkey(&data, DAO_CREATOR_OFFSET)?;
    let nonce_seed = nonce.to_le_bytes();
    let (expected_dao, expected_bump) = Pubkey::find_program_address(
        &[b"dao", creator.as_ref(), &nonce_seed],
        &METADAO_FUTARCHY_PROGRAM_ID,
    );
    require_keys_eq!(step[1].key(), expected_dao, ArbitrageError::InvalidAccount);
    require!(
        data[PDA_BUMP_OFFSET] == expected_bump,
        ArbitrageError::InvalidAccount
    );

    let (expected_in, expected_out) = if direction == 0 {
        (quote_mint, base_mint)
    } else {
        (base_mint, quote_mint)
    };
    require_keys_eq!(in_mint.key(), expected_in, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        out_mint.key(),
        expected_out,
        ArbitrageError::InvalidTokenMint
    );
    validate_vault(
        &step[2],
        base_mint,
        step[1].key(),
        base_reserve
            .checked_add(base_fee)
            .ok_or(ArbitrageError::MathOverflow)?,
        token_program,
    )?;
    validate_vault(
        &step[3],
        quote_mint,
        step[1].key(),
        quote_reserve
            .checked_add(quote_fee)
            .ok_or(ArbitrageError::MathOverflow)?,
        token_program,
    )
}

fn validate_vault<'info>(
    vault: &AccountInfo<'info>,
    expected_mint: Pubkey,
    expected_authority: Pubkey,
    expected_amount: u64,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *vault.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = vault.try_borrow_data()?;
    require_keys_eq!(
        read_pubkey(&data, 0)?,
        expected_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, 32)?,
        expected_authority,
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(&data, 64)? == expected_amount,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let array: &[u8; 32] = bytes
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(*array))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let array: &[u8; 8] = bytes
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(*array))
}

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let end = offset.checked_add(16).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let array: &[u8; 16] = bytes
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u128::from_le_bytes(*array))
}

pub fn spot_swap_data(amount_in: u64, min_amount_out: u64, direction: u8) -> [u8; 25] {
    let mut data = [0u8; 25];
    data[..8].copy_from_slice(&METADAO_FUTARCHY_SPOT_SWAP_DISCRIMINATOR);
    data[8..16].copy_from_slice(&amount_in.to_le_bytes());
    data[16] = direction;
    data[17..25].copy_from_slice(&min_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account_with_data(
        key: Pubkey,
        owner: Pubkey,
        is_writable: bool,
        executable: bool,
        data: Vec<u8>,
    ) -> AccountInfo<'static> {
        AccountInfo::new(
            Box::leak(Box::new(key)),
            false,
            is_writable,
            Box::leak(Box::new(0_u64)),
            Box::leak(data.into_boxed_slice()),
            Box::leak(Box::new(owner)),
            executable,
            0,
        )
    }

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + 32].copy_from_slice(key.as_ref());
    }

    fn token_account_data(mint: Pubkey, authority: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 72];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data
    }

    #[test]
    fn spot_swap_data_matches_official_layout() {
        let data = spot_swap_data(1_000_000, 1_900_000, 0);
        assert_eq!(&data[..8], &METADAO_FUTARCHY_SPOT_SWAP_DISCRIMINATOR);
        assert_eq!(
            u64::from_le_bytes(data[8..16].try_into().unwrap()),
            1_000_000
        );
        assert_eq!(data[16], 0);
        assert_eq!(
            u64::from_le_bytes(data[17..25].try_into().unwrap()),
            1_900_000
        );
    }

    #[test]
    fn semantic_validation_checks_spot_pda_vaults_and_direction() {
        let creator = Pubkey::new_unique();
        let nonce = 7_u64;
        let nonce_seed = nonce.to_le_bytes();
        let (dao, bump) = Pubkey::find_program_address(
            &[b"dao", creator.as_ref(), &nonce_seed],
            &METADAO_FUTARCHY_PROGRAM_ID,
        );
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let base_vault = Pubkey::new_unique();
        let quote_vault = Pubkey::new_unique();
        let base_reserve = 1_000_000_u64;
        let quote_reserve = 2_000_000_u64;
        let base_fee = 10_u64;
        let quote_fee = 20_u64;
        let token_program = anchor_spl::token::ID;
        let mut dao_data = vec![0_u8; PDA_BUMP_OFFSET + 1];
        dao_data[..8].copy_from_slice(&DAO_DISCRIMINATOR);
        dao_data[SPOT_POOL_TAG_OFFSET] = 0;
        dao_data[QUOTE_RESERVE_OFFSET..QUOTE_RESERVE_OFFSET + 8]
            .copy_from_slice(&quote_reserve.to_le_bytes());
        dao_data[BASE_RESERVE_OFFSET..BASE_RESERVE_OFFSET + 8]
            .copy_from_slice(&base_reserve.to_le_bytes());
        dao_data[QUOTE_PROTOCOL_FEE_OFFSET..QUOTE_PROTOCOL_FEE_OFFSET + 8]
            .copy_from_slice(&quote_fee.to_le_bytes());
        dao_data[BASE_PROTOCOL_FEE_OFFSET..BASE_PROTOCOL_FEE_OFFSET + 8]
            .copy_from_slice(&base_fee.to_le_bytes());
        dao_data[TOTAL_LIQUIDITY_OFFSET..TOTAL_LIQUIDITY_OFFSET + 16]
            .copy_from_slice(&100_u128.to_le_bytes());
        write_pubkey(&mut dao_data, BASE_MINT_OFFSET, base_mint);
        write_pubkey(&mut dao_data, QUOTE_MINT_OFFSET, quote_mint);
        write_pubkey(&mut dao_data, BASE_VAULT_OFFSET, base_vault);
        write_pubkey(&mut dao_data, QUOTE_VAULT_OFFSET, quote_vault);
        dao_data[NONCE_OFFSET..NONCE_OFFSET + 8].copy_from_slice(&nonce.to_le_bytes());
        write_pubkey(&mut dao_data, DAO_CREATOR_OFFSET, creator);
        dao_data[PDA_BUMP_OFFSET] = bump;

        let step = vec![
            test_account_with_data(
                METADAO_FUTARCHY_PROGRAM_ID,
                Pubkey::default(),
                false,
                true,
                vec![],
            ),
            test_account_with_data(dao, METADAO_FUTARCHY_PROGRAM_ID, true, false, dao_data),
            test_account_with_data(
                base_vault,
                token_program,
                true,
                false,
                token_account_data(base_mint, dao, base_reserve + base_fee),
            ),
            test_account_with_data(
                quote_vault,
                token_program,
                true,
                false,
                token_account_data(quote_mint, dao, quote_reserve + quote_fee),
            ),
            test_account_with_data(
                METADAO_FUTARCHY_EVENT_AUTHORITY,
                Pubkey::default(),
                false,
                false,
                vec![],
            ),
        ];
        let base = test_account_with_data(base_mint, token_program, false, false, vec![]);
        let quote = test_account_with_data(quote_mint, token_program, false, false, vec![]);
        let token = test_account_with_data(token_program, Pubkey::default(), false, true, vec![]);

        assert!(
            validate_metadao_futarchy_semantic_accounts(&step, 0, &quote, &base, &token).is_ok()
        );
        assert!(
            validate_metadao_futarchy_semantic_accounts(&step, 1, &base, &quote, &token).is_ok()
        );

        step[1].try_borrow_mut_data().unwrap()[SPOT_POOL_TAG_OFFSET] = 1;
        assert!(
            validate_metadao_futarchy_semantic_accounts(&step, 0, &quote, &base, &token).is_err()
        );
    }
}
