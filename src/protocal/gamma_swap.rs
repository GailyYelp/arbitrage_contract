use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const GAMMA_SWAP_BASE_INPUT_SELECTOR: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub const GAMMA_SWAP_MIN_ACCOUNTS: usize = 11;

#[derive(Clone)]
pub struct GammaSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub amm_config: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub input_vault: &'info AccountInfo<'info>,
    pub output_vault: &'info AccountInfo<'info>,
    pub input_token_program: &'info AccountInfo<'info>,
    pub output_token_program: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub observation_state: &'info AccountInfo<'info>,
}

pub fn gamma_swap_base_input<'info>(
    accounts: GammaSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;

    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.amm_config.key(), false),
        AccountMeta::new(accounts.pool_state.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new(accounts.input_vault.key(), false),
        AccountMeta::new(accounts.output_vault.key(), false),
        AccountMeta::new_readonly(accounts.input_token_program.key(), false),
        AccountMeta::new_readonly(accounts.output_token_program.key(), false),
        AccountMeta::new_readonly(accounts.input_mint.key(), false),
        AccountMeta::new_readonly(accounts.output_mint.key(), false),
        AccountMeta::new(accounts.observation_state.key(), false),
    ];
    let account_infos = vec![
        accounts.payer.clone(),
        accounts.authority.clone(),
        accounts.amm_config.clone(),
        accounts.pool_state.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.input_vault.clone(),
        accounts.output_vault.clone(),
        accounts.input_token_program.clone(),
        accounts.output_token_program.clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.observation_state.clone(),
        accounts.program.clone(),
    ];

    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: gamma_swap_base_input_data(amount_in, minimum_amount_out),
    };
    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(accounts.output_token_account, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

fn gamma_swap_base_input_data(amount_in: u64, minimum_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(GAMMA_SWAP_BASE_INPUT_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_base_input_data_layout_matches_gamma_builder_contract() {
        let data = gamma_swap_base_input_data(100, 90);

        assert_eq!(data.len(), 24);
        assert_eq!(&data[0..8], GAMMA_SWAP_BASE_INPUT_SELECTOR);
        assert_eq!(&data[8..16], &100_u64.to_le_bytes());
        assert_eq!(&data[16..24], &90_u64.to_le_bytes());
    }
}
