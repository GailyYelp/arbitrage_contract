use crate::errors::ArbitrageError;
use crate::instructions::types::{read_token_amount, token_balance_delta, SwapResult};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

pub const PERENA_STAR_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("save8RQVPMWNTzU18t3GBvBkN9hT7jsGjiCQ28FpD9H");
pub const PERENA_STAR_BANK_STATE: Pubkey =
    anchor_lang::pubkey!("sM6P4mh53CnG4faN4Fo3seY7wMSAiHdy8o6gKjwQF7A");
pub const PERENA_STAR_USDC_VAULT: Pubkey =
    anchor_lang::pubkey!("3bZ1qY6wfzyDH7QMPiRKLr6k8p1asdtyjvJyJsJBdv23");
pub const PERENA_STAR_ORACLE_STATE: Pubkey =
    anchor_lang::pubkey!("CmKFP4YJg5QpAryUm9xk5QD611bccYMzZvpvQDJkMwt6");
pub const PERENA_STAR_USDC_MINT: Pubkey =
    anchor_lang::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const PERENA_STAR_USD_STAR_MINT: Pubkey =
    anchor_lang::pubkey!("star9agSpjiFe3M49B3RniVU4CMBBEK3Qnaqn3RGiFM");
pub const PERENA_STAR_YIELDING_VAULT: Pubkey =
    anchor_lang::pubkey!("HvG7HSrNHVAcjzgwt3UVtnY9srkrY7qnMG4zS1SnPQT2");
pub const PERENA_STAR_TEAM_STATE: Pubkey =
    anchor_lang::pubkey!("6tqLkhbqJSx4KG616VhNCvsaFqcDPok7wdbzU2DmEAub");
pub const PERENA_STAR_FEE_TEAM_ATA: Pubkey =
    anchor_lang::pubkey!("3msJbxNbSeosztbNEB1eFPitMFnP8ogCszegPUswipdL");
pub const MARGINFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");
pub const MARGINFI_GROUP: Pubkey =
    anchor_lang::pubkey!("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");
pub const MARGINFI_ACCOUNT: Pubkey =
    anchor_lang::pubkey!("C8JLFVMuSFFFmDX2AEyMmP6zRXtXwJgptHeBUrnj4zLE");
pub const MARGINFI_BANK: Pubkey =
    anchor_lang::pubkey!("2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB");
pub const MARGINFI_VAULT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("3uxNepDbmkDNq6JhRja5Z8QwbTrfmkKP8AKZV5chYDGG");
pub const MARGINFI_LIQUIDITY_VAULT: Pubkey =
    anchor_lang::pubkey!("7jaiZR5Sk8hdYN9MxTpczTcwbWpb5WEoxSANuUwveuat");
pub const MARGINFI_USDC_ORACLE: Pubkey =
    anchor_lang::pubkey!("Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX");

pub const PERENA_STAR_STEP_ACCOUNTS: usize = 14;
pub const PERENA_STAR_MINT_DISCRIMINATOR: [u8; 8] = [31, 100, 17, 215, 62, 12, 31, 2];
pub const PERENA_STAR_BURN_DISCRIMINATOR: [u8; 8] = [167, 22, 56, 95, 212, 15, 185, 218];
const BANK_DISCRIMINATOR: [u8; 8] = [16, 169, 126, 99, 35, 169, 73, 200];
const VAULT_DISCRIMINATOR: [u8; 8] = [238, 187, 81, 47, 96, 228, 197, 55];
const ORACLE_DISCRIMINATOR: [u8; 8] = [130, 253, 66, 122, 94, 135, 208, 214];
const TEAM_DISCRIMINATOR: [u8; 8] = [30, 252, 138, 15, 198, 186, 51, 100];

