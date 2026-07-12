use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const PERENA_NUMERAIRE_SWAP_DISCRIMINATOR: [u8; 8] = [104, 104, 131, 86, 161, 189, 180, 216];
pub const PERENA_NUMERAIRE_POOL_DISCRIMINATOR: [u8; 8] = [239, 91, 93, 162, 171, 14, 42, 66];
pub const PERENA_NUMERAIRE_POOL_LEN: usize = 4_024;
pub const PERENA_NUMERAIRE_PAIR_OFFSET: usize = 152;
pub const PERENA_NUMERAIRE_PAIR_LEN: usize = 368;
pub const PERENA_NUMERAIRE_STEP_ACCOUNTS: usize = 9;

#[derive(Clone)]
pub struct PerenaNumeraireAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub in_mint: &'info AccountInfo<'info>,
    pub out_mint: &'info AccountInfo<'info>,
    pub in_trader: &'info AccountInfo<'info>,
    pub out_trader: &'info AccountInfo<'info>,
    pub in_vault: &'info AccountInfo<'info>,
    pub out_vault: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub token_2022_program: &'info AccountInfo<'info>,
}

pub fn perena_numeraire_swap<'info>(
    accounts: PerenaNumeraireAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    in_index: u8,
    out_index: u8,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.out_trader)?;
    let metas = vec![
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new(accounts.in_mint.key(), false),
        AccountMeta::new(accounts.out_mint.key(), false),
        AccountMeta::new(accounts.in_trader.key(), false),
        AccountMeta::new(accounts.out_trader.key(), false),
        AccountMeta::new(accounts.in_vault.key(), false),
        AccountMeta::new(accounts.out_vault.key(), false),
        AccountMeta::new_readonly(accounts.config.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_2022_program.key(), false),
    ];
    let account_infos = vec![
        accounts.pool.clone(),
        accounts.in_mint.clone(),
        accounts.out_mint.clone(),
        accounts.in_trader.clone(),
        accounts.out_trader.clone(),
        accounts.in_vault.clone(),
        accounts.out_vault.clone(),
        accounts.config.clone(),
        accounts.payer.clone(),
        accounts.token_program.clone(),
        accounts.token_2022_program.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: swap_data(amount_in, min_amount_out, in_index, out_index),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.out_trader, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_perena_numeraire_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    token_2022_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == PERENA_NUMERAIRE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let program = &step[0];
    let pool = &step[1];
    let mint_0 = &step[2];
    let mint_1 = &step[3];
    let vault_0 = &step[4];
    let vault_1 = &step[5];
    let config = &step[6];
    require_keys_eq!(
        step[7].key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        token_2022_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(*pool.owner, program.key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(*config.owner, program.key(), ArbitrageError::InvalidAccount);
    let expected_config = Pubkey::find_program_address(&[b"config"], &program.key()).0;
    require_keys_eq!(
        config.key(),
        expected_config,
        ArbitrageError::InvalidAccount
    );

    let data = pool.try_borrow_data()?;
    require!(
        data.len() == PERENA_NUMERAIRE_POOL_LEN
            && data.starts_with(&PERENA_NUMERAIRE_POOL_DISCRIMINATOR)
            && data[3_893] == 2,
        ArbitrageError::InvalidAccount
    );
    let pool_seed = read_pubkey(&data, 8)?;
    let expected_pool = Pubkey::find_program_address(&[pool_seed.as_ref()], &program.key()).0;
    require_keys_eq!(pool.key(), expected_pool, ArbitrageError::InvalidAccount);
    let fee_num = read_u32(&data, 3_884)?;
    let fee_denom = read_u32(&data, 3_888)?;
    require!(
        fee_num > 0 && fee_denom > 0 && fee_num <= fee_denom,
        ArbitrageError::InvalidAccount
    );

    validate_pair(
        &data,
        0,
        pool,
        mint_0,
        vault_0,
        token_program,
        token_2022_program,
    )?;
    validate_pair(
        &data,
        1,
        pool,
        mint_1,
        vault_1,
        token_program,
        token_2022_program,
    )?;
    let (expected_in, expected_out) = if direction == 0 {
        (mint_0, mint_1)
    } else {
        (mint_1, mint_0)
    };
    require_keys_eq!(
        in_mint.key(),
        expected_in.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        out_mint.key(),
        expected_out.key(),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_pair<'info>(
    data: &[u8],
    index: usize,
    pool: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    token_2022_program: &AccountInfo<'info>,
) -> Result<()> {
    let base = PERENA_NUMERAIRE_PAIR_OFFSET + index * PERENA_NUMERAIRE_PAIR_LEN;
    require_keys_eq!(
        read_pubkey(data, base)?,
        pool.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(data, base + 112)?,
        pool.key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(data, base + 32)? > 0
            && read_u64(data, base + 40)? > 0
            && read_u128(data, base + 48)? > 0
            && read_u128(data, base + 96)? > 0
            && data[base + 233] == index as u8,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(data, base + 144)?,
        mint.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(data, base + 176)?,
        vault.key(),
        ArbitrageError::InvalidAccount
    );
    let expected_token_program = if data[base + 234] == 0 {
        token_program
    } else {
        require!(data[base + 234] == 1, ArbitrageError::InvalidAccount);
        token_2022_program
    };
    require_keys_eq!(
        *mint.owner,
        expected_token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *vault.owner,
        expected_token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let vault_data = vault.try_borrow_data()?;
    require!(vault_data.len() >= 72, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&vault_data, 0)?,
        mint.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&vault_data, 32)?,
        pool.key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(&vault_data, 64)? >= read_u64(data, base + 32)?,
        ArbitrageError::InvalidAccount
    );
    Ok(())
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

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    Ok(u128::from_le_bytes(
        data.get(offset..offset + 16)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    Ok(Pubkey::new_from_array(
        data.get(offset..offset + 32)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn swap_data(amount_in: u64, min_amount_out: u64, in_index: u8, out_index: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(26);
    data.extend_from_slice(&PERENA_NUMERAIRE_SWAP_DISCRIMINATOR);
    data.push(in_index);
    data.push(out_index);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account_with_data(
        key: Pubkey,
        owner: Pubkey,
        is_writable: bool,
        executable: bool,
        data: Vec<u8>,
    ) -> AccountInfo<'static> {
        AccountInfo::new(
            Box::leak(Box::new(key)),
            false,
            is_writable,
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

    fn pair_data(
        pool_data: &mut [u8],
        pair_index: usize,
        pool: Pubkey,
        mint: Pubkey,
        vault: Pubkey,
        is_token_2022: bool,
    ) {
        let base = PERENA_NUMERAIRE_PAIR_OFFSET + pair_index * PERENA_NUMERAIRE_PAIR_LEN;
        write_pubkey(pool_data, base, pool);
        pool_data[base + 32..base + 40].copy_from_slice(&1_000_000_u64.to_le_bytes());
        pool_data[base + 40..base + 48].copy_from_slice(&1_000_000_u64.to_le_bytes());
        pool_data[base + 48..base + 64].copy_from_slice(&1_u128.to_le_bytes());
        pool_data[base + 96..base + 112].copy_from_slice(&1_u128.to_le_bytes());
        write_pubkey(pool_data, base + 112, pool);
        write_pubkey(pool_data, base + 144, mint);
        write_pubkey(pool_data, base + 176, vault);
        pool_data[base + 233] = pair_index as u8;
        pool_data[base + 234] = u8::from(is_token_2022);
    }

    fn token_account_data(mint: Pubkey, authority: Pubkey) -> Vec<u8> {
        let mut data = vec![0_u8; 72];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&2_000_000_u64.to_le_bytes());
        data
    }

    #[test]
    fn instruction_data_matches_official_hintless_layout() {
        let data = swap_data(10_000_000, 9_900_000, 0, 1);
        assert_eq!(data.len(), 26);
        assert_eq!(&data[..8], &PERENA_NUMERAIRE_SWAP_DISCRIMINATOR);
        assert_eq!(&data[8..10], &[0, 1]);
        assert_eq!(&data[10..18], &10_000_000_u64.to_le_bytes());
        assert_eq!(&data[18..26], &9_900_000_u64.to_le_bytes());
    }

    #[test]
    fn semantic_validation_checks_pool_pairs_vaults_and_direction() {
        let program = Pubkey::new_unique();
        let pool_seed = Pubkey::new_unique();
        let pool = Pubkey::find_program_address(&[pool_seed.as_ref()], &program).0;
        let config = Pubkey::find_program_address(&[b"config"], &program).0;
        let mint_0 = Pubkey::new_unique();
        let mint_1 = Pubkey::new_unique();
        let vault_0 = Pubkey::new_unique();
        let vault_1 = Pubkey::new_unique();
        let token_program = anchor_spl::token::ID;
        let token_2022_program = anchor_spl::token_2022::ID;
        let mut pool_data = vec![0_u8; PERENA_NUMERAIRE_POOL_LEN];
        pool_data[..8].copy_from_slice(&PERENA_NUMERAIRE_POOL_DISCRIMINATOR);
        write_pubkey(&mut pool_data, 8, pool_seed);
        pool_data[3_884..3_888].copy_from_slice(&9_999_u32.to_le_bytes());
        pool_data[3_888..3_892].copy_from_slice(&10_000_u32.to_le_bytes());
        pool_data[3_893] = 2;
        pair_data(&mut pool_data, 0, pool, mint_0, vault_0, false);
        pair_data(&mut pool_data, 1, pool, mint_1, vault_1, true);

        let accounts = vec![
            test_account_with_data(program, Pubkey::default(), false, true, vec![]),
            test_account_with_data(pool, program, true, false, pool_data),
            test_account_with_data(mint_0, token_program, true, false, vec![]),
            test_account_with_data(mint_1, token_2022_program, true, false, vec![]),
            test_account_with_data(
                vault_0,
                token_program,
                true,
                false,
                token_account_data(mint_0, pool),
            ),
            test_account_with_data(
                vault_1,
                token_2022_program,
                true,
                false,
                token_account_data(mint_1, pool),
            ),
            test_account_with_data(config, program, false, false, vec![]),
            test_account_with_data(token_program, Pubkey::default(), false, true, vec![]),
            test_account_with_data(token_2022_program, Pubkey::default(), false, true, vec![]),
        ];

        assert!(validate_perena_numeraire_semantic_accounts(
            &accounts,
            0,
            &accounts[2],
            &accounts[3],
            &accounts[7],
            &accounts[8],
        )
        .is_ok());
        assert!(validate_perena_numeraire_semantic_accounts(
            &accounts,
            1,
            &accounts[3],
            &accounts[2],
            &accounts[7],
            &accounts[8],
        )
        .is_ok());

        let bad_index_offset = PERENA_NUMERAIRE_PAIR_OFFSET + PERENA_NUMERAIRE_PAIR_LEN + 233;
        accounts[1].try_borrow_mut_data().unwrap()[bad_index_offset] = 0;
        assert!(validate_perena_numeraire_semantic_accounts(
            &accounts,
            0,
            &accounts[2],
            &accounts[3],
            &accounts[7],
            &accounts[8],
        )
        .is_err());
    }
}
