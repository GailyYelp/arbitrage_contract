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
        program_ids::MARCOPOLO_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const MARCOPOLO_STEP_ACCOUNTS: usize = 15;

const STATE: Pubkey = anchor_lang::pubkey!("BE5YRQ6N6LCw7UL3JwzVp317EWa4mzJY6JKDaudcXu7A");
const PROGRAM_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("JM78XNzeQRmZXDAP4DSq88ZdErbuSXSLE6fkRsVDKSu");
const REFERRER: Pubkey = anchor_lang::pubkey!("MaPoNRu6RgbTkE978RYGuaXKobsciLLTDdgL6juqh68");
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const STATE_DISCRIMINATOR: [u8; 8] = [216, 146, 107, 94, 104, 75, 182, 177];
const POOL_LEN: usize = 417;
const STATE_LEN: usize = 74;
const STATE_AUTHORITY_OFFSET: usize = 40;
const STATE_BUMP_OFFSET: usize = 72;
const TOKEN_X_OFFSET: usize = 8;
const TOKEN_Y_OFFSET: usize = 40;
const POOL_X_OFFSET: usize = 72;
const POOL_Y_OFFSET: usize = 104;
const TOKEN_X_RESERVE_OFFSET: usize = 200;
const TOKEN_Y_RESERVE_OFFSET: usize = 208;
const CONST_K_OFFSET: usize = 312;
const PRICE_OFFSET: usize = 328;
const LP_FEE_OFFSET: usize = 344;
const BUYBACK_FEE_OFFSET: usize = 360;
const PROJECT_FEE_OFFSET: usize = 376;
const MERCANTI_FEE_OFFSET: usize = 392;
const POOL_BUMP_OFFSET: usize = 416;
const FEE_SCALE: u128 = 1_000_000_000_000;
const STATE_SEED: &[u8] = b"marcostatev1";
const POOL_SEED: &[u8] = b"marcopoolv1";

pub struct MarcoPoloAccounts<'info> {
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

