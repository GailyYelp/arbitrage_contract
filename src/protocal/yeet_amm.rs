use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program::invoke,
    },
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::yeet_amm_program_id,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint,
            validate_token_account_for_mint_and_authority, SwapResult,
        },
    },
};

pub const YEET_AMM_STEP_ACCOUNTS: usize = 8;

const POOL_ACCOUNT_LEN: usize = 593;
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const SWAP_BASE_IN_DISCRIMINATOR: [u8; 8] = [42, 236, 72, 162, 242, 24, 39, 84];
const SWAP_BASE_IN_DATA_LEN: usize = 48;
const TOTAL_FEE_BPS: u16 = 100;
const QUOTE_DEADLINE_SLOTS: u64 = 150;
const MINT_A_OFFSET: usize = 10;
const MINT_B_OFFSET: usize = 42;
const VAULT_A_OFFSET: usize = 74;
const VAULT_B_OFFSET: usize = 106;
const LP_MINT_OFFSET: usize = 138;
const CREATOR_OFFSET: usize = 170;
const RESERVE_A_OFFSET: usize = 266;
const RESERVE_B_OFFSET: usize = 274;
const FEE_BPS_OFFSET: usize = 282;
const INITIALIZED_OFFSET: usize = 414;
const CREATION_SLOT_OFFSET: usize = 448;
const VIRTUAL_RESERVE_A_OFFSET: usize = 496;
const VIRTUAL_RESERVE_B_OFFSET: usize = 504;
const POOL_MODE_OFFSET: usize = 520;
const GRAD_THRESHOLD_OFFSET: usize = 521;
const GRAD_SLOT_OFFSET: usize = 529;
const QUOTE_MINT_OFFSET: usize = 537;
const DBC_MODE: u8 = 0;
const AMM_MODE: u8 = 1;
const WSOL_MINT: Pubkey = anchor_lang::pubkey!("So11111111111111111111111111111111111111112");

pub struct YeetAmmAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub output_token_account: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

