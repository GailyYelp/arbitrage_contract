use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
    system_instruction,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::sugar_money_program_id,
        types::{
            read_token_amount, read_token_mint_from_data, read_token_owner_from_data,
            token_balance_delta, SwapResult,
        },
    },
    protocal::wsol::{close_wsol_for_native, restore_wsol_after_native, WsolBridgeAccounts},
};

pub const SUGAR_MONEY_STEP_ACCOUNTS: usize = 11;
pub const SUGAR_MONEY_BUY_EXACT_IN_DISCRIMINATOR: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236];
pub const SUGAR_MONEY_SELL_EXACT_IN_DISCRIMINATOR: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26];

const STATE_DISCRIMINATOR: [u8; 8] = [216, 146, 107, 94, 104, 75, 182, 177];
const CURVE_DISCRIMINATOR: [u8; 8] = [191, 180, 249, 66, 180, 71, 51, 182];
const STATE_ACCOUNT_LEN: usize = 235;
const CURVE_ACCOUNT_LEN: usize = 51;
const STATE_FEE_RECEIVER_OFFSET: usize = 107;
const STATE_FEE_BPS_OFFSET: usize = 139;
const STATE_INITIAL_TOKEN_RESERVE_OFFSET: usize = 147;
const STATE_INITIAL_SOL_RESERVE_OFFSET: usize = 155;
const CURVE_COMPLETED_OFFSET: usize = 10;
const CURVE_REAL_TOKEN_RESERVE_OFFSET: usize = 18;
const CURVE_REAL_SOL_RESERVE_OFFSET: usize = 26;
const CURVE_VIRTUAL_TOKEN_RESERVE_OFFSET: usize = 34;
const CURVE_VIRTUAL_SOL_RESERVE_OFFSET: usize = 42;
const CURVE_MIGRATION_KIND_OFFSET: usize = 50;

#[cfg(feature = "devnet")]
const STATE_VERSION: u8 = 0;
#[cfg(not(feature = "devnet"))]
const STATE_VERSION: u8 = 1;

#[derive(Clone)]
pub struct SugarMoneyAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub wsol_token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i64(data: &[u8], offset: usize) -> Result<i64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(i64::from_le_bytes(bytes))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

