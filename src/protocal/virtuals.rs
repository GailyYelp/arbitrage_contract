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

pub const VIRTUALS_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("5U3EU2ubXtK84QcRjWVmYt9RaDyA8gKxdUrPFXmZyaki");
pub const VIRTUALS_STEP_ACCOUNTS: usize = 7;

const VIRTUALS_MINT: Pubkey = anchor_lang::pubkey!("3iQL8BFS2vE7mww4ehAqQHAsbmRNCrPxizWAT2Zfyr9y");
const PLATFORM_PROTOTYPE: Pubkey =
    anchor_lang::pubkey!("933jV351WDG23QTcHPqLFJxyYRrEPWRTR3qoPWi3jwEL");
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const POOL_ACCOUNT_LEN: usize = 90;
const POOL_DISCRIMINATOR: [u8; 8] = [71, 118, 5, 203, 5, 98, 135, 116];
const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
const FEE_BPS: u16 = 100;

pub struct VirtualsAccounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub pool: &'a AccountInfo<'info>,
    pub token_mint: &'a AccountInfo<'info>,
    pub user_virtuals_ata: &'a AccountInfo<'info>,
    pub user_token_ata: &'a AccountInfo<'info>,
    pub pool_token_ata: &'a AccountInfo<'info>,
    pub platform_prototype: &'a AccountInfo<'info>,
    pub platform_virtuals_ata: &'a AccountInfo<'info>,
    pub pool_virtuals_ata: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub user: &'a AccountInfo<'info>,
    pub user_token_out: &'a AccountInfo<'info>,
}

