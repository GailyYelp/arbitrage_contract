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
        program_ids::XORCA_PROGRAM_ID,
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const XORCA_STEP_ACCOUNTS: usize = 5;
const XORCA_STATE: Pubkey = anchor_lang::pubkey!("CSqKhyW1cpdyjheAx5HXx4ibcnYrzpL5JywEMAkZixBK");
const XORCA_VAULT: Pubkey = anchor_lang::pubkey!("Ce5j11WAsSzM3nkzrw4Kw6v6ic3nbyqpv5eywjYKeKc5");
const ORCA_MINT: Pubkey = anchor_lang::pubkey!("orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE");
const XORCA_MINT: Pubkey = anchor_lang::pubkey!("xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N");
const ASSOCIATED_TOKEN_PROGRAM: Pubkey =
    anchor_lang::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const STATE_ACCOUNT_LEN: usize = 2_048;
const TOKEN_ACCOUNT_LEN: usize = 165;
const MINT_ACCOUNT_LEN: usize = 82;
const VIRTUAL_AMOUNT: u128 = 100;

pub struct XOrcaAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub owner: &'info AccountInfo<'info>,
    pub state: &'info AccountInfo<'info>,
    pub vault: &'info AccountInfo<'info>,
    pub user_orca: &'info AccountInfo<'info>,
    pub user_xorca: &'info AccountInfo<'info>,
    pub orca_mint: &'info AccountInfo<'info>,
    pub xorca_mint: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn xorca_swap<'info>(accounts: XOrcaAccounts<'info>, amount_in: u64) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let pre_out = read_token_amount(accounts.user_xorca)?;
    let mut data = Vec::with_capacity(9);
    data.push(0);
    data.extend_from_slice(&amount_in.to_le_bytes());
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: vec![
                AccountMeta::new(accounts.owner.key(), true),
                AccountMeta::new(accounts.vault.key(), false),
                AccountMeta::new(accounts.user_orca.key(), false),
                AccountMeta::new(accounts.user_xorca.key(), false),
                AccountMeta::new(accounts.xorca_mint.key(), false),
                AccountMeta::new_readonly(accounts.state.key(), false),
                AccountMeta::new_readonly(accounts.orca_mint.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
            ],
            data,
        },
        &[
            accounts.owner.clone(),
            accounts.vault.clone(),
            accounts.user_orca.clone(),
            accounts.user_xorca.clone(),
            accounts.xorca_mint.clone(),
            accounts.state.clone(),
            accounts.orca_mint.clone(),
            accounts.token_program.clone(),
            accounts.program.clone(),
        ],
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_xorca, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_xorca_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step.len() == XORCA_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction == 0, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        XORCA_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(step[1].key(), XORCA_STATE, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[2].key(), XORCA_VAULT, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[3].key(), XORCA_MINT, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        step[4].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        input_mint.key(),
        ORCA_MINT,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output_mint.key(),
        XORCA_MINT,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );

    let state = step[1].try_borrow_data()?;
    require!(
        state.len() == STATE_ACCOUNT_LEN && state[0] == 1,
        ArbitrageError::InvalidAccount
    );
    let state_bump = state[6];
    let vault_bump = state[7];
    let escrowed = read_u64(&state, 8)?;
    let cooldown = read_i64(&state, 16)?;
    let update_authority = read_pubkey(&state, 24)?;
    drop(state);
    let (expected_state, expected_state_bump) =
        Pubkey::find_program_address(&[b"state"], &step[0].key());
    require_keys_eq!(
        expected_state,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        state_bump == expected_state_bump,
        ArbitrageError::InvalidAccount
    );
    let (expected_vault, expected_vault_bump) = Pubkey::find_program_address(
        &[
            step[1].key().as_ref(),
            step[4].key().as_ref(),
            input_mint.key().as_ref(),
        ],
        &ASSOCIATED_TOKEN_PROGRAM,
    );
    require_keys_eq!(
        expected_vault,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        vault_bump == expected_vault_bump,
        ArbitrageError::InvalidAccount
    );
    require!(
        cooldown >= 0 && update_authority != Pubkey::default(),
        ArbitrageError::InvalidAccount
    );

    let orca = validate_mint(input_mint, &step[4], None)?;
    let xorca = validate_mint(&step[3], &step[4], Some(step[1].key()))?;
    require!(
        orca.decimals == xorca.decimals,
        ArbitrageError::InvalidTokenMint
    );
    require!(xorca.supply > 0, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[3].key(),
        output_mint.key(),
        ArbitrageError::InvalidTokenMint
    );

    let vault_amount = validate_vault(&step[2], &step[4], input_mint.key(), step[1].key())?;
    let non_escrowed = vault_amount
        .checked_sub(escrowed)
        .ok_or(ArbitrageError::InvalidAccount)?;
    require!(non_escrowed > 0, ArbitrageError::InvalidAccount);
    Ok(())
}

