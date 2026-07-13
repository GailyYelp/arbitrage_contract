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
        program_ids::{HELIUM_CIRCUIT_BREAKER_PROGRAM_ID, HELIUM_TREASURY_MANAGEMENT_PROGRAM_ID},
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const HELIUM_TREASURY_MANAGEMENT_STEP_ACCOUNTS: usize = 8;
const TREASURY_MANAGEMENT_ACCOUNT_LEN: usize = 228;
const CIRCUIT_BREAKER_ACCOUNT_LEN: usize = 212;
const TREASURY_MANAGEMENT_DISCRIMINATOR: [u8; 8] = [68, 111, 120, 209, 16, 110, 215, 59];
const CIRCUIT_BREAKER_DISCRIMINATOR: [u8; 8] = [134, 11, 69, 100, 90, 132, 174, 187];
const REDEEM_DISCRIMINATOR: [u8; 8] = [235, 127, 171, 139, 119, 77, 235, 118];

const TREASURY_MINT_OFFSET: usize = 8;
const SUPPLY_MINT_OFFSET: usize = 40;
const AUTHORITY_OFFSET: usize = 72;
const TREASURY_OFFSET: usize = 104;
const CURVE_TAG_OFFSET: usize = 136;
const CURVE_K_OFFSET: usize = 137;
const FREEZE_UNIX_TIME_OFFSET: usize = 153;
const TREASURY_BUMP_OFFSET: usize = 161;

const BREAKER_TOKEN_ACCOUNT_OFFSET: usize = 8;
const BREAKER_AUTHORITY_OFFSET: usize = 40;
const BREAKER_OWNER_OFFSET: usize = 72;
const BREAKER_WINDOW_SECONDS_OFFSET: usize = 104;
const BREAKER_THRESHOLD_TYPE_OFFSET: usize = 112;
const BREAKER_THRESHOLD_OFFSET: usize = 113;
const BREAKER_LAST_AGGREGATED_VALUE_OFFSET: usize = 121;
const BREAKER_LAST_UNIX_TIMESTAMP_OFFSET: usize = 129;
const BREAKER_BUMP_OFFSET: usize = 137;

pub struct HeliumTreasuryManagementAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub treasury_management: &'info AccountInfo<'info>,
    pub treasury_mint: &'info AccountInfo<'info>,
    pub supply_mint: &'info AccountInfo<'info>,
    pub treasury: &'info AccountInfo<'info>,
    pub circuit_breaker: &'info AccountInfo<'info>,
    pub circuit_breaker_program: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub user_supply: &'info AccountInfo<'info>,
    pub user_treasury: &'info AccountInfo<'info>,
}

pub fn helium_treasury_management_swap<'info>(
    accounts: HeliumTreasuryManagementAccounts<'info>,
    direction: u8,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    require!(direction == 0, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(
        amount_in <= read_mint_supply(accounts.supply_mint)?,
        ArbitrageError::InvalidAmount
    );
    let pre_out = read_token_amount(accounts.user_treasury)?;
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: vec![
                AccountMeta::new_readonly(accounts.treasury_management.key(), false),
                AccountMeta::new_readonly(accounts.treasury_mint.key(), false),
                AccountMeta::new(accounts.supply_mint.key(), false),
                AccountMeta::new(accounts.treasury.key(), false),
                AccountMeta::new(accounts.circuit_breaker.key(), false),
                AccountMeta::new(accounts.user_supply.key(), false),
                AccountMeta::new(accounts.user_treasury.key(), false),
                AccountMeta::new_readonly(accounts.owner.key(), true),
                AccountMeta::new_readonly(accounts.circuit_breaker_program.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
            ],
            data: redeem_data(amount_in, min_amount_out),
        },
        &[
            accounts.treasury_management.clone(),
            accounts.treasury_mint.clone(),
            accounts.supply_mint.clone(),
            accounts.treasury.clone(),
            accounts.circuit_breaker.clone(),
            accounts.user_supply.clone(),
            accounts.user_treasury.clone(),
            accounts.owner.clone(),
            accounts.circuit_breaker_program.clone(),
            accounts.token_program.clone(),
            accounts.program.clone(),
        ],
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_treasury, pre_out)?,
        fee_amount: 0,
    })
}

