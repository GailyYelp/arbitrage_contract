use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::CARROT_PROGRAM_ID,
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const CARROT_STEP_ACCOUNTS: usize = 15;
const CARROT_VAULT: Pubkey = anchor_lang::pubkey!("FfCRL34rkJiMiX5emNDrYp3MdWH2mES3FvDQyFppqgpJ");
const CARROT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("CarrotLYPhQzYL4fEsTUvEzw5QDaMGSZUENHSkh7qzQa");
const CRT_MINT: Pubkey = anchor_lang::pubkey!("CRTx1JouZhzSU6XytsE42UQraoGqiHgxabocVfARTy2s");
const USDC_MINT: Pubkey = anchor_lang::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const USDT_MINT: Pubkey = anchor_lang::pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
const PYUSD_MINT: Pubkey = anchor_lang::pubkey!("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo");
const USDC_VAULT: Pubkey = anchor_lang::pubkey!("Gfedc4JEmMahEMBJXcXfLHWgNs9d7UzLPq1tkba5S11U");
const USDT_VAULT: Pubkey = anchor_lang::pubkey!("Hpxgqa8dvk2jSfNgTfdYncxSE2YY2c52TTzPaH1V98RW");
const PYUSD_VAULT: Pubkey = anchor_lang::pubkey!("4cugtfkFydmoPe9CZJ4wFZzDUEmGJFNaThvumYABTFDS");
const USDC_ORACLE: Pubkey = anchor_lang::pubkey!("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX");
const USDT_ORACLE: Pubkey = anchor_lang::pubkey!("HT2PLQBcG5EiCcNSaMHAjSgd9F98ecpATbk4Sk5oYuM");
const PYUSD_ORACLE: Pubkey = anchor_lang::pubkey!("9zXQxpYH3kYhtoybmZfUNNCRVuud7fY9jswTg1hLyT8k");
const CARROT_LOG_PROGRAM: Pubkey =
    anchor_lang::pubkey!("7Mc3vSdRWoThArpni6t5W4XjvQf4BuMny1uC8b6VBn48");
const PYTH_RECEIVER_PROGRAM: Pubkey =
    anchor_lang::pubkey!("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ");
const TOKEN_PROGRAM: Pubkey = anchor_lang::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_PROGRAM: Pubkey =
    anchor_lang::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const SYSTEM_PROGRAM: Pubkey = anchor_lang::pubkey!("11111111111111111111111111111111");
const VAULT_DISCRIMINATOR: [u8; 8] = [0xd3, 0x08, 0xe8, 0x2b, 0x02, 0x98, 0x75, 0x77];
const PRICE_UPDATE_DISCRIMINATOR: [u8; 8] = [0x22, 0xf1, 0x23, 0x63, 0x9d, 0x7e, 0xf4, 0xcd];
const ISSUE_DISCRIMINATOR: [u8; 8] = [0xbe, 0x01, 0x62, 0xd6, 0x51, 0x63, 0xde, 0xf7];
const REDEEM_DISCRIMINATOR: [u8; 8] = [0xb8, 0x0c, 0x56, 0x95, 0x46, 0xc4, 0x61, 0xe1];
const ASSET_VECTOR_OFFSET: usize = 111;
const ASSET_RECORD_LEN: usize = 99;

pub struct CarrotAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub vault: &'info AccountInfo<'info>,
    pub shares_mint: &'info AccountInfo<'info>,
    pub user_shares: &'info AccountInfo<'info>,
    pub asset_mint: &'info AccountInfo<'info>,
    pub asset_vault: &'info AccountInfo<'info>,
    pub user_asset: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub asset_token_program: &'info AccountInfo<'info>,
    pub shares_token_program: &'info AccountInfo<'info>,
    pub log_program: &'info AccountInfo<'info>,
    pub oracles: [&'info AccountInfo<'info>; 3],
    pub reserve_vaults: [&'info AccountInfo<'info>; 3],
}

