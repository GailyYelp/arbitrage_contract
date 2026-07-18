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
        program_ids::DRADEX_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const DRADEX_STEP_ACCOUNTS: usize = 14;

const DRADEX_MASTER: Pubkey = anchor_lang::pubkey!("76ygWLcvJLR6nSzRUpiQqqtX4Pabkq4vpaTxwr3mDcXA");
const DRADEX_LOGGER_PROGRAM: Pubkey =
    anchor_lang::pubkey!("1gE3LGQze8DQ3KD2C4ZUCmRX5g4njhY5yLfYmnmcvJR");
const RENT_SYSVAR: Pubkey = anchor_lang::pubkey!("SysvarRent111111111111111111111111111111111");

const MARKET_LEN: usize = 570;
const EVENT_QUEUE_LEN: usize = 9_232;
const ORDERBOOK_LEN: usize = 36_505;
const MARKET_DISCRIMINATOR: [u8; 8] = [219, 190, 213, 55, 0, 227, 198, 154];
const PAIR_DISCRIMINATOR: [u8; 8] = [85, 72, 49, 176, 182, 228, 141, 82];
const MARKET_USER_DISCRIMINATOR: [u8; 8] = [236, 236, 149, 82, 151, 154, 15, 23];
const DEX_USER_DISCRIMINATOR: [u8; 8] = [97, 238, 123, 182, 146, 251, 87, 192];
const CREATE_ORDER_DISCRIMINATOR: [u8; 8] = [141, 54, 37, 207, 237, 210, 250, 215];
const SETTLE_FUNDS_DISCRIMINATOR: [u8; 8] = [238, 64, 163, 96, 75, 171, 16, 33];

const PAIR_OFFSET: usize = 8;
const T0_MINT_OFFSET: usize = 40;
const T1_MINT_OFFSET: usize = 72;
const T0_VAULT_OFFSET: usize = 104;
const T1_VAULT_OFFSET: usize = 136;
#[cfg(test)]
const MARKET_AUTHORITY_OFFSET: usize = 200;
const BIDS_OFFSET: usize = 264;
const ASKS_OFFSET: usize = 296;
const EVENT_QUEUE_OFFSET: usize = 378;

pub struct DradexAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

