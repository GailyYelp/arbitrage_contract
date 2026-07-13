use crate::{
    errors::ArbitrageError,
    instructions::types::{read_token_amount, token_balance_delta, SwapResult},
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const GUACSWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
pub const GUACSWAP_MIN_ACCOUNTS: usize = 10;
pub const GUACSWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("Gswppe6ERWKpUTXvRPfXdzHhiCyJvLadVvXGfdpBqcE1");
pub const GUACSWAP_STATE: Pubkey =
    anchor_lang::pubkey!("EzV4QKycvp4EDuAoTGjyY9HUS9Zn15vcmHkHJdNKHCnY");
pub const GUACSWAP_PROGRAM_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("FRwL3qJHuT5iYTNopCyhU1tyDn1HwtHbSa1ieYF6q2m9");
pub const GUACSWAP_REFERRER: Pubkey =
    anchor_lang::pubkey!("BUX7s2ef2htTGb2KKoPHWkmzxPj4nTWMWRgs5CSbQxf9");
const GUACSWAP_POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const GUACSWAP_STATE_DISCRIMINATOR: [u8; 8] = [216, 146, 107, 94, 104, 75, 182, 177];
const GUACSWAP_POOL_SEED: &[u8] = b"guacswappoolv1";
const GUACSWAP_STATE_SEED: &[u8] = b"guacswapstatev1";
const GUACSWAP_POOL_LEN: usize = 417;
const GUACSWAP_STATE_LEN: usize = 74;
const GUACSWAP_FEE_SCALE: u128 = 1_000_000_000_000;

#[derive(Clone)]
pub struct GuacswapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub state: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub token_x: &'info AccountInfo<'info>,
    pub token_y: &'info AccountInfo<'info>,
    pub pool_x_account: &'info AccountInfo<'info>,
    pub pool_y_account: &'info AccountInfo<'info>,
    pub swapper_x_account: &'info AccountInfo<'info>,
    pub swapper_y_account: &'info AccountInfo<'info>,
    pub swapper: &'info AccountInfo<'info>,
    pub referrer_x_account: &'info AccountInfo<'info>,
    pub referrer_y_account: &'info AccountInfo<'info>,
    pub referrer: &'info AccountInfo<'info>,
    pub program_authority: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub rent: &'info AccountInfo<'info>,
}

