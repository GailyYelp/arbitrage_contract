use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program::invoke,
        sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID,
    },
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::TAURUSFI_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const TAURUSFI_STEP_ACCOUNTS: usize = 8;
pub const TAURUSFI_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("544UsNRFNU7tzWKcHGZorwy7UKnsso9x83N9WWMvY5JX");
pub const TAURUSFI_GLOBAL: Pubkey =
    anchor_lang::pubkey!("EWqXjWSd4sfVsZHdSc25DoD1QfMbMrVpQZPpayoiFPSP");
pub const TAURUSFI_PRICES: Pubkey =
    anchor_lang::pubkey!("Hg2PTGCBUwRvp7S3AF5BF4tfawNshXc1N2DDWijjJndo");

const PAIR_ACCOUNT_LEN: usize = 410;
const MINT_A_OFFSET: usize = 0;
const MINT_B_OFFSET: usize = 32;
const VAULT_A_OFFSET: usize = 346;
const VAULT_B_OFFSET: usize = 378;
const GLOBAL_ACCOUNT_LEN: usize = 504;
const GLOBAL_STATE_SLOT_OFFSET: usize = 480;
const GLOBAL_PRICE_SLOT_OFFSET: usize = 488;
const GLOBAL_TIMESTAMP_MS_OFFSET: usize = 496;
const PRICES_ACCOUNT_LEN: usize = 792;
const SWAP_SELECTOR: u8 = 8;
const SWAP_TAIL: [u8; 2] = [0, 1];

