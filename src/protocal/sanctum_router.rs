use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const SANCTUM_ROUTER_STEP_ACCOUNTS: usize = 15;
pub const SANCTUM_STAKE_WRAPPED_SOL_IX: u8 = 0;
pub const SANCTUM_WITHDRAW_WRAPPED_SOL_IX: u8 = 8;

#[derive(Clone)]
pub struct SanctumRouterAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

pub fn sanctum_router_swap<'info>(
    accounts: SanctumRouterAccounts<'info>,
    amount_in: u64,
    deposit: bool,
) -> Result<SwapResult> {
    require!(
        accounts.step.len() == SANCTUM_ROUTER_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    let mut data = Vec::with_capacity(9);
    data.push(if deposit {
        SANCTUM_STAKE_WRAPPED_SOL_IX
    } else {
        SANCTUM_WITHDRAW_WRAPPED_SOL_IX
    });
    data.extend_from_slice(&amount_in.to_le_bytes());

    let (metas, mut account_infos) = if deposit {
        (
            vec![
                AccountMeta::new_readonly(accounts.payer.key(), true),
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new(accounts.user_output.key(), false),
                AccountMeta::new(accounts.step[8].key(), false),
                AccountMeta::new(accounts.step[9].key(), false),
                AccountMeta::new(accounts.step[6].key(), false),
                AccountMeta::new(accounts.step[14].key(), false),
                AccountMeta::new_readonly(accounts.input_mint.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new_readonly(accounts.step[10].key(), false),
                AccountMeta::new_readonly(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new_readonly(accounts.step[3].key(), false),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new(accounts.step[5].key(), false),
            ],
            vec![
                accounts.payer.clone(),
                accounts.user_input.clone(),
                accounts.user_output.clone(),
                accounts.step[8].clone(),
                accounts.step[9].clone(),
                accounts.step[6].clone(),
                accounts.step[14].clone(),
                accounts.input_mint.clone(),
                accounts.token_program.clone(),
                accounts.step[10].clone(),
                accounts.step[1].clone(),
                accounts.step[2].clone(),
                accounts.step[3].clone(),
                accounts.step[4].clone(),
                accounts.step[5].clone(),
            ],
        )
    } else {
        (
            vec![
                AccountMeta::new_readonly(accounts.payer.key(), true),
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new(accounts.user_output.key(), false),
                AccountMeta::new(accounts.step[7].key(), false),
                AccountMeta::new(accounts.step[14].key(), false),
                AccountMeta::new_readonly(accounts.output_mint.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new(accounts.step[1].key(), false),
                AccountMeta::new(accounts.step[2].key(), false),
                AccountMeta::new_readonly(accounts.step[3].key(), false),
                AccountMeta::new(accounts.step[4].key(), false),
                AccountMeta::new(accounts.step[5].key(), false),
                AccountMeta::new_readonly(accounts.step[11].key(), false),
                AccountMeta::new_readonly(accounts.step[12].key(), false),
                AccountMeta::new_readonly(accounts.step[13].key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
            ],
            vec![
                accounts.payer.clone(),
                accounts.user_input.clone(),
                accounts.user_output.clone(),
                accounts.step[7].clone(),
                accounts.step[14].clone(),
                accounts.output_mint.clone(),
                accounts.token_program.clone(),
                accounts.step[1].clone(),
                accounts.step[2].clone(),
                accounts.step[3].clone(),
                accounts.step[4].clone(),
                accounts.step[5].clone(),
                accounts.step[11].clone(),
                accounts.step[12].clone(),
                accounts.step[13].clone(),
                accounts.token_program.clone(),
            ],
        )
    };
    account_infos.push(accounts.step[0].clone());
    invoke(
        &Instruction {
            program_id: accounts.step[0].key(),
            accounts: metas,
            data,
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
    fn instruction_data_matches_sanctum_router_layout() {
        let amount = 1_000_000_u64;
        let mut deposit = vec![SANCTUM_STAKE_WRAPPED_SOL_IX];
        deposit.extend_from_slice(&amount.to_le_bytes());
        let mut withdraw = vec![SANCTUM_WITHDRAW_WRAPPED_SOL_IX];
        withdraw.extend_from_slice(&amount.to_le_bytes());
        assert_eq!(deposit, [vec![0], amount.to_le_bytes().to_vec()].concat());
        assert_eq!(withdraw, [vec![8], amount.to_le_bytes().to_vec()].concat());
    }
}