fn redeem_data(amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = REDEEM_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

pub fn validate_helium_treasury_management_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    validate_helium_treasury_management_semantic_accounts_at(
        step,
        direction,
        fee_rate,
        input_mint,
        output_mint,
        Clock::get()?.unix_timestamp,
    )
}

fn validate_helium_treasury_management_semantic_accounts_at(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
    now: i64,
) -> Result<()> {
    require!(
        step.len() == HELIUM_TREASURY_MANAGEMENT_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction == 0, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        HELIUM_TREASURY_MANAGEMENT_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[6].key(),
        HELIUM_CIRCUIT_BREAKER_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[7].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[5].owner,
        step[6].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        input_mint.key(),
        step[3].key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output_mint.key(),
        step[2].key(),
        ArbitrageError::InvalidTokenMint
    );

    let treasury_state = step[1].try_borrow_data()?;
    require!(
        treasury_state.len() == TREASURY_MANAGEMENT_ACCOUNT_LEN
            && treasury_state.starts_with(&TREASURY_MANAGEMENT_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let treasury_mint = read_pubkey(&treasury_state, TREASURY_MINT_OFFSET)?;
    let supply_mint = read_pubkey(&treasury_state, SUPPLY_MINT_OFFSET)?;
    let authority = read_pubkey(&treasury_state, AUTHORITY_OFFSET)?;
    let treasury = read_pubkey(&treasury_state, TREASURY_OFFSET)?;
    let curve_tag = read_byte(&treasury_state, CURVE_TAG_OFFSET)?;
    let curve_k = read_u128(&treasury_state, CURVE_K_OFFSET)?;
    let freeze_unix_time = read_i64(&treasury_state, FREEZE_UNIX_TIME_OFFSET)?;
    let treasury_bump = read_byte(&treasury_state, TREASURY_BUMP_OFFSET)?;
    drop(treasury_state);

    require_keys_eq!(
        treasury_mint,
        step[2].key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(supply_mint, step[3].key(), ArbitrageError::InvalidTokenMint);
    require_keys_eq!(treasury, step[4].key(), ArbitrageError::InvalidAccount);
    require!(
        authority != Pubkey::default() && curve_tag == 0 && curve_k == 0 && freeze_unix_time > now,
        ArbitrageError::InvalidAccount
    );
    let (expected_treasury_management, expected_treasury_bump) = Pubkey::find_program_address(
        &[b"treasury_management", step[3].key().as_ref()],
        &step[0].key(),
    );
    require_keys_eq!(
        step[1].key(),
        expected_treasury_management,
        ArbitrageError::InvalidAccount
    );
    require!(
        treasury_bump == expected_treasury_bump,
        ArbitrageError::InvalidAccount
    );

    let (expected_breaker, expected_breaker_bump) = Pubkey::find_program_address(
        &[b"account_windowed_breaker", step[4].key().as_ref()],
        &step[6].key(),
    );
    require_keys_eq!(
        step[5].key(),
        expected_breaker,
        ArbitrageError::InvalidAccount
    );
    let breaker = step[5].try_borrow_data()?;
    require!(
        breaker.len() == CIRCUIT_BREAKER_ACCOUNT_LEN
            && breaker.starts_with(&CIRCUIT_BREAKER_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&breaker, BREAKER_TOKEN_ACCOUNT_OFFSET)?,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&breaker, BREAKER_AUTHORITY_OFFSET)?,
        authority,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&breaker, BREAKER_OWNER_OFFSET)?,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    let window_seconds = read_u64(&breaker, BREAKER_WINDOW_SECONDS_OFFSET)?;
    let threshold_type = read_byte(&breaker, BREAKER_THRESHOLD_TYPE_OFFSET)?;
    let threshold = read_u64(&breaker, BREAKER_THRESHOLD_OFFSET)?;
    let _last_aggregated_value = read_u64(&breaker, BREAKER_LAST_AGGREGATED_VALUE_OFFSET)?;
    let last_unix_timestamp = read_i64(&breaker, BREAKER_LAST_UNIX_TIMESTAMP_OFFSET)?;
    let breaker_bump = read_byte(&breaker, BREAKER_BUMP_OFFSET)?;
    require!(
        window_seconds > 0
            && threshold_type <= 1
            && threshold > 0
            && last_unix_timestamp >= 0
            && last_unix_timestamp <= now
            && breaker_bump == expected_breaker_bump,
        ArbitrageError::InvalidAccount
    );
    drop(breaker);

    let treasury_supply = validate_classic_mint(&step[2], false)?;
    let supply = validate_classic_mint(&step[3], true)?;
    require!(
        treasury_supply > 0 && supply > 0,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(&step[4], &step[2], &step[7], &step[5])?;
    require!(
        read_token_amount(&step[4])? > 0,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_classic_mint(mint: &AccountInfo<'_>, require_no_authority: bool) -> Result<u64> {
    require_keys_eq!(
        *mint.owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidAccount
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == 82 && read_byte(&data, 45)? == 1,
        ArbitrageError::InvalidAccount
    );
    if require_no_authority {
        require!(read_u32(&data, 0)? == 0, ArbitrageError::InvalidAccount);
    }
    read_u64(&data, 36)
}

fn read_mint_supply(mint: &AccountInfo<'_>) -> Result<u64> {
    let data = mint.try_borrow_data()?;
    require!(data.len() == 82, ArbitrageError::InvalidAccount);
    read_u64(&data, 36)
}

fn read_byte(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or_else(|| ArbitrageError::InvalidAccount.into())
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_i64(data: &[u8], offset: usize) -> Result<i64> {
    Ok(i64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    Ok(u128::from_le_bytes(
        data.get(offset..offset + 16)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(
        key: Pubkey,
        owner: Pubkey,
        data: Vec<u8>,
        writable: bool,
        executable: bool,
    ) -> AccountInfo<'static> {
        AccountInfo::new(
            Box::leak(Box::new(key)),
            false,
            writable,
            Box::leak(Box::new(1_u64)),
            Box::leak(data.into_boxed_slice()),
            Box::leak(Box::new(owner)),
            executable,
            0,
        )
    }

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + 32].copy_from_slice(key.as_ref());
    }

    fn mint_data(authority: Option<Pubkey>, supply: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 82];
        if let Some(authority) = authority {
            data[..4].copy_from_slice(&1_u32.to_le_bytes());
            write_pubkey(&mut data, 4, authority);
        }
        data[36..44].copy_from_slice(&supply.to_le_bytes());
        data[44] = 8;
        data[45] = 1;
        data
    }

    fn token_data(mint: Pubkey, authority: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 165];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1;
        data
    }

    fn semantic_fixture() -> Vec<AccountInfo<'static>> {
        let treasury_mint = Pubkey::new_from_array([2; 32]);
        let supply_mint = Pubkey::new_from_array([3; 32]);
        let authority = Pubkey::new_from_array([4; 32]);
        let (treasury_management, treasury_bump) = Pubkey::find_program_address(
            &[b"treasury_management", supply_mint.as_ref()],
            &HELIUM_TREASURY_MANAGEMENT_PROGRAM_ID,
        );
        let treasury = Pubkey::find_program_address(
            &[
                treasury_management.as_ref(),
                anchor_spl::token::ID.as_ref(),
                treasury_mint.as_ref(),
            ],
            &anchor_spl::associated_token::ID,
        )
        .0;
        let (breaker, breaker_bump) = Pubkey::find_program_address(
            &[b"account_windowed_breaker", treasury.as_ref()],
            &HELIUM_CIRCUIT_BREAKER_PROGRAM_ID,
        );

        let mut treasury_state = vec![0_u8; TREASURY_MANAGEMENT_ACCOUNT_LEN];
        treasury_state[..8].copy_from_slice(&TREASURY_MANAGEMENT_DISCRIMINATOR);
        write_pubkey(&mut treasury_state, TREASURY_MINT_OFFSET, treasury_mint);
        write_pubkey(&mut treasury_state, SUPPLY_MINT_OFFSET, supply_mint);
        write_pubkey(&mut treasury_state, AUTHORITY_OFFSET, authority);
        write_pubkey(&mut treasury_state, TREASURY_OFFSET, treasury);
        treasury_state[CURVE_TAG_OFFSET] = 0;
        treasury_state[CURVE_K_OFFSET..CURVE_K_OFFSET + 16].copy_from_slice(&0_u128.to_le_bytes());
        treasury_state[FREEZE_UNIX_TIME_OFFSET..FREEZE_UNIX_TIME_OFFSET + 8]
            .copy_from_slice(&i64::MAX.to_le_bytes());
        treasury_state[TREASURY_BUMP_OFFSET] = treasury_bump;

        let mut breaker_state = vec![0_u8; CIRCUIT_BREAKER_ACCOUNT_LEN];
        breaker_state[..8].copy_from_slice(&CIRCUIT_BREAKER_DISCRIMINATOR);
        write_pubkey(&mut breaker_state, BREAKER_TOKEN_ACCOUNT_OFFSET, treasury);
        write_pubkey(&mut breaker_state, BREAKER_AUTHORITY_OFFSET, authority);
        write_pubkey(
            &mut breaker_state,
            BREAKER_OWNER_OFFSET,
            treasury_management,
        );
        breaker_state[BREAKER_WINDOW_SECONDS_OFFSET..BREAKER_WINDOW_SECONDS_OFFSET + 8]
            .copy_from_slice(&86_400_u64.to_le_bytes());
        breaker_state[BREAKER_THRESHOLD_TYPE_OFFSET] = 0;
        breaker_state[BREAKER_THRESHOLD_OFFSET..BREAKER_THRESHOLD_OFFSET + 8]
            .copy_from_slice(&3_689_348_814_741_910_323_u64.to_le_bytes());
        breaker_state
            [BREAKER_LAST_AGGREGATED_VALUE_OFFSET..BREAKER_LAST_AGGREGATED_VALUE_OFFSET + 8]
            .copy_from_slice(&1_000_u64.to_le_bytes());
        breaker_state[BREAKER_LAST_UNIX_TIMESTAMP_OFFSET..BREAKER_LAST_UNIX_TIMESTAMP_OFFSET + 8]
            .copy_from_slice(&1_700_000_000_i64.to_le_bytes());
        breaker_state[BREAKER_BUMP_OFFSET] = breaker_bump;

        vec![
            account(
                HELIUM_TREASURY_MANAGEMENT_PROGRAM_ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(
                treasury_management,
                HELIUM_TREASURY_MANAGEMENT_PROGRAM_ID,
                treasury_state,
                false,
                false,
            ),
            account(
                treasury_mint,
                anchor_spl::token::ID,
                mint_data(Some(Pubkey::new_unique()), 500_000_000_000),
                false,
                false,
            ),
            account(
                supply_mint,
                anchor_spl::token::ID,
                mint_data(None, 20_000_000_000_000),
                true,
                false,
            ),
            account(
                treasury,
                anchor_spl::token::ID,
                token_data(treasury_mint, breaker, 300_000_000_000),
                true,
                false,
            ),
            account(
                breaker,
                HELIUM_CIRCUIT_BREAKER_PROGRAM_ID,
                breaker_state,
                true,
                false,
            ),
            account(
                HELIUM_CIRCUIT_BREAKER_PROGRAM_ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(
                anchor_spl::token::ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
        ]
    }

    #[test]
    fn redeem_wire_matches_official_instruction() {
        let data = redeem_data(3_133_591_059, 47_419_626);
        assert_eq!(data.len(), 24);
        assert_eq!(&data[..8], &REDEEM_DISCRIMINATOR);
        assert_eq!(&data[8..16], &3_133_591_059_u64.to_le_bytes());
        assert_eq!(&data[16..24], &47_419_626_u64.to_le_bytes());
    }

    #[test]
    fn semantic_validation_binds_state_breaker_pdas_and_linear_subset() {
        let step = semantic_fixture();
        assert!(validate_helium_treasury_management_semantic_accounts_at(
            &step,
            0,
            0,
            &step[3],
            &step[2],
            1_700_000_100,
        )
        .is_ok());

        let step = semantic_fixture();
        step[1].try_borrow_mut_data().unwrap()[CURVE_K_OFFSET] = 1;
        assert!(validate_helium_treasury_management_semantic_accounts_at(
            &step,
            0,
            0,
            &step[3],
            &step[2],
            1_700_000_100,
        )
        .is_err());

        let step = semantic_fixture();
        step[5].try_borrow_mut_data().unwrap()[BREAKER_AUTHORITY_OFFSET] ^= 1;
        assert!(validate_helium_treasury_management_semantic_accounts_at(
            &step,
            0,
            0,
            &step[3],
            &step[2],
            1_700_000_100,
        )
        .is_err());
    }
}
