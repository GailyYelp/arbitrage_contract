use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::rise_program_id,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const RISE_STEP_ACCOUNTS: usize = 17;

#[cfg(not(feature = "devnet"))]
const MAYFLOWER_PROGRAM_ID_MAINNET: Pubkey =
    anchor_lang::pubkey!("AVMmmRzwc2kETQNhPiFVnyu62HrgsQXTD6D7SnSfEz7v");
#[cfg(feature = "devnet")]
const MAYFLOWER_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("MD2pPJCjpUT5ttJFUVeP2Xka1ZSvCJMZUoX4XTdPdet");
const RISE_TENANT: Pubkey = anchor_lang::pubkey!("5scY2JGWLnBubCMbWrn1gi8FQEP8SPjvQ1hfjW4ktYUb");
const RISE_TENANT_SEED: Pubkey =
    anchor_lang::pubkey!("Eg4Akr8HRv3gy4MaSp3zgKgC5qnN1V5ZTqAjhT54xJ9L");
const MAYFLOWER_TENANT: Pubkey =
    anchor_lang::pubkey!("HeBDu9g5EN6qdDJWijHHpxYuMBE6aWvy1BmzFyEa7Q7C");
const TOKEN_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const RISE_MARKET_ACCOUNT_LEN: usize = 598;
const MAYFLOWER_MARKET_LINEAR_ACCOUNT_LEN: usize = 304;
const MAYFLOWER_MARKET_META_MIN_LEN: usize = 361;
const MAYFLOWER_LOG_ACCOUNT_LEN: usize = 17;
const RISE_MARKET_DISCRIMINATOR: [u8; 8] = [219, 190, 213, 55, 0, 227, 198, 154];
const MAYFLOWER_MARKET_LINEAR_DISCRIMINATOR: [u8; 8] = [133, 114, 237, 100, 77, 96, 120, 49];
const MAYFLOWER_MARKET_META_DISCRIMINATOR: [u8; 8] = [95, 146, 205, 231, 152, 205, 151, 183];
const RISE_BUY_DISCRIMINATOR: [u8; 8] = [53, 248, 95, 20, 54, 162, 146, 247];
const RISE_SELL_DISCRIMINATOR: [u8; 8] = [27, 141, 98, 109, 197, 168, 104, 84];

const MARKET_TENANT_OFFSET: usize = 8;
const MARKET_META_OFFSET: usize = 40;
const MARKET_TOKEN_MINT_OFFSET: usize = 72;
const MARKET_MAIN_MINT_OFFSET: usize = 104;
const MARKET_CASH_ESCROW_OFFSET: usize = 137;
const MARKET_BUY_FEE_OFFSET: usize = 169;
const MARKET_SELL_FEE_OFFSET: usize = 201;

#[cfg(feature = "devnet")]
const fn mayflower_program_id() -> Pubkey {
    MAYFLOWER_PROGRAM_ID_DEVNET
}

#[cfg(not(feature = "devnet"))]
const fn mayflower_program_id() -> Pubkey {
    MAYFLOWER_PROGRAM_ID_MAINNET
}

