use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
    system_program,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::{FUSIONAMM_PROGRAM_ID, MEMO_PROGRAM_V2_ID},
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const FUSIONAMM_STEP_ACCOUNTS: usize = 14;
pub const FUSIONAMM_SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const FUSION_POOL_DISCRIMINATOR: [u8; 8] = [254, 204, 207, 98, 25, 181, 29, 67];
const TICK_ARRAY_DISCRIMINATOR: [u8; 8] = [85, 1, 199, 2, 188, 97, 101, 139];
const FUSION_POOL_LEN: usize = 423;
const TICK_ARRAY_SIZE: i32 = 88;

pub struct FusionAmmAccounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub token_program_a: &'a AccountInfo<'info>,
    pub token_program_b: &'a AccountInfo<'info>,
    pub memo_program: &'a AccountInfo<'info>,
    pub payer: &'a AccountInfo<'info>,
    pub pool: &'a AccountInfo<'info>,
    pub mint_a: &'a AccountInfo<'info>,
    pub mint_b: &'a AccountInfo<'info>,
    pub owner_a: &'a AccountInfo<'info>,
    pub owner_b: &'a AccountInfo<'info>,
    pub vault_a: &'a AccountInfo<'info>,
    pub vault_b: &'a AccountInfo<'info>,
    pub tick_arrays: [&'a AccountInfo<'info>; 5],
}

