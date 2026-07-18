use anchor_lang::{prelude::*, solana_program::program::invoke};

use crate::{errors::ArbitrageError, instructions::types::read_token_amount};

const WSOL_MINT: Pubkey = anchor_lang::pubkey!("So11111111111111111111111111111111111111112");

#[derive(Clone, Copy)]
pub struct WsolBridgeAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_wsol: &'a AccountInfo<'info>,
    pub wsol_mint: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub associated_token_program: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl WsolBridgeAccounts<'_, '_> {
    fn validate(&self) -> Result<()> {
        require_keys_eq!(
            self.wsol_mint.key(),
            WSOL_MINT,
            ArbitrageError::InvalidTokenMint
        );
        require_keys_eq!(
            self.token_program.key(),
            anchor_spl::token::ID,
            ArbitrageError::InvalidProgramId
        );
        require_keys_eq!(
            self.associated_token_program.key(),
            anchor_spl::associated_token::ID,
            ArbitrageError::InvalidProgramId
        );
        require_keys_eq!(
            self.system_program.key(),
            anchor_lang::system_program::ID,
            ArbitrageError::InvalidProgramId
        );
        let expected = anchor_spl::associated_token::get_associated_token_address_with_program_id(
            self.payer.key,
            self.wsol_mint.key,
            self.token_program.key,
        );
        require_keys_eq!(
            self.user_wsol.key(),
            expected,
            ArbitrageError::InvalidAccount
        );
        Ok(())
    }
}

pub fn close_wsol_for_native(accounts: WsolBridgeAccounts<'_, '_>, amount: u64) -> Result<u64> {
    accounts.validate()?;
    require!(amount > 0, ArbitrageError::InvalidAmount);
    let current_amount = read_token_amount(accounts.user_wsol)?;
    let remaining_amount = current_amount
        .checked_sub(amount)
        .ok_or(ArbitrageError::InvalidAmount)?;
    let instruction = anchor_spl::token::spl_token::instruction::close_account(
        accounts.token_program.key,
        accounts.user_wsol.key,
        accounts.payer.key,
        accounts.payer.key,
        &[],
    )?;
    invoke(
        &instruction,
        &[
            accounts.user_wsol.clone(),
            accounts.payer.clone(),
            accounts.payer.clone(),
            accounts.token_program.clone(),
        ],
    )?;
    Ok(remaining_amount)
}

pub fn restore_wsol_after_native(
    accounts: WsolBridgeAccounts<'_, '_>,
    remaining_amount: u64,
) -> Result<()> {
    accounts.validate()?;
    let create = anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        accounts.payer.key,
        accounts.payer.key,
        accounts.wsol_mint.key,
        accounts.token_program.key,
    );
    invoke(
        &create,
        &[
            accounts.payer.clone(),
            accounts.user_wsol.clone(),
            accounts.payer.clone(),
            accounts.wsol_mint.clone(),
            accounts.system_program.clone(),
            accounts.token_program.clone(),
            accounts.associated_token_program.clone(),
        ],
    )?;
    if remaining_amount > 0 {
        invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                accounts.payer.key,
                accounts.user_wsol.key,
                remaining_amount,
            ),
            &[
                accounts.payer.clone(),
                accounts.user_wsol.clone(),
                accounts.system_program.clone(),
            ],
        )?;
        invoke(
            &anchor_spl::token::spl_token::instruction::sync_native(
                accounts.token_program.key,
                accounts.user_wsol.key,
            )?,
            &[accounts.user_wsol.clone(), accounts.token_program.clone()],
        )?;
    }
    Ok(())
}
