use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{
        read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
        SwapResult,
    },
};

pub const CROPPER_SWAP_SELECTOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
pub const CROPPER_WHIRLPOOL_DISCRIMINATOR: [u8; 8] = [63, 149, 209, 12, 225, 128, 99, 9];
pub const CROPPER_TICK_ARRAY_DISCRIMINATOR: [u8; 8] = [69, 97, 189, 190, 110, 7, 66, 187];
pub const CROPPER_STEP_ACCOUNTS: usize = 8;
pub const CROPPER_WHIRLPOOL_LEN: usize = 653;
pub const CROPPER_TICK_ARRAY_LEN: usize = 9_988;
pub const CROPPER_TICK_ARRAY_POOL_OFFSET: usize = 9_956;
pub const CROPPER_MIN_SQRT_PRICE: u128 = 4_295_048_016;
pub const CROPPER_MAX_SQRT_PRICE: u128 = 79_226_673_515_401_279_992_447_579_055;

#[derive(Clone)]
pub struct CropperAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub whirlpool: &'info AccountInfo<'info>,
    pub token_owner_account_a: &'info AccountInfo<'info>,
    pub token_vault_a: &'info AccountInfo<'info>,
    pub token_owner_account_b: &'info AccountInfo<'info>,
    pub token_vault_b: &'info AccountInfo<'info>,
    pub tick_array_0: &'info AccountInfo<'info>,
    pub tick_array_1: &'info AccountInfo<'info>,
    pub tick_array_2: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
}