pub fn marcopolo_swap<'info>(
    accounts: MarcoPoloAccounts<'info>,
    amount_in: u64,
    x_to_y: bool,
    pool_price: u128,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let output_token_account = if x_to_y {
        accounts.swapper_y_account
    } else {
        accounts.swapper_x_account
    };
    let pre_out = read_token_amount(output_token_account)?;
    let price_limit = if x_to_y {
        0
    } else {
        pool_price
            .checked_mul(2)
            .ok_or(ArbitrageError::MathOverflow)?
    };
    let mut data = Vec::with_capacity(33);
    data.extend_from_slice(&SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&price_limit.to_le_bytes());
    data.push(u8::from(x_to_y));
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
    let infos = vec![
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
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_marcopolo_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
) -> Result<u128> {
    require!(
        step.len() == MARCOPOLO_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require_keys_eq!(
        step[0].key(),
        MARCOPOLO_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(step[1].key(), STATE, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[1].owner,
        MARCOPOLO_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        MARCOPOLO_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[11].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[12].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[13].key(),
        anchor_spl::associated_token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[14].key(),
        anchor_lang::solana_program::sysvar::rent::ID,
        ArbitrageError::InvalidAccount
    );

    let state_data = step[1].try_borrow_data()?;
    require!(
        state_data.len() == STATE_LEN && state_data.starts_with(&STATE_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let (expected_state, state_bump) =
        Pubkey::find_program_address(&[STATE_SEED], &MARCOPOLO_PROGRAM_ID);
    require_keys_eq!(expected_state, STATE, ArbitrageError::InvalidAccount);
    require!(
        state_data[STATE_BUMP_OFFSET] == state_bump,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&state_data, STATE_AUTHORITY_OFFSET)?,
        PROGRAM_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[10].key(),
        PROGRAM_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    drop(state_data);

    let pool_data = step[2].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_LEN && pool_data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let token_x = read_pubkey(&pool_data, TOKEN_X_OFFSET)?;
    let token_y = read_pubkey(&pool_data, TOKEN_Y_OFFSET)?;
    let pool_x = read_pubkey(&pool_data, POOL_X_OFFSET)?;
    let pool_y = read_pubkey(&pool_data, POOL_Y_OFFSET)?;
    let reserve_x = read_u64(&pool_data, TOKEN_X_RESERVE_OFFSET)?;
    let reserve_y = read_u64(&pool_data, TOKEN_Y_RESERVE_OFFSET)?;
    let const_k = read_u128(&pool_data, CONST_K_OFFSET)?;
    let price = read_u128(&pool_data, PRICE_OFFSET)?;
    let total_fee = [
        LP_FEE_OFFSET,
        BUYBACK_FEE_OFFSET,
        PROJECT_FEE_OFFSET,
        MERCANTI_FEE_OFFSET,
    ]
    .into_iter()
    .try_fold(0_u128, |sum, offset| {
        sum.checked_add(read_u128(&pool_data, offset)?)
            .ok_or_else(|| error!(ArbitrageError::MathOverflow))
    })?;
    let invariant = u128::from(reserve_x)
        .checked_mul(u128::from(reserve_y))
        .ok_or(ArbitrageError::MathOverflow)?;
    let expected_price = u128::from(reserve_y)
        .checked_mul(FEE_SCALE)
        .and_then(|value| value.checked_div(u128::from(reserve_x)))
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        token_x != token_y
            && pool_x != pool_y
            && reserve_x > 0
            && reserve_y > 0
            && const_k == invariant
            && price == expected_price
            && total_fee < FEE_SCALE,
        ArbitrageError::InvalidAccount
    );
    let expected_fee_rate = u16::try_from(
        total_fee
            .checked_mul(10_000)
            .and_then(|value| value.checked_div(FEE_SCALE))
            .ok_or(ArbitrageError::MathOverflow)?,
    )
    .map_err(|_| ArbitrageError::MathOverflow)?;
    require!(
        fee_rate == expected_fee_rate,
        ArbitrageError::InvalidFeeAmount
    );
    let (expected_pool, pool_bump) = Pubkey::find_program_address(
        &[POOL_SEED, token_x.as_ref(), token_y.as_ref()],
        &MARCOPOLO_PROGRAM_ID,
    );
    require_keys_eq!(expected_pool, step[2].key(), ArbitrageError::InvalidAccount);
    require!(
        pool_data[POOL_BUMP_OFFSET] == pool_bump,
        ArbitrageError::InvalidAccount
    );
    drop(pool_data);

    require_keys_eq!(step[3].key(), token_x, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(step[4].key(), token_y, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(pool_x, step[5].key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(pool_y, step[6].key(), ArbitrageError::InvalidAccount);
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
    validate_token_account_for_mint_and_authority(&step[5], &step[3], &step[12], &step[10])?;
    validate_token_account_for_mint_and_authority(&step[6], &step[4], &step[12], &step[10])?;
    require_keys_eq!(step[9].key(), REFERRER, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[7].key(),
        associated_token_address(&REFERRER, &token_x),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        associated_token_address(&REFERRER, &token_y),
        ArbitrageError::InvalidAccount
    );
    Ok(price)
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

fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let bytes: [u8; 16] = data
        .get(offset..offset + 16)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u128::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_wire_matches_confirmed_jupiter_cpi() {
        let mut data = Vec::new();
        data.extend_from_slice(&SWAP_DISCRIMINATOR);
        data.extend_from_slice(&65_170_809_u64.to_le_bytes());
        data.extend_from_slice(&0_u128.to_le_bytes());
        data.push(1);
        assert_eq!(
            data,
            [
                248, 198, 158, 145, 225, 117, 135, 200, 121, 109, 226, 3, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]
        );
    }

    #[test]
    fn fixed_pdas_match_mainnet_accounts() {
        assert_eq!(
            Pubkey::find_program_address(&[STATE_SEED], &MARCOPOLO_PROGRAM_ID).0,
            STATE
        );
        let token_x = anchor_lang::pubkey!("So11111111111111111111111111111111111111112");
        let token_y = anchor_lang::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        assert_eq!(
            Pubkey::find_program_address(
                &[POOL_SEED, token_x.as_ref(), token_y.as_ref()],
                &MARCOPOLO_PROGRAM_ID,
            )
            .0,
            anchor_lang::pubkey!("8GmMLucoAARw9X6g2aUVBdzRpxF6egeHMDtLJen2Kkq7")
        );
    }
}
