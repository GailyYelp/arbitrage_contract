use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{read_token_amount, token_balance_delta, SwapResult},
};

pub const SAROS_DLMM_STEP_ACCOUNTS: usize = 14;
pub const SAROS_DLMM_SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];

#[derive(Clone)]
pub struct SarosDlmmAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

fn swap_data(amount: u64, swap_for_y: bool) -> Vec<u8> {
    let mut data = Vec::with_capacity(26);
    data.extend_from_slice(&SAROS_DLMM_SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&1_u64.to_le_bytes());
    data.push(u8::from(swap_for_y));
    data.push(0); // SwapType::ExactInput
    data
}

pub fn saros_dlmm_swap<'info>(
    accounts: SarosDlmmAccounts<'info>,
    amount: u64,
    swap_for_y: bool,
) -> Result<SwapResult> {
    require!(
        accounts.step.len() == SAROS_DLMM_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    let (user_vault_x, user_vault_y) = if swap_for_y {
        (accounts.user_input, accounts.user_output)
    } else {
        (accounts.user_output, accounts.user_input)
    };
    let instruction_accounts = vec![
        AccountMeta::new(accounts.step[1].key(), false),
        AccountMeta::new_readonly(accounts.step[2].key(), false),
        AccountMeta::new_readonly(accounts.step[3].key(), false),
        AccountMeta::new(accounts.step[4].key(), false),
        AccountMeta::new(accounts.step[5].key(), false),
        AccountMeta::new(accounts.step[6].key(), false),
        AccountMeta::new(accounts.step[7].key(), false),
        AccountMeta::new(user_vault_x.key(), false),
        AccountMeta::new(user_vault_y.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.step[8].key(), false),
        AccountMeta::new_readonly(accounts.step[9].key(), false),
        AccountMeta::new_readonly(accounts.step[10].key(), false),
        AccountMeta::new(accounts.step[11].key(), false),
        AccountMeta::new_readonly(accounts.step[12].key(), false),
        AccountMeta::new_readonly(accounts.step[13].key(), false),
        AccountMeta::new_readonly(accounts.step[0].key(), false),
    ];
    let account_infos = vec![
        accounts.step[1].clone(),
        accounts.step[2].clone(),
        accounts.step[3].clone(),
        accounts.step[4].clone(),
        accounts.step[5].clone(),
        accounts.step[6].clone(),
        accounts.step[7].clone(),
        user_vault_x.clone(),
        user_vault_y.clone(),
        accounts.payer.clone(),
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
            data: swap_data(amount, swap_for_y),
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
        let data = swap_data(7, true);
        assert_eq!(data.len(), 26);
        assert_eq!(&data[..8], &SAROS_DLMM_SWAP_DISCRIMINATOR);
        assert_eq!(data[24], 1);
        assert_eq!(data[25], 0);
    }
}
