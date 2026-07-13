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
        program_ids::HUMA_PROGRAM_ID,
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const HUMA_STEP_ACCOUNTS: usize = 15;
const POOL_CONFIG_DISCRIMINATOR: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];
const POOL_STATE_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];
const MODE_CONFIG_DISCRIMINATOR: [u8; 8] = [249, 180, 144, 225, 126, 159, 202, 209];
const LENDER_STATE_DISCRIMINATOR: [u8; 8] = [240, 118, 235, 226, 18, 3, 58, 25];
const DEPLOYMENT_CONFIG_DISCRIMINATOR: [u8; 8] = [13, 112, 57, 81, 43, 26, 156, 18];
const DEPLOYMENT_STATE_DISCRIMINATOR: [u8; 8] = [212, 136, 79, 121, 16, 38, 112, 116];
const DEPOSIT_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
const INSTANT_WITHDRAW_DISCRIMINATOR: [u8; 8] = [171, 49, 145, 176, 48, 101, 112, 162];

pub struct HumaAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub huma_config: &'info AccountInfo<'info>,
    pub pool_config: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub mode_config: &'info AccountInfo<'info>,
    pub mode_mint: &'info AccountInfo<'info>,
    pub deployment_config: &'info AccountInfo<'info>,
    pub deployment_state: &'info AccountInfo<'info>,
    pub lender_state: &'info AccountInfo<'info>,
    pub underlying_mint: &'info AccountInfo<'info>,
    pub pool_authority: &'info AccountInfo<'info>,
    pub pool_underlying_token: &'info AccountInfo<'info>,
    pub treasury_underlying_token: &'info AccountInfo<'info>,
    pub underlying_token_program: &'info AccountInfo<'info>,
    pub mode_token_program: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_underlying: &'info AccountInfo<'info>,
    pub user_mode: &'info AccountInfo<'info>,
}

