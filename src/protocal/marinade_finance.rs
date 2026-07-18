#![allow(dead_code)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
    system_instruction,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::MARINADE_FINANCE_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint,
            validate_token_account_for_mint_and_authority, SwapResult,
        },
    },
    protocal::wsol::{close_wsol_for_native, restore_wsol_after_native, WsolBridgeAccounts},
};

pub const MARINADE_FINANCE_STEP_ACCOUNTS: usize = 11;
pub const MARINADE_DEPOSIT_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
pub const MARINADE_LIQUID_UNSTAKE_DISCRIMINATOR: [u8; 8] = [30, 30, 119, 240, 191, 227, 12, 16];
const MARINADE_STATE: Pubkey = anchor_lang::pubkey!("8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC");
const MARINADE_MSOL_MINT: Pubkey =
    anchor_lang::pubkey!("mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So");
const MARINADE_WSOL_MINT: Pubkey =
    anchor_lang::pubkey!("So11111111111111111111111111111111111111112");
const MARINADE_STATE_DISCRIMINATOR: [u8; 8] = [216, 146, 107, 94, 104, 75, 182, 177];
const FEE_BPS_DENOMINATOR: u32 = 10_000;
const FEE_CENTS_DENOMINATOR: u32 = 1_000_000;

#[derive(Clone, AnchorDeserialize)]
struct Fee {
    basis_points: u32,
}

#[derive(Clone, AnchorDeserialize)]
struct FeeCents {
    bp_cents: u32,
}

#[derive(Clone, AnchorDeserialize)]
struct List {
    account: Pubkey,
    item_size: u32,
    count: u32,
    reserved1: Pubkey,
    reserved2: u32,
}

#[derive(Clone, AnchorDeserialize)]
struct StakeSystem {
    stake_list: List,
    delayed_unstake_cooling_down: u64,
    stake_deposit_bump_seed: u8,
    stake_withdraw_bump_seed: u8,
    slots_for_stake_delta: u64,
    last_stake_delta_epoch: u64,
    min_stake: u64,
    extra_stake_delta_runs: u32,
}

#[derive(Clone, AnchorDeserialize)]
struct ValidatorSystem {
    validator_list: List,
    manager_authority: Pubkey,
    total_validator_score: u32,
    total_active_balance: u64,
    auto_add_validator_enabled: u8,
}

#[derive(Clone, AnchorDeserialize)]
struct LiqPool {
    lp_mint: Pubkey,
    lp_mint_authority_bump_seed: u8,
    sol_leg_bump_seed: u8,
    msol_leg_authority_bump_seed: u8,
    msol_leg: Pubkey,
    lp_liquidity_target: u64,
    lp_max_fee: Fee,
    lp_min_fee: Fee,
    treasury_cut: Fee,
    lp_supply: u64,
    lent_from_sol_leg: u64,
    liquidity_sol_cap: u64,
}

#[derive(Clone, AnchorDeserialize)]
enum DelinquentUpgraderState {
    IteratingStakes {
        visited_count: u32,
        total_active_balance: u64,
        total_delinquent_balance: u64,
    },
    IteratingValidators {
        visited_count: u32,
        delinquent_balance_left: u64,
    },
    Done,
}

#[derive(Clone, AnchorDeserialize)]
struct MarinadeState {
    msol_mint: Pubkey,
    admin_authority: Pubkey,
    operational_sol_account: Pubkey,
    treasury_msol_account: Pubkey,
    reserve_bump_seed: u8,
    msol_mint_authority_bump_seed: u8,
    rent_exempt_for_token_acc: u64,
    reward_fee: Fee,
    stake_system: StakeSystem,
    validator_system: ValidatorSystem,
    liq_pool: LiqPool,
    available_reserve_balance: u64,
    msol_supply: u64,
    msol_price: u64,
    circulating_ticket_count: u64,
    circulating_ticket_balance: u64,
    lent_from_reserve: u64,
    min_deposit: u64,
    min_withdraw: u64,
    staking_sol_cap: u64,
    emergency_cooling_down: u64,
    pause_authority: Pubkey,
    paused: bool,
    delayed_unstake_fee: FeeCents,
    withdraw_stake_account_fee: FeeCents,
    withdraw_stake_account_enabled: bool,
    last_stake_move_epoch: u64,
    stake_moved: u64,
    max_stake_moved_per_epoch: Fee,
    delinquent_upgrader: DelinquentUpgraderState,
    deposit_sol_fee: FeeCents,
    deposit_stake_account_fee: FeeCents,
}

