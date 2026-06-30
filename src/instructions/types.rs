use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::AccountMeta;

use crate::errors::ArbitrageError;
#[cfg(not(feature = "flex"))]
use crate::instructions::program_ids::PUMPFUN_SWAP_FEE_CONFIG_PROGRAM_ID;
use crate::instructions::program_ids::{PUMPFUN_AMM_FEE_CONFIG, PUMPFUN_SWAP_FEE_CONFIG};

pub const RAYDIUM_POOL_V4_AUTHORITY_SEED: &[u8] = b"amm authority";
pub const PUMPFUN_SWAP_CREATOR_VAULT_SEED: &[u8] = b"creator-vault";
pub const PUMPFUN_AMM_CREATOR_VAULT_SEED: &[u8] = b"creator_vault";
pub const PUMPFUN_GLOBAL_VOLUME_ACCUMULATOR_SEED: &[u8] = b"global_volume_accumulator";
pub const PUMPFUN_USER_VOLUME_ACCUMULATOR_SEED: &[u8] = b"user_volume_accumulator";
const RAYDIUM_POOL_V4_NONCE_OFFSET: usize = 8;
const U64_FIELD_LEN: usize = 8;
const PUBKEY_FIELD_LEN: usize = 32;
const PUMPFUN_SWAP_BONDING_CURVE_DISCRIMINATOR: &[u8; 8] = &[23, 183, 248, 55, 96, 216, 172, 96];
const PUMPFUN_SWAP_BONDING_CURVE_CREATOR_OFFSET: usize = 49;
const PUMPFUN_AMM_POOL_DISCRIMINATOR: &[u8; 8] = &[241, 154, 109, 4, 17, 177, 109, 188];
const PUMPFUN_AMM_POOL_CREATOR_OFFSET: usize = 11;
const PUMPFUN_AMM_POOL_BASE_MINT_OFFSET: usize = PUMPFUN_AMM_POOL_CREATOR_OFFSET + PUBKEY_FIELD_LEN;
const PUMPFUN_AMM_POOL_QUOTE_MINT_OFFSET: usize =
    PUMPFUN_AMM_POOL_BASE_MINT_OFFSET + PUBKEY_FIELD_LEN;
const PUMPFUN_AMM_POOL_LP_MINT_OFFSET: usize =
    PUMPFUN_AMM_POOL_QUOTE_MINT_OFFSET + PUBKEY_FIELD_LEN;
const PUMPFUN_AMM_POOL_BASE_TOKEN_ACCOUNT_OFFSET: usize =
    PUMPFUN_AMM_POOL_LP_MINT_OFFSET + PUBKEY_FIELD_LEN;
const PUMPFUN_AMM_POOL_QUOTE_TOKEN_ACCOUNT_OFFSET: usize =
    PUMPFUN_AMM_POOL_BASE_TOKEN_ACCOUNT_OFFSET + PUBKEY_FIELD_LEN;
const PUMPFUN_AMM_POOL_LP_SUPPLY_OFFSET: usize =
    PUMPFUN_AMM_POOL_QUOTE_TOKEN_ACCOUNT_OFFSET + PUBKEY_FIELD_LEN;
const PUMPFUN_AMM_POOL_COIN_CREATOR_OFFSET: usize =
    PUMPFUN_AMM_POOL_LP_SUPPLY_OFFSET + U64_FIELD_LEN;

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