pub struct RiseAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_rise_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == RISE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require_keys_eq!(
        step[0].key(),
        rise_program_id(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(step[1].key(), RISE_TENANT, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[4].key(),
        MAYFLOWER_TENANT,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        RISE_TENANT_SEED,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[12].key(),
        mayflower_program_id(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        token_program.key(),
        TOKEN_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *input_mint.owner,
        TOKEN_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *output_mint.owner,
        TOKEN_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );

    let market_data = step[2].try_borrow_data()?;
    require!(
        *step[2].owner == rise_program_id()
            && market_data.len() == RISE_MARKET_ACCOUNT_LEN
            && market_data.starts_with(&RISE_MARKET_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let market_tenant = read_pubkey(&market_data, MARKET_TENANT_OFFSET)?;
    let market_meta = read_pubkey(&market_data, MARKET_META_OFFSET)?;
    let token_mint = read_pubkey(&market_data, MARKET_TOKEN_MINT_OFFSET)?;
    let main_mint = read_pubkey(&market_data, MARKET_MAIN_MINT_OFFSET)?;
    let cash_escrow = read_pubkey(&market_data, MARKET_CASH_ESCROW_OFFSET)?;
    require_keys_eq!(market_tenant, step[1].key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(market_meta, step[6].key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(cash_escrow, step[3].key(), ArbitrageError::InvalidAccount);
    let micro_basis_points = read_u32(
        &market_data,
        if direction == 0 {
            MARKET_BUY_FEE_OFFSET
        } else {
            MARKET_SELL_FEE_OFFSET
        },
    )?;
    require!(
        micro_basis_points % 100 == 0
            && u32::from(fee_rate) == micro_basis_points / 100
            && micro_basis_points < 1_000_000,
        ArbitrageError::InvalidAccount
    );
    drop(market_data);

    let (expected_input, expected_output) = if direction == 0 {
        (main_mint, token_mint)
    } else {
        (token_mint, main_mint)
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

    let meta_data = step[6].try_borrow_data()?;
    require!(
        *step[6].owner == mayflower_program_id()
            && meta_data.len() >= MAYFLOWER_MARKET_META_MIN_LEN
            && meta_data.starts_with(&MAYFLOWER_MARKET_META_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 8)?,
        main_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 40)?,
        token_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 104)?,
        step[5].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 136)?,
        step[7].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 168)?,
        TOKEN_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 200)?,
        step[9].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 232)?,
        step[10].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&meta_data, 264)?,
        step[11].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        *meta_data.get(360).ok_or(ArbitrageError::InvalidAccount)? == 0,
        ArbitrageError::InvalidAccount
    );
    drop(meta_data);

    let linear_data = step[7].try_borrow_data()?;
    require!(
        *step[7].owner == mayflower_program_id()
            && linear_data.len() == MAYFLOWER_MARKET_LINEAR_ACCOUNT_LEN
            && linear_data.starts_with(&MAYFLOWER_MARKET_LINEAR_DISCRIMINATOR)
            && read_pubkey(&linear_data, 8)? == market_meta,
        ArbitrageError::InvalidAccount
    );
    drop(linear_data);

    require_keys_eq!(
        step[7].key(),
        pda(b"market_linear", market_meta, mayflower_program_id()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[9].key(),
        pda(b"liq_vault_main", market_meta, mayflower_program_id()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[10].key(),
        pda(b"rev_escrow_group", market_meta, mayflower_program_id()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[11].key(),
        pda(b"rev_escrow_tenant", market_meta, mayflower_program_id()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[13].key(),
        Pubkey::find_program_address(&[b"log"], &mayflower_program_id()).0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[14].key(),
        pda(b"creator_escrow", step[2].key(), rise_program_id()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[15].key(),
        pda(b"team_escrow", main_mint, rise_program_id()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[16].key(),
        Pubkey::find_program_address(&[b"__event_authority"], &rise_program_id()).0,
        ArbitrageError::InvalidAccount
    );
    require!(
        *step[5].owner == mayflower_program_id()
            && *step[13].owner == mayflower_program_id()
            && step[13].data_len() == MAYFLOWER_LOG_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );

    let main_mint_account = if direction == 0 {
        input_mint
    } else {
        output_mint
    };
    validate_token_account_for_mint_and_authority(
        &step[3],
        main_mint_account,
        token_program,
        &step[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[9],
        main_mint_account,
        token_program,
        &step[6],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[10],
        main_mint_account,
        token_program,
        &step[6],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[11],
        main_mint_account,
        token_program,
        &step[4],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[14],
        main_mint_account,
        token_program,
        &step[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[15],
        main_mint_account,
        token_program,
        &step[15],
    )?;
    Ok(())
}

pub fn rise_swap<'info>(
    accounts: RiseAccounts<'_, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(
        accounts.step.len() == RISE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    let (metas, infos, data) = if direction == 0 {
        build_buy(&accounts, amount_in, min_amount_out)
    } else {
        build_sell(&accounts, amount_in, min_amount_out)
    };
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

fn build_buy<'a, 'info>(
    accounts: &'a RiseAccounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
) -> (Vec<AccountMeta>, Vec<AccountInfo<'info>>, Vec<u8>) {
    let s = accounts.step;
    let ordered = vec![
        accounts.payer,
        &s[1],
        &s[2],
        &s[3],
        &s[4],
        &s[5],
        &s[6],
        &s[7],
        &s[8],
        accounts.output_mint,
        accounts.input_mint,
        accounts.user_output,
        accounts.user_input,
        &s[9],
        &s[10],
        &s[11],
        accounts.token_program,
        accounts.token_program,
        &s[12],
        &s[13],
        &s[14],
        &s[15],
        &s[16],
        &s[0],
    ];
    let metas = ordered
        .iter()
        .enumerate()
        .map(|(index, account)| {
            rise_meta(
                account,
                index == 0,
                matches!(
                    index,
                    0 | 1 | 2 | 3 | 7 | 9 | 11 | 12 | 13 | 14 | 15 | 19 | 20 | 21
                ),
            )
        })
        .collect();
    (
        metas,
        ordered.into_iter().cloned().collect(),
        buy_data(amount_in, min_amount_out),
    )
}

fn build_sell<'a, 'info>(
    accounts: &'a RiseAccounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
) -> (Vec<AccountMeta>, Vec<AccountInfo<'info>>, Vec<u8>) {
    let s = accounts.step;
    let ordered = vec![
        accounts.payer,
        &s[1],
        &s[2],
        &s[3],
        &s[4],
        &s[5],
        &s[6],
        &s[7],
        accounts.input_mint,
        accounts.output_mint,
        accounts.user_input,
        accounts.user_output,
        &s[9],
        &s[10],
        &s[11],
        accounts.token_program,
        accounts.token_program,
        &s[12],
        &s[13],
        &s[14],
        &s[15],
        &s[16],
        &s[0],
    ];
    let metas = ordered
        .iter()
        .enumerate()
        .map(|(index, account)| {
            rise_meta(
                account,
                index == 0,
                matches!(
                    index,
                    0 | 1 | 2 | 3 | 7 | 8 | 10 | 11 | 12 | 13 | 14 | 18 | 19 | 20
                ),
            )
        })
        .collect();
    (
        metas,
        ordered.into_iter().cloned().collect(),
        sell_data(amount_in, min_amount_out),
    )
}

fn rise_meta(account: &AccountInfo<'_>, signer: bool, writable: bool) -> AccountMeta {
    if writable {
        AccountMeta::new(account.key(), signer)
    } else {
        AccountMeta::new_readonly(account.key(), signer)
    }
}

fn buy_data(amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(88);
    data.extend_from_slice(&RISE_BUY_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
    data.extend_from_slice(&[0; 16]);
    data.extend_from_slice(&[0; 16]);
    data.extend_from_slice(&100_000_000_u64.to_le_bytes());
    data.extend_from_slice(&[0; 16]);
    data
}

fn sell_data(amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&RISE_SELL_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn pda(seed: &[u8], key: Pubkey, program: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[seed, key.as_ref()], &program).0
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes: [u8; 32] = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset.checked_add(4).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account(is_signer: bool, is_writable: bool) -> AccountInfo<'static> {
        let key = Box::leak(Box::new(Pubkey::new_unique()));
        let owner = Box::leak(Box::new(Pubkey::new_unique()));
        let lamports = Box::leak(Box::new(1_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            lamports,
            data,
            owner,
            false,
            u64::MAX,
        )
    }

    #[test]
    fn rise_instruction_wires_are_stable() {
        let buy = buy_data(7, 5);
        assert_eq!(buy.len(), 88);
        assert_eq!(&buy[..8], &RISE_BUY_DISCRIMINATOR);
        assert_eq!(u64::from_le_bytes(buy[8..16].try_into().unwrap()), 7);
        assert_eq!(u64::from_le_bytes(buy[16..24].try_into().unwrap()), 5);
        assert_eq!(
            u64::from_le_bytes(buy[64..72].try_into().unwrap()),
            100_000_000
        );

        let sell = sell_data(11, 9);
        assert_eq!(sell.len(), 24);
        assert_eq!(&sell[..8], &RISE_SELL_DISCRIMINATOR);
        assert_eq!(u64::from_le_bytes(sell[8..16].try_into().unwrap()), 11);
        assert_eq!(u64::from_le_bytes(sell[16..24].try_into().unwrap()), 9);
    }

    #[test]
    fn rise_cpi_marks_buyer_signer_and_writable() {
        let payer = test_account(true, true);
        let user_input = test_account(false, true);
        let user_output = test_account(false, true);
        let input_mint = test_account(false, true);
        let output_mint = test_account(false, true);
        let token_program = test_account(false, false);
        let step = (0..RISE_STEP_ACCOUNTS)
            .map(|_| test_account(false, true))
            .collect::<Vec<_>>();
        let accounts = RiseAccounts {
            payer: &payer,
            user_input: &user_input,
            user_output: &user_output,
            input_mint: &input_mint,
            output_mint: &output_mint,
            token_program: &token_program,
            step: &step,
        };

        let (buy_metas, _, _) = build_buy(&accounts, 7, 5);
        let (sell_metas, _, _) = build_sell(&accounts, 7, 5);
        for buyer in [&buy_metas[0], &sell_metas[0]] {
            assert!(buyer.is_signer);
            assert!(buyer.is_writable);
        }
    }
}
