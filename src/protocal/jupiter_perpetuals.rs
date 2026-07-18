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
        program_ids::JUPITER_PERPETUALS_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const JUPITER_PERPETUALS_STEP_ACCOUNTS: usize = 14;
const SWAP2_DISCRIMINATOR: [u8; 8] = [65, 75, 63, 76, 235, 91, 91, 136];
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const CUSTODY_DISCRIMINATOR: [u8; 8] = [1, 184, 48, 81, 93, 131, 63, 145];
const PERPETUALS_DISCRIMINATOR: [u8; 8] = [28, 167, 98, 191, 104, 82, 108, 196];
const CUSTODY_POOL_OFFSET: usize = 8;
const CUSTODY_MINT_OFFSET: usize = 40;
const CUSTODY_VAULT_OFFSET: usize = 72;
const CUSTODY_PYTHNET_OFFSET: usize = 106;
const CUSTODY_ALLOW_SWAP_OFFSET: usize = 199;
const CUSTODY_DOVES_AG_OFFSET: usize = 384;
const CUSTODY_REQUIRED_LEN: usize = 416;

pub struct JupiterPerpetualsAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_jupiter_perpetuals_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == JUPITER_PERPETUALS_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        JUPITER_PERPETUALS_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[12].key(),
        token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    let expected_transfer_authority =
        Pubkey::find_program_address(&[b"transfer_authority"], &JUPITER_PERPETUALS_PROGRAM_ID).0;
    let expected_perpetuals =
        Pubkey::find_program_address(&[b"perpetuals"], &JUPITER_PERPETUALS_PROGRAM_ID).0;
    let expected_event_authority =
        Pubkey::find_program_address(&[b"__event_authority"], &JUPITER_PERPETUALS_PROGRAM_ID).0;
    require_keys_eq!(
        step[1].key(),
        expected_transfer_authority,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[2].key(),
        expected_perpetuals,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[13].key(),
        expected_event_authority,
        ArbitrageError::InvalidAccount
    );
    for account in [&step[2], &step[3], &step[4], &step[8]] {
        require_keys_eq!(
            *account.owner,
            JUPITER_PERPETUALS_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
    }
    let perpetuals = step[2].try_borrow_data()?;
    require!(
        perpetuals.len() > 15 && perpetuals[..8] == PERPETUALS_DISCRIMINATOR && perpetuals[8] == 1,
        ArbitrageError::InvalidAccount
    );
    drop(perpetuals);
    validate_pool_custodies(&step[3], step[4].key(), step[8].key())?;

    let (side_a_mint, side_b_mint) = if direction == 0 {
        (input_mint, output_mint)
    } else {
        (output_mint, input_mint)
    };
    validate_custody(
        &step[4],
        &step[3],
        side_a_mint,
        &step[5],
        &step[6],
        &step[7],
        &step[1],
        token_program,
    )?;
    validate_custody(
        &step[8],
        &step[3],
        side_b_mint,
        &step[9],
        &step[10],
        &step[11],
        &step[1],
        token_program,
    )?;
    Ok(())
}

