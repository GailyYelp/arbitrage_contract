use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const OMNIPAIR_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("omnixgS8fnqHfCcTGKWj6JtKjzpJZ1Y5y9pyFkQDkYE");
pub const OMNIPAIR_STEP_ACCOUNTS: usize = 7;

const PAIR_ACCOUNT_LEN: usize = 350;
const RATE_MODEL_ACCOUNT_LEN: usize = 64;
const FUTARCHY_AUTHORITY_ACCOUNT_LEN: usize = 149;
const PAIR_DISCRIMINATOR: [u8; 8] = [85, 72, 49, 176, 182, 228, 141, 82];
const RATE_MODEL_DISCRIMINATOR: [u8; 8] = [94, 3, 203, 219, 107, 137, 4, 162];
const FUTARCHY_AUTHORITY_DISCRIMINATOR: [u8; 8] = [175, 247, 160, 182, 140, 128, 211, 226];
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

#[derive(Clone)]
pub struct OmnipairAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pair: &'info AccountInfo<'info>,
    pub rate_model: &'info AccountInfo<'info>,
    pub futarchy_authority: &'info AccountInfo<'info>,
    pub token_in_vault: &'info AccountInfo<'info>,
    pub token_out_vault: &'info AccountInfo<'info>,
    pub user_token_in_account: &'info AccountInfo<'info>,
    pub user_token_out_account: &'info AccountInfo<'info>,
    pub token_in_mint: &'info AccountInfo<'info>,
    pub token_out_mint: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub token_2022_program: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
}