fn mint_supply_and_authority(data: &[u8]) -> Result<(u64, Pubkey)> {
    require!(data.len() >= 82, ArbitrageError::InvalidAccount);
    let authority_tag = u32::from_le_bytes(
        data[0..4]
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    );
    require!(authority_tag == 1, ArbitrageError::InvalidAccount);
    let authority = Pubkey::new_from_array(
        data[4..36]
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    );
    let supply = u64::from_le_bytes(
        data[36..44]
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    );
    require!(
        data[44] == 9 && data[45] == 1,
        ArbitrageError::InvalidAccount
    );
    Ok((supply, authority))
}

pub fn validate_marinade_finance_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == MARINADE_FINANCE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require_keys_eq!(
        step[0].key(),
        MARINADE_FINANCE_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[1].key(),
        MARINADE_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[1].owner.key(),
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[2].key(),
        MARINADE_MSOL_MINT,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        step[2].owner.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].owner.key(),
        system_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].owner.key(),
        system_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[9].key(),
        system_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[10].key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let (expected_input, expected_output) = if direction == 0 {
        (MARINADE_WSOL_MINT, MARINADE_MSOL_MINT)
    } else {
        (MARINADE_MSOL_MINT, MARINADE_WSOL_MINT)
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

    let state_data = step[1].try_borrow_data()?;
    require!(
        state_data.starts_with(&MARINADE_STATE_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let mut encoded = state_data
        .get(MARINADE_STATE_DISCRIMINATOR.len()..)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let state =
        MarinadeState::deserialize(&mut encoded).map_err(|_| ArbitrageError::InvalidAccount)?;
    require!(!state.paused, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        state.msol_mint,
        step[2].key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        state.liq_pool.msol_leg,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        state.treasury_msol_account,
        step[7].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        state.deposit_sol_fee.bp_cents <= FEE_CENTS_DENOMINATOR
            && state.liq_pool.lp_min_fee.basis_points <= state.liq_pool.lp_max_fee.basis_points
            && state.liq_pool.lp_max_fee.basis_points <= FEE_BPS_DENOMINATOR
            && state.liq_pool.treasury_cut.basis_points <= FEE_BPS_DENOMINATOR
            && state.liq_pool.lp_liquidity_target > 0,
        ArbitrageError::InvalidAccount
    );
    let total_under_control = state
        .validator_system
        .total_active_balance
        .checked_add(state.stake_system.delayed_unstake_cooling_down)
        .and_then(|value| value.checked_add(state.emergency_cooling_down))
        .and_then(|value| value.checked_add(state.available_reserve_balance))
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        state.msol_supply > 0
            && total_under_control.saturating_sub(state.circulating_ticket_balance) > 0,
        ArbitrageError::InsufficientLiquidity
    );

    let (expected_reserve, reserve_bump) =
        Pubkey::find_program_address(&[step[1].key.as_ref(), b"reserve"], step[0].key);
    let (expected_mint_authority, mint_authority_bump) =
        Pubkey::find_program_address(&[step[1].key.as_ref(), b"st_mint"], step[0].key);
    let (expected_sol_leg, sol_leg_bump) =
        Pubkey::find_program_address(&[step[1].key.as_ref(), b"liq_sol"], step[0].key);
    let (expected_msol_authority, msol_authority_bump) = Pubkey::find_program_address(
        &[step[1].key.as_ref(), b"liq_st_sol_authority"],
        step[0].key,
    );
    require_keys_eq!(
        step[6].key(),
        expected_reserve,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        expected_mint_authority,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].key(),
        expected_sol_leg,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        expected_msol_authority,
        ArbitrageError::InvalidAccount
    );
    require!(
        state.reserve_bump_seed == reserve_bump
            && state.msol_mint_authority_bump_seed == mint_authority_bump
            && state.liq_pool.sol_leg_bump_seed == sol_leg_bump
            && state.liq_pool.msol_leg_authority_bump_seed == msol_authority_bump,
        ArbitrageError::InvalidAccount
    );

    validate_token_account_for_mint_and_authority(&step[4], &step[2], token_program, &step[5])?;
    validate_token_account_for_mint(&step[7], &step[2], token_program)?;
    let mint_data = step[2].try_borrow_data()?;
    let (actual_supply, actual_authority) = mint_supply_and_authority(&mint_data)?;
    require_keys_eq!(
        actual_authority,
        step[8].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        actual_supply <= state.msol_supply,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

#[derive(Clone)]
pub struct MarinadeFinanceAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

fn swap_data(discriminator: [u8; 8], amount: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&amount.to_le_bytes());
    data
}

fn invoke_deposit<'info>(accounts: &MarinadeFinanceAccounts<'info>, amount: u64) -> Result<()> {
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: vec![
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new(accounts.step[3].key(), false),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new_readonly(accounts.step[5].key(), false),
                AccountMeta::new(accounts.step[6].key(), false),
                AccountMeta::new(accounts.payer.key(), true),
                AccountMeta::new(accounts.user_output.key(), false),
                AccountMeta::new_readonly(accounts.step[8].key(), false),
                AccountMeta::new_readonly(accounts.step[9].key(), false),
                AccountMeta::new_readonly(accounts.step[10].key(), false),
            ],
            data: swap_data(MARINADE_DEPOSIT_DISCRIMINATOR, amount),
        },
        &[
            accounts.step[1].clone(),
            accounts.step[2].clone(),
            accounts.step[3].clone(),
            accounts.step[4].clone(),
            accounts.step[5].clone(),
            accounts.step[6].clone(),
            accounts.payer.clone(),
            accounts.user_output.clone(),
            accounts.step[8].clone(),
            accounts.step[9].clone(),
            accounts.step[10].clone(),
            accounts.step[0].clone(),
        ],
    )?;
    Ok(())
}