pub fn jupiter_perpetuals_swap<'a, 'info>(
    accounts: JupiterPerpetualsAccounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(
        amount_in > 0 && min_amount_out > 0,
        ArbitrageError::InvalidAmount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let (receiving, dispensing) = if direction == 0 {
        (&accounts.step[4..8], &accounts.step[8..12])
    } else {
        (&accounts.step[8..12], &accounts.step[4..8])
    };
    let instruction = Instruction {
        program_id: accounts.step[0].key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.payer.key(), true),
            AccountMeta::new(accounts.user_input.key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new_readonly(accounts.step[1].key(), false),
            AccountMeta::new_readonly(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(receiving[0].key(), false),
            AccountMeta::new_readonly(receiving[1].key(), false),
            AccountMeta::new_readonly(receiving[2].key(), false),
            AccountMeta::new(receiving[3].key(), false),
            AccountMeta::new(dispensing[0].key(), false),
            AccountMeta::new_readonly(dispensing[1].key(), false),
            AccountMeta::new_readonly(dispensing[2].key(), false),
            AccountMeta::new(dispensing[3].key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(accounts.step[13].key(), false),
            AccountMeta::new_readonly(accounts.step[0].key(), false),
        ],
        data: swap2_data(amount_in, min_amount_out),
    };
    let pre_out = read_token_amount(accounts.user_output)?;
    let infos = vec![
        accounts.payer.clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.step[1].clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        receiving[0].clone(),
        receiving[1].clone(),
        receiving[2].clone(),
        receiving[3].clone(),
        dispensing[0].clone(),
        dispensing[1].clone(),
        dispensing[2].clone(),
        dispensing[3].clone(),
        accounts.token_program.clone(),
        accounts.step[13].clone(),
        accounts.step[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

fn validate_pool_custodies(
    pool: &AccountInfo<'_>,
    custody_a: Pubkey,
    custody_b: Pubkey,
) -> Result<()> {
    let data = pool.try_borrow_data()?;
    require!(
        data.len() >= 16 && data[..8] == POOL_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    let name_len =
        usize::try_from(read_u32(&data, 8)?).map_err(|_| ArbitrageError::InvalidAccount)?;
    require!(
        name_len > 0 && name_len <= 64,
        ArbitrageError::InvalidAccount
    );
    let count_offset = 12usize
        .checked_add(name_len)
        .ok_or(ArbitrageError::MathOverflow)?;
    let custody_count = usize::try_from(read_u32(&data, count_offset)?)
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    require!(
        (2..=16).contains(&custody_count),
        ArbitrageError::InvalidAccount
    );
    let start = count_offset
        .checked_add(4)
        .ok_or(ArbitrageError::MathOverflow)?;
    let end = start
        .checked_add(
            custody_count
                .checked_mul(32)
                .ok_or(ArbitrageError::MathOverflow)?,
        )
        .ok_or(ArbitrageError::MathOverflow)?;
    let list = data.get(start..end).ok_or(ArbitrageError::InvalidAccount)?;
    let mut found_a = false;
    let mut found_b = false;
    for entry in list.chunks_exact(32) {
        let key = Pubkey::new_from_array(
            entry
                .try_into()
                .map_err(|_| ArbitrageError::InvalidAccount)?,
        );
        found_a |= key == custody_a;
        found_b |= key == custody_b;
    }
    require!(found_a && found_b, ArbitrageError::InvalidAccount);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_custody<'info>(
    custody: &AccountInfo<'info>,
    pool: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    doves: &AccountInfo<'info>,
    pythnet: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    transfer_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    let data = custody.try_borrow_data()?;
    require!(
        data.len() >= CUSTODY_REQUIRED_LEN && data[..8] == CUSTODY_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, CUSTODY_POOL_OFFSET)?,
        pool.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, CUSTODY_MINT_OFFSET)?,
        mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&data, CUSTODY_VAULT_OFFSET)?,
        vault.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, CUSTODY_PYTHNET_OFFSET)?,
        pythnet.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, CUSTODY_DOVES_AG_OFFSET)?,
        doves.key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        data[CUSTODY_ALLOW_SWAP_OFFSET] == 1,
        ArbitrageError::InvalidAccount
    );
    drop(data);
    require!(
        !doves.try_borrow_data()?.is_empty() && !pythnet.try_borrow_data()?.is_empty(),
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(vault, mint, token_program, transfer_authority)?;
    require!(
        read_token_amount(vault)? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
}

fn swap2_data(amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&SWAP2_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 32] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 4] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap2_wire_preserves_exact_input_and_minimum_output() {
        let data = swap2_data(100_000, 99_000);
        assert_eq!(data.len(), 24);
        assert_eq!(&data[..8], &SWAP2_DISCRIMINATOR);
        assert_eq!(&data[8..16], &100_000_u64.to_le_bytes());
        assert_eq!(&data[16..24], &99_000_u64.to_le_bytes());
    }
}
