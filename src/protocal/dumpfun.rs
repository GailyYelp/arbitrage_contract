use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
    system_instruction,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::DUMPFUN_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
    protocal::wsol::{close_wsol_for_native, restore_wsol_after_native, WsolBridgeAccounts},
};

pub const DUMPFUN_STEP_ACCOUNTS: usize = 11;
pub const DUMPFUN_BUY_DISCRIMINATOR: [u8; 8] = [107, 198, 250, 226, 105, 48, 89, 135];
pub const DUMPFUN_SELL_DISCRIMINATOR: [u8; 8] = [34, 198, 103, 105, 28, 0, 138, 92];
const DUMPFUN_POOL_DISCRIMINATOR: [u8; 8] = [66, 38, 17, 64, 188, 80, 68, 129];
const DUMPFUN_POOL_ACCOUNT_LEN: usize = 254;
const DUMPFUN_WSOL_MINT: Pubkey =
    anchor_lang::pubkey!("So11111111111111111111111111111111111111112");

const CREATOR_OFFSET: usize = 8;
const PLATFORM_FEE_OFFSET: usize = 40;
const COMPANY_TAX_OFFSET: usize = 72;
const MINT_OFFSET: usize = 104;
const SELL_LOCK_PERIOD_OFFSET: usize = 136;
const VIRTUAL_SOL_RESERVE_OFFSET: usize = 144;
const REAL_TOKEN_RESERVE_OFFSET: usize = 160;
const CREATE_TIME_OFFSET: usize = 168;
const RAMPING_LIMIT_COUNT_OFFSET: usize = 176;
const RAMPING_LIMITS_OFFSET: usize = 180;
const RAMPING_LIMIT_LEN: usize = 18;
const MAX_RAMPING_LIMITS: usize = 4;