#[allow(clippy::too_many_arguments)]
pub fn validate_dradex_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    payer: &AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == DRADEX_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        DRADEX_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[10].key(),
        DRADEX_MASTER,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[11].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[12].key(),
        DRADEX_LOGGER_PROGRAM,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(step[13].key(), RENT_SYSVAR, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );

    validate_owned_account(&step[1], 8, &PAIR_DISCRIMINATOR)?;
    validate_owned_account(&step[2], MARKET_LEN, &MARKET_DISCRIMINATOR)?;
    validate_owned_account(&step[3], EVENT_QUEUE_LEN, &[])?;
    validate_owned_account(&step[6], ORDERBOOK_LEN, &[])?;
    validate_owned_account(&step[7], ORDERBOOK_LEN, &[])?;
    validate_owned_account(&step[4], 17, &DEX_USER_DISCRIMINATOR)?;
    validate_owned_account(&step[5], 48, &MARKET_USER_DISCRIMINATOR)?;

    let payer_key = payer.key();
    let market_key = step[2].key();
    let dex_user =
        Pubkey::find_program_address(&[b"dex_user", payer_key.as_ref()], &DRADEX_PROGRAM_ID).0;
    let market_user = Pubkey::find_program_address(
        &[b"market_user_v2", market_key.as_ref(), payer_key.as_ref()],
        &DRADEX_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(step[4].key(), dex_user, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[5].key(), market_user, ArbitrageError::InvalidAccount);

    let market = step[2].try_borrow_data()?;
    let pair = read_pubkey(&market, PAIR_OFFSET)?;
    let t0_mint = read_pubkey(&market, T0_MINT_OFFSET)?;
    let t1_mint = read_pubkey(&market, T1_MINT_OFFSET)?;
    let t0_vault = read_pubkey(&market, T0_VAULT_OFFSET)?;
    let t1_vault = read_pubkey(&market, T1_VAULT_OFFSET)?;
    let bids = read_pubkey(&market, BIDS_OFFSET)?;
    let asks = read_pubkey(&market, ASKS_OFFSET)?;
    let event_queue = read_pubkey(&market, EVENT_QUEUE_OFFSET)?;
    drop(market);

    require_keys_eq!(step[1].key(), pair, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[3].key(), event_queue, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[6].key(), bids, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[7].key(), asks, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[8].key(), t0_vault, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[9].key(), t1_vault, ArbitrageError::InvalidAccount);
    let (expected_input, expected_output, t0_mint_ai, t1_mint_ai) = match direction {
        0 => (t0_mint, t1_mint, input_mint, output_mint),
        1 => (t1_mint, t0_mint, output_mint, input_mint),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
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
    validate_token_account_for_mint_and_authority(&step[8], t0_mint_ai, token_program, &step[10])?;
    validate_token_account_for_mint_and_authority(&step[9], t1_mint_ai, token_program, &step[10])?;
    require!(
        read_token_amount(&step[8])? > 0 && read_token_amount(&step[9])? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
}

pub fn dradex_swap<'a, 'info>(
    accounts: DradexAccounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(min_amount_out > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let pre_out = read_token_amount(accounts.user_output)?;
    let create_order = create_order_instruction(&accounts, amount_in, min_amount_out, direction);
    invoke(&create_order, &create_order_infos(&accounts, direction))?;
    let settle_funds = settle_funds_instruction(&accounts, direction);
    invoke(&settle_funds, &settle_funds_infos(&accounts, direction))?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

fn create_order_instruction(
    accounts: &DradexAccounts<'_, '_>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Instruction {
    let s = accounts.step;
    let (t0_user, t1_user) = direction_users(accounts, direction);
    Instruction {
        program_id: s[0].key(),
        accounts: vec![
            AccountMeta::new(s[1].key(), false),
            AccountMeta::new(s[2].key(), false),
            AccountMeta::new(s[3].key(), false),
            AccountMeta::new_readonly(s[4].key(), false),
            AccountMeta::new(s[5].key(), false),
            AccountMeta::new(s[6].key(), false),
            AccountMeta::new(s[7].key(), false),
            AccountMeta::new(s[8].key(), false),
            AccountMeta::new(s[9].key(), false),
            AccountMeta::new(t0_user.key(), false),
            AccountMeta::new(t1_user.key(), false),
            AccountMeta::new_readonly(s[10].key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new_readonly(s[11].key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(s[12].key(), false),
        ],
        data: create_order_data(amount_in, min_amount_out, direction),
    }
}

fn settle_funds_instruction(accounts: &DradexAccounts<'_, '_>, direction: u8) -> Instruction {
    let s = accounts.step;
    let (t0_user, t1_user) = direction_users(accounts, direction);
    let (t0_mint, t1_mint) = direction_mints(accounts, direction);
    Instruction {
        program_id: s[0].key(),
        accounts: vec![
            AccountMeta::new(s[1].key(), false),
            AccountMeta::new(s[2].key(), false),
            AccountMeta::new(s[3].key(), false),
            AccountMeta::new_readonly(s[4].key(), false),
            AccountMeta::new(s[5].key(), false),
            AccountMeta::new_readonly(t0_mint.key(), false),
            AccountMeta::new_readonly(t1_mint.key(), false),
            AccountMeta::new(s[6].key(), false),
            AccountMeta::new(s[7].key(), false),
            AccountMeta::new(s[8].key(), false),
            AccountMeta::new(s[9].key(), false),
            AccountMeta::new(t0_user.key(), false),
            AccountMeta::new(t1_user.key(), false),
            AccountMeta::new_readonly(s[10].key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new_readonly(s[13].key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(s[12].key(), false),
        ],
        data: SETTLE_FUNDS_DISCRIMINATOR.to_vec(),
    }
}

fn create_order_infos<'a, 'info>(
    accounts: &DradexAccounts<'a, 'info>,
    direction: u8,
) -> Vec<AccountInfo<'info>> {
    let s = accounts.step;
    let (t0_user, t1_user) = direction_users(accounts, direction);
    vec![
        s[1].clone(),
        s[2].clone(),
        s[3].clone(),
        s[4].clone(),
        s[5].clone(),
        s[6].clone(),
        s[7].clone(),
        s[8].clone(),
        s[9].clone(),
        t0_user.clone(),
        t1_user.clone(),
        s[10].clone(),
        accounts.payer.clone(),
        s[11].clone(),
        accounts.token_program.clone(),
        s[12].clone(),
        s[0].clone(),
    ]
}

fn settle_funds_infos<'a, 'info>(
    accounts: &DradexAccounts<'a, 'info>,
    direction: u8,
) -> Vec<AccountInfo<'info>> {
    let s = accounts.step;
    let (t0_user, t1_user) = direction_users(accounts, direction);
    let (t0_mint, t1_mint) = direction_mints(accounts, direction);
    vec![
        s[1].clone(),
        s[2].clone(),
        s[3].clone(),
        s[4].clone(),
        s[5].clone(),
        t0_mint.clone(),
        t1_mint.clone(),
        s[6].clone(),
        s[7].clone(),
        s[8].clone(),
        s[9].clone(),
        t0_user.clone(),
        t1_user.clone(),
        s[10].clone(),
        accounts.payer.clone(),
        s[13].clone(),
        accounts.token_program.clone(),
        s[12].clone(),
        s[0].clone(),
    ]
}

fn direction_users<'a, 'info>(
    accounts: &'a DradexAccounts<'a, 'info>,
    direction: u8,
) -> (&'a AccountInfo<'info>, &'a AccountInfo<'info>) {
    if direction == 0 {
        (accounts.user_input, accounts.user_output)
    } else {
        (accounts.user_output, accounts.user_input)
    }
}

fn direction_mints<'a, 'info>(
    accounts: &'a DradexAccounts<'a, 'info>,
    direction: u8,
) -> (&'a AccountInfo<'info>, &'a AccountInfo<'info>) {
    if direction == 0 {
        (accounts.input_mint, accounts.output_mint)
    } else {
        (accounts.output_mint, accounts.input_mint)
    }
}

fn create_order_data(amount_in: u64, min_amount_out: u64, direction: u8) -> Vec<u8> {
    let (side, amount, limit_total) = if direction == 0 {
        (1_u8, amount_in, None)
    } else {
        (0_u8, u64::MAX, Some(amount_in))
    };
    let mut data = Vec::with_capacity(if limit_total.is_some() { 52 } else { 44 });
    data.extend_from_slice(&CREATE_ORDER_DISCRIMINATOR);
    data.push(side);
    data.extend_from_slice(&0_u64.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
    data.push(0);
    if let Some(limit_total) = limit_total {
        data.push(1);
        data.extend_from_slice(&limit_total.to_le_bytes());
    } else {
        data.push(0);
    }
    data.push(1);
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn validate_owned_account(
    account: &AccountInfo<'_>,
    expected_len: usize,
    discriminator: &[u8],
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        DRADEX_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() == expected_len && data.starts_with(discriminator),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN_ACCOUNT_LEN: usize = 165;

    fn account(
        key: Pubkey,
        owner: Pubkey,
        data: Vec<u8>,
        writable: bool,
        executable: bool,
    ) -> AccountInfo<'static> {
        AccountInfo::new(
            Box::leak(Box::new(key)),
            false,
            writable,
            Box::leak(Box::new(1_u64)),
            Box::leak(data.into_boxed_slice()),
            Box::leak(Box::new(owner)),
            executable,
            0,
        )
    }

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + 32].copy_from_slice(key.as_ref());
    }

    fn token_data(mint: Pubkey, authority: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0_u8; TOKEN_ACCOUNT_LEN];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1;
        data
    }

    fn semantic_fixture() -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let payer_key = Pubkey::new_unique();
        let pair = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let event_queue = Pubkey::new_unique();
        let bids = Pubkey::new_unique();
        let asks = Pubkey::new_unique();
        let t0_mint_key = Pubkey::new_unique();
        let t1_mint_key = Pubkey::new_unique();
        let t0_vault = Pubkey::new_unique();
        let t1_vault = Pubkey::new_unique();
        let market_authority = Pubkey::new_unique();
        let dex_user =
            Pubkey::find_program_address(&[b"dex_user", payer_key.as_ref()], &DRADEX_PROGRAM_ID).0;
        let market_user = Pubkey::find_program_address(
            &[b"market_user_v2", market.as_ref(), payer_key.as_ref()],
            &DRADEX_PROGRAM_ID,
        )
        .0;

        let mut market_data = vec![0_u8; MARKET_LEN];
        market_data[..8].copy_from_slice(&MARKET_DISCRIMINATOR);
        for (offset, key) in [
            (PAIR_OFFSET, pair),
            (T0_MINT_OFFSET, t0_mint_key),
            (T1_MINT_OFFSET, t1_mint_key),
            (T0_VAULT_OFFSET, t0_vault),
            (T1_VAULT_OFFSET, t1_vault),
            (MARKET_AUTHORITY_OFFSET, market_authority),
            (BIDS_OFFSET, bids),
            (ASKS_OFFSET, asks),
            (EVENT_QUEUE_OFFSET, event_queue),
        ] {
            write_pubkey(&mut market_data, offset, key);
        }
        let mut dex_user_data = vec![0_u8; 17];
        dex_user_data[..8].copy_from_slice(&DEX_USER_DISCRIMINATOR);
        let mut market_user_data = vec![0_u8; 48];
        market_user_data[..8].copy_from_slice(&MARKET_USER_DISCRIMINATOR);

        let step = vec![
            account(DRADEX_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(
                pair,
                DRADEX_PROGRAM_ID,
                PAIR_DISCRIMINATOR.to_vec(),
                true,
                false,
            ),
            account(market, DRADEX_PROGRAM_ID, market_data, true, false),
            account(
                event_queue,
                DRADEX_PROGRAM_ID,
                vec![0; EVENT_QUEUE_LEN],
                true,
                false,
            ),
            account(dex_user, DRADEX_PROGRAM_ID, dex_user_data, false, false),
            account(
                market_user,
                DRADEX_PROGRAM_ID,
                market_user_data,
                true,
                false,
            ),
            account(bids, DRADEX_PROGRAM_ID, vec![0; ORDERBOOK_LEN], true, false),
            account(asks, DRADEX_PROGRAM_ID, vec![0; ORDERBOOK_LEN], true, false),
            account(
                t0_vault,
                anchor_spl::token::ID,
                token_data(t0_mint_key, DRADEX_MASTER, 1_000_000),
                true,
                false,
            ),
            account(
                t1_vault,
                anchor_spl::token::ID,
                token_data(t1_mint_key, DRADEX_MASTER, 2_000_000),
                true,
                false,
            ),
            account(DRADEX_MASTER, DRADEX_PROGRAM_ID, vec![], false, false),
            account(
                anchor_lang::system_program::ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(
                DRADEX_LOGGER_PROGRAM,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(RENT_SYSVAR, Pubkey::default(), vec![], false, false),
        ];
        let payer = account(payer_key, Pubkey::default(), vec![], true, false);
        let t0_mint = account(t0_mint_key, anchor_spl::token::ID, vec![], false, false);
        let t1_mint = account(t1_mint_key, anchor_spl::token::ID, vec![], false, false);
        let token_program = account(
            anchor_spl::token::ID,
            Pubkey::default(),
            vec![],
            false,
            true,
        );
        (step, payer, t0_mint, t1_mint, token_program)
    }

    #[test]
    fn create_order_wire_matches_exact_in_bid_and_ask_layouts() {
        let ask = create_order_data(25, 20, 0);
        assert_eq!(ask.len(), 44);
        assert_eq!(ask[8], 1);
        assert_eq!(&ask[17..25], &25_u64.to_le_bytes());
        assert_eq!(ask[34], 0);
        assert_eq!(ask[35], 1);

        let bid = create_order_data(25, 20, 1);
        assert_eq!(bid.len(), 52);
        assert_eq!(bid[8], 0);
        assert_eq!(&bid[17..25], &u64::MAX.to_le_bytes());
        assert_eq!(bid[34], 1);
        assert_eq!(&bid[35..43], &25_u64.to_le_bytes());
        assert_eq!(bid[43], 1);
    }

    #[test]
    fn semantic_validation_binds_market_users_books_vaults_and_direction() {
        let (step, payer, t0_mint, t1_mint, token_program) = semantic_fixture();
        assert!(validate_dradex_semantic_accounts(
            &step,
            &payer,
            0,
            0,
            &t0_mint,
            &t1_mint,
            &token_program,
        )
        .is_ok());
        assert!(validate_dradex_semantic_accounts(
            &step,
            &payer,
            1,
            0,
            &t1_mint,
            &t0_mint,
            &token_program,
        )
        .is_ok());

        let (bad_market, payer, t0_mint, t1_mint, token_program) = semantic_fixture();
        bad_market[2].try_borrow_mut_data().unwrap()[BIDS_OFFSET] ^= 1;
        assert!(validate_dradex_semantic_accounts(
            &bad_market,
            &payer,
            0,
            0,
            &t0_mint,
            &t1_mint,
            &token_program,
        )
        .is_err());

        let (mut bad_user, payer, t0_mint, t1_mint, token_program) = semantic_fixture();
        bad_user[5] = account(
            Pubkey::new_unique(),
            DRADEX_PROGRAM_ID,
            {
                let mut data = vec![0_u8; 48];
                data[..8].copy_from_slice(&MARKET_USER_DISCRIMINATOR);
                data
            },
            true,
            false,
        );
        assert!(validate_dradex_semantic_accounts(
            &bad_user,
            &payer,
            0,
            0,
            &t0_mint,
            &t1_mint,
            &token_program,
        )
        .is_err());
    }
}
