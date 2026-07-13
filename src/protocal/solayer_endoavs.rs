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
        program_ids::SOLAYER_ENDOAVS_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const SOLAYER_ENDOAVS_STEP_ACCOUNTS: usize = 6;
const ENDOAVS_DISCRIMINATOR: [u8; 8] = [66, 168, 41, 177, 187, 11, 173, 123];
const DELEGATE_NO_INIT_DISCRIMINATOR: [u8; 8] = [254, 0, 222, 40, 145, 118, 110, 20];
const UNDELEGATE_NO_INIT_DISCRIMINATOR: [u8; 8] = [213, 17, 215, 81, 5, 114, 124, 78];

const ENDOAVS_BUMP_OFFSET: usize = 8;
const ENDOAVS_AVS_MINT_OFFSET: usize = 41;
const ENDOAVS_DELEGATED_MINT_OFFSET: usize = 73;
const ENDOAVS_VAULT_OFFSET: usize = 105;

pub struct SolayerEndoAvsAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub endoavs: &'info AccountInfo<'info>,
    pub avs_mint: &'info AccountInfo<'info>,
    pub delegated_vault: &'info AccountInfo<'info>,
    pub delegated_mint: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_delegated: &'info AccountInfo<'info>,
    pub user_avs: &'info AccountInfo<'info>,
}

pub fn solayer_endoavs_swap<'info>(
    accounts: SolayerEndoAvsAccounts<'info>,
    direction: u8,
    amount_in: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);

    let avs_supply = read_mint_supply(accounts.avs_mint)?;
    let vault_amount = read_token_amount(accounts.delegated_vault)?;
    let max_input = if direction == 0 {
        u64::MAX
            .checked_sub(avs_supply)
            .ok_or(ArbitrageError::MathOverflow)?
    } else {
        avs_supply.min(vault_amount)
    };
    require!(amount_in <= max_input, ArbitrageError::InvalidAmount);

    let output = if direction == 0 {
        accounts.user_avs
    } else {
        accounts.user_delegated
    };
    let pre_out = read_token_amount(output)?;
    let data = solayer_endoavs_data(direction, amount_in);
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: vec![
                AccountMeta::new_readonly(accounts.user.key(), true),
                AccountMeta::new_readonly(accounts.endoavs.key(), false),
                AccountMeta::new(accounts.avs_mint.key(), false),
                AccountMeta::new(accounts.delegated_vault.key(), false),
                AccountMeta::new_readonly(accounts.delegated_mint.key(), false),
                AccountMeta::new(accounts.user_delegated.key(), false),
                AccountMeta::new(accounts.user_avs.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
            ],
            data,
        },
        &[
            accounts.user.clone(),
            accounts.endoavs.clone(),
            accounts.avs_mint.clone(),
            accounts.delegated_vault.clone(),
            accounts.delegated_mint.clone(),
            accounts.user_delegated.clone(),
            accounts.user_avs.clone(),
            accounts.token_program.clone(),
            accounts.program.clone(),
        ],
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn solayer_endoavs_data(direction: u8, amount_in: u64) -> Vec<u8> {
    let mut data = if direction == 0 {
        DELEGATE_NO_INIT_DISCRIMINATOR.to_vec()
    } else {
        UNDELEGATE_NO_INIT_DISCRIMINATOR.to_vec()
    };
    data.extend_from_slice(&amount_in.to_le_bytes());
    data
}

