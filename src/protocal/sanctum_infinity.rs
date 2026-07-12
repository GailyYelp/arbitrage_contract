use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    bpf_loader_upgradeable,
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const SANCTUM_INFINITY_STEP_ACCOUNTS: usize = 13;
pub const SANCTUM_INFINITY_POOL_STATE: Pubkey =
    anchor_lang::pubkey!("AYhux5gJzCoeoc1PoJ1VxwPDe22RwcvpHviLDD1oCGvW");
pub const SANCTUM_INFINITY_LST_STATE_LIST: Pubkey =
    anchor_lang::pubkey!("Gb7m4daakbVbrFLR33FKMDVMHAprRZ66CSYt4bpFwUgS");
pub const SANCTUM_INFINITY_JUPSOL_MINT: Pubkey =
    anchor_lang::pubkey!("jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v");
pub const SANCTUM_INFINITY_WSOL_MINT: Pubkey =
    anchor_lang::pubkey!("So11111111111111111111111111111111111111112");

const WSOL_CALCULATOR_PROGRAM: Pubkey =
    anchor_lang::pubkey!("wsoGmxQLSvwWpuaidCApxN5kEowLe2HLQLJhCQnj4bE");
const SANCTUM_SPL_MULTI_CALCULATOR_PROGRAM: Pubkey =
    anchor_lang::pubkey!("ssmbu3KZxgonUtjEMCKspZzxvUQCxAFnyh1rcHUeEDo");
const SANCTUM_SPL_MULTI_CALCULATOR_STATE: Pubkey =
    anchor_lang::pubkey!("Ehcuy2BzuY9BscqcH2K43tDKqoi6xQHxChtVjzrMfvU8");
const JUPSOL_STAKE_POOL: Pubkey =
    anchor_lang::pubkey!("8VpRhuxa7sUUepdY3kQiTmX9rS5vx4WgaXiAnXq4KCtr");
const SANCTUM_SPL_MULTI_POOL_PROGRAM: Pubkey =
    anchor_lang::pubkey!("SPMBzsVUuoHA4Jm6KunbsotaahvVikZs1JyTW6iJvbn");
const SANCTUM_SPL_MULTI_POOL_PROGRAM_DATA: Pubkey =
    anchor_lang::pubkey!("HxBTMuB7cFBPVWVJjTi9iBF8MPd7mfY1QnrrWfLAySFd");
const FLAT_SLAB_PRICING_PROGRAM: Pubkey =
    anchor_lang::pubkey!("s1b6NRXj6ygNu1QMKXh2H9LUR2aPApAAm1UQ2DjdhNV");
const FLAT_SLAB_ACCOUNT: Pubkey =
    anchor_lang::pubkey!("4T9YzXnmQFMyYi2nrxyXjhtUANavmCkxGCsU3GKaNjwT");

const SWAP_EXACT_IN_V2_DISCRIMINATOR: u8 = 23;
const SWAP_EXACT_IN_V2_DATA_LEN: usize = 27;
const LST_STATE_LEN: usize = 80;
const LST_STATE_MINT_OFFSET: usize = 16;
const LST_STATE_CALCULATOR_OFFSET: usize = 48;

#[derive(Clone)]
pub struct SanctumInfinityAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

