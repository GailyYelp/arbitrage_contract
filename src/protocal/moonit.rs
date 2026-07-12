use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
    system_instruction,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{read_token_amount, token_balance_delta, SwapResult},
};

pub const MOONIT_STEP_ACCOUNTS: usize = 9;
pub const MOONIT_BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
pub const MOONIT_SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
const TOKEN_UNWRAP_LAMPORTS_TAG: u8 = 45;
const TOKEN_UNWRAP_LAMPORTS_EXACT: u8 = 1;

#[derive(Clone)]
pub struct MoonitAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

fn trade_data(
    discriminator: [u8; 8],
    token_amount: u64,
    collateral_amount: u64,
    slippage_bps: u64,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(33);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&token_amount.to_le_bytes());
    data.extend_from_slice(&collateral_amount.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&slippage_bps.to_le_bytes());
    data
}

fn unwrap_wsol<'info>(accounts: &MoonitAccounts<'info>, amount: u64) -> Result<()> {
    let mut data = vec![TOKEN_UNWRAP_LAMPORTS_TAG, TOKEN_UNWRAP_LAMPORTS_EXACT];
    data.extend_from_slice(&amount.to_le_bytes());
    invoke(
        &Instruction {
            program_id: accounts.step[8].key(),
            accounts: vec![
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new(accounts.payer.key(), false),
                AccountMeta::new_readonly(accounts.payer.key(), true),
            ],
            data,
        },
        &[
            accounts.user_input.clone(),
            accounts.payer.clone(),
            accounts.payer.clone(),
            accounts.step[8].clone(),
        ],
    )?;
    Ok(())
}

fn invoke_moonit<'info>(accounts: &MoonitAccounts<'info>, amount: u64, buy: bool) -> Result<()> {
    let user_token = if buy {
        accounts.user_output
    } else {
        accounts.user_input
    };
    let token_mint = if buy {
        accounts.output_mint
    } else {
        accounts.input_mint
    };
    let data = if buy {
        trade_data(MOONIT_BUY_DISCRIMINATOR, 0, amount, 0)
    } else {
        trade_data(MOONIT_SELL_DISCRIMINATOR, amount, 1, 1)
    };
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: vec![
                AccountMeta::new(accounts.payer.key(), true),
                AccountMeta::new(user_token.key(), false),
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new(accounts.step[3].key(), false),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new_readonly(token_mint.key(), false),
                AccountMeta::new_readonly(accounts.step[5].key(), false),
                AccountMeta::new_readonly(accounts.step[8].key(), false),
                AccountMeta::new_readonly(accounts.step[6].key(), false),
                AccountMeta::new_readonly(accounts.step[7].key(), false),
            ],
            data,
        },
        &[
            accounts.payer.clone(),
            user_token.clone(),
            accounts.step[1].clone(),
            accounts.step[2].clone(),
            accounts.step[3].clone(),
            accounts.step[4].clone(),
            token_mint.clone(),
            accounts.step[5].clone(),
            accounts.step[8].clone(),
            accounts.step[6].clone(),
            accounts.step[7].clone(),
            accounts.step[0].clone(),
        ],
    )?;
    Ok(())
}

pub fn moonit_swap<'info>(
    accounts: MoonitAccounts<'info>,
    amount: u64,
    buy: bool,
) -> Result<SwapResult> {
    require!(
        accounts.step.len() == MOONIT_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    if buy {
        unwrap_wsol(&accounts, amount)?;
        invoke_moonit(&accounts, amount, true)?;
    } else {
        let pre_lamports = accounts.payer.lamports();
        invoke_moonit(&accounts, amount, false)?;
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
    fn exact_in_trade_payloads_match_confirmed_wire_layout() {
        assert_eq!(
            trade_data(MOONIT_BUY_DISCRIMINATOR, 0, 9_970_000, 0).len(),
            33
        );
        assert_eq!(
            &trade_data(MOONIT_SELL_DISCRIMINATOR, 123, 1, 1)[..8],
            &MOONIT_SELL_DISCRIMINATOR
        );
    }
}
