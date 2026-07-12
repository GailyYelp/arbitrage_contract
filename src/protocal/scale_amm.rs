use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const SCALE_AMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SCALEwAvEK5gtkdHiFzXfPgtk2YwJxPDzaV3aDmR7tA");
pub const SCALE_AMM_BASE_STEP_ACCOUNTS: usize = 8;
pub const SCALE_AMM_MAX_STEP_ACCOUNTS: usize = 13;

const POOL_ACCOUNT_LEN: usize = 326;
const CONFIG_ACCOUNT_LEN: usize = 107;
const MAX_BENEFICIARIES: usize = 5;
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const CONFIG_DISCRIMINATOR: [u8; 8] = [160, 78, 128, 0, 248, 83, 230, 160];
const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

pub struct ScaleAmmAccounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub pool: &'a AccountInfo<'info>,
    pub owner: &'a AccountInfo<'info>,
    pub mint_a: &'a AccountInfo<'info>,
    pub mint_b: &'a AccountInfo<'info>,
    pub user_ta_a: &'a AccountInfo<'info>,
    pub user_ta_b: &'a AccountInfo<'info>,
    pub vault_a: &'a AccountInfo<'info>,
    pub vault_b: &'a AccountInfo<'info>,
    pub platform_fee_ta_a: &'a AccountInfo<'info>,
    pub token_program_a: &'a AccountInfo<'info>,
    pub token_program_b: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub config: &'a AccountInfo<'info>,
    pub user: &'a AccountInfo<'info>,
    pub user_token_out: &'a AccountInfo<'info>,
    pub beneficiaries: &'a [AccountInfo<'info>],
}

