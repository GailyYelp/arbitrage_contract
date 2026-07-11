use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const RAYDIUM_STABLE_SWAP_BASE_IN_SELECTOR: &[u8; 1] = &[9];
pub const RAYDIUM_STABLE_SWAP_MIN_ACCOUNTS: usize = 15;

#[derive(Clone)]
pub struct RaydiumStableSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub amm_authority: &'info AccountInfo<'info>,
    pub open_orders: &'info AccountInfo<'info>,
    pub coin_vault: &'info AccountInfo<'info>,
    pub pc_vault: &'info AccountInfo<'info>,
    pub model_data_account: &'info AccountInfo<'info>,
    pub market_program: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub market_bids: &'info AccountInfo<'info>,
    pub market_asks: &'info AccountInfo<'info>,
    pub market_event_queue: &'info AccountInfo<'info>,
    pub market_coin_vault: &'info AccountInfo<'info>,
    pub market_pc_vault: &'info AccountInfo<'info>,
    pub market_vault_signer: &'info AccountInfo<'info>,
    pub input_token_account: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
}

pub fn raydium_stable_swap<'info>(
    accounts: RaydiumStableSwapAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.output_token_account)?;

    let metas = raydium_stable_swap_account_metas(&accounts);
    let account_infos: Vec<AccountInfo<'info>> = vec![
        accounts.token_program.clone(),
        accounts.pool_state.clone(),
        accounts.amm_authority.clone(),
        accounts.open_orders.clone(),
        accounts.coin_vault.clone(),
        accounts.pc_vault.clone(),
        accounts.model_data_account.clone(),
        accounts.market_program.clone(),
        accounts.market.clone(),
        accounts.market_bids.clone(),
        accounts.market_asks.clone(),
        accounts.market_event_queue.clone(),
        accounts.market_coin_vault.clone(),
        accounts.market_pc_vault.clone(),
        accounts.market_vault_signer.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.payer.clone(),
        accounts.program.clone(),
    ];

    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(RAYDIUM_STABLE_SWAP_BASE_IN_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());

    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data,
    };
    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(accounts.output_token_account, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

fn raydium_stable_swap_account_metas(accounts: &RaydiumStableSwapAccounts<'_>) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new(accounts.pool_state.key(), false),
        AccountMeta::new_readonly(accounts.amm_authority.key(), false),
        AccountMeta::new(accounts.open_orders.key(), false),
        AccountMeta::new(accounts.coin_vault.key(), false),
        AccountMeta::new(accounts.pc_vault.key(), false),
        AccountMeta::new_readonly(accounts.model_data_account.key(), false),
        AccountMeta::new_readonly(accounts.market_program.key(), false),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new(accounts.market_bids.key(), false),
        AccountMeta::new(accounts.market_asks.key(), false),
        AccountMeta::new(accounts.market_event_queue.key(), false),
        AccountMeta::new(accounts.market_coin_vault.key(), false),
        AccountMeta::new(accounts.market_pc_vault.key(), false),
        AccountMeta::new_readonly(accounts.market_vault_signer.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pubkey(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn test_account(seed: u8) -> AccountInfo<'static> {
        let key = Box::leak(Box::new(test_pubkey(seed)));
        let owner = Box::leak(Box::new(test_pubkey(seed.wrapping_add(100))));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        AccountInfo::new(key, false, false, lamports, data, owner, false, 0)
    }

    fn test_account_ref(seed: u8) -> &'static AccountInfo<'static> {
        Box::leak(Box::new(test_account(seed)))
    }

    fn test_accounts() -> RaydiumStableSwapAccounts<'static> {
        RaydiumStableSwapAccounts {
            program: test_account_ref(1),
            token_program: test_account_ref(2),
            pool_state: test_account_ref(3),
            amm_authority: test_account_ref(4),
            open_orders: test_account_ref(5),
            coin_vault: test_account_ref(6),
            pc_vault: test_account_ref(7),
            model_data_account: test_account_ref(8),
            market_program: test_account_ref(9),
            market: test_account_ref(10),
            market_bids: test_account_ref(11),
            market_asks: test_account_ref(12),
            market_event_queue: test_account_ref(13),
            market_coin_vault: test_account_ref(14),
            market_pc_vault: test_account_ref(15),
            market_vault_signer: test_account_ref(16),
            input_token_account: test_account_ref(17),
            output_token_account: test_account_ref(18),
            payer: test_account_ref(19),
        }
    }

    #[test]
    fn stable_swap_account_metas_match_official_order() {
        let accounts = test_accounts();
        let metas = raydium_stable_swap_account_metas(&accounts);

        assert_eq!(metas.len(), 18);
        assert_eq!(metas[0].pubkey, accounts.token_program.key());
        assert_eq!(metas[1].pubkey, accounts.pool_state.key());
        assert_eq!(metas[6].pubkey, accounts.model_data_account.key());
        assert_eq!(metas[7].pubkey, accounts.market_program.key());
        assert_eq!(metas[14].pubkey, accounts.market_vault_signer.key());
        assert_eq!(metas[15].pubkey, accounts.input_token_account.key());
        assert_eq!(metas[16].pubkey, accounts.output_token_account.key());
        assert_eq!(metas[17].pubkey, accounts.payer.key());
        assert!(!metas[0].is_writable);
        assert!(metas[1].is_writable);
        assert!(!metas[2].is_writable);
        assert!(!metas[6].is_writable);
        assert!(!metas[7].is_writable);
        assert!(!metas[14].is_writable);
        assert!(metas[17].is_signer);
    }
}