pub fn fusionamm_swap<'info>(
    accounts: FusionAmmAccounts<'_, 'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    a_to_b: bool,
) -> Result<SwapResult> {
    let output_account = if a_to_b {
        accounts.owner_b
    } else {
        accounts.owner_a
    };
    let pre_out = read_token_amount(output_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.token_program_a.key(), false),
        AccountMeta::new_readonly(accounts.token_program_b.key(), false),
        AccountMeta::new_readonly(accounts.memo_program.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.mint_a.key(), false),
        AccountMeta::new_readonly(accounts.mint_b.key(), false),
        AccountMeta::new(accounts.owner_a.key(), false),
        AccountMeta::new(accounts.owner_b.key(), false),
        AccountMeta::new(accounts.vault_a.key(), false),
        AccountMeta::new(accounts.vault_b.key(), false),
        AccountMeta::new(accounts.tick_arrays[0].key(), false),
        AccountMeta::new(accounts.tick_arrays[1].key(), false),
        AccountMeta::new(accounts.tick_arrays[2].key(), false),
        AccountMeta::new(accounts.tick_arrays[3].key(), false),
        AccountMeta::new(accounts.tick_arrays[4].key(), false),
    ];
    let account_infos = vec![
        accounts.token_program_a.clone(),
        accounts.token_program_b.clone(),
        accounts.memo_program.clone(),
        accounts.payer.clone(),
        accounts.pool.clone(),
        accounts.mint_a.clone(),
        accounts.mint_b.clone(),
        accounts.owner_a.clone(),
        accounts.owner_b.clone(),
        accounts.vault_a.clone(),
        accounts.vault_b.clone(),
        accounts.tick_arrays[0].clone(),
        accounts.tick_arrays[1].clone(),
        accounts.tick_arrays[2].clone(),
        accounts.tick_arrays[3].clone(),
        accounts.tick_arrays[4].clone(),
        accounts.program.clone(),
    ];
    let instruction = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: swap_data(amount_in, minimum_amount_out, a_to_b),
    };
    invoke(&instruction, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_fusionamm_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == FUSIONAMM_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidPath);
    require_keys_eq!(
        step[0].key(),
        FUSIONAMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].key(),
        MEMO_PROGRAM_V2_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[4].owner,
        FUSIONAMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );

    let data = step[4].try_borrow_data()?;
    require!(
        data.len() == FUSION_POOL_LEN && data.starts_with(&FUSION_POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let mint_a = pubkey_at(&data, 11)?;
    let mint_b = pubkey_at(&data, 43)?;
    let vault_a = pubkey_at(&data, 75)?;
    let vault_b = pubkey_at(&data, 107)?;
    let tick_spacing = u16_at(&data, 139)?;
    let liquidity = u128_at(&data, 151)?;
    let sqrt_price = u128_at(&data, 167)?;
    let current_tick = i32_at(&data, 183)?;
    require!(
        mint_a != mint_b && tick_spacing > 0 && liquidity > 0 && sqrt_price > 0,
        ArbitrageError::InvalidAccount
    );
    let tick_spacing_bytes = tick_spacing.to_le_bytes();
    let (expected_pool, bump) = Pubkey::find_program_address(
        &[
            b"fusion_pool",
            mint_a.as_ref(),
            mint_b.as_ref(),
            &tick_spacing_bytes,
        ],
        &FUSIONAMM_PROGRAM_ID,
    );
    require_keys_eq!(step[4].key(), expected_pool, ArbitrageError::InvalidAccount);
    require!(data[8] == bump, ArbitrageError::InvalidAccount);
    drop(data);

    require_keys_eq!(step[7].key(), mint_a, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[8].key(), mint_b, ArbitrageError::InvalidAccount);
    let (expected_input_mint, expected_output_mint) = if direction == 0 {
        (mint_a, mint_b)
    } else {
        (mint_b, mint_a)
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output_mint,
        ArbitrageError::InvalidAccount
    );
    let (expected_program_a, expected_program_b) = if direction == 0 {
        (input_token_program.key(), output_token_program.key())
    } else {
        (output_token_program.key(), input_token_program.key())
    };
    require_keys_eq!(
        step[1].key(),
        expected_program_a,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[2].key(),
        expected_program_b,
        ArbitrageError::InvalidAccount
    );
    let (actual_vault_a, actual_vault_b) = if direction == 0 {
        (step[5].key(), step[6].key())
    } else {
        (step[6].key(), step[5].key())
    };
    require_keys_eq!(actual_vault_a, vault_a, ArbitrageError::InvalidAccount);
    require_keys_eq!(actual_vault_b, vault_b, ArbitrageError::InvalidAccount);

    let starts = tick_array_start_indexes(current_tick, tick_spacing)?;
    for (account, start) in step[9..14].iter().zip(starts) {
        validate_tick_array(account, step[4].key(), start)?;
    }
    Ok(())
}

fn swap_data(amount_in: u64, minimum_amount_out: u64, a_to_b: bool) -> Vec<u8> {
    let mut data = Vec::with_capacity(49);
    data.extend_from_slice(&FUSIONAMM_SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&0_u128.to_le_bytes());
    data.push(1);
    data.push(u8::from(a_to_b));
    data.extend_from_slice(&[1, 1, 0, 0, 0, 5, 2]);
    data
}

fn tick_array_start_indexes(current_tick: i32, tick_spacing: u16) -> Result<[i32; 5]> {
    let width = i32::from(tick_spacing)
        .checked_mul(TICK_ARRAY_SIZE)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let current = current_tick
        .div_euclid(i32::from(tick_spacing))
        .div_euclid(TICK_ARRAY_SIZE)
        .checked_mul(width)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok([
        current,
        current
            .checked_add(width)
            .ok_or(ArbitrageError::InvalidAccount)?,
        current
            .checked_add(width.checked_mul(2).ok_or(ArbitrageError::InvalidAccount)?)
            .ok_or(ArbitrageError::InvalidAccount)?,
        current
            .checked_sub(width)
            .ok_or(ArbitrageError::InvalidAccount)?,
        current
            .checked_sub(width.checked_mul(2).ok_or(ArbitrageError::InvalidAccount)?)
            .ok_or(ArbitrageError::InvalidAccount)?,
    ])
}

fn validate_tick_array<'info>(
    account: &AccountInfo<'info>,
    pool: Pubkey,
    start_tick_index: i32,
) -> Result<()> {
    let start_tick_index_string = start_tick_index.to_string();
    let (expected, _) = Pubkey::find_program_address(
        &[
            b"tick_array",
            pool.as_ref(),
            start_tick_index_string.as_bytes(),
        ],
        &FUSIONAMM_PROGRAM_ID,
    );
    require_keys_eq!(account.key(), expected, ArbitrageError::InvalidAccount);
    if account.data_is_empty() {
        require_keys_eq!(
            *account.owner,
            system_program::ID,
            ArbitrageError::InvalidAccount
        );
        return Ok(());
    }
    require_keys_eq!(
        *account.owner,
        FUSIONAMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() >= 44 && data.starts_with(&TICK_ARRAY_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        i32_at(&data, 8)? == start_tick_index,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(pubkey_at(&data, 12)?, pool, ArbitrageError::InvalidAccount);
    Ok(())
}

fn bytes_at<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N]> {
    data.get(offset..offset + N)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount.into())
}

fn pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey> {
    Ok(Pubkey::new_from_array(bytes_at(data, offset)?))
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes_at(data, offset)?))
}

fn i32_at(data: &[u8], offset: usize) -> Result<i32> {
    Ok(i32::from_le_bytes(bytes_at(data, offset)?))
}

