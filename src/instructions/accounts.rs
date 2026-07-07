use anchor_lang::prelude::*;
use anchor_spl::{associated_token, token, token_2022};
use core::ops::Range;

use crate::errors::ArbitrageError;
use crate::protocal::{
    orca_whirlpool::ORCA_WHIRLPOOL_MIN_ACCOUNTS, pumpfun_amm::PUMPFUN_AMM_MIN_ACCOUNTS,
    raydium_clmm::RAYDIUM_CLMM_MIN_ACCOUNTS, raydium_cpmm::RAYDIUM_CPMM_MIN_ACCOUNTS,
    raydium_launchpad::RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
    raydium_pool_v4::RAYDIUM_POOL_V4_MIN_ACCOUNTS,
};
use crate::state::{Protocol, SwapArbParams, REMAINING_ACCOUNTS_FIXED_PREFIX_LEN};

pub const FIXED_REMAINING_ACCOUNTS: usize = REMAINING_ACCOUNTS_FIXED_PREFIX_LEN;

pub struct RouteAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub associated_token_program: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub token_2022_program: &'a AccountInfo<'info>,
    pub mints: &'a [AccountInfo<'info>],
    pub user_accounts: &'a [AccountInfo<'info>],
    pub step_ranges: Vec<Range<usize>>,
}

pub fn parse_route_accounts<'a, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    params: &SwapArbParams,
) -> Result<RouteAccounts<'a, 'info>> {
    let mints_count = mints_count_to_usize(params.mints_count);
    let step_ranges =
        calculate_step_ranges(remaining_accounts.len(), params.mints_count, &params.steps)?;
    require!(params.input_amount > 0, ArbitrageError::InvalidAmount);
    validate_fixed_programs(remaining_accounts)?;
    validate_route_account_flags(remaining_accounts, mints_count)?;
    let mints_start = FIXED_REMAINING_ACCOUNTS;
    let user_accounts_start = mints_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;
    let user_accounts_end = user_accounts_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;

    Ok(RouteAccounts {
        payer: &remaining_accounts[0],
        system_program: &remaining_accounts[1],
        associated_token_program: &remaining_accounts[2],
        token_program: &remaining_accounts[3],
        token_2022_program: &remaining_accounts[4],
        mints: &remaining_accounts[mints_start..user_accounts_start],
        user_accounts: &remaining_accounts[user_accounts_start..user_accounts_end],
        step_ranges,
    })
}

fn validate_route_account_flags<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    mints_count: usize,
) -> Result<()> {
    let payer = &remaining_accounts[0];
    require!(payer.is_signer, ArbitrageError::InvalidAccount);
    require!(payer.is_writable, ArbitrageError::InvalidAccount);

    for account in &remaining_accounts[1..FIXED_REMAINING_ACCOUNTS] {
        require!(account.executable, ArbitrageError::InvalidAccount);
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.is_writable, ArbitrageError::InvalidAccount);
    }

    let mints_start = FIXED_REMAINING_ACCOUNTS;
    let user_accounts_start = mints_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;
    let user_accounts_end = user_accounts_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;

    for mint in &remaining_accounts[mints_start..user_accounts_start] {
        require!(!mint.is_signer, ArbitrageError::InvalidAccount);
        require!(!mint.is_writable, ArbitrageError::InvalidAccount);
    }

    for user_account in &remaining_accounts[user_accounts_start..user_accounts_end] {
        require!(!user_account.is_signer, ArbitrageError::InvalidAccount);
        require!(user_account.is_writable, ArbitrageError::InvalidAccount);
    }

    Ok(())
}

fn validate_fixed_programs<'info>(remaining_accounts: &[AccountInfo<'info>]) -> Result<()> {
    require_keys_eq!(
        remaining_accounts[1].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        remaining_accounts[2].key(),
        associated_token::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        remaining_accounts[3].key(),
        token::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        remaining_accounts[4].key(),
        token_2022::ID,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

pub fn validate_step_account_flags<'info>(
    protocol: Protocol,
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    match protocol {
        Protocol::RaydiumCPMM => validate_fixed_len_step_account_flags(
            step_accounts,
            RAYDIUM_CPMM_MIN_ACCOUNTS,
            &[0],
            &[3, 4, 5, 6],
        ),
        Protocol::RaydiumCLMM => validate_variable_len_step_account_flags(
            step_accounts,
            RAYDIUM_CLMM_MIN_ACCOUNTS,
            7,
            &[0, 6],
            &[2, 3, 4, 5],
        ),
        Protocol::RaydiumPoolV4 => validate_fixed_len_step_account_flags(
            step_accounts,
            RAYDIUM_POOL_V4_MIN_ACCOUNTS,
            &[0, 7],
            &[1, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13],
        ),
        Protocol::RaydiumLaunchPad => validate_fixed_len_step_account_flags(
            step_accounts,
            RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
            &[0],
            &[4, 5, 6],
        ),
        Protocol::PumpFunSwap => validate_pumpfun_swap_step_account_flags(step_accounts),
        Protocol::PumpFunAMM => validate_pumpfun_amm_step_account_flags(step_accounts),
        Protocol::OrcaWhirlpool => validate_fixed_len_step_account_flags(
            step_accounts,
            ORCA_WHIRLPOOL_MIN_ACCOUNTS,
            &[0, 1, 2, 3],
            &[4, 5, 6, 9, 10, 11, 12],
        ),
    }
}

fn validate_fixed_len_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
    expected_len: usize,
    executable_readonly_indices: &[usize],
    writable_indices: &[usize],
) -> Result<()> {
    require!(
        step_accounts.len() == expected_len,
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        expected_len,
        executable_readonly_indices,
        writable_indices,
    )
}

