use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::CUBE_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const CUBE_STEP_ACCOUNTS: usize = 4;
pub const CUBE_SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

const POOL_ACCOUNT_LEN: usize = 1_683;
const POOL_DISCRIMINATOR: [u8; 8] = [137, 210, 42, 22, 209, 156, 43, 78];
const TOKEN_SLOTS_OFFSET: usize = 179;
const TOKEN_SLOT_LEN: usize = 144;
const MAX_TOKENS: usize = 10;
const MIN_TOKENS: usize = 2;
const WEIGHT_SCALE: u64 = 10_000;
const MAX_SWAP_FEE_RATE: u32 = 100_000;

#[derive(Clone)]
pub struct CubeAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub input_token_program: &'a AccountInfo<'info>,
    pub output_token_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_cube_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<(u8, u8)> {
    require!(
        step.len() == CUBE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require_keys_eq!(
        step[0].key(),
        CUBE_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        input_token_program.key() == anchor_spl::token::ID
            || input_token_program.key() == anchor_spl::token_2022::ID,
        ArbitrageError::InvalidProgramId
    );
    require!(
        output_token_program.key() == anchor_spl::token::ID
            || output_token_program.key() == anchor_spl::token_2022::ID,
        ArbitrageError::InvalidProgramId
    );

    let pool_data = step[1].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_ACCOUNT_LEN && pool_data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let config = read_pubkey(&pool_data, 8)?;
    let bump = read_u8(&pool_data, 40)?;
    let token_count = usize::from(read_u8(&pool_data, 41)?);
    let pool_salt = read_u64(&pool_data, 42)?;
    let swap_fee_rate = read_u32(&pool_data, 50)?;
    let protocol_fee_rate = read_u16(&pool_data, 54)?;
    require!(
        (MIN_TOKENS..=MAX_TOKENS).contains(&token_count)
            && swap_fee_rate <= MAX_SWAP_FEE_RATE
            && swap_fee_rate == u32::from(fee_rate) * 100
            && protocol_fee_rate <= 5_000
            && read_bool(&pool_data, 64)?
            && read_bool(&pool_data, 65)?,
        ArbitrageError::InvalidInstructionData
    );
    let salt_bytes = pool_salt.to_le_bytes();
    let (expected_pool, expected_bump) = Pubkey::find_program_address(
        &[b"cubic_pool", config.as_ref(), &salt_bytes],
        &step[0].key(),
    );
    require_keys_eq!(step[1].key(), expected_pool, ArbitrageError::InvalidAccount);
    require!(bump == expected_bump, ArbitrageError::InvalidAccount);

    let mut input_index = None;
    let mut output_index = None;
    let mut weight_sum = 0_u64;
    for index in 0..token_count {
        let offset = TOKEN_SLOTS_OFFSET
            .checked_add(
                index
                    .checked_mul(TOKEN_SLOT_LEN)
                    .ok_or(ArbitrageError::MathOverflow)?,
            )
            .ok_or(ArbitrageError::MathOverflow)?;
        let mint = read_pubkey(&pool_data, offset)?;
        let token_program = read_pubkey(&pool_data, offset + 32)?;
        let weight = read_u64(&pool_data, offset + 64)?;
        let virtual_balance = read_u64(&pool_data, offset + 88)?;
        require!(
            mint != Pubkey::default()
                && (token_program == anchor_spl::token::ID
                    || token_program == anchor_spl::token_2022::ID)
                && weight > 0
                && virtual_balance > 0,
            ArbitrageError::InvalidAccount
        );
        weight_sum = weight_sum
            .checked_add(weight)
            .ok_or(ArbitrageError::MathOverflow)?;
        if mint == input_mint.key() {
            require_keys_eq!(
                token_program,
                input_token_program.key(),
                ArbitrageError::InvalidProgramId
            );
            input_index = Some(u8::try_from(index).map_err(|_| ArbitrageError::MathOverflow)?);
            validate_slot_vault(
                &pool_data,
                offset,
                &step[1],
                &step[2],
                input_mint,
                input_token_program,
            )?;
        }
        if mint == output_mint.key() {
            require_keys_eq!(
                token_program,
                output_token_program.key(),
                ArbitrageError::InvalidProgramId
            );
            output_index = Some(u8::try_from(index).map_err(|_| ArbitrageError::MathOverflow)?);
            validate_slot_vault(
                &pool_data,
                offset,
                &step[1],
                &step[3],
                output_mint,
                output_token_program,
            )?;
        }
    }
    require!(weight_sum == WEIGHT_SCALE, ArbitrageError::InvalidAccount);
    drop(pool_data);
    let input_index = input_index.ok_or(ArbitrageError::InvalidTokenMint)?;
    let output_index = output_index.ok_or(ArbitrageError::InvalidTokenMint)?;
    require!(
        input_index != output_index,
        ArbitrageError::InvalidTokenMint
    );
    Ok((input_index, output_index))
}

fn validate_slot_vault<'info>(
    pool_data: &[u8],
    slot_offset: usize,
    pool: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    let expected_vault = anchor_spl::associated_token::get_associated_token_address_with_program_id(
        &pool.key(),
        &mint.key(),
        &token_program.key(),
    );
    require_keys_eq!(vault.key(), expected_vault, ArbitrageError::InvalidAccount);
    validate_token_account_for_mint_and_authority(vault, mint, token_program, pool)?;
    let lp_actual_balance = read_u64(pool_data, slot_offset + 96)?;
    let protocol_fees_owed = read_u64(pool_data, slot_offset + 104)?;
    let expected_amount = lp_actual_balance
        .checked_add(protocol_fees_owed)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        read_token_amount(vault)? == expected_amount,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

pub fn cube_swap<'a, 'info>(
    accounts: CubeAccounts<'a, 'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    input_index: u8,
    output_index: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(
        input_index != output_index,
        ArbitrageError::InvalidTokenMint
    );
    let mut data = Vec::with_capacity(26);
    data.extend_from_slice(&CUBE_SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.push(input_index);
    data.push(output_index);
    let instruction = Instruction {
        program_id: accounts.step[0].key(),
        accounts: vec![
            AccountMeta::new(accounts.step[1].key(), false),
            AccountMeta::new_readonly(accounts.input_mint.key(), false),
            AccountMeta::new_readonly(accounts.output_mint.key(), false),
            AccountMeta::new(accounts.user_input.key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new_readonly(accounts.input_token_program.key(), false),
            AccountMeta::new_readonly(accounts.output_token_program.key(), false),
        ],
        data,
    };
    let pre_output = read_token_amount(accounts.user_output)?;
    let infos = vec![
        accounts.step[1].clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.payer.clone(),
        accounts.input_token_program.clone(),
        accounts.output_token_program.clone(),
        accounts.step[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    let amount_out = token_balance_delta(accounts.user_output, pre_output)?;
    require!(
        amount_out >= minimum_amount_out,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

fn read_bool(data: &[u8], offset: usize) -> Result<bool> {
    match read_u8(data, offset)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => err!(ArbitrageError::InvalidAccount),
    }
}

fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or(ArbitrageError::InvalidAccount.into())
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}