pub fn validate_sugar_money_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == SUGAR_MONEY_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require_keys_eq!(
        step[0].key(),
        sugar_money_program_id(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[7].key(),
        anchor_spl::associated_token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[8].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[9].key(),
        anchor_lang::solana_program::sysvar::rent::ID,
        ArbitrageError::InvalidAccount
    );

    let token_mint = if direction == 0 {
        output_mint
    } else {
        input_mint
    };
    let token_program = if direction == 0 {
        output_token_program
    } else {
        input_token_program
    };
    require_keys_eq!(
        step[6].key(),
        token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    require!(
        step[6].key() == anchor_spl::token::ID || step[6].key() == anchor_spl::token_2022::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *token_mint.owner,
        step[6].key(),
        ArbitrageError::InvalidTokenMint
    );

    let (expected_state, state_bump) = Pubkey::find_program_address(&[b"state"], &step[0].key());
    let (expected_curve, curve_bump) = Pubkey::find_program_address(
        &[b"bonding_curve_", token_mint.key().as_ref()],
        &step[0].key(),
    );
    let (expected_sol_vault, _) = Pubkey::find_program_address(
        &[b"bonding_curve_", token_mint.key().as_ref(), b"_sol_vault"],
        &step[0].key(),
    );
    let expected_event = Pubkey::find_program_address(&[b"__event_authority"], &step[0].key()).0;
    require_keys_eq!(
        step[1].key(),
        expected_state,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[2].key(),
        expected_curve,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].key(),
        expected_sol_vault,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[10].key(),
        expected_event,
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
    require_keys_eq!(
        *step[3].owner,
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidAccount
    );
    require!(step[3].data_is_empty(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[4].owner,
        step[6].key(),
        ArbitrageError::InvalidAccount
    );

    let state_data = step[1].try_borrow_data()?;
    require!(
        state_data.len() == STATE_ACCOUNT_LEN && state_data.starts_with(&STATE_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        state_data[8] == state_bump && state_data[9] == STATE_VERSION && state_data[10] == 1,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&state_data, STATE_FEE_RECEIVER_OFFSET)?,
        step[5].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(&state_data, STATE_FEE_BPS_OFFSET)? == u64::from(fee_rate)
            && fee_rate < 10_000
            && read_u64(&state_data, STATE_INITIAL_TOKEN_RESERVE_OFFSET)? > 0
            && read_u64(&state_data, STATE_INITIAL_SOL_RESERVE_OFFSET)? > 0,
        ArbitrageError::InvalidInstructionData
    );
    drop(state_data);

    let curve_data = step[2].try_borrow_data()?;
    require!(
        curve_data.len() == CURVE_ACCOUNT_LEN && curve_data.starts_with(&CURVE_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let real_token_reserve = read_u64(&curve_data, CURVE_REAL_TOKEN_RESERVE_OFFSET)?;
    let real_sol_reserve = read_u64(&curve_data, CURVE_REAL_SOL_RESERVE_OFFSET)?;
    require!(
        curve_data[8] == curve_bump
            && curve_data[9] == 1
            && read_i64(&curve_data, CURVE_COMPLETED_OFFSET)? == 0
            && real_token_reserve > 0
            && real_sol_reserve > 0
            && read_u64(&curve_data, CURVE_VIRTUAL_TOKEN_RESERVE_OFFSET)? > 0
            && read_u64(&curve_data, CURVE_VIRTUAL_SOL_RESERVE_OFFSET)? > 0
            && curve_data[CURVE_MIGRATION_KIND_OFFSET] <= 1,
        ArbitrageError::InvalidAccount
    );
    drop(curve_data);

    let expected_token_vault =
        anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &step[2].key(),
            &token_mint.key(),
            &step[6].key(),
        );
    require_keys_eq!(
        step[4].key(),
        expected_token_vault,
        ArbitrageError::InvalidAccount
    );
    let token_vault_data = step[4].try_borrow_data()?;
    require_keys_eq!(
        read_token_mint_from_data(&token_vault_data)?,
        token_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_token_owner_from_data(&token_vault_data)?,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(&token_vault_data, 64)? == real_token_reserve,
        ArbitrageError::InvalidAccount
    );
    drop(token_vault_data);

    let rent_reserve = Rent::get()?.minimum_balance(0);
    require!(
        step[3].lamports()
            == real_sol_reserve
                .checked_add(rent_reserve)
                .ok_or(ArbitrageError::MathOverflow)?,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn exact_in_data(
    discriminator: [u8; 8],
    curve_bump: u8,
    sol_vault_bump: u8,
    amount_in: u64,
    min_amount_out: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(26);
    data.extend_from_slice(&discriminator);
    data.push(curve_bump);
    data.push(sol_vault_bump);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn invoke_sugar_money<'info>(
    accounts: &SugarMoneyAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    buy: bool,
) -> Result<()> {
    let token_mint = if buy {
        accounts.output_mint
    } else {
        accounts.input_mint
    };
    let user_token = if buy {
        accounts.user_output
    } else {
        accounts.user_input
    };
    let receiver = if buy {
        AccountMeta::new_readonly(accounts.payer.key(), false)
    } else {
        AccountMeta::new(accounts.payer.key(), false)
    };
    let discriminator = if buy {
        SUGAR_MONEY_BUY_EXACT_IN_DISCRIMINATOR
    } else {
        SUGAR_MONEY_SELL_EXACT_IN_DISCRIMINATOR
    };
    let (_, curve_bump) = Pubkey::find_program_address(
        &[b"bonding_curve_", token_mint.key().as_ref()],
        &accounts.step[0].key(),
    );
    let (_, sol_vault_bump) = Pubkey::find_program_address(
        &[b"bonding_curve_", token_mint.key().as_ref(), b"_sol_vault"],
        &accounts.step[0].key(),
    );
    let instruction = Instruction {
        program_id: accounts.step[0].key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.step[1].key(), false),
            AccountMeta::new_readonly(token_mint.key(), false),
            AccountMeta::new(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(accounts.step[4].key(), false),
            AccountMeta::new(user_token.key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            receiver,
            AccountMeta::new(accounts.step[5].key(), false),
            AccountMeta::new_readonly(accounts.step[6].key(), false),
            AccountMeta::new_readonly(accounts.step[7].key(), false),
            AccountMeta::new_readonly(accounts.step[8].key(), false),
            AccountMeta::new_readonly(accounts.step[9].key(), false),
            AccountMeta::new_readonly(accounts.step[10].key(), false),
            AccountMeta::new_readonly(accounts.step[0].key(), false),
        ],
        data: exact_in_data(
            discriminator,
            curve_bump,
            sol_vault_bump,
            amount_in,
            min_amount_out,
        ),
    };
    invoke(
        &instruction,
        &[
            accounts.step[1].clone(),
            token_mint.clone(),
            accounts.step[2].clone(),
            accounts.step[3].clone(),
            accounts.step[4].clone(),
            user_token.clone(),
            accounts.payer.clone(),
            accounts.payer.clone(),
            accounts.step[5].clone(),
            accounts.step[6].clone(),
            accounts.step[7].clone(),
            accounts.step[8].clone(),
            accounts.step[9].clone(),
            accounts.step[10].clone(),
            accounts.step[0].clone(),
        ],
    )?;
    Ok(())
}

pub fn sugar_money_swap<'info>(
    accounts: SugarMoneyAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    buy: bool,
) -> Result<SwapResult> {
    require!(
        accounts.step.len() == SUGAR_MONEY_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    if buy {
        let bridge = WsolBridgeAccounts {
            payer: accounts.payer,
            user_wsol: accounts.user_input,
            wsol_mint: accounts.input_mint,
            token_program: accounts.wsol_token_program,
            associated_token_program: accounts.associated_token_program,
            system_program: &accounts.step[8],
        };
        let remaining_wsol = close_wsol_for_native(bridge, amount_in)?;
        invoke_sugar_money(&accounts, amount_in, min_amount_out, true)?;
        restore_wsol_after_native(bridge, remaining_wsol)?;
    } else {
        let pre_lamports = accounts.payer.lamports();
        invoke_sugar_money(&accounts, amount_in, min_amount_out, false)?;
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
                accounts.step[8].clone(),
            ],
        )?;
        invoke(
            &anchor_spl::token::spl_token::instruction::sync_native(
                accounts.wsol_token_program.key,
                accounts.user_output.key,
            )?,
            &[
                accounts.user_output.clone(),
                accounts.wsol_token_program.clone(),
            ],
        )?;
    }
    let amount_out = token_balance_delta(accounts.user_output, pre_out)?;
    require!(
        amount_out >= min_amount_out,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_in_payload_matches_current_anchor_layout() {
        let data = exact_in_data(SUGAR_MONEY_BUY_EXACT_IN_DISCRIMINATOR, 1, 2, 3, 4);
        assert_eq!(data.len(), 26);
        assert_eq!(&data[..8], &SUGAR_MONEY_BUY_EXACT_IN_DISCRIMINATOR);
        assert_eq!(&data[8..10], &[1, 2]);
        assert_eq!(&data[10..18], &3_u64.to_le_bytes());
        assert_eq!(&data[18..26], &4_u64.to_le_bytes());
    }
}