fn validate_variable_len_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
    min_len: usize,
    fixed_prefix_len: usize,
    executable_readonly_indices: &[usize],
    writable_indices: &[usize],
) -> Result<()> {
    require!(
        fixed_prefix_len <= min_len,
        ArbitrageError::InvalidAccountCount
    );
    require!(
        step_accounts.len() >= min_len,
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        fixed_prefix_len,
        executable_readonly_indices,
        writable_indices,
    )?;

    for account in &step_accounts[fixed_prefix_len..] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }

    Ok(())
}

fn validate_step_account_flags_by_index<'info>(
    step_accounts: &[AccountInfo<'info>],
    checked_len: usize,
    executable_readonly_indices: &[usize],
    writable_indices: &[usize],
) -> Result<()> {
    for (index, account) in step_accounts[..checked_len].iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        if writable_indices.contains(&index) {
            require!(account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        } else {
            require!(!account.is_writable, ArbitrageError::InvalidAccount);
            require!(
                account.executable == executable_readonly_indices.contains(&index),
                ArbitrageError::InvalidAccount
            );
        }
    }
    Ok(())
}

fn validate_pumpfun_swap_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == 17 || step_accounts.len() == 18,
        ArbitrageError::InvalidAccountCount
    );
    require!(step_accounts[0].executable, ArbitrageError::InvalidAccount);
    require!(
        !step_accounts[0].is_writable,
        ArbitrageError::InvalidAccount
    );
    let fee_program_index = step_accounts.len() - 1;
    require!(
        step_accounts[fee_program_index].executable,
        ArbitrageError::InvalidAccount
    );
    require!(
        !step_accounts[fee_program_index].is_writable,
        ArbitrageError::InvalidAccount
    );

    let writable_indices: &[usize] = if step_accounts.len() == 18 {
        &[2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 14]
    } else {
        &[2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13]
    };
    for (index, account) in step_accounts.iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        if index == 0 || index == fee_program_index {
            continue;
        }
        if writable_indices.contains(&index) {
            require!(account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        } else {
            require!(!account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        }
    }
    Ok(())
}

fn validate_pumpfun_amm_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == 12 || step_accounts.len() == 14 || step_accounts.len() == 16,
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        PUMPFUN_AMM_MIN_ACCOUNTS,
        &[0],
        &[1, 3, 4, 6, 8],
    )?;
    validate_pumpfun_dynamic_suffix_flags(step_accounts, PUMPFUN_AMM_MIN_ACCOUNTS)
}

fn validate_pumpfun_dynamic_suffix_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
    suffix_start: usize,
) -> Result<()> {
    let tail_len = step_accounts
        .len()
        .checked_sub(suffix_start)
        .ok_or(ArbitrageError::InvalidAccountCount)?;
    require!(
        tail_len == 2 || tail_len == 4 || tail_len == 6,
        ArbitrageError::InvalidAccountCount
    );
    let fee_config_index = match tail_len {
        2 | 4 => step_accounts
            .len()
            .checked_sub(2)
            .ok_or(ArbitrageError::InvalidAccountCount)?,
        6 => suffix_start
            .checked_add(2)
            .ok_or(ArbitrageError::InvalidAccountCount)?,
        _ => return Err(ArbitrageError::InvalidAccountCount.into()),
    };

    for account in &step_accounts[suffix_start..fee_config_index] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }
    for account in &step_accounts[fee_config_index + 2..] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }

    let fee_config = &step_accounts[fee_config_index];
    require!(!fee_config.is_signer, ArbitrageError::InvalidAccount);
    require!(!fee_config.is_writable, ArbitrageError::InvalidAccount);
    require!(!fee_config.executable, ArbitrageError::InvalidAccount);

    let fee_program = &step_accounts[fee_config_index + 1];
    require!(!fee_program.is_signer, ArbitrageError::InvalidAccount);
    require!(!fee_program.is_writable, ArbitrageError::InvalidAccount);
    require!(fee_program.executable, ArbitrageError::InvalidAccount);

    Ok(())
}