pub fn carrot_swap<'info>(
    accounts: CarrotAccounts<'info>,
    amount_in: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidPath);
    let output_account = if direction == 0 {
        accounts.user_shares
    } else {
        accounts.user_asset
    };
    let pre_out = read_token_amount(output_account)?;
    let metas = vec![
        AccountMeta::new(accounts.vault.key(), false),
        AccountMeta::new(accounts.shares_mint.key(), false),
        AccountMeta::new(accounts.user_shares.key(), false),
        AccountMeta::new_readonly(accounts.asset_mint.key(), false),
        AccountMeta::new(accounts.asset_vault.key(), false),
        AccountMeta::new(accounts.user_asset.key(), false),
        AccountMeta::new(accounts.user.key(), true),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.asset_token_program.key(), false),
        AccountMeta::new_readonly(accounts.shares_token_program.key(), false),
        AccountMeta::new_readonly(accounts.log_program.key(), false),
        AccountMeta::new_readonly(accounts.oracles[0].key(), false),
        AccountMeta::new_readonly(accounts.oracles[1].key(), false),
        AccountMeta::new_readonly(accounts.oracles[2].key(), false),
        AccountMeta::new_readonly(accounts.reserve_vaults[0].key(), false),
        AccountMeta::new_readonly(accounts.reserve_vaults[1].key(), false),
        AccountMeta::new_readonly(accounts.reserve_vaults[2].key(), false),
    ];
    let account_infos = vec![
        accounts.vault.clone(),
        accounts.shares_mint.clone(),
        accounts.user_shares.clone(),
        accounts.asset_mint.clone(),
        accounts.asset_vault.clone(),
        accounts.user_asset.clone(),
        accounts.user.clone(),
        accounts.system_program.clone(),
        accounts.asset_token_program.clone(),
        accounts.shares_token_program.clone(),
        accounts.log_program.clone(),
        accounts.oracles[0].clone(),
        accounts.oracles[1].clone(),
        accounts.oracles[2].clone(),
        accounts.reserve_vaults[0].clone(),
        accounts.reserve_vaults[1].clone(),
        accounts.reserve_vaults[2].clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(if direction == 0 {
        &ISSUE_DISCRIMINATOR
    } else {
        &REDEEM_DISCRIMINATOR
    });
    data.extend_from_slice(&amount_in.to_le_bytes());
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &account_infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_carrot_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == CARROT_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidPath);
    require_keys_eq!(
        step[0].key(),
        CARROT_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[1].key(), CARROT_VAULT, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[1].owner,
        CARROT_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[2].key(), CRT_MINT, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[2].owner,
        TOKEN_2022_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        SYSTEM_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[7].key(),
        TOKEN_2022_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        CARROT_LOG_PROGRAM,
        ArbitrageError::InvalidAccount
    );

    let (asset_mint, asset_program, asset_vault, oracle_index, asset_index) =
        expected_asset(step[3].key())?;
    require_keys_eq!(step[3].key(), asset_mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[3].owner,
        asset_program,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[4].key(), asset_vault, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[6].key(), asset_program, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[9 + oracle_index].key(),
        expected_oracles()[oracle_index],
        ArbitrageError::InvalidAccount
    );
    validate_mint(&step[2], 9)?;
    validate_mint(&step[3], 6)?;

    let (
        expected_input_mint,
        expected_output_mint,
        expected_input_program,
        expected_output_program,
    ) = if direction == 0 {
        (asset_mint, CRT_MINT, asset_program, TOKEN_2022_PROGRAM)
    } else {
        (CRT_MINT, asset_mint, TOKEN_2022_PROGRAM, asset_program)
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        input_token_program.key(),
        expected_input_program,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_token_program.key(),
        expected_output_program,
        ArbitrageError::InvalidAccount
    );

    let vault_data = step[1].try_borrow_data()?;
    require!(
        vault_data.len() >= ASSET_VECTOR_OFFSET + 3 * ASSET_RECORD_LEN
            && vault_data.get(..8) == Some(VAULT_DISCRIMINATOR.as_slice())
            && pubkey_at(&vault_data, 8)? == CARROT_AUTHORITY
            && pubkey_at(&vault_data, 40)? == CRT_MINT
            && vault_data[102] == 0
            && u32_at(&vault_data, 107)? == 3,
        ArbitrageError::InvalidAccount
    );
    let redemption_fee_bps = u16_at(&vault_data, 72)?;
    require!(
        fee_rate
            == if direction == 0 {
                0
            } else {
                redemption_fee_bps
            },
        ArbitrageError::InvalidInstructionData
    );
    for index in 0..3 {
        validate_asset_record(&vault_data, index)?;
    }
    require!(
        pubkey_at(
            &vault_data,
            ASSET_VECTOR_OFFSET + asset_index * ASSET_RECORD_LEN + 2
        )? == asset_mint,
        ArbitrageError::InvalidAccount
    );
    drop(vault_data);

    let mints = [USDC_MINT, USDT_MINT, PYUSD_MINT];
    let token_programs = [TOKEN_PROGRAM, TOKEN_PROGRAM, TOKEN_2022_PROGRAM];
    let vaults = expected_vaults();
    for index in 0..3 {
        require_keys_eq!(
            step[9 + index].key(),
            expected_oracles()[index],
            ArbitrageError::InvalidAccount
        );
        require_keys_eq!(
            *step[9 + index].owner,
            PYTH_RECEIVER_PROGRAM,
            ArbitrageError::InvalidAccount
        );
        validate_oracle(&step[9 + index])?;
        require_keys_eq!(
            step[12 + index].key(),
            vaults[index],
            ArbitrageError::InvalidAccount
        );
        validate_reserve(&step[12 + index], mints[index], token_programs[index])?;
    }
    Ok(())
}