pub fn validate_token_account_for_mint_and_authority<'info>(
    token_account: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    expected_token_program: &AccountInfo<'info>,
    expected_authority: &AccountInfo<'info>,
) -> Result<()> {
    validate_token_account_for_mint(token_account, mint, expected_token_program)?;

    let data = token_account.try_borrow_data()?;
    let token_account_authority = read_token_owner_from_data(&data)?;
    require_keys_eq!(
        token_account_authority,
        expected_authority.key(),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

pub fn validate_raydium_pool_v4_authority<'info>(
    program: &AccountInfo<'info>,
    pool_state: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
) -> Result<()> {
    let data = pool_state.try_borrow_data()?;
    let nonce = read_raydium_pool_v4_nonce_from_data(&data)?;
    let nonce = u8::try_from(nonce).map_err(|_| ArbitrageError::InvalidAccount)?;
    let expected_authority =
        Pubkey::create_program_address(&[RAYDIUM_POOL_V4_AUTHORITY_SEED, &[nonce]], program.key)
            .map_err(|_| ArbitrageError::InvalidAccount)?;
    require_keys_eq!(
        authority.key(),
        expected_authority,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

pub fn validate_pumpfun_swap_semantic_accounts<'info>(
    program: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    step_accounts: &[AccountInfo<'info>],
    direction: u8,
) -> Result<()> {
    let expected_len = match direction {
        0 => 9,
        1 => 11,
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    require!(
        step_accounts.len() == expected_len,
        ArbitrageError::InvalidAccountCount
    );

    let data = step_accounts[3].try_borrow_data()?;
    require!(
        data.starts_with(PUMPFUN_SWAP_BONDING_CURVE_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let creator = read_pubkey_from_data(&data, PUMPFUN_SWAP_BONDING_CURVE_CREATOR_OFFSET)?;
    let expected_creator_vault = Pubkey::find_program_address(
        &[PUMPFUN_SWAP_CREATOR_VAULT_SEED, creator.as_ref()],
        program.key,
    )
    .0;
    require_keys_eq!(
        step_accounts[5].key(),
        expected_creator_vault,
        ArbitrageError::InvalidAccount
    );

    let fee_config_index = expected_len - 2;
    if direction == 1 {
        validate_pumpfun_volume_accounts(program, payer, &step_accounts[7], &step_accounts[8])?;
    }
    validate_pumpfun_fee_config_accounts(
        &step_accounts[fee_config_index],
        &step_accounts[fee_config_index + 1],
        PUMPFUN_SWAP_FEE_CONFIG,
    )
}

pub fn validate_pumpfun_amm_semantic_accounts<'info>(
    program: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    associated_token_program: &AccountInfo<'info>,
    quote_token_program: &AccountInfo<'info>,
    base_mint: &AccountInfo<'info>,
    quote_mint: &AccountInfo<'info>,
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == 12 || step_accounts.len() == 14,
        ArbitrageError::InvalidAccountCount
    );

    let data = step_accounts[1].try_borrow_data()?;
    let pool = read_pumpfun_amm_pool_semantic_keys(&data)?;
    require_keys_eq!(
        pool.base_mint,
        base_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        pool.quote_mint,
        quote_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        pool.pool_base_token_account,
        step_accounts[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        pool.pool_quote_token_account,
        step_accounts[4].key(),
        ArbitrageError::InvalidAccount
    );

    let expected_creator_vault_authority = Pubkey::find_program_address(
        &[PUMPFUN_AMM_CREATOR_VAULT_SEED, pool.coin_creator.as_ref()],
        program.key,
    )
    .0;
    require_keys_eq!(
        step_accounts[9].key(),
        expected_creator_vault_authority,
        ArbitrageError::InvalidAccount
    );

    let expected_creator_vault_ata = associated_token_address_with_program_id(
        &expected_creator_vault_authority,
        quote_mint.key,
        quote_token_program.key,
        associated_token_program.key,
    );
    require_keys_eq!(
        step_accounts[8].key(),
        expected_creator_vault_ata,
        ArbitrageError::InvalidAccount
    );

    let fee_config_index = step_accounts.len() - 2;
    if step_accounts.len() == 14 {
        validate_pumpfun_volume_accounts(program, payer, &step_accounts[10], &step_accounts[11])?;
    }
    validate_pumpfun_fee_config_accounts(
        &step_accounts[fee_config_index],
        &step_accounts[fee_config_index + 1],
        PUMPFUN_AMM_FEE_CONFIG,
    )
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct PumpFunAmmPoolSemanticKeys {
    base_mint: Pubkey,
    quote_mint: Pubkey,
    pool_base_token_account: Pubkey,
    pool_quote_token_account: Pubkey,
    coin_creator: Pubkey,
}

fn read_pumpfun_amm_pool_semantic_keys(data: &[u8]) -> Result<PumpFunAmmPoolSemanticKeys> {
    require!(
        data.starts_with(PUMPFUN_AMM_POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    Ok(PumpFunAmmPoolSemanticKeys {
        base_mint: read_pubkey_from_data(data, PUMPFUN_AMM_POOL_BASE_MINT_OFFSET)?,
        quote_mint: read_pubkey_from_data(data, PUMPFUN_AMM_POOL_QUOTE_MINT_OFFSET)?,
        pool_base_token_account: read_pubkey_from_data(
            data,
            PUMPFUN_AMM_POOL_BASE_TOKEN_ACCOUNT_OFFSET,
        )?,
        pool_quote_token_account: read_pubkey_from_data(
            data,
            PUMPFUN_AMM_POOL_QUOTE_TOKEN_ACCOUNT_OFFSET,
        )?,
        coin_creator: read_pubkey_from_data(data, PUMPFUN_AMM_POOL_COIN_CREATOR_OFFSET)?,
    })
}

fn validate_pumpfun_volume_accounts<'info>(
    program: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    global_volume_accumulator: &AccountInfo<'info>,
    user_volume_accumulator: &AccountInfo<'info>,
) -> Result<()> {
    let expected_global =
        Pubkey::find_program_address(&[PUMPFUN_GLOBAL_VOLUME_ACCUMULATOR_SEED], program.key).0;
    let expected_user = Pubkey::find_program_address(
        &[PUMPFUN_USER_VOLUME_ACCUMULATOR_SEED, payer.key.as_ref()],
        program.key,
    )
    .0;
    require_keys_eq!(
        global_volume_accumulator.key(),
        expected_global,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        user_volume_accumulator.key(),
        expected_user,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_pumpfun_fee_config_accounts<'info>(
    fee_config: &AccountInfo<'info>,
    fee_program: &AccountInfo<'info>,
    expected_fee_config: Pubkey,
) -> Result<()> {
    #[cfg(not(feature = "flex"))]
    {
        require_keys_eq!(
            fee_config.key(),
            expected_fee_config,
            ArbitrageError::InvalidAccount
        );
        require_keys_eq!(
            fee_program.key(),
            PUMPFUN_SWAP_FEE_CONFIG_PROGRAM_ID,
            ArbitrageError::InvalidProgramId
        );
    }

    #[cfg(feature = "flex")]
    {
        let _ = expected_fee_config;
        require!(
            fee_config.key() != Pubkey::default(),
            ArbitrageError::InvalidAccount
        );
        require!(
            fee_program.key() != Pubkey::default(),
            ArbitrageError::InvalidProgramId
        );
    }

    Ok(())
}

fn associated_token_address_with_program_id(
    wallet: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
    associated_token_program: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        associated_token_program,
    )
    .0
}

pub fn read_raydium_pool_v4_nonce_from_data(data: &[u8]) -> Result<u64> {
    let start = RAYDIUM_POOL_V4_NONCE_OFFSET;
    let end = start
        .checked_add(U64_FIELD_LEN)
        .ok_or(ArbitrageError::MathOverflow)?;
    let bytes = data
        .get(start..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
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
    use crate::instructions::program_ids::{
        PUMPFUN_AMM_FEE_CONFIG, PUMPFUN_AMM_PROGRAM_ID, PUMPFUN_SWAP_FEE_CONFIG,
        PUMPFUN_SWAP_FEE_CONFIG_PROGRAM_ID, PUMPFUN_SWAP_PROGRAM_ID,
    };

    fn make_token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> [u8; 72] {
        let mut data = [0u8; 72];
        data[0..32].copy_from_slice(mint.as_ref());
        data[32..64].copy_from_slice(owner.as_ref());
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data
    }

    fn test_account_with_data(
        key: Pubkey,
        owner: Pubkey,
        is_signer: bool,
        is_writable: bool,
        executable: bool,
        data: Vec<u8>,
    ) -> AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(owner));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(data.into_boxed_slice());
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            lamports,
            data,
            owner,
            executable,
            0,
        )
    }

    fn raydium_pool_v4_state_data(nonce: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 16];
        data[0..8].copy_from_slice(&6_u64.to_le_bytes());
        data[8..16].copy_from_slice(&nonce.to_le_bytes());
        data
    }

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + PUBKEY_FIELD_LEN].copy_from_slice(key.as_ref());
    }

    fn pumpfun_swap_bonding_curve_data(creator: Pubkey) -> Vec<u8> {
        let mut data = vec![0_u8; PUMPFUN_SWAP_BONDING_CURVE_CREATOR_OFFSET + PUBKEY_FIELD_LEN];
        data[0..8].copy_from_slice(PUMPFUN_SWAP_BONDING_CURVE_DISCRIMINATOR);
        write_pubkey(
            &mut data,
            PUMPFUN_SWAP_BONDING_CURVE_CREATOR_OFFSET,
            creator,
        );
        data
    }

    fn pumpfun_amm_pool_data(
        base_mint: Pubkey,
        quote_mint: Pubkey,
        pool_base_token_account: Pubkey,
        pool_quote_token_account: Pubkey,
        coin_creator: Pubkey,
    ) -> Vec<u8> {
        let mut data = vec![0_u8; PUMPFUN_AMM_POOL_COIN_CREATOR_OFFSET + PUBKEY_FIELD_LEN];
        data[0..8].copy_from_slice(PUMPFUN_AMM_POOL_DISCRIMINATOR);
        write_pubkey(&mut data, PUMPFUN_AMM_POOL_BASE_MINT_OFFSET, base_mint);
        write_pubkey(&mut data, PUMPFUN_AMM_POOL_QUOTE_MINT_OFFSET, quote_mint);
        write_pubkey(
            &mut data,
            PUMPFUN_AMM_POOL_BASE_TOKEN_ACCOUNT_OFFSET,
            pool_base_token_account,
        );
        write_pubkey(
            &mut data,
            PUMPFUN_AMM_POOL_QUOTE_TOKEN_ACCOUNT_OFFSET,
            pool_quote_token_account,
        );
        write_pubkey(
            &mut data,
            PUMPFUN_AMM_POOL_COIN_CREATOR_OFFSET,
            coin_creator,
        );
        data
    }

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

    #[test]
    fn validates_token_account_owner_and_mint() {
        let token_program_key = Pubkey::new_unique();
        let token_account_key = Pubkey::new_unique();
        let token_owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut token_program_lamports = 0;
        let mut token_program_data = [];
        let mut mint_lamports = 0;
        let mut mint_data = [];
        let mut token_owner_lamports = 0;
        let mut token_owner_data = [];
        let mut token_account_lamports = 0;
        let mut token_account_data = make_token_account_data(mint_key, token_owner_key, 42);

        let token_program = AccountInfo::new(
            &token_program_key,
            false,
            false,
            &mut token_program_lamports,
            &mut token_program_data,
            &token_program_key,
            true,
            0,
        );
        let mint = AccountInfo::new(
            &mint_key,
            false,
            false,
            &mut mint_lamports,
            &mut mint_data,
            &token_program_key,
            false,
            0,
        );
        let token_owner = AccountInfo::new(
            &token_owner_key,
            false,
            false,
            &mut token_owner_lamports,
            &mut token_owner_data,
            &token_program_key,
            false,
            0,
        );
        let token_account = AccountInfo::new(
            &token_account_key,
            false,
            true,
            &mut token_account_lamports,
            &mut token_account_data,
            &token_program_key,
            false,
            0,
        );

        assert!(validate_token_account_for_mint(&token_account, &mint, &token_program).is_ok());
        assert!(validate_token_account_for_mint_and_authority(
            &token_account,
            &mint,
            &token_program,
            &token_owner
        )
        .is_ok());
    }

    #[test]
    fn token_account_validation_rejects_owner_or_mint_mismatch() {
        let token_program_key = Pubkey::new_unique();
        let wrong_program_key = Pubkey::new_unique();
        let token_account_key = Pubkey::new_unique();
        let token_owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let wrong_mint_key = Pubkey::new_unique();
        let mut token_program_lamports = 0;
        let mut token_program_data = [];
        let mut mint_lamports = 0;
        let mut mint_data = [];
        let mut wrong_mint_lamports = 0;
        let mut wrong_mint_data = [];
        let mut token_account_lamports = 0;
        let mut token_account_data = make_token_account_data(mint_key, token_owner_key, 42);

        let token_program = AccountInfo::new(
            &token_program_key,
            false,
            false,
            &mut token_program_lamports,
            &mut token_program_data,
            &token_program_key,
            true,
            0,
        );
        let mint = AccountInfo::new(
            &mint_key,
            false,
            false,
            &mut mint_lamports,
            &mut mint_data,
            &token_program_key,
            false,
            0,
        );
        let wrong_mint = AccountInfo::new(
            &wrong_mint_key,
            false,
            false,
            &mut wrong_mint_lamports,
            &mut wrong_mint_data,
            &token_program_key,
            false,
            0,
        );
        let token_account_wrong_owner = AccountInfo::new(
            &token_account_key,
            false,
            true,
            &mut token_account_lamports,
            &mut token_account_data,
            &wrong_program_key,
            false,
            0,
        );

        let err =
            validate_token_account_for_mint(&token_account_wrong_owner, &mint, &token_program)
                .unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());

        let mut token_account_lamports = 0;
        let mut token_account_data = make_token_account_data(mint_key, token_owner_key, 42);
        let token_account_wrong_mint = AccountInfo::new(
            &token_account_key,
            false,
            true,
            &mut token_account_lamports,
            &mut token_account_data,
            &token_program_key,
            false,
            0,
        );

        let err =
            validate_token_account_for_mint(&token_account_wrong_mint, &wrong_mint, &token_program)
                .unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidTokenMint.into());
    }

    #[test]
    fn token_account_validation_rejects_authority_mismatch() {
        let token_program_key = Pubkey::new_unique();
        let token_account_key = Pubkey::new_unique();
        let token_owner_key = Pubkey::new_unique();
        let wrong_owner_key = Pubkey::new_unique();
        let mint_key = Pubkey::new_unique();
        let mut token_program_lamports = 0;
        let mut token_program_data = [];
        let mut mint_lamports = 0;
        let mut mint_data = [];
        let mut token_owner_lamports = 0;
        let mut token_owner_data = [];
        let mut wrong_owner_lamports = 0;
        let mut wrong_owner_data = [];
        let mut token_account_lamports = 0;
        let mut token_account_data = make_token_account_data(mint_key, token_owner_key, 42);

        let token_program = AccountInfo::new(
            &token_program_key,
            false,
            false,
            &mut token_program_lamports,
            &mut token_program_data,
            &token_program_key,
            true,
            0,
        );
        let mint = AccountInfo::new(
            &mint_key,
            false,
            false,
            &mut mint_lamports,
            &mut mint_data,
            &token_program_key,
            false,
            0,
        );
        let token_owner = AccountInfo::new(
            &token_owner_key,
            false,
            false,
            &mut token_owner_lamports,
            &mut token_owner_data,
            &token_program_key,
            false,
            0,
        );
        let wrong_owner = AccountInfo::new(
            &wrong_owner_key,
            false,
            false,
            &mut wrong_owner_lamports,
            &mut wrong_owner_data,
            &token_program_key,
            false,
            0,
        );
        let token_account = AccountInfo::new(
            &token_account_key,
            false,
            true,
            &mut token_account_lamports,
            &mut token_account_data,
            &token_program_key,
            false,
            0,
        );

        assert!(validate_token_account_for_mint_and_authority(
            &token_account,
            &mint,
            &token_program,
            &token_owner
        )
        .is_ok());
        let err = validate_token_account_for_mint_and_authority(
            &token_account,
            &mint,
            &token_program,
            &wrong_owner,
        )
        .unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn raydium_pool_v4_authority_validation_uses_pool_nonce_pda() {
        let program_id = Pubkey::new_unique();
        let nonce = (0..=u8::MAX)
            .find(|nonce| {
                Pubkey::create_program_address(
                    &[RAYDIUM_POOL_V4_AUTHORITY_SEED, &[*nonce]],
                    &program_id,
                )
                .is_ok()
            })
            .expect("valid nonce");
        let authority = Pubkey::create_program_address(
            &[RAYDIUM_POOL_V4_AUTHORITY_SEED, &[nonce]],
            &program_id,
        )
        .expect("authority pda");
        let program = test_account_with_data(
            program_id,
            Pubkey::new_unique(),
            false,
            false,
            true,
            Vec::new(),
        );
        let pool_state = test_account_with_data(
            Pubkey::new_unique(),
            program_id,
            false,
            true,
            false,
            raydium_pool_v4_state_data(u64::from(nonce)),
        );
        let authority = test_account_with_data(
            authority,
            Pubkey::new_unique(),
            false,
            false,
            false,
            Vec::new(),
        );

        assert!(validate_raydium_pool_v4_authority(&program, &pool_state, &authority).is_ok());
    }

    #[test]
    fn pumpfun_swap_semantic_validation_checks_creator_volume_and_fee_config() {
        let payer_key = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let creator_vault = Pubkey::find_program_address(
            &[PUMPFUN_SWAP_CREATOR_VAULT_SEED, creator.as_ref()],
            &PUMPFUN_SWAP_PROGRAM_ID,
        )
        .0;
        let global_volume = Pubkey::find_program_address(
            &[PUMPFUN_GLOBAL_VOLUME_ACCUMULATOR_SEED],
            &PUMPFUN_SWAP_PROGRAM_ID,
        )
        .0;
        let user_volume = Pubkey::find_program_address(
            &[PUMPFUN_USER_VOLUME_ACCUMULATOR_SEED, payer_key.as_ref()],
            &PUMPFUN_SWAP_PROGRAM_ID,
        )
        .0;

        let program = test_account_with_data(
            PUMPFUN_SWAP_PROGRAM_ID,
            Pubkey::new_unique(),
            false,
            false,
            true,
            vec![],
        );
        let payer =
            test_account_with_data(payer_key, Pubkey::new_unique(), true, true, false, vec![]);
        let step_accounts = vec![
            program.clone(),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                true,
                false,
                pumpfun_swap_bonding_curve_data(creator),
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                creator_vault,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                global_volume,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                user_volume,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                PUMPFUN_SWAP_FEE_CONFIG,
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                PUMPFUN_SWAP_FEE_CONFIG_PROGRAM_ID,
                Pubkey::new_unique(),
                false,
                false,
                true,
                vec![],
            ),
        ];

        assert!(
            validate_pumpfun_swap_semantic_accounts(&program, &payer, &step_accounts, 1).is_ok()
        );

        let mut wrong_user_volume = step_accounts.clone();
        wrong_user_volume[8] = test_account_with_data(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            false,
            true,
            false,
            vec![],
        );
        let err = validate_pumpfun_swap_semantic_accounts(&program, &payer, &wrong_user_volume, 1)
            .unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn pumpfun_amm_semantic_validation_checks_pool_state_and_pdas() {
        let payer_key = Pubkey::new_unique();
        let base_mint_key = Pubkey::new_unique();
        let quote_mint_key = Pubkey::new_unique();
        let base_vault_key = Pubkey::new_unique();
        let quote_vault_key = Pubkey::new_unique();
        let coin_creator = Pubkey::new_unique();
        let associated_token_program_key = Pubkey::new_unique();
        let quote_token_program_key = Pubkey::new_unique();
        let creator_vault_authority = Pubkey::find_program_address(
            &[PUMPFUN_AMM_CREATOR_VAULT_SEED, coin_creator.as_ref()],
            &PUMPFUN_AMM_PROGRAM_ID,
        )
        .0;
        let creator_vault_ata = associated_token_address_with_program_id(
            &creator_vault_authority,
            &quote_mint_key,
            &quote_token_program_key,
            &associated_token_program_key,
        );
        let global_volume = Pubkey::find_program_address(
            &[PUMPFUN_GLOBAL_VOLUME_ACCUMULATOR_SEED],
            &PUMPFUN_AMM_PROGRAM_ID,
        )
        .0;
        let user_volume = Pubkey::find_program_address(
            &[PUMPFUN_USER_VOLUME_ACCUMULATOR_SEED, payer_key.as_ref()],
            &PUMPFUN_AMM_PROGRAM_ID,
        )
        .0;

        let program = test_account_with_data(
            PUMPFUN_AMM_PROGRAM_ID,
            Pubkey::new_unique(),
            false,
            false,
            true,
            vec![],
        );
        let payer =
            test_account_with_data(payer_key, Pubkey::new_unique(), true, true, false, vec![]);
        let associated_token_program = test_account_with_data(
            associated_token_program_key,
            Pubkey::new_unique(),
            false,
            false,
            true,
            vec![],
        );
        let quote_token_program = test_account_with_data(
            quote_token_program_key,
            Pubkey::new_unique(),
            false,
            false,
            true,
            vec![],
        );
        let base_mint = test_account_with_data(
            base_mint_key,
            Pubkey::new_unique(),
            false,
            false,
            false,
            vec![],
        );
        let quote_mint = test_account_with_data(
            quote_mint_key,
            Pubkey::new_unique(),
            false,
            false,
            false,
            vec![],
        );
        let step_accounts = vec![
            program.clone(),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                true,
                false,
                pumpfun_amm_pool_data(
                    base_mint_key,
                    quote_mint_key,
                    base_vault_key,
                    quote_vault_key,
                    coin_creator,
                ),
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                base_vault_key,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                quote_vault_key,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                creator_vault_ata,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                creator_vault_authority,
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                global_volume,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                user_volume,
                Pubkey::new_unique(),
                false,
                true,
                false,
                vec![],
            ),
            test_account_with_data(
                PUMPFUN_AMM_FEE_CONFIG,
                Pubkey::new_unique(),
                false,
                false,
                false,
                vec![],
            ),
            test_account_with_data(
                PUMPFUN_SWAP_FEE_CONFIG_PROGRAM_ID,
                Pubkey::new_unique(),
                false,
                false,
                true,
                vec![],
            ),
        ];

        assert!(validate_pumpfun_amm_semantic_accounts(
            &program,
            &payer,
            &associated_token_program,
            &quote_token_program,
            &base_mint,
            &quote_mint,
            &step_accounts,
        )
        .is_ok());

        let mut wrong_pool_vault = step_accounts.clone();
        wrong_pool_vault[3] = test_account_with_data(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            false,
            true,
            false,
            vec![],
        );
        let err = validate_pumpfun_amm_semantic_accounts(
            &program,
            &payer,
            &associated_token_program,
            &quote_token_program,
            &base_mint,
            &quote_mint,
            &wrong_pool_vault,
        )
        .unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn raydium_pool_v4_authority_validation_rejects_wrong_authority() {
        let program_id = Pubkey::new_unique();
        let program = test_account_with_data(
            program_id,
            Pubkey::new_unique(),
            false,
            false,
            true,
            Vec::new(),
        );
        let pool_state = test_account_with_data(
            Pubkey::new_unique(),
            program_id,
            false,
            true,
            false,
            raydium_pool_v4_state_data(1),
        );
        let authority = test_account_with_data(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            false,
            false,
            false,
            Vec::new(),
        );

        let err =
            validate_raydium_pool_v4_authority(&program, &pool_state, &authority).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }
}
