use crate::errors::ArbitrageError;
use crate::instructions::types::append_remaining_accounts;
use crate::instructions::types::read_token_amount;
use crate::instructions::types::token_balance_delta;
use crate::instructions::types::SwapResult;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

// buy discriminator
pub const PUMPFUN_AMM_BUY_DISCRIMINATOR: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
// sell discriminator
pub const PUMPFUN_AMM_SELL_DISCRIMINATOR: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
pub const PUMPFUN_AMM_MIN_ACCOUNTS: usize = 10;
pub const PUMPFUN_BUY_LAMPORT_BUFFER: u64 = 2;

#[derive(Clone)]
pub struct PumpFunAmmAccounts<'info> {
    pub pool_state: &'info AccountInfo<'info>, // 1. 池地址 (pool_id)
    pub payer: &'info AccountInfo<'info>,      // 2. payer账户
    pub global_config: &'info AccountInfo<'info>, // 3. global配置地址
    pub base_mint: &'info AccountInfo<'info>,  // 4. 基础代币mint
    pub quote_mint: &'info AccountInfo<'info>, // 5. 计价代币mint
    pub user_base_token_account: &'info AccountInfo<'info>, // 6. 用户基础代币账户
    pub user_quote_token_account: &'info AccountInfo<'info>, // 7. 用户计价代币账户
    pub pool_base_token_account: &'info AccountInfo<'info>, // 8. 池基础代币账户
    pub pool_quote_token_account: &'info AccountInfo<'info>, // 9. 池计价代币账户
    pub fee_recipient: &'info AccountInfo<'info>, // 10. fee_recipient
    pub fee_recipient_ata: &'info AccountInfo<'info>, // 11. fee_recipient_ata
    pub base_token_program: &'info AccountInfo<'info>, // 12. 基础代币program
    pub quote_token_program: &'info AccountInfo<'info>, // 13. 计价代币program
    pub system_program: &'info AccountInfo<'info>, // 14. system_program
    pub associated_token_program: &'info AccountInfo<'info>, // 15. associated_token_program
    pub event_authority: &'info AccountInfo<'info>, // 16. event_authority
    pub program: &'info AccountInfo<'info>,    // 17. program
    pub coin_creator_vault_ata: &'info AccountInfo<'info>, // 18. coin_creator_vault_ata
    pub coin_creator_vault_authority: &'info AccountInfo<'info>, // 19. coin_creator_vault_authority
    pub remaining_accounts: Vec<AccountInfo<'info>>, // 20. remaining accounts (globalVolumeAccumulator + userVolumeAccumulator)
}

