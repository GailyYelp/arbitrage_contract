use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{
        read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
        SwapResult,
    },
};

pub const VOLTR_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8");
pub const VOLTR_STEP_ACCOUNTS: usize = 11;
const VAULT_LEN: usize = 928;
const PROTOCOL_LEN: usize = 110;
const VAULT_DISCRIMINATOR: [u8; 8] = [211, 8, 232, 43, 2, 152, 117, 119];
const PROTOCOL_DISCRIMINATOR: [u8; 8] = [45, 39, 101, 43, 115, 72, 131, 40];
const DEPOSIT_DISCRIMINATOR: [u8; 8] = [126, 224, 21, 255, 228, 53, 117, 33];
const WITHDRAW_DISCRIMINATOR: [u8; 8] = [221, 56, 115, 168, 128, 220, 235, 245];
const MAX_FEE_BPS: u16 = 10_000;
const VAULT_DISABLE_DEPOSIT: u16 = 1 << 0;
const VAULT_DISABLE_INSTANT_WITHDRAW: u16 = 1 << 1;
const PROTOCOL_ALLOW_DEPOSIT: u16 = 1 << 1;
const PROTOCOL_ALLOW_INSTANT_WITHDRAW: u16 = 1 << 2;

pub struct VoltrAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub protocol: &'info AccountInfo<'info>,
    pub vault: &'info AccountInfo<'info>,
    pub asset_mint: &'info AccountInfo<'info>,
    pub lp_mint: &'info AccountInfo<'info>,
    pub idle_vault: &'info AccountInfo<'info>,
    pub idle_authority: &'info AccountInfo<'info>,
    pub lp_mint_authority: &'info AccountInfo<'info>,
    pub asset_token_program: &'info AccountInfo<'info>,
    pub lp_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_asset: &'info AccountInfo<'info>,
    pub user_lp: &'info AccountInfo<'info>,
}