fn u128_at(data: &[u8], offset: usize) -> Result<u128> {
    Ok(u128::from_le_bytes(bytes_at(data, offset)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account(
        key: Pubkey,
        owner: Pubkey,
        writable: bool,
        executable: bool,
        data: Vec<u8>,
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

    #[test]
    fn exact_input_data_matches_anchor_layout() {
        let data = swap_data(100, 90, true);
        assert_eq!(data.len(), 49);
        assert_eq!(&data[..8], &FUSIONAMM_SWAP_DISCRIMINATOR);
        assert_eq!(&data[8..16], &100_u64.to_le_bytes());
        assert_eq!(&data[16..24], &90_u64.to_le_bytes());
        assert_eq!(data[40], 1);
        assert_eq!(data[41], 1);
        assert_eq!(&data[42..], &[1, 1, 0, 0, 0, 5, 2]);
    }

    #[test]
    fn centered_tick_window_handles_negative_ticks() {
        assert_eq!(
            tick_array_start_indexes(-25_668, 4).expect("window"),
            [-25_696, -25_344, -24_992, -26_048, -26_400]
        );
    }

    #[test]
    fn semantic_validation_binds_pool_pda_vaults_and_tick_arrays() {
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();
        let vault_a = Pubkey::new_unique();
        let vault_b = Pubkey::new_unique();
        let token_program_a = Pubkey::new_unique();
        let token_program_b = Pubkey::new_unique();
        let spacing = 64_u16;
        let spacing_bytes = spacing.to_le_bytes();
        let (pool, bump) = Pubkey::find_program_address(
            &[
                b"fusion_pool",
                mint_a.as_ref(),
                mint_b.as_ref(),
                &spacing_bytes,
            ],
            &FUSIONAMM_PROGRAM_ID,
        );
        let mut pool_data = vec![0_u8; FUSION_POOL_LEN];
        pool_data[..8].copy_from_slice(&FUSION_POOL_DISCRIMINATOR);
        pool_data[8] = bump;
        pool_data[11..43].copy_from_slice(mint_a.as_ref());
        pool_data[43..75].copy_from_slice(mint_b.as_ref());
        pool_data[75..107].copy_from_slice(vault_a.as_ref());
        pool_data[107..139].copy_from_slice(vault_b.as_ref());
        pool_data[139..141].copy_from_slice(&spacing_bytes);
        pool_data[151..167].copy_from_slice(&1_000_u128.to_le_bytes());
        pool_data[167..183].copy_from_slice(&(1_u128 << 64).to_le_bytes());
        pool_data[183..187].copy_from_slice(&29_823_i32.to_le_bytes());
        let starts = tick_array_start_indexes(29_823, spacing).expect("tick starts");
        let mut step = vec![
            test_account(
                FUSIONAMM_PROGRAM_ID,
                system_program::ID,
                false,
                true,
                vec![],
            ),
            test_account(token_program_a, system_program::ID, false, true, vec![]),
            test_account(token_program_b, system_program::ID, false, true, vec![]),
            test_account(MEMO_PROGRAM_V2_ID, system_program::ID, false, true, vec![]),
            test_account(pool, FUSIONAMM_PROGRAM_ID, true, false, pool_data),
            test_account(vault_a, token_program_a, true, false, vec![]),
            test_account(vault_b, token_program_b, true, false, vec![]),
            test_account(mint_a, token_program_a, false, false, vec![]),
            test_account(mint_b, token_program_b, false, false, vec![]),
        ];
        for start in starts {
            let start_string = start.to_string();
            let (address, _) = Pubkey::find_program_address(
                &[b"tick_array", pool.as_ref(), start_string.as_bytes()],
                &FUSIONAMM_PROGRAM_ID,
            );
            step.push(test_account(
                address,
                system_program::ID,
                true,
                false,
                vec![],
            ));
        }
        validate_fusionamm_semantic_accounts(&step, 0, &step[7], &step[8], &step[1], &step[2])
            .expect("forward semantics");
        let mut reverse_step = step.clone();
        reverse_step.swap(5, 6);
        validate_fusionamm_semantic_accounts(
            &reverse_step,
            1,
            &reverse_step[8],
            &reverse_step[7],
            &reverse_step[2],
            &reverse_step[1],
        )
        .expect("reverse semantics");

        step[13] = test_account(
            Pubkey::new_unique(),
            system_program::ID,
            true,
            false,
            vec![],
        );
        assert!(validate_fusionamm_semantic_accounts(
            &step, 0, &step[7], &step[8], &step[1], &step[2],
        )
        .is_err());
    }
}