pub fn huma_swap<'info>(
    accounts: HumaAccounts<'info>,
    direction: u8,
    amount_in: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let output = if direction == 0 {
        accounts.user_mode
    } else {
        accounts.user_underlying
    };
    let pre_out = read_token_amount(output)?;
    let (metas, infos, data) = if direction == 0 {
        (
            vec![
                AccountMeta::new_readonly(accounts.user.key(), true),
                AccountMeta::new_readonly(accounts.huma_config.key(), false),
                AccountMeta::new_readonly(accounts.pool_config.key(), false),
                AccountMeta::new(accounts.pool_state.key(), false),
                AccountMeta::new_readonly(accounts.mode_config.key(), false),
                AccountMeta::new(accounts.mode_mint.key(), false),
                AccountMeta::new_readonly(accounts.pool_authority.key(), false),
                AccountMeta::new_readonly(accounts.underlying_mint.key(), false),
                AccountMeta::new(accounts.pool_underlying_token.key(), false),
                AccountMeta::new(accounts.user_underlying.key(), false),
                AccountMeta::new(accounts.user_mode.key(), false),
                AccountMeta::new_readonly(accounts.underlying_token_program.key(), false),
                AccountMeta::new_readonly(accounts.mode_token_program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.huma_config.clone(),
                accounts.pool_config.clone(),
                accounts.pool_state.clone(),
                accounts.mode_config.clone(),
                accounts.mode_mint.clone(),
                accounts.pool_authority.clone(),
                accounts.underlying_mint.clone(),
                accounts.pool_underlying_token.clone(),
                accounts.user_underlying.clone(),
                accounts.user_mode.clone(),
                accounts.underlying_token_program.clone(),
                accounts.mode_token_program.clone(),
                accounts.program.clone(),
            ],
            huma_data(direction, amount_in),
        )
    } else {
        (
            vec![
                AccountMeta::new_readonly(accounts.user.key(), true),
                AccountMeta::new_readonly(accounts.huma_config.key(), false),
                AccountMeta::new_readonly(accounts.pool_config.key(), false),
                AccountMeta::new(accounts.pool_state.key(), false),
                AccountMeta::new_readonly(accounts.mode_config.key(), false),
                AccountMeta::new(accounts.mode_mint.key(), false),
                AccountMeta::new_readonly(accounts.deployment_config.key(), false),
                AccountMeta::new(accounts.deployment_state.key(), false),
                AccountMeta::new(accounts.lender_state.key(), false),
                AccountMeta::new_readonly(accounts.underlying_mint.key(), false),
                AccountMeta::new(accounts.pool_authority.key(), false),
                AccountMeta::new(accounts.pool_underlying_token.key(), false),
                AccountMeta::new(accounts.user_underlying.key(), false),
                AccountMeta::new(accounts.treasury_underlying_token.key(), false),
                AccountMeta::new(accounts.user_mode.key(), false),
                AccountMeta::new_readonly(accounts.underlying_token_program.key(), false),
                AccountMeta::new_readonly(accounts.mode_token_program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.huma_config.clone(),
                accounts.pool_config.clone(),
                accounts.pool_state.clone(),
                accounts.mode_config.clone(),
                accounts.mode_mint.clone(),
                accounts.deployment_config.clone(),
                accounts.deployment_state.clone(),
                accounts.lender_state.clone(),
                accounts.underlying_mint.clone(),
                accounts.pool_authority.clone(),
                accounts.pool_underlying_token.clone(),
                accounts.user_underlying.clone(),
                accounts.treasury_underlying_token.clone(),
                accounts.user_mode.clone(),
                accounts.underlying_token_program.clone(),
                accounts.mode_token_program.clone(),
                accounts.program.clone(),
            ],
            huma_data(direction, amount_in),
        )
    };
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn huma_data(direction: u8, amount_in: u64) -> Vec<u8> {
    let mut data = if direction == 0 {
        DEPOSIT_DISCRIMINATOR.to_vec()
    } else {
        INSTANT_WITHDRAW_DISCRIMINATOR.to_vec()
    };
    data.extend_from_slice(&amount_in.to_le_bytes());
    if direction == 0 {
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.push(0);
    } else {
        data.extend_from_slice(&u64::MAX.to_le_bytes());
    }
    data
}

pub fn validate_huma_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
    user: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step.len() == HUMA_STEP_ACCOUNTS && direction <= 1 && fee_rate == 0,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step[0].key(),
        HUMA_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    for index in [2_usize, 3, 4, 6, 7] {
        require_keys_eq!(
            *step[index].owner,
            HUMA_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
    }
    require_keys_eq!(
        step[13].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[14].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );

    let pool_config = step[2].try_borrow_data()?;
    require!(
        pool_config.starts_with(&POOL_CONFIG_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let huma_config = read_pubkey(&pool_config, 9)?;
    let treasury = read_pubkey(&pool_config, 73)?;
    let underlying_mint = read_pubkey(&pool_config, 105)?;
    let pool_authority_bump = *pool_config.get(137).ok_or(ArbitrageError::InvalidAccount)?;
    let pool_id = read_pubkey(&pool_config, 138)?;
    let name_len =
        usize::try_from(read_u32(&pool_config, 170)?).map_err(|_| ArbitrageError::MathOverflow)?;
    let instant_config_offset = 382_usize
        .checked_add(name_len)
        .ok_or(ArbitrageError::MathOverflow)?;
    let fee_count = usize::try_from(read_u32(&pool_config, instant_config_offset + 8)?)
        .map_err(|_| ArbitrageError::MathOverflow)?;
    require!(
        fee_count > 0 && fee_count <= 32,
        ArbitrageError::InvalidAccount
    );
    let liquidity_source_option = instant_config_offset
        .checked_add(12)
        .and_then(|offset| offset.checked_add(fee_count.checked_mul(164)?))
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        *pool_config
            .get(liquidity_source_option)
            .ok_or(ArbitrageError::InvalidAccount)?
            == 1,
        ArbitrageError::InvalidAccount
    );
    let deployment_config = read_pubkey(&pool_config, liquidity_source_option + 1)?;
    drop(pool_config);

    require_keys_eq!(step[1].key(), huma_config, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[9].key(),
        underlying_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        step[2].key(),
        Pubkey::find_program_address(&[b"pool_config", pool_id.as_ref()], &HUMA_PROGRAM_ID).0,
        ArbitrageError::InvalidAccount
    );
    let (expected_pool_authority, expected_bump) = Pubkey::find_program_address(
        &[b"pool_authority", step[2].key().as_ref()],
        &HUMA_PROGRAM_ID,
    );
    require!(
        expected_bump == pool_authority_bump,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[10].key(),
        expected_pool_authority,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        deployment_config,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[7].key(),
        Pubkey::find_program_address(
            &[b"deployment_state", step[6].key().as_ref()],
            &HUMA_PROGRAM_ID
        )
        .0,
        ArbitrageError::InvalidAccount
    );
    validate_discriminator(&step[6], &DEPLOYMENT_CONFIG_DISCRIMINATOR)?;
    validate_discriminator(&step[7], &DEPLOYMENT_STATE_DISCRIMINATOR)?;

    require_keys_eq!(
        step[3].key(),
        Pubkey::find_program_address(&[b"pool_state", step[2].key().as_ref()], &HUMA_PROGRAM_ID).0,
        ArbitrageError::InvalidAccount
    );
    let mode_config = step[4].try_borrow_data()?;
    require!(
        mode_config.starts_with(&MODE_CONFIG_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let mode_id = read_pubkey(&mode_config, 10)?;
    drop(mode_config);
    require_keys_eq!(
        step[4].key(),
        Pubkey::find_program_address(
            &[b"mode_config", step[2].key().as_ref(), mode_id.as_ref()],
            &HUMA_PROGRAM_ID,
        )
        .0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        Pubkey::find_program_address(
            &[b"mode_mint", step[2].key().as_ref(), step[4].key().as_ref()],
            &HUMA_PROGRAM_ID,
        )
        .0,
        ArbitrageError::InvalidAccount
    );
    validate_mode_mint(&step[5], &step[10])?;

    let pool_state = step[3].try_borrow_data()?;
    require!(
        pool_state.starts_with(&POOL_STATE_DISCRIMINATOR) && pool_state.get(9) == Some(&1),
        ArbitrageError::InvalidAccount
    );
    let mode_count =
        usize::try_from(read_u32(&pool_state, 26)?).map_err(|_| ArbitrageError::MathOverflow)?;
    require!(
        mode_count > 0 && mode_count <= 32,
        ArbitrageError::InvalidAccount
    );
    let keys_count_offset = 30_usize
        .checked_add(
            mode_count
                .checked_mul(216)
                .ok_or(ArbitrageError::MathOverflow)?,
        )
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        usize::try_from(read_u32(&pool_state, keys_count_offset)?)
            .map_err(|_| ArbitrageError::MathOverflow)?
            == mode_count,
        ArbitrageError::InvalidAccount
    );
    let mut found_mode = false;
    for index in 0..mode_count {
        let offset = keys_count_offset
            .checked_add(4)
            .and_then(|value| value.checked_add(index.checked_mul(32)?))
            .ok_or(ArbitrageError::MathOverflow)?;
        if read_pubkey(&pool_state, offset)? == step[4].key() {
            found_mode = true;
            break;
        }
    }
    require!(found_mode, ArbitrageError::InvalidAccount);
    drop(pool_state);

    validate_token_account(&step[11], &step[9], &step[10], &step[13])?;
    let expected_pool_vault = Pubkey::find_program_address(
        &[
            step[10].key().as_ref(),
            step[13].key().as_ref(),
            step[9].key().as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0;
    require_keys_eq!(
        step[11].key(),
        expected_pool_vault,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_key_authority(&step[12], &step[9], treasury, &step[13])?;
    let expected_treasury_vault = Pubkey::find_program_address(
        &[
            treasury.as_ref(),
            step[13].key().as_ref(),
            step[9].key().as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0;
    require_keys_eq!(
        step[12].key(),
        expected_treasury_vault,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        Pubkey::find_program_address(
            &[b"lender_state", step[4].key().as_ref(), user.key().as_ref()],
            &HUMA_PROGRAM_ID,
        )
        .0,
        ArbitrageError::InvalidAccount
    );
    if direction == 1 {
        validate_discriminator(&step[8], &LENDER_STATE_DISCRIMINATOR)?;
    }
    let (expected_input, expected_output) = if direction == 0 {
        (underlying_mint, step[5].key())
    } else {
        (step[5].key(), underlying_mint)
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
    Ok(())
}

fn validate_discriminator(account: &AccountInfo<'_>, discriminator: &[u8; 8]) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.starts_with(discriminator),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_mode_mint(mint: &AccountInfo<'_>, authority: &AccountInfo<'_>) -> Result<()> {
    require_keys_eq!(
        *mint.owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidAccount
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == 82
            && read_u32(&data, 0)? == 1
            && read_pubkey(&data, 4)? == authority.key()
            && read_u64(&data, 36)? > 0
            && data[45] == 1,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_token_account(
    account: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    authority: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
) -> Result<()> {
    validate_token_account_key_authority(account, mint, authority.key(), token_program)
}

fn validate_token_account_key_authority(
    account: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    authority: Pubkey,
    token_program: &AccountInfo<'_>,
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() >= 165
            && read_pubkey(&data, 0)? == mint.key()
            && read_pubkey(&data, 32)? == authority,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(bytes);
    Ok(Pubkey::new_from_array(raw))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_wires_match_deployed_idl() {
        let deposit = huma_data(0, 1_000_000);
        assert_eq!(deposit.len(), 21);
        assert_eq!(&deposit[..8], &DEPOSIT_DISCRIMINATOR);
        assert_eq!(&deposit[16..], &[0, 0, 0, 0, 0]);
        let withdraw = huma_data(1, 958_336);
        assert_eq!(withdraw.len(), 24);
        assert_eq!(&withdraw[..8], &INSTANT_WITHDRAW_DISCRIMINATOR);
        assert_eq!(&withdraw[16..], &u64::MAX.to_le_bytes());
    }
}