fn invoke_liquid_unstake<'info>(
    accounts: &MarinadeFinanceAccounts<'info>,
    amount: u64,
) -> Result<()> {
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: vec![
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new(accounts.step[3].key(), false),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new(accounts.step[7].key(), false),
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new_readonly(accounts.payer.key(), true),
                AccountMeta::new(accounts.payer.key(), false),
                AccountMeta::new_readonly(accounts.step[9].key(), false),
                AccountMeta::new_readonly(accounts.step[10].key(), false),
            ],
            data: swap_data(MARINADE_LIQUID_UNSTAKE_DISCRIMINATOR, amount),
        },
        &[
            accounts.step[1].clone(),
            accounts.step[2].clone(),
            accounts.step[3].clone(),
            accounts.step[4].clone(),
            accounts.step[7].clone(),
            accounts.user_input.clone(),
            accounts.payer.clone(),
            accounts.payer.clone(),
            accounts.step[9].clone(),
            accounts.step[10].clone(),
            accounts.step[0].clone(),
        ],
    )?;
    Ok(())
}

pub fn marinade_finance_swap<'info>(
    accounts: MarinadeFinanceAccounts<'info>,
    amount: u64,
    deposit: bool,
) -> Result<SwapResult> {
    require!(
        accounts.step.len() == MARINADE_FINANCE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    if deposit {
        let bridge = WsolBridgeAccounts {
            payer: accounts.payer,
            user_wsol: accounts.user_input,
            wsol_mint: accounts.input_mint,
            token_program: &accounts.step[10],
            associated_token_program: accounts.associated_token_program,
            system_program: &accounts.step[9],
        };
        let remaining_wsol = close_wsol_for_native(bridge, amount)?;
        invoke_deposit(&accounts, amount)?;
        restore_wsol_after_native(bridge, remaining_wsol)?;
    } else {
        let pre_lamports = accounts.payer.lamports();
        invoke_liquid_unstake(&accounts, amount)?;
        let received = accounts
            .payer
            .lamports()
            .checked_sub(pre_lamports)
            .ok_or(ArbitrageError::MathOverflow)?;
        require!(received > 0, ArbitrageError::InvalidAmount);
        invoke(
            &system_instruction::transfer(accounts.payer.key, accounts.user_output.key, received),
            &[
                accounts.payer.clone(),
                accounts.user_output.clone(),
                accounts.step[9].clone(),
            ],
        )?;
        invoke(
            &anchor_spl::token::spl_token::instruction::sync_native(
                accounts.step[10].key,
                accounts.user_output.key,
            )?,
            &[accounts.user_output.clone(), accounts.step[10].clone()],
        )?;
    }
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_payloads_are_exactly_sixteen_bytes() {
        assert_eq!(swap_data(MARINADE_DEPOSIT_DISCRIMINATOR, 1).len(), 16);
        assert_eq!(
            &swap_data(MARINADE_LIQUID_UNSTAKE_DISCRIMINATOR, 7)[8..],
            &7_u64.to_le_bytes()
        );
    }
}