pub fn validate_cropper_semantic_accounts<'info>(
    step_accounts: &[AccountInfo<'info>],
    direction: u8,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step_accounts.len() == CROPPER_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let program = &step_accounts[0];
    let whirlpool = &step_accounts[1];
    let (mint_a, mint_b) = if direction == 0 {
        (input_mint, output_mint)
    } else {
        (output_mint, input_mint)
    };

    require_keys_eq!(
        *whirlpool.owner,
        program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = whirlpool.try_borrow_data()?;
    require!(
        data.len() == CROPPER_WHIRLPOOL_LEN,
        ArbitrageError::InvalidAccount
    );
    require!(
        data.get(..8) == Some(CROPPER_WHIRLPOOL_DISCRIMINATOR.as_slice()),
        ArbitrageError::InvalidAccount
    );
    let config = read_pubkey(&data, 8)?;
    let bump = *data.get(40).ok_or(ArbitrageError::InvalidAccount)?;
    let tick_spacing = read_u16(&data, 41)?;
    let fee_tier_seed = read_u16(&data, 43)?;
    let liquidity = read_u128(&data, 49)?;
    let sqrt_price = read_u128(&data, 65)?;
    let current_tick = read_i32(&data, 81)?;
    let state_mint_a = read_pubkey(&data, 101)?;
    let state_vault_a = read_pubkey(&data, 133)?;
    let state_mint_b = read_pubkey(&data, 181)?;
    let state_vault_b = read_pubkey(&data, 213)?;
    drop(data);

    require!(tick_spacing > 0, ArbitrageError::InvalidAccount);
    require!(
        fee_tier_seed == tick_spacing,
        ArbitrageError::InvalidAccount
    );
    require!(
        liquidity > 0 && sqrt_price > 0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(state_mint_a, mint_a.key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(state_mint_b, mint_b.key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        state_vault_a,
        step_accounts[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        state_vault_b,
        step_accounts[3].key(),
        ArbitrageError::InvalidAccount
    );

    let spacing_seed = tick_spacing.to_le_bytes();
    let (expected_pool, expected_bump) = Pubkey::find_program_address(
        &[
            b"whirlpool",
            config.as_ref(),
            mint_a.key().as_ref(),
            mint_b.key().as_ref(),
            &spacing_seed,
        ],
        program.key,
    );
    require_keys_eq!(
        expected_pool,
        whirlpool.key(),
        ArbitrageError::InvalidAccount
    );
    require!(bump == expected_bump, ArbitrageError::InvalidAccount);

    validate_token_account_for_mint_and_authority(
        &step_accounts[2],
        mint_a,
        token_program,
        whirlpool,
    )?;
    validate_token_account_for_mint_and_authority(
        &step_accounts[3],
        mint_b,
        token_program,
        whirlpool,
    )?;

    let expected_starts = directional_start_indexes(current_tick, tick_spacing, direction == 0)?;
    let ticks = [&step_accounts[4], &step_accounts[5], &step_accounts[6]];
    for index in 0..ticks.len() {
        if index > 0 && ticks[index].key() == ticks[index - 1].key() {
            continue;
        }
        validate_tick_array(ticks[index], program, whirlpool, expected_starts[index])?;
    }

    let expected_oracle =
        Pubkey::find_program_address(&[b"oracle", whirlpool.key().as_ref()], program.key).0;
    require_keys_eq!(
        expected_oracle,
        step_accounts[7].key(),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

pub fn cropper_swap<'info>(
    accounts: CropperAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    a_to_b: bool,
) -> Result<SwapResult> {
    let output = if a_to_b {
        accounts.token_owner_account_b
    } else {
        accounts.token_owner_account_a
    };
    let pre_out = read_token_amount(output)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.whirlpool.key(), false),
        AccountMeta::new(accounts.token_owner_account_a.key(), false),
        AccountMeta::new(accounts.token_vault_a.key(), false),
        AccountMeta::new(accounts.token_owner_account_b.key(), false),
        AccountMeta::new(accounts.token_vault_b.key(), false),
        AccountMeta::new(accounts.tick_array_0.key(), false),
        AccountMeta::new(accounts.tick_array_1.key(), false),
        AccountMeta::new(accounts.tick_array_2.key(), false),
        AccountMeta::new_readonly(accounts.oracle.key(), false),
    ];
    let account_infos = vec![
        accounts.token_program.clone(),
        accounts.payer.clone(),
        accounts.whirlpool.clone(),
        accounts.token_owner_account_a.clone(),
        accounts.token_vault_a.clone(),
        accounts.token_owner_account_b.clone(),
        accounts.token_vault_b.clone(),
        accounts.tick_array_0.clone(),
        accounts.tick_array_1.clone(),
        accounts.tick_array_2.clone(),
        accounts.oracle.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(42);
    data.extend_from_slice(&CROPPER_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(
        &(if a_to_b {
            CROPPER_MIN_SQRT_PRICE
        } else {
            CROPPER_MAX_SQRT_PRICE
        })
        .to_le_bytes(),
    );
    data.push(1);
    data.push(u8::from(a_to_b));
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &account_infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn validate_tick_array<'info>(
    tick_array: &AccountInfo<'info>,
    program: &AccountInfo<'info>,
    whirlpool: &AccountInfo<'info>,
    expected_start: i32,
) -> Result<()> {
    require_keys_eq!(
        *tick_array.owner,
        program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = tick_array.try_borrow_data()?;
    require!(
        data.len() == CROPPER_TICK_ARRAY_LEN,
        ArbitrageError::InvalidAccount
    );
    require!(
        data.get(..8) == Some(CROPPER_TICK_ARRAY_DISCRIMINATOR.as_slice()),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_i32(&data, 8)? == expected_start,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, CROPPER_TICK_ARRAY_POOL_OFFSET)?,
        whirlpool.key(),
        ArbitrageError::InvalidAccount
    );
    drop(data);
    let seed = expected_start.to_string();
    let expected = Pubkey::find_program_address(
        &[b"tick_array", whirlpool.key().as_ref(), seed.as_bytes()],
        program.key,
    )
    .0;
    require_keys_eq!(expected, tick_array.key(), ArbitrageError::InvalidAccount);
    Ok(())
}

fn directional_start_indexes(
    current_tick: i32,
    tick_spacing: u16,
    a_to_b: bool,
) -> Result<[i32; 3]> {
    let array_spacing = i32::from(tick_spacing)
        .checked_mul(88)
        .ok_or(ArbitrageError::InvalidAccount)?;
    require!(array_spacing > 0, ArbitrageError::InvalidAccount);
    let current_start = current_tick
        .div_euclid(array_spacing)
        .checked_mul(array_spacing)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let delta = if a_to_b {
        -array_spacing
    } else {
        array_spacing
    };
    Ok([
        current_start,
        current_start
            .checked_add(delta)
            .ok_or(ArbitrageError::InvalidAccount)?,
        current_start
            .checked_add(delta.checked_mul(2).ok_or(ArbitrageError::InvalidAccount)?)
            .ok_or(ArbitrageError::InvalidAccount)?,
    ])
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

fn read_i32(data: &[u8], offset: usize) -> Result<i32> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(i32::from_le_bytes(bytes))
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
    use crate::instructions::program_ids::CROPPER_PROGRAM_ID;

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

    fn token_account_data(mint: Pubkey, authority: Pubkey) -> Vec<u8> {
        let mut data = vec![0_u8; 165];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data
    }

    fn semantic_fixture(
        a_to_b: bool,
        repeat_missing_ticks: bool,
    ) -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let config = Pubkey::new_unique();
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();
        let vault_a = Pubkey::new_unique();
        let vault_b = Pubkey::new_unique();
        let tick_spacing = 64_u16;
        let spacing_seed = tick_spacing.to_le_bytes();
        let (pool, bump) = Pubkey::find_program_address(
            &[
                b"whirlpool",
                config.as_ref(),
                mint_a.as_ref(),
                mint_b.as_ref(),
                &spacing_seed,
            ],
            &CROPPER_PROGRAM_ID,
        );
        let current_tick = 126_773_i32;
        let starts = directional_start_indexes(current_tick, tick_spacing, a_to_b).unwrap();
        let mut pool_data = vec![0_u8; CROPPER_WHIRLPOOL_LEN];
        pool_data[..8].copy_from_slice(&CROPPER_WHIRLPOOL_DISCRIMINATOR);
        write_pubkey(&mut pool_data, 8, config);
        pool_data[40] = bump;
        pool_data[41..43].copy_from_slice(&tick_spacing.to_le_bytes());
        pool_data[43..45].copy_from_slice(&tick_spacing.to_le_bytes());
        pool_data[45..47].copy_from_slice(&3_000_u16.to_le_bytes());
        pool_data[49..65].copy_from_slice(&1_u128.to_le_bytes());
        pool_data[65..81].copy_from_slice(&(1_u128 << 64).to_le_bytes());
        pool_data[81..85].copy_from_slice(&current_tick.to_le_bytes());
        write_pubkey(&mut pool_data, 101, mint_a);
        write_pubkey(&mut pool_data, 133, vault_a);
        write_pubkey(&mut pool_data, 181, mint_b);
        write_pubkey(&mut pool_data, 213, vault_b);

        let tick_keys = starts.map(|start| {
            Pubkey::find_program_address(
                &[b"tick_array", pool.as_ref(), start.to_string().as_bytes()],
                &CROPPER_PROGRAM_ID,
            )
            .0
        });
        let tick_accounts: Vec<AccountInfo<'static>> = starts
            .into_iter()
            .zip(tick_keys)
            .map(|(start, key)| {
                let mut data = vec![0_u8; CROPPER_TICK_ARRAY_LEN];
                data[..8].copy_from_slice(&CROPPER_TICK_ARRAY_DISCRIMINATOR);
                data[8..12].copy_from_slice(&start.to_le_bytes());
                write_pubkey(&mut data, CROPPER_TICK_ARRAY_POOL_OFFSET, pool);
                account(key, CROPPER_PROGRAM_ID, data, true, false)
            })
            .collect();
        let oracle =
            Pubkey::find_program_address(&[b"oracle", pool.as_ref()], &CROPPER_PROGRAM_ID).0;
        let first_tick_key = tick_accounts[0].key();
        let mut step = vec![
            account(CROPPER_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(pool, CROPPER_PROGRAM_ID, pool_data, true, false),
            account(
                vault_a,
                anchor_spl::token::ID,
                token_account_data(mint_a, pool),
                true,
                false,
            ),
            account(
                vault_b,
                anchor_spl::token::ID,
                token_account_data(mint_b, pool),
                true,
                false,
            ),
        ];
        if repeat_missing_ticks {
            step.extend([
                tick_accounts[0].clone(),
                account(first_tick_key, CROPPER_PROGRAM_ID, vec![], true, false),
                account(first_tick_key, CROPPER_PROGRAM_ID, vec![], true, false),
            ]);
        } else {
            step.extend(tick_accounts);
        }
        step.push(account(oracle, Pubkey::default(), vec![], false, false));
        let token_program = account(
            anchor_spl::token::ID,
            Pubkey::default(),
            vec![],
            false,
            true,
        );
        let mint_a_account = account(mint_a, anchor_spl::token::ID, vec![], false, false);
        let mint_b_account = account(mint_b, anchor_spl::token::ID, vec![], false, false);
        (step, token_program, mint_a_account, mint_b_account)
    }

    #[test]
    fn swap_data_matches_confirmed_whirlpool_v1_wire_layout() {
        let mut data = Vec::new();
        data.extend_from_slice(&CROPPER_SWAP_SELECTOR);
        data.extend_from_slice(&1_000_000_u64.to_le_bytes());
        data.extend_from_slice(&123_u64.to_le_bytes());
        data.extend_from_slice(&CROPPER_MIN_SQRT_PRICE.to_le_bytes());
        data.extend_from_slice(&[1, 1]);
        assert_eq!(data.len(), 42);
        assert_eq!(&data[24..40], &CROPPER_MIN_SQRT_PRICE.to_le_bytes());
    }

    #[test]
    fn directional_tick_starts_match_cropper_pool_fixture() {
        assert_eq!(
            directional_start_indexes(126_773, 64, true).unwrap(),
            [123_904, 118_272, 112_640]
        );
        assert_eq!(
            directional_start_indexes(126_773, 64, false).unwrap(),
            [123_904, 129_536, 135_168]
        );
    }

    #[test]
    fn semantic_validation_accepts_unique_and_fallback_tick_windows() {
        let (forward, token_program, mint_a, mint_b) = semantic_fixture(true, false);
        assert!(
            validate_cropper_semantic_accounts(&forward, 0, &mint_a, &mint_b, &token_program,)
                .is_ok()
        );

        let (reverse, token_program, mint_a, mint_b) = semantic_fixture(false, true);
        assert!(
            validate_cropper_semantic_accounts(&reverse, 1, &mint_b, &mint_a, &token_program,)
                .is_ok()
        );

        let mut invalid = reverse;
        invalid[7] = account(
            Pubkey::new_unique(),
            Pubkey::default(),
            vec![],
            false,
            false,
        );
        assert!(
            validate_cropper_semantic_accounts(&invalid, 1, &mint_b, &mint_a, &token_program,)
                .is_err()
        );
    }
}
