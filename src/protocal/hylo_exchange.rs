use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::HYLO_EXCHANGE_PROGRAM_ID,
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const HYLO_EXCHANGE_STEP_ACCOUNTS: usize = 10;

const HYLO_STATE: Pubkey = anchor_lang::pubkey!("9cd2sAfbBvKs4SX9YKo4dcjwP3TgTVQ8dT5koshGcDND");
const HYUSD_MINT: Pubkey = anchor_lang::pubkey!("5YMkXAYccHSGnHn9nob9xEvv6Pvka9DZWH7nTbotTu9E");
const JITOSOL_MINT: Pubkey = anchor_lang::pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
const JITOSOL_FEE_AUTH: Pubkey =
    anchor_lang::pubkey!("FpLaqELxKRm6S3bjfNSknwZu43TL89VYkwuMDwsRMj59");
const JITOSOL_VAULT_AUTH: Pubkey =
    anchor_lang::pubkey!("82MNhUCha26wY4kohTUEC965b4ypEe7RPa4itp9UMrKK");
const HYUSD_AUTH: Pubkey = anchor_lang::pubkey!("CfuSViqf6wvUKEprLhtuCsSanvfAsMbDmkAW92FP95qe");
const JITOSOL_FEE_VAULT: Pubkey =
    anchor_lang::pubkey!("3JENUTyYnMMtZUSg5ErSHEvowjQteYD7wr7RDNw12bei");
const JITOSOL_VAULT: Pubkey = anchor_lang::pubkey!("2Y3TLkdGoJwbdizxqrZmQwNLYJyGKTgzC4tbetbkvQ43");
const JITOSOL_HEADER: Pubkey = anchor_lang::pubkey!("8Ri52tZXZehgAHKbx1MQiXhWXXkVsvAL9op6C5HytDKF");
const SOL_USD_ORACLE: Pubkey = anchor_lang::pubkey!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
const EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("4VzpNE51Be5vD5Yg8MC3z6TVHq5gGbLJptjv18QbD6WP");
const PYTH_RECEIVER_PROGRAM: Pubkey =
    anchor_lang::pubkey!("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ");
const TOKEN_PROGRAM: Pubkey = anchor_lang::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const SYSTEM_PROGRAM: Pubkey = anchor_lang::pubkey!("11111111111111111111111111111111");
const ASSOCIATED_TOKEN_PROGRAM: Pubkey =
    anchor_lang::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

const HYLO_ACCOUNT_DISCRIMINATOR: [u8; 8] = [114, 161, 169, 210, 204, 175, 149, 174];
const LST_HEADER_DISCRIMINATOR: [u8; 8] = [125, 135, 217, 151, 122, 202, 138, 59];
const PRICE_UPDATE_V2_DISCRIMINATOR: [u8; 8] = [34, 241, 35, 99, 157, 126, 244, 205];
const MINT_STABLECOIN_DISCRIMINATOR: [u8; 8] = [196, 235, 215, 70, 211, 5, 214, 238];
const REDEEM_STABLECOIN_DISCRIMINATOR: [u8; 8] = [69, 46, 6, 97, 170, 130, 160, 237];

const HYLO_STABLECOIN_MINT_OFFSET: usize = 104;
const HYLO_SOL_USD_ORACLE_OFFSET: usize = 392;
const LST_HEADER_MINT_OFFSET: usize = 8;
const LST_HEADER_VAULT_OFFSET: usize = 40;
const PRICE_UPDATE_VERIFICATION_LEVEL_OFFSET: usize = 40;

pub struct HyloExchangeAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub hylo: &'info AccountInfo<'info>,
    pub fee_auth: &'info AccountInfo<'info>,
    pub vault_auth: &'info AccountInfo<'info>,
    pub stablecoin_auth: &'info AccountInfo<'info>,
    pub fee_vault: &'info AccountInfo<'info>,
    pub lst_vault: &'info AccountInfo<'info>,
    pub lst_header: &'info AccountInfo<'info>,
    pub sol_usd_oracle: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
}

pub struct HyloExchangeValidationAccounts<'a, 'info> {
    pub input_mint: &'a AccountInfo<'info>,
    pub output_mint: &'a AccountInfo<'info>,
    pub input_token_program: &'a AccountInfo<'info>,
    pub output_token_program: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub associated_token_program: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
}

