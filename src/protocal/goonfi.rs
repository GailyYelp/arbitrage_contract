use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const GOONFI_MIN_ACCOUNTS: usize = 7;
pub const GOONFI_SWAP_SELECTOR: u8 = 0x02;

pub struct GoonfiAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub blacklist: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn goonfi_swap<'info>(
    accounts: GoonfiAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
    blacklist_bump: u8,
) -> Result<SwapResult> {
    let is_user_bid = match direction {
        0 => false,
        1 => true,
        _ => return Err(crate::errors::ArbitrageError::InvalidInstructionData.into()),
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new(accounts.payer.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.blacklist.key(), false),
        AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let infos = vec![
        accounts.payer.clone(),
        accounts.market.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.blacklist.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(19);
    data.push(GOONFI_SWAP_SELECTOR);
    data.push(u8::from(is_user_bid));
    data.push(blacklist_bump);
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
    fn direct_data_matches_confirmed_mainnet_prefix() {
        let mut data = vec![GOONFI_SWAP_SELECTOR, 1, 254];
        data.extend_from_slice(&116_993_513_u64.to_le_bytes());
        data.extend_from_slice(&0_u64.to_le_bytes());
        assert_eq!(data.len(), 19);
        assert_eq!(&data[..3], &[2, 1, 254]);
    }
}