pub struct TaurusFiAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub pair: &'info AccountInfo<'info>,
    pub mint_a: &'info AccountInfo<'info>,
    pub mint_b: &'info AccountInfo<'info>,
    pub vault_a: &'info AccountInfo<'info>,
    pub vault_b: &'info AccountInfo<'info>,
    pub user_source: &'info AccountInfo<'info>,
    pub user_destination: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub global: &'info AccountInfo<'info>,
    pub prices: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn taurusfi_swap<'info>(
    accounts: TaurusFiAccounts<'info>,
    amount_in: u64,
    direction: u8,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let instruction = build_taurusfi_swap_instruction(&accounts, amount_in, direction)?;
    let (user_token_a, user_token_b, input_mint, output_mint) = match direction {
        0 => (
            accounts.user_source,
            accounts.user_destination,
            accounts.mint_a,
            accounts.mint_b,
        ),
        1 => (
            accounts.user_destination,
            accounts.user_source,
            accounts.mint_b,
            accounts.mint_a,
        ),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    let infos = vec![
        accounts.vault_a.clone(),
        accounts.vault_b.clone(),
        accounts.payer.clone(),
        user_token_a.clone(),
        user_token_b.clone(),
        accounts.authority.clone(),
        accounts.pair.clone(),
        accounts.global.clone(),
        accounts.prices.clone(),
        accounts.token_program.clone(),
        accounts.token_program.clone(),
        input_mint.clone(),
        output_mint.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.program.clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn build_taurusfi_swap_instruction(
    accounts: &TaurusFiAccounts<'_>,
    amount_in: u64,
    direction: u8,
) -> Result<Instruction> {
    let (user_token_a, user_token_b, input_mint, output_mint) = match direction {
        0 => (
            accounts.user_source,
            accounts.user_destination,
            accounts.mint_a,
            accounts.mint_b,
        ),
        1 => (
            accounts.user_destination,
            accounts.user_source,
            accounts.mint_b,
            accounts.mint_a,
        ),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    let mut data = Vec::with_capacity(11);
    data.push(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&SWAP_TAIL);
    Ok(Instruction {
        program_id: accounts.program.key(),
        accounts: vec![
            AccountMeta::new(accounts.vault_a.key(), false),
            AccountMeta::new(accounts.vault_b.key(), false),
            AccountMeta::new_readonly(accounts.payer.key(), true),
            AccountMeta::new(user_token_a.key(), false),
            AccountMeta::new(user_token_b.key(), false),
            AccountMeta::new(accounts.authority.key(), false),
            AccountMeta::new_readonly(accounts.pair.key(), false),
            AccountMeta::new_readonly(accounts.global.key(), false),
            AccountMeta::new_readonly(accounts.prices.key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(input_mint.key(), false),
            AccountMeta::new_readonly(output_mint.key(), false),
            AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
        ],
        data,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn validate_taurusfi_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == TAURUSFI_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        TAURUSFI_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        TAURUSFI_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        TAURUSFI_GLOBAL,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        TAURUSFI_PRICES,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[7].key(),
        INSTRUCTIONS_SYSVAR_ID,
        ArbitrageError::InvalidProgramId
    );
    for dependency in [&step[5], &step[6]] {
        require_keys_eq!(
            *dependency.owner,
            step[0].key(),
            ArbitrageError::InvalidAccount
        );
    }
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

    let pair = step[1].try_borrow_data()?;
    require!(
        pair.len() == PAIR_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    let mint_a = read_pubkey(&pair, MINT_A_OFFSET)?;
    let mint_b = read_pubkey(&pair, MINT_B_OFFSET)?;
    let vault_a = read_pubkey(&pair, VAULT_A_OFFSET)?;
    let vault_b = read_pubkey(&pair, VAULT_B_OFFSET)?;
    drop(pair);
    require!(
        ![mint_a, mint_b, vault_a, vault_b].contains(&Pubkey::default())
            && mint_a != mint_b
            && vault_a != vault_b,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[2].key(), vault_a, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[3].key(), vault_b, ArbitrageError::InvalidAccount);

    let global = step[5].try_borrow_data()?;
    require!(
        global.len() == GLOBAL_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    let state_slot = read_u64(&global, GLOBAL_STATE_SLOT_OFFSET)?;
    let price_slot = read_u64(&global, GLOBAL_PRICE_SLOT_OFFSET)?;
    let timestamp_ms = read_u64(&global, GLOBAL_TIMESTAMP_MS_OFFSET)?;
    require!(
        state_slot > 0 && price_slot > 0 && price_slot <= state_slot && timestamp_ms > 0,
        ArbitrageError::InvalidAccount
    );
    drop(global);
    require!(
        step[6].data_len() == PRICES_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output, mint_a_account, mint_b_account, program_a, program_b) =
        match direction {
            0 => (
                mint_a,
                mint_b,
                input_mint,
                output_mint,
                input_token_program,
                output_token_program,
            ),
            1 => (
                mint_b,
                mint_a,
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
    validate_token_account_for_mint_and_authority(&step[2], mint_a_account, program_a, &step[4])?;
    validate_token_account_for_mint_and_authority(&step[3], mint_b_account, program_b, &step[4])?;
    require!(
        read_token_amount(&step[2])? > 0 && read_token_amount(&step[3])? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
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

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 8] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_confirmed_forward_and_reverse_wire() {
        fn account(key: Pubkey) -> &'static AccountInfo<'static> {
            Box::leak(Box::new(AccountInfo::new(
                Box::leak(Box::new(key)),
                false,
                true,
                Box::leak(Box::new(0)),
                Box::leak(Vec::new().into_boxed_slice()),
                Box::leak(Box::new(Pubkey::default())),
                false,
                0,
            )))
        }
        let keys: Vec<_> = (1..=15)
            .map(|seed| Pubkey::new_from_array([seed; 32]))
            .collect();
        let infos: Vec<_> = keys.iter().copied().map(account).collect();
        let accounts = TaurusFiAccounts {
            program: infos[0],
            payer: infos[1],
            pair: infos[2],
            mint_a: infos[3],
            mint_b: infos[4],
            vault_a: infos[5],
            vault_b: infos[6],
            user_source: infos[7],
            user_destination: infos[8],
            authority: infos[9],
            global: infos[10],
            prices: infos[11],
            token_program: infos[12],
            instructions_sysvar: infos[13],
            output_token_account: infos[8],
        };
        let forward = build_taurusfi_swap_instruction(&accounts, 793_086_470, 0).unwrap();
        assert_eq!(
            forward.data,
            [
                vec![SWAP_SELECTOR],
                793_086_470_u64.to_le_bytes().to_vec(),
                SWAP_TAIL.to_vec(),
            ]
            .concat()
        );
        assert_eq!(forward.accounts[3].pubkey, infos[7].key());
        assert_eq!(forward.accounts[11].pubkey, infos[3].key());
        let reverse = build_taurusfi_swap_instruction(&accounts, 500_000_000, 1).unwrap();
        assert_eq!(reverse.accounts[3].pubkey, infos[8].key());
        assert_eq!(reverse.accounts[4].pubkey, infos[7].key());
        assert_eq!(reverse.accounts[11].pubkey, infos[4].key());
    }
}