pub fn pumpfun_amm_swap<'info>(
    accounts: PumpFunAmmAccounts<'info>,
    direction: u8, // 0: sell, 1: buy
    amount_in: u64,
    total_fee_base_point: u16,
) -> Result<SwapResult> {
    let total_fee_base_point = fee_bps_to_u64(total_fee_base_point);
    validate_total_fee_base_point(total_fee_base_point)?;

    let output_token_account = if direction == 0 {
        accounts.user_quote_token_account // sell
    } else {
        accounts.user_base_token_account // buy
    };
    let pre_out = read_token_amount(output_token_account)?;

    let minimum_amount_out = if direction != 0 {
        // buy
        simulate_buy_amount_by_input(
            amount_in,
            total_fee_base_point,
            accounts.pool_base_token_account,
            accounts.pool_quote_token_account,
        )?
    } else {
        0_u64
    };

    // 账户 metas（参照引擎构造顺序）
    let mut metas = vec![
        AccountMeta::new(accounts.pool_state.key(), false), // pool
        AccountMeta::new(accounts.payer.key(), true),       // user
        AccountMeta::new_readonly(accounts.global_config.key(), false), // global
        AccountMeta::new_readonly(accounts.base_mint.key(), false), // base_mint
        AccountMeta::new_readonly(accounts.quote_mint.key(), false), // quote_mint
        AccountMeta::new(accounts.user_base_token_account.key(), false), // user_base_ata
        AccountMeta::new(accounts.user_quote_token_account.key(), false), // user_quote_ata
        AccountMeta::new(accounts.pool_base_token_account.key(), false), // pool_base_ata
        AccountMeta::new(accounts.pool_quote_token_account.key(), false), // pool_quote_ata
        AccountMeta::new_readonly(accounts.fee_recipient.key(), false), // fee_recipient
        AccountMeta::new(accounts.fee_recipient_ata.key(), false), // fee_recipient_ata
        AccountMeta::new_readonly(accounts.base_token_program.key(), false), // base_token_program
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false), // quote_token_program
        AccountMeta::new_readonly(accounts.system_program.key(), false), // system_program
        AccountMeta::new_readonly(accounts.associated_token_program.key(), false), // associated_token_program
        AccountMeta::new_readonly(accounts.event_authority.key(), false), // event_authority
        AccountMeta::new_readonly(accounts.program.key(), false),         // amm_program
        AccountMeta::new(accounts.coin_creator_vault_ata.key(), false),   // creator_vault_ata
        AccountMeta::new_readonly(accounts.coin_creator_vault_authority.key(), false), // creator_vault_authority
    ];

    let mut account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.pool_state.clone(),
        accounts.payer.clone(),
        accounts.global_config.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.user_base_token_account.clone(),
        accounts.user_quote_token_account.clone(),
        accounts.pool_base_token_account.clone(),
        accounts.pool_quote_token_account.clone(),
        accounts.fee_recipient.clone(),
        accounts.fee_recipient_ata.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.system_program.clone(),
        accounts.associated_token_program.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
        accounts.coin_creator_vault_ata.clone(),
        accounts.coin_creator_vault_authority.clone(),
    ];

    // 动态补充：从 remaining_accounts 追加与 PumpFunAMM 程序相关且不在基础集中的账户（例如 global_volume_accumulator + user_volume_accumulator)
    append_remaining_accounts(&mut metas, &mut account_infos, accounts.remaining_accounts);
    account_infos.push(accounts.program.clone());

    // 构造 data
    let mut data = Vec::with_capacity(8 + 8 + 8);
    if direction == 0 {
        // sell
        data.extend_from_slice(PUMPFUN_AMM_SELL_DISCRIMINATOR);
        data.extend_from_slice(&amount_in.to_le_bytes()); // amount_in
        data.extend_from_slice(&minimum_amount_out.to_le_bytes()); // min_sol_output
    } else {
        // buy
        data.extend_from_slice(PUMPFUN_AMM_BUY_DISCRIMINATOR);
        data.extend_from_slice(&minimum_amount_out.to_le_bytes()); // amount_out
        data.extend_from_slice(&amount_in.to_le_bytes()); // max_sol_cost
    }

    let program_id = accounts.program.key();
    let ix = Instruction {
        program_id,
        accounts: metas,
        data,
    };

    // Invoke
    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(output_token_account, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

pub fn simulate_buy_amount_by_input<'info>(
    input_amount: u64,
    total_fee_base_point: u64,
    pool_base_token_account: &'info AccountInfo<'info>,
    pool_quote_token_account: &'info AccountInfo<'info>,
) -> Result<u64> {
    if input_amount == 0 {
        return Ok(0_u64);
    }

    let base_amount = read_token_amount(pool_base_token_account)?;
    let quote_amout = read_token_amount(pool_quote_token_account)?;
    let effective_input_amount = pumpfun_buy_effective_input_amount(input_amount)?;

    // simulate
    let output_amount = simulate_swap_base_input(
        base_amount,
        quote_amout,
        total_fee_base_point,
        effective_input_amount,
    )?;
    Ok(output_amount)
}

pub fn pumpfun_buy_effective_input_amount(input_amount: u64) -> Result<u64> {
    if input_amount == 0 {
        return Ok(0);
    }

    let effective_input_amount = input_amount
        .checked_sub(PUMPFUN_BUY_LAMPORT_BUFFER)
        .ok_or(ArbitrageError::InvalidAmount)?;
    require!(effective_input_amount > 0, ArbitrageError::InvalidAmount);
    Ok(effective_input_amount)
}

pub fn validate_total_fee_base_point(total_fee_base_point: u64) -> Result<()> {
    if total_fee_base_point >= 10_000 {
        return Err(ArbitrageError::FeeTooHigh.into());
    }
    Ok(())
}

pub fn fee_bps_to_u64(total_fee_base_point: u16) -> u64 {
    u64::from(total_fee_base_point)
}

// 除法向上取整
pub fn div_up(a: u128, b: u128) -> Option<u128> {
    if b == 0 {
        return None;
    }
    a.checked_add(b.checked_sub(1)?)?.checked_div(b)
}

pub fn swap_base_input(
    source_amount: u128,           // 输入数量
    swap_source_amount: u128,      // 池中源代币数量
    swap_destination_amount: u128, // 池中目标代币数量
) -> Option<u128> {
    // 公式: Δy = (Δx * y) / (x + Δx)
    let numerator = source_amount.checked_mul(swap_destination_amount)?;
    let denominator = swap_source_amount.checked_add(source_amount)?;
    let destinsation_amount_swapped = numerator.checked_div(denominator)?;
    Some(destinsation_amount_swapped)
}

