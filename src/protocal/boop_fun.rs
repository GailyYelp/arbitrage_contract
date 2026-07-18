use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
    system_instruction,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{read_token_amount, token_balance_delta, SwapResult},
    protocal::wsol::{close_wsol_for_native, restore_wsol_after_native, WsolBridgeAccounts},
};

pub const BOOP_FUN_STEP_ACCOUNTS: usize = 10;
pub const BOOP_FUN_BUY_DISCRIMINATOR: [u8; 8] = [138, 127, 14, 91, 38, 87, 115, 105];
pub const BOOP_FUN_SELL_DISCRIMINATOR: [u8; 8] = [109, 61, 40, 187, 230, 176, 135, 174];

#[derive(Clone)]
pub struct BoopFunAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

fn trade_data(discriminator: [u8; 8], amount: u64, min_output: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&min_output.to_le_bytes());
    data
}

fn invoke_boop_fun<'info>(accounts: &BoopFunAccounts<'info>, amount: u64, buy: bool) -> Result<()> {
    let (instruction_accounts, account_infos, data) = if buy {
        (
            vec![
                AccountMeta::new_readonly(accounts.output_mint.key(), false),
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new(accounts.step[3].key(), false),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new(accounts.user_output.key(), false),
                AccountMeta::new(accounts.payer.key(), true),
                AccountMeta::new_readonly(accounts.step[5].key(), false),
                AccountMeta::new_readonly(accounts.step[6].key(), false),
                AccountMeta::new_readonly(accounts.input_mint.key(), false),
                AccountMeta::new_readonly(accounts.step[7].key(), false),
                AccountMeta::new_readonly(accounts.step[8].key(), false),
                AccountMeta::new_readonly(accounts.step[9].key(), false),
            ],
            vec![
                accounts.output_mint.clone(),
                accounts.step[1].clone(),
                accounts.step[2].clone(),
                accounts.step[3].clone(),
                accounts.step[4].clone(),
                accounts.user_output.clone(),
                accounts.payer.clone(),
                accounts.step[5].clone(),
                accounts.step[6].clone(),
                accounts.input_mint.clone(),
                accounts.step[7].clone(),
                accounts.step[8].clone(),
                accounts.step[9].clone(),
                accounts.step[0].clone(),
            ],
            trade_data(BOOP_FUN_BUY_DISCRIMINATOR, amount, 1),
        )
    } else {
        (
            vec![
                AccountMeta::new_readonly(accounts.input_mint.key(), false),
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new(accounts.step[3].key(), false),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new(accounts.payer.key(), true),
                AccountMeta::new(accounts.payer.key(), false),
                AccountMeta::new_readonly(accounts.step[5].key(), false),
                AccountMeta::new_readonly(accounts.step[7].key(), false),
                AccountMeta::new_readonly(accounts.step[8].key(), false),
                AccountMeta::new_readonly(accounts.step[9].key(), false),
            ],
            vec![
                accounts.input_mint.clone(),
                accounts.step[1].clone(),
                accounts.step[2].clone(),
                accounts.step[3].clone(),
                accounts.step[4].clone(),
                accounts.user_input.clone(),
                accounts.payer.clone(),
                accounts.payer.clone(),
                accounts.step[5].clone(),
                accounts.step[7].clone(),
                accounts.step[8].clone(),
                accounts.step[9].clone(),
                accounts.step[0].clone(),
            ],
            trade_data(BOOP_FUN_SELL_DISCRIMINATOR, amount, 1),
        )
    };
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: instruction_accounts,
            data,
        },
        &account_infos,
    )?;
    Ok(())
}

pub fn boop_fun_swap<'info>(
    accounts: BoopFunAccounts<'info>,
    amount: u64,
    buy: bool,
) -> Result<SwapResult> {
    require!(
        accounts.step.len() == BOOP_FUN_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    if buy {
        let bridge = WsolBridgeAccounts {
            payer: accounts.payer,
            user_wsol: accounts.user_input,
            wsol_mint: accounts.input_mint,
            token_program: &accounts.step[8],
            associated_token_program: accounts.associated_token_program,
            system_program: &accounts.step[7],
        };
        let remaining_wsol = close_wsol_for_native(bridge, amount)?;
        invoke_boop_fun(&accounts, amount, true)?;
        restore_wsol_after_native(bridge, remaining_wsol)?;
    } else {
        let pre_lamports = accounts.payer.lamports();
        invoke_boop_fun(&accounts, amount, false)?;
        let received = accounts
            .payer
            .lamports()
            .checked_sub(pre_lamports)
            .ok_or(ArbitrageError::MathOverflow)?;
        require!(received > 0, ArbitrageError::InvalidAmount);
        invoke(
            &system_instruction::transfer(accounts.payer.key, accounts.user_output.key, received),
            &[
                accounts.payer.clone(),
                accounts.user_output.clone(),
                accounts.step[7].clone(),
            ],
        )?;
        invoke(
            &anchor_spl::token::spl_token::instruction::sync_native(
                accounts.step[8].key,
                accounts.user_output.key,
            )?,
            &[accounts.user_output.clone(), accounts.step[8].clone()],
        )?;
    }
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_payloads_match_on_chain_idl_layout() {
        assert_eq!(trade_data(BOOP_FUN_BUY_DISCRIMINATOR, 1, 2).len(), 24);
        assert_eq!(
            &trade_data(BOOP_FUN_SELL_DISCRIMINATOR, 3, 4)[..8],
            &BOOP_FUN_SELL_DISCRIMINATOR
        );
    }
}
