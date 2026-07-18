use anchor_lang::prelude::*;

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::SEGA_CPMM_PROGRAM_ID,
        types::{validate_raydium_cpmm_semantic_accounts, SwapResult},
    },
    protocal::raydium_cpmm::{raydium_cpmm_swap, RaydiumCpmmAccounts},
};

pub const SEGA_CPMM_STEP_ACCOUNTS: usize = 7;

const AUTHORITY_SEED: &[u8] = b"vault_and_lp_mint_auth_seed";
const POOL_ACCOUNT_LEN: usize = 637;
const CONFIG_ACCOUNT_LEN: usize = 236;
const POOL_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];
const CONFIG_DISCRIMINATOR: [u8; 8] = [218, 244, 33, 104, 203, 203, 43, 111];
const AUTH_BUMP_OFFSET: usize = 328;
const STATUS_OFFSET: usize = 329;
const OPEN_TIME_OFFSET: usize = 373;
const CONFIG_TRADE_FEE_RATE_OFFSET: usize = 12;
const CONFIG_PROTOCOL_FEE_RATE_OFFSET: usize = 20;
const CONFIG_FUND_FEE_RATE_OFFSET: usize = 28;
const FEE_RATE_DENOMINATOR: u64 = 1_000_000;
const SWAP_STATUS_BIT: u8 = 2;

pub type SegaCpmmAccounts<'info> = RaydiumCpmmAccounts<'info>;

pub fn validate_sega_cpmm_semantic_accounts<'info>(
    step_accounts: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step_accounts.len() == SEGA_CPMM_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let program = &step_accounts[0];
    require_keys_eq!(
        program.key(),
        SEGA_CPMM_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step_accounts[2].owner,
        program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step_accounts[3].owner,
        program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step_accounts[6].owner,
        program.key(),
        ArbitrageError::InvalidAccount
    );

    validate_raydium_cpmm_semantic_accounts(
        step_accounts,
        direction,
        input_mint,
        output_mint,
        input_token_program,
        output_token_program,
    )?;

    let pool_data = step_accounts[3].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_ACCOUNT_LEN && pool_data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let (expected_authority, expected_bump) =
        Pubkey::find_program_address(&[AUTHORITY_SEED], &program.key());
    require_keys_eq!(
        step_accounts[1].key(),
        expected_authority,
        ArbitrageError::InvalidAccount
    );
    require!(
        pool_data[AUTH_BUMP_OFFSET] == expected_bump,
        ArbitrageError::InvalidAccount
    );
    require!(
        pool_data[STATUS_OFFSET] & (1 << SWAP_STATUS_BIT) == 0,
        ArbitrageError::InvalidAccount
    );
    let open_time = read_u64(&pool_data, OPEN_TIME_OFFSET);
    drop(pool_data);

    let unix_timestamp = Clock::get()?.unix_timestamp;
    let current_time =
        u64::try_from(unix_timestamp).map_err(|_| error!(ArbitrageError::InvalidAccount))?;
    require!(open_time <= current_time, ArbitrageError::InvalidAccount);

    let config_data = step_accounts[2].try_borrow_data()?;
    require!(
        config_data.len() == CONFIG_ACCOUNT_LEN && config_data.starts_with(&CONFIG_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let trade_fee_rate = read_u64(&config_data, CONFIG_TRADE_FEE_RATE_OFFSET);
    let protocol_fee_rate = read_u64(&config_data, CONFIG_PROTOCOL_FEE_RATE_OFFSET);
    let fund_fee_rate = read_u64(&config_data, CONFIG_FUND_FEE_RATE_OFFSET);
    require!(
        trade_fee_rate < FEE_RATE_DENOMINATOR
            && protocol_fee_rate <= FEE_RATE_DENOMINATOR
            && fund_fee_rate <= FEE_RATE_DENOMINATOR,
        ArbitrageError::InvalidInstructionData
    );
    let quoted_fee_rate = trade_fee_rate
        .checked_add(99)
        .ok_or(ArbitrageError::InvalidInstructionData)?
        / 100;
    require!(
        quoted_fee_rate == u64::from(fee_rate),
        ArbitrageError::InvalidInstructionData
    );
    Ok(())
}

pub fn sega_cpmm_swap<'info>(
    accounts: SegaCpmmAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    raydium_cpmm_swap(accounts, amount_in, minimum_amount_out)
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}
