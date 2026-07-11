use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const ALDRIN_V2_SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
pub const ALDRIN_V2_MIN_ACCOUNTS: usize = 8;

#[derive(Clone)]
pub struct AldrinV2Accounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub pool_signer: &'info AccountInfo<'info>,
    pub pool_mint: &'info AccountInfo<'info>,
    pub base_token_vault: &'info AccountInfo<'info>,
    pub quote_token_vault: &'info AccountInfo<'info>,
    pub fee_pool_token_account: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub user_base_token_account: &'info AccountInfo<'info>,
    pub user_quote_token_account: &'info AccountInfo<'info>,
    pub curve: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
}

pub fn aldrin_v2_swap<'info>(
    accounts: AldrinV2Accounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    base_to_quote: bool,
) -> Result<SwapResult> {
    let output_token_account = if base_to_quote {
        accounts.user_quote_token_account
    } else {
        accounts.user_base_token_account
    };
    let pre_out = read_token_amount(output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.pool_signer.key(), false),
        AccountMeta::new(accounts.pool_mint.key(), false),
        AccountMeta::new(accounts.base_token_vault.key(), false),
        AccountMeta::new(accounts.quote_token_vault.key(), false),
        AccountMeta::new(accounts.fee_pool_token_account.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.user_base_token_account.key(), false),
        AccountMeta::new(accounts.user_quote_token_account.key(), false),
        AccountMeta::new_readonly(accounts.curve.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let account_infos = vec![
        accounts.pool.clone(),
        accounts.pool_signer.clone(),
        accounts.pool_mint.clone(),
        accounts.base_token_vault.clone(),
        accounts.quote_token_vault.clone(),
        accounts.fee_pool_token_account.clone(),
        accounts.payer.clone(),
        accounts.user_base_token_account.clone(),
        accounts.user_quote_token_account.clone(),
        accounts.curve.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: aldrin_v2_swap_data(amount_in, minimum_amount_out, base_to_quote),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn aldrin_v2_swap_data(amount_in: u64, minimum_amount_out: u64, base_to_quote: bool) -> Vec<u8> {
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(&ALDRIN_V2_SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.push(u8::from(base_to_quote));
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_data_matches_official_sdk_layout() {
        let bid = aldrin_v2_swap_data(234, 1, false);
        assert_eq!(bid.len(), 25);
        assert_eq!(&bid[..8], &ALDRIN_V2_SWAP_DISCRIMINATOR);
        assert_eq!(&bid[8..16], &234_u64.to_le_bytes());
        assert_eq!(&bid[16..24], &1_u64.to_le_bytes());
        assert_eq!(bid[24], 0);

        let ask = aldrin_v2_swap_data(1_000_000, 999_000, true);
        assert_eq!(ask[24], 1);
    }
}
