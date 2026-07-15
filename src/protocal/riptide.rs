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
        program_ids::RIPTIDE_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const RIPTIDE_STEP_ACCOUNTS: usize = 6;
pub const RIPTIDE_MEMO_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
pub const RIPTIDE_INSTRUCTIONS_SYSVAR_ID: Pubkey =
    anchor_lang::pubkey!("Sysvar1nstructions1111111111111111111111111");

const RIPTIDE_MARKET_ACCOUNT_LEN: usize = 1_024;
const RIPTIDE_MARKET_DISCRIMINATOR: [u8; 8] = [0x02, 0xfe, 0x00, 0x64, 0xd7, 0x42, 0x4c, 0xf4];
const RIPTIDE_AUTHORITY_OFFSET: usize = 8;
const RIPTIDE_ADMIN_OFFSET: usize = 40;
const RIPTIDE_BASE_MINT_OFFSET: usize = 72;
const RIPTIDE_QUOTE_MINT_OFFSET: usize = 104;
const RIPTIDE_SWAP_SELECTOR: u8 = 2;
const RIPTIDE_BASE_TO_QUOTE: u8 = 1;
const RIPTIDE_QUOTE_TO_BASE: u8 = 0;

pub struct RiptideAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub memo_program: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn riptide_swap<'info>(
    accounts: RiptideAccounts<'info>,
    amount_in: u64,
    direction: u8,
) -> Result<SwapResult> {
    let native_direction = match direction {
        0 => RIPTIDE_BASE_TO_QUOTE,
        1 => RIPTIDE_QUOTE_TO_BASE,
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let instruction = build_riptide_swap_instruction(&accounts, amount_in, native_direction);
    let infos = vec![
        accounts.payer.clone(),
        accounts.market.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.token_program.clone(),
        accounts.token_program.clone(),
        accounts.memo_program.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.program.clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn build_riptide_swap_instruction(
    accounts: &RiptideAccounts<'_>,
    amount_in: u64,
    native_direction: u8,
) -> Instruction {
    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.memo_program.key(), false),
        AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
    ];
    let mut data = Vec::with_capacity(12);
    data.push(RIPTIDE_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(native_direction);
    data.extend_from_slice(&[0, 0]);
    Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn validate_riptide_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == RIPTIDE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        RIPTIDE_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        RIPTIDE_MEMO_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[5].key(),
        RIPTIDE_INSTRUCTIONS_SYSVAR_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        input_token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        output_token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );

    let market = step[1].try_borrow_data()?;
    require!(
        market.len() == RIPTIDE_MARKET_ACCOUNT_LEN
            && market.starts_with(&RIPTIDE_MARKET_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let authority = read_pubkey(&market, RIPTIDE_AUTHORITY_OFFSET)?;
    let admin = read_pubkey(&market, RIPTIDE_ADMIN_OFFSET)?;
    let base_mint = read_pubkey(&market, RIPTIDE_BASE_MINT_OFFSET)?;
    let quote_mint = read_pubkey(&market, RIPTIDE_QUOTE_MINT_OFFSET)?;
    drop(market);
    require!(
        authority != Pubkey::default()
            && admin != Pubkey::default()
            && authority != admin
            && base_mint != Pubkey::default()
            && quote_mint != Pubkey::default()
            && base_mint != quote_mint,
        ArbitrageError::InvalidAccount
    );

    let (
        expected_input,
        expected_output,
        base_mint_account,
        quote_mint_account,
        base_program,
        quote_program,
    ) = match direction {
        0 => (
            base_mint,
            quote_mint,
            input_mint,
            output_mint,
            input_token_program,
            output_token_program,
        ),
        1 => (
            quote_mint,
            base_mint,
            output_mint,
            input_mint,
            output_token_program,
            input_token_program,
        ),
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

    let expected_base_vault = associated_token_address(step[1].key(), base_mint);
    let expected_quote_vault = associated_token_address(step[1].key(), quote_mint);
    require_keys_eq!(
        step[2].key(),
        expected_base_vault,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].key(),
        expected_quote_vault,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(
        &step[2],
        base_mint_account,
        base_program,
        &step[1],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[3],
        quote_mint_account,
        quote_program,
        &step[1],
    )?;
    require!(
        read_token_amount(&step[2])? > 0 && read_token_amount(&step[3])? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
}

fn associated_token_address(owner: Pubkey, mint: Pubkey) -> Pubkey {
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
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 32] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
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
    ) {
        let market_key = Pubkey::new_unique();
        let base_mint_key = Pubkey::new_unique();
        let quote_mint_key = Pubkey::new_unique();
        let mut market_data = vec![0_u8; RIPTIDE_MARKET_ACCOUNT_LEN];
        market_data[..8].copy_from_slice(&RIPTIDE_MARKET_DISCRIMINATOR);
        write_pubkey(
            &mut market_data,
            RIPTIDE_AUTHORITY_OFFSET,
            Pubkey::new_unique(),
        );
        write_pubkey(&mut market_data, RIPTIDE_ADMIN_OFFSET, Pubkey::new_unique());
        write_pubkey(&mut market_data, RIPTIDE_BASE_MINT_OFFSET, base_mint_key);
        write_pubkey(&mut market_data, RIPTIDE_QUOTE_MINT_OFFSET, quote_mint_key);
        let step = vec![
            account(RIPTIDE_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(market_key, RIPTIDE_PROGRAM_ID, market_data, true, false),
            account(
                associated_token_address(market_key, base_mint_key),
                anchor_spl::token::ID,
                token_data(base_mint_key, market_key, 1_000_000),
                true,
                false,
            ),
            account(
                associated_token_address(market_key, quote_mint_key),
                anchor_spl::token::ID,
                token_data(quote_mint_key, market_key, 2_000_000),
                true,
                false,
            ),
            account(
                RIPTIDE_MEMO_PROGRAM_ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(
                RIPTIDE_INSTRUCTIONS_SYSVAR_ID,
                anchor_lang::system_program::ID,
                vec![],
                false,
                false,
            ),
        ];
        let base_mint = account(base_mint_key, anchor_spl::token::ID, vec![], false, false);
        let quote_mint = account(quote_mint_key, anchor_spl::token::ID, vec![], false, false);
        let token_program = account(
            anchor_spl::token::ID,
            Pubkey::default(),
            vec![],
            false,
            true,
        );
        (step, base_mint, quote_mint, token_program)
    }

    #[test]
    fn wire_matches_confirmed_mainnet_layout_and_direction_mapping() {
        let mut base_to_quote = vec![RIPTIDE_SWAP_SELECTOR];
        base_to_quote.extend_from_slice(&100_000_000_u64.to_le_bytes());
        base_to_quote.push(RIPTIDE_BASE_TO_QUOTE);
        base_to_quote.extend_from_slice(&[0, 0]);
        assert_eq!(base_to_quote, [2, 0, 225, 245, 5, 0, 0, 0, 0, 1, 0, 0]);

        let mut quote_to_base = base_to_quote;
        quote_to_base[9] = RIPTIDE_QUOTE_TO_BASE;
        assert_eq!(quote_to_base[9], 0);
        assert_ne!(RIPTIDE_MEMO_PROGRAM_ID, RIPTIDE_INSTRUCTIONS_SYSVAR_ID);
    }

    #[test]
    fn semantic_validation_binds_market_mints_vault_atas_and_fixed_accounts() {
        let (step, base_mint, quote_mint, token_program) = semantic_fixture();
        assert!(validate_riptide_semantic_accounts(
            &step,
            0,
            0,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_ok());
        assert!(validate_riptide_semantic_accounts(
            &step,
            1,
            0,
            &quote_mint,
            &base_mint,
            &token_program,
            &token_program,
        )
        .is_ok());

        let (bad_market, base_mint, quote_mint, token_program) = semantic_fixture();
        bad_market[1].try_borrow_mut_data().unwrap()[RIPTIDE_BASE_MINT_OFFSET] ^= 1;
        assert!(validate_riptide_semantic_accounts(
            &bad_market,
            0,
            0,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_err());

        let (mut bad_memo, base_mint, quote_mint, token_program) = semantic_fixture();
        bad_memo[4] = account(Pubkey::new_unique(), Pubkey::default(), vec![], false, true);
        assert!(validate_riptide_semantic_accounts(
            &bad_memo,
            0,
            0,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_err());
    }
}
