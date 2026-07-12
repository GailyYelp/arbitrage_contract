use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{read_token_amount, token_balance_delta, SwapResult},
};

pub const HEAVEN_STEP_ACCOUNTS: usize = 14;
pub const HEAVEN_BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
pub const HEAVEN_SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

#[derive(Clone)]
pub struct HeavenAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

fn trade_data(discriminator: [u8; 8], amount: u64, min_output: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(28);
    data.extend_from_slice(&discriminator);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&min_output.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data
}

pub fn heaven_swap<'info>(
    accounts: HeavenAccounts<'info>,
    amount: u64,
    buy: bool,
) -> Result<SwapResult> {
    require!(
        accounts.step.len() == HEAVEN_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    let (user_token_a, user_token_b, discriminator) = if buy {
        (
            accounts.user_output,
            accounts.user_input,
            HEAVEN_BUY_DISCRIMINATOR,
        )
    } else {
        (
            accounts.user_input,
            accounts.user_output,
            HEAVEN_SELL_DISCRIMINATOR,
        )
    };
    let instruction_accounts = vec![
        AccountMeta::new_readonly(accounts.step[1].key(), false),
        AccountMeta::new_readonly(accounts.step[2].key(), false),
        AccountMeta::new_readonly(accounts.step[3].key(), false),
        AccountMeta::new_readonly(accounts.step[4].key(), false),
        AccountMeta::new(accounts.step[5].key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.step[6].key(), false),
        AccountMeta::new_readonly(accounts.step[7].key(), false),
        AccountMeta::new(user_token_a.key(), false),
        AccountMeta::new(user_token_b.key(), false),
        AccountMeta::new(accounts.step[8].key(), false),
        AccountMeta::new(accounts.step[9].key(), false),
        AccountMeta::new(accounts.step[10].key(), false),
        AccountMeta::new_readonly(accounts.step[11].key(), false),
        AccountMeta::new_readonly(accounts.step[12].key(), false),
        AccountMeta::new_readonly(accounts.step[13].key(), false),
    ];
    let account_infos = vec![
        accounts.step[1].clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.step[4].clone(),
        accounts.step[5].clone(),
        accounts.payer.clone(),
        accounts.step[6].clone(),
        accounts.step[7].clone(),
        user_token_a.clone(),
        user_token_b.clone(),
        accounts.step[8].clone(),
        accounts.step[9].clone(),
        accounts.step[10].clone(),
        accounts.step[11].clone(),
        accounts.step[12].clone(),
        accounts.step[13].clone(),
        accounts.step[0].clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: instruction_accounts,
            data: trade_data(discriminator, amount, 1),
        },
        &account_infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payload_matches_official_idl() {
        let data = trade_data(HEAVEN_BUY_DISCRIMINATOR, 1, 2);
        assert_eq!(data.len(), 28);
        assert_eq!(&data[24..], &[0, 0, 0, 0]);
    }
}
