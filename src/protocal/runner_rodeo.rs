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
        program_ids::RUNNER_RODEO_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta,
            validate_token_account_for_mint_and_authority_key, SwapResult,
        },
    },
};

pub const RUNNER_RODEO_STEP_ACCOUNTS: usize = 14;

const COIN_ACCOUNT_LEN: usize = 621;
const POOL_ACCOUNT_LEN: usize = 313;
const COIN_DISCRIMINATOR: [u8; 8] = [155, 12, 170, 224, 30, 250, 204, 130];
const POOL_DISCRIMINATOR: [u8; 8] = [213, 210, 227, 191, 48, 170, 222, 114];
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const ACTIVE_COIN_STATE: u8 = 3;
const ACTIVE_POOL_FLAG: u8 = 0;
const SELL_DIRECTION: u8 = 0;
const BUY_DIRECTION: u8 = 1;
const BPS_DENOMINATOR: u64 = 10_000;
const MINT_SUPPLY_OFFSET: usize = 36;
const WRAPPED_SOL_MINT: Pubkey =
    anchor_lang::pubkey!("So11111111111111111111111111111111111111112");

pub struct RunnerRodeoAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunnerRodeoValidation {
    pub quoted_output: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CoinState {
    state: u8,
    subject: Pubkey,
    creator: Pubkey,
    buy_fee_bps: [u16; 3],
    sell_fee_bps: [u16; 3],
    migration_threshold: u64,
    token_supply: u64,
    migration_token_allocation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CurvePoolState {
    coin: Pubkey,
    coin_identity: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    migration_token_allocation: u64,
    virtual_base: u64,
    virtual_quote: u64,
    circulating_base: u64,
    real_quote: u64,
    migration_threshold: u64,
    pool_flag: u8,
    base_mint: Pubkey,
    quote_mint: Pubkey,
}

#[allow(clippy::too_many_arguments)]
pub fn validate_runner_rodeo_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    amount_in: u64,
    minimum_amount_out: u64,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<RunnerRodeoValidation> {
    require!(
        step.len() == RUNNER_RODEO_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(
        direction <= 1 && amount_in > 0,
        ArbitrageError::InvalidInstructionData
    );
    require_keys_eq!(
        step[0].key(),
        RUNNER_RODEO_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[11].key(),
        anchor_spl::token_2022::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[12].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require!(
        step[0].executable && step[11].executable && step[12].executable,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidAccount
    );
    require!(step[1].data_is_empty(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[2].owner,
        RUNNER_RODEO_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[3].owner,
        RUNNER_RODEO_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[9].owner,
        anchor_spl::token_2022::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[10].owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );

    let coin_data = step[2].try_borrow_data()?;
    let coin = decode_coin(&coin_data)?;
    drop(coin_data);
    let pool_data = step[3].try_borrow_data()?;
    let pool = decode_pool(&pool_data)?;
    drop(pool_data);

    require_keys_eq!(pool.coin, step[2].key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        pool.base_vault,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        pool.quote_vault,
        step[5].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        pool.base_mint,
        step[9].key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        pool.quote_mint,
        step[10].key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        pool.quote_mint,
        WRAPPED_SOL_MINT,
        ArbitrageError::InvalidTokenMint
    );
    require!(
        pool.migration_threshold == coin.migration_threshold
            && pool.migration_token_allocation == coin.migration_token_allocation,
        ArbitrageError::InvalidAccount
    );

    let program_id = step[0].key();
    let authority = Pubkey::find_program_address(&[b"authority"], &program_id).0;
    let curve_pool =
        Pubkey::find_program_address(&[b"pool", pool.base_mint.as_ref()], &program_id).0;
    let coin_identity =
        Pubkey::find_program_address(&[b"coin", pool.base_mint.as_ref()], &program_id).0;
    let base_vault =
        Pubkey::find_program_address(&[b"pool_vault", pool.base_mint.as_ref()], &program_id).0;
    let quote_vault = Pubkey::find_program_address(
        &[
            b"pool_vault",
            pool.base_mint.as_ref(),
            pool.quote_mint.as_ref(),
        ],
        &program_id,
    )
    .0;
    let creator_rewards = Pubkey::find_program_address(
        &[b"creator_rewards_vault", pool.base_mint.as_ref()],
        &program_id,
    )
    .0;
    let holder_rewards = Pubkey::find_program_address(
        &[b"holder_rewards_vault", pool.base_mint.as_ref()],
        &program_id,
    )
    .0;
    let subject_rewards =
        anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &coin.subject,
            &pool.quote_mint,
            &anchor_spl::token::ID,
        );
    let event_authority = Pubkey::find_program_address(&[b"__event_authority"], &program_id).0;
    require_keys_eq!(step[1].key(), authority, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[3].key(), curve_pool, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        pool.coin_identity,
        coin_identity,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[4].key(), base_vault, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[5].key(), quote_vault, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[6].key(),
        creator_rewards,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[7].key(),
        holder_rewards,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        subject_rewards,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[13].key(),
        event_authority,
        ArbitrageError::InvalidAccount
    );

    validate_token_account_for_mint_and_authority_key(&step[4], &step[9], &step[11], authority)?;
    validate_token_account_for_mint_and_authority_key(&step[5], &step[10], &step[12], authority)?;
    validate_token_account_for_mint_and_authority_key(&step[6], &step[10], &step[12], authority)?;
    validate_token_account_for_mint_and_authority_key(&step[7], &step[10], &step[12], authority)?;
    validate_token_account_for_mint_and_authority_key(
        &step[8],
        &step[10],
        &step[12],
        coin.subject,
    )?;

    let base_mint_data = step[9].try_borrow_data()?;
    let token_supply = read_u64(&base_mint_data, MINT_SUPPLY_OFFSET)?;
    drop(base_mint_data);
    require!(
        token_supply == coin.token_supply,
        ArbitrageError::InvalidAccount
    );
    let expected_base_vault = token_supply
        .checked_sub(pool.circulating_base)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        read_token_amount(&step[4])? == expected_base_vault
            && read_token_amount(&step[5])? == pool.real_quote,
        ArbitrageError::InvalidAccount
    );

    let (
        expected_input_mint,
        expected_output_mint,
        expected_input_program,
        expected_output_program,
    ) = if direction == SELL_DIRECTION {
        (
            pool.base_mint,
            pool.quote_mint,
            step[11].key(),
            step[12].key(),
        )
    } else {
        (
            pool.quote_mint,
            pool.base_mint,
            step[12].key(),
            step[11].key(),
        )
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        input_token_program.key(),
        expected_input_program,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        output_token_program.key(),
        expected_output_program,
        ArbitrageError::InvalidProgramId
    );
    let fees = if direction == SELL_DIRECTION {
        coin.sell_fee_bps
    } else {
        coin.buy_fee_bps
    };
    require!(
        fee_total_bps(fees)? == fee_rate,
        ArbitrageError::InvalidFeeAmount
    );
    let quoted_output = quote_exact_in(pool, fees, direction, amount_in, expected_base_vault)?;
    require!(
        minimum_amount_out <= quoted_output,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(RunnerRodeoValidation { quoted_output })
}

pub fn runner_rodeo_swap<'a, 'info>(
    accounts: RunnerRodeoAccounts<'a, 'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    validation: RunnerRodeoValidation,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(
        minimum_amount_out <= validation.quoted_output,
        ArbitrageError::InsufficientOutputAmount
    );
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    let instruction = Instruction {
        program_id: accounts.step[0].key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.step[1].key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new_readonly(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(accounts.user_input.key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new(accounts.step[4].key(), false),
            AccountMeta::new(accounts.step[5].key(), false),
            AccountMeta::new(accounts.step[6].key(), false),
            AccountMeta::new(accounts.step[7].key(), false),
            AccountMeta::new(accounts.step[8].key(), false),
            AccountMeta::new_readonly(accounts.step[9].key(), false),
            AccountMeta::new_readonly(accounts.step[10].key(), false),
            AccountMeta::new_readonly(accounts.step[11].key(), false),
            AccountMeta::new_readonly(accounts.step[12].key(), false),
            AccountMeta::new_readonly(accounts.system_program.key(), false),
            AccountMeta::new_readonly(accounts.step[13].key(), false),
            AccountMeta::new_readonly(accounts.step[0].key(), false),
        ],
        data,
    };
    let pre_output = read_token_amount(accounts.user_output)?;
    let infos = vec![
        accounts.step[1].clone(),
        accounts.payer.clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.step[4].clone(),
        accounts.step[5].clone(),
        accounts.step[6].clone(),
        accounts.step[7].clone(),
        accounts.step[8].clone(),
        accounts.step[9].clone(),
        accounts.step[10].clone(),
        accounts.step[11].clone(),
        accounts.step[12].clone(),
        accounts.system_program.clone(),
        accounts.step[13].clone(),
        accounts.step[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_output)?,
        fee_amount: 0,
    })
}

fn decode_coin(data: &[u8]) -> Result<CoinState> {
    require!(
        data.len() == COIN_ACCOUNT_LEN && data.starts_with(&COIN_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let coin = CoinState {
        state: read_u8(data, 8)?,
        subject: read_pubkey(data, 9)?,
        creator: read_pubkey(data, 41)?,
        buy_fee_bps: [
            read_u16(data, 73)?,
            read_u16(data, 75)?,
            read_u16(data, 77)?,
        ],
        sell_fee_bps: [
            read_u16(data, 79)?,
            read_u16(data, 81)?,
            read_u16(data, 83)?,
        ],
        migration_threshold: read_u64(data, 85)?,
        token_supply: read_u64(data, 93)?,
        migration_token_allocation: read_u64(data, 101)?,
    };
    require!(
        coin.state == ACTIVE_COIN_STATE
            && coin.subject != Pubkey::default()
            && coin.creator != Pubkey::default()
            && coin.migration_threshold > 0
            && coin.token_supply > 0
            && coin.migration_token_allocation > 0
            && coin.migration_token_allocation <= coin.token_supply
            && fee_total_bps(coin.buy_fee_bps)? < BPS_DENOMINATOR as u16
            && fee_total_bps(coin.sell_fee_bps)? < BPS_DENOMINATOR as u16,
        ArbitrageError::InvalidAccount
    );
    Ok(coin)
}

fn decode_pool(data: &[u8]) -> Result<CurvePoolState> {
    require!(
        data.len() == POOL_ACCOUNT_LEN && data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let pool = CurvePoolState {
        coin: read_pubkey(data, 8)?,
        coin_identity: read_pubkey(data, 40)?,
        base_vault: read_pubkey(data, 72)?,
        quote_vault: read_pubkey(data, 104)?,
        migration_token_allocation: read_u64(data, 136)?,
        virtual_base: read_u64(data, 144)?,
        virtual_quote: read_u64(data, 152)?,
        circulating_base: read_u64(data, 160)?,
        real_quote: read_u64(data, 168)?,
        migration_threshold: read_u64(data, 176)?,
        pool_flag: read_u8(data, 184)?,
        base_mint: read_pubkey(data, 185)?,
        quote_mint: read_pubkey(data, 217)?,
    };
    require!(
        pool.coin != Pubkey::default()
            && pool.coin_identity != Pubkey::default()
            && pool.base_vault != Pubkey::default()
            && pool.quote_vault != Pubkey::default()
            && pool.base_mint != Pubkey::default()
            && pool.quote_mint == WRAPPED_SOL_MINT
            && pool.virtual_base > pool.circulating_base
            && pool.virtual_quote > 0
            && pool.real_quote < pool.migration_threshold
            && pool.pool_flag == ACTIVE_POOL_FLAG,
        ArbitrageError::InvalidAccount
    );
    Ok(pool)
}

fn quote_exact_in(
    pool: CurvePoolState,
    fees: [u16; 3],
    direction: u8,
    amount_in: u64,
    available_base: u64,
) -> Result<u64> {
    let effective_base = pool
        .virtual_base
        .checked_sub(pool.circulating_base)
        .ok_or(ArbitrageError::MathOverflow)?;
    let effective_quote = pool
        .virtual_quote
        .checked_add(pool.real_quote)
        .ok_or(ArbitrageError::MathOverflow)?;
    match direction {
        SELL_DIRECTION => {
            let gross = u128::from(amount_in)
                .checked_mul(u128::from(effective_quote))
                .ok_or(ArbitrageError::MathOverflow)?
                .checked_div(
                    u128::from(effective_base)
                        .checked_add(u128::from(amount_in))
                        .ok_or(ArbitrageError::MathOverflow)?,
                )
                .ok_or(ArbitrageError::MathOverflow)?;
            let gross = u64::try_from(gross).map_err(|_| ArbitrageError::MathOverflow)?;
            require!(
                gross <= pool.real_quote,
                ArbitrageError::InsufficientLiquidity
            );
            let output = gross
                .checked_sub(component_fees(gross, fees)?)
                .ok_or(ArbitrageError::MathOverflow)?;
            require!(output > 0, ArbitrageError::InsufficientOutputAmount);
            Ok(output)
        }
        BUY_DIRECTION => {
            let net_input = amount_in
                .checked_sub(component_fees(amount_in, fees)?)
                .ok_or(ArbitrageError::MathOverflow)?;
            let remaining_quote = pool
                .migration_threshold
                .checked_sub(pool.real_quote)
                .ok_or(ArbitrageError::MathOverflow)?;
            require!(
                net_input > 0 && net_input <= remaining_quote,
                ArbitrageError::InsufficientLiquidity
            );
            let output = u128::from(net_input)
                .checked_mul(u128::from(effective_base))
                .ok_or(ArbitrageError::MathOverflow)?
                .checked_div(
                    u128::from(effective_quote)
                        .checked_add(u128::from(net_input))
                        .ok_or(ArbitrageError::MathOverflow)?,
                )
                .ok_or(ArbitrageError::MathOverflow)?;
            let output = u64::try_from(output).map_err(|_| ArbitrageError::MathOverflow)?;
            require!(
                output > 0 && output <= available_base,
                ArbitrageError::InsufficientLiquidity
            );
            Ok(output)
        }
        _ => Err(ArbitrageError::InvalidInstructionData.into()),
    }
}

fn component_fees(amount: u64, fees: [u16; 3]) -> Result<u64> {
    fees.into_iter().try_fold(0_u64, |total, bps| {
        let fee = u128::from(amount)
            .checked_mul(u128::from(bps))
            .ok_or(ArbitrageError::MathOverflow)?
            .checked_div(u128::from(BPS_DENOMINATOR))
            .ok_or(ArbitrageError::MathOverflow)?;
        total
            .checked_add(u64::try_from(fee).map_err(|_| ArbitrageError::MathOverflow)?)
            .ok_or_else(|| ArbitrageError::MathOverflow.into())
    })
}

fn fee_total_bps(fees: [u16; 3]) -> Result<u16> {
    fees.into_iter().try_fold(0_u16, |total, bps| {
        total
            .checked_add(bps)
            .ok_or_else(|| ArbitrageError::MathOverflow.into())
    })
}

fn read_u8(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or_else(|| ArbitrageError::InvalidAccount.into())
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

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_pool() -> CurvePoolState {
        CurvePoolState {
            coin: Pubkey::new_unique(),
            coin_identity: Pubkey::new_unique(),
            base_vault: Pubkey::new_unique(),
            quote_vault: Pubkey::new_unique(),
            migration_token_allocation: 800_000_000_000_000,
            virtual_base: 1_062_416_998_672_472,
            virtual_quote: 19_681_274_900,
            circulating_base: 46_442_062_432_983,
            real_quote: 899_666_879,
            migration_threshold: 60_000_000_000,
            pool_flag: 0,
            base_mint: Pubkey::new_unique(),
            quote_mint: WRAPPED_SOL_MINT,
        }
    }

    #[test]
    fn quote_matches_real_program_in_both_directions() {
        let pool = current_pool();
        assert_eq!(
            quote_exact_in(
                pool,
                [60, 5, 55],
                SELL_DIRECTION,
                1_000_000_000_000,
                953_557_937_567_017,
            )
            .expect("sell quote"),
            19_994_565
        );
        assert_eq!(
            quote_exact_in(
                pool,
                [60, 5, 55],
                BUY_DIRECTION,
                100_000_000,
                953_557_937_567_017,
            )
            .expect("buy quote"),
            4_853_944_733_603
        );
    }

    #[test]
    fn component_fees_floor_each_destination_independently() {
        assert_eq!(
            component_fees(260_169_804, [60, 5, 55]).expect("component fees"),
            3_122_035
        );
    }
}
