use anchor_lang::prelude::*;

use crate::errors::ArbitrageError;

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub amount_out: u64,
    pub fee_amount: u64,
}
/// 读取 SPL Token(或Token-2022) 账户的 amount 字段（余额差法）
pub fn read_token_amount<'info>(ai: &AccountInfo<'info>) -> Result<u64> {
    // 至少包含 mint(32) + owner(32) + amount(u64) = 72 字节
    if ai.data_len() < 72 {
        return Err(ArbitrageError::InvalidAccount.into());
    }
    let data = ai.try_borrow_data()?;
    Ok(u64::from_le_bytes(data[64..72].try_into().ok().unwrap()))
}

// 通用工具：读取 token 账户的 mint（前 32 字节）
// fn token_account_mint(ai: &AccountInfo) -> Option<Pubkey> {
//     if ai.data_len() < 32 {
//         return None;
//     }
//     if let Ok(data) = ai.try_borrow_data() {
//         let mut mint_bytes = [0u8; 32];
//         mint_bytes.copy_from_slice(&data[0..32]);
//         return Some(Pubkey::new_from_array(mint_bytes));
//     }
//     None
// }