pub fn omnipair_swap<'info>(
    accounts: OmnipairAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let pre_out = read_token_amount(accounts.user_token_out_account)?;
    let metas = vec![
        AccountMeta::new(accounts.pair.key(), false),
        AccountMeta::new(accounts.rate_model.key(), false),
        AccountMeta::new_readonly(accounts.futarchy_authority.key(), false),
        AccountMeta::new(accounts.token_in_vault.key(), false),
        AccountMeta::new(accounts.token_out_vault.key(), false),
        AccountMeta::new(accounts.user_token_in_account.key(), false),
        AccountMeta::new(accounts.user_token_out_account.key(), false),
        AccountMeta::new_readonly(accounts.token_in_mint.key(), false),
        AccountMeta::new_readonly(accounts.token_out_mint.key(), false),
        AccountMeta::new_readonly(accounts.user.key(), true),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_2022_program.key(), false),
        AccountMeta::new_readonly(accounts.event_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
    ];
    let infos = vec![
        accounts.pair.clone(),
        accounts.rate_model.clone(),
        accounts.futarchy_authority.clone(),
        accounts.token_in_vault.clone(),
        accounts.token_out_vault.clone(),
        accounts.user_token_in_account.clone(),
        accounts.user_token_out_account.clone(),
        accounts.token_in_mint.clone(),
        accounts.token_out_mint.clone(),
        accounts.user.clone(),
        accounts.token_program.clone(),
        accounts.token_2022_program.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: exact_in_data(amount_in, min_amount_out).to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_token_out_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_omnipair_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == OMNIPAIR_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[0].key(),
        OMNIPAIR_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        OMNIPAIR_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *in_mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *out_mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );

    let pair_data = step[1].try_borrow_data()?;
    require!(
        pair_data.len() == PAIR_ACCOUNT_LEN && pair_data.starts_with(&PAIR_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let token0 = read_pubkey(&pair_data, 8)?;
    let token1 = read_pubkey(&pair_data, 40)?;
    require!(token0 != token1, ArbitrageError::InvalidAccount);
    let rate_model = read_pubkey(&pair_data, 104)?;
    require_keys_eq!(step[2].key(), rate_model, ArbitrageError::InvalidAccount);
    let swap_fee_bps = read_u16(&pair_data, 136)?;
    require!(swap_fee_bps < 10_000, ArbitrageError::InvalidAccount);
    let half_life = read_u64(&pair_data, 138)?;
    require!(
        (60_000..=43_200_000).contains(&half_life),
        ArbitrageError::InvalidAccount
    );
    let option_tag = *pair_data.get(146).ok_or(ArbitrageError::InvalidAccount)?;
    let reserve_offset = match option_tag {
        0 => 147,
        1 => {
            require!(
                read_u16(&pair_data, 147)? <= 8_500,
                ArbitrageError::InvalidAccount
            );
            149
        }
        _ => return Err(ArbitrageError::InvalidAccount.into()),
    };
    let reserve0 = read_u64(&pair_data, reserve_offset)?;
    let reserve1 = read_u64(&pair_data, reserve_offset + 8)?;
    let cash_reserve0 = read_u64(&pair_data, reserve_offset + 16)?;
    let cash_reserve1 = read_u64(&pair_data, reserve_offset + 24)?;
    let total_debt0 = read_u64(&pair_data, reserve_offset + 88)?;
    let total_debt1 = read_u64(&pair_data, reserve_offset + 96)?;
    let total_debt0_shares = read_u128(&pair_data, reserve_offset + 104)?;
    let total_debt1_shares = read_u128(&pair_data, reserve_offset + 120)?;
    let total_supply = read_u64(&pair_data, reserve_offset + 136)?;
    let token0_decimals = *pair_data
        .get(reserve_offset + 160)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let token1_decimals = *pair_data
        .get(reserve_offset + 161)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let params_hash: [u8; 32] = pair_data
        .get(reserve_offset + 162..reserve_offset + 194)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    let version = *pair_data
        .get(reserve_offset + 194)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let pair_bump = *pair_data
        .get(reserve_offset + 195)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let reserve0_bump = *pair_data
        .get(reserve_offset + 196)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let reserve1_bump = *pair_data
        .get(reserve_offset + 197)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let expected_reserve0 = cash_reserve0
        .checked_add(total_debt0)
        .ok_or(ArbitrageError::MathOverflow)?;
    let expected_reserve1 = cash_reserve1
        .checked_add(total_debt1)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        reserve0 > 0
            && reserve1 > 0
            && reserve0 == expected_reserve0
            && reserve1 == expected_reserve1
            && ((total_debt0 == 0) == (total_debt0_shares == 0))
            && ((total_debt1 == 0) == (total_debt1_shares == 0))
            && total_supply > 0
            && version == 1,
        ArbitrageError::InvalidAccount
    );

    let (expected_pair, expected_pair_bump) = Pubkey::find_program_address(
        &[b"gamm_pair", token0.as_ref(), token1.as_ref(), &params_hash],
        &OMNIPAIR_PROGRAM_ID,
    );
    require_keys_eq!(step[1].key(), expected_pair, ArbitrageError::InvalidAccount);
    require!(
        pair_bump == expected_pair_bump,
        ArbitrageError::InvalidAccount
    );

    validate_rate_model(&step[2])?;
    validate_futarchy_authority(&step[3])?;
    let (expected_vault0, expected_vault0_bump) = Pubkey::find_program_address(
        &[b"reserve_vault", step[1].key().as_ref(), token0.as_ref()],
        &OMNIPAIR_PROGRAM_ID,
    );
    let (expected_vault1, expected_vault1_bump) = Pubkey::find_program_address(
        &[b"reserve_vault", step[1].key().as_ref(), token1.as_ref()],
        &OMNIPAIR_PROGRAM_ID,
    );
    require!(
        reserve0_bump == expected_vault0_bump && reserve1_bump == expected_vault1_bump,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        expected_vault0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        expected_vault1,
        ArbitrageError::InvalidAccount
    );
    validate_vault(
        &step[4],
        token0,
        step[1].key(),
        cash_reserve0,
        token_program,
    )?;
    validate_vault(
        &step[5],
        token1,
        step[1].key(),
        cash_reserve1,
        token_program,
    )?;

    let expected_event =
        Pubkey::find_program_address(&[b"__event_authority"], &OMNIPAIR_PROGRAM_ID).0;
    require_keys_eq!(
        step[6].key(),
        expected_event,
        ArbitrageError::InvalidAccount
    );
    let (expected_in, expected_out, expected_in_decimals, expected_out_decimals) = if direction == 0
    {
        (token0, token1, token0_decimals, token1_decimals)
    } else {
        (token1, token0, token1_decimals, token0_decimals)
    };
    require_keys_eq!(in_mint.key(), expected_in, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        out_mint.key(),
        expected_out,
        ArbitrageError::InvalidTokenMint
    );
    validate_mint_decimals(in_mint, expected_in_decimals)?;
    validate_mint_decimals(out_mint, expected_out_decimals)
}