pub fn hylo_exchange_swap<'info>(
    accounts: HyloExchangeAccounts<'info>,
    amount_in: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidPath);

    let pre_out = read_token_amount(accounts.user_output)?;
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(if direction == 0 {
        &MINT_STABLECOIN_DISCRIMINATOR
    } else {
        &REDEEM_STABLECOIN_DISCRIMINATOR
    });
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(0); // Deployed ABI: Option<SlippageConfig>::None.

    let (metas, infos) = if direction == 0 {
        (
            vec![
                AccountMeta::new(accounts.user.key(), true),
                AccountMeta::new(accounts.hylo.key(), false),
                AccountMeta::new_readonly(accounts.fee_auth.key(), false),
                AccountMeta::new_readonly(accounts.vault_auth.key(), false),
                AccountMeta::new_readonly(accounts.stablecoin_auth.key(), false),
                AccountMeta::new(accounts.fee_vault.key(), false),
                AccountMeta::new(accounts.lst_vault.key(), false),
                AccountMeta::new_readonly(accounts.lst_header.key(), false),
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new(accounts.user_output.key(), false),
                AccountMeta::new_readonly(accounts.input_mint.key(), false),
                AccountMeta::new(accounts.output_mint.key(), false),
                AccountMeta::new_readonly(accounts.sol_usd_oracle.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
                AccountMeta::new_readonly(accounts.system_program.key(), false),
                AccountMeta::new_readonly(accounts.event_authority.key(), false),
                AccountMeta::new_readonly(accounts.program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.hylo.clone(),
                accounts.fee_auth.clone(),
                accounts.vault_auth.clone(),
                accounts.stablecoin_auth.clone(),
                accounts.fee_vault.clone(),
                accounts.lst_vault.clone(),
                accounts.lst_header.clone(),
                accounts.user_input.clone(),
                accounts.user_output.clone(),
                accounts.input_mint.clone(),
                accounts.output_mint.clone(),
                accounts.sol_usd_oracle.clone(),
                accounts.token_program.clone(),
                accounts.associated_token_program.clone(),
                accounts.system_program.clone(),
                accounts.event_authority.clone(),
                accounts.program.clone(),
            ],
        )
    } else {
        (
            vec![
                AccountMeta::new(accounts.user.key(), true),
                AccountMeta::new(accounts.hylo.key(), false),
                AccountMeta::new_readonly(accounts.fee_auth.key(), false),
                AccountMeta::new_readonly(accounts.vault_auth.key(), false),
                AccountMeta::new(accounts.fee_vault.key(), false),
                AccountMeta::new(accounts.lst_vault.key(), false),
                AccountMeta::new_readonly(accounts.lst_header.key(), false),
                AccountMeta::new(accounts.user_input.key(), false),
                AccountMeta::new(accounts.user_output.key(), false),
                AccountMeta::new(accounts.input_mint.key(), false),
                AccountMeta::new_readonly(accounts.output_mint.key(), false),
                AccountMeta::new_readonly(accounts.sol_usd_oracle.key(), false),
                AccountMeta::new_readonly(accounts.system_program.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
                AccountMeta::new_readonly(accounts.event_authority.key(), false),
                AccountMeta::new_readonly(accounts.program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.hylo.clone(),
                accounts.fee_auth.clone(),
                accounts.vault_auth.clone(),
                accounts.fee_vault.clone(),
                accounts.lst_vault.clone(),
                accounts.lst_header.clone(),
                accounts.user_input.clone(),
                accounts.user_output.clone(),
                accounts.input_mint.clone(),
                accounts.output_mint.clone(),
                accounts.sol_usd_oracle.clone(),
                accounts.system_program.clone(),
                accounts.token_program.clone(),
                accounts.associated_token_program.clone(),
                accounts.event_authority.clone(),
                accounts.program.clone(),
            ],
        )
    };
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;

    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_hylo_exchange_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    accounts: HyloExchangeValidationAccounts<'_, 'info>,
) -> Result<()> {
    require!(
        step.len() == HYLO_EXCHANGE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidPath);
    require!(fee_rate == 0, ArbitrageError::InvalidInstructionData);

    let expected_keys = [
        HYLO_EXCHANGE_PROGRAM_ID,
        HYLO_STATE,
        JITOSOL_FEE_AUTH,
        JITOSOL_VAULT_AUTH,
        HYUSD_AUTH,
        JITOSOL_FEE_VAULT,
        JITOSOL_VAULT,
        JITOSOL_HEADER,
        SOL_USD_ORACLE,
        EVENT_AUTHORITY,
    ];
    for (account, expected) in step.iter().zip(expected_keys) {
        require_keys_eq!(account.key(), expected, ArbitrageError::InvalidAccount);
    }
    require_keys_eq!(
        *step[1].owner,
        HYLO_EXCHANGE_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[7].owner,
        HYLO_EXCHANGE_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[8].owner,
        PYTH_RECEIVER_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        accounts.system_program.key(),
        SYSTEM_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        accounts.associated_token_program.key(),
        ASSOCIATED_TOKEN_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        accounts.token_program.key(),
        TOKEN_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        accounts.input_token_program.key(),
        TOKEN_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        accounts.output_token_program.key(),
        TOKEN_PROGRAM,
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output, input_decimals, output_decimals) = if direction == 0 {
        (JITOSOL_MINT, HYUSD_MINT, 9, 6)
    } else {
        (HYUSD_MINT, JITOSOL_MINT, 6, 9)
    };
    require_keys_eq!(
        accounts.input_mint.key(),
        expected_input,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        accounts.output_mint.key(),
        expected_output,
        ArbitrageError::InvalidAccount
    );
    validate_mint(accounts.input_mint, input_decimals)?;
    validate_mint(accounts.output_mint, output_decimals)?;
    validate_token_account(&step[5], JITOSOL_MINT, JITOSOL_FEE_AUTH)?;
    validate_token_account(&step[6], JITOSOL_MINT, JITOSOL_VAULT_AUTH)?;

    let hylo_data = step[1].try_borrow_data()?;
    require!(
        hylo_data.get(..8) == Some(HYLO_ACCOUNT_DISCRIMINATOR.as_slice())
            && pubkey_at(&hylo_data, HYLO_STABLECOIN_MINT_OFFSET)? == HYUSD_MINT
            && pubkey_at(&hylo_data, HYLO_SOL_USD_ORACLE_OFFSET)? == SOL_USD_ORACLE,
        ArbitrageError::InvalidAccount
    );
    drop(hylo_data);

    let header_data = step[7].try_borrow_data()?;
    require!(
        header_data.get(..8) == Some(LST_HEADER_DISCRIMINATOR.as_slice())
            && pubkey_at(&header_data, LST_HEADER_MINT_OFFSET)? == JITOSOL_MINT
            && pubkey_at(&header_data, LST_HEADER_VAULT_OFFSET)? == JITOSOL_VAULT,
        ArbitrageError::InvalidAccount
    );
    drop(header_data);

    let oracle_data = step[8].try_borrow_data()?;
    require!(
        oracle_data.get(..8) == Some(PRICE_UPDATE_V2_DISCRIMINATOR.as_slice())
            && oracle_data
                .get(PRICE_UPDATE_VERIFICATION_LEVEL_OFFSET)
                .copied()
                == Some(1),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_mint(mint: &AccountInfo, decimals: u8) -> Result<()> {
    require_keys_eq!(*mint.owner, TOKEN_PROGRAM, ArbitrageError::InvalidAccount);
    let data = mint.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[44] == decimals,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_token_account(account: &AccountInfo, mint: Pubkey, authority: Pubkey) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        TOKEN_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(data.len() >= 72, ArbitrageError::InvalidAccount);
    require_keys_eq!(pubkey_at(&data, 0)?, mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        pubkey_at(&data, 32)?,
        authority,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let source = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(source);
    Ok(Pubkey::new_from_array(bytes))
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

    fn mint_account(key: Pubkey, decimals: u8) -> AccountInfo<'static> {
        let mut data = vec![0_u8; 45];
        data[44] = decimals;
        account(key, TOKEN_PROGRAM, data, false, false)
    }

    fn token_account(key: Pubkey, mint: Pubkey, authority: Pubkey) -> AccountInfo<'static> {
        let mut data = vec![0_u8; 72];
        data[..32].copy_from_slice(mint.as_ref());
        data[32..64].copy_from_slice(authority.as_ref());
        account(key, TOKEN_PROGRAM, data, true, false)
    }

    fn step_accounts() -> Vec<AccountInfo<'static>> {
        let mut hylo = vec![0_u8; HYLO_SOL_USD_ORACLE_OFFSET + 32];
        hylo[..8].copy_from_slice(&HYLO_ACCOUNT_DISCRIMINATOR);
        hylo[HYLO_STABLECOIN_MINT_OFFSET..HYLO_STABLECOIN_MINT_OFFSET + 32]
            .copy_from_slice(HYUSD_MINT.as_ref());
        hylo[HYLO_SOL_USD_ORACLE_OFFSET..HYLO_SOL_USD_ORACLE_OFFSET + 32]
            .copy_from_slice(SOL_USD_ORACLE.as_ref());

        let mut header = vec![0_u8; LST_HEADER_VAULT_OFFSET + 32];
        header[..8].copy_from_slice(&LST_HEADER_DISCRIMINATOR);
        header[LST_HEADER_MINT_OFFSET..LST_HEADER_MINT_OFFSET + 32]
            .copy_from_slice(JITOSOL_MINT.as_ref());
        header[LST_HEADER_VAULT_OFFSET..LST_HEADER_VAULT_OFFSET + 32]
            .copy_from_slice(JITOSOL_VAULT.as_ref());

        let mut oracle = vec![0_u8; PRICE_UPDATE_VERIFICATION_LEVEL_OFFSET + 1];
        oracle[..8].copy_from_slice(&PRICE_UPDATE_V2_DISCRIMINATOR);
        oracle[PRICE_UPDATE_VERIFICATION_LEVEL_OFFSET] = 1;

        vec![
            account(
                HYLO_EXCHANGE_PROGRAM_ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(HYLO_STATE, HYLO_EXCHANGE_PROGRAM_ID, hylo, true, false),
            account(JITOSOL_FEE_AUTH, Pubkey::default(), vec![], false, false),
            account(JITOSOL_VAULT_AUTH, Pubkey::default(), vec![], false, false),
            account(HYUSD_AUTH, Pubkey::default(), vec![], false, false),
            token_account(JITOSOL_FEE_VAULT, JITOSOL_MINT, JITOSOL_FEE_AUTH),
            token_account(JITOSOL_VAULT, JITOSOL_MINT, JITOSOL_VAULT_AUTH),
            account(
                JITOSOL_HEADER,
                HYLO_EXCHANGE_PROGRAM_ID,
                header,
                false,
                false,
            ),
            account(SOL_USD_ORACLE, PYTH_RECEIVER_PROGRAM, oracle, false, false),
            account(EVENT_AUTHORITY, Pubkey::default(), vec![], false, false),
        ]
    }

    #[test]
    fn deployed_instruction_data_uses_legacy_discriminator_and_none_slippage() {
        let amount = 251_793_038_u64;
        let mut data = Vec::new();
        data.extend_from_slice(&MINT_STABLECOIN_DISCRIMINATOR);
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(0);
        assert_eq!(data.len(), 17);
        assert_eq!(&data[..8], &MINT_STABLECOIN_DISCRIMINATOR);
        assert_eq!(u64::from_le_bytes(data[8..16].try_into().unwrap()), amount);
        assert_eq!(data[16], 0);
    }

    #[test]
    fn semantic_validation_binds_deployed_state_vaults_oracle_and_directions() {
        let step = step_accounts();
        let jitosol = mint_account(JITOSOL_MINT, 9);
        let hyusd = mint_account(HYUSD_MINT, 6);
        let token = account(TOKEN_PROGRAM, Pubkey::default(), vec![], false, true);
        let system = account(SYSTEM_PROGRAM, Pubkey::default(), vec![], false, true);
        let associated = account(
            ASSOCIATED_TOKEN_PROGRAM,
            Pubkey::default(),
            vec![],
            false,
            true,
        );

        assert!(validate_hylo_exchange_semantic_accounts(
            &step,
            0,
            0,
            HyloExchangeValidationAccounts {
                input_mint: &jitosol,
                output_mint: &hyusd,
                input_token_program: &token,
                output_token_program: &token,
                system_program: &system,
                associated_token_program: &associated,
                token_program: &token,
            },
        )
        .is_ok());
        assert!(validate_hylo_exchange_semantic_accounts(
            &step,
            1,
            0,
            HyloExchangeValidationAccounts {
                input_mint: &hyusd,
                output_mint: &jitosol,
                input_token_program: &token,
                output_token_program: &token,
                system_program: &system,
                associated_token_program: &associated,
                token_program: &token,
            },
        )
        .is_ok());

        let mut wrong_header = step_accounts();
        wrong_header[7] = account(
            JITOSOL_HEADER,
            HYLO_EXCHANGE_PROGRAM_ID,
            vec![0_u8; LST_HEADER_VAULT_OFFSET + 32],
            false,
            false,
        );
        assert!(validate_hylo_exchange_semantic_accounts(
            &wrong_header,
            0,
            0,
            HyloExchangeValidationAccounts {
                input_mint: &jitosol,
                output_mint: &hyusd,
                input_token_program: &token,
                output_token_program: &token,
                system_program: &system,
                associated_token_program: &associated,
                token_program: &token,
            },
        )
        .is_err());
    }
}
