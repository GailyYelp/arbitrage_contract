use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};

pub const OPENBOOK_V2_MIN_ACCOUNTS: usize = 10;
pub const OPENBOOK_V2_PLACE_TAKE_ORDER_DISCRIMINATOR: [u8; 8] = [3, 44, 71, 3, 26, 199, 203, 85];
pub const OPENBOOK_V2_PLACE_TAKE_ORDER_DATA_LEN: usize = 35;
pub const OPENBOOK_V2_DEFAULT_MATCH_LIMIT: u8 = 45;
pub const OPENBOOK_V2_FEE_SCALE: u128 = 1_000_000;

#[derive(Clone)]
pub struct OpenBookV2SwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub market_authority: &'info AccountInfo<'info>,
    pub bids: &'info AccountInfo<'info>,
    pub asks: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub event_heap: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
}

#[allow(clippy::too_many_arguments)]
pub fn openbook_v2_swap<'info>(
    accounts: OpenBookV2SwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_in: bool,
    base_lot_size: u64,
    quote_lot_size: u64,
    maker_rebate_rate: u64,
    taker_fee: u64,
) -> Result<SwapResult> {
    let output_account = if is_base_in {
        accounts.user_quote
    } else {
        accounts.user_base
    };
    let pre_out = read_token_amount(output_account)?;
    let data = openbook_v2_swap_data(
        amount_in,
        minimum_amount_out,
        is_base_in,
        base_lot_size,
        quote_lot_size,
        maker_rebate_rate,
        taker_fee,
    )?;
    let (metas, infos) = openbook_v2_swap_accounts(&accounts);
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

#[allow(clippy::too_many_arguments)]
fn openbook_v2_swap_data(
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_in: bool,
    base_lot_size: u64,
    quote_lot_size: u64,
    maker_rebate_rate: u64,
    taker_fee: u64,
) -> Result<Vec<u8>> {
    require!(base_lot_size > 0, ArbitrageError::InvalidAmount);
    require!(quote_lot_size > 0, ArbitrageError::InvalidAmount);
    require!(
        u128::from(maker_rebate_rate) <= OPENBOOK_V2_FEE_SCALE,
        ArbitrageError::InvalidAmount
    );
    require!(
        u128::from(taker_fee) <= OPENBOOK_V2_FEE_SCALE,
        ArbitrageError::InvalidAmount
    );

    let (side, price_lots, max_base_lots, max_quote_lots) = if is_base_in {
        let base_lots = floor_nonzero_lots(amount_in, base_lot_size)?;
        let max_quote_lots = i64::MAX
            .checked_div(i64::try_from(quote_lot_size).map_err(|_| ArbitrageError::MathOverflow)?)
            .ok_or(ArbitrageError::MathOverflow)?;
        let net_scale = OPENBOOK_V2_FEE_SCALE
            .checked_sub(u128::from(maker_rebate_rate))
            .ok_or(ArbitrageError::MathOverflow)?;
        require!(net_scale > 0, ArbitrageError::InvalidAmount);
        let gross_required = div_ceil(
            u128::from(minimum_amount_out)
                .checked_mul(OPENBOOK_V2_FEE_SCALE)
                .ok_or(ArbitrageError::MathOverflow)?,
            net_scale,
        )?;
        let denominator = u128::from(base_lots)
            .checked_mul(u128::from(quote_lot_size))
            .ok_or(ArbitrageError::MathOverflow)?;
        let price = div_ceil(gross_required, denominator)?.max(1);
        (
            1_u8,
            positive_i64(price)?,
            i64::try_from(base_lots).map_err(|_| ArbitrageError::MathOverflow)?,
            max_quote_lots,
        )
    } else {
        let quote_lots = floor_nonzero_lots(amount_in, quote_lot_size)?;
        let max_base_lots = i64::MAX
            .checked_div(i64::try_from(base_lot_size).map_err(|_| ArbitrageError::MathOverflow)?)
            .ok_or(ArbitrageError::MathOverflow)?;
        let min_base_lots = div_ceil(u128::from(minimum_amount_out), u128::from(base_lot_size))?;
        let price = if min_base_lots == 0 {
            i64::MAX
        } else {
            let spendable = u128::from(quote_lots)
                .checked_mul(OPENBOOK_V2_FEE_SCALE)
                .and_then(|value| {
                    value.checked_div(OPENBOOK_V2_FEE_SCALE.checked_add(u128::from(taker_fee))?)
                })
                .ok_or(ArbitrageError::MathOverflow)?;
            positive_i64(
                spendable
                    .checked_div(min_base_lots)
                    .ok_or(ArbitrageError::MathOverflow)?
                    .max(1),
            )?
        };
        (
            0_u8,
            price,
            max_base_lots,
            i64::try_from(quote_lots).map_err(|_| ArbitrageError::MathOverflow)?,
        )
    };

    let mut data = Vec::with_capacity(OPENBOOK_V2_PLACE_TAKE_ORDER_DATA_LEN);
    data.extend_from_slice(&OPENBOOK_V2_PLACE_TAKE_ORDER_DISCRIMINATOR);
    data.push(side);
    data.extend_from_slice(&price_lots.to_le_bytes());
    data.extend_from_slice(&max_base_lots.to_le_bytes());
    data.extend_from_slice(&max_quote_lots.to_le_bytes());
    data.push(1); // PlaceOrderType::ImmediateOrCancel
    data.push(OPENBOOK_V2_DEFAULT_MATCH_LIMIT);
    require!(
        data.len() == OPENBOOK_V2_PLACE_TAKE_ORDER_DATA_LEN,
        ArbitrageError::InvalidInstructionData
    );
    Ok(data)
}

fn openbook_v2_swap_accounts<'info>(
    accounts: &OpenBookV2SwapAccounts<'info>,
) -> (Vec<AccountMeta>, Vec<AccountInfo<'info>>) {
    let program = accounts.program.key();
    (
        vec![
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new(accounts.market.key(), false),
            AccountMeta::new_readonly(accounts.market_authority.key(), false),
            AccountMeta::new(accounts.bids.key(), false),
            AccountMeta::new(accounts.asks.key(), false),
            AccountMeta::new(accounts.base_vault.key(), false),
            AccountMeta::new(accounts.quote_vault.key(), false),
            AccountMeta::new(accounts.event_heap.key(), false),
            AccountMeta::new(accounts.user_base.key(), false),
            AccountMeta::new(accounts.user_quote.key(), false),
            AccountMeta::new_readonly(program, false),
            AccountMeta::new_readonly(program, false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(accounts.system_program.key(), false),
            AccountMeta::new_readonly(program, false),
        ],
        vec![
            accounts.payer.clone(),
            accounts.payer.clone(),
            accounts.market.clone(),
            accounts.market_authority.clone(),
            accounts.bids.clone(),
            accounts.asks.clone(),
            accounts.base_vault.clone(),
            accounts.quote_vault.clone(),
            accounts.event_heap.clone(),
            accounts.user_base.clone(),
            accounts.user_quote.clone(),
            accounts.program.clone(),
            accounts.program.clone(),
            accounts.token_program.clone(),
            accounts.system_program.clone(),
            accounts.program.clone(),
        ],
    )
}

fn floor_nonzero_lots(atoms: u64, atoms_per_lot: u64) -> Result<u64> {
    let lots = atoms
        .checked_div(atoms_per_lot)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(lots > 0, ArbitrageError::InvalidAmount);
    Ok(lots)
}

fn div_ceil(value: u128, divisor: u128) -> Result<u128> {
    require!(divisor > 0, ArbitrageError::InvalidAmount);
    let quotient = value
        .checked_div(divisor)
        .ok_or(ArbitrageError::MathOverflow)?;
    if value
        .checked_rem(divisor)
        .ok_or(ArbitrageError::MathOverflow)?
        == 0
    {
        Ok(quotient)
    } else {
        quotient
            .checked_add(1)
            .ok_or_else(|| ArbitrageError::MathOverflow.into())
    }
}

fn positive_i64(value: u128) -> Result<i64> {
    require!(value > 0, ArbitrageError::InvalidAmount);
    i64::try_from(value).map_err(|_| ArbitrageError::MathOverflow.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_take_order_data_matches_official_borsh_layout() {
        let sell =
            openbook_v2_swap_data(60, 10_000, true, 10, 1_000, 1_000, 2_000).expect("sell data");
        assert_eq!(sell.len(), 35);
        assert_eq!(&sell[..8], &OPENBOOK_V2_PLACE_TAKE_ORDER_DISCRIMINATOR);
        assert_eq!(sell[8], 1);
        assert_eq!(&sell[17..25], &6_i64.to_le_bytes());
        assert_eq!(sell[33], 1);
        assert_eq!(sell[34], 45);

        let buy =
            openbook_v2_swap_data(20_000, 40, false, 10, 1_000, 1_000, 2_000).expect("buy data");
        assert_eq!(buy[8], 0);
        assert_eq!(&buy[25..33], &20_i64.to_le_bytes());
    }
}
