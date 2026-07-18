use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::SYMMETRY_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta,
            validate_token_account_for_mint_and_authority_key, SwapResult,
        },
    },
};

pub const SYMMETRY_STEP_ACCOUNTS: usize = 13;

const TOKEN_LIST: Pubkey = anchor_lang::pubkey!("3SnUughtueoVrhevXTLMf586qvKNNXggNsc7NgoMUU1t");
const CURVE_DATA: Pubkey = anchor_lang::pubkey!("4QMjSHuM3iS7Fdfi8kZJfHRKoEJSDHEtEwqbChsTcUVK");
const SYMMETRY_PDA: Pubkey = anchor_lang::pubkey!("BLBYiq48WcLQ5SxiftyKmPtmsZPUBEnDEjqEnKGAR4zx");
const SWAP_FEE_OWNER: Pubkey = anchor_lang::pubkey!("AWfpfzA6FYbqx4JLz75PDgsjH7jtBnnmJ6MXW5zNY2Ei");
const PYTH_RECEIVER_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ");
const WRAPPED_SOL_MINT: Pubkey =
    anchor_lang::pubkey!("So11111111111111111111111111111111111111112");

const FUND_STATE_LEN: usize = 10_208;
const TOKEN_LIST_LEN: usize = 39_816;
const CURVE_DATA_LEN: usize = 64_008;
const PYTH_PRICE_UPDATE_LEN: usize = 134;
const TOKEN_RECORD_LEN: usize = 199;
const MAX_TOKEN_LIST_TOKENS: usize = 200;
const CURVE_POINTS: usize = 10;
const WEIGHT_SCALE: u64 = 10_000;
const BPS_SCALE: u64 = 10_000;
const MAX_ORACLE_AGE: u64 = 60;
const PYTH_ORACLE_TYPE: u8 = 3;

const FUND_DISCRIMINATOR: [u8; 8] = [3, 254, 145, 43, 146, 96, 162, 104];
const TOKEN_LIST_DISCRIMINATOR: [u8; 8] = [145, 167, 153, 173, 5, 187, 157, 150];
const CURVE_DATA_DISCRIMINATOR: [u8; 8] = [126, 79, 160, 183, 77, 115, 246, 38];
const PYTH_PRICE_UPDATE_DISCRIMINATOR: [u8; 8] = [34, 241, 35, 99, 157, 126, 244, 205];
const LIQUIDITY_PROVISION_DISCRIMINATOR: [u8; 8] = [130, 113, 21, 240, 202, 190, 11, 3];