pub fn validate_solayer_endoavs_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step.len() == SOLAYER_ENDOAVS_STEP_ACCOUNTS && direction <= 1 && fee_rate == 0,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step[0].key(),
        SOLAYER_ENDOAVS_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[5].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        SOLAYER_ENDOAVS_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );

    let state = step[1].try_borrow_data()?;
    require!(
        state.starts_with(&ENDOAVS_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let bump = *state
        .get(ENDOAVS_BUMP_OFFSET)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let avs_mint = read_pubkey(&state, ENDOAVS_AVS_MINT_OFFSET)?;
    let delegated_mint = read_pubkey(&state, ENDOAVS_DELEGATED_MINT_OFFSET)?;
    let delegated_vault = read_pubkey(&state, ENDOAVS_VAULT_OFFSET)?;
    drop(state);

    require_keys_eq!(step[2].key(), avs_mint, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        step[4].key(),
        delegated_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        step[3].key(),
        delegated_vault,
        ArbitrageError::InvalidAccount
    );
    let (expected_endoavs, expected_bump) = Pubkey::find_program_address(
        &[b"endo_avs", step[2].key().as_ref()],
        &SOLAYER_ENDOAVS_PROGRAM_ID,
    );
    require!(expected_bump == bump, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[1].key(),
        expected_endoavs,
        ArbitrageError::InvalidAccount
    );

    let avs_decimals = validate_mint(&step[2], &step[5], Some(step[1].key()))?;
    let delegated_decimals = validate_mint(&step[4], &step[5], None)?;
    require!(
        avs_decimals == delegated_decimals,
        ArbitrageError::InvalidTokenMint
    );
    validate_token_account_for_mint_and_authority(&step[3], &step[4], &step[5], &step[1])?;
    let expected_vault = Pubkey::find_program_address(
        &[
            step[1].key().as_ref(),
            step[5].key().as_ref(),
            step[4].key().as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0;
    require_keys_eq!(
        step[3].key(),
        expected_vault,
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output) = if direction == 0 {
        (delegated_mint, avs_mint)
    } else {
        (avs_mint, delegated_mint)
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

fn validate_mint(
    mint: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    expected_authority: Option<Pubkey>,
) -> Result<u8> {
    require_keys_eq!(
        *mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == 82 && data.get(45) == Some(&1),
        ArbitrageError::InvalidAccount
    );
    if let Some(authority) = expected_authority {
        require!(read_u32(&data, 0)? == 1, ArbitrageError::InvalidAccount);
        require_keys_eq!(
            read_pubkey(&data, 4)?,
            authority,
            ArbitrageError::InvalidAccount
        );
    }
    data.get(44)
        .copied()
        .ok_or_else(|| ArbitrageError::InvalidAccount.into())
}

fn read_mint_supply(mint: &AccountInfo<'_>) -> Result<u64> {
    let data = mint.try_borrow_data()?;
    read_u64(&data, 36)
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
        let mut data = vec![0_u8; 82];
        if let Some(authority) = authority {
            data[..4].copy_from_slice(&1_u32.to_le_bytes());
            write_pubkey(&mut data, 4, authority);
        }
        data[36..44].copy_from_slice(&supply.to_le_bytes());
        data[44] = decimals;
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
        let avs_mint = Pubkey::new_unique();
        let delegated_mint = Pubkey::new_unique();
        let (endoavs, bump) = Pubkey::find_program_address(
            &[b"endo_avs", avs_mint.as_ref()],
            &SOLAYER_ENDOAVS_PROGRAM_ID,
        );
        let delegated_vault = Pubkey::find_program_address(
            &[
                endoavs.as_ref(),
                anchor_spl::token::ID.as_ref(),
                delegated_mint.as_ref(),
            ],
            &anchor_spl::associated_token::ID,
        )
        .0;
        let mut state = vec![0_u8; 137];
        state[..8].copy_from_slice(&ENDOAVS_DISCRIMINATOR);
        state[ENDOAVS_BUMP_OFFSET] = bump;
        write_pubkey(&mut state, ENDOAVS_AVS_MINT_OFFSET, avs_mint);
        write_pubkey(&mut state, ENDOAVS_DELEGATED_MINT_OFFSET, delegated_mint);
        write_pubkey(&mut state, ENDOAVS_VAULT_OFFSET, delegated_vault);

        vec![
            account(
                SOLAYER_ENDOAVS_PROGRAM_ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(endoavs, SOLAYER_ENDOAVS_PROGRAM_ID, state, false, false),
            account(
                avs_mint,
                anchor_spl::token::ID,
                mint_data(Some(endoavs), 22_518_014_794_203, 9),
                true,
                false,
            ),
            account(
                delegated_vault,
                anchor_spl::token::ID,
                token_data(delegated_mint, endoavs, 22_615_191_141_539),
                true,
                false,
            ),
            account(
                delegated_mint,
                anchor_spl::token::ID,
                mint_data(None, 1_000_000_000_000_000, 9),
                false,
                false,
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
    fn no_init_wires_match_deployed_program() {
        let delegate = solayer_endoavs_data(0, 206_517_910);
        assert_eq!(delegate.len(), 16);
        assert_eq!(&delegate[..8], &DELEGATE_NO_INIT_DISCRIMINATOR);
        assert_eq!(&delegate[8..], &206_517_910_u64.to_le_bytes());

        let undelegate = solayer_endoavs_data(1, 306_283_164);
        assert_eq!(undelegate.len(), 16);
        assert_eq!(&undelegate[..8], &UNDELEGATE_NO_INIT_DISCRIMINATOR);
        assert_eq!(&undelegate[8..], &306_283_164_u64.to_le_bytes());
    }

    #[test]
    fn semantic_validation_binds_pda_mints_vault_and_both_directions() {
        for direction in [0, 1] {
            let step = semantic_fixture();
            let (input_mint, output_mint) = if direction == 0 {
                (&step[4], &step[2])
            } else {
                (&step[2], &step[4])
            };
            assert!(validate_solayer_endoavs_semantic_accounts(
                &step,
                direction,
                0,
                input_mint,
                output_mint,
            )
            .is_ok());
        }

        let step = semantic_fixture();
        step[3].try_borrow_mut_data().unwrap()[0] ^= 1;
        assert!(
            validate_solayer_endoavs_semantic_accounts(&step, 0, 0, &step[4], &step[2]).is_err()
        );
    }
}
