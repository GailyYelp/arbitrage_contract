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
        program_ids::BONFIDA_DEX_V4_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const BONFIDA_DEX_V4_STEP_ACCOUNTS: usize = 9;
const MARKET_LEN: usize = 280;
const MARKET_TAG: u64 = 1;
const AOB_MARKET_LEN: usize = 120;
const AOB_MARKET_TAG: u64 = 128;
const EVENT_QUEUE_TAG: u64 = 129;
const BIDS_TAG: u64 = 130;
const ASKS_TAG: u64 = 131;
const EVENT_HEADER_LEN: usize = 32;
const EVENT_LEN: usize = 106;
const SLAB_HEADER_LEN: usize = 40;
const SLAB_LEAF_AND_CALLBACK_LEN: usize = 57;
const SLAB_STRIDE: usize = 89;
const SWAP_TAG: u8 = 2;
const MATCH_LIMIT: u64 = 64;

pub struct BonfidaDexV4Accounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_bonfida_dex_v4_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == BONFIDA_DEX_V4_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        BONFIDA_DEX_V4_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    for account in &step[1..=5] {
        require_keys_eq!(
            *account.owner,
            BONFIDA_DEX_V4_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
    }

    let market = step[1].try_borrow_data()?;
    require!(
        market.len() == MARKET_LEN && read_u64(&market, 0)? == MARKET_TAG,
        ArbitrageError::InvalidAccount
    );
    let base_mint = read_pubkey(&market, 8)?;
    let quote_mint = read_pubkey(&market, 40)?;
    let base_vault = read_pubkey(&market, 72)?;
    let quote_vault = read_pubkey(&market, 104)?;
    let orderbook = read_pubkey(&market, 136)?;
    let min_base_order_size = read_u64(&market, 232)?;
    let royalties_bps = read_u64(&market, 240)?;
    let base_multiplier = read_u64(&market, 256)?;
    let quote_multiplier = read_u64(&market, 264)?;
    let signer_nonce = market[272];
    let fee_type = market[273];
    require!(
        base_mint != Pubkey::default()
            && quote_mint != Pubkey::default()
            && base_mint != quote_mint
            && base_vault != quote_vault
            && min_base_order_size > 0
            && royalties_bps <= 10_000
            && base_multiplier > 0
            && quote_multiplier > 0
            && fee_type <= 1
            && market[274..280].iter().all(|byte| *byte == 0),
        ArbitrageError::InvalidAccount
    );
    require!(
        direction == 0 || royalties_bps == 0,
        ArbitrageError::InvalidFeeAmount
    );
    drop(market);
    require_keys_eq!(step[2].key(), orderbook, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[6].key(), base_vault, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[7].key(), quote_vault, ArbitrageError::InvalidAccount);

    let nonce = [signer_nonce];
    let expected_signer = Pubkey::create_program_address(
        &[step[1].key().as_ref(), nonce.as_ref()],
        &BONFIDA_DEX_V4_PROGRAM_ID,
    )
    .map_err(|_| ArbitrageError::InvalidAccount)?;
    require_keys_eq!(
        step[8].key(),
        expected_signer,
        ArbitrageError::InvalidAccount
    );

    let orderbook_data = step[2].try_borrow_data()?;
    require!(
        orderbook_data.len() == AOB_MARKET_LEN && read_u64(&orderbook_data, 0)? == AOB_MARKET_TAG,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&orderbook_data, 8)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&orderbook_data, 40)?,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&orderbook_data, 72)?,
        step[5].key(),
        ArbitrageError::InvalidAccount
    );
    let aob_min_base_order_size = read_u64(&orderbook_data, 104)?;
    let tick_size = read_u64(&orderbook_data, 112)?;
    require!(
        aob_min_base_order_size > 0
            && tick_size > 0
            && min_base_order_size / base_multiplier == aob_min_base_order_size,
        ArbitrageError::InvalidAccount
    );
    drop(orderbook_data);

    validate_event_queue(&step[3])?;
    validate_slab(&step[4], BIDS_TAG)?;
    validate_slab(&step[5], ASKS_TAG)?;

    let (base_mint_ai, quote_mint_ai) = if direction == 0 {
        (input_mint, output_mint)
    } else {
        (output_mint, input_mint)
    };
    require_keys_eq!(
        base_mint_ai.key(),
        base_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        quote_mint_ai.key(),
        quote_mint,
        ArbitrageError::InvalidTokenMint
    );
    validate_token_account_for_mint_and_authority(&step[6], base_mint_ai, token_program, &step[8])?;
    validate_token_account_for_mint_and_authority(
        &step[7],
        quote_mint_ai,
        token_program,
        &step[8],
    )?;
    require!(
        read_token_amount(&step[6])? > 0 && read_token_amount(&step[7])? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
}

