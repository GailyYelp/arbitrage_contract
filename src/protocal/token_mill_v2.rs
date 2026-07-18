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
        program_ids::TOKEN_MILL_V2_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint,
            validate_token_account_for_mint_and_authority, SwapResult,
        },
    },
};

pub const TOKEN_MILL_V2_STEP_ACCOUNTS: usize = 10;
pub const TOKEN_MILL_V2_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("2JtBWhLnkYbE8tn2DT8QXVweeQjnNYr43GbyiKhANdt5");
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const MARKET_DISCRIMINATOR: [u8; 8] = [0xdb, 0xbe, 0xd5, 0x37, 0x00, 0xe3, 0xc6, 0x9a];
const CONFIG_DISCRIMINATOR: [u8; 8] = [0x1c, 0xc8, 0x8d, 0xce, 0x8d, 0xb7, 0xcb, 0x10];
const FEE_DENOMINATOR: u64 = 1_000_000;
const CONFIG_QUOTE_MINT_OFFSET: usize = 40;
const CONFIG_PROTOCOL_FEE_SHARE_OFFSET: usize = 72;
const CONFIG_PROTOCOL_FEE_RESERVE_OFFSET: usize = 76;
const CONFIG_CREATOR_FEE_POOL_OFFSET: usize = 108;
const CONFIG_PREFIX_LEN: usize = 140;
const MARKET_CONFIG_OFFSET: usize = 8;
const MARKET_SWAP_AUTHORITY_OPTION_OFFSET: usize = 72;
const MARKET_TOKEN_0_MINT_OFFSET: usize = 73;
const MARKET_TOKEN_1_MINT_OFFSET: usize = 105;
const MARKET_RESERVE_0_OFFSET: usize = 137;
const MARKET_RESERVE_1_OFFSET: usize = 169;
const MARKET_FEE_RESERVE_OPTION_OFFSET: usize = 201;