fn expected_asset(mint: Pubkey) -> Result<(Pubkey, Pubkey, Pubkey, usize, usize)> {
    match mint {
        USDC_MINT => Ok((USDC_MINT, TOKEN_PROGRAM, USDC_VAULT, 0, 0)),
        USDT_MINT => Ok((USDT_MINT, TOKEN_PROGRAM, USDT_VAULT, 1, 1)),
        PYUSD_MINT => Ok((PYUSD_MINT, TOKEN_2022_PROGRAM, PYUSD_VAULT, 2, 2)),
        _ => Err(ArbitrageError::InvalidAccount.into()),
    }
}

fn expected_vaults() -> [Pubkey; 3] {
    [USDC_VAULT, USDT_VAULT, PYUSD_VAULT]
}

fn expected_oracles() -> [Pubkey; 3] {
    [USDC_ORACLE, USDT_ORACLE, PYUSD_ORACLE]
}

fn validate_asset_record(data: &[u8], index: usize) -> Result<()> {
    let offset = ASSET_VECTOR_OFFSET
        .checked_add(
            index
                .checked_mul(ASSET_RECORD_LEN)
                .ok_or(ArbitrageError::MathOverflow)?,
        )
        .ok_or(ArbitrageError::MathOverflow)?;
    let mints = [USDC_MINT, USDT_MINT, PYUSD_MINT];
    require!(
        u16_at(data, offset)? == index as u16
            && pubkey_at(data, offset + 2)? == mints[index]
            && data.get(offset + 34).copied() == Some(6)
            && pubkey_at(data, offset + 35)? == expected_vaults()[index]
            && pubkey_at(data, offset + 67)? == expected_oracles()[index],
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_mint(mint: &AccountInfo, decimals: u8) -> Result<()> {
    let data = mint.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[44] == decimals,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_reserve(account: &AccountInfo, mint: Pubkey, token_program: Pubkey) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        token_program,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(data.len() >= 72, ArbitrageError::InvalidAccount);
    require_keys_eq!(pubkey_at(&data, 0)?, mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        pubkey_at(&data, 32)?,
        CARROT_VAULT,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_oracle(account: &AccountInfo) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.len() == 134
            && data.get(..8) == Some(PRICE_UPDATE_DISCRIMINATOR.as_slice())
            && data.get(40).copied() == Some(1),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn bytes_at<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N]> {
    let end = offset.checked_add(N).ok_or(ArbitrageError::MathOverflow)?;
    let source = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut output = [0_u8; N];
    output.copy_from_slice(source);
    Ok(output)
}

fn pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey> {
    Ok(Pubkey::new_from_array(bytes_at(data, offset)?))
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(bytes_at(data, offset)?))
}

fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(bytes_at(data, offset)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(
        key: Pubkey,
        owner: Pubkey,
        data: Vec<u8>,
        writable: bool,
        executable: bool,
    ) -> AccountInfo<'static> {
        AccountInfo::new(
            Box::leak(Box::new(key)),
            false,
            writable,
            Box::leak(Box::new(1_u64)),
            Box::leak(data.into_boxed_slice()),
            Box::leak(Box::new(owner)),
            executable,
            0,
        )
    }

    fn mint_account(
        key: Pubkey,
        owner: Pubkey,
        decimals: u8,
        writable: bool,
    ) -> AccountInfo<'static> {
        let mut data = vec![0_u8; 45];
        data[44] = decimals;
        account(key, owner, data, writable, false)
    }

    fn reserve_account(
        key: Pubkey,
        mint: Pubkey,
        token_program: Pubkey,
        writable: bool,
    ) -> AccountInfo<'static> {
        let mut data = vec![0_u8; 72];
        data[..32].copy_from_slice(mint.as_ref());
        data[32..64].copy_from_slice(CARROT_VAULT.as_ref());
        account(key, token_program, data, writable, false)
    }

    fn oracle_account(key: Pubkey) -> AccountInfo<'static> {
        let mut data = vec![0_u8; 134];
        data[..8].copy_from_slice(&PRICE_UPDATE_DISCRIMINATOR);
        data[40] = 1;
        account(key, PYTH_RECEIVER_PROGRAM, data, false, false)
    }

    fn vault_account() -> AccountInfo<'static> {
        let mut data = vec![0_u8; ASSET_VECTOR_OFFSET + 3 * ASSET_RECORD_LEN];
        data[..8].copy_from_slice(&VAULT_DISCRIMINATOR);
        data[8..40].copy_from_slice(CARROT_AUTHORITY.as_ref());
        data[40..72].copy_from_slice(CRT_MINT.as_ref());
        data[72..74].copy_from_slice(&1_u16.to_le_bytes());
        data[107..111].copy_from_slice(&3_u32.to_le_bytes());
        for (index, (mint, vault, oracle)) in [
            (USDC_MINT, USDC_VAULT, USDC_ORACLE),
            (USDT_MINT, USDT_VAULT, USDT_ORACLE),
            (PYUSD_MINT, PYUSD_VAULT, PYUSD_ORACLE),
        ]
        .into_iter()
        .enumerate()
        {
            let offset = ASSET_VECTOR_OFFSET + index * ASSET_RECORD_LEN;
            data[offset..offset + 2].copy_from_slice(&(index as u16).to_le_bytes());
            data[offset + 2..offset + 34].copy_from_slice(mint.as_ref());
            data[offset + 34] = 6;
            data[offset + 35..offset + 67].copy_from_slice(vault.as_ref());
            data[offset + 67..offset + 99].copy_from_slice(oracle.as_ref());
        }
        account(CARROT_VAULT, CARROT_PROGRAM_ID, data, true, false)
    }

    fn usdc_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            account(CARROT_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            vault_account(),
            mint_account(CRT_MINT, TOKEN_2022_PROGRAM, 9, true),
            mint_account(USDC_MINT, TOKEN_PROGRAM, 6, false),
            reserve_account(USDC_VAULT, USDC_MINT, TOKEN_PROGRAM, true),
            account(SYSTEM_PROGRAM, Pubkey::default(), vec![], false, true),
            account(TOKEN_PROGRAM, Pubkey::default(), vec![], false, true),
            account(TOKEN_2022_PROGRAM, Pubkey::default(), vec![], false, true),
            account(CARROT_LOG_PROGRAM, Pubkey::default(), vec![], false, true),
            oracle_account(USDC_ORACLE),
            oracle_account(USDT_ORACLE),
            oracle_account(PYUSD_ORACLE),
            reserve_account(USDC_VAULT, USDC_MINT, TOKEN_PROGRAM, true),
            reserve_account(USDT_VAULT, USDT_MINT, TOKEN_PROGRAM, false),
            reserve_account(PYUSD_VAULT, PYUSD_MINT, TOKEN_2022_PROGRAM, false),
        ]
    }

    #[test]
    fn instruction_data_matches_confirmed_redeem_layout() {
        let mut data = Vec::new();
        data.extend_from_slice(&REDEEM_DISCRIMINATOR);
        data.extend_from_slice(&13_674_430_372_u64.to_le_bytes());
        assert_eq!(
            data,
            vec![
                0xb8, 0x0c, 0x56, 0x95, 0x46, 0xc4, 0x61, 0xe1, 0xa4, 0x3f, 0x0f, 0x2f, 0x03, 0x00,
                0x00, 0x00,
            ]
        );
    }

    #[test]
    fn semantic_validation_binds_vault_assets_and_both_directions() {
        let step = usdc_step_accounts();
        assert!(validate_carrot_semantic_accounts(
            &step, 0, 0, &step[3], &step[2], &step[6], &step[7],
        )
        .is_ok());
        assert!(validate_carrot_semantic_accounts(
            &step, 1, 1, &step[2], &step[3], &step[7], &step[6],
        )
        .is_ok());

        let mut wrong_oracle = usdc_step_accounts();
        wrong_oracle[9] = oracle_account(Pubkey::new_unique());
        assert!(validate_carrot_semantic_accounts(
            &wrong_oracle,
            0,
            0,
            &wrong_oracle[3],
            &wrong_oracle[2],
            &wrong_oracle[6],
            &wrong_oracle[7],
        )
        .is_err());
    }
}
