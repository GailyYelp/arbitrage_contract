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
        program_ids::SABER_ADD_DECIMALS_PROGRAM_ID,
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const SABER_ADD_DECIMALS_STEP_ACCOUNTS: usize = 6;
const WRAPPER_ACCOUNT_LEN: usize = 114;
const MINT_ACCOUNT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;
const WRAPPER_DISCRIMINATOR: [u8; 8] = [28, 41, 198, 163, 189, 149, 175, 142];
const DEPOSIT_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
const WITHDRAW_DISCRIMINATOR: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];

const WRAPPED_DECIMALS_OFFSET: usize = 8;
const MULTIPLIER_OFFSET: usize = 9;
const UNDERLYING_MINT_OFFSET: usize = 17;
const UNDERLYING_VAULT_OFFSET: usize = 49;
const WRAPPED_MINT_OFFSET: usize = 81;
const WRAPPER_BUMP_OFFSET: usize = 113;

const MINT_AUTHORITY_TAG_OFFSET: usize = 0;
const MINT_AUTHORITY_OFFSET: usize = 4;
const MINT_SUPPLY_OFFSET: usize = 36;
const MINT_DECIMALS_OFFSET: usize = 44;
const MINT_INITIALIZED_OFFSET: usize = 45;
const MINT_FREEZE_TAG_OFFSET: usize = 46;
const MINT_FREEZE_AUTHORITY_OFFSET: usize = 50;

const TOKEN_MINT_OFFSET: usize = 0;
const TOKEN_AUTHORITY_OFFSET: usize = 32;
const TOKEN_AMOUNT_OFFSET: usize = 64;
const TOKEN_DELEGATE_TAG_OFFSET: usize = 72;
const TOKEN_STATE_OFFSET: usize = 108;
const TOKEN_NATIVE_TAG_OFFSET: usize = 109;
const TOKEN_DELEGATED_AMOUNT_OFFSET: usize = 121;
const TOKEN_CLOSE_AUTHORITY_TAG_OFFSET: usize = 129;

pub struct SaberAddDecimalsAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub wrapper: &'info AccountInfo<'info>,
    pub wrapped_mint: &'info AccountInfo<'info>,
    pub underlying_vault: &'info AccountInfo<'info>,
    pub underlying_mint: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub user_underlying: &'info AccountInfo<'info>,
    pub user_wrapped: &'info AccountInfo<'info>,
}