pub fn guacswap<'info>(
    accounts: GuacswapAccounts<'info>,
    amount_in: u64,
    x_to_y: bool,
) -> Result<SwapResult> {
    let output_token_account = if x_to_y {
        accounts.swapper_y_account
    } else {
        accounts.swapper_x_account
    };
    let pre_out = read_token_amount(output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.state.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.token_x.key(), false),
        AccountMeta::new_readonly(accounts.token_y.key(), false),
        AccountMeta::new(accounts.pool_x_account.key(), false),
        AccountMeta::new(accounts.pool_y_account.key(), false),
        AccountMeta::new(accounts.swapper_x_account.key(), false),
        AccountMeta::new(accounts.swapper_y_account.key(), false),
        AccountMeta::new(accounts.swapper.key(), true),
        AccountMeta::new(accounts.referrer_x_account.key(), false),
        AccountMeta::new(accounts.referrer_y_account.key(), false),
        AccountMeta::new(accounts.referrer.key(), false),
        AccountMeta::new_readonly(accounts.program_authority.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(accounts.rent.key(), false),
    ];
    let account_infos = vec![
        accounts.state.clone(),
        accounts.pool.clone(),
        accounts.token_x.clone(),
        accounts.token_y.clone(),
        accounts.pool_x_account.clone(),
        accounts.pool_y_account.clone(),
        accounts.swapper_x_account.clone(),
        accounts.swapper_y_account.clone(),
        accounts.swapper.clone(),
        accounts.referrer_x_account.clone(),
        accounts.referrer_y_account.clone(),
        accounts.referrer.clone(),
        accounts.program_authority.clone(),
        accounts.system_program.clone(),
        accounts.token_program.clone(),
        accounts.associated_token_program.clone(),
        accounts.rent.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: guacswap_data(amount_in, x_to_y),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn guacswap_data(amount_in: u64, x_to_y: bool) -> Vec<u8> {
    let price_limit = if x_to_y { 0 } else { u128::MAX };
    let mut data = Vec::with_capacity(33);
    data.extend_from_slice(&GUACSWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&price_limit.to_le_bytes());
    data.push(u8::from(x_to_y));
    data
}

pub fn validate_guacswap_semantic_accounts(
    step_accounts: &[AccountInfo<'_>],
    direction: u8,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step_accounts.len() == GUACSWAP_MIN_ACCOUNTS && direction <= 1,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step_accounts[0].key(),
        GUACSWAP_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step_accounts[1].key(),
        GUACSWAP_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step_accounts[1].owner,
        GUACSWAP_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let (expected_state, state_bump) =
        Pubkey::find_program_address(&[GUACSWAP_STATE_SEED], &GUACSWAP_PROGRAM_ID);
    require_keys_eq!(
        expected_state,
        GUACSWAP_STATE,
        ArbitrageError::InvalidAccount
    );
    let state_data = step_accounts[1].try_borrow_data()?;
    require!(
        state_data.len() == GUACSWAP_STATE_LEN
            && state_data.starts_with(&GUACSWAP_STATE_DISCRIMINATOR)
            && state_data[72] == state_bump,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&state_data, 40)?,
        GUACSWAP_PROGRAM_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    drop(state_data);

    require_keys_eq!(
        *step_accounts[2].owner,
        GUACSWAP_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let pool_data = step_accounts[2].try_borrow_data()?;
    require!(
        pool_data.len() == GUACSWAP_POOL_LEN && pool_data.starts_with(&GUACSWAP_POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let token_x = read_pubkey(&pool_data, 8)?;
    let token_y = read_pubkey(&pool_data, 40)?;
    require!(token_x != token_y, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        read_pubkey(&pool_data, 72)?,
        step_accounts[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&pool_data, 104)?,
        step_accounts[4].key(),
        ArbitrageError::InvalidAccount
    );
    let reserve_x = read_u64(&pool_data, 200)?;
    let reserve_y = read_u64(&pool_data, 208)?;
    let invariant = read_u128(&pool_data, 312)?;
    let price = read_u128(&pool_data, 328)?;
    let mut total_fee = 0_u128;
    for offset in [344, 360, 376, 392] {
        total_fee = total_fee
            .checked_add(read_u128(&pool_data, offset)?)
            .ok_or(ArbitrageError::MathOverflow)?;
    }
    let expected_invariant = u128::from(reserve_x)
        .checked_mul(u128::from(reserve_y))
        .ok_or(ArbitrageError::MathOverflow)?;
    let expected_price = u128::from(reserve_y)
        .checked_mul(GUACSWAP_FEE_SCALE)
        .ok_or(ArbitrageError::MathOverflow)?
        .checked_div(u128::from(reserve_x))
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        reserve_x > 0
            && reserve_y > 0
            && invariant == expected_invariant
            && price == expected_price
            && total_fee < GUACSWAP_FEE_SCALE,
        ArbitrageError::InvalidAccount
    );
    let (expected_pool, pool_bump) = Pubkey::find_program_address(
        &[GUACSWAP_POOL_SEED, token_x.as_ref(), token_y.as_ref()],
        &GUACSWAP_PROGRAM_ID,
    );
    require_keys_eq!(
        expected_pool,
        step_accounts[2].key(),
        ArbitrageError::InvalidAccount
    );
    require!(pool_data[416] == pool_bump, ArbitrageError::InvalidAccount);
    drop(pool_data);

    let (expected_input, expected_output) = if direction == 0 {
        (token_x, token_y)
    } else {
        (token_y, token_x)
    };
    require_keys_eq!(
        expected_input,
        input_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        expected_output,
        output_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        step_accounts[7].key(),
        GUACSWAP_REFERRER,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step_accounts[8].key(),
        GUACSWAP_PROGRAM_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step_accounts[5].key(),
        associated_token_address(&GUACSWAP_REFERRER, &token_x),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step_accounts[6].key(),
        associated_token_address(&GUACSWAP_REFERRER, &token_y),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step_accounts[9].key(),
        anchor_lang::solana_program::sysvar::rent::ID,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            anchor_spl::token::ID.as_ref(),
            mint.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(bytes);
    Ok(Pubkey::new_from_array(raw))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 8];
    raw.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(raw))
}

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let bytes = data
        .get(offset..offset.checked_add(16).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 16];
    raw.copy_from_slice(bytes);
    Ok(u128::from_le_bytes(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn step() -> (Vec<AccountInfo<'static>>, Pubkey, Pubkey) {
        let token_x = Pubkey::new_unique();
        let token_y = Pubkey::new_unique();
        let pool_x = Pubkey::new_unique();
        let pool_y = Pubkey::new_unique();
        let (pool, pool_bump) = Pubkey::find_program_address(
            &[GUACSWAP_POOL_SEED, token_x.as_ref(), token_y.as_ref()],
            &GUACSWAP_PROGRAM_ID,
        );
        let (_, state_bump) =
            Pubkey::find_program_address(&[GUACSWAP_STATE_SEED], &GUACSWAP_PROGRAM_ID);
        let mut state_data = vec![0_u8; GUACSWAP_STATE_LEN];
        state_data[..8].copy_from_slice(&GUACSWAP_STATE_DISCRIMINATOR);
        write_pubkey(&mut state_data, 40, GUACSWAP_PROGRAM_AUTHORITY);
        state_data[72] = state_bump;

        let reserve_x = 244_585_232_089_u64;
        let reserve_y = 18_931_145_372_u64;
        let mut pool_data = vec![0_u8; GUACSWAP_POOL_LEN];
        pool_data[..8].copy_from_slice(&GUACSWAP_POOL_DISCRIMINATOR);
        write_pubkey(&mut pool_data, 8, token_x);
        write_pubkey(&mut pool_data, 40, token_y);
        write_pubkey(&mut pool_data, 72, pool_x);
        write_pubkey(&mut pool_data, 104, pool_y);
        pool_data[200..208].copy_from_slice(&reserve_x.to_le_bytes());
        pool_data[208..216].copy_from_slice(&reserve_y.to_le_bytes());
        pool_data[312..328]
            .copy_from_slice(&(u128::from(reserve_x) * u128::from(reserve_y)).to_le_bytes());
        pool_data[328..344].copy_from_slice(
            &(u128::from(reserve_y) * GUACSWAP_FEE_SCALE / u128::from(reserve_x)).to_le_bytes(),
        );
        pool_data[344..360].copy_from_slice(&2_500_000_000_u128.to_le_bytes());
        pool_data[360..376].copy_from_slice(&250_000_000_u128.to_le_bytes());
        pool_data[392..408].copy_from_slice(&250_000_000_u128.to_le_bytes());
        pool_data[416] = pool_bump;

        let referrer_x = associated_token_address(&GUACSWAP_REFERRER, &token_x);
        let referrer_y = associated_token_address(&GUACSWAP_REFERRER, &token_y);
        (
            vec![
                account(GUACSWAP_PROGRAM_ID, Pubkey::default(), vec![], false, true),
                account(
                    GUACSWAP_STATE,
                    GUACSWAP_PROGRAM_ID,
                    state_data,
                    false,
                    false,
                ),
                account(pool, GUACSWAP_PROGRAM_ID, pool_data, true, false),
                account(pool_x, anchor_spl::token::ID, vec![], true, false),
                account(pool_y, anchor_spl::token::ID, vec![], true, false),
                account(referrer_x, anchor_spl::token::ID, vec![], true, false),
                account(referrer_y, anchor_spl::token::ID, vec![], true, false),
                account(GUACSWAP_REFERRER, Pubkey::default(), vec![], true, false),
                account(
                    GUACSWAP_PROGRAM_AUTHORITY,
                    Pubkey::default(),
                    vec![],
                    false,
                    false,
                ),
                account(
                    anchor_lang::solana_program::sysvar::rent::ID,
                    Pubkey::default(),
                    vec![],
                    false,
                    false,
                ),
            ],
            token_x,
            token_y,
        )
    }

    #[test]
    fn swap_data_matches_official_borsh_layout() {
        let x_to_y = guacswap_data(1_000_000, true);
        assert_eq!(x_to_y.len(), 33);
        assert_eq!(&x_to_y[..8], &GUACSWAP_DISCRIMINATOR);
        assert_eq!(&x_to_y[8..16], &1_000_000_u64.to_le_bytes());
        assert_eq!(&x_to_y[16..32], &0_u128.to_le_bytes());
        assert_eq!(x_to_y[32], 1);

        let y_to_x = guacswap_data(2_000_000, false);
        assert_eq!(&y_to_x[16..32], &u128::MAX.to_le_bytes());
        assert_eq!(y_to_x[32], 0);
    }

    #[test]
    fn semantic_validation_accepts_both_directions_and_rejects_fee_drift() {
        let (accounts, token_x, token_y) = step();
        let mint_x = account(token_x, anchor_spl::token::ID, vec![], false, false);
        let mint_y = account(token_y, anchor_spl::token::ID, vec![], false, false);
        assert!(validate_guacswap_semantic_accounts(&accounts, 0, &mint_x, &mint_y).is_ok());
        assert!(validate_guacswap_semantic_accounts(&accounts, 1, &mint_y, &mint_x).is_ok());
        assert!(validate_guacswap_semantic_accounts(&accounts, 0, &mint_y, &mint_x).is_err());

        let (drifted, token_x, token_y) = step();
        {
            let mut data = drifted[2].try_borrow_mut_data().expect("pool data");
            data[344..360].copy_from_slice(&GUACSWAP_FEE_SCALE.to_le_bytes());
        }
        let mint_x = account(token_x, anchor_spl::token::ID, vec![], false, false);
        let mint_y = account(token_y, anchor_spl::token::ID, vec![], false, false);
        assert!(validate_guacswap_semantic_accounts(&drifted, 0, &mint_x, &mint_y).is_err());
    }
}