pub struct DumpFunAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_dumpfun_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == DUMPFUN_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        DUMPFUN_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[7].key(),
        token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[8].key(),
        anchor_spl::associated_token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[9].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[10].key(),
        anchor_lang::solana_program::sysvar::rent::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        DUMPFUN_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidAccount
    );
    require!(
        step[2].try_borrow_data()?.is_empty(),
        ArbitrageError::InvalidAccount
    );

    let pool_data = step[1].try_borrow_data()?;
    require!(
        pool_data.len() == DUMPFUN_POOL_ACCOUNT_LEN && pool_data[..8] == DUMPFUN_POOL_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    let creator = read_pubkey(&pool_data, CREATOR_OFFSET)?;
    let platform_fee = read_pubkey(&pool_data, PLATFORM_FEE_OFFSET)?;
    let company_tax = read_pubkey(&pool_data, COMPANY_TAX_OFFSET)?;
    let mint = read_pubkey(&pool_data, MINT_OFFSET)?;
    let sell_lock_period = read_i64(&pool_data, SELL_LOCK_PERIOD_OFFSET)?;
    let virtual_sol_reserve = read_u64(&pool_data, VIRTUAL_SOL_RESERVE_OFFSET)?;
    let real_token_reserve = read_u64(&pool_data, REAL_TOKEN_RESERVE_OFFSET)?;
    let create_time = read_i64(&pool_data, CREATE_TIME_OFFSET)?;
    let limit_count = usize::try_from(read_u32(&pool_data, RAMPING_LIMIT_COUNT_OFFSET)?)
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    require!(
        limit_count <= MAX_RAMPING_LIMITS,
        ArbitrageError::InvalidAccount
    );
    validate_ramping_limits(&pool_data, limit_count)?;
    let flags_offset = RAMPING_LIMITS_OFFSET
        .checked_add(
            limit_count
                .checked_mul(RAMPING_LIMIT_LEN)
                .ok_or(ArbitrageError::MathOverflow)?,
        )
        .ok_or(ArbitrageError::MathOverflow)?;
    let in_use = read_bool(&pool_data, flags_offset)?;
    let closed = read_bool(
        &pool_data,
        flags_offset
            .checked_add(1)
            .ok_or(ArbitrageError::MathOverflow)?,
    )?;
    require!(
        creator != Pubkey::default()
            && platform_fee != Pubkey::default()
            && company_tax != Pubkey::default()
            && mint != Pubkey::default()
            && sell_lock_period >= 0
            && create_time > 0
            && virtual_sol_reserve > 0
            && real_token_reserve > 0
            && !in_use
            && !closed,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[4].key(), platform_fee, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[5].key(), company_tax, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[6].key(), creator, ArbitrageError::InvalidAccount);

    let expected_pool =
        Pubkey::find_program_address(&[mint.as_ref(), b"liquidity"], &DUMPFUN_PROGRAM_ID).0;
    require_keys_eq!(step[1].key(), expected_pool, ArbitrageError::InvalidAccount);
    let pool_key = step[1].key();
    let expected_authority = Pubkey::find_program_address(
        &[pool_key.as_ref(), mint.as_ref(), b"authority"],
        &DUMPFUN_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(
        step[2].key(),
        expected_authority,
        ArbitrageError::InvalidAccount
    );
    let expected_vault = associated_token_address(expected_authority, mint, token_program.key());
    require_keys_eq!(
        step[3].key(),
        expected_vault,
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output) = if direction == 0 {
        (DUMPFUN_WSOL_MINT, mint)
    } else {
        (mint, DUMPFUN_WSOL_MINT)
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output,
        ArbitrageError::InvalidTokenMint
    );
    let token_mint_ai = if direction == 0 {
        output_mint
    } else {
        input_mint
    };
    validate_token_account_for_mint_and_authority(
        &step[3],
        token_mint_ai,
        token_program,
        &step[2],
    )?;
    require!(
        read_token_amount(&step[3])? == real_token_reserve,
        ArbitrageError::InvalidAccount
    );
    require!(
        step[2].lamports() > 0,
        ArbitrageError::InsufficientLiquidity
    );
    if direction == 1 {
        let unlock_time = create_time
            .checked_add(sell_lock_period)
            .ok_or(ArbitrageError::MathOverflow)?;
        require!(
            Clock::get()?.unix_timestamp >= unlock_time,
            ArbitrageError::InvalidAccount
        );
    }
    Ok(())
}

pub fn dumpfun_swap<'a, 'info>(
    accounts: DumpFunAccounts<'a, 'info>,
    amount: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(
        amount > 0 && min_amount_out > 0,
        ArbitrageError::InvalidAmount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let pre_out = read_token_amount(accounts.user_output)?;
    if direction == 0 {
        let bridge = WsolBridgeAccounts {
            payer: accounts.payer,
            user_wsol: accounts.user_input,
            wsol_mint: accounts.input_mint,
            token_program: &accounts.step[7],
            associated_token_program: &accounts.step[8],
            system_program: &accounts.step[9],
        };
        let remaining_wsol = close_wsol_for_native(bridge, amount)?;
        invoke_dumpfun(&accounts, amount, min_amount_out, true)?;
        restore_wsol_after_native(bridge, remaining_wsol)?;
    } else {
        let pre_lamports = accounts.payer.lamports();
        invoke_dumpfun(&accounts, amount, min_amount_out, false)?;
        let received = accounts
            .payer
            .lamports()
            .checked_sub(pre_lamports)
            .ok_or(ArbitrageError::MathOverflow)?;
        require!(
            received >= min_amount_out,
            ArbitrageError::InsufficientOutputAmount
        );
        invoke(
            &system_instruction::transfer(accounts.payer.key, accounts.user_output.key, received),
            &[
                accounts.payer.clone(),
                accounts.user_output.clone(),
                accounts.step[9].clone(),
            ],
        )?;
        invoke(
            &anchor_spl::token::spl_token::instruction::sync_native(
                accounts.step[7].key,
                accounts.user_output.key,
            )?,
            &[accounts.user_output.clone(), accounts.step[7].clone()],
        )?;
    }
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

fn invoke_dumpfun<'a, 'info>(
    accounts: &DumpFunAccounts<'a, 'info>,
    amount: u64,
    min_amount_out: u64,
    buy: bool,
) -> Result<()> {
    let (data, metas, account_infos) = if buy {
        (
            trade_data(DUMPFUN_BUY_DISCRIMINATOR, amount, min_amount_out),
            vec![
                AccountMeta::new(accounts.payer.key(), true),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new(accounts.output_mint.key(), false),
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new(accounts.step[3].key(), false),
                AccountMeta::new(accounts.user_output.key(), false),
                AccountMeta::new_readonly(accounts.step[7].key(), false),
                AccountMeta::new_readonly(accounts.step[8].key(), false),
                AccountMeta::new_readonly(accounts.step[9].key(), false),
                AccountMeta::new_readonly(accounts.step[10].key(), false),
            ],
            vec![
                accounts.payer.clone(),
                accounts.step[4].clone(),
                accounts.output_mint.clone(),
                accounts.step[1].clone(),
                accounts.step[2].clone(),
                accounts.step[3].clone(),
                accounts.user_output.clone(),
                accounts.step[7].clone(),
                accounts.step[8].clone(),
                accounts.step[9].clone(),
                accounts.step[10].clone(),
                accounts.step[0].clone(),
            ],
        )
    } else {
        (
            trade_data(DUMPFUN_SELL_DISCRIMINATOR, amount, min_amount_out),
            vec![
                AccountMeta::new(accounts.payer.key(), true),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new(accounts.step[5].key(), false),
                AccountMeta::new(accounts.step[6].key(), false),
                AccountMeta::new(accounts.input_mint.key(), false),
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new(accounts.step[3].key(), false),
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new_readonly(accounts.step[7].key(), false),
                AccountMeta::new_readonly(accounts.step[8].key(), false),
                AccountMeta::new_readonly(accounts.step[9].key(), false),
                AccountMeta::new_readonly(accounts.step[10].key(), false),
            ],
            vec![
                accounts.payer.clone(),
                accounts.step[4].clone(),
                accounts.step[5].clone(),
                accounts.step[6].clone(),
                accounts.input_mint.clone(),
                accounts.step[1].clone(),
                accounts.step[2].clone(),
                accounts.step[3].clone(),
                accounts.user_input.clone(),
                accounts.step[7].clone(),
                accounts.step[8].clone(),
                accounts.step[9].clone(),
                accounts.step[10].clone(),
                accounts.step[0].clone(),
            ],
        )
    };
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: metas,
            data,
        },
        &account_infos,
    )?;
    Ok(())
}

fn trade_data(discriminator: [u8; 8], amount: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn validate_ramping_limits(data: &[u8], count: usize) -> Result<()> {
    let mut previous_end = 0_i64;
    for index in 0..count {
        let offset = RAMPING_LIMITS_OFFSET
            .checked_add(
                index
                    .checked_mul(RAMPING_LIMIT_LEN)
                    .ok_or(ArbitrageError::MathOverflow)?,
            )
            .ok_or(ArbitrageError::MathOverflow)?;
        let start = read_i64(data, offset)?;
        let end = read_i64(
            data,
            offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?,
        )?;
        let bps = read_u16(
            data,
            offset.checked_add(16).ok_or(ArbitrageError::MathOverflow)?,
        )?;
        require!(
            start >= 0 && end > start && bps > 0 && bps <= 10_000 && start >= previous_end,
            ArbitrageError::InvalidAccount
        );
        previous_end = end;
    }
    Ok(())
}

fn associated_token_address(owner: Pubkey, mint: Pubkey, token_program: Pubkey) -> Pubkey {
    anchor_spl::associated_token::get_associated_token_address_with_program_id(
        &owner,
        &mint,
        &token_program,
    )
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

fn read_i64(data: &[u8], offset: usize) -> Result<i64> {
    let bytes = data
        .get(offset..offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(i64::from_le_bytes(
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

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset.checked_add(2).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u16::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_bool(data: &[u8], offset: usize) -> Result<bool> {
    match data.get(offset).copied() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => err!(ArbitrageError::InvalidAccount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_input_payloads_match_deployed_idl() {
        let buy = trade_data(DUMPFUN_BUY_DISCRIMINATOR, 100_000_000, 17_000_000_000_000);
        assert_eq!(buy.len(), 24);
        assert_eq!(&buy[..8], &DUMPFUN_BUY_DISCRIMINATOR);
        assert_eq!(&buy[8..16], &100_000_000_u64.to_le_bytes());
        let sell = trade_data(DUMPFUN_SELL_DISCRIMINATOR, 63_242_919_358_124, 647_000_000);
        assert_eq!(&sell[..8], &DUMPFUN_SELL_DISCRIMINATOR);
        assert_eq!(&sell[16..24], &647_000_000_u64.to_le_bytes());
    }
}