pub struct TokenMillV2Accounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_token0: &'a AccountInfo<'info>,
    pub user_token1: &'a AccountInfo<'info>,
    pub output_token_account: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_token_mill_v2_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == TOKEN_MILL_V2_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require_keys_eq!(
        step[0].key(),
        TOKEN_MILL_V2_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[8].key(),
        token_program.key(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[9].key(),
        TOKEN_MILL_V2_EVENT_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    require!(
        step[0].executable && step[8].executable && !step[9].executable,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        TOKEN_MILL_V2_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        TOKEN_MILL_V2_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );

    let config_data = step[1].try_borrow_data()?;
    require!(
        config_data.len() >= CONFIG_PREFIX_LEN && config_data[..8] == CONFIG_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    let quote_mint = read_pubkey(&config_data, CONFIG_QUOTE_MINT_OFFSET)?;
    let protocol_fee_share = read_u32(&config_data, CONFIG_PROTOCOL_FEE_SHARE_OFFSET)?;
    require!(
        u64::from(protocol_fee_share) <= FEE_DENOMINATOR,
        ArbitrageError::InvalidFeeAmount
    );
    let protocol_fee_reserve = read_pubkey(&config_data, CONFIG_PROTOCOL_FEE_RESERVE_OFFSET)?;
    let creator_fee_pool = read_pubkey(&config_data, CONFIG_CREATOR_FEE_POOL_OFFSET)?;
    drop(config_data);

    let market_data = step[2].try_borrow_data()?;
    require!(
        market_data.len() > MARKET_FEE_RESERVE_OPTION_OFFSET
            && market_data[..8] == MARKET_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&market_data, MARKET_CONFIG_OFFSET)?,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        market_data[MARKET_SWAP_AUTHORITY_OPTION_OFFSET] == 0,
        ArbitrageError::InvalidAccount
    );
    let token0_mint = read_pubkey(&market_data, MARKET_TOKEN_0_MINT_OFFSET)?;
    let token1_mint = read_pubkey(&market_data, MARKET_TOKEN_1_MINT_OFFSET)?;
    let reserve0 = read_pubkey(&market_data, MARKET_RESERVE_0_OFFSET)?;
    let reserve1 = read_pubkey(&market_data, MARKET_RESERVE_1_OFFSET)?;
    let fee_reserve_tag = market_data[MARKET_FEE_RESERVE_OPTION_OFFSET];
    let (effective_fee_reserve, settings_offset) = match fee_reserve_tag {
        0 => (creator_fee_pool, 210usize),
        1 => (read_pubkey(&market_data, 202)?, 242usize),
        _ => return Err(ArbitrageError::InvalidAccount.into()),
    };
    let sqrt_price_a = read_u128(&market_data, settings_offset + 8)?;
    let sqrt_price_b = read_u128(&market_data, settings_offset + 24)?;
    let liquidity_a = read_u128(&market_data, settings_offset + 40)?;
    let liquidity_b = read_u128(&market_data, settings_offset + 56)?;
    let fee = read_u32(&market_data, settings_offset + 72)?;
    let sqrt_price = read_u128(&market_data, settings_offset + 76)?;
    require!(
        sqrt_price_a > 0
            && sqrt_price_a < sqrt_price_b
            && liquidity_a > 0
            && liquidity_b > 0
            && sqrt_price >= sqrt_price_a
            && u64::from(fee) < FEE_DENOMINATOR,
        ArbitrageError::InvalidAccount
    );
    let expected_fee_rate = u64::from(fee)
        .checked_mul(10_000)
        .ok_or(ArbitrageError::MathOverflow)?
        / FEE_DENOMINATOR;
    require!(
        u64::from(fee_rate) == expected_fee_rate,
        ArbitrageError::InvalidFeeAmount
    );
    require_keys_eq!(step[3].key(), reserve0, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[4].key(), reserve1, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[5].key(),
        effective_fee_reserve,
        ArbitrageError::InvalidAccount
    );
    drop(market_data);
    require_keys_eq!(token1_mint, quote_mint, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        step[6].key(),
        protocol_fee_reserve,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[7].key(),
        creator_fee_pool,
        ArbitrageError::InvalidAccount
    );

    let (expected_input_mint, expected_output_mint, token0_mint_account, token1_mint_account) =
        if direction == 0 {
            (token0_mint, token1_mint, input_mint, output_mint)
        } else {
            (token1_mint, token0_mint, output_mint, input_mint)
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
    validate_token_account_for_mint_and_authority(
        &step[3],
        token0_mint_account,
        token_program,
        &step[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[4],
        token1_mint_account,
        token_program,
        &step[2],
    )?;
    validate_token_account_for_mint(&step[5], token1_mint_account, token_program)?;
    validate_token_account_for_mint(&step[6], token1_mint_account, token_program)?;
    validate_token_account_for_mint(&step[7], token1_mint_account, token_program)?;
    Ok(())
}

pub fn token_mill_v2_swap<'a, 'info>(
    accounts: TokenMillV2Accounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(
        amount_in > 0 && min_amount_out > 0,
        ArbitrageError::InvalidAmount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let variant = if direction == 0 { 2 } else { 0 };
    let instruction = Instruction {
        program_id: accounts.step[0].key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.step[1].key(), false),
            AccountMeta::new(accounts.step[2].key(), false),
            AccountMeta::new(accounts.step[3].key(), false),
            AccountMeta::new(accounts.user_token0.key(), false),
            AccountMeta::new(accounts.step[4].key(), false),
            AccountMeta::new(accounts.user_token1.key(), false),
            AccountMeta::new(accounts.step[5].key(), false),
            AccountMeta::new(accounts.step[6].key(), false),
            AccountMeta::new(accounts.step[7].key(), false),
            AccountMeta::new_readonly(accounts.payer.key(), true),
            AccountMeta::new_readonly(accounts.step[0].key(), false),
            AccountMeta::new_readonly(accounts.step[8].key(), false),
            AccountMeta::new_readonly(accounts.step[9].key(), false),
            AccountMeta::new_readonly(accounts.step[0].key(), false),
        ],
        data: swap_exact_in_data(variant, amount_in, min_amount_out),
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let infos = vec![
        accounts.step[1].clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.user_token0.clone(),
        accounts.step[4].clone(),
        accounts.user_token1.clone(),
        accounts.step[5].clone(),
        accounts.step[6].clone(),
        accounts.step[7].clone(),
        accounts.payer.clone(),
        accounts.step[0].clone(),
        accounts.step[8].clone(),
        accounts.step[9].clone(),
        accounts.step[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn swap_exact_in_data(variant: u8, amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(&SWAP_DISCRIMINATOR);
    data.push(variant);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(array))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let array: [u8; 4] = bytes
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(array))
}

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let bytes = data
        .get(offset..offset + 16)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let array: [u8; 16] = bytes
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u128::from_le_bytes(array))
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
        let lamports = Box::leak(Box::new(1_u64));
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

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + 32].copy_from_slice(key.as_ref());
    }

    fn write_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u128(data: &mut [u8], offset: usize, value: u128) {
        data[offset..offset + 16].copy_from_slice(&value.to_le_bytes());
    }

    fn token_account_data(mint: Pubkey, authority: Pubkey) -> Vec<u8> {
        let mut data = vec![0_u8; 165];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        write_u64(&mut data, 64, 1_000_000);
        data
    }

    fn semantic_fixture(
        custom_fee_reserve: bool,
        permissioned: bool,
    ) -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let config = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let token0 = Pubkey::new_unique();
        let token1 = Pubkey::new_unique();
        let reserve0 = Pubkey::new_unique();
        let reserve1 = Pubkey::new_unique();
        let protocol_fee_reserve = Pubkey::new_unique();
        let creator_fee_pool = Pubkey::new_unique();
        let custom_fee = Pubkey::new_unique();

        let mut config_data = vec![0_u8; CONFIG_PREFIX_LEN];
        config_data[..8].copy_from_slice(&CONFIG_DISCRIMINATOR);
        write_pubkey(&mut config_data, CONFIG_QUOTE_MINT_OFFSET, token1);
        write_u32(&mut config_data, CONFIG_PROTOCOL_FEE_SHARE_OFFSET, 250_000);
        write_pubkey(
            &mut config_data,
            CONFIG_PROTOCOL_FEE_RESERVE_OFFSET,
            protocol_fee_reserve,
        );
        write_pubkey(
            &mut config_data,
            CONFIG_CREATOR_FEE_POOL_OFFSET,
            creator_fee_pool,
        );

        let settings_offset = if custom_fee_reserve { 242 } else { 210 };
        let mut market_data = vec![0_u8; settings_offset + 92];
        market_data[..8].copy_from_slice(&MARKET_DISCRIMINATOR);
        write_pubkey(&mut market_data, MARKET_CONFIG_OFFSET, config);
        market_data[MARKET_SWAP_AUTHORITY_OPTION_OFFSET] = u8::from(permissioned);
        write_pubkey(&mut market_data, MARKET_TOKEN_0_MINT_OFFSET, token0);
        write_pubkey(&mut market_data, MARKET_TOKEN_1_MINT_OFFSET, token1);
        write_pubkey(&mut market_data, MARKET_RESERVE_0_OFFSET, reserve0);
        write_pubkey(&mut market_data, MARKET_RESERVE_1_OFFSET, reserve1);
        if custom_fee_reserve {
            market_data[MARKET_FEE_RESERVE_OPTION_OFFSET] = 1;
            write_pubkey(&mut market_data, 202, custom_fee);
        }
        write_u64(&mut market_data, settings_offset, 1_000_000_000);
        write_u128(&mut market_data, settings_offset + 8, 100);
        write_u128(&mut market_data, settings_offset + 24, 1_000);
        write_u128(&mut market_data, settings_offset + 40, 10_000);
        write_u128(&mut market_data, settings_offset + 56, 20_000);
        write_u32(&mut market_data, settings_offset + 72, 20_000);
        write_u128(&mut market_data, settings_offset + 76, 500);

        let effective_fee_reserve = if custom_fee_reserve {
            custom_fee
        } else {
            creator_fee_pool
        };
        let token_program = test_account(
            anchor_spl::token::ID,
            anchor_lang::system_program::ID,
            false,
            true,
            Vec::new(),
        );
        let step = vec![
            test_account(
                TOKEN_MILL_V2_PROGRAM_ID,
                anchor_lang::system_program::ID,
                false,
                true,
                Vec::new(),
            ),
            test_account(config, TOKEN_MILL_V2_PROGRAM_ID, false, false, config_data),
            test_account(market, TOKEN_MILL_V2_PROGRAM_ID, true, false, market_data),
            test_account(
                reserve0,
                anchor_spl::token::ID,
                true,
                false,
                token_account_data(token0, market),
            ),
            test_account(
                reserve1,
                anchor_spl::token::ID,
                true,
                false,
                token_account_data(token1, market),
            ),
            test_account(
                effective_fee_reserve,
                anchor_spl::token::ID,
                true,
                false,
                token_account_data(token1, config),
            ),
            test_account(
                protocol_fee_reserve,
                anchor_spl::token::ID,
                true,
                false,
                token_account_data(token1, config),
            ),
            test_account(
                creator_fee_pool,
                anchor_spl::token::ID,
                true,
                false,
                token_account_data(token1, config),
            ),
            token_program.clone(),
            test_account(
                TOKEN_MILL_V2_EVENT_AUTHORITY,
                anchor_lang::system_program::ID,
                false,
                false,
                Vec::new(),
            ),
        ];
        let token0_mint = test_account(token0, anchor_spl::token::ID, false, false, Vec::new());
        let token1_mint = test_account(token1, anchor_spl::token::ID, false, false, Vec::new());
        (step, token0_mint, token1_mint, token_program)
    }

    #[test]
    fn swap_data_matches_official_exact_in_layout() {
        let data = swap_exact_in_data(2, 1_000_000, 35);
        assert_eq!(data.len(), 25);
        assert_eq!(&data[..8], &SWAP_DISCRIMINATOR);
        assert_eq!(data[8], 2);
        assert_eq!(&data[9..17], &1_000_000_u64.to_le_bytes());
        assert_eq!(&data[17..25], &35_u64.to_le_bytes());
    }

    #[test]
    fn semantic_validation_accepts_default_and_custom_fee_reserves() {
        for custom_fee_reserve in [false, true] {
            let (step, token0, token1, token_program) = semantic_fixture(custom_fee_reserve, false);
            assert!(validate_token_mill_v2_semantic_accounts(
                &step,
                0,
                200,
                &token0,
                &token1,
                &token_program,
            )
            .is_ok());
        }
    }

    #[test]
    fn semantic_validation_rejects_permissioned_market_and_wrong_fee() {
        let (permissioned, token0, token1, token_program) = semantic_fixture(false, true);
        assert!(validate_token_mill_v2_semantic_accounts(
            &permissioned,
            0,
            200,
            &token0,
            &token1,
            &token_program,
        )
        .is_err());

        let (permissionless, token0, token1, token_program) = semantic_fixture(false, false);
        assert!(validate_token_mill_v2_semantic_accounts(
            &permissionless,
            0,
            199,
            &token0,
            &token1,
            &token_program,
        )
        .is_err());
    }
}
