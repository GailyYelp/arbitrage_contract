use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
    sysvar,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::LIMO_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const LIMO_STEP_ACCOUNTS: usize = 15;
pub const LIMO_EXPRESS_RELAY_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("PytERJFhAKuNNuaiXkApLfWzwNwSNDACpigT3LwQfou");

const TAKE_ORDER_DISCRIMINATOR: [u8; 8] = [163, 208, 20, 172, 223, 65, 255, 228];
const ORDER_DISCRIMINATOR: [u8; 8] = [134, 173, 223, 185, 77, 86, 28, 51];
const GLOBAL_DISCRIMINATOR: [u8; 8] = [149, 8, 156, 202, 160, 252, 176, 217];
const ORDER_ACCOUNT_LEN: usize = 424;
const GLOBAL_ACCOUNT_LEN: usize = 2_168;
const WSOL_MINT: Pubkey = anchor_lang::pubkey!("So11111111111111111111111111111111111111112");

#[derive(Clone)]
pub struct LimoAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub input_token_program: &'a AccountInfo<'info>,
    pub output_token_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_limo_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<(u64, u64, u64)> {
    require!(
        step.len() == LIMO_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(
        direction == 0 && fee_rate == 0,
        ArbitrageError::InvalidInstructionData
    );
    require_keys_eq!(
        step[0].key(),
        LIMO_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[4].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );

    let order = step[4].try_borrow_data()?;
    require!(
        order.len() == ORDER_ACCOUNT_LEN && order.starts_with(&ORDER_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&order, 8)?,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&order, 40)?,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&order, 72)?,
        output_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&order, 104)?,
        output_token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        read_pubkey(&order, 136)?,
        input_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&order, 168)?,
        input_token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    let initial_input_amount = read_u64(&order, 200)?;
    let expected_output_amount = read_u64(&order, 208)?;
    let remaining_input_amount = read_u64(&order, 216)?;
    let input_vault_bump = read_u8(&order, 250)?;
    require!(
        initial_input_amount > 0
            && expected_output_amount > 0
            && remaining_input_amount > 0
            && remaining_input_amount <= initial_input_amount
            && read_u8(&order, 248)? == 0
            && read_u8(&order, 249)? == 0
            && read_u8(&order, 251)? == 0
            && read_u8(&order, 252)? == 1
            && read_pubkey(&order, 272)? == Pubkey::default(),
        ArbitrageError::InvalidAccount
    );
    drop(order);

    let global = step[2].try_borrow_data()?;
    require!(
        global.len() == GLOBAL_ACCOUNT_LEN && global.starts_with(&GLOBAL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u8(&global, 8)? == 0 && read_u8(&global, 11)? == 0,
        ArbitrageError::InvalidAccount
    );
    let authority =
        Pubkey::find_program_address(&[b"authority", step[2].key().as_ref()], &step[0].key()).0;
    require_keys_eq!(step[3].key(), authority, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&global, 120)?,
        authority,
        ArbitrageError::InvalidAccount
    );
    drop(global);

    let expected_vault = Pubkey::find_program_address(
        &[
            b"escrow_vault",
            step[2].key().as_ref(),
            output_mint.key().as_ref(),
        ],
        &step[0].key(),
    );
    require!(
        input_vault_bump == expected_vault.1,
        ArbitrageError::InvalidAccount
    );
    let expected_vault = expected_vault.0;
    require_keys_eq!(
        step[5].key(),
        expected_vault,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(
        &step[5],
        output_mint,
        output_token_program,
        &step[3],
    )?;
    require!(
        read_token_amount(&step[5])? >= remaining_input_amount,
        ArbitrageError::InvalidAccount
    );

    if input_mint.key() == WSOL_MINT {
        let intermediary = Pubkey::find_program_address(
            &[b"intermediary", step[4].key().as_ref()],
            &step[0].key(),
        )
        .0;
        require_keys_eq!(step[6].key(), intermediary, ArbitrageError::InvalidAccount);
        require_keys_eq!(step[7].key(), step[0].key(), ArbitrageError::InvalidAccount);
    } else {
        require_keys_eq!(step[6].key(), step[0].key(), ArbitrageError::InvalidAccount);
        let maker_output =
            anchor_spl::associated_token::get_associated_token_address_with_program_id(
                &step[1].key(),
                &input_mint.key(),
                &input_token_program.key(),
            );
        require_keys_eq!(step[7].key(), maker_output, ArbitrageError::InvalidAccount);
        validate_token_account_for_mint_and_authority(
            &step[7],
            input_mint,
            input_token_program,
            &step[1],
        )?;
    }

    require_keys_eq!(
        step[8].key(),
        LIMO_EXPRESS_RELAY_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    let relay_metadata =
        Pubkey::find_program_address(&[b"metadata"], &LIMO_EXPRESS_RELAY_PROGRAM_ID).0;
    require_keys_eq!(
        step[9].key(),
        relay_metadata,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[9].owner,
        LIMO_EXPRESS_RELAY_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[10].key(),
        sysvar::instructions::ID,
        ArbitrageError::InvalidAccount
    );
    let config_router = Pubkey::find_program_address(
        &[b"config_router", authority.as_ref()],
        &LIMO_EXPRESS_RELAY_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(
        step[11].key(),
        config_router,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[12].key(),
        sysvar::rent::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[13].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    let event_authority = Pubkey::find_program_address(&[b"__event_authority"], &step[0].key()).0;
    require_keys_eq!(
        step[14].key(),
        event_authority,
        ArbitrageError::InvalidAccount
    );

    Ok((
        initial_input_amount,
        expected_output_amount,
        remaining_input_amount,
    ))
}

pub fn limo_take_order<'a, 'info>(
    accounts: LimoAccounts<'a, 'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    initial_input_amount: u64,
    expected_output_amount: u64,
    remaining_input_amount: u64,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let amount_out_u128 = u128::from(amount_in)
        .checked_mul(u128::from(initial_input_amount))
        .ok_or(ArbitrageError::MathOverflow)?
        .checked_div(u128::from(expected_output_amount))
        .ok_or(ArbitrageError::MathOverflow)?;
    let amount_out = u64::try_from(amount_out_u128).map_err(|_| ArbitrageError::MathOverflow)?;
    require!(
        amount_out > 0 && amount_out <= remaining_input_amount,
        ArbitrageError::InvalidAmount
    );
    let required_payment = ceil_mul_div(amount_out, expected_output_amount, initial_input_amount)?;
    require!(required_payment <= amount_in, ArbitrageError::MathOverflow);

    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(&TAKE_ORDER_DISCRIMINATOR);
    data.extend_from_slice(&amount_out.to_le_bytes());
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
    let program = &accounts.step[0];
    let optional_intermediary = &accounts.step[6];
    let optional_maker_output = &accounts.step[7];
    let instruction = Instruction {
        program_id: program.key(),
        accounts: vec![
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new(accounts.step[1].key(), false),
            AccountMeta::new(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(accounts.step[4].key(), false),
            AccountMeta::new_readonly(accounts.output_mint.key(), false),
            AccountMeta::new_readonly(accounts.input_mint.key(), false),
            AccountMeta::new(accounts.step[5].key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new(accounts.user_input.key(), false),
            optional_meta(optional_intermediary, program.key()),
            optional_meta(optional_maker_output, program.key()),
            AccountMeta::new_readonly(accounts.step[8].key(), false),
            AccountMeta::new_readonly(accounts.step[9].key(), false),
            AccountMeta::new_readonly(accounts.step[10].key(), false),
            AccountMeta::new_readonly(program.key(), false),
            AccountMeta::new_readonly(accounts.step[11].key(), false),
            AccountMeta::new_readonly(accounts.output_token_program.key(), false),
            AccountMeta::new_readonly(accounts.input_token_program.key(), false),
            AccountMeta::new_readonly(accounts.step[12].key(), false),
            AccountMeta::new_readonly(accounts.step[13].key(), false),
            AccountMeta::new_readonly(accounts.step[14].key(), false),
            AccountMeta::new_readonly(program.key(), false),
        ],
        data,
    };
    let pre_output = read_token_amount(accounts.user_output)?;
    let infos = vec![
        accounts.payer.clone(),
        accounts.step[1].clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.step[4].clone(),
        accounts.output_mint.clone(),
        accounts.input_mint.clone(),
        accounts.step[5].clone(),
        accounts.user_output.clone(),
        accounts.user_input.clone(),
        accounts.step[6].clone(),
        accounts.step[7].clone(),
        accounts.step[8].clone(),
        accounts.step[9].clone(),
        accounts.step[10].clone(),
        accounts.step[11].clone(),
        accounts.output_token_program.clone(),
        accounts.input_token_program.clone(),
        accounts.step[12].clone(),
        accounts.step[13].clone(),
        accounts.step[14].clone(),
        accounts.step[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    let actual_amount_out = token_balance_delta(accounts.user_output, pre_output)?;
    require!(
        actual_amount_out >= minimum_amount_out,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(SwapResult {
        amount_out: actual_amount_out,
        fee_amount: 0,
    })
}

fn optional_meta(account: &AccountInfo<'_>, program_id: Pubkey) -> AccountMeta {
    if account.key() == program_id {
        AccountMeta::new_readonly(account.key(), false)
    } else {
        AccountMeta::new(account.key(), false)
    }
}

fn ceil_mul_div(value: u64, numerator: u64, denominator: u64) -> Result<u64> {
    require!(denominator > 0, ArbitrageError::MathOverflow);
    let result = u128::from(value)
        .checked_mul(u128::from(numerator))
        .ok_or(ArbitrageError::MathOverflow)?
        .checked_add(u128::from(denominator - 1))
        .ok_or(ArbitrageError::MathOverflow)?
        .checked_div(u128::from(denominator))
        .ok_or(ArbitrageError::MathOverflow)?;
    u64::try_from(result).map_err(|_| ArbitrageError::MathOverflow.into())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or_else(|| ArbitrageError::InvalidAccount.into())
}
