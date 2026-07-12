use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const SERUM_V3_STEP_ACCOUNTS: usize = 10;
pub const SERUM_V3_MARKET_LEN: usize = 388;
pub const SERUM_V3_SEND_TAKE_LEN: usize = 51;
const SEND_TAKE_TAG: u32 = 13;
const MATCH_LIMIT: u16 = u16::MAX;

#[derive(Clone)]
pub struct SerumV3Accounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub request_queue: &'info AccountInfo<'info>,
    pub event_queue: &'info AccountInfo<'info>,
    pub bids: &'info AccountInfo<'info>,
    pub asks: &'info AccountInfo<'info>,
    pub trader_base: &'info AccountInfo<'info>,
    pub trader_quote: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub vault_signer: &'info AccountInfo<'info>,
}

pub fn serum_v3_swap<'info>(
    accounts: SerumV3Accounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_in: bool,
    base_lot_size: u64,
    quote_lot_size: u64,
) -> Result<SwapResult> {
    let output = if is_base_in {
        accounts.trader_quote
    } else {
        accounts.trader_base
    };
    let pre_out = read_token_amount(output)?;
    let data = send_take_data(
        amount_in,
        minimum_amount_out,
        is_base_in,
        base_lot_size,
        quote_lot_size,
    )?;
    let metas = vec![
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.request_queue.key(), false),
        AccountMeta::new(accounts.event_queue.key(), false),
        AccountMeta::new(accounts.bids.key(), false),
        AccountMeta::new(accounts.asks.key(), false),
        AccountMeta::new(accounts.trader_base.key(), false),
        AccountMeta::new(accounts.trader_quote.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.vault_signer.key(), false),
    ];
    let infos = vec![
        accounts.market.clone(),
        accounts.request_queue.clone(),
        accounts.event_queue.clone(),
        accounts.bids.clone(),
        accounts.asks.clone(),
        accounts.trader_base.clone(),
        accounts.trader_quote.clone(),
        accounts.payer.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.token_program.clone(),
        accounts.vault_signer.clone(),
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
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_serum_v3_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<(u64, u64)> {
    require!(
        step.len() == SERUM_V3_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let program = &step[0];
    let market = &step[1];
    require_keys_eq!(*market.owner, program.key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[9].key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    for account in &step[2..=5] {
        require_keys_eq!(
            *account.owner,
            program.key(),
            ArbitrageError::InvalidAccount
        );
    }

    let data = market.try_borrow_data()?;
    require!(
        data.len() == SERUM_V3_MARKET_LEN
            && data.starts_with(b"serum")
            && data.ends_with(b"padding"),
        ArbitrageError::InvalidAccount
    );
    let body = &data[5..data.len() - 7];
    require!(read_u64(body, 0)? == 3, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(body, 8)?,
        market.key(),
        ArbitrageError::InvalidAccount
    );
    let nonce = read_u64(body, 40)?;
    let base_mint = read_pubkey(body, 48)?;
    let quote_mint = read_pubkey(body, 80)?;
    require_keys_eq!(
        read_pubkey(body, 112)?,
        step[6].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(body, 160)?,
        step[7].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(body, 216)?,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(body, 248)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(body, 280)?,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(body, 312)?,
        step[5].key(),
        ArbitrageError::InvalidAccount
    );
    let base_lot_size = read_u64(body, 344)?;
    let quote_lot_size = read_u64(body, 352)?;
    require!(
        base_lot_size > 0 && quote_lot_size > 0,
        ArbitrageError::InvalidAmount
    );
    let nonce_bytes = nonce.to_le_bytes();
    let expected_vault_signer = Pubkey::create_program_address(
        &[market.key().as_ref(), nonce_bytes.as_ref()],
        &program.key(),
    )
    .map_err(|_| ArbitrageError::InvalidAccount)?;
    require_keys_eq!(
        step[8].key(),
        expected_vault_signer,
        ArbitrageError::InvalidAccount
    );
    validate_vault(&step[6], base_mint, expected_vault_signer, token_program)?;
    validate_vault(&step[7], quote_mint, expected_vault_signer, token_program)?;
    let (expected_in, expected_out) = if direction == 0 {
        (base_mint, quote_mint)
    } else {
        (quote_mint, base_mint)
    };
    require_keys_eq!(in_mint.key(), expected_in, ArbitrageError::InvalidAccount);
    require_keys_eq!(out_mint.key(), expected_out, ArbitrageError::InvalidAccount);
    validate_padded_flags(&step[2], 9)?;
    validate_padded_flags(&step[3], 17)?;
    validate_padded_flags(&step[4], 33)?;
    validate_padded_flags(&step[5], 65)?;
    Ok((base_lot_size, quote_lot_size))
}

fn validate_vault<'info>(
    vault: &AccountInfo<'info>,
    mint: Pubkey,
    authority: Pubkey,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *vault.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = vault.try_borrow_data()?;
    require!(data.len() >= 72, ArbitrageError::InvalidAccount);
    require_keys_eq!(read_pubkey(&data, 0)?, mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&data, 32)?,
        authority,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_padded_flags(account: &AccountInfo<'_>, flags: u64) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.len() >= 20 && data.starts_with(b"serum") && data.ends_with(b"padding"),
        ArbitrageError::InvalidAccount
    );
    require!(read_u64(&data, 5)? == flags, ArbitrageError::InvalidAccount);
    Ok(())
}

fn send_take_data(
    amount_in: u64,
    minimum_out: u64,
    base_in: bool,
    base_lot: u64,
    quote_lot: u64,
) -> Result<Vec<u8>> {
    require!(
        amount_in > 0 && base_lot > 0 && quote_lot > 0,
        ArbitrageError::InvalidAmount
    );
    let (side, price, max_coin, max_pc, min_coin, min_pc) = if base_in {
        (
            1_u32,
            1_u64,
            floor_nonzero_lots(amount_in, base_lot)?,
            amount_in,
            0,
            minimum_out,
        )
    } else {
        (
            0_u32,
            u64::MAX,
            floor_nonzero_lots(amount_in, quote_lot)?,
            amount_in,
            ceil_lots(minimum_out, base_lot)?,
            0,
        )
    };
    let mut data = Vec::with_capacity(SERUM_V3_SEND_TAKE_LEN);
    data.push(0);
    data.extend_from_slice(&SEND_TAKE_TAG.to_le_bytes());
    data.extend_from_slice(&side.to_le_bytes());
    for value in [price, max_coin, max_pc, min_coin, min_pc] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(&MATCH_LIMIT.to_le_bytes());
    require!(
        data.len() == SERUM_V3_SEND_TAKE_LEN,
        ArbitrageError::InvalidInstructionData
    );
    Ok(data)
}

fn floor_nonzero_lots(amount: u64, lot: u64) -> Result<u64> {
    let value = amount
        .checked_div(lot)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(value > 0, ArbitrageError::InvalidAmount);
    Ok(value)
}

fn ceil_lots(amount: u64, lot: u64) -> Result<u64> {
    if amount == 0 {
        return Ok(0);
    }
    amount
        .checked_add(lot - 1)
        .and_then(|value| value.checked_div(lot))
        .ok_or_else(|| ArbitrageError::MathOverflow.into())
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    Ok(Pubkey::new_from_array(
        data.get(offset..offset + 32)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(
        key: Pubkey,
        owner: Pubkey,
        writable: bool,
        executable: bool,
        data: Vec<u8>,
    ) -> AccountInfo<'static> {
        AccountInfo::new(
            Box::leak(Box::new(key)),
            false,
            writable,
            Box::leak(Box::new(0_u64)),
            Box::leak(data.into_boxed_slice()),
            Box::leak(Box::new(owner)),
            executable,
            0,
        )
    }

    fn padded_flags(flags: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 64];
        data[..5].copy_from_slice(b"serum");
        data[5..13].copy_from_slice(&flags.to_le_bytes());
        data[57..].copy_from_slice(b"padding");
        data
    }

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + 32].copy_from_slice(key.as_ref());
    }

    #[test]
    fn send_take_data_matches_official_bincode_layout() {
        let sell = send_take_data(300, 123, true, 100, 10).expect("sell");
        assert_eq!(sell.len(), 51);
        assert_eq!(&sell[..9], &[0, 13, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(&sell[9..17], &1_u64.to_le_bytes());
        assert_eq!(&sell[17..25], &3_u64.to_le_bytes());
        assert_eq!(&sell[41..49], &123_u64.to_le_bytes());

        let buy = send_take_data(1_000, 201, false, 100, 10).expect("buy");
        assert_eq!(u32::from_le_bytes(buy[5..9].try_into().unwrap()), 0);
        assert_eq!(u64::from_le_bytes(buy[9..17].try_into().unwrap()), u64::MAX);
        assert_eq!(u64::from_le_bytes(buy[17..25].try_into().unwrap()), 100);
        assert_eq!(u64::from_le_bytes(buy[33..41].try_into().unwrap()), 3);
    }

    #[test]
    fn semantic_validation_binds_market_books_vaults_and_direction() {
        let program = Pubkey::new_unique();
        let market = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let base_vault = Pubkey::new_unique();
        let quote_vault = Pubkey::new_unique();
        let request = Pubkey::new_unique();
        let event = Pubkey::new_unique();
        let bids = Pubkey::new_unique();
        let asks = Pubkey::new_unique();
        let token = anchor_spl::token::ID;
        let mut nonce = 0_u64;
        let vault_signer = loop {
            if let Ok(key) = Pubkey::create_program_address(
                &[market.as_ref(), nonce.to_le_bytes().as_ref()],
                &program,
            ) {
                break key;
            }
            nonce += 1;
        };
        let mut market_data = vec![0_u8; SERUM_V3_MARKET_LEN];
        market_data[..5].copy_from_slice(b"serum");
        market_data[SERUM_V3_MARKET_LEN - 7..].copy_from_slice(b"padding");
        let body = &mut market_data[5..SERUM_V3_MARKET_LEN - 7];
        body[0..8].copy_from_slice(&3_u64.to_le_bytes());
        write_pubkey(body, 8, market);
        body[40..48].copy_from_slice(&nonce.to_le_bytes());
        for (offset, key) in [
            (48, base_mint),
            (80, quote_mint),
            (112, base_vault),
            (160, quote_vault),
            (216, request),
            (248, event),
            (280, bids),
            (312, asks),
        ] {
            write_pubkey(body, offset, key);
        }
        body[344..352].copy_from_slice(&100_u64.to_le_bytes());
        body[352..360].copy_from_slice(&10_u64.to_le_bytes());
        let token_data = |mint: Pubkey| {
            let mut data = vec![0_u8; 72];
            write_pubkey(&mut data, 0, mint);
            write_pubkey(&mut data, 32, vault_signer);
            data
        };
        let step = vec![
            account(program, Pubkey::default(), false, true, Vec::new()),
            account(market, program, true, false, market_data),
            account(request, program, true, false, padded_flags(9)),
            account(event, program, true, false, padded_flags(17)),
            account(bids, program, true, false, padded_flags(33)),
            account(asks, program, true, false, padded_flags(65)),
            account(base_vault, token, true, false, token_data(base_mint)),
            account(quote_vault, token, true, false, token_data(quote_mint)),
            account(vault_signer, program, false, false, Vec::new()),
            account(token, Pubkey::default(), false, true, Vec::new()),
        ];
        let in_mint = account(base_mint, token, false, false, Vec::new());
        let out_mint = account(quote_mint, token, false, false, Vec::new());
        let token_program = account(token, Pubkey::default(), false, true, Vec::new());
        assert_eq!(
            validate_serum_v3_semantic_accounts(&step, 0, &in_mint, &out_mint, &token_program,)
                .expect("semantic accounts"),
            (100, 10)
        );
        assert!(
            validate_serum_v3_semantic_accounts(&step, 1, &in_mint, &out_mint, &token_program,)
                .is_err()
        );
    }
}
