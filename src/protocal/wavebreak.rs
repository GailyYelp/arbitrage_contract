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
        program_ids::WAVEBREAK_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const WAVEBREAK_STEP_ACCOUNTS: usize = 8;

const CURVE_ACCOUNT_LEN: usize = 2_048;
const CURVE_DISCRIMINATOR: u8 = 2;
const BASE_MINT_OFFSET: usize = 1;
const QUOTE_MINT_OFFSET: usize = 33;
const BUY_REQUIRES_PERMISSION_OFFSET: usize = 98;
const SELL_REQUIRES_PERMISSION_OFFSET: usize = 131;
const QUOTE_AMOUNT_OFFSET: usize = 208;
const BASE_AMOUNT_OFFSET: usize = 216;
const MAX_BUY_AMOUNT_OFFSET: usize = 264;
const MAX_SELL_AMOUNT_OFFSET: usize = 272;
const SWAP_FEE_BPS_OFFSET: usize = 280;
const BASE_ALLOCATION_BPS_OFFSET: usize = 282;
const SELL_EXACT_IN_SELECTOR: u8 = 10;
const BUY_EXACT_IN_SELECTOR: u8 = 8;
const EXACT_IN_DATA_LEN: usize = 27;

pub struct WavebreakAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub curve: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub base_token_program: &'info AccountInfo<'info>,
    pub quote_token_program: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn wavebreak_swap<'info>(
    accounts: WavebreakAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(min_amount_out > 0, ArbitrageError::InvalidAmount);
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let data = build_wavebreak_exact_in_data(amount_in, min_amount_out, direction)?;
    let instruction = Instruction {
        program_id: accounts.program.key(),
        accounts: vec![
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new(accounts.curve.key(), false),
            AccountMeta::new(accounts.base_mint.key(), false),
            AccountMeta::new(accounts.user_base.key(), false),
            AccountMeta::new_readonly(accounts.quote_mint.key(), false),
            AccountMeta::new(accounts.quote_vault.key(), false),
            AccountMeta::new(accounts.user_quote.key(), false),
            AccountMeta::new_readonly(accounts.system_program.key(), false),
            AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
            AccountMeta::new_readonly(accounts.base_token_program.key(), false),
            AccountMeta::new_readonly(accounts.quote_token_program.key(), false),
        ],
        data,
    };
    let infos = vec![
        accounts.payer.clone(),
        accounts.curve.clone(),
        accounts.base_mint.clone(),
        accounts.user_base.clone(),
        accounts.quote_mint.clone(),
        accounts.quote_vault.clone(),
        accounts.user_quote.clone(),
        accounts.system_program.clone(),
        accounts.associated_token_program.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.program.clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn build_wavebreak_exact_in_data(
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<Vec<u8>> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(min_amount_out > 0, ArbitrageError::InvalidAmount);
    let (selector, quote_threshold, base_threshold) = match direction {
        0 => (SELL_EXACT_IN_SELECTOR, min_amount_out, amount_in),
        1 => (BUY_EXACT_IN_SELECTOR, amount_in, min_amount_out),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    let mut data = Vec::with_capacity(EXACT_IN_DATA_LEN);
    data.push(selector);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(0); // Closed arbitrage routes must never accept partial fills.
    data.push(1); // Some((quote_threshold, base_threshold)).
    data.extend_from_slice(&quote_threshold.to_le_bytes());
    data.extend_from_slice(&base_threshold.to_le_bytes());
    Ok(data)
}

#[allow(clippy::too_many_arguments)]
pub fn validate_wavebreak_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    amount_in: u64,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == WAVEBREAK_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        WAVEBREAK_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[3].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[4].key(),
        anchor_spl::associated_token::ID,
        ArbitrageError::InvalidProgramId
    );
    for token_program in [&step[5], &step[6]] {
        require!(
            matches!(
                token_program.key(),
                anchor_spl::token::ID | anchor_spl::token_2022::ID
            ),
            ArbitrageError::InvalidProgramId
        );
    }
    let (base_mint, quote_mint, base_token_program, quote_token_program) = match direction {
        0 => (
            input_mint,
            output_mint,
            input_token_program,
            output_token_program,
        ),
        1 => (
            output_mint,
            input_mint,
            output_token_program,
            input_token_program,
        ),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    require_keys_eq!(
        step[5].key(),
        base_token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[6].key(),
        quote_token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[7].key(),
        base_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        *base_mint.owner,
        base_token_program.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        *quote_mint.owner,
        quote_token_program.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        *step[1].owner,
        WAVEBREAK_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );

    let curve_data = step[1].try_borrow_data()?;
    require!(
        curve_data.len() == CURVE_ACCOUNT_LEN && curve_data[0] == CURVE_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&curve_data, BASE_MINT_OFFSET)?,
        base_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&curve_data, QUOTE_MINT_OFFSET)?,
        quote_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    let permission_offset = if direction == 0 {
        SELL_REQUIRES_PERMISSION_OFFSET
    } else {
        BUY_REQUIRES_PERMISSION_OFFSET
    };
    require!(
        curve_data.get(permission_offset).copied() == Some(0),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(&curve_data, QUOTE_AMOUNT_OFFSET)? > 0
            && read_u64(&curve_data, BASE_AMOUNT_OFFSET)? > 0
            && read_u16(&curve_data, SWAP_FEE_BPS_OFFSET)? <= 10_000
            && read_u16(&curve_data, BASE_ALLOCATION_BPS_OFFSET)? <= 10_000,
        ArbitrageError::InvalidAccount
    );
    let max_amount = if direction == 0 {
        read_u64(&curve_data, MAX_SELL_AMOUNT_OFFSET)?
    } else {
        read_u64(&curve_data, MAX_BUY_AMOUNT_OFFSET)?
    };
    require!(
        max_amount > 0 && amount_in <= max_amount,
        ArbitrageError::InvalidAmount
    );
    drop(curve_data);

    let (expected_quote_vault, _) = Pubkey::find_program_address(
        &[
            step[1].key.as_ref(),
            quote_token_program.key.as_ref(),
            quote_mint.key.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    );
    require_keys_eq!(
        step[2].key(),
        expected_quote_vault,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(
        &step[2],
        quote_mint,
        quote_token_program,
        &step[1],
    )
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
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
    fn exact_in_wire_matches_wavebreak_borsh_layout() {
        let sell = build_wavebreak_exact_in_data(1_000, 900, 0).unwrap();
        assert_eq!(sell.len(), EXACT_IN_DATA_LEN);
        assert_eq!(sell[0], SELL_EXACT_IN_SELECTOR);
        assert_eq!(sell[9], 0);
        assert_eq!(sell[10], 1);
        assert_eq!(&sell[11..19], &900_u64.to_le_bytes());
        assert_eq!(&sell[19..27], &1_000_u64.to_le_bytes());

        let buy = build_wavebreak_exact_in_data(1_000, 900, 1).unwrap();
        assert_eq!(buy[0], BUY_EXACT_IN_SELECTOR);
        assert_eq!(&buy[11..19], &1_000_u64.to_le_bytes());
        assert_eq!(&buy[19..27], &900_u64.to_le_bytes());
    }
}
