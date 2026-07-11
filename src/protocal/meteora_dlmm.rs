use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

pub const METEORA_DLMM_SWAP2_SELECTOR: &[u8; 8] = &[65, 75, 63, 76, 235, 91, 91, 136];
pub const METEORA_DLMM_MIN_ACCOUNTS: usize = 13;
pub const METEORA_DLMM_FIXED_STEP_ACCOUNTS: usize = 12;

#[derive(Clone)]
pub struct MeteoraDlmmAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub token_x_program: &'info AccountInfo<'info>,
    pub token_y_program: &'info AccountInfo<'info>,
    pub memo_program: &'info AccountInfo<'info>,
    pub lb_pair: &'info AccountInfo<'info>,
    pub bin_array_bitmap_extension: &'info AccountInfo<'info>,
    pub reserve_x: &'info AccountInfo<'info>,
    pub reserve_y: &'info AccountInfo<'info>,
    pub user_token_in: &'info AccountInfo<'info>,
    pub user_token_out: &'info AccountInfo<'info>,
    pub token_x_mint: &'info AccountInfo<'info>,
    pub token_y_mint: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub bin_arrays: Vec<AccountInfo<'info>>,
}

pub fn meteora_dlmm_swap<'info>(
    accounts: MeteoraDlmmAccounts<'info>,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<SwapResult> {
    let pre_out = read_token_amount(accounts.user_token_out)?;

    let metas = meteora_dlmm_swap2_metas(&accounts);
    let account_infos = meteora_dlmm_swap2_account_infos(&accounts);
    let data = meteora_dlmm_swap2_data(amount_in, minimum_amount_out);

    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data,
    };

    invoke(&ix, &account_infos)?;

    let amount_out = token_balance_delta(accounts.user_token_out, pre_out)?;
    Ok(SwapResult {
        amount_out,
        fee_amount: 0,
    })
}

fn meteora_dlmm_swap2_metas(accounts: &MeteoraDlmmAccounts<'_>) -> Vec<AccountMeta> {
    let mut metas = Vec::with_capacity(16 + accounts.bin_arrays.len());
    metas.extend([
        AccountMeta::new(accounts.lb_pair.key(), false),
        bitmap_extension_meta(accounts),
        AccountMeta::new(accounts.reserve_x.key(), false),
        AccountMeta::new(accounts.reserve_y.key(), false),
        AccountMeta::new(accounts.user_token_in.key(), false),
        AccountMeta::new(accounts.user_token_out.key(), false),
        AccountMeta::new_readonly(accounts.token_x_mint.key(), false),
        AccountMeta::new_readonly(accounts.token_y_mint.key(), false),
        AccountMeta::new(accounts.oracle.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new_readonly(accounts.token_x_program.key(), false),
        AccountMeta::new_readonly(accounts.token_y_program.key(), false),
        AccountMeta::new_readonly(accounts.memo_program.key(), false),
        AccountMeta::new_readonly(accounts.event_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
    ]);
    metas.extend(
        accounts
            .bin_arrays
            .iter()
            .map(|account| AccountMeta::new(account.key(), false)),
    );
    metas
}

fn bitmap_extension_meta(accounts: &MeteoraDlmmAccounts<'_>) -> AccountMeta {
    if accounts.bin_array_bitmap_extension.key() == accounts.program.key() {
        AccountMeta::new_readonly(accounts.bin_array_bitmap_extension.key(), false)
    } else {
        AccountMeta::new(accounts.bin_array_bitmap_extension.key(), false)
    }
}

fn meteora_dlmm_swap2_account_infos<'info>(
    accounts: &MeteoraDlmmAccounts<'info>,
) -> Vec<AccountInfo<'info>> {
    let mut account_infos = Vec::with_capacity(16 + accounts.bin_arrays.len());
    account_infos.extend([
        accounts.lb_pair.clone(),
        accounts.bin_array_bitmap_extension.clone(),
        accounts.reserve_x.clone(),
        accounts.reserve_y.clone(),
        accounts.user_token_in.clone(),
        accounts.user_token_out.clone(),
        accounts.token_x_mint.clone(),
        accounts.token_y_mint.clone(),
        accounts.oracle.clone(),
        accounts.program.clone(),
        accounts.payer.clone(),
        accounts.token_x_program.clone(),
        accounts.token_y_program.clone(),
        accounts.memo_program.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
    ]);
    account_infos.extend(accounts.bin_arrays.iter().cloned());
    account_infos
}

fn meteora_dlmm_swap2_data(amount_in: u64, minimum_amount_out: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(28);
    data.extend_from_slice(METEORA_DLMM_SWAP2_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&0_u32.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pubkey(seed: u8) -> Pubkey {
        Pubkey::new_from_array([seed; 32])
    }

    fn test_account(key: Pubkey) -> &'static AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(test_pubkey(200)));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        Box::leak(Box::new(AccountInfo::new(
            key, false, false, lamports, data, owner, false, 0,
        )))
    }

    #[test]
    fn swap2_data_layout_matches_meteora_dlmm_contract() {
        let data = meteora_dlmm_swap2_data(100, 90);

        assert_eq!(data.len(), 28);
        assert_eq!(&data[0..8], METEORA_DLMM_SWAP2_SELECTOR);
        assert_eq!(&data[8..16], &100_u64.to_le_bytes());
        assert_eq!(&data[16..24], &90_u64.to_le_bytes());
        assert_eq!(&data[24..28], &0_u32.to_le_bytes());
    }

    #[test]
    fn bitmap_placeholder_meta_is_readonly() {
        let program = test_account(test_pubkey(1));
        let payer = test_account(test_pubkey(2));
        let token_x_program = test_account(test_pubkey(3));
        let token_y_program = test_account(test_pubkey(4));
        let memo_program = test_account(test_pubkey(5));
        let lb_pair = test_account(test_pubkey(6));
        let reserve_x = test_account(test_pubkey(7));
        let reserve_y = test_account(test_pubkey(8));
        let user_token_in = test_account(test_pubkey(9));
        let user_token_out = test_account(test_pubkey(10));
        let token_x_mint = test_account(test_pubkey(11));
        let token_y_mint = test_account(test_pubkey(12));
        let oracle = test_account(test_pubkey(13));
        let event_authority = test_account(test_pubkey(14));
        let real_bitmap = test_account(test_pubkey(15));

        let placeholder_accounts = MeteoraDlmmAccounts {
            program,
            payer,
            token_x_program,
            token_y_program,
            memo_program,
            lb_pair,
            bin_array_bitmap_extension: program,
            reserve_x,
            reserve_y,
            user_token_in,
            user_token_out,
            token_x_mint,
            token_y_mint,
            oracle,
            event_authority,
            bin_arrays: Vec::new(),
        };
        let placeholder_meta = bitmap_extension_meta(&placeholder_accounts);
        assert_eq!(placeholder_meta.pubkey, program.key());
        assert!(!placeholder_meta.is_writable);

        let real_bitmap_accounts = MeteoraDlmmAccounts {
            bin_array_bitmap_extension: real_bitmap,
            ..placeholder_accounts
        };
        let real_bitmap_meta = bitmap_extension_meta(&real_bitmap_accounts);
        assert_eq!(real_bitmap_meta.pubkey, real_bitmap.key());
        assert!(real_bitmap_meta.is_writable);
    }
}