#[derive(Clone)]
pub struct PerenaStarAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub bank_state: &'info AccountInfo<'info>,
    pub vault_state: &'info AccountInfo<'info>,
    pub oracle_state: &'info AccountInfo<'info>,
    pub yielding_mint: &'info AccountInfo<'info>,
    pub bank_mint: &'info AccountInfo<'info>,
    pub yielding_user_ta: &'info AccountInfo<'info>,
    pub bank_mint_user_ta: &'info AccountInfo<'info>,
    pub yielding_vault_ata: &'info AccountInfo<'info>,
    pub team_state: &'info AccountInfo<'info>,
    pub fee_team_ata: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub marginfi_program: &'info AccountInfo<'info>,
    pub marginfi_group: &'info AccountInfo<'info>,
    pub marginfi_account: &'info AccountInfo<'info>,
    pub marginfi_bank: &'info AccountInfo<'info>,
    pub marginfi_vault_authority: &'info AccountInfo<'info>,
    pub marginfi_liquidity_vault: &'info AccountInfo<'info>,
    pub marginfi_oracle: &'info AccountInfo<'info>,
}

pub fn perena_star_swap<'info>(
    accounts: PerenaStarAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let output_account = if direction == 0 {
        accounts.bank_mint_user_ta
    } else {
        accounts.yielding_user_ta
    };
    let pre_out = read_token_amount(output_account)?;
    let mut metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.bank_state.key(), false),
        AccountMeta::new(accounts.vault_state.key(), false),
        AccountMeta::new_readonly(accounts.oracle_state.key(), false),
        AccountMeta::new_readonly(accounts.yielding_mint.key(), false),
        AccountMeta::new(accounts.bank_mint.key(), false),
        AccountMeta::new(accounts.yielding_user_ta.key(), false),
        AccountMeta::new(accounts.bank_mint_user_ta.key(), false),
        AccountMeta::new(accounts.yielding_vault_ata.key(), false),
        AccountMeta::new(accounts.team_state.key(), false),
        AccountMeta::new(accounts.fee_team_ata.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(accounts.marginfi_program.key(), false),
    ];
    let mut account_infos = vec![
        accounts.payer.clone(),
        accounts.bank_state.clone(),
        accounts.vault_state.clone(),
        accounts.oracle_state.clone(),
        accounts.yielding_mint.clone(),
        accounts.bank_mint.clone(),
        accounts.yielding_user_ta.clone(),
        accounts.bank_mint_user_ta.clone(),
        accounts.yielding_vault_ata.clone(),
        accounts.team_state.clone(),
        accounts.fee_team_ata.clone(),
        accounts.system_program.clone(),
        accounts.token_program.clone(),
        accounts.token_program.clone(),
        accounts.associated_token_program.clone(),
        accounts.marginfi_program.clone(),
    ];
    if direction == 0 {
        metas.extend([
            AccountMeta::new_readonly(accounts.marginfi_group.key(), false),
            AccountMeta::new(accounts.marginfi_account.key(), false),
            AccountMeta::new(accounts.marginfi_bank.key(), false),
            AccountMeta::new(accounts.marginfi_liquidity_vault.key(), false),
        ]);
        account_infos.extend([
            accounts.marginfi_group.clone(),
            accounts.marginfi_account.clone(),
            accounts.marginfi_bank.clone(),
            accounts.marginfi_liquidity_vault.clone(),
        ]);
    } else {
        metas.extend([
            AccountMeta::new(accounts.marginfi_group.key(), false),
            AccountMeta::new(accounts.marginfi_account.key(), false),
            AccountMeta::new(accounts.marginfi_bank.key(), false),
            AccountMeta::new_readonly(accounts.marginfi_vault_authority.key(), false),
            AccountMeta::new(accounts.marginfi_liquidity_vault.key(), false),
            AccountMeta::new_readonly(accounts.marginfi_bank.key(), false),
            AccountMeta::new_readonly(accounts.marginfi_oracle.key(), false),
        ]);
        account_infos.extend([
            accounts.marginfi_group.clone(),
            accounts.marginfi_account.clone(),
            accounts.marginfi_bank.clone(),
            accounts.marginfi_vault_authority.clone(),
            accounts.marginfi_liquidity_vault.clone(),
            accounts.marginfi_bank.clone(),
            accounts.marginfi_oracle.clone(),
        ]);
    }
    account_infos.push(accounts.program.clone());
    let ix = Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data: swap_data(amount_in, min_amount_out, direction),
    };
    invoke(&ix, &account_infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_perena_star_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == PERENA_STAR_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let expected = [
        PERENA_STAR_PROGRAM_ID,
        PERENA_STAR_BANK_STATE,
        PERENA_STAR_USDC_VAULT,
        PERENA_STAR_ORACLE_STATE,
        PERENA_STAR_YIELDING_VAULT,
        PERENA_STAR_TEAM_STATE,
        PERENA_STAR_FEE_TEAM_ATA,
        MARGINFI_PROGRAM_ID,
        MARGINFI_GROUP,
        MARGINFI_ACCOUNT,
        MARGINFI_BANK,
        MARGINFI_VAULT_AUTHORITY,
        MARGINFI_LIQUIDITY_VAULT,
        MARGINFI_USDC_ORACLE,
    ];
    for (account, expected_key) in step.iter().zip(expected) {
        require_keys_eq!(account.key(), expected_key, ArbitrageError::InvalidAccount);
    }
    require_keys_eq!(
        *in_mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *out_mint.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let (expected_in, expected_out) = if direction == 0 {
        (PERENA_STAR_USDC_MINT, PERENA_STAR_USD_STAR_MINT)
    } else {
        (PERENA_STAR_USD_STAR_MINT, PERENA_STAR_USDC_MINT)
    };
    require_keys_eq!(in_mint.key(), expected_in, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(
        out_mint.key(),
        expected_out,
        ArbitrageError::InvalidTokenMint
    );
    require!(
        step[0].executable && step[7].executable,
        ArbitrageError::InvalidAccount
    );
    for account in [&step[1], &step[2], &step[3], &step[5]] {
        require_keys_eq!(
            *account.owner,
            step[0].key(),
            ArbitrageError::InvalidAccount
        );
    }
    for account in [&step[8], &step[9], &step[10]] {
        require_keys_eq!(
            *account.owner,
            step[7].key(),
            ArbitrageError::InvalidAccount
        );
    }
    validate_program_state(step, direction)?;
    validate_token_account(
        &step[4],
        PERENA_STAR_USDC_MINT,
        step[2].key(),
        token_program,
    )?;
    validate_token_account(
        &step[6],
        PERENA_STAR_USDC_MINT,
        step[5].key(),
        token_program,
    )?;
    validate_token_account(
        &step[12],
        PERENA_STAR_USDC_MINT,
        step[11].key(),
        token_program,
    )?;
    let expected_marginfi_account = Pubkey::find_program_address(
        &[
            b"marginfi_account",
            MARGINFI_GROUP.as_ref(),
            PERENA_STAR_USDC_VAULT.as_ref(),
            &0_u16.to_le_bytes(),
            &0_u16.to_le_bytes(),
        ],
        &MARGINFI_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(
        step[9].key(),
        expected_marginfi_account,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_program_state(step: &[AccountInfo], direction: u8) -> Result<()> {
    let bank = step[1].try_borrow_data()?;
    let vault = step[2].try_borrow_data()?;
    let oracle = step[3].try_borrow_data()?;
    let team = step[5].try_borrow_data()?;
    require!(
        bank.len() == 1_560 && bank.starts_with(&BANK_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        vault.len() == 984 && vault.starts_with(&VAULT_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        oracle.len() == 1_096 && oracle.starts_with(&ORACLE_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        team.len() == 704 && team.starts_with(&TEAM_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(bank[9] == 0, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&bank, 320)?,
        PERENA_STAR_USD_STAR_MINT,
        ArbitrageError::InvalidAccount
    );
    require!(
        read_u64(&bank, 352)? > 0 && bank[360] == 6,
        ArbitrageError::InvalidAccount
    );
    validate_status(&bank, 152, direction)?;

    require!(vault[10] == 3, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&vault, 48)?,
        PERENA_STAR_BANK_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&vault, 80)?,
        PERENA_STAR_TEAM_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&vault, 112)?,
        PERENA_STAR_ORACLE_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&vault, 176)?,
        PERENA_STAR_USDC_MINT,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&vault, 208)?,
        PERENA_STAR_YIELDING_VAULT,
        ArbitrageError::InvalidAccount
    );
    require!(vault[240] == 6, ArbitrageError::InvalidAccount);
    require!(
        read_u16(&vault, 380)? <= 10_000 && read_u16(&vault, 382)? <= 10_000,
        ArbitrageError::InvalidAccount
    );
    require!(read_u64(&vault, 584)? > 0, ArbitrageError::InvalidAccount);
    validate_status(&vault, 480, direction)?;

    require_keys_eq!(
        read_pubkey(&oracle, 16)?,
        PERENA_STAR_BANK_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&oracle, 48)?,
        PERENA_STAR_USDC_VAULT,
        ArbitrageError::InvalidAccount
    );
    require!(
        oracle[90] == 6 && read_u64(&oracle, 720)? > 0,
        ArbitrageError::InvalidAccount
    );

    require_keys_eq!(
        read_pubkey(&team, 16)?,
        PERENA_STAR_BANK_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&team, 48)?,
        PERENA_STAR_USDC_VAULT,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&team, 112)?,
        PERENA_STAR_FEE_TEAM_ATA,
        ArbitrageError::InvalidAccount
    );

    let expected_bank = Pubkey::find_program_address(&[b"BANK", &[0]], &PERENA_STAR_PROGRAM_ID).0;
    let expected_vault = Pubkey::find_program_address(
        &[
            b"VAULT",
            expected_bank.as_ref(),
            PERENA_STAR_USDC_MINT.as_ref(),
            &[3],
        ],
        &PERENA_STAR_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(
        expected_bank,
        PERENA_STAR_BANK_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        expected_vault,
        PERENA_STAR_USDC_VAULT,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_status(data: &[u8], offset: usize, direction: u8) -> Result<()> {
    require!(
        data[offset] <= 1 && data[offset + 1] <= 1 && data[offset + 2] <= 1,
        ArbitrageError::InvalidAccount
    );
    require!(data[offset] == 0, ArbitrageError::InvalidAccount);
    let directional_offset = if direction == 0 {
        offset + 1
    } else {
        offset + 2
    };
    require!(
        data[directional_offset] == 0,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_token_account(
    account: &AccountInfo,
    mint: Pubkey,
    authority: Pubkey,
    token_program: &AccountInfo,
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(data.len() >= 72, ArbitrageError::InvalidAccount);
    require_keys_eq!(read_pubkey(&data, 0)?, mint, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&data, 32)?,
        authority,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    Ok(Pubkey::new_from_array(
        data.get(offset..offset + 32)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        data.get(offset..offset + 2)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

pub fn swap_data(amount_in: u64, min_amount_out: u64, direction: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(if direction == 0 {
        &PERENA_STAR_MINT_DISCRIMINATOR
    } else {
        &PERENA_STAR_BURN_DISCRIMINATOR
    });
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_data_matches_official_idl() {
        let mint = swap_data(10, 9, 0);
        let burn = swap_data(10, 9, 1);
        assert_eq!(mint.len(), 24);
        assert_eq!(&mint[..8], &PERENA_STAR_MINT_DISCRIMINATOR);
        assert_eq!(&burn[..8], &PERENA_STAR_BURN_DISCRIMINATOR);
        assert_eq!(&burn[8..16], &10_u64.to_le_bytes());
        assert_eq!(&burn[16..24], &9_u64.to_le_bytes());
    }
}