pub fn validate_xorca_amount(
    state: &AccountInfo<'_>,
    vault: &AccountInfo<'_>,
    xorca_mint: &AccountInfo<'_>,
    amount_in: u64,
) -> Result<()> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let state_data = state.try_borrow_data()?;
    let escrowed = read_u64(&state_data, 8)?;
    drop(state_data);
    let vault_amount = read_token_amount(vault)?;
    let supply = read_mint_supply(xorca_mint)?;
    let non_escrowed = vault_amount
        .checked_sub(escrowed)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let output = u128::from(amount_in)
        .checked_mul(
            u128::from(supply)
                .checked_add(VIRTUAL_AMOUNT)
                .ok_or(ArbitrageError::MathOverflow)?,
        )
        .ok_or(ArbitrageError::MathOverflow)?
        .checked_div(
            u128::from(non_escrowed)
                .checked_add(VIRTUAL_AMOUNT)
                .ok_or(ArbitrageError::MathOverflow)?,
        )
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        output > 0 && output <= u128::from(u64::MAX),
        ArbitrageError::InvalidAmount
    );
    vault_amount
        .checked_add(amount_in)
        .ok_or(ArbitrageError::MathOverflow)?;
    supply
        .checked_add(u64::try_from(output).map_err(|_| ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::MathOverflow)?;
    Ok(())
}

struct MintState {
    supply: u64,
    decimals: u8,
}

fn validate_mint(
    mint: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    expected_authority: Option<Pubkey>,
) -> Result<MintState> {
    require_keys_eq!(
        *mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == MINT_ACCOUNT_LEN && data[45] == 1,
        ArbitrageError::InvalidAccount
    );
    if let Some(authority) = expected_authority {
        require!(read_u32(&data, 0)? == 1, ArbitrageError::InvalidAccount);
        require_keys_eq!(
            read_pubkey(&data, 4)?,
            authority,
            ArbitrageError::InvalidAccount
        );
        require!(read_u32(&data, 46)? == 0, ArbitrageError::InvalidAccount);
    }
    Ok(MintState {
        supply: read_u64(&data, 36)?,
        decimals: data[44],
    })
}

