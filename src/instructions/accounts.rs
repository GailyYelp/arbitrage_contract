use anchor_lang::prelude::*;
use anchor_spl::{associated_token, token, token_2022};
use core::ops::Range;

use crate::errors::ArbitrageError;
use crate::state::SwapArbParams;

pub const FIXED_REMAINING_ACCOUNTS: usize = 5;

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
    let mints_count = params.mints_count as usize;
    let step_ranges =
        calculate_step_ranges(remaining_accounts.len(), params.mints_count, &params.steps)?;
    require!(params.input_amount > 0, ArbitrageError::InvalidAmount);
    validate_fixed_programs(remaining_accounts)?;

    Ok(RouteAccounts {
        payer: &remaining_accounts[0],
        system_program: &remaining_accounts[1],
        associated_token_program: &remaining_accounts[2],
        token_program: &remaining_accounts[3],
        token_2022_program: &remaining_accounts[4],
        mints: &remaining_accounts
            [FIXED_REMAINING_ACCOUNTS..FIXED_REMAINING_ACCOUNTS + mints_count],
        user_accounts: &remaining_accounts
            [FIXED_REMAINING_ACCOUNTS + mints_count..FIXED_REMAINING_ACCOUNTS + 2 * mints_count],
        step_ranges,
    })
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

pub fn calculate_step_ranges(
    remaining_accounts_len: usize,
    mints_count: u8,
    steps: &[crate::state::SwapStepMeta],
) -> Result<Vec<Range<usize>>> {
    require!(mints_count > 1, ArbitrageError::InvalidAccountCount);
    require!(
        steps.len() == mints_count as usize,
        ArbitrageError::InvalidAccountCount
    );

    for step in steps {
        step.validate()?;
    }

    let m = mints_count as usize;
    let base_len = FIXED_REMAINING_ACCOUNTS
        .checked_add(2usize.checked_mul(m).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        remaining_accounts_len >= base_len,
        ArbitrageError::InvalidAccountCount
    );

    let declared_steps_len = steps.iter().try_fold(0usize, |acc, s| {
        acc.checked_add(s.accounts_len as usize)
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
        let step_len = step.accounts_len as usize;
        let end = cursor
            .checked_add(step_len)
            .ok_or(ArbitrageError::MathOverflow)?;
        ranges.push(cursor..end);
        cursor = end;
    }

    Ok(ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Protocol, SwapStepMeta};

    fn step(accounts_len: u8) -> SwapStepMeta {
        SwapStepMeta {
            protocol: Protocol::RaydiumCPMM,
            accounts_len,
            direction: 0,
            fee_rate: 0,
        }
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
}
