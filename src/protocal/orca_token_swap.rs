use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const ORCA_TOKEN_SWAP_MIN_ACCOUNTS: usize = 7;
pub const ORCA_TOKEN_SWAP_SWAP_TAG: u8 = 1;

#[derive(Clone)]
pub struct OrcaTokenSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub input_vault: &'info AccountInfo<'info>,
    pub output_vault: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub pool_mint: &'info AccountInfo<'info>,
    pub pool_fee_account: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn orca_token_swap<'info>(
    accounts: OrcaTokenSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let mut data = Vec::with_capacity(17);
    data.push(ORCA_TOKEN_SWAP_SWAP_TAG);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());

    let metas = orca_token_swap_account_metas(&accounts);
    let account_infos = vec![
        accounts.pool_state.clone(),
        accounts.authority.clone(),
        accounts.payer.clone(),
        accounts.input_token_account.clone(),
        accounts.input_vault.clone(),
        accounts.output_vault.clone(),
        accounts.output_token_account.clone(),
        accounts.pool_mint.clone(),
        accounts.pool_fee_account.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &account_infos,
    )?;

    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn orca_token_swap_account_metas(accounts: &OrcaTokenSwapAccounts<'_>) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(accounts.pool_state.key(), false),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.input_vault.key(), false),
        AccountMeta::new(accounts.output_vault.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.pool_mint.key(), false),
        AccountMeta::new(accounts.pool_fee_account.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pubkey(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn test_account(seed: u8) -> &'static AccountInfo<'static> {
        let key = Box::leak(Box::new(test_pubkey(seed)));
        let owner = Box::leak(Box::new(test_pubkey(seed.wrapping_add(100))));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        Box::leak(Box::new(AccountInfo::new(
            key, false, false, lamports, data, owner, false, 0,
        )))
    }

    #[test]
    fn cpi_account_metas_match_pinned_spl_token_swap_layout() {
        let accounts = OrcaTokenSwapAccounts {
            program: test_account(1),
            pool_state: test_account(2),
            authority: test_account(3),
            payer: test_account(4),
            input_token_account: test_account(5),
            input_vault: test_account(6),
            output_vault: test_account(7),
            output_token_account: test_account(8),
            pool_mint: test_account(9),
            pool_fee_account: test_account(10),
            token_program: test_account(11),
        };
        let metas = orca_token_swap_account_metas(&accounts);

        assert_eq!(metas.len(), 10);
        assert_eq!(
            metas[0],
            AccountMeta::new_readonly(accounts.pool_state.key(), false)
        );
        assert_eq!(
            metas[1],
            AccountMeta::new_readonly(accounts.authority.key(), false)
        );
        assert_eq!(
            metas[2],
            AccountMeta::new_readonly(accounts.payer.key(), true)
        );
        assert_eq!(
            metas[4],
            AccountMeta::new(accounts.input_vault.key(), false)
        );
        assert_eq!(
            metas[5],
            AccountMeta::new(accounts.output_vault.key(), false)
        );
        assert_eq!(
            metas[9],
            AccountMeta::new_readonly(accounts.token_program.key(), false)
        );
    }
}