pub fn simulate_swap_base_input(
    x: u64,
    y: u64,
    total_fee_base_point: u64,
    input_amount: u64,
) -> Result<u64> {
    validate_total_fee_base_point(total_fee_base_point)?;

    // msg!("x: {:?}, y: {:?}", x, y);
    // 计算手续费
    let input_amount_without_fee = div_up(
        u128::from(input_amount)
            .checked_mul(10000)
            .ok_or(ArbitrageError::MathOverflow)?,
        10000u128
            .checked_add(u128::from(total_fee_base_point))
            .ok_or(ArbitrageError::MathOverflow)?,
    )
    .ok_or(ArbitrageError::MathOverflow)?;
    // msg!("input_amount_without_fee: {:?}", input_amount_without_fee);
    let total_fee = div_up(
        input_amount_without_fee
            .checked_mul(u128::from(total_fee_base_point))
            .ok_or(ArbitrageError::MathOverflow)?,
        10000,
    )
    .ok_or(ArbitrageError::MathOverflow)?;
    let input_amount_without_fee = u128::from(input_amount)
        .checked_sub(total_fee)
        .ok_or(ArbitrageError::MathOverflow)?;
    // msg!("total_fee: {:?}", total_fee);
    // msg!("input_amount_without_fee: {:?}", input_amount_without_fee);
    let output_amount = swap_base_input(input_amount_without_fee, u128::from(y), u128::from(x))
        .ok_or(ArbitrageError::MathOverflow)?;
    // msg!("output_amount: {:?}", output_amount);
    u64::try_from(output_amount).map_err(|_| ArbitrageError::MathOverflow.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div_up_handles_zero_denominator() {
        assert_eq!(div_up(1, 0), None);
    }

    #[test]
    fn simulate_swap_handles_tiny_input_without_underflow() {
        assert_eq!(simulate_swap_base_input(1_000, 1_000, 100, 1).unwrap(), 0);
    }

    #[test]
    fn pumpfun_buy_effective_input_requires_buffer_plus_swap_amount() {
        assert_eq!(pumpfun_buy_effective_input_amount(0).unwrap(), 0);
        assert!(pumpfun_buy_effective_input_amount(1).is_err());
        assert!(pumpfun_buy_effective_input_amount(2).is_err());
        assert_eq!(pumpfun_buy_effective_input_amount(3).unwrap(), 1);
    }

    #[test]
    fn simulate_swap_uses_checked_large_math() {
        let out = simulate_swap_base_input(u64::MAX, u64::MAX, 100, u64::MAX).unwrap();
        assert!(out > 0);
    }

    #[test]
    fn pumpfun_total_fee_rejects_bps_denominator_or_higher() {
        assert!(validate_total_fee_base_point(9_999).is_ok());
        assert!(validate_total_fee_base_point(10_000).is_err());
        assert!(validate_total_fee_base_point(u64::MAX).is_err());
    }

    #[test]
    fn pumpfun_fee_bps_uses_lossless_conversion_helper() {
        assert_eq!(fee_bps_to_u64(9_999), 9_999);

        for (name, source) in [
            ("pumpfun_amm", include_str!("pumpfun_amm.rs")),
            ("pumpfun_swap", include_str!("pumpfun_swap.rs")),
        ] {
            let production = production_source_text(source);
            assert!(
                !production.contains("total_fee_base_point as u64"),
                "{name} fee bps conversion must use fee_bps_to_u64"
            );
        }
    }

    #[test]
    fn simulate_swap_rejects_invalid_fee_instead_of_zero_output() {
        assert!(simulate_swap_base_input(1_000_000, 2_000_000, 10_000, 100_000).is_err());
        assert!(simulate_swap_base_input(1_000_000, 2_000_000, u64::MAX, 100_000).is_err());
    }

    #[test]
    fn simulate_swap_output_never_exceeds_destination_reserve_for_sampled_boundaries() {
        let reserves = [1_u64, 2, 1_000, u64::MAX];
        let fees = [0_u64, 1, 9_999];
        let inputs = [1_u64, 2, 10_000, u64::MAX];

        for x in reserves {
            for y in reserves {
                for fee in fees {
                    for input in inputs {
                        let output =
                            simulate_swap_base_input(x, y, fee, input).expect("valid fee quote");
                        assert!(output <= x);
                    }
                }
            }
        }
    }

    fn production_source_text(source: &str) -> &str {
        match source.split_once("\n#[cfg(test)]") {
            Some((production, _)) => production,
            None => source,
        }
    }
}
