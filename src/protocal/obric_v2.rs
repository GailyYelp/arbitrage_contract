use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const OBRIC_V2_MIN_ACCOUNTS: usize = 10;
pub const OBRIC_V2_SWAP_V2_DISCRIMINATOR: [u8; 8] =
    [0x41, 0x4b, 0x3f, 0x4c, 0xeb, 0x5b, 0x5b, 0x88];

pub struct ObricV2Accounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub second_reference_oracle: &'info AccountInfo<'info>,
    pub third_reference_oracle: &'info AccountInfo<'info>,
    pub reserve_x: &'info AccountInfo<'info>,
    pub reserve_y: &'info AccountInfo<'info>,
    pub user_x: &'info AccountInfo<'info>,
    pub user_y: &'info AccountInfo<'info>,
    pub reference_oracle: &'info AccountInfo<'info>,
    pub x_price_feed: &'info AccountInfo<'info>,
    pub y_price_feed: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn obric_v2_swap<'info>(
    accounts: ObricV2Accounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new_readonly(accounts.second_reference_oracle.key(), false),
        AccountMeta::new_readonly(accounts.third_reference_oracle.key(), false),
        AccountMeta::new(accounts.reserve_x.key(), false),
        AccountMeta::new(accounts.reserve_y.key(), false),
        AccountMeta::new(accounts.user_x.key(), false),
        AccountMeta::new(accounts.user_y.key(), false),
        AccountMeta::new(accounts.reference_oracle.key(), false),
        AccountMeta::new_readonly(accounts.x_price_feed.key(), false),
        AccountMeta::new_readonly(accounts.y_price_feed.key(), false),
        AccountMeta::new(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let infos = vec![
        accounts.market.clone(),
        accounts.second_reference_oracle.clone(),
        accounts.third_reference_oracle.clone(),
        accounts.reserve_x.clone(),
        accounts.reserve_y.clone(),
        accounts.user_x.clone(),
        accounts.user_y.clone(),
        accounts.reference_oracle.clone(),
        accounts.x_price_feed.clone(),
        accounts.y_price_feed.clone(),
        accounts.payer.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(&OBRIC_V2_SWAP_V2_DISCRIMINATOR);
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
    fn instruction_data_matches_swap_v2_layout() {
        let mut data = Vec::new();
        data.extend_from_slice(&OBRIC_V2_SWAP_V2_DISCRIMINATOR);
        data.push(0);
        data.extend_from_slice(&10_000_000_000_u64.to_le_bytes());
        data.extend_from_slice(&1_u64.to_le_bytes());
        assert_eq!(data.len(), 25);
        assert_eq!(data[8], 0);
        assert_eq!(&data[9..17], &10_000_000_000_u64.to_le_bytes());
    }
}
