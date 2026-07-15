use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const WHALESTREET_MIN_ACCOUNTS: usize = 6;
pub const WHALESTREET_SWAP_DISCRIMINATOR: [u8; 8] =
    [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62];
pub const WHALESTREET_BASE_TO_QUOTE_SIDE: u8 = 1;
pub const WHALESTREET_QUOTE_TO_BASE_SIDE: u8 = 3;

pub struct WhaleStreetAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn whalestreet_swap<'info>(
    accounts: WhaleStreetAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    let side = match direction {
        0 => WHALESTREET_BASE_TO_QUOTE_SIDE,
        1 => WHALESTREET_QUOTE_TO_BASE_SIDE,
        _ => return Err(crate::errors::ArbitrageError::InvalidInstructionData.into()),
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
    ];
    let infos = vec![
        accounts.pool.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.payer.clone(),
        accounts.token_program.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(41);
    data.extend_from_slice(&WHALESTREET_SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.push(side);
    // Current v2 accepts zero authorization fields and computes against live orderbook state.
    data.extend_from_slice(&0_u64.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
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
    fn v2_wire_matches_confirmed_mainnet_layout() {
        let mut data = Vec::new();
        data.extend_from_slice(&WHALESTREET_SWAP_DISCRIMINATOR);
        data.extend_from_slice(&100_000_000_u64.to_le_bytes());
        data.extend_from_slice(&7_500_000_u64.to_le_bytes());
        data.push(WHALESTREET_BASE_TO_QUOTE_SIDE);
        data.extend_from_slice(&[0; 16]);
        assert_eq!(data.len(), 41);
        assert_eq!(&data[..8], &WHALESTREET_SWAP_DISCRIMINATOR);
        assert_eq!(data[24], 1);
    }
}