#[allow(clippy::too_many_arguments)]
pub fn validate_yeet_amm_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<(u64, u64)> {
    require!(
        step.len() == YEET_AMM_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == TOTAL_FEE_BPS, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        yeet_amm_program_id(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[7].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        input_token_program.key(),
        step[7].key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        output_token_program.key(),
        step[7].key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        token_program.key(),
        step[7].key(),
        ArbitrageError::InvalidProgramId
    );
    require!(
        step[0].executable && step[7].executable,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *input_mint.owner,
        step[7].key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        *output_mint.owner,
        step[7].key(),
        ArbitrageError::InvalidTokenMint
    );

    let pool_data = step[1].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_ACCOUNT_LEN
            && pool_data.get(..8) == Some(POOL_DISCRIMINATOR.as_slice()),
        ArbitrageError::InvalidAccount
    );
    let mint_a = read_pubkey(&pool_data, MINT_A_OFFSET)?;
    let mint_b = read_pubkey(&pool_data, MINT_B_OFFSET)?;
    let vault_a = read_pubkey(&pool_data, VAULT_A_OFFSET)?;
    let vault_b = read_pubkey(&pool_data, VAULT_B_OFFSET)?;
    let lp_mint = read_pubkey(&pool_data, LP_MINT_OFFSET)?;
    let creator = read_pubkey(&pool_data, CREATOR_OFFSET)?;
    let reserve_a = read_u64(&pool_data, RESERVE_A_OFFSET)?;
    let reserve_b = read_u64(&pool_data, RESERVE_B_OFFSET)?;
    let fee_bps = read_u16(&pool_data, FEE_BPS_OFFSET)?;
    let initialized = read_u8(&pool_data, INITIALIZED_OFFSET)?;
    let creation_slot = read_u64(&pool_data, CREATION_SLOT_OFFSET)?;
    let virtual_reserve_a = read_u64(&pool_data, VIRTUAL_RESERVE_A_OFFSET)?;
    let virtual_reserve_b = read_u64(&pool_data, VIRTUAL_RESERVE_B_OFFSET)?;
    let pool_mode = read_u8(&pool_data, POOL_MODE_OFFSET)?;
    let grad_threshold = read_u64(&pool_data, GRAD_THRESHOLD_OFFSET)?;
    let grad_slot = read_u64(&pool_data, GRAD_SLOT_OFFSET)?;
    let quote_mint = read_pubkey(&pool_data, QUOTE_MINT_OFFSET)?;
    drop(pool_data);

    let lifecycle_is_valid = match pool_mode {
        DBC_MODE => {
            grad_threshold > 0 && grad_slot == 0 && virtual_reserve_a > 0 && virtual_reserve_b > 0
        }
        AMM_MODE => grad_threshold == 0 || grad_slot > 0,
        _ => false,
    };
    require!(
        initialized == 1
            && mint_a < mint_b
            && creator != Pubkey::default()
            && reserve_a > 0
            && reserve_b > 0
            && fee_bps == TOTAL_FEE_BPS
            && creation_slot > 0
            && quote_mint == WSOL_MINT
            && (quote_mint == mint_a || quote_mint == mint_b)
            && lifecycle_is_valid,
        ArbitrageError::InvalidAccount
    );
    let (expected_input_mint, expected_output_mint, mint_a_account, mint_b_account) =
        if direction == 0 {
            (mint_a, mint_b, input_mint, output_mint)
        } else {
            (mint_b, mint_a, output_mint, input_mint)
        };
    require_keys_eq!(
        input_mint.key(),
        expected_input_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output_mint,
        ArbitrageError::InvalidTokenMint
    );

    let program_id = step[0].key();
    let expected_pool = Pubkey::find_program_address(
        &[b"yeet_amm_pool", mint_a.as_ref(), mint_b.as_ref()],
        &program_id,
    )
    .0;
    let expected_authority =
        Pubkey::find_program_address(&[b"authority", step[1].key.as_ref()], &program_id).0;
    let expected_vault_a = Pubkey::find_program_address(
        &[b"vault", step[1].key.as_ref(), mint_a.as_ref()],
        &program_id,
    )
    .0;
    let expected_vault_b = Pubkey::find_program_address(
        &[b"vault", step[1].key.as_ref(), mint_b.as_ref()],
        &program_id,
    )
    .0;
    let expected_lp_mint =
        Pubkey::find_program_address(&[b"lp_mint", step[1].key.as_ref()], &program_id).0;
    let expected_creator_fee_vault = Pubkey::find_program_address(
        &[b"fee_vault", step[1].key.as_ref(), input_mint.key.as_ref()],
        &program_id,
    )
    .0;
    let expected_protocol_fee_vault = Pubkey::find_program_address(
        &[
            b"yeet_fee_vault",
            step[1].key.as_ref(),
            input_mint.key.as_ref(),
        ],
        &program_id,
    )
    .0;
    require_keys_eq!(step[1].key(), expected_pool, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[2].key(),
        expected_authority,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[3].key(), vault_a, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[3].key(),
        expected_vault_a,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[4].key(), vault_b, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[4].key(),
        expected_vault_b,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(lp_mint, expected_lp_mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[5].key(),
        expected_creator_fee_vault,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        expected_protocol_fee_vault,
        ArbitrageError::InvalidAccount
    );

    validate_token_account_for_mint_and_authority(&step[3], mint_a_account, &step[7], &step[2])?;
    validate_token_account_for_mint_and_authority(&step[4], mint_b_account, &step[7], &step[2])?;
    validate_token_account_for_mint(&step[5], input_mint, &step[7])?;
    validate_token_account_for_mint(&step[6], input_mint, &step[7])?;
    require!(
        read_token_amount(&step[3])? == reserve_a && read_token_amount(&step[4])? == reserve_b,
        ArbitrageError::InvalidAccount
    );
    Ok((reserve_a, reserve_b))
}

pub fn yeet_amm_swap<'a, 'info>(
    accounts: YeetAmmAccounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    quoted_reserve_a: u64,
    quoted_reserve_b: u64,
) -> Result<SwapResult> {
    require!(
        amount_in > 0 && min_amount_out > 0,
        ArbitrageError::InvalidAmount
    );
    require!(
        quoted_reserve_a > 0 && quoted_reserve_b > 0,
        ArbitrageError::InvalidAmount
    );
    let deadline_slot = Clock::get()?
        .slot
        .checked_add(QUOTE_DEADLINE_SLOTS)
        .ok_or(ArbitrageError::MathOverflow)?;
    let data = build_swap_base_in_data(
        amount_in,
        min_amount_out,
        deadline_slot,
        quoted_reserve_a,
        quoted_reserve_b,
    )?;
    let instruction = Instruction {
        program_id: accounts.step[0].key(),
        accounts: vec![
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new(accounts.step[1].key(), false),
            AccountMeta::new_readonly(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(accounts.step[4].key(), false),
            AccountMeta::new(accounts.user_input.key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new(accounts.step[5].key(), false),
            AccountMeta::new(accounts.step[6].key(), false),
            AccountMeta::new_readonly(accounts.input_mint.key(), false),
            AccountMeta::new_readonly(accounts.output_mint.key(), false),
            AccountMeta::new_readonly(accounts.step[7].key(), false),
        ],
        data,
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let infos = vec![
        accounts.payer.clone(),
        accounts.step[1].clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.step[4].clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.step[5].clone(),
        accounts.step[6].clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.step[7].clone(),
        accounts.step[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn build_swap_base_in_data(
    amount_in: u64,
    min_amount_out: u64,
    deadline_slot: u64,
    quoted_reserve_a: u64,
    quoted_reserve_b: u64,
) -> Result<Vec<u8>> {
    require!(
        amount_in > 0 && min_amount_out > 0 && deadline_slot > 0,
        ArbitrageError::InvalidAmount
    );
    require!(
        quoted_reserve_a > 0 && quoted_reserve_b > 0,
        ArbitrageError::InvalidAmount
    );
    let mut data = Vec::with_capacity(SWAP_BASE_IN_DATA_LEN);
    data.extend_from_slice(&SWAP_BASE_IN_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.extend_from_slice(&deadline_slot.to_le_bytes());
    data.extend_from_slice(&quoted_reserve_a.to_le_bytes());
    data.extend_from_slice(&quoted_reserve_b.to_le_bytes());
    Ok(data)
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or(ArbitrageError::InvalidAccount.into())
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_swap_base_in_wire_is_pinned() {
        let data = build_swap_base_in_data(
            1_000_000,
            56_410_094_890_134,
            476_976_796,
            1_000_000_000_000_000_000,
            700_000_000,
        )
        .unwrap();

        assert_eq!(data.len(), SWAP_BASE_IN_DATA_LEN);
        assert_eq!(&data[..8], &SWAP_BASE_IN_DISCRIMINATOR);
        assert_eq!(&data[8..16], &1_000_000_u64.to_le_bytes());
        assert_eq!(&data[16..24], &56_410_094_890_134_u64.to_le_bytes());
        assert_eq!(&data[24..32], &476_976_796_u64.to_le_bytes());
        assert_eq!(&data[32..40], &1_000_000_000_000_000_000_u64.to_le_bytes());
        assert_eq!(&data[40..48], &700_000_000_u64.to_le_bytes());
    }
}