fn validate_rate_model(account: &AccountInfo<'_>) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        OMNIPAIR_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() == RATE_MODEL_ACCOUNT_LEN && data.starts_with(&RATE_MODEL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let exp_rate = read_u64(&data, 8)?;
    let target_start = read_u64(&data, 16)?;
    let target_end = read_u64(&data, 24)?;
    let half_life = read_u64(&data, 32)?;
    let min_rate = read_u64(&data, 40)?;
    let max_rate = read_u64(&data, 48)?;
    let initial_rate = read_u64(&data, 56)?;
    require!(
        exp_rate > 0
            && target_start < target_end
            && target_start >= 10_000_000
            && target_end <= 1_000_000_000
            && (3_600_000..=2_592_000_000).contains(&half_life)
            && min_rate <= 10_000_000_000
            && (max_rate == 0 || max_rate <= 10_000_000_000)
            && (1_000_000..=1_000_000_000).contains(&initial_rate)
            && initial_rate >= min_rate
            && (max_rate == 0 || (initial_rate <= max_rate && min_rate <= max_rate)),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_futarchy_authority(account: &AccountInfo<'_>) -> Result<()> {
    let (expected, expected_bump) =
        Pubkey::find_program_address(&[b"futarchy_authority"], &OMNIPAIR_PROGRAM_ID);
    require_keys_eq!(account.key(), expected, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *account.owner,
        OMNIPAIR_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() == FUTARCHY_AUTHORITY_ACCOUNT_LEN
            && data.starts_with(&FUTARCHY_AUTHORITY_DISCRIMINATOR)
            && data[8] == 1
            && read_u16(&data, 137)? <= 10_000
            && read_u16(&data, 139)? <= 10_000
            && u32::from(read_u16(&data, 141)?)
                + u32::from(read_u16(&data, 143)?)
                + u32::from(read_u16(&data, 145)?)
                == 10_000
            && data[148] == expected_bump,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_mint_decimals(mint: &AccountInfo<'_>, expected_decimals: u8) -> Result<()> {
    let data = mint.try_borrow_data()?;
    require!(
        data.get(44).copied() == Some(expected_decimals),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_vault<'info>(
    vault: &AccountInfo<'info>,
    expected_mint: Pubkey,
    expected_authority: Pubkey,
    minimum_amount: u64,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *vault.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = vault.try_borrow_data()?;
    require_keys_eq!(
        read_pubkey(&data, 0)?,
        expected_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, 32)?,
        expected_authority,
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(&data, 64)? >= minimum_amount,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn exact_in_data(amount_in: u64, min_amount_out: u64) -> [u8; 24] {
    let mut data = [0_u8; 24];
    data[..8].copy_from_slice(&SWAP_DISCRIMINATOR);
    data[8..16].copy_from_slice(&amount_in.to_le_bytes());
    data[16..24].copy_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset.checked_add(2).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u16::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let bytes = data
        .get(offset..offset.checked_add(16).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u128::from_le_bytes(
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
        signer: bool,
        writable: bool,
        executable: bool,
        data: Vec<u8>,
    ) -> AccountInfo<'static> {
        AccountInfo::new(
            Box::leak(Box::new(key)),
            signer,
            writable,
            Box::leak(Box::new(0_u64)),
            Box::leak(data.into_boxed_slice()),
            Box::leak(Box::new(owner)),
            executable,
            0,
        )
    }

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + 32].copy_from_slice(key.as_ref());
    }

    fn fixture() -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let token_program = anchor_spl::token::ID;
        let token0 = Pubkey::new_unique();
        let token1 = Pubkey::new_unique();
        let params_hash = [9_u8; 32];
        let (pair, pair_bump) = Pubkey::find_program_address(
            &[b"gamm_pair", token0.as_ref(), token1.as_ref(), &params_hash],
            &OMNIPAIR_PROGRAM_ID,
        );
        let (vault0, vault0_bump) = Pubkey::find_program_address(
            &[b"reserve_vault", pair.as_ref(), token0.as_ref()],
            &OMNIPAIR_PROGRAM_ID,
        );
        let (vault1, vault1_bump) = Pubkey::find_program_address(
            &[b"reserve_vault", pair.as_ref(), token1.as_ref()],
            &OMNIPAIR_PROGRAM_ID,
        );
        let rate_model = Pubkey::new_unique();
        let (futarchy, futarchy_bump) =
            Pubkey::find_program_address(&[b"futarchy_authority"], &OMNIPAIR_PROGRAM_ID);
        let event = Pubkey::find_program_address(&[b"__event_authority"], &OMNIPAIR_PROGRAM_ID).0;

        let mut pair_data = vec![0_u8; PAIR_ACCOUNT_LEN];
        pair_data[..8].copy_from_slice(&PAIR_DISCRIMINATOR);
        write_pubkey(&mut pair_data, 8, token0);
        write_pubkey(&mut pair_data, 40, token1);
        write_pubkey(&mut pair_data, 72, Pubkey::new_unique());
        write_pubkey(&mut pair_data, 104, rate_model);
        pair_data[136..138].copy_from_slice(&25_u16.to_le_bytes());
        pair_data[138..146].copy_from_slice(&420_000_u64.to_le_bytes());
        pair_data[146] = 0;
        let base = 147;
        pair_data[base..base + 8].copy_from_slice(&5_184_886_321_903_u64.to_le_bytes());
        pair_data[base + 8..base + 16].copy_from_slice(&4_782_780_889_u64.to_le_bytes());
        pair_data[base + 16..base + 24].copy_from_slice(&5_184_886_321_903_u64.to_le_bytes());
        pair_data[base + 24..base + 32].copy_from_slice(&4_782_780_889_u64.to_le_bytes());
        pair_data[base + 136..base + 144].copy_from_slice(&1_000_u64.to_le_bytes());
        pair_data[base + 160] = 6;
        pair_data[base + 161] = 9;
        pair_data[base + 162..base + 194].copy_from_slice(&params_hash);
        pair_data[base + 194] = 1;
        pair_data[base + 195] = pair_bump;
        pair_data[base + 196] = vault0_bump;
        pair_data[base + 197] = vault1_bump;

        let mut rate_data = vec![0_u8; RATE_MODEL_ACCOUNT_LEN];
        rate_data[..8].copy_from_slice(&RATE_MODEL_DISCRIMINATOR);
        rate_data[8..16].copy_from_slice(&1_u64.to_le_bytes());
        rate_data[16..24].copy_from_slice(&300_000_000_u64.to_le_bytes());
        rate_data[24..32].copy_from_slice(&500_000_000_u64.to_le_bytes());
        rate_data[32..40].copy_from_slice(&259_200_000_u64.to_le_bytes());
        rate_data[40..48].copy_from_slice(&10_000_000_u64.to_le_bytes());
        rate_data[56..64].copy_from_slice(&20_000_000_u64.to_le_bytes());

        let mut futarchy_data = vec![0_u8; FUTARCHY_AUTHORITY_ACCOUNT_LEN];
        futarchy_data[..8].copy_from_slice(&FUTARCHY_AUTHORITY_DISCRIMINATOR);
        futarchy_data[8] = 1;
        futarchy_data[137..139].copy_from_slice(&1_000_u16.to_le_bytes());
        futarchy_data[139..141].copy_from_slice(&1_000_u16.to_le_bytes());
        futarchy_data[141..143].copy_from_slice(&4_000_u16.to_le_bytes());
        futarchy_data[143..145].copy_from_slice(&3_000_u16.to_le_bytes());
        futarchy_data[145..147].copy_from_slice(&3_000_u16.to_le_bytes());
        futarchy_data[148] = futarchy_bump;

        let vault_data = |mint: Pubkey, amount: u64| {
            let mut data = vec![0_u8; 72];
            write_pubkey(&mut data, 0, mint);
            write_pubkey(&mut data, 32, pair);
            data[64..72].copy_from_slice(&amount.to_le_bytes());
            data
        };
        let mut mint0_data = vec![0_u8; 45];
        mint0_data[44] = 6;
        let mut mint1_data = vec![0_u8; 45];
        mint1_data[44] = 9;
        let steps = vec![
            account(
                OMNIPAIR_PROGRAM_ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
                vec![],
            ),
            account(pair, OMNIPAIR_PROGRAM_ID, false, true, false, pair_data),
            account(
                rate_model,
                OMNIPAIR_PROGRAM_ID,
                false,
                true,
                false,
                rate_data,
            ),
            account(
                futarchy,
                OMNIPAIR_PROGRAM_ID,
                false,
                false,
                false,
                futarchy_data,
            ),
            account(
                vault0,
                token_program,
                false,
                true,
                false,
                vault_data(token0, 5_184_886_321_903),
            ),
            account(
                vault1,
                token_program,
                false,
                true,
                false,
                vault_data(token1, 4_782_780_889),
            ),
            account(
                event,
                anchor_lang::system_program::ID,
                false,
                false,
                false,
                vec![],
            ),
        ];
        (
            steps,
            account(token0, token_program, false, false, false, mint0_data),
            account(token1, token_program, false, false, false, mint1_data),
        )
    }

    #[test]
    fn exact_in_wire_data_matches_official_anchor_layout() {
        let data = exact_in_data(6_962_737, 7_500_000_000);
        assert_eq!(&data[..8], &SWAP_DISCRIMINATOR);
        assert_eq!(&data[8..16], &6_962_737_u64.to_le_bytes());
        assert_eq!(&data[16..24], &7_500_000_000_u64.to_le_bytes());
    }

    #[test]
    fn semantic_validation_accepts_consistent_debt_and_rejects_broken_invariants() {
        let (steps, mint0, mint1) = fixture();
        let token_program = account(
            anchor_spl::token::ID,
            anchor_lang::system_program::ID,
            false,
            false,
            true,
            vec![],
        );
        crate::instructions::accounts::validate_step_account_flags(
            crate::state::Protocol::Omnipair,
            &steps,
        )
        .expect("Omnipair route account flags");
        validate_omnipair_semantic_accounts(&steps, 0, &mint0, &mint1, &token_program)
            .expect("token0 to token1");
        validate_omnipair_semantic_accounts(&steps, 1, &mint1, &mint0, &token_program)
            .expect("token1 to token0");

        {
            let mut data = steps[2].try_borrow_mut_data().expect("rate model data");
            data[16..24].copy_from_slice(&0_u64.to_le_bytes());
        }
        assert!(
            validate_omnipair_semantic_accounts(&steps, 0, &mint0, &mint1, &token_program).is_err()
        );
        {
            let mut data = steps[2].try_borrow_mut_data().expect("rate model data");
            data[16..24].copy_from_slice(&300_000_000_u64.to_le_bytes());
        }

        {
            let mut data = steps[1].try_borrow_mut_data().expect("pair data");
            data[147..147 + 8].copy_from_slice(&5_184_886_321_904_u64.to_le_bytes());
            data[147 + 88..147 + 96].copy_from_slice(&1_u64.to_le_bytes());
            data[147 + 104..147 + 120].copy_from_slice(&1_000_000_u128.to_le_bytes());
        }
        validate_omnipair_semantic_accounts(&steps, 0, &mint0, &mint1, &token_program)
            .expect("consistent debt state");

        {
            let mut data = steps[1].try_borrow_mut_data().expect("pair data");
            data[147 + 104..147 + 120].fill(0);
        }
        assert!(
            validate_omnipair_semantic_accounts(&steps, 0, &mint0, &mint1, &token_program).is_err()
        );
    }
}