pub fn virtuals_swap<'info>(
    accounts: VirtualsAccounts<'_, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let pre_out = read_token_amount(accounts.user_token_out)?;
    let data = if direction == 0 {
        let amount_out = quote_buy_exact_in(
            accounts.pool,
            accounts.pool_token_ata,
            accounts.pool_virtuals_ata,
            amount_in,
        )?;
        require!(
            amount_out >= min_amount_out,
            ArbitrageError::InvalidSlippage
        );
        swap_data(BUY_DISCRIMINATOR, amount_out, amount_in)
    } else {
        swap_data(SELL_DISCRIMINATOR, amount_in, min_amount_out)
    };
    let metas = vec![
        AccountMeta::new_readonly(accounts.user.key(), true),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.token_mint.key(), false),
        AccountMeta::new(accounts.user_virtuals_ata.key(), false),
        AccountMeta::new(accounts.user_token_ata.key(), false),
        AccountMeta::new(accounts.pool_token_ata.key(), false),
        AccountMeta::new(accounts.platform_prototype.key(), false),
        AccountMeta::new(accounts.platform_virtuals_ata.key(), false),
        AccountMeta::new(accounts.pool_virtuals_ata.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let infos = vec![
        accounts.user.clone(),
        accounts.pool.clone(),
        accounts.token_mint.clone(),
        accounts.user_virtuals_ata.clone(),
        accounts.user_token_ata.clone(),
        accounts.pool_token_ata.clone(),
        accounts.platform_prototype.clone(),
        accounts.platform_virtuals_ata.clone(),
        accounts.pool_virtuals_ata.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: data.to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_token_out, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_virtuals_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == VIRTUALS_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(
        direction <= 1 && fee_rate == FEE_BPS,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[0].key(),
        VIRTUALS_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        VIRTUALS_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        token_program.key(),
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
    require_keys_eq!(
        step[4].key(),
        PLATFORM_PROTOTYPE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[4].owner,
        system_program.key(),
        ArbitrageError::InvalidAccount
    );

    let pool_data = step[1].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_ACCOUNT_LEN && pool_data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let token_mint = read_pubkey(&pool_data, 40)?;
    let virtual_y = read_u64(&pool_data, 72)?;
    let graduation_x = read_u64(&pool_data, 80)?;
    let state = *pool_data.get(88).ok_or(ArbitrageError::InvalidAccount)?;
    let bump = *pool_data.get(89).ok_or(ArbitrageError::InvalidAccount)?;
    require!(
        token_mint != VIRTUALS_MINT && virtual_y > 0 && graduation_x > 0 && state == 1,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[2].key(), token_mint, ArbitrageError::InvalidAccount);
    let expected_pool =
        Pubkey::find_program_address(&[b"vpool", token_mint.as_ref()], &VIRTUALS_PROGRAM_ID);
    require_keys_eq!(
        step[1].key(),
        expected_pool.0,
        ArbitrageError::InvalidAccount
    );
    require!(bump == expected_pool.1, ArbitrageError::InvalidAccount);
    let (expected_in, expected_out) = if direction == 0 {
        (VIRTUALS_MINT, token_mint)
    } else {
        (token_mint, VIRTUALS_MINT)
    };
    require_keys_eq!(in_mint.key(), expected_in, ArbitrageError::InvalidAccount);
    require_keys_eq!(out_mint.key(), expected_out, ArbitrageError::InvalidAccount);

    require_keys_eq!(
        step[3].key(),
        associated_token_address(step[1].key(), token_mint, token_program.key()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        associated_token_address(PLATFORM_PROTOTYPE, VIRTUALS_MINT, token_program.key()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        associated_token_address(step[1].key(), VIRTUALS_MINT, token_program.key()),
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(&step[3], &step[2], token_program, &step[1])?;
    validate_token_account_for_mint_and_authority(
        &step[5],
        out_or_in_virtuals(direction, in_mint, out_mint),
        token_program,
        &step[4],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[6],
        out_or_in_virtuals(direction, in_mint, out_mint),
        token_program,
        &step[1],
    )?;
    let virtuals_reserve = read_token_amount(&step[6])?;
    require!(
        virtuals_reserve < graduation_x && virtual_y.checked_add(virtuals_reserve).is_some(),
        ArbitrageError::InvalidAccount
    );
    require!(
        read_token_amount(&step[3])? > 0,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn out_or_in_virtuals<'a, 'info>(
    direction: u8,
    in_mint: &'a AccountInfo<'info>,
    out_mint: &'a AccountInfo<'info>,
) -> &'a AccountInfo<'info> {
    if direction == 0 {
        in_mint
    } else {
        out_mint
    }
}

fn associated_token_address(owner: Pubkey, mint: Pubkey, token_program: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

fn quote_buy_exact_in(
    pool: &AccountInfo<'_>,
    token_vault: &AccountInfo<'_>,
    virtuals_vault: &AccountInfo<'_>,
    amount_in: u64,
) -> Result<u64> {
    let pool_data = pool.try_borrow_data()?;
    let virtual_y = read_u64(&pool_data, 72)?;
    let graduation_x = read_u64(&pool_data, 80)?;
    let token_x = u128::from(read_token_amount(token_vault)?);
    let actual_virtuals = read_token_amount(virtuals_vault)?;
    let virtuals_y = u128::from(
        virtual_y
            .checked_add(actual_virtuals)
            .ok_or(ArbitrageError::MathOverflow)?,
    );
    let quotient = amount_in / 101;
    let net = quotient
        .checked_mul(100)
        .and_then(|value| value.checked_add((amount_in % 101).min(99)))
        .ok_or(ArbitrageError::MathOverflow)?;
    let total_cost = net
        .checked_add(net / 100)
        .ok_or(ArbitrageError::MathOverflow)?;
    let post_virtuals = actual_virtuals
        .checked_add(net)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        net > 0 && total_cost <= amount_in,
        ArbitrageError::InvalidAmount
    );
    require!(post_virtuals < graduation_x, ArbitrageError::InvalidAmount);
    let n = u128::from(net)
        .checked_add(1)
        .ok_or(ArbitrageError::MathOverflow)?;
    let output = token_x
        .checked_mul(n)
        .and_then(|value| value.checked_sub(1))
        .and_then(|value| value.checked_div(virtuals_y.checked_add(n)?))
        .ok_or(ArbitrageError::MathOverflow)?;
    u64::try_from(output).map_err(|_| ArbitrageError::MathOverflow.into())
}

fn swap_data(discriminator: [u8; 8], amount: u64, limit: u64) -> [u8; 24] {
    let mut data = [0u8; 24];
    data[..8].copy_from_slice(&discriminator);
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

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
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
    fn buy_exact_input_inverse_matches_confirmed_event() {
        let x = 896_632_774_576_136u128;
        let y = 6_691_702_746_163u128;
        let net = 112_080_054u128;
        let output = (x * (net + 1) - 1) / (y + net + 1);
        assert_eq!(output, 15_017_548_049);
        assert_eq!(y * output / (x - output), net);
        assert_eq!(y * (output + 1) / (x - output - 1), net + 1);
    }

    #[test]
    fn wire_data_matches_deployed_anchor_idl() {
        let buy = swap_data(BUY_DISCRIMINATOR, 15_017_548_049, 113_212_177);
        assert_eq!(&buy[..8], &BUY_DISCRIMINATOR);
        let mut amount = [0u8; 8];
        amount.copy_from_slice(&buy[8..16]);
        assert_eq!(u64::from_le_bytes(amount), 15_017_548_049);
        amount.copy_from_slice(&buy[16..24]);
        assert_eq!(u64::from_le_bytes(amount), 113_212_177);
        let sell = swap_data(SELL_DISCRIMINATOR, 2_051_832_376_587, 0);
        assert_eq!(&sell[..8], &SELL_DISCRIMINATOR);
        amount.copy_from_slice(&sell[8..16]);
        assert_eq!(u64::from_le_bytes(amount), 2_051_832_376_587);
    }

    #[test]
    fn semantic_validation_binds_token_program_before_authority_in_both_directions() {
        let token_program_key = anchor_spl::token::ID;
        let token_mint = Pubkey::new_unique();
        let (pool, bump) =
            Pubkey::find_program_address(&[b"vpool", token_mint.as_ref()], &VIRTUALS_PROGRAM_ID);
        let token_vault = associated_token_address(pool, token_mint, token_program_key);
        let pool_virtuals_vault = associated_token_address(pool, VIRTUALS_MINT, token_program_key);
        let platform_vault =
            associated_token_address(PLATFORM_PROTOTYPE, VIRTUALS_MINT, token_program_key);
        let mut pool_data = vec![0u8; POOL_ACCOUNT_LEN];
        pool_data[..8].copy_from_slice(&POOL_DISCRIMINATOR);
        pool_data[40..72].copy_from_slice(token_mint.as_ref());
        pool_data[72..80].copy_from_slice(&6_000_000_000_000_u64.to_le_bytes());
        pool_data[80..88].copy_from_slice(&125_000_000_000_000_u64.to_le_bytes());
        pool_data[88] = 1;
        pool_data[89] = bump;
        let step = vec![
            test_account(
                VIRTUALS_PROGRAM_ID,
                Pubkey::new_unique(),
                false,
                true,
                Vec::new(),
            ),
            test_account(pool, VIRTUALS_PROGRAM_ID, true, false, pool_data),
            test_account(token_mint, token_program_key, false, false, vec![0; 82]),
            test_account(
                token_vault,
                token_program_key,
                true,
                false,
                token_data(token_mint, pool, 896_617_758_028_087),
            ),
            test_account(
                PLATFORM_PROTOTYPE,
                anchor_lang::system_program::ID,
                true,
                false,
                Vec::new(),
            ),
            test_account(
                platform_vault,
                token_program_key,
                true,
                false,
                token_data(VIRTUALS_MINT, PLATFORM_PROTOTYPE, 1),
            ),
            test_account(
                pool_virtuals_vault,
                token_program_key,
                true,
                false,
                token_data(VIRTUALS_MINT, pool, 691_814_818_753),
            ),
        ];
        let virtuals_mint =
            test_account(VIRTUALS_MINT, token_program_key, false, false, vec![0; 82]);
        let token_mint_account =
            test_account(token_mint, token_program_key, false, false, vec![0; 82]);
        let token_program = test_account(
            token_program_key,
            Pubkey::new_unique(),
            false,
            true,
            Vec::new(),
        );
        let system_program = test_account(
            anchor_lang::system_program::ID,
            Pubkey::new_unique(),
            false,
            true,
            Vec::new(),
        );

        validate_virtuals_semantic_accounts(
            &step,
            0,
            FEE_BPS,
            &virtuals_mint,
            &token_mint_account,
            &token_program,
            &system_program,
        )
        .expect("validate Virtuals buy");
        validate_virtuals_semantic_accounts(
            &step,
            1,
            FEE_BPS,
            &token_mint_account,
            &virtuals_mint,
            &token_program,
            &system_program,
        )
        .expect("validate Virtuals sell");
    }
}
