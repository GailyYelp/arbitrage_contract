use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const HUMIDIFI_MIN_ACCOUNTS: usize = 13;
pub const HUMIDIFI_SWAP_V3_SELECTOR: u8 = 0x14;
const HUMIDIFI_IX_DATA_KEY: u64 = u64::from_le_bytes([58, 255, 47, 255, 226, 186, 235, 195]);
const HUMIDIFI_POSITION_MASK: u64 = 0x0001_0001_0001_0001;

pub struct HumidifiAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub clock: &'info AccountInfo<'info>,
    pub base_token_program: &'info AccountInfo<'info>,
    pub quote_token_program: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub extra_account: &'info AccountInfo<'info>,
    pub vote_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn humidifi_swap_v3<'info>(
    accounts: HumidifiAccounts<'info>,
    swap_id: u64,
    amount_in: u64,
    direction: u8,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new_readonly(accounts.clock.key(), false),
        AccountMeta::new_readonly(accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(accounts.extra_account.key(), false),
        AccountMeta::new_readonly(accounts.vote_account.key(), false),
    ];
    let infos = vec![
        accounts.payer.clone(),
        accounts.market.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.clock.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.extra_account.clone(),
        accounts.vote_account.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(&swap_id.to_le_bytes());
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(direction);
    data.extend_from_slice(&[0_u8; 7]);
    data.push(HUMIDIFI_SWAP_V3_SELECTOR);
    obfuscate_instruction_data(&mut data);
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

fn obfuscate_instruction_data(data: &mut [u8]) {
    for (index, chunk) in data.chunks_mut(8).enumerate() {
        let mut bytes = [0_u8; 8];
        bytes[..chunk.len()].copy_from_slice(chunk);
        let value = u64::from_le_bytes(bytes)
            ^ HUMIDIFI_IX_DATA_KEY
            ^ HUMIDIFI_POSITION_MASK.wrapping_mul(index as u64);
        chunk.copy_from_slice(&value.to_le_bytes()[..chunk.len()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_data_matches_known_v3_plaintext_after_roundtrip() {
        let mut data = Vec::new();
        data.extend_from_slice(&4_755_617_054_045_973_570_u64.to_le_bytes());
        data.extend_from_slice(&15_084_948_u64.to_le_bytes());
        data.push(1);
        data.extend_from_slice(&[0_u8; 7]);
        data.push(HUMIDIFI_SWAP_V3_SELECTOR);
        let plain = data.clone();
        obfuscate_instruction_data(&mut data);
        assert_ne!(data, plain);
        obfuscate_instruction_data(&mut data);
        assert_eq!(data, plain);
    }
}