pub fn voltr_swap<'info>(
    accounts: VoltrAccounts<'info>,
    direction: u8,
    amount_in: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let output = if direction == 0 {
        accounts.user_lp
    } else {
        accounts.user_asset
    };
    let pre_out = read_token_amount(output)?;
    let (metas, infos, data) = if direction == 0 {
        (
            vec![
                AccountMeta::new_readonly(accounts.user.key(), true),
                AccountMeta::new_readonly(accounts.protocol.key(), false),
                AccountMeta::new(accounts.vault.key(), false),
                AccountMeta::new_readonly(accounts.asset_mint.key(), false),
                AccountMeta::new(accounts.lp_mint.key(), false),
                AccountMeta::new(accounts.user_asset.key(), false),
                AccountMeta::new(accounts.idle_vault.key(), false),
                AccountMeta::new_readonly(accounts.idle_authority.key(), false),
                AccountMeta::new(accounts.user_lp.key(), false),
                AccountMeta::new_readonly(accounts.lp_mint_authority.key(), false),
                AccountMeta::new_readonly(accounts.asset_token_program.key(), false),
                AccountMeta::new_readonly(accounts.lp_token_program.key(), false),
                AccountMeta::new_readonly(accounts.system_program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.protocol.clone(),
                accounts.vault.clone(),
                accounts.asset_mint.clone(),
                accounts.lp_mint.clone(),
                accounts.user_asset.clone(),
                accounts.idle_vault.clone(),
                accounts.idle_authority.clone(),
                accounts.user_lp.clone(),
                accounts.lp_mint_authority.clone(),
                accounts.asset_token_program.clone(),
                accounts.lp_token_program.clone(),
                accounts.system_program.clone(),
                accounts.program.clone(),
            ],
            voltr_data(direction, amount_in)?,
        )
    } else {
        (
            vec![
                AccountMeta::new_readonly(accounts.user.key(), true),
                AccountMeta::new_readonly(accounts.protocol.key(), false),
                AccountMeta::new(accounts.vault.key(), false),
                AccountMeta::new_readonly(accounts.asset_mint.key(), false),
                AccountMeta::new(accounts.lp_mint.key(), false),
                AccountMeta::new(accounts.user_lp.key(), false),
                AccountMeta::new(accounts.idle_vault.key(), false),
                AccountMeta::new(accounts.idle_authority.key(), false),
                AccountMeta::new(accounts.user_asset.key(), false),
                AccountMeta::new_readonly(accounts.asset_token_program.key(), false),
                AccountMeta::new_readonly(accounts.lp_token_program.key(), false),
                AccountMeta::new_readonly(accounts.system_program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.protocol.clone(),
                accounts.vault.clone(),
                accounts.asset_mint.clone(),
                accounts.lp_mint.clone(),
                accounts.user_lp.clone(),
                accounts.idle_vault.clone(),
                accounts.idle_authority.clone(),
                accounts.user_asset.clone(),
                accounts.asset_token_program.clone(),
                accounts.lp_token_program.clone(),
                accounts.system_program.clone(),
                accounts.program.clone(),
            ],
            voltr_data(direction, amount_in)?,
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
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn voltr_data(direction: u8, amount_in: u64) -> Result<Vec<u8>> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let mut data = if direction == 0 {
        DEPOSIT_DISCRIMINATOR.to_vec()
    } else {
        WITHDRAW_DISCRIMINATOR.to_vec()
    };
    data.extend_from_slice(&amount_in.to_le_bytes());
    if direction == 1 {
        data.extend_from_slice(&[1, 0]);
    }
    Ok(data)
}

pub fn validate_voltr_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    validate_voltr_semantic_accounts_at_time(
        step,
        direction,
        fee_rate,
        input_mint,
        output_mint,
        Clock::get()?.unix_timestamp,
    )
}

fn validate_voltr_semantic_accounts_at_time(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
    current_ts: i64,
) -> Result<()> {
    require!(
        step.len() == VOLTR_STEP_ACCOUNTS && direction <= 1 && fee_rate == 0,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step[0].key(),
        VOLTR_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        VOLTR_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        VOLTR_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[1].key(),
        Pubkey::find_program_address(&[b"protocol"], &VOLTR_PROGRAM_ID).0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[9].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[10].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require!(
        matches!(
            step[8].key(),
            anchor_spl::token::ID | anchor_spl::token_2022::ID
        ),
        ArbitrageError::InvalidProgramId
    );

    let protocol = step[1].try_borrow_data()?;
    let protocol_operational_state = read_u16(&protocol, 40)?;
    require!(
        protocol.len() == PROTOCOL_LEN
            && protocol.starts_with(&PROTOCOL_DISCRIMINATOR)
            && protocol_operational_state != 0
            && if direction == 0 {
                protocol_operational_state & PROTOCOL_ALLOW_DEPOSIT != 0
            } else {
                protocol_operational_state & PROTOCOL_ALLOW_INSTANT_WITHDRAW != 0
            },
        ArbitrageError::InvalidAccount
    );
    drop(protocol);

    let vault = step[2].try_borrow_data()?;
    require!(
        vault.len() == VAULT_LEN && vault.starts_with(&VAULT_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let asset_mint = read_pubkey(&vault, 104)?;
    let idle_vault = read_pubkey(&vault, 136)?;
    let total_asset_value = read_u64(&vault, 168)?;
    let idle_bump = vault[176];
    let lp_mint = read_pubkey(&vault, 272)?;
    let max_cap = read_u64(&vault, 432)?;
    let start_at_ts = read_u64(&vault, 440)?;
    let withdrawal_waiting_period = read_u64(&vault, 456)?;
    let disabled_operations = read_u16(&vault, 464)?;
    let management_fee = read_u16(&vault, 516)?
        .checked_add(read_u16(&vault, 518)?)
        .and_then(|value| value.checked_add(read_u16(&vault, 526).ok()?))
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        total_asset_value > 0
            && max_cap >= total_asset_value
            && read_u16(&vault, 520)? < MAX_FEE_BPS
            && read_u16(&vault, 522)? < MAX_FEE_BPS
            && management_fee < MAX_FEE_BPS
            && vault[664] > 0
            && current_ts >= 0
            && u64::try_from(current_ts).map_err(|_| ArbitrageError::InvalidAccount)?
                >= start_at_ts
            && (direction == 0 || withdrawal_waiting_period == 0),
        ArbitrageError::InvalidAccount
    );
    require!(
        if direction == 0 {
            disabled_operations & VAULT_DISABLE_DEPOSIT == 0
        } else {
            disabled_operations & VAULT_DISABLE_INSTANT_WITHDRAW == 0
        },
        ArbitrageError::InvalidAccount
    );
    drop(vault);

    require_keys_eq!(asset_mint, step[3].key(), ArbitrageError::InvalidTokenMint);
    require_keys_eq!(lp_mint, step[4].key(), ArbitrageError::InvalidTokenMint);
    require_keys_eq!(idle_vault, step[5].key(), ArbitrageError::InvalidAccount);
    let (expected_idle_authority, expected_idle_bump) = Pubkey::find_program_address(
        &[b"vault_asset_idle_auth", step[2].key().as_ref()],
        &VOLTR_PROGRAM_ID,
    );
    require!(
        idle_bump == expected_idle_bump,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        expected_idle_authority,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        Pubkey::find_program_address(
            &[b"vault_lp_mint", step[2].key().as_ref()],
            &VOLTR_PROGRAM_ID,
        )
        .0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[7].key(),
        Pubkey::find_program_address(
            &[b"vault_lp_mint_auth", step[2].key().as_ref()],
            &VOLTR_PROGRAM_ID,
        )
        .0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[3].owner,
        step[8].key(),
        ArbitrageError::InvalidAccount
    );
    validate_classic_lp_mint(&step[4], &step[7])?;
    validate_token_account_for_mint_and_authority(&step[5], &step[3], &step[8], &step[6])?;
    require!(
        read_token_amount(&step[5])? <= total_asset_value,
        ArbitrageError::InvalidAccount
    );
    let expected_idle_vault = Pubkey::find_program_address(
        &[
            step[6].key().as_ref(),
            step[8].key().as_ref(),
            step[3].key().as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0;
    require_keys_eq!(
        step[5].key(),
        expected_idle_vault,
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output) = if direction == 0 {
        (asset_mint, lp_mint)
    } else {
        (lp_mint, asset_mint)
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output,
        ArbitrageError::InvalidTokenMint
    );
    Ok(())
}

fn validate_classic_lp_mint(mint: &AccountInfo<'_>, authority: &AccountInfo<'_>) -> Result<()> {
    require_keys_eq!(
        *mint.owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidAccount
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == 82
            && read_u32(&data, 0)? == 1
            && read_pubkey(&data, 4)? == authority.key()
            && read_u64(&data, 36)? > 0
            && data[45] == 1,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(bytes);
    Ok(Pubkey::new_from_array(raw))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_wires_match_on_chain_idl() {
        let deposit = voltr_data(0, 1_000_000).expect("deposit wire");
        assert_eq!(deposit.len(), 16);
        assert_eq!(&deposit[..8], &DEPOSIT_DISCRIMINATOR);
        let withdraw = voltr_data(1, 50_000_000).expect("withdraw wire");
        assert_eq!(withdraw.len(), 18);
        assert_eq!(&withdraw[..8], &WITHDRAW_DISCRIMINATOR);
        assert_eq!(&withdraw[16..], &[1, 0]);
        assert!(voltr_data(2, 1).is_err());
    }
}