pub fn calculate_step_ranges(
    remaining_accounts_len: usize,
    mints_count: u8,
    steps: &[crate::state::SwapStepMeta],
) -> Result<Vec<Range<usize>>> {
    require!(mints_count > 1, ArbitrageError::InvalidAccountCount);
    let m = mints_count_to_usize(mints_count);
    require!(steps.len() == m, ArbitrageError::InvalidAccountCount);

    for step in steps {
        step.validate()?;
    }

    let base_len = FIXED_REMAINING_ACCOUNTS
        .checked_add(m.checked_mul(2).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        remaining_accounts_len >= base_len,
        ArbitrageError::InvalidAccountCount
    );

    let declared_steps_len = steps.iter().try_fold(0usize, |acc, s| {
        acc.checked_add(step_accounts_len_to_usize(s.accounts_len))
            .ok_or(ArbitrageError::MathOverflow)
    })?;
    let actual_steps_len = remaining_accounts_len
        .checked_sub(base_len)
        .ok_or(ArbitrageError::InvalidAccountCount)?;
    require!(
        actual_steps_len == declared_steps_len,
        ArbitrageError::InvalidAccountCount
    );

    let mut cursor = base_len;
    let mut ranges = Vec::with_capacity(steps.len());
    for step in steps {
        let step_len = step_accounts_len_to_usize(step.accounts_len);
        let end = cursor
            .checked_add(step_len)
            .ok_or(ArbitrageError::MathOverflow)?;
        ranges.push(cursor..end);
        cursor = end;
    }

    Ok(ranges)
}

fn mints_count_to_usize(mints_count: u8) -> usize {
    usize::from(mints_count)
}

