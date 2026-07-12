use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const GAVEL_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("srAMMzfVHVAtgSJc8iH6CfKzuWuUTzLHVCE81QU1rgi");
pub const GAVEL_STEP_ACCOUNTS: usize = 5;

const POOL_ACCOUNT_LEN: usize = 624;
const POOL_DISCRIMINATOR: [u8; 8] = [116, 210, 187, 119, 196, 196, 52, 137];
const BASE_DECIMALS_OFFSET: usize = 16;
const BASE_VAULT_BUMP_OFFSET: usize = 20;
const BASE_MINT_OFFSET: usize = 24;
const BASE_VAULT_OFFSET: usize = 56;
const QUOTE_DECIMALS_OFFSET: usize = 88;
const QUOTE_VAULT_BUMP_OFFSET: usize = 92;
const QUOTE_MINT_OFFSET: usize = 96;
const QUOTE_VAULT_OFFSET: usize = 128;
const FEE_BPS_OFFSET: usize = 528;
const PROTOCOL_ALLOCATION_OFFSET: usize = 532;
const TOTAL_LP_SHARES_OFFSET: usize = 560;
const BASE_RESERVES_SNAPSHOT_OFFSET: usize = 576;
const QUOTE_RESERVES_SNAPSHOT_OFFSET: usize = 584;
const BASE_RESERVES_OFFSET: usize = 592;
const QUOTE_RESERVES_OFFSET: usize = 600;