fn validate_vault(
    vault: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    mint: Pubkey,
    authority: Pubkey,
) -> Result<u64> {
    require_keys_eq!(
        *vault.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = vault.try_borrow_data()?;
    require!(
        data.len() == TOKEN_ACCOUNT_LEN
            && data[108] == 1
            && read_u32(&data, 72)? == 0
            && read_u32(&data, 109)? == 0
            && read_u32(&data, 129)? == 0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, 0)?,
        mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&data, 32)?,
        authority,
        ArbitrageError::InvalidAccount
    );
    read_u64(&data, 64)
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i64(data: &[u8], offset: usize) -> Result<i64> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(i64::from_le_bytes(bytes))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn read_mint_supply(mint: &AccountInfo<'_>) -> Result<u64> {
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == MINT_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    read_u64(&data, 36)
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
            data[..4].copy_from_slice(&1_u32.to_le_bytes());
            write_pubkey(&mut data, 4, authority);
        }
        data[36..44].copy_from_slice(&supply.to_le_bytes());
        data[44] = decimals;
        data[45] = 1;
        data
    }

    fn token_data(mint: Pubkey, authority: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0_u8; TOKEN_ACCOUNT_LEN];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1;
        data
    }

    fn semantic_fixture() -> (Vec<AccountInfo<'static>>, AccountInfo<'static>) {
        let (state, state_bump) = Pubkey::find_program_address(&[b"state"], &XORCA_PROGRAM_ID);
        let (vault, vault_bump) = Pubkey::find_program_address(
            &[
                state.as_ref(),
                anchor_spl::token::ID.as_ref(),
                ORCA_MINT.as_ref(),
            ],
            &ASSOCIATED_TOKEN_PROGRAM,
        );
        assert_eq!(state, XORCA_STATE);
        assert_eq!(vault, XORCA_VAULT);
        let mut state_data = vec![0_u8; STATE_ACCOUNT_LEN];
        state_data[0] = 1;
        state_data[6] = state_bump;
        state_data[7] = vault_bump;
        state_data[8..16].copy_from_slice(&407_679_193_351_u64.to_le_bytes());
        state_data[16..24].copy_from_slice(&604_800_i64.to_le_bytes());
        write_pubkey(&mut state_data, 24, Pubkey::new_unique());

        let step = vec![
            account(XORCA_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(state, XORCA_PROGRAM_ID, state_data, false, false),
            account(
                vault,
                anchor_spl::token::ID,
                token_data(ORCA_MINT, state, 7_839_571_452_636),
                true,
                false,
            ),
            account(
                XORCA_MINT,
                anchor_spl::token::ID,
                mint_data(Some(state), 4_794_072_347_001, 6),
                true,
                false,
            ),
            account(
                anchor_spl::token::ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
        ];
        let orca_mint = account(
            ORCA_MINT,
            anchor_spl::token::ID,
            mint_data(Some(Pubkey::new_unique()), 74_999_550_476_933, 6),
            false,
            false,
        );
        (step, orca_mint)
    }

    #[test]
    fn native_wire_matches_confirmed_stake_instruction() {
        let mut data = vec![0];
        data.extend_from_slice(&96_845_u64.to_le_bytes());
        assert_eq!(data.len(), 9);
        assert_eq!(data[0], 0);
        assert_eq!(&data[1..], &96_845_u64.to_le_bytes());
    }

    #[test]
    fn confirmed_virtual_offset_vector_is_exact() {
        let amount = 96_845_u128;
        let pre_supply = 4_794_072_284_528_u128;
        let pre_non_escrowed = 7_839_343_523_235_u128 - 407_679_193_351_u128;
        assert_eq!(
            amount * (pre_supply + VIRTUAL_AMOUNT) / (pre_non_escrowed + VIRTUAL_AMOUNT),
            62_473
        );
    }

    #[test]
    fn semantic_validation_binds_state_vault_mints_and_one_way_direction() {
        let (step, orca_mint) = semantic_fixture();
        assert!(validate_xorca_semantic_accounts(&step, 0, 0, &orca_mint, &step[3]).is_ok());
        assert!(validate_xorca_amount(&step[1], &step[2], &step[3], 1_000_000).is_ok());
        assert!(validate_xorca_semantic_accounts(&step, 1, 0, &step[3], &orca_mint).is_err());

        let (corrupt, orca_mint) = semantic_fixture();
        corrupt[3].try_borrow_mut_data().unwrap()[4] ^= 1;
        assert!(validate_xorca_semantic_accounts(&corrupt, 0, 0, &orca_mint, &corrupt[3]).is_err());

        let (underbacked, _) = semantic_fixture();
        underbacked[1].try_borrow_mut_data().unwrap()[8..16]
            .copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(
            validate_xorca_amount(&underbacked[1], &underbacked[2], &underbacked[3], 1).is_err()
        );
    }
}