pub fn scale_amm_swap<'info>(
    accounts: ScaleAmmAccounts<'_, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let pre_out = read_token_amount(accounts.user_token_out)?;
    let mut metas = vec![
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new(accounts.user.key(), true),
        AccountMeta::new_readonly(accounts.owner.key(), false),
        AccountMeta::new_readonly(accounts.mint_a.key(), false),
        AccountMeta::new_readonly(accounts.mint_b.key(), false),
        AccountMeta::new(accounts.user_ta_a.key(), false),
        AccountMeta::new(accounts.user_ta_b.key(), false),
        AccountMeta::new(accounts.vault_a.key(), false),
        AccountMeta::new(accounts.vault_b.key(), false),
        AccountMeta::new(accounts.platform_fee_ta_a.key(), false),
        AccountMeta::new_readonly(accounts.token_program_a.key(), false),
        AccountMeta::new_readonly(accounts.token_program_b.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.config.key(), false),
    ];
    let mut infos = vec![
        accounts.pool.clone(),
        accounts.user.clone(),
        accounts.owner.clone(),
        accounts.mint_a.clone(),
        accounts.mint_b.clone(),
        accounts.user_ta_a.clone(),
        accounts.user_ta_b.clone(),
        accounts.vault_a.clone(),
        accounts.vault_b.clone(),
        accounts.platform_fee_ta_a.clone(),
        accounts.token_program_a.clone(),
        accounts.token_program_b.clone(),
        accounts.system_program.clone(),
        accounts.config.clone(),
    ];
    for beneficiary in accounts.beneficiaries {
        metas.push(AccountMeta::new(beneficiary.key(), false));
        infos.push(beneficiary.clone());
    }
    infos.push(accounts.program.clone());
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: swap_data(amount_in, min_amount_out, direction).to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_token_out, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_scale_amm_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    token_2022_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        (SCALE_AMM_BASE_STEP_ACCOUNTS..=SCALE_AMM_MAX_STEP_ACCOUNTS).contains(&step.len()),
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[0].key(),
        SCALE_AMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        SCALE_AMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        anchor_lang::system_program::ID,
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

    let pool_data = step[1].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_ACCOUNT_LEN && pool_data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        *pool_data.get(8).ok_or(ArbitrageError::InvalidAccount)? == 1,
        ArbitrageError::InvalidAccount
    );
    let owner = read_pubkey(&pool_data, 9)?;
    let mint_a = read_pubkey(&pool_data, 41)?;
    let mint_b = read_pubkey(&pool_data, 73)?;
    let reserve_a = read_u128(&pool_data, 105)?;
    let reserve_b = read_u128(&pool_data, 121)?;
    let shift = read_u128(&pool_data, 137)?;
    let curve = *pool_data.get(153).ok_or(ArbitrageError::InvalidAccount)?;
    let beneficiary_count = usize::from(*pool_data.get(154).ok_or(ArbitrageError::InvalidAccount)?);
    let bump = *pool_data.get(325).ok_or(ArbitrageError::InvalidAccount)?;
    require!(
        owner != Pubkey::default()
            && mint_a != Pubkey::default()
            && mint_b != Pubkey::default()
            && mint_a != mint_b,
        ArbitrageError::InvalidAccount
    );
    require!(
        reserve_a > 0 && reserve_b > 0,
        ArbitrageError::InvalidAccount
    );
    require!(
        reserve_a <= u128::from(u64::MAX) && reserve_b <= u128::from(u64::MAX),
        ArbitrageError::InvalidAccount
    );
    require!(
        reserve_a.checked_add(shift).is_some(),
        ArbitrageError::MathOverflow
    );
    require!(curve <= 1, ArbitrageError::InvalidAccount);
    require!(
        beneficiary_count <= MAX_BENEFICIARIES
            && step.len() == SCALE_AMM_BASE_STEP_ACCOUNTS + beneficiary_count,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(step[2].key(), owner, ArbitrageError::InvalidAccount);
    let (expected_pool, expected_bump) = Pubkey::find_program_address(
        &[b"pool", owner.as_ref(), mint_a.as_ref(), mint_b.as_ref()],
        &SCALE_AMM_PROGRAM_ID,
    );
    require_keys_eq!(step[1].key(), expected_pool, ArbitrageError::InvalidAccount);
    require!(bump == expected_bump, ArbitrageError::InvalidAccount);

    let config_address = Pubkey::find_program_address(&[b"config"], &SCALE_AMM_PROGRAM_ID);
    require_keys_eq!(
        step[7].key(),
        config_address.0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[7].owner,
        SCALE_AMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let config_data = step[7].try_borrow_data()?;
    require!(
        config_data.len() == CONFIG_ACCOUNT_LEN && config_data.starts_with(&CONFIG_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let platform_wallet = read_pubkey(&config_data, 40)?;
    let base_token = read_pubkey(&config_data, 72)?;
    let platform_fee_bps = read_u16(&config_data, 104)?;
    let config_bump = *config_data.get(106).ok_or(ArbitrageError::InvalidAccount)?;
    require!(
        config_bump == config_address.1
            && platform_wallet != Pubkey::default()
            && base_token != Pubkey::default()
            && platform_fee_bps < 10_000,
        ArbitrageError::InvalidAccount
    );
    require!(
        base_token == mint_a || base_token == mint_b,
        ArbitrageError::InvalidAccount
    );

    let token_program_a = token_program_for_mint_owner(
        if direction == 0 { in_mint } else { out_mint },
        token_program,
        token_2022_program,
    )?;
    let token_program_b = token_program_for_mint_owner(
        if direction == 0 { out_mint } else { in_mint },
        token_program,
        token_2022_program,
    )?;
    let (expected_in, expected_out) = if direction == 0 {
        (mint_a, mint_b)
    } else {
        (mint_b, mint_a)
    };
    require_keys_eq!(in_mint.key(), expected_in, ArbitrageError::InvalidAccount);
    require_keys_eq!(out_mint.key(), expected_out, ArbitrageError::InvalidAccount);

    let expected_vault_a = Pubkey::find_program_address(
        &[step[1].key().as_ref(), mint_a.as_ref()],
        &SCALE_AMM_PROGRAM_ID,
    )
    .0;
    let expected_vault_b = Pubkey::find_program_address(
        &[step[1].key().as_ref(), mint_b.as_ref()],
        &SCALE_AMM_PROGRAM_ID,
    )
    .0;
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
    validate_token_account(
        &step[3],
        mint_a,
        step[1].key(),
        Some(reserve_a as u64),
        token_program_a,
    )?;
    validate_token_account(
        &step[4],
        mint_b,
        step[1].key(),
        Some(reserve_b as u64),
        token_program_b,
    )?;

    let expected_platform =
        associated_token_address(platform_wallet, mint_a, token_program_a.key());
    require_keys_eq!(
        step[5].key(),
        expected_platform,
        ArbitrageError::InvalidAccount
    );
    validate_token_account(&step[5], mint_a, platform_wallet, None, token_program_a)?;

    let mut total_fee_bps = u32::from(platform_fee_bps);
    let mut wallets = [Pubkey::default(); MAX_BENEFICIARIES];
    for index in 0..beneficiary_count {
        let offset = 155usize
            .checked_add(index.checked_mul(34).ok_or(ArbitrageError::MathOverflow)?)
            .ok_or(ArbitrageError::MathOverflow)?;
        let wallet = read_pubkey(&pool_data, offset)?;
        let share_bps = read_u16(&pool_data, offset + 32)?;
        require!(
            wallet != Pubkey::default() && share_bps < 10_000,
            ArbitrageError::InvalidAccount
        );
        for previous in &wallets[..index] {
            require!(*previous != wallet, ArbitrageError::InvalidAccount);
        }
        wallets[index] = wallet;
        total_fee_bps = total_fee_bps
            .checked_add(u32::from(share_bps))
            .ok_or(ArbitrageError::MathOverflow)?;
        let expected = associated_token_address(wallet, mint_a, token_program_a.key());
        require_keys_eq!(
            step[SCALE_AMM_BASE_STEP_ACCOUNTS + index].key(),
            expected,
            ArbitrageError::InvalidAccount
        );
        validate_token_account(
            &step[SCALE_AMM_BASE_STEP_ACCOUNTS + index],
            mint_a,
            wallet,
            None,
            token_program_a,
        )?;
    }
    require!(total_fee_bps < 10_000, ArbitrageError::InvalidAccount);
    require!(
        u32::from(fee_rate) == total_fee_bps,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

pub fn token_programs_for_direction<'a, 'info>(
    direction: u8,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &'a AccountInfo<'info>,
    token_2022_program: &'a AccountInfo<'info>,
) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>)> {
    let input_program = token_program_for_mint_owner(in_mint, token_program, token_2022_program)?;
    let output_program = token_program_for_mint_owner(out_mint, token_program, token_2022_program)?;
    if direction == 0 {
        Ok((input_program, output_program))
    } else {
        Ok((output_program, input_program))
    }
}

fn token_program_for_mint_owner<'a, 'info>(
    mint: &AccountInfo<'info>,
    token_program: &'a AccountInfo<'info>,
    token_2022_program: &'a AccountInfo<'info>,
) -> Result<&'a AccountInfo<'info>> {
    if mint.owner == token_program.key {
        Ok(token_program)
    } else if mint.owner == token_2022_program.key {
        Ok(token_2022_program)
    } else {
        Err(ArbitrageError::InvalidAccount.into())
    }
}

fn associated_token_address(wallet: Pubkey, mint: Pubkey, token_program: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &anchor_spl::associated_token::ID,
    )
    .0
}

fn validate_token_account<'info>(
    account: &AccountInfo<'info>,
    expected_mint: Pubkey,
    expected_authority: Pubkey,
    expected_amount: Option<u64>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(data.len() >= 72, ArbitrageError::InvalidAccount);
    require!(
        read_pubkey(&data, 0)? == expected_mint && read_pubkey(&data, 32)? == expected_authority,
        ArbitrageError::InvalidAccount
    );
    if let Some(expected) = expected_amount {
        require!(
            read_u64(&data, 64)? == expected,
            ArbitrageError::InvalidAccount
        );
    }
    Ok(())
}

fn swap_data(amount: u64, limit: u64, direction: u8) -> [u8; 24] {
    let mut data = [0u8; 24];
    data[..8].copy_from_slice(if direction == 0 {
        &BUY_DISCRIMINATOR
    } else {
        &SELL_DISCRIMINATOR
    });
    data[8..16].copy_from_slice(&amount.to_le_bytes());
    data[16..24].copy_from_slice(&limit.to_le_bytes());
    data
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

    fn test_account(
        key: Pubkey,
        owner: Pubkey,
        is_writable: bool,
        executable: bool,
        data: Vec<u8>,
    ) -> AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(owner));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(data.into_boxed_slice());
        AccountInfo::new(
            key,
            false,
            is_writable,
            lamports,
            data,
            owner,
            executable,
            0,
        )
    }

    fn token_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0u8; 165];
        data[0..32].copy_from_slice(mint.as_ref());
        data[32..64].copy_from_slice(owner.as_ref());
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data
    }

    #[test]
    fn wire_data_matches_onchain_anchor_idl() {
        let buy = swap_data(1_000_000, 900_000, 0);
        let sell = swap_data(1_000_000, 900_000, 1);
        assert_eq!(&buy[..8], &BUY_DISCRIMINATOR);
        assert_eq!(&sell[..8], &SELL_DISCRIMINATOR);
        assert_eq!(&buy[8..16], &1_000_000_u64.to_le_bytes());
        assert_eq!(&buy[16..24], &900_000_u64.to_le_bytes());
    }

    #[test]
    fn semantic_validation_binds_dynamic_fee_accounts_and_both_directions() {
        let owner = Pubkey::new_unique();
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();
        let platform_wallet = Pubkey::new_unique();
        let beneficiary_wallet = Pubkey::new_unique();
        let token_program_key = anchor_spl::token::ID;
        let token_2022_program_key = anchor_spl::token_2022::ID;
        let (pool, pool_bump) = Pubkey::find_program_address(
            &[b"pool", owner.as_ref(), mint_a.as_ref(), mint_b.as_ref()],
            &SCALE_AMM_PROGRAM_ID,
        );
        let vault_a =
            Pubkey::find_program_address(&[pool.as_ref(), mint_a.as_ref()], &SCALE_AMM_PROGRAM_ID)
                .0;
        let vault_b =
            Pubkey::find_program_address(&[pool.as_ref(), mint_b.as_ref()], &SCALE_AMM_PROGRAM_ID)
                .0;
        let config = Pubkey::find_program_address(&[b"config"], &SCALE_AMM_PROGRAM_ID);
        let platform_ata = associated_token_address(platform_wallet, mint_a, token_program_key);
        let beneficiary_ata =
            associated_token_address(beneficiary_wallet, mint_a, token_program_key);
        let reserve_a = 635_210_081_512_u64;
        let reserve_b = 67_980_392_974_486_u64;

        let mut pool_data = vec![0u8; POOL_ACCOUNT_LEN];
        pool_data[..8].copy_from_slice(&POOL_DISCRIMINATOR);
        pool_data[8] = 1;
        pool_data[9..41].copy_from_slice(owner.as_ref());
        pool_data[41..73].copy_from_slice(mint_a.as_ref());
        pool_data[73..105].copy_from_slice(mint_b.as_ref());
        pool_data[105..121].copy_from_slice(&u128::from(reserve_a).to_le_bytes());
        pool_data[121..137].copy_from_slice(&u128::from(reserve_b).to_le_bytes());
        pool_data[137..153].copy_from_slice(&46_331_461_930_u128.to_le_bytes());
        pool_data[153] = 0;
        pool_data[154] = 1;
        pool_data[155..187].copy_from_slice(beneficiary_wallet.as_ref());
        pool_data[187..189].copy_from_slice(&100_u16.to_le_bytes());
        pool_data[325] = pool_bump;

        let mut config_data = vec![0u8; CONFIG_ACCOUNT_LEN];
        config_data[..8].copy_from_slice(&CONFIG_DISCRIMINATOR);
        config_data[8..40].copy_from_slice(Pubkey::new_unique().as_ref());
        config_data[40..72].copy_from_slice(platform_wallet.as_ref());
        config_data[72..104].copy_from_slice(mint_b.as_ref());
        config_data[104..106].copy_from_slice(&20_u16.to_le_bytes());
        config_data[106] = config.1;

        let step = vec![
            test_account(
                SCALE_AMM_PROGRAM_ID,
                Pubkey::new_unique(),
                false,
                true,
                Vec::new(),
            ),
            test_account(pool, SCALE_AMM_PROGRAM_ID, true, false, pool_data),
            test_account(
                owner,
                anchor_lang::system_program::ID,
                false,
                false,
                Vec::new(),
            ),
            test_account(
                vault_a,
                token_program_key,
                true,
                false,
                token_data(mint_a, pool, reserve_a),
            ),
            test_account(
                vault_b,
                token_program_key,
                true,
                false,
                token_data(mint_b, pool, reserve_b),
            ),
            test_account(
                platform_ata,
                token_program_key,
                true,
                false,
                token_data(mint_a, platform_wallet, 1),
            ),
            test_account(
                anchor_lang::system_program::ID,
                Pubkey::new_unique(),
                false,
                true,
                Vec::new(),
            ),
            test_account(config.0, SCALE_AMM_PROGRAM_ID, false, false, config_data),
            test_account(
                beneficiary_ata,
                token_program_key,
                true,
                false,
                token_data(mint_a, beneficiary_wallet, 1),
            ),
        ];
        let mint_a_account = test_account(mint_a, token_program_key, false, false, vec![0; 82]);
        let mint_b_account = test_account(mint_b, token_program_key, false, false, vec![0; 82]);
        let token_program = test_account(
            token_program_key,
            Pubkey::new_unique(),
            false,
            true,
            Vec::new(),
        );
        let token_2022_program = test_account(
            token_2022_program_key,
            Pubkey::new_unique(),
            false,
            true,
            Vec::new(),
        );

        validate_scale_amm_semantic_accounts(
            &step,
            0,
            120,
            &mint_a_account,
            &mint_b_account,
            &token_program,
            &token_2022_program,
        )
        .expect("validate Scale AMM buy");
        validate_scale_amm_semantic_accounts(
            &step,
            1,
            120,
            &mint_b_account,
            &mint_a_account,
            &token_program,
            &token_2022_program,
        )
        .expect("validate Scale AMM sell");
        assert!(validate_scale_amm_semantic_accounts(
            &step,
            0,
            119,
            &mint_a_account,
            &mint_b_account,
            &token_program,
            &token_2022_program,
        )
        .is_err());
    }
}
