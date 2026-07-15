use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const BINARYFI_MIN_ACCOUNTS: usize = 9;
pub const BINARYFI_SWAP_SELECTOR: u8 = 8;

pub struct BinaryFiAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub input_vault: &'info AccountInfo<'info>,
    pub output_vault: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_token_program: &'info AccountInfo<'info>,
    pub output_token_program: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
}

pub fn binaryfi_swap<'info>(
    accounts: BinaryFiAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(
        direction == 0,
        crate::errors::ArbitrageError::InvalidInstructionData
    );
    let pre_out = read_token_amount(accounts.user_output)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.config.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new_readonly(accounts.input_mint.key(), false),
        AccountMeta::new_readonly(accounts.output_mint.key(), false),
        AccountMeta::new(accounts.input_vault.key(), false),
        AccountMeta::new(accounts.output_vault.key(), false),
        AccountMeta::new(accounts.user_input.key(), false),
        AccountMeta::new(accounts.user_output.key(), false),
        AccountMeta::new_readonly(accounts.input_token_program.key(), false),
        AccountMeta::new_readonly(accounts.output_token_program.key(), false),
        AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
    ];
    let infos = vec![
        accounts.payer.clone(),
        accounts.config.clone(),
        accounts.pool.clone(),
        accounts.authority.clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.input_vault.clone(),
        accounts.output_vault.clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.input_token_program.clone(),
        accounts.output_token_program.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(17);
    data.push(BINARYFI_SWAP_SELECTOR);
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
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_matches_confirmed_mainnet_layout() {
        let mut data = vec![BINARYFI_SWAP_SELECTOR];
        data.extend_from_slice(&100_000_000_u64.to_le_bytes());
        data.extend_from_slice(&7_400_000_u64.to_le_bytes());
        assert_eq!(data.len(), 17);
        assert_eq!(data[0], 8);
        assert_eq!(&data[1..9], &100_000_000_u64.to_le_bytes());
        assert_eq!(&data[9..17], &7_400_000_u64.to_le_bytes());
    }
}