fn step_accounts_len_to_usize(accounts_len: u8) -> usize {
    usize::from(accounts_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Protocol, SwapStepMeta};

    fn test_account(
        key: Pubkey,
        owner: Pubkey,
        is_signer: bool,
        is_writable: bool,
        executable: bool,
    ) -> AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(owner));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            lamports,
            data,
            owner,
            executable,
            0,
        )
    }

    fn step(accounts_len: u8) -> SwapStepMeta {
        SwapStepMeta {
            protocol: Protocol::RaydiumCPMM,
            accounts_len,
            direction: 0,
            fee_rate: 0,
            min_output_amount: 0,
        }
    }

    fn params_for_two_mints() -> SwapArbParams {
        SwapArbParams {
            mints_count: 2,
            input_amount: 1,
            min_profit_lamports: 0,
            steps: vec![step(0), step(0)],
        }
    }

    fn valid_route_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            test_account(
                Pubkey::new_unique(),
                anchor_lang::system_program::ID,
                true,
                true,
                false,
            ),
            test_account(
                anchor_lang::system_program::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(
                associated_token::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(
                token::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(
                token_2022::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(Pubkey::new_unique(), token::ID, false, false, false),
            test_account(Pubkey::new_unique(), token::ID, false, false, false),
            test_account(Pubkey::new_unique(), token::ID, false, true, false),
            test_account(Pubkey::new_unique(), token::ID, false, true, false),
        ]
    }

    fn step_account(is_writable: bool, executable: bool) -> AccountInfo<'static> {
        test_account(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            false,
            is_writable,
            executable,
        )
    }

    fn readonly_step_account() -> AccountInfo<'static> {
        step_account(false, false)
    }

    fn writable_step_account() -> AccountInfo<'static> {
        step_account(true, false)
    }

    fn executable_step_account() -> AccountInfo<'static> {
        step_account(false, true)
    }

    fn cpmm_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn clmm_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            writable_step_account(),
        ]
    }

    fn pumpfun_swap_buy_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
        ]
    }

    fn pumpfun_amm_step_accounts_with_volume() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
        ]
    }

    fn orca_whirlpool_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    #[test]
    fn calculates_ranges_after_fixed_mints_and_user_accounts() {
        let steps = vec![step(7), step(8), step(10)];
        let ranges = calculate_step_ranges(5 + 2 * 3 + 25, 3, &steps).unwrap();

        assert_eq!(ranges, vec![11..18, 18..26, 26..36]);
    }

    #[test]
    fn rejects_missing_fixed_accounts() {
        let steps = vec![step(7), step(8)];
        let err = calculate_step_ranges(2 * 2 + 15, 2, &steps).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn rejects_mismatched_step_account_lengths() {
        let steps = vec![step(7), step(8)];
        let err = calculate_step_ranges(5 + 2 * 2 + 14, 2, &steps).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn rejects_invalid_direction_values() {
        let mut steps = vec![step(7), step(8)];
        steps[1].direction = 2;

        let err = calculate_step_ranges(5 + 2 * 2 + 15, 2, &steps).unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidInstructionData.into());
    }

    #[test]
    fn parse_route_accounts_accepts_expected_account_flags() {
        let accounts = valid_route_accounts();
        let route_accounts =
            parse_route_accounts(&accounts, &params_for_two_mints()).expect("route accounts");

        assert_eq!(route_accounts.mints.len(), 2);
        assert_eq!(route_accounts.user_accounts.len(), 2);
    }

    #[test]
    fn parse_route_accounts_rejects_non_signer_payer() {
        let mut accounts = valid_route_accounts();
        accounts[0].is_signer = false;

        let err = match parse_route_accounts(&accounts, &params_for_two_mints()) {
            Ok(_) => panic!("payer must sign"),
            Err(err) => err,
        };

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn parse_route_accounts_rejects_readonly_user_token_account() {
        let mut accounts = valid_route_accounts();
        accounts[7].is_writable = false;

        let err = match parse_route_accounts(&accounts, &params_for_two_mints()) {
            Ok(_) => panic!("user token account must be writable"),
            Err(err) => err,
        };

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn parse_route_accounts_rejects_writable_mint() {
        let mut accounts = valid_route_accounts();
        accounts[5].is_writable = true;

        let err = match parse_route_accounts(&accounts, &params_for_two_mints()) {
            Ok(_) => panic!("mint readonly"),
            Err(err) => err,
        };

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_expected_cpmm_flags() {
        let accounts = cpmm_step_accounts();

        assert!(validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_rejects_writable_readonly_cpmm_account() {
        let mut accounts = cpmm_step_accounts();
        accounts[1].is_writable = true;

        let err = validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_cpmm_vault() {
        let mut accounts = cpmm_step_accounts();
        accounts[4].is_writable = false;

        let err = validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_extra_fixed_len_cpmm_account() {
        let mut accounts = cpmm_step_accounts();
        accounts.push(readonly_step_account());

        let err = validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_clmm_dynamic_account_flags() {
        let accounts = clmm_step_accounts();

        assert!(validate_step_account_flags(Protocol::RaydiumCLMM, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_rejects_signer_dynamic_account() {
        let mut accounts = clmm_step_accounts();
        accounts[7].is_signer = true;

        let err = validate_step_account_flags(Protocol::RaydiumCLMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_pumpfun_fee_program_suffix() {
        let swap_accounts = pumpfun_swap_buy_step_accounts();
        assert!(validate_step_account_flags(Protocol::PumpFunSwap, &swap_accounts).is_ok());

        let amm_accounts = pumpfun_amm_step_accounts_with_volume();
        assert!(validate_step_account_flags(Protocol::PumpFunAMM, &amm_accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_orca_whirlpool_accounts() {
        let accounts = orca_whirlpool_step_accounts();

        assert!(validate_step_account_flags(Protocol::OrcaWhirlpool, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_orca_tick_array() {
        let mut accounts = orca_whirlpool_step_accounts();
        accounts[9].is_writable = false;

        let err = validate_step_account_flags(Protocol::OrcaWhirlpool, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_non_executable_pumpfun_fee_program() {
        let mut accounts = pumpfun_swap_buy_step_accounts();
        accounts[17].executable = false;

        let err = validate_step_account_flags(Protocol::PumpFunSwap, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_unexpected_pumpfun_tail_length() {
        let mut accounts = pumpfun_amm_step_accounts_with_volume();
        accounts.remove(11);

        let err = validate_step_account_flags(Protocol::PumpFunAMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn route_account_lengths_use_lossless_index_helpers() {
        assert_eq!(mints_count_to_usize(7), 7);
        assert_eq!(step_accounts_len_to_usize(9), 9);

        for (name, source) in [
            ("accounts", include_str!("accounts.rs")),
            ("execute_arbitrage", include_str!("execute_arbitrage.rs")),
        ] {
            let production = production_source_text(source);
            for forbidden in [
                "params.mints_count as usize",
                "mints_count as usize",
                "accounts_len as usize",
                "2 * mints_count",
            ] {
                assert!(
                    !production.contains(forbidden),
                    "{name} route account boundary math must use checked helpers instead of {forbidden}"
                );
            }
        }
    }

    fn production_source_text(source: &str) -> &str {
        match source.split_once("\n#[cfg(test)]") {
            Some((production, _)) => production,
            None => source,
        }
    }
}
