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

pub const BISONFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("BiSoNHVpsVZW2F7rx2eQ59yQwKxzU5NvBcmKshCSUypi");
pub const BISONFI_STEP_ACCOUNTS: usize = 9;
const MARKET_LEN: usize = 2_048;
const MARKET_DISCRIMINATOR: [u8; 8] = *b"POOLSTAT";
const MARKET_VERSION: u64 = 2;
const MAX_PRICE_AGE_SLOTS: u64 = 300;

pub struct BisonFiAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub base_token_program: &'info AccountInfo<'info>,
    pub quote_token_program: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
}

pub fn bisonfi_swap<'info>(
    accounts: BisonFiAccounts<'info>,
    direction: u8,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let output = if direction == 0 {
        accounts.user_quote
    } else {
        accounts.user_base
    };
    let pre_out = read_token_amount(output)?;
    let metas = vec![
        AccountMeta::new(accounts.user.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new_readonly(accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
    ];
    let infos = vec![
        accounts.user.clone(),
        accounts.market.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: bisonfi_data(direction, amount_in, min_amount_out)?,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn bisonfi_data(direction: u8, amount_in: u64, min_amount_out: u64) -> Result<Vec<u8>> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let mut data = Vec::with_capacity(18);
    data.push(2);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.push(direction);
    Ok(data)
}

pub fn validate_bisonfi_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    validate_bisonfi_semantic_accounts_at_slot(
        step,
        direction,
        fee_rate,
        input_mint,
        output_mint,
        Clock::get()?.slot,
    )
}

fn validate_bisonfi_semantic_accounts_at_slot(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
    current_slot: u64,
) -> Result<()> {
    require!(
        step.len() == BISONFI_STEP_ACCOUNTS && direction <= 1 && fee_rate == 0,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step[0].key(),
        BISONFI_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        BISONFI_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[7].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[8].key(),
        anchor_lang::solana_program::sysvar::instructions::ID,
        ArbitrageError::InvalidAccount
    );

    let market = step[1].try_borrow_data()?;
    require!(
        market.len() == MARKET_LEN
            && market.starts_with(&MARKET_DISCRIMINATOR)
            && read_u64(&market, 8)? == MARKET_VERSION,
        ArbitrageError::InvalidAccount
    );
    let state_slot = read_u64(&market, 72)?;
    let price_slot = read_u64(&market, 80)?;
    let mirrored_base_amount = read_u64(&market, 48)?;
    let mirrored_quote_amount = read_u64(&market, 56)?;
    require!(
        mirrored_base_amount > 0
            && mirrored_quote_amount > 0
            && read_u64(&market, 64)? > 0
            && current_slot >= state_slot
            && state_slot >= price_slot
            && current_slot - state_slot <= MAX_PRICE_AGE_SLOTS
            && state_slot - price_slot <= MAX_PRICE_AGE_SLOTS,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&market, 120)?,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&market, 152)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    let base_mint = read_pubkey(&market, 184)?;
    let quote_mint = read_pubkey(&market, 216)?;
    require!(base_mint != quote_mint, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(base_mint, step[4].key(), ArbitrageError::InvalidTokenMint);
    require_keys_eq!(quote_mint, step[5].key(), ArbitrageError::InvalidTokenMint);
    let target = read_u32(&market, 416)?;
    let current = read_u32(&market, 420)?;
    require!(
        read_u32(&market, 292)? > 0
            && read_u32(&market, 352)? > 0
            && read_u32(&market, 464)? > 0
            && read_u16(&market, 480)? < 10_000
            && read_u16(&market, 482)? < 10_000
            && target > 0
            && (current == target || current.checked_add(1) == Some(target))
            && market[484] <= 9
            && market[485] <= 9,
        ArbitrageError::InvalidAccount
    );
    drop(market);

    let (expected_input, expected_output) = if direction == 0 {
        (base_mint, quote_mint)
    } else {
        (quote_mint, base_mint)
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
    validate_token_account_for_mint_and_authority(&step[2], &step[4], &step[6], &step[1])?;
    validate_token_account_for_mint_and_authority(&step[3], &step[5], &step[7], &step[1])?;
    require!(
        read_token_amount(&step[2])? == mirrored_base_amount
            && read_token_amount(&step[3])? == mirrored_quote_amount,
        ArbitrageError::InvalidAccount
    );
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
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset.checked_add(4).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset.checked_add(2).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u16::from_le_bytes(
        bytes
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
        let mut data = vec![0_u8; 165];
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
    ) {
        let market = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let quote_mint = Pubkey::new_unique();
        let base_vault = Pubkey::new_unique();
        let quote_vault = Pubkey::new_unique();
        let token_program = anchor_spl::token::ID;
        let mut market_data = vec![0_u8; MARKET_LEN];
        market_data[..8].copy_from_slice(&MARKET_DISCRIMINATOR);
        market_data[8..16].copy_from_slice(&MARKET_VERSION.to_le_bytes());
        market_data[48..56].copy_from_slice(&13_761_019_322_325_u64.to_le_bytes());
        market_data[56..64].copy_from_slice(&3_136_999_575_420_u64.to_le_bytes());
        market_data[64..72].copy_from_slice(&23_241_926_162_404_064_u64.to_le_bytes());
        market_data[72..80].copy_from_slice(&401_101_387_u64.to_le_bytes());
        market_data[80..88].copy_from_slice(&401_101_250_u64.to_le_bytes());
        write_pubkey(&mut market_data, 120, base_vault);
        write_pubkey(&mut market_data, 152, quote_vault);
        write_pubkey(&mut market_data, 184, base_mint);
        write_pubkey(&mut market_data, 216, quote_mint);
        market_data[292..296].copy_from_slice(&2_000_u32.to_le_bytes());
        market_data[352..356].copy_from_slice(&2_000_u32.to_le_bytes());
        market_data[416..420].copy_from_slice(&500_u32.to_le_bytes());
        market_data[420..424].copy_from_slice(&499_u32.to_le_bytes());
        market_data[464..468].copy_from_slice(&1_000_u32.to_le_bytes());
        market_data[482..484].copy_from_slice(&1_u16.to_le_bytes());
        market_data[484] = 7;
        market_data[485] = 8;
        let step = vec![
            account(BISONFI_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(market, BISONFI_PROGRAM_ID, market_data, true, false),
            account(
                base_vault,
                token_program,
                token_data(base_mint, market, 13_761_019_322_325),
                true,
                false,
            ),
            account(
                quote_vault,
                token_program,
                token_data(quote_mint, market, 3_136_999_575_420),
                true,
                false,
            ),
            account(base_mint, token_program, vec![], false, false),
            account(quote_mint, token_program, vec![], false, false),
            account(token_program, Pubkey::default(), vec![], false, true),
            account(token_program, Pubkey::default(), vec![], false, true),
            account(
                anchor_lang::solana_program::sysvar::instructions::ID,
                anchor_lang::solana_program::sysvar::ID,
                vec![],
                false,
                false,
            ),
        ];
        (
            step,
            account(base_mint, token_program, vec![], false, false),
            account(quote_mint, token_program, vec![], false, false),
        )
    }

    #[test]
    fn bisonfi_wire_matches_real_program_layout() {
        let data = bisonfi_data(1, 100_000_000, 1_200_000_000).expect("wire");
        assert_eq!(data.len(), 18);
        assert_eq!(data[0], 2);
        assert_eq!(
            u64::from_le_bytes(data[1..9].try_into().unwrap()),
            100_000_000
        );
        assert_eq!(
            u64::from_le_bytes(data[9..17].try_into().unwrap()),
            1_200_000_000
        );
        assert_eq!(data[17], 1);
    }

    #[test]
    fn semantic_validation_binds_state_vaults_sysvar_and_both_directions() {
        let (step, base_mint, quote_mint) = semantic_fixture();
        assert!(validate_bisonfi_semantic_accounts_at_slot(
            &step,
            0,
            0,
            &base_mint,
            &quote_mint,
            401_101_387,
        )
        .is_ok());
        assert!(validate_bisonfi_semantic_accounts_at_slot(
            &step,
            1,
            0,
            &quote_mint,
            &base_mint,
            401_101_387,
        )
        .is_ok());
        assert!(validate_bisonfi_semantic_accounts_at_slot(
            &step,
            0,
            0,
            &base_mint,
            &quote_mint,
            401_101_688,
        )
        .is_err());
    }
}