#[derive(Clone)]
pub struct SymmetryAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_token_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymmetryValidation {
    pub input_token_id: u64,
    pub output_token_id: u64,
    pub quoted_output: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct OraclePrices {
    avg: u64,
    buy: u64,
    sell: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TokenSettings {
    token_id: u64,
    mint: Pubkey,
    decimals: u8,
    vault: Pubkey,
    oracle: Pubkey,
    oracle_confidence_pct: u8,
    fixed_confidence_bps: u8,
    fee_after_target_weight_bps: u8,
    fee_before_target_weight_bps: u8,
    amount: u64,
    target_weight: u64,
    buy_curve_amounts: [u64; CURVE_POINTS],
    sell_curve_amounts: [u64; CURVE_POINTS],
    prices: OraclePrices,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FundState {
    manager: Pubkey,
    host: Pubkey,
    token_ids: [u64; 2],
    amounts: [u64; 2],
    target_weights: [u64; 2],
    weight_sum: u64,
    rebalance_threshold: u64,
    lp_offset_threshold: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QuoteState {
    token0: TokenSettings,
    token1: TokenSettings,
    weight_sum: u64,
    rebalance_threshold: u64,
    lp_offset_threshold: u64,
    symmetry_fee_pct: u8,
    host_fee_pct: u8,
    manager_fee_pct: u8,
}

#[allow(clippy::too_many_arguments)]
pub fn validate_symmetry_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    amount_in: u64,
    minimum_amount_out: u64,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<SymmetryValidation> {
    require!(
        step.len() == SYMMETRY_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(
        direction <= 1 && fee_rate == 0 && amount_in > 0,
        ArbitrageError::InvalidInstructionData
    );
    require_keys_eq!(
        step[0].key(),
        SYMMETRY_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        input_token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        output_token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *input_mint.owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *output_mint.owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(step[2].key(), SYMMETRY_PDA, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[2].owner,
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidAccount
    );
    require!(step[2].data_is_empty(), ArbitrageError::InvalidAccount);
    require_keys_eq!(step[8].key(), TOKEN_LIST, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[9].key(), CURVE_DATA, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[8].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[9].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );

    let fund_data = step[1].try_borrow_data()?;
    let fund = decode_fund_state(&fund_data)?;
    drop(fund_data);
    let token_list_data = step[8].try_borrow_data()?;
    validate_token_list(&token_list_data)?;
    let curve_data = step[9].try_borrow_data()?;
    validate_curve_data(&curve_data)?;
    let mut token0 = decode_token_settings(
        &token_list_data,
        &curve_data,
        fund.token_ids[0],
        fund.amounts[0],
        fund.target_weights[0],
    )?;
    let mut token1 = decode_token_settings(
        &token_list_data,
        &curve_data,
        fund.token_ids[1],
        fund.amounts[1],
        fund.target_weights[1],
    )?;
    let (sol_oracle, sol_oracle_confidence_pct, sol_fixed_confidence_bps) =
        token_oracle_settings(&token_list_data, 1, WRAPPED_SOL_MINT)?;
    let symmetry_fee_pct = read_u8(&token_list_data, 212)?;
    let host_fee_pct = read_u8(&token_list_data, 213)?;
    let manager_fee_pct = read_u8(&token_list_data, 214)?;
    require!(
        u16::from(symmetry_fee_pct)
            .checked_add(u16::from(host_fee_pct))
            .and_then(|value| value.checked_add(u16::from(manager_fee_pct)))
            .ok_or(ArbitrageError::MathOverflow)?
            <= 100,
        ArbitrageError::InvalidAccount
    );
    drop(curve_data);
    drop(token_list_data);

    require_keys_eq!(step[10].key(), sol_oracle, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[11].key(),
        token0.oracle,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[12].key(),
        token1.oracle,
        ArbitrageError::InvalidAccount
    );
    let unix_timestamp = Clock::get()?.unix_timestamp;
    decode_pyth_prices(
        &step[10],
        unix_timestamp,
        sol_oracle_confidence_pct,
        sol_fixed_confidence_bps,
    )?;
    token0.prices = decode_pyth_prices(
        &step[11],
        unix_timestamp,
        token0.oracle_confidence_pct,
        token0.fixed_confidence_bps,
    )?;
    token1.prices = decode_pyth_prices(
        &step[12],
        unix_timestamp,
        token1.oracle_confidence_pct,
        token1.fixed_confidence_bps,
    )?;

    require!(
        token0.mint != token1.mint
            && token0.vault != token1.vault
            && token0.oracle != token1.oracle,
        ArbitrageError::InvalidAccount
    );
    let (input, output) = if direction == 0 {
        (token0, token1)
    } else {
        (token1, token0)
    };
    require_keys_eq!(
        input.mint,
        input_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output.mint,
        output_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(step[3].key(), input.vault, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[4].key(), output.vault, ArbitrageError::InvalidAccount);
    validate_token_account_for_mint_and_authority_key(
        &step[3],
        input_mint,
        input_token_program,
        SYMMETRY_PDA,
    )?;
    validate_token_account_for_mint_and_authority_key(
        &step[4],
        output_mint,
        output_token_program,
        SYMMETRY_PDA,
    )?;
    require!(
        read_token_amount(&step[3])? >= input.amount
            && read_token_amount(&step[4])? >= output.amount,
        ArbitrageError::InvalidAccount
    );
    validate_fee_account(&step[5], output_mint, output_token_program, SWAP_FEE_OWNER)?;
    validate_fee_account(&step[6], output_mint, output_token_program, fund.host)?;
    validate_fee_account(&step[7], output_mint, output_token_program, fund.manager)?;

    let quote_state = QuoteState {
        token0,
        token1,
        weight_sum: fund.weight_sum,
        rebalance_threshold: fund.rebalance_threshold,
        lp_offset_threshold: fund.lp_offset_threshold,
        symmetry_fee_pct,
        host_fee_pct,
        manager_fee_pct,
    };
    let quoted_output = quote_exact_in(&quote_state, direction, amount_in)?;
    require!(
        minimum_amount_out <= quoted_output,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(SymmetryValidation {
        input_token_id: input.token_id,
        output_token_id: output.token_id,
        quoted_output,
    })
}

fn validate_fee_account<'info>(
    account: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    owner: Pubkey,
) -> Result<()> {
    let expected = anchor_spl::associated_token::get_associated_token_address_with_program_id(
        &owner,
        &output_mint.key(),
        &token_program.key(),
    );
    require_keys_eq!(account.key(), expected, ArbitrageError::InvalidAccount);
    validate_token_account_for_mint_and_authority_key(account, output_mint, token_program, owner)
}

pub fn symmetry_swap<'a, 'info>(
    accounts: SymmetryAccounts<'a, 'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    validation: SymmetryValidation,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(
        minimum_amount_out <= validation.quoted_output,
        ArbitrageError::InsufficientOutputAmount
    );
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(&LIQUIDITY_PROVISION_DISCRIMINATOR);
    data.extend_from_slice(&validation.input_token_id.to_le_bytes());
    data.extend_from_slice(&validation.output_token_id.to_le_bytes());
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    let instruction = Instruction {
        program_id: accounts.step[0].key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.payer.key(), true),
            AccountMeta::new(accounts.step[1].key(), false),
            AccountMeta::new_readonly(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(accounts.user_input.key(), false),
            AccountMeta::new(accounts.step[4].key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new_readonly(accounts.step[10].key(), false),
            AccountMeta::new(accounts.step[5].key(), false),
            AccountMeta::new(accounts.step[6].key(), false),
            AccountMeta::new(accounts.step[7].key(), false),
            AccountMeta::new_readonly(accounts.step[8].key(), false),
            AccountMeta::new_readonly(accounts.step[9].key(), false),
            AccountMeta::new_readonly(accounts.input_token_program.key(), false),
            AccountMeta::new_readonly(accounts.step[11].key(), false),
            AccountMeta::new_readonly(accounts.step[12].key(), false),
        ],
        data,
    };
    let pre_output = read_token_amount(accounts.user_output)?;
    let infos = vec![
        accounts.payer.clone(),
        accounts.step[1].clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.user_input.clone(),
        accounts.step[4].clone(),
        accounts.user_output.clone(),
        accounts.step[10].clone(),
        accounts.step[5].clone(),
        accounts.step[6].clone(),
        accounts.step[7].clone(),
        accounts.step[8].clone(),
        accounts.step[9].clone(),
        accounts.input_token_program.clone(),
        accounts.step[11].clone(),
        accounts.step[12].clone(),
        accounts.step[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    let amount_out = token_balance_delta(accounts.user_output, pre_output)?;
    require!(
        amount_out >= minimum_amount_out,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

fn decode_fund_state(data: &[u8]) -> Result<FundState> {
    require!(
        data.len() == FUND_STATE_LEN && data.starts_with(&FUND_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(read_u64(data, 168)? == 2, ArbitrageError::InvalidAccount);
    require!(read_u64(data, 9432)? == 0, ArbitrageError::InvalidAccount);
    let state = FundState {
        manager: read_pubkey(data, 16)?,
        host: read_pubkey(data, 128)?,
        token_ids: [read_u64(data, 176)?, read_u64(data, 184)?],
        amounts: [read_u64(data, 336)?, read_u64(data, 344)?],
        target_weights: [read_u64(data, 656)?, read_u64(data, 664)?],
        weight_sum: read_u64(data, 816)?,
        rebalance_threshold: read_u64(data, 1024)?,
        lp_offset_threshold: read_u64(data, 1040)?,
    };
    require!(
        state.manager != Pubkey::default()
            && state.host != Pubkey::default()
            && state.token_ids[0] != state.token_ids[1]
            && state.token_ids[0] < MAX_TOKEN_LIST_TOKENS as u64
            && state.token_ids[1] < MAX_TOKEN_LIST_TOKENS as u64
            && state.amounts[0] > 0
            && state.amounts[1] > 0
            && state.target_weights[0] > 0
            && state.target_weights[1] > 0
            && state.weight_sum == WEIGHT_SCALE
            && state.target_weights[0]
                .checked_add(state.target_weights[1])
                .ok_or(ArbitrageError::MathOverflow)?
                == state.weight_sum,
        ArbitrageError::InvalidAccount
    );
    Ok(state)
}

fn validate_token_list(data: &[u8]) -> Result<()> {
    require!(
        data.len() == TOKEN_LIST_LEN && data.starts_with(&TOKEN_LIST_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let count = read_u64(data, 8)?;
    require!(
        count > 1 && count <= MAX_TOKEN_LIST_TOKENS as u64,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_curve_data(data: &[u8]) -> Result<()> {
    require!(
        data.len() == CURVE_DATA_LEN && data.starts_with(&CURVE_DATA_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn token_record_base(data: &[u8], token_id: u64) -> Result<usize> {
    let count = read_u64(data, 8)?;
    require!(
        token_id < count && token_id < MAX_TOKEN_LIST_TOKENS as u64,
        ArbitrageError::InvalidAccount
    );
    let index = usize::try_from(token_id).map_err(|_| ArbitrageError::MathOverflow)?;
    16_usize
        .checked_add(
            index
                .checked_mul(TOKEN_RECORD_LEN)
                .ok_or(ArbitrageError::MathOverflow)?,
        )
        .ok_or(ArbitrageError::MathOverflow.into())
}

fn token_oracle_settings(
    data: &[u8],
    token_id: u64,
    expected_mint: Pubkey,
) -> Result<(Pubkey, u8, u8)> {
    let base = token_record_base(data, token_id)?;
    require_keys_eq!(
        read_pubkey(data, base)?,
        expected_mint,
        ArbitrageError::InvalidTokenMint
    );
    require!(
        read_u8(data, base + 95)? == PYTH_ORACLE_TYPE,
        ArbitrageError::InvalidAccount
    );
    let oracle = read_pubkey(data, base + 96)?;
    require!(oracle != Pubkey::default(), ArbitrageError::InvalidAccount);
    Ok((
        oracle,
        read_u8(data, base + 129)?,
        read_u8(data, base + 130)?,
    ))
}

fn decode_token_settings(
    token_list: &[u8],
    curve_data: &[u8],
    token_id: u64,
    amount: u64,
    target_weight: u64,
) -> Result<TokenSettings> {
    let base = token_record_base(token_list, token_id)?;
    let index = usize::try_from(token_id).map_err(|_| ArbitrageError::MathOverflow)?;
    let buy_base = 8_usize
        .checked_add(index.checked_mul(160).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::MathOverflow)?;
    let sell_base = 32_008_usize
        .checked_add(index.checked_mul(160).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::MathOverflow)?;
    let mut buy_curve_amounts = [0_u64; CURVE_POINTS];
    let mut sell_curve_amounts = [0_u64; CURVE_POINTS];
    for point in 0..CURVE_POINTS {
        let point_offset = point.checked_mul(8).ok_or(ArbitrageError::MathOverflow)?;
        buy_curve_amounts[point] = read_u64(curve_data, buy_base + point_offset)?;
        sell_curve_amounts[point] = read_u64(curve_data, sell_base + point_offset)?;
    }
    let settings = TokenSettings {
        token_id,
        mint: read_pubkey(token_list, base)?,
        decimals: read_u8(token_list, base + 32)?,
        vault: read_pubkey(token_list, base + 63)?,
        oracle: read_pubkey(token_list, base + 96)?,
        oracle_confidence_pct: read_u8(token_list, base + 129)?,
        fixed_confidence_bps: read_u8(token_list, base + 130)?,
        fee_after_target_weight_bps: read_u8(token_list, base + 131)?,
        fee_before_target_weight_bps: read_u8(token_list, base + 132)?,
        amount,
        target_weight,
        buy_curve_amounts,
        sell_curve_amounts,
        prices: OraclePrices::default(),
    };
    require!(
        settings.mint != Pubkey::default()
            && settings.vault != Pubkey::default()
            && settings.oracle != Pubkey::default()
            && settings.decimals <= 18
            && read_u8(token_list, base + 95)? == PYTH_ORACLE_TYPE
            && read_u8(token_list, base + 133)? == 1
            && read_u8(token_list, base + 134)? == 1
            && read_u8(token_list, base + 135)? == 0,
        ArbitrageError::InvalidAccount
    );
    Ok(settings)
}

fn decode_pyth_prices<'info>(
    account: &AccountInfo<'info>,
    unix_timestamp: i64,
    oracle_confidence_pct: u8,
    fixed_confidence_bps: u8,
) -> Result<OraclePrices> {
    require_keys_eq!(
        *account.owner,
        PYTH_RECEIVER_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() == PYTH_PRICE_UPDATE_LEN
            && data.starts_with(&PYTH_PRICE_UPDATE_DISCRIMINATOR)
            && read_u8(&data, 40)? == 1,
        ArbitrageError::InvalidAccount
    );
    let raw_price = read_i64(&data, 73)?;
    require!(raw_price > 0, ArbitrageError::InvalidAccount);
    let raw_price = u64::try_from(raw_price).map_err(|_| ArbitrageError::MathOverflow)?;
    let raw_confidence = read_u64(&data, 81)?;
    let exponent = read_i32(&data, 89)?;
    let publish_time = read_i64(&data, 93)?;
    require!(
        unix_timestamp >= publish_time,
        ArbitrageError::InvalidAccount
    );
    let age = u64::try_from(
        unix_timestamp
            .checked_sub(publish_time)
            .ok_or(ArbitrageError::MathOverflow)?,
    )
    .map_err(|_| ArbitrageError::MathOverflow)?;
    require!(age <= MAX_ORACLE_AGE, ArbitrageError::InvalidAccount);
    let avg = scale_pyth_value(raw_price, exponent)?;
    let confidence = scale_pyth_value(raw_confidence, exponent)?;
    let fixed_confidence = mul_div(avg, u64::from(fixed_confidence_bps), BPS_SCALE)?;
    let total_confidence = mul_div(confidence, u64::from(oracle_confidence_pct), 100)?
        .checked_add(mul_div(avg, age, BPS_SCALE)?)
        .and_then(|value| value.checked_add(fixed_confidence))
        .ok_or(ArbitrageError::MathOverflow)?;
    Ok(OraclePrices {
        avg,
        buy: avg
            .checked_add(total_confidence)
            .ok_or(ArbitrageError::MathOverflow)?,
        sell: avg
            .checked_sub(total_confidence)
            .ok_or(ArbitrageError::InvalidAccount)?,
    })
}

fn quote_exact_in(state: &QuoteState, direction: u8, amount_in: u64) -> Result<u64> {
    require!(
        amount_in > 0 && direction <= 1,
        ArbitrageError::InvalidAmount
    );
    let (input, output) = if direction == 0 {
        (state.token0, state.token1)
    } else {
        (state.token1, state.token0)
    };
    let fund_worth = amount_to_usd(
        state.token0.amount,
        state.token0.decimals,
        state.token0.prices.avg,
    )?
    .checked_add(amount_to_usd(
        state.token1.amount,
        state.token1.decimals,
        state.token1.prices.avg,
    )?)
    .ok_or(ArbitrageError::MathOverflow)?;
    require!(fund_worth > 0, ArbitrageError::InvalidAccount);
    let input_target_amount = usd_to_amount(
        mul_div(input.target_weight, fund_worth, state.weight_sum)?,
        input.decimals,
        input.prices.avg,
    )?;
    let output_target_amount = usd_to_amount(
        mul_div(output.target_weight, fund_worth, state.weight_sum)?,
        output.decimals,
        output.prices.avg,
    )?;
    let sold_value = compute_sold_value(amount_in, input, input_target_amount)?;
    let quoted_output = compute_bought_amount(sold_value, output, output_target_amount)?;
    let mut amount_without_fees = usd_to_amount(
        amount_to_usd(amount_in, input.decimals, input.prices.sell)?,
        output.decimals,
        output.prices.buy,
    )?;
    if amount_without_fees > output.amount {
        amount_without_fees = output.amount;
    }
    let output_amount = quoted_output.min(amount_without_fees);
    require!(output_amount > 0, ArbitrageError::InvalidAmount);
    let total_fees = amount_without_fees
        .checked_sub(output_amount)
        .ok_or(ArbitrageError::MathOverflow)?;
    let manager_fee = mul_div(total_fees, u64::from(state.manager_fee_pct), 100)?;
    let external_fees = mul_div(total_fees, u64::from(state.symmetry_fee_pct), 100)?
        .checked_add(mul_div(total_fees, u64::from(state.host_fee_pct), 100)?)
        .and_then(|value| value.checked_add(manager_fee))
        .ok_or(ArbitrageError::MathOverflow)?;
    let fund_fee = total_fees
        .checked_sub(external_fees)
        .ok_or(ArbitrageError::InvalidAccount)?;
    validate_weight_limits(
        state,
        input,
        output,
        amount_in,
        amount_without_fees,
        fund_fee,
        fund_worth,
    )?;
    Ok(output_amount)
}

fn compute_sold_value(amount: u64, token: TokenSettings, target_amount: u64) -> Result<u64> {
    let mut current_amount = token.amount;
    let mut curve_offset = current_amount.saturating_sub(target_amount);
    let mut output_value = 0_u64;
    let mut amount_left = amount;
    for step in 0..=CURVE_POINTS {
        let step_amount = if step < CURVE_POINTS {
            token.sell_curve_amounts[step]
        } else {
            amount_left
        };
        if step == CURVE_POINTS {
            curve_offset = 0;
        }
        if step_amount <= curve_offset {
            curve_offset = curve_offset
                .checked_sub(step_amount)
                .ok_or(ArbitrageError::MathOverflow)?;
            continue;
        }
        let mut interval = step_amount
            .checked_sub(curve_offset)
            .ok_or(ArbitrageError::MathOverflow)?;
        curve_offset = 0;
        if interval > amount_left {
            interval = amount_left;
        }
        let mut before = interval;
        if current_amount >= target_amount {
            before = 0;
        } else if current_amount
            .checked_add(interval)
            .ok_or(ArbitrageError::MathOverflow)?
            >= target_amount
        {
            before = before
                .checked_sub(
                    current_amount
                        .checked_add(interval)
                        .and_then(|value| value.checked_sub(target_amount))
                        .ok_or(ArbitrageError::MathOverflow)?,
                )
                .ok_or(ArbitrageError::MathOverflow)?;
        }
        let after = interval
            .checked_sub(before)
            .ok_or(ArbitrageError::MathOverflow)?;
        let value_before = amount_to_usd(before, token.decimals, token.prices.sell)?;
        let value_after = amount_to_usd(after, token.decimals, token.prices.sell)?;
        let fees = mul_div(
            value_before,
            u64::from(token.fee_before_target_weight_bps),
            BPS_SCALE,
        )?
        .checked_add(mul_div(
            value_after,
            u64::from(token.fee_after_target_weight_bps),
            BPS_SCALE,
        )?)
        .ok_or(ArbitrageError::MathOverflow)?;
        output_value = output_value
            .checked_add(value_before)
            .and_then(|value| value.checked_add(value_after))
            .and_then(|value| value.checked_sub(fees))
            .ok_or(ArbitrageError::MathOverflow)?;
        amount_left = amount_left
            .checked_sub(interval)
            .ok_or(ArbitrageError::MathOverflow)?;
        current_amount = current_amount
            .checked_add(interval)
            .ok_or(ArbitrageError::MathOverflow)?;
        if amount_left == 0 {
            break;
        }
    }
    Ok(output_value)
}

fn compute_bought_amount(value: u64, token: TokenSettings, target_amount: u64) -> Result<u64> {
    let mut current_amount = token.amount;
    let mut curve_offset = target_amount.saturating_sub(current_amount);
    let mut output_amount = 0_u64;
    let mut value_left = value;
    for step in 0..=CURVE_POINTS {
        let step_amount = if step < CURVE_POINTS {
            token.buy_curve_amounts[step]
        } else {
            usd_to_amount(
                value_left
                    .checked_mul(2)
                    .ok_or(ArbitrageError::MathOverflow)?,
                token.decimals,
                token.prices.buy,
            )?
        };
        if step == CURVE_POINTS {
            curve_offset = 0;
        }
        if step_amount <= curve_offset {
            curve_offset = curve_offset
                .checked_sub(step_amount)
                .ok_or(ArbitrageError::MathOverflow)?;
            continue;
        }
        let mut interval = step_amount
            .checked_sub(curve_offset)
            .ok_or(ArbitrageError::MathOverflow)?;
        curve_offset = 0;
        let mut interval_value = amount_to_usd(interval, token.decimals, token.prices.buy)?;
        if interval_value > value_left {
            interval_value = value_left;
            interval = usd_to_amount(interval_value, token.decimals, token.prices.buy)?;
        }
        let mut value_before = interval_value;
        if current_amount <= target_amount {
            value_before = 0;
        } else if current_amount
            <= target_amount
                .checked_add(interval)
                .ok_or(ArbitrageError::MathOverflow)?
        {
            let crossing = target_amount
                .checked_add(interval)
                .and_then(|value| value.checked_sub(current_amount))
                .ok_or(ArbitrageError::MathOverflow)?;
            value_before = value_before
                .checked_sub(amount_to_usd(crossing, token.decimals, token.prices.buy)?)
                .ok_or(ArbitrageError::MathOverflow)?;
        }
        let value_after = interval_value
            .checked_sub(value_before)
            .ok_or(ArbitrageError::MathOverflow)?;
        let fees = mul_div(
            value_before,
            u64::from(token.fee_before_target_weight_bps),
            BPS_SCALE,
        )?
        .checked_add(mul_div(
            value_after,
            u64::from(token.fee_after_target_weight_bps),
            BPS_SCALE,
        )?)
        .ok_or(ArbitrageError::MathOverflow)?;
        let bought = usd_to_amount(
            interval_value
                .checked_sub(fees)
                .ok_or(ArbitrageError::MathOverflow)?,
            token.decimals,
            token.prices.buy,
        )?;
        output_amount = output_amount
            .checked_add(bought)
            .ok_or(ArbitrageError::MathOverflow)?;
        value_left = value_left
            .checked_sub(interval_value)
            .ok_or(ArbitrageError::MathOverflow)?;
        current_amount = current_amount.saturating_sub(bought);
        if value_left == 0 {
            break;
        }
    }
    Ok(output_amount)
}

#[allow(clippy::too_many_arguments)]
fn validate_weight_limits(
    state: &QuoteState,
    input: TokenSettings,
    output: TokenSettings,
    amount_in: u64,
    amount_without_fees: u64,
    fund_fee: u64,
    original_fund_worth: u64,
) -> Result<()> {
    let input_before = amount_to_usd(input.amount, input.decimals, input.prices.avg)?;
    let output_before = amount_to_usd(output.amount, output.decimals, output.prices.avg)?;
    let safe_input = mul_div(amount_in, 101, 100)?;
    let input_after = amount_to_usd(
        input
            .amount
            .checked_add(safe_input)
            .ok_or(ArbitrageError::MathOverflow)?,
        input.decimals,
        input.prices.avg,
    )?;
    let mut safe_output = mul_div(
        amount_without_fees
            .checked_sub(fund_fee)
            .ok_or(ArbitrageError::MathOverflow)?,
        101,
        100,
    )?;
    if safe_output > output.amount {
        safe_output = output.amount;
    }
    let output_after = amount_to_usd(
        output
            .amount
            .checked_sub(safe_output)
            .ok_or(ArbitrageError::MathOverflow)?,
        output.decimals,
        output.prices.avg,
    )?;
    let with_after = original_fund_worth
        .checked_add(input_after)
        .and_then(|value| value.checked_add(output_after))
        .ok_or(ArbitrageError::MathOverflow)?;
    let without_input = if with_after < input_before {
        0
    } else {
        with_after
            .checked_sub(input_before)
            .ok_or(ArbitrageError::MathOverflow)?
    };
    let new_fund_worth = if without_input < output_before {
        0
    } else {
        without_input
            .checked_sub(output_before)
            .ok_or(ArbitrageError::MathOverflow)?
    };
    require!(new_fund_worth > 0, ArbitrageError::InvalidAccount);
    let input_weight = mul_div(input_after, WEIGHT_SCALE, new_fund_worth)?;
    let output_weight = mul_div(output_after, WEIGHT_SCALE, new_fund_worth)?;
    let allowed_offset = state
        .rebalance_threshold
        .checked_mul(state.lp_offset_threshold)
        .ok_or(ArbitrageError::MathOverflow)?;
    let scale_squared = BPS_SCALE
        .checked_mul(BPS_SCALE)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        allowed_offset <= scale_squared,
        ArbitrageError::InvalidAccount
    );
    let mut max_input_weight = mul_div(
        input.target_weight,
        scale_squared
            .checked_add(allowed_offset)
            .ok_or(ArbitrageError::MathOverflow)?,
        scale_squared,
    )?;
    if max_input_weight > WEIGHT_SCALE {
        max_input_weight = WEIGHT_SCALE;
    }
    let min_output_weight = mul_div(
        output.target_weight,
        scale_squared
            .checked_sub(allowed_offset)
            .ok_or(ArbitrageError::MathOverflow)?,
        scale_squared,
    )?;
    let removing_dust = input.token_id == 0 && output.target_weight == 0;
    require!(
        (input_weight <= max_input_weight || removing_dust) && output_weight >= min_output_weight,
        ArbitrageError::InvalidAmount
    );
    Ok(())
}

fn scale_pyth_value(value: u64, exponent: i32) -> Result<u64> {
    let scale_exponent = exponent
        .checked_add(12)
        .ok_or(ArbitrageError::MathOverflow)?;
    if scale_exponent >= 0 {
        let exponent = u32::try_from(scale_exponent).map_err(|_| ArbitrageError::MathOverflow)?;
        value
            .checked_mul(
                10_u64
                    .checked_pow(exponent)
                    .ok_or(ArbitrageError::MathOverflow)?,
            )
            .ok_or(ArbitrageError::MathOverflow.into())
    } else {
        value
            .checked_div(
                10_u64
                    .checked_pow(scale_exponent.unsigned_abs())
                    .ok_or(ArbitrageError::MathOverflow)?,
            )
            .ok_or(ArbitrageError::MathOverflow.into())
    }
}

fn amount_to_usd(amount: u64, decimals: u8, price: u64) -> Result<u64> {
    mul_div(
        amount,
        price,
        10_u64
            .checked_pow(u32::from(decimals))
            .ok_or(ArbitrageError::MathOverflow)?,
    )
}

fn usd_to_amount(worth: u64, decimals: u8, price: u64) -> Result<u64> {
    mul_div(
        worth,
        10_u64
            .checked_pow(u32::from(decimals))
            .ok_or(ArbitrageError::MathOverflow)?,
        price,
    )
}

fn mul_div(a: u64, b: u64, c: u64) -> Result<u64> {
    require!(c > 0, ArbitrageError::MathOverflow);
    let value = u128::from(a)
        .checked_mul(u128::from(b))
        .and_then(|value| value.checked_div(u128::from(c)))
        .ok_or(ArbitrageError::MathOverflow)?;
    u64::try_from(value).map_err(|_| ArbitrageError::MathOverflow.into())
}

fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or(ArbitrageError::InvalidAccount.into())
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i32(data: &[u8], offset: usize) -> Result<i32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_i64(data: &[u8], offset: usize) -> Result<i64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(i64::from_le_bytes(bytes))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(
        token_id: u64,
        amount: u64,
        price: u64,
        confidence: u64,
        curve: [u64; CURVE_POINTS],
    ) -> TokenSettings {
        TokenSettings {
            token_id,
            mint: Pubkey::new_unique(),
            decimals: 6,
            vault: Pubkey::new_unique(),
            oracle: Pubkey::new_unique(),
            oracle_confidence_pct: 50,
            fixed_confidence_bps: 0,
            fee_after_target_weight_bps: 20,
            fee_before_target_weight_bps: 10,
            amount,
            target_weight: 5_000,
            buy_curve_amounts: curve,
            sell_curve_amounts: curve,
            prices: OraclePrices {
                avg: price,
                buy: price + confidence,
                sell: price - confidence,
            },
        }
    }

    fn real_vector_state() -> QuoteState {
        let token0 = token(0, 2_028_173, 999_889_310_000, 3_534_155_792, [0; 10]);
        let token1 = token(
            4,
            1_942_937,
            999_235_100_000,
            3_468_607_320,
            [
                44_740_391,
                199_254_145,
                747_516_255,
                1_996_366_978,
                4_497_261_233,
                7_497_289_516,
                84_969_281_183,
                99_963_860_215,
                299_891_580_646,
                499_819_301_078,
            ],
        );
        QuoteState {
            token0,
            token1,
            weight_sum: 10_000,
            rebalance_threshold: 300,
            lp_offset_threshold: 0,
            symmetry_fee_pct: 0,
            host_fee_pct: 0,
            manager_fee_pct: 0,
        }
    }

    #[test]
    fn quote_matches_real_program_vector() {
        assert_eq!(quote_exact_in(&real_vector_state(), 1, 1_000).unwrap(), 990);
    }

    #[test]
    fn quote_rejects_real_program_weight_limit_vector() {
        assert!(quote_exact_in(&real_vector_state(), 1, 100_000).is_err());
    }
}