pub fn sanctum_infinity_swap<'info>(
    accounts: SanctumInfinityAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let (input_index, output_index) = validate_sanctum_infinity_semantic_accounts(
        accounts.step,
        direction,
        accounts.input_mint,
        accounts.output_mint,
        accounts.token_program,
    )?;
    let pre_out = read_token_amount(accounts.user_output)?;
    let data = swap_exact_in_v2_data(
        direction,
        input_index,
        output_index,
        minimum_amount_out,
        amount_in,
    );
    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.input_mint.key(), false),
        AccountMeta::new_readonly(accounts.output_mint.key(), false),
        AccountMeta::new(accounts.user_input.key(), false),
        AccountMeta::new(accounts.user_output.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new(accounts.step[1].key(), false),
        AccountMeta::new(accounts.step[2].key(), false),
        AccountMeta::new(accounts.step[3].key(), false),
        AccountMeta::new(accounts.step[4].key(), false),
        AccountMeta::new_readonly(accounts.step[5].key(), false),
        AccountMeta::new_readonly(accounts.step[6].key(), false),
        AccountMeta::new_readonly(accounts.step[7].key(), false),
        AccountMeta::new_readonly(accounts.step[8].key(), false),
        AccountMeta::new_readonly(accounts.step[9].key(), false),
        AccountMeta::new_readonly(accounts.step[10].key(), false),
        AccountMeta::new_readonly(accounts.step[11].key(), false),
        AccountMeta::new_readonly(accounts.step[12].key(), false),
    ];
    let mut infos = vec![
        accounts.payer.clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.token_program.clone(),
        accounts.token_program.clone(),
    ];
    infos.extend(accounts.step[1..].iter().cloned());
    infos.push(accounts.step[0].clone());
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: metas,
            data: data.to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_sanctum_infinity_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<(u32, u32)> {
    require!(
        step.len() == SANCTUM_INFINITY_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[1].key(),
        SANCTUM_INFINITY_POOL_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[2].key(),
        SANCTUM_INFINITY_LST_STATE_LIST,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output) = if direction == 0 {
        (SANCTUM_INFINITY_WSOL_MINT, SANCTUM_INFINITY_JUPSOL_MINT)
    } else {
        (SANCTUM_INFINITY_JUPSOL_MINT, SANCTUM_INFINITY_WSOL_MINT)
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output,
        ArbitrageError::InvalidAccount
    );

    let list_data = step[2].try_borrow_data()?;
    let (input_index, input_calculator) =
        find_lst_index_and_calculator(&list_data, input_mint.key())?;
    let (output_index, output_calculator) =
        find_lst_index_and_calculator(&list_data, output_mint.key())?;
    let expected_input_calculator = if direction == 0 {
        WSOL_CALCULATOR_PROGRAM
    } else {
        SANCTUM_SPL_MULTI_CALCULATOR_PROGRAM
    };
    let expected_output_calculator = if direction == 0 {
        SANCTUM_SPL_MULTI_CALCULATOR_PROGRAM
    } else {
        WSOL_CALCULATOR_PROGRAM
    };
    require_keys_eq!(
        input_calculator,
        expected_input_calculator,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_calculator,
        expected_output_calculator,
        ArbitrageError::InvalidAccount
    );

    validate_reserve(&step[3], input_mint.key(), token_program)?;
    validate_reserve(&step[4], output_mint.key(), token_program)?;
    validate_calculator_accounts(step, direction)?;
    require_keys_eq!(
        step[11].key(),
        FLAT_SLAB_PRICING_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[12].key(),
        FLAT_SLAB_ACCOUNT,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[12].owner,
        FLAT_SLAB_PRICING_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    Ok((input_index, output_index))
}

fn validate_calculator_accounts(step: &[AccountInfo<'_>], direction: u8) -> Result<()> {
    let multi_start = if direction == 0 { 6 } else { 5 };
    let wsol_index = if direction == 0 { 5 } else { 10 };
    require_keys_eq!(
        step[wsol_index].key(),
        WSOL_CALCULATOR_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[multi_start].key(),
        SANCTUM_SPL_MULTI_CALCULATOR_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[multi_start + 1].key(),
        SANCTUM_SPL_MULTI_CALCULATOR_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[multi_start + 2].key(),
        JUPSOL_STAKE_POOL,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[multi_start + 3].key(),
        SANCTUM_SPL_MULTI_POOL_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[multi_start + 4].key(),
        SANCTUM_SPL_MULTI_POOL_PROGRAM_DATA,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[multi_start + 1].owner,
        SANCTUM_SPL_MULTI_CALCULATOR_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[multi_start + 2].owner,
        SANCTUM_SPL_MULTI_POOL_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[multi_start + 4].owner,
        bpf_loader_upgradeable::ID,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_reserve(
    reserve: &AccountInfo<'_>,
    mint: Pubkey,
    token_program: &AccountInfo<'_>,
) -> Result<()> {
    require_keys_eq!(
        *reserve.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = reserve.try_borrow_data()?;
    require!(data.len() >= 72, ArbitrageError::InvalidAccount);
    require_keys_eq!(read_pubkey(&data, 0)?, mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&data, 32)?,
        SANCTUM_INFINITY_POOL_STATE,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

#[allow(clippy::manual_is_multiple_of)]
fn find_lst_index_and_calculator(data: &[u8], mint: Pubkey) -> Result<(u32, Pubkey)> {
    require!(
        !data.is_empty() && data.len() % LST_STATE_LEN == 0,
        ArbitrageError::InvalidAccount
    );
    for (index, state) in data.chunks_exact(LST_STATE_LEN).enumerate() {
        if read_pubkey(state, LST_STATE_MINT_OFFSET)? == mint {
            let index = u32::try_from(index).map_err(|_| ArbitrageError::MathOverflow)?;
            return Ok((index, read_pubkey(state, LST_STATE_CALCULATOR_OFFSET)?));
        }
    }
    Err(ArbitrageError::InvalidAccount.into())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(array))
}

fn swap_exact_in_v2_data(
    direction: u8,
    input_index: u32,
    output_index: u32,
    minimum_amount_out: u64,
    amount_in: u64,
) -> [u8; SWAP_EXACT_IN_V2_DATA_LEN] {
    let mut data = [0_u8; SWAP_EXACT_IN_V2_DATA_LEN];
    data[0] = SWAP_EXACT_IN_V2_DISCRIMINATOR;
    data[1] = if direction == 0 { 1 } else { 5 };
    data[2] = if direction == 0 { 5 } else { 1 };
    data[3..7].copy_from_slice(&input_index.to_le_bytes());
    data[7..11].copy_from_slice(&output_index.to_le_bytes());
    data[11..19].copy_from_slice(&minimum_amount_out.to_le_bytes());
    data[19..27].copy_from_slice(&amount_in.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_in_data_matches_official_v2_layout() {
        let data = swap_exact_in_v2_data(0, 7, 19, 55, 1_000_000);
        assert_eq!(data[0], 23);
        assert_eq!(&data[1..3], &[1, 5]);
        assert_eq!(&data[3..7], &7_u32.to_le_bytes());
        assert_eq!(&data[7..11], &19_u32.to_le_bytes());
        assert_eq!(&data[11..19], &55_u64.to_le_bytes());
        assert_eq!(&data[19..27], &1_000_000_u64.to_le_bytes());

        let reverse = swap_exact_in_v2_data(1, 19, 7, 0, 42);
        assert_eq!(&reverse[1..3], &[5, 1]);
    }

    #[test]
    fn packed_lst_list_lookup_uses_eighty_byte_records() {
        let mut data = vec![0_u8; LST_STATE_LEN * 2];
        data[LST_STATE_MINT_OFFSET..LST_STATE_MINT_OFFSET + 32]
            .copy_from_slice(SANCTUM_INFINITY_WSOL_MINT.as_ref());
        data[LST_STATE_CALCULATOR_OFFSET..LST_STATE_CALCULATOR_OFFSET + 32]
            .copy_from_slice(WSOL_CALCULATOR_PROGRAM.as_ref());
        let second = LST_STATE_LEN;
        data[second + LST_STATE_MINT_OFFSET..second + LST_STATE_MINT_OFFSET + 32]
            .copy_from_slice(SANCTUM_INFINITY_JUPSOL_MINT.as_ref());
        data[second + LST_STATE_CALCULATOR_OFFSET..second + LST_STATE_CALCULATOR_OFFSET + 32]
            .copy_from_slice(SANCTUM_SPL_MULTI_CALCULATOR_PROGRAM.as_ref());

        assert_eq!(
            find_lst_index_and_calculator(&data, SANCTUM_INFINITY_JUPSOL_MINT).unwrap(),
            (1, SANCTUM_SPL_MULTI_CALCULATOR_PROGRAM)
        );
        assert!(
            find_lst_index_and_calculator(&data[..data.len() - 1], SANCTUM_INFINITY_WSOL_MINT)
                .is_err()
        );
    }
}
