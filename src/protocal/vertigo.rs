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
        program_ids::VERTIGO_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const VERTIGO_STEP_ACCOUNTS: usize = 7;

const POOL_ACCOUNT_LEN: usize = 229;
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
const OWNER_OFFSET: usize = 9;
const MINT_A_OFFSET: usize = 41;
const MINT_B_OFFSET: usize = 73;
const RESERVE_A_OFFSET: usize = 105;
const RESERVE_B_OFFSET: usize = 121;
const SHIFT_OFFSET: usize = 137;
const BUMP_OFFSET: usize = 169;
const NORMALIZATION_PERIOD_OFFSET: usize = 170;
const DECAY_OFFSET: usize = 178;
const ROYALTIES_BPS_OFFSET: usize = 194;
const PRIVILEGED_SWAPPER_OPTION_OFFSET: usize = 196;

pub struct VertigoAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub mint_a: &'info AccountInfo<'info>,
    pub mint_b: &'info AccountInfo<'info>,
    pub user_token_a: &'info AccountInfo<'info>,
    pub user_token_b: &'info AccountInfo<'info>,
    pub vault_a: &'info AccountInfo<'info>,
    pub vault_b: &'info AccountInfo<'info>,
    pub token_program_a: &'info AccountInfo<'info>,
    pub token_program_b: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn vertigo_swap<'info>(
    accounts: VertigoAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(min_amount_out > 0, ArbitrageError::InvalidAmount);
    let discriminator = match direction {
        0 => BUY_DISCRIMINATOR,
        1 => SELL_DISCRIMINATOR,
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    let instruction = Instruction {
        program_id: accounts.program.key(),
        accounts: vec![
            AccountMeta::new(accounts.pool.key(), false),
            AccountMeta::new_readonly(accounts.payer.key(), true),
            AccountMeta::new_readonly(accounts.owner.key(), false),
            AccountMeta::new_readonly(accounts.mint_a.key(), false),
            AccountMeta::new_readonly(accounts.mint_b.key(), false),
            AccountMeta::new(accounts.user_token_a.key(), false),
            AccountMeta::new(accounts.user_token_b.key(), false),
            AccountMeta::new(accounts.vault_a.key(), false),
            AccountMeta::new(accounts.vault_b.key(), false),
            AccountMeta::new_readonly(accounts.token_program_a.key(), false),
            AccountMeta::new_readonly(accounts.token_program_b.key(), false),
            AccountMeta::new_readonly(accounts.system_program.key(), false),
            AccountMeta::new_readonly(accounts.program.key(), false),
        ],
        data,
    };
    let infos = vec![
        accounts.pool.clone(),
        accounts.payer.clone(),
        accounts.owner.clone(),
        accounts.mint_a.clone(),
        accounts.mint_b.clone(),
        accounts.user_token_a.clone(),
        accounts.user_token_b.clone(),
        accounts.vault_a.clone(),
        accounts.vault_b.clone(),
        accounts.token_program_a.clone(),
        accounts.token_program_b.clone(),
        accounts.system_program.clone(),
        accounts.program.clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn validate_vertigo_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == VERTIGO_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        VERTIGO_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        VERTIGO_PROGRAM_ID,
        ArbitrageError::InvalidAccount
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
    let (mint_a, mint_b, token_program_a, token_program_b) = match direction {
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
        token_program_a.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[6].key(),
        token_program_b.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *mint_a.owner,
        token_program_a.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        *mint_b.owner,
        token_program_b.key(),
        ArbitrageError::InvalidTokenMint
    );

    let pool_data = step[1].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_ACCOUNT_LEN
            && pool_data.get(..8) == Some(POOL_DISCRIMINATOR.as_slice())
            && pool_data.get(8).copied() == Some(1),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&pool_data, OWNER_OFFSET)?,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&pool_data, MINT_A_OFFSET)?,
        mint_a.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&pool_data, MINT_B_OFFSET)?,
        mint_b.key(),
        ArbitrageError::InvalidTokenMint
    );
    let bump = *pool_data
        .get(BUMP_OFFSET)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let decay = f64::from_bits(read_u64(&pool_data, DECAY_OFFSET)?);
    let privileged_swapper_option = pool_data
        .get(PRIVILEGED_SWAPPER_OPTION_OFFSET)
        .copied()
        .ok_or(ArbitrageError::InvalidAccount)?;
    let privileged_swapper_is_valid = match privileged_swapper_option {
        0 => true,
        1 => read_pubkey(&pool_data, PRIVILEGED_SWAPPER_OPTION_OFFSET + 1)? != Pubkey::default(),
        _ => false,
    };
    require!(
        read_u128(&pool_data, RESERVE_A_OFFSET)? > 0
            && read_u128(&pool_data, RESERVE_B_OFFSET)? > 0
            && read_u128(&pool_data, SHIFT_OFFSET)? > 0
            && read_u64(&pool_data, NORMALIZATION_PERIOD_OFFSET)? > 0
            && decay.is_finite()
            && decay > 0.0
            && read_u16(&pool_data, ROYALTIES_BPS_OFFSET)? <= 10_000
            && privileged_swapper_is_valid,
        ArbitrageError::InvalidAccount
    );
    drop(pool_data);

    let (expected_pool, expected_bump) = Pubkey::find_program_address(
        &[
            b"pool",
            step[2].key.as_ref(),
            mint_a.key.as_ref(),
            mint_b.key.as_ref(),
        ],
        &VERTIGO_PROGRAM_ID,
    );
    require_keys_eq!(step[1].key(), expected_pool, ArbitrageError::InvalidAccount);
    require!(bump == expected_bump, ArbitrageError::InvalidAccount);
    let (expected_vault_a, _) = Pubkey::find_program_address(
        &[step[1].key.as_ref(), mint_a.key.as_ref()],
        &VERTIGO_PROGRAM_ID,
    );
    let (expected_vault_b, _) = Pubkey::find_program_address(
        &[step[1].key.as_ref(), mint_b.key.as_ref()],
        &VERTIGO_PROGRAM_ID,
    );
    require_keys_eq!(
        step[3].key(),
        expected_vault_a,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        expected_vault_b,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(&step[3], mint_a, token_program_a, &step[1])?;
    validate_token_account_for_mint_and_authority(&step[4], mint_b, token_program_b, &step[1])
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

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let bytes: [u8; 16] = data
        .get(offset..offset + 16)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u128::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_swap_wire_constants_are_pinned() {
        assert_eq!(BUY_DISCRIMINATOR, [102, 6, 61, 18, 1, 218, 235, 234]);
        assert_eq!(SELL_DISCRIMINATOR, [51, 230, 133, 164, 1, 127, 131, 173]);
        assert_eq!(POOL_ACCOUNT_LEN, 229);
        assert_eq!(VERTIGO_STEP_ACCOUNTS, 7);
    }
}
