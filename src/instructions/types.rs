use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::AccountMeta;

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
    read_token_amount_from_data(&data)
}

pub fn validate_token_account_for_mint<'info>(
    token_account: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    expected_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        token_account.owner.key(),
        expected_token_program.key(),
        ArbitrageError::InvalidAccount
    );

    let data = token_account.try_borrow_data()?;
    let token_account_mint = read_token_mint_from_data(&data)?;
    require_keys_eq!(
        token_account_mint,
        mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    Ok(())
}

pub fn token_program_for_mint<'a, 'info>(
    mint: &AccountInfo<'info>,
    token_program: &'a AccountInfo<'info>,
    token_2022_program: &'a AccountInfo<'info>,
) -> Result<&'a AccountInfo<'info>> {
    if mint.owner.key() == token_program.key() {
        Ok(token_program)
    } else if mint.owner.key() == token_2022_program.key() {
        Ok(token_2022_program)
    } else {
        Err(ArbitrageError::InvalidTokenMint.into())
    }
}

pub fn read_token_amount_from_data(data: &[u8]) -> Result<u64> {
    let amount_bytes = data.get(64..72).ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(
        amount_bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

pub fn read_token_mint_from_data(data: &[u8]) -> Result<Pubkey> {
    read_pubkey_from_data(data, 0)
}

pub fn read_token_owner_from_data(data: &[u8]) -> Result<Pubkey> {
    read_pubkey_from_data(data, 32)
}

fn read_pubkey_from_data(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let key_bytes = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut key = [0u8; 32];
    key.copy_from_slice(key_bytes);
    Ok(Pubkey::new_from_array(key))
}

pub fn token_balance_delta<'info>(account: &AccountInfo<'info>, pre_amount: u64) -> Result<u64> {
    let post_amount = read_token_amount(account)?;
    Ok(post_amount.saturating_sub(pre_amount))
}

pub fn append_remaining_accounts<'info>(
    metas: &mut Vec<AccountMeta>,
    account_infos: &mut Vec<AccountInfo<'info>>,
    remaining_accounts: Vec<AccountInfo<'info>>,
) {
    for ai in remaining_accounts {
        if ai.is_writable {
            metas.push(AccountMeta::new(ai.key(), false));
        } else {
            metas.push(AccountMeta::new_readonly(ai.key(), false));
        }
        account_infos.push(ai);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_token_amount_from_canonical_offset() {
        let mut data = [0u8; 72];
        data[64..72].copy_from_slice(&42u64.to_le_bytes());

        assert_eq!(read_token_amount_from_data(&data).unwrap(), 42);
    }

    #[test]
    fn reads_token_mint_and_owner_from_canonical_offsets() {
        let mint = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mut data = [0u8; 72];
        data[0..32].copy_from_slice(mint.as_ref());
        data[32..64].copy_from_slice(owner.as_ref());

        assert_eq!(read_token_mint_from_data(&data).unwrap(), mint);
        assert_eq!(read_token_owner_from_data(&data).unwrap(), owner);
    }

    #[test]
    fn rejects_short_token_account_data() {
        assert!(read_token_amount_from_data(&[0u8; 71]).is_err());
        assert!(read_token_mint_from_data(&[0u8; 31]).is_err());
        assert!(read_token_owner_from_data(&[0u8; 63]).is_err());
    }
}