pub fn bonfida_dex_v4_swap<'a, 'info>(
    accounts: BonfidaDexV4Accounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(min_amount_out > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let s = accounts.step;
    let (side, base_qty, quote_qty, user_base, user_quote) = if direction == 0 {
        (
            1,
            amount_in,
            min_amount_out,
            accounts.user_input,
            accounts.user_output,
        )
    } else {
        (
            0,
            min_amount_out,
            amount_in,
            accounts.user_output,
            accounts.user_input,
        )
    };
    let market = s[1].try_borrow_data()?;
    require!(
        base_qty >= read_u64(&market, 232)?,
        ArbitrageError::InvalidAmount
    );
    drop(market);
    let pre_out = read_token_amount(accounts.user_output)?;
    let instruction = Instruction {
        program_id: s[0].key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(accounts.system_program.key(), false),
            AccountMeta::new(s[1].key(), false),
            AccountMeta::new(s[2].key(), false),
            AccountMeta::new(s[3].key(), false),
            AccountMeta::new(s[4].key(), false),
            AccountMeta::new(s[5].key(), false),
            AccountMeta::new(s[6].key(), false),
            AccountMeta::new(s[7].key(), false),
            AccountMeta::new_readonly(s[8].key(), false),
            AccountMeta::new(user_base.key(), false),
            AccountMeta::new(user_quote.key(), false),
            AccountMeta::new(accounts.payer.key(), true),
        ],
        data: swap_data(base_qty, quote_qty, side),
    };
    let infos = vec![
        accounts.token_program.clone(),
        accounts.system_program.clone(),
        s[1].clone(),
        s[2].clone(),
        s[3].clone(),
        s[4].clone(),
        s[5].clone(),
        s[6].clone(),
        s[7].clone(),
        s[8].clone(),
        user_base.clone(),
        user_quote.clone(),
        accounts.payer.clone(),
        s[0].clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

fn swap_data(base_qty: u64, quote_qty: u64, side: u8) -> Vec<u8> {
    let mut data = vec![0_u8; 8];
    data[0] = SWAP_TAG;
    data.extend_from_slice(&base_qty.to_le_bytes());
    data.extend_from_slice(&quote_qty.to_le_bytes());
    data.extend_from_slice(&MATCH_LIMIT.to_le_bytes());
    data.push(side);
    data.push(0);
    data.extend_from_slice(&[0_u8; 6]);
    data
}

fn validate_event_queue(account: &AccountInfo<'_>) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.len() >= EVENT_HEADER_LEN
            && read_u64(&data, 0)? == EVENT_QUEUE_TAG
            && (data.len() - EVENT_HEADER_LEN).checked_rem(EVENT_LEN) == Some(0),
        ArbitrageError::InvalidAccount
    );
    let capacity = (data.len() - EVENT_HEADER_LEN) / EVENT_LEN;
    let count =
        usize::try_from(read_u64(&data, 16)?).map_err(|_| ArbitrageError::InvalidAccount)?;
    require!(
        capacity > 0 && count < capacity,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_slab(account: &AccountInfo<'_>, tag: u64) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.len() >= SLAB_HEADER_LEN + SLAB_LEAF_AND_CALLBACK_LEN
            && read_u64(&data, 0)? == tag
            && (data.len() - SLAB_HEADER_LEN - SLAB_LEAF_AND_CALLBACK_LEN).checked_rem(SLAB_STRIDE)
                == Some(0),
        ArbitrageError::InvalidAccount
    );
    let inner_capacity = (data.len() - SLAB_HEADER_LEN - SLAB_LEAF_AND_CALLBACK_LEN) / SLAB_STRIDE;
    let leaf_capacity = inner_capacity + 1;
    let leaf_bump =
        usize::try_from(read_u32(&data, 16)?).map_err(|_| ArbitrageError::InvalidAccount)?;
    let inner_bump =
        usize::try_from(read_u32(&data, 28)?).map_err(|_| ArbitrageError::InvalidAccount)?;
    let leaf_count =
        usize::try_from(read_u32(&data, 36)?).map_err(|_| ArbitrageError::InvalidAccount)?;
    require!(
        leaf_bump <= leaf_capacity && inner_bump <= inner_capacity && leaf_count <= leaf_bump,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
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

    #[test]
    fn swap_wire_layout_matches_native_program() {
        let data = swap_data(11, 22, 1);
        assert_eq!(data.len(), 40);
        assert_eq!(data[0], SWAP_TAG);
        assert_eq!(u64::from_le_bytes(data[8..16].try_into().unwrap()), 11);
        assert_eq!(u64::from_le_bytes(data[16..24].try_into().unwrap()), 22);
        assert_eq!(
            u64::from_le_bytes(data[24..32].try_into().unwrap()),
            MATCH_LIMIT
        );
        assert_eq!(data[32], 1);
        assert!(data[33..].iter().all(|byte| *byte == 0));
    }
}