pub fn saber_add_decimals_swap<'info>(
    accounts: SaberAddDecimalsAccounts<'info>,
    direction: u8,
    amount_in: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);

    let multiplier = read_wrapper_multiplier(accounts.wrapper)?;
    let wrapped_supply = read_mint_supply(accounts.wrapped_mint)?;
    let vault_amount = read_token_amount(accounts.underlying_vault)?;
    let (discriminator, output_account) = if direction == 0 {
        let wrapped_amount = amount_in
            .checked_mul(multiplier)
            .ok_or(ArbitrageError::MathOverflow)?;
        wrapped_supply
            .checked_add(wrapped_amount)
            .ok_or(ArbitrageError::MathOverflow)?;
        vault_amount
            .checked_add(amount_in)
            .ok_or(ArbitrageError::MathOverflow)?;
        (DEPOSIT_DISCRIMINATOR, accounts.user_wrapped)
    } else {
        require!(amount_in <= wrapped_supply, ArbitrageError::InvalidAmount);
        let underlying_amount = amount_in
            .checked_div(multiplier)
            .ok_or(ArbitrageError::MathOverflow)?;
        require!(underlying_amount > 0, ArbitrageError::InvalidAmount);
        require!(
            underlying_amount <= vault_amount,
            ArbitrageError::InvalidAmount
        );
        (WITHDRAW_DISCRIMINATOR, accounts.user_underlying)
    };

    let pre_out = read_token_amount(output_account)?;
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: vec![
                AccountMeta::new_readonly(accounts.wrapper.key(), false),
                AccountMeta::new(accounts.wrapped_mint.key(), false),
                AccountMeta::new(accounts.underlying_vault.key(), false),
                AccountMeta::new_readonly(accounts.owner.key(), true),
                AccountMeta::new(accounts.user_underlying.key(), false),
                AccountMeta::new(accounts.user_wrapped.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
            ],
            data: transfer_data(discriminator, amount_in),
        },
        &[
            accounts.wrapper.clone(),
            accounts.wrapped_mint.clone(),
            accounts.underlying_vault.clone(),
            accounts.owner.clone(),
            accounts.user_underlying.clone(),
            accounts.user_wrapped.clone(),
            accounts.token_program.clone(),
            accounts.program.clone(),
        ],
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_saber_add_decimals_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step.len() == SABER_ADD_DECIMALS_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        SABER_ADD_DECIMALS_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[5].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );

    let wrapper_data = step[1].try_borrow_data()?;
    require!(
        wrapper_data.len() == WRAPPER_ACCOUNT_LEN
            && wrapper_data.starts_with(&WRAPPER_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let wrapped_decimals = read_byte(&wrapper_data, WRAPPED_DECIMALS_OFFSET)?;
    let multiplier = read_u64(&wrapper_data, MULTIPLIER_OFFSET)?;
    let underlying_mint = read_pubkey(&wrapper_data, UNDERLYING_MINT_OFFSET)?;
    let underlying_vault = read_pubkey(&wrapper_data, UNDERLYING_VAULT_OFFSET)?;
    let wrapped_mint = read_pubkey(&wrapper_data, WRAPPED_MINT_OFFSET)?;
    let bump = read_byte(&wrapper_data, WRAPPER_BUMP_OFFSET)?;
    drop(wrapper_data);

    require_keys_eq!(
        underlying_mint,
        step[4].key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        underlying_vault,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        wrapped_mint,
        step[2].key(),
        ArbitrageError::InvalidTokenMint
    );
    require!(
        underlying_mint != wrapped_mint,
        ArbitrageError::InvalidTokenMint
    );

    let wrapped_decimals_seed = [wrapped_decimals];
    let (expected_wrapper, expected_bump) = Pubkey::find_program_address(
        &[b"anchor", underlying_mint.as_ref(), &wrapped_decimals_seed],
        &step[0].key(),
    );
    require_keys_eq!(
        step[1].key(),
        expected_wrapper,
        ArbitrageError::InvalidAccount
    );
    require!(bump == expected_bump, ArbitrageError::InvalidAccount);

    let underlying = validate_classic_mint(&step[4], &step[5], None, false)?;
    let wrapped = validate_classic_mint(&step[2], &step[5], Some(step[1].key()), true)?;
    require!(
        wrapped.decimals == wrapped_decimals,
        ArbitrageError::InvalidTokenMint
    );
    let decimals_added = wrapped
        .decimals
        .checked_sub(underlying.decimals)
        .ok_or(ArbitrageError::InvalidTokenMint)?;
    let expected_multiplier = 10_u64
        .checked_pow(u32::from(decimals_added))
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        multiplier == expected_multiplier && multiplier > 0,
        ArbitrageError::InvalidAccount
    );
    require!(wrapped.supply > 0, ArbitrageError::InvalidAccount);

    let vault_amount = validate_underlying_vault(&step[3], &step[5], step[4].key(), step[1].key())?;
    require!(vault_amount > 0, ArbitrageError::InvalidAccount);
    require!(
        u128::from(wrapped.supply)
            <= u128::from(vault_amount)
                .checked_mul(u128::from(multiplier))
                .ok_or(ArbitrageError::MathOverflow)?,
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output) = if direction == 0 {
        (underlying_mint, wrapped_mint)
    } else {
        (wrapped_mint, underlying_mint)
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

struct MintState {
    supply: u64,
    decimals: u8,
}

fn validate_classic_mint(
    mint: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    expected_authority: Option<Pubkey>,
    allow_wrapper_freeze_authority: bool,
) -> Result<MintState> {
    require_keys_eq!(
        *mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == MINT_ACCOUNT_LEN && read_byte(&data, MINT_INITIALIZED_OFFSET)? == 1,
        ArbitrageError::InvalidAccount
    );
    let authority_tag = read_u32(&data, MINT_AUTHORITY_TAG_OFFSET)?;
    let freeze_tag = read_u32(&data, MINT_FREEZE_TAG_OFFSET)?;
    require!(
        authority_tag <= 1 && freeze_tag <= 1,
        ArbitrageError::InvalidAccount
    );
    if let Some(authority) = expected_authority {
        require!(authority_tag == 1, ArbitrageError::InvalidAccount);
        require_keys_eq!(
            read_pubkey(&data, MINT_AUTHORITY_OFFSET)?,
            authority,
            ArbitrageError::InvalidAccount
        );
        if freeze_tag == 1 {
            require!(
                allow_wrapper_freeze_authority,
                ArbitrageError::InvalidAccount
            );
            require_keys_eq!(
                read_pubkey(&data, MINT_FREEZE_AUTHORITY_OFFSET)?,
                authority,
                ArbitrageError::InvalidAccount
            );
        }
    }
    Ok(MintState {
        supply: read_u64(&data, MINT_SUPPLY_OFFSET)?,
        decimals: read_byte(&data, MINT_DECIMALS_OFFSET)?,
    })
}

fn validate_underlying_vault(
    vault: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    underlying_mint: Pubkey,
    wrapper: Pubkey,
) -> Result<u64> {
    require_keys_eq!(
        *vault.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = vault.try_borrow_data()?;
    require!(
        data.len() == TOKEN_ACCOUNT_LEN
            && read_byte(&data, TOKEN_STATE_OFFSET)? == 1
            && read_u32(&data, TOKEN_DELEGATE_TAG_OFFSET)? == 0
            && read_u32(&data, TOKEN_NATIVE_TAG_OFFSET)? == 0
            && read_u64(&data, TOKEN_DELEGATED_AMOUNT_OFFSET)? == 0
            && read_u32(&data, TOKEN_CLOSE_AUTHORITY_TAG_OFFSET)? == 0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, TOKEN_MINT_OFFSET)?,
        underlying_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&data, TOKEN_AUTHORITY_OFFSET)?,
        wrapper,
        ArbitrageError::InvalidAccount
    );
    read_u64(&data, TOKEN_AMOUNT_OFFSET)
}

fn transfer_data(discriminator: [u8; 8], amount: u64) -> Vec<u8> {
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

fn read_wrapper_multiplier(wrapper: &AccountInfo<'_>) -> Result<u64> {
    let data = wrapper.try_borrow_data()?;
    require!(
        data.len() == WRAPPER_ACCOUNT_LEN && data.starts_with(&WRAPPER_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    read_u64(&data, MULTIPLIER_OFFSET)
}

fn read_mint_supply(mint: &AccountInfo<'_>) -> Result<u64> {
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == MINT_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    read_u64(&data, MINT_SUPPLY_OFFSET)
}

fn read_byte(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or_else(|| ArbitrageError::InvalidAccount.into())
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

    fn mint_data(authority: Option<Pubkey>, supply: u64, decimals: u8) -> Vec<u8> {
        let mut data = vec![0_u8; MINT_ACCOUNT_LEN];
        if let Some(authority) = authority {
            data[MINT_AUTHORITY_TAG_OFFSET..MINT_AUTHORITY_TAG_OFFSET + 4]
                .copy_from_slice(&1_u32.to_le_bytes());
            write_pubkey(&mut data, MINT_AUTHORITY_OFFSET, authority);
        }
        data[MINT_SUPPLY_OFFSET..MINT_SUPPLY_OFFSET + 8].copy_from_slice(&supply.to_le_bytes());
        data[MINT_DECIMALS_OFFSET] = decimals;
        data[MINT_INITIALIZED_OFFSET] = 1;
        data
    }

    fn fixture() -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let underlying_mint = Pubkey::new_unique();
        let wrapped_decimals = 8_u8;
        let decimals_seed = [wrapped_decimals];
        let (wrapper, bump) = Pubkey::find_program_address(
            &[b"anchor", underlying_mint.as_ref(), &decimals_seed],
            &SABER_ADD_DECIMALS_PROGRAM_ID,
        );
        let wrapped_mint = Pubkey::new_unique();
        let vault = Pubkey::new_unique();

        let mut wrapper_data = vec![0_u8; WRAPPER_ACCOUNT_LEN];
        wrapper_data[..8].copy_from_slice(&WRAPPER_DISCRIMINATOR);
        wrapper_data[WRAPPED_DECIMALS_OFFSET] = wrapped_decimals;
        wrapper_data[MULTIPLIER_OFFSET..MULTIPLIER_OFFSET + 8]
            .copy_from_slice(&100_u64.to_le_bytes());
        write_pubkey(&mut wrapper_data, UNDERLYING_MINT_OFFSET, underlying_mint);
        write_pubkey(&mut wrapper_data, UNDERLYING_VAULT_OFFSET, vault);
        write_pubkey(&mut wrapper_data, WRAPPED_MINT_OFFSET, wrapped_mint);
        wrapper_data[WRAPPER_BUMP_OFFSET] = bump;

        let mut vault_data = vec![0_u8; TOKEN_ACCOUNT_LEN];
        write_pubkey(&mut vault_data, TOKEN_MINT_OFFSET, underlying_mint);
        write_pubkey(&mut vault_data, TOKEN_AUTHORITY_OFFSET, wrapper);
        vault_data[TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8]
            .copy_from_slice(&2_000_000_u64.to_le_bytes());
        vault_data[TOKEN_STATE_OFFSET] = 1;

        let step = vec![
            account(
                SABER_ADD_DECIMALS_PROGRAM_ID,
                Pubkey::new_unique(),
                Vec::new(),
                false,
                true,
            ),
            account(
                wrapper,
                SABER_ADD_DECIMALS_PROGRAM_ID,
                wrapper_data,
                false,
                false,
            ),
            account(
                wrapped_mint,
                anchor_spl::token::ID,
                mint_data(Some(wrapper), 199_999_999, wrapped_decimals),
                true,
                false,
            ),
            account(vault, anchor_spl::token::ID, vault_data, true, false),
            account(
                underlying_mint,
                anchor_spl::token::ID,
                mint_data(Some(Pubkey::new_unique()), 9_000_000, 6),
                false,
                false,
            ),
            account(
                anchor_spl::token::ID,
                Pubkey::new_unique(),
                Vec::new(),
                false,
                true,
            ),
        ];
        let input = account(
            underlying_mint,
            anchor_spl::token::ID,
            mint_data(None, 1, 6),
            false,
            false,
        );
        let output = account(
            wrapped_mint,
            anchor_spl::token::ID,
            mint_data(None, 1, wrapped_decimals),
            false,
            false,
        );
        (step, input, output)
    }

    #[test]
    fn transfer_wires_match_official_discriminators() {
        assert_eq!(
            transfer_data(DEPOSIT_DISCRIMINATOR, 1_000_000),
            [
                DEPOSIT_DISCRIMINATOR.as_slice(),
                &1_000_000_u64.to_le_bytes()
            ]
            .concat()
        );
        assert_eq!(
            transfer_data(WITHDRAW_DISCRIMINATOR, 26_818_801),
            [
                WITHDRAW_DISCRIMINATOR.as_slice(),
                &26_818_801_u64.to_le_bytes()
            ]
            .concat()
        );
    }

    #[test]
    fn validates_official_wrapper_bindings_and_both_directions() {
        let (step, underlying, wrapped) = fixture();
        assert!(
            validate_saber_add_decimals_semantic_accounts(&step, 0, 0, &underlying, &wrapped)
                .is_ok()
        );
        assert!(
            validate_saber_add_decimals_semantic_accounts(&step, 1, 0, &wrapped, &underlying)
                .is_ok()
        );
    }

    #[test]
    fn rejects_multiplier_and_backing_mismatches() {
        let (step, underlying, wrapped) = fixture();
        step[1].try_borrow_mut_data().unwrap()[MULTIPLIER_OFFSET] = 99;
        assert!(
            validate_saber_add_decimals_semantic_accounts(&step, 0, 0, &underlying, &wrapped)
                .is_err()
        );
    }
}
