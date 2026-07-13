use crate::{
    errors::ArbitrageError,
    instructions::types::{
        read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
        SwapResult,
    },
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const LEMMINGSFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("BQEJZUB4CzoT6UhRffoCkqCyqQNrCPCSGHcPEmsdbEsX");
pub const LEMMINGSFI_STEP_ACCOUNTS: usize = 8;
const MARKET_LEN: usize = 310;
const CONFIG_LEN: usize = 76;
const MARKET_DISCRIMINATOR: [u8; 8] = [219, 190, 213, 55, 0, 227, 198, 154];
const CONFIG_DISCRIMINATOR: [u8; 8] = [149, 8, 156, 202, 160, 252, 176, 217];
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

#[derive(Clone)]
pub struct LemmingsFiAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub global_config: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub vault_base: &'info AccountInfo<'info>,
    pub vault_quote: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn lemmingsfi_swap<'info>(
    accounts: LemmingsFiAccounts<'info>,
    direction: u8,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    let output = if direction == 0 {
        accounts.user_base
    } else {
        accounts.user_quote
    };
    let pre_out = read_token_amount(output)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.user.key(), true),
        AccountMeta::new_readonly(accounts.global_config.key(), false),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.vault_base.key(), false),
        AccountMeta::new(accounts.vault_quote.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let account_infos = vec![
        accounts.user.clone(),
        accounts.global_config.clone(),
        accounts.market.clone(),
        accounts.vault_base.clone(),
        accounts.vault_quote.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: lemmingsfi_data(direction, amount_in, min_amount_out),
        },
        &account_infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn lemmingsfi_data(direction: u8, amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(&SWAP_DISCRIMINATOR);
    data.push(direction);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

pub fn validate_lemmingsfi_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step.len() == LEMMINGSFI_STEP_ACCOUNTS && direction <= 1,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step[0].key(),
        LEMMINGSFI_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[7].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );

    let (expected_config, config_bump) =
        Pubkey::find_program_address(&[b"config"], &LEMMINGSFI_PROGRAM_ID);
    require_keys_eq!(
        step[1].key(),
        expected_config,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        LEMMINGSFI_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let config_data = step[1].try_borrow_data()?;
    require!(
        config_data.len() == CONFIG_LEN
            && config_data.starts_with(&CONFIG_DISCRIMINATOR)
            && config_data[74] == 0
            && config_data[75] == config_bump,
        ArbitrageError::InvalidAccount
    );
    drop(config_data);

    require_keys_eq!(
        *step[2].owner,
        LEMMINGSFI_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let market_data = step[2].try_borrow_data()?;
    require!(
        market_data.len() == MARKET_LEN
            && market_data.starts_with(&MARKET_DISCRIMINATOR)
            && market_data[240] == 0
            && market_data[290..].iter().all(|byte| *byte == 0),
        ArbitrageError::InvalidAccount
    );
    let base_mint = read_pubkey(&market_data, 8)?;
    let quote_mint = read_pubkey(&market_data, 40)?;
    require!(base_mint != quote_mint, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(base_mint, step[5].key(), ArbitrageError::InvalidTokenMint);
    require_keys_eq!(quote_mint, step[6].key(), ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        read_pubkey(&market_data, 72)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&market_data, 104)?,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    let (expected_market, market_bump) = Pubkey::find_program_address(
        &[b"market", base_mint.as_ref(), quote_mint.as_ref()],
        &LEMMINGSFI_PROGRAM_ID,
    );
    require_keys_eq!(
        expected_market,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require!(
        market_data[241] == market_bump,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        Pubkey::find_program_address(
            &[b"vault_base", step[2].key().as_ref()],
            &LEMMINGSFI_PROGRAM_ID,
        )
        .0,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        Pubkey::find_program_address(
            &[b"vault_quote", step[2].key().as_ref()],
            &LEMMINGSFI_PROGRAM_ID,
        )
        .0,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    let oracle_price = read_u64(&market_data, 168)?;
    let oracle_slot = read_u64(&market_data, 192)?;
    let bid_spread = read_u16(&market_data, 200)?;
    let ask_spread = read_u16(&market_data, 202)?;
    let fee = read_u16(&market_data, 204)?;
    let max_staleness = read_u64(&market_data, 230)?;
    let current_slot = Clock::get()?.slot;
    require!(
        oracle_price > 0
            && bid_spread < 10_000
            && ask_spread < 10_000
            && fee < 10_000
            && current_slot >= oracle_slot
            && current_slot - oracle_slot <= max_staleness,
        ArbitrageError::InvalidAccount
    );
    drop(market_data);

    let (expected_input, expected_output) = if direction == 0 {
        (quote_mint, base_mint)
    } else {
        (base_mint, quote_mint)
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
    validate_token_account_for_mint_and_authority(&step[3], &step[5], &step[7], &step[2])?;
    validate_token_account_for_mint_and_authority(&step[4], &step[6], &step[7], &step[2])?;
    Ok(())
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

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset.checked_add(2).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 2];
    raw.copy_from_slice(bytes);
    Ok(u16::from_le_bytes(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lemmingsfi_wire_data_matches_litesvm_verified_layout() {
        let data = lemmingsfi_data(1, 10_000_000, 9_900_000);
        assert_eq!(data.len(), 25);
        assert_eq!(&data[..8], &SWAP_DISCRIMINATOR);
        assert_eq!(data[8], 1);
        assert_eq!(
            u64::from_le_bytes(data[9..17].try_into().unwrap()),
            10_000_000
        );
        assert_eq!(
            u64::from_le_bytes(data[17..25].try_into().unwrap()),
            9_900_000
        );
    }
}
