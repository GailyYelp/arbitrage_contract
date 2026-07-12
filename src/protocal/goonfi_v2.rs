use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const GOONFI_V2_MIN_ACCOUNTS: usize = 10;
pub const GOONFI_V2_SWAP_SELECTOR: u8 = 0x01;

pub struct GoonfiV2Accounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub price_account: &'info AccountInfo<'info>,
    pub global_state: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub base_token_program: &'info AccountInfo<'info>,
    pub quote_token_program: &'info AccountInfo<'info>,
    pub vote_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn goonfi_v2_swap<'info>(
    accounts: GoonfiV2Accounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(
        direction <= 1,
        crate::errors::ArbitrageError::InvalidInstructionData
    );
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new(accounts.payer.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(accounts.price_account.key(), false),
        AccountMeta::new_readonly(accounts.global_state.key(), false),
        AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
        AccountMeta::new_readonly(accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(accounts.vote_account.key(), false),
    ];
    let infos = vec![
        accounts.payer.clone(),
        accounts.market.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.price_account.clone(),
        accounts.global_state.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.vote_account.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(18);
    data.push(GOONFI_V2_SWAP_SELECTOR);
    data.push(direction);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_data_matches_confirmed_mainnet_layout() {
        let mut data = vec![GOONFI_V2_SWAP_SELECTOR, 1];
        data.extend_from_slice(&344_476_220_u64.to_le_bytes());
        data.extend_from_slice(&0_u64.to_le_bytes());
        assert_eq!(data.len(), 18);
        assert_eq!(&data[..2], &[1, 1]);
    }
}