#[derive(Clone)]
pub struct GavelAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub log_authority: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn gavel_swap<'info>(
    accounts: GavelAccounts<'info>,
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
        AccountMeta::new_readonly(accounts.program.key(), false),
        AccountMeta::new_readonly(accounts.log_authority.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let infos = vec![
        accounts.program.clone(),
        accounts.log_authority.clone(),
        accounts.pool.clone(),
        accounts.payer.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.token_program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: exact_in_data(direction, amount_in, min_amount_out).to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_gavel_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == GAVEL_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[0].key(),
        GAVEL_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
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
    let expected_log = Pubkey::find_program_address(&[b"log"], &GAVEL_PROGRAM_ID).0;
    require_keys_eq!(step[1].key(), expected_log, ArbitrageError::InvalidAccount);

    let data = step[2].try_borrow_data()?;
    require!(
        data.len() == POOL_ACCOUNT_LEN && data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let base_decimals = read_u32(&data, BASE_DECIMALS_OFFSET)?;
    let quote_decimals = read_u32(&data, QUOTE_DECIMALS_OFFSET)?;
    require!(
        base_decimals <= u8::MAX.into(),
        ArbitrageError::InvalidAccount
    );
    require!(
        quote_decimals <= u8::MAX.into(),
        ArbitrageError::InvalidAccount
    );
    let base_mint = read_pubkey(&data, BASE_MINT_OFFSET)?;
    let quote_mint = read_pubkey(&data, QUOTE_MINT_OFFSET)?;
    require!(base_mint != quote_mint, ArbitrageError::InvalidAccount);
    let base_vault = read_pubkey(&data, BASE_VAULT_OFFSET)?;
    let quote_vault = read_pubkey(&data, QUOTE_VAULT_OFFSET)?;
    require_keys_eq!(step[3].key(), base_vault, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[4].key(), quote_vault, ArbitrageError::InvalidAccount);

    let (expected_base_vault, base_bump) = Pubkey::find_program_address(
        &[b"vault", step[2].key().as_ref(), base_mint.as_ref()],
        &GAVEL_PROGRAM_ID,
    );
    let (expected_quote_vault, quote_bump) = Pubkey::find_program_address(
        &[b"vault", step[2].key().as_ref(), quote_mint.as_ref()],
        &GAVEL_PROGRAM_ID,
    );
    require_keys_eq!(
        base_vault,
        expected_base_vault,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        quote_vault,
        expected_quote_vault,
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u32(&data, BASE_VAULT_BUMP_OFFSET)? == u32::from(base_bump),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u32(&data, QUOTE_VAULT_BUMP_OFFSET)? == u32::from(quote_bump),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u32(&data, FEE_BPS_OFFSET)? < 10_000,
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u32(&data, PROTOCOL_ALLOCATION_OFFSET)? <= 100,
        ArbitrageError::InvalidAccount
    );
    let base_reserves = read_u64(&data, BASE_RESERVES_OFFSET)?;
    let quote_reserves = read_u64(&data, QUOTE_RESERVES_OFFSET)?;
    require!(
        read_u64(&data, TOTAL_LP_SHARES_OFFSET)? > 0
            && read_u64(&data, BASE_RESERVES_SNAPSHOT_OFFSET)? > 0
            && read_u64(&data, QUOTE_RESERVES_SNAPSHOT_OFFSET)? > 0
            && base_reserves > 0
            && quote_reserves > 0,
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
    validate_vault(&step[3], base_mint, base_reserves, true, token_program)?;
    validate_vault(&step[4], quote_mint, quote_reserves, false, token_program)
}

fn validate_vault<'info>(
    vault: &AccountInfo<'info>,
    expected_mint: Pubkey,
    reserve: u64,
    exact_amount: bool,
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
        vault.key(),
        ArbitrageError::InvalidAccount
    );
    let amount = read_u64(&data, 64)?;
    require!(
        if exact_amount {
            amount == reserve
        } else {
            amount >= reserve
        },
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn exact_in_data(direction: u8, amount_in: u64, min_amount_out: u64) -> [u8; 19] {
    let mut data = [0u8; 19];
    data[0] = 0;
    data[1] = direction;
    data[2] = 0;
    data[3..11].copy_from_slice(&amount_in.to_le_bytes());
    data[11..19].copy_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset.checked_add(4).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
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
    fn exact_in_data_matches_official_borsh_layout() {
        let data = exact_in_data(1, 42, 40);
        assert_eq!(&data[..3], &[0, 1, 0]);
        assert_eq!(u64::from_le_bytes(data[3..11].try_into().unwrap()), 42);
        assert_eq!(u64::from_le_bytes(data[11..19].try_into().unwrap()), 40);
    }

    #[test]
    fn semantic_validation_binds_pool_vaults_reserves_and_direction() {
        let pool = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let (base_vault, base_bump) = Pubkey::find_program_address(
            &[b"vault", pool.as_ref(), base_mint.as_ref()],
            &GAVEL_PROGRAM_ID,
        );
        let (quote_vault, quote_bump) = Pubkey::find_program_address(
            &[b"vault", pool.as_ref(), quote_mint.as_ref()],
            &GAVEL_PROGRAM_ID,
        );
        let base_reserves = 1_000_000_u64;
        let quote_reserves = 2_000_000_u64;
        let token_program = anchor_spl::token::ID;
        let log_authority = Pubkey::find_program_address(&[b"log"], &GAVEL_PROGRAM_ID).0;
        let mut pool_data = vec![0_u8; POOL_ACCOUNT_LEN];
        pool_data[..8].copy_from_slice(&POOL_DISCRIMINATOR);
        pool_data[BASE_DECIMALS_OFFSET..BASE_DECIMALS_OFFSET + 4]
            .copy_from_slice(&6_u32.to_le_bytes());
        pool_data[BASE_VAULT_BUMP_OFFSET..BASE_VAULT_BUMP_OFFSET + 4]
            .copy_from_slice(&u32::from(base_bump).to_le_bytes());
        write_pubkey(&mut pool_data, BASE_MINT_OFFSET, base_mint);
        write_pubkey(&mut pool_data, BASE_VAULT_OFFSET, base_vault);
        pool_data[QUOTE_DECIMALS_OFFSET..QUOTE_DECIMALS_OFFSET + 4]
            .copy_from_slice(&9_u32.to_le_bytes());
        pool_data[QUOTE_VAULT_BUMP_OFFSET..QUOTE_VAULT_BUMP_OFFSET + 4]
            .copy_from_slice(&u32::from(quote_bump).to_le_bytes());
        write_pubkey(&mut pool_data, QUOTE_MINT_OFFSET, quote_mint);
        write_pubkey(&mut pool_data, QUOTE_VAULT_OFFSET, quote_vault);
        pool_data[FEE_BPS_OFFSET..FEE_BPS_OFFSET + 4].copy_from_slice(&30_u32.to_le_bytes());
        pool_data[PROTOCOL_ALLOCATION_OFFSET..PROTOCOL_ALLOCATION_OFFSET + 4]
            .copy_from_slice(&20_u32.to_le_bytes());
        for (offset, value) in [
            (TOTAL_LP_SHARES_OFFSET, 100_u64),
            (BASE_RESERVES_SNAPSHOT_OFFSET, base_reserves),
            (QUOTE_RESERVES_SNAPSHOT_OFFSET, quote_reserves),
            (BASE_RESERVES_OFFSET, base_reserves),
            (QUOTE_RESERVES_OFFSET, quote_reserves),
        ] {
            pool_data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        }

        let step = vec![
            test_account_with_data(GAVEL_PROGRAM_ID, Pubkey::default(), false, true, vec![]),
            test_account_with_data(log_authority, Pubkey::default(), false, false, vec![]),
            test_account_with_data(pool, GAVEL_PROGRAM_ID, true, false, pool_data),
            test_account_with_data(
                base_vault,
                token_program,
                true,
                false,
                token_account_data(base_mint, base_vault, base_reserves),
            ),
            test_account_with_data(
                quote_vault,
                token_program,
                true,
                false,
                token_account_data(quote_mint, quote_vault, quote_reserves + 50),
            ),
        ];
        let base = test_account_with_data(base_mint, token_program, false, false, vec![]);
        let quote = test_account_with_data(quote_mint, token_program, false, false, vec![]);
        let token = test_account_with_data(token_program, Pubkey::default(), false, true, vec![]);

        assert!(validate_gavel_semantic_accounts(&step, 0, &quote, &base, &token).is_ok());
        assert!(validate_gavel_semantic_accounts(&step, 1, &base, &quote, &token).is_ok());

        step[3].try_borrow_mut_data().unwrap()[64..72]
            .copy_from_slice(&(base_reserves - 1).to_le_bytes());
        assert!(validate_gavel_semantic_accounts(&step, 0, &quote, &base, &token).is_err());
    }
}
