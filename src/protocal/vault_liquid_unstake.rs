use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program::invoke,
    },
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::VAULT_LIQUID_UNSTAKE_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const VAULT_LIQUID_UNSTAKE_STEP_ACCOUNTS: usize = 15;
pub const BUY_LST_DISCRIMINATOR: [u8; 8] = [56, 97, 82, 138, 114, 186, 53, 12];
pub const SELL_LST_DISCRIMINATOR: [u8; 8] = [100, 3, 79, 209, 191, 157, 180, 11];

const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
const JITO_SENTINEL: Pubkey = pubkey!("jitodontfronttitans111111111111111111111111");
const POOL_ACCOUNT_LEN: usize = 316;
const LST_INFO_ACCOUNT_LEN: usize = 225;
const INVENTORY_SUMMARY_ACCOUNT_LEN: usize = 80;
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const LST_INFO_DISCRIMINATOR: [u8; 8] = [79, 113, 226, 60, 171, 8, 142, 33];
const INVENTORY_SUMMARY_DISCRIMINATOR: [u8; 8] = [157, 77, 59, 213, 161, 246, 95, 57];

pub struct VaultLiquidUnstakeAccounts<'info> {
    pub payer: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub step: &'info [AccountInfo<'info>],
}

pub fn vault_liquid_unstake_swap<'info>(
    accounts: VaultLiquidUnstakeAccounts<'info>,
    amount_in: u64,
    minimum_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    if direction == 0 {
        require!(minimum_out > 0, ArbitrageError::InvalidAmount);
    }
    let pre_out = read_token_amount(accounts.user_output)?;
    let instruction = if direction == 0 {
        build_buy_instruction(&accounts, minimum_out, amount_in)?
    } else {
        build_sell_instruction(&accounts, amount_in)?
    };
    let mut infos = instruction
        .accounts
        .iter()
        .map(|meta| account_info_for_key(&accounts, &meta.pubkey))
        .collect::<Result<Vec<_>>>()?;
    infos.push(accounts.step[0].clone());
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

fn build_buy_instruction(
    accounts: &VaultLiquidUnstakeAccounts<'_>,
    output_amount: u64,
    max_input: u64,
) -> Result<Instruction> {
    let step = accounts.step;
    let data = buy_data(output_amount, max_input);
    Ok(Instruction {
        program_id: step[0].key(),
        accounts: vec![
            AccountMeta::new(step[1].key(), false),
            AccountMeta::new(step[2].key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new(accounts.user_input.key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new_readonly(accounts.output_mint.key(), false),
            AccountMeta::new_readonly(accounts.input_mint.key(), false),
            AccountMeta::new_readonly(step[8].key(), false),
            AccountMeta::new(step[3].key(), false),
            AccountMeta::new(step[4].key(), false),
            AccountMeta::new(step[5].key(), false),
            AccountMeta::new(step[6].key(), false),
            AccountMeta::new(step[7].key(), false),
            AccountMeta::new_readonly(step[10].key(), false),
            AccountMeta::new_readonly(step[11].key(), false),
            AccountMeta::new_readonly(step[12].key(), false),
            AccountMeta::new_readonly(step[13].key(), false),
            AccountMeta::new_readonly(step[14].key(), false),
        ],
        data,
    })
}

fn build_sell_instruction(
    accounts: &VaultLiquidUnstakeAccounts<'_>,
    input_amount: u64,
) -> Result<Instruction> {
    let step = accounts.step;
    let data = sell_data(input_amount);
    Ok(Instruction {
        program_id: step[0].key(),
        accounts: vec![
            AccountMeta::new(step[1].key(), false),
            AccountMeta::new(step[2].key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new(accounts.user_input.key(), false),
            AccountMeta::new(accounts.user_output.key(), false),
            AccountMeta::new_readonly(accounts.input_mint.key(), false),
            AccountMeta::new_readonly(step[8].key(), false),
            AccountMeta::new(step[3].key(), false),
            AccountMeta::new(step[4].key(), false),
            AccountMeta::new(step[5].key(), false),
            AccountMeta::new(step[7].key(), false),
            AccountMeta::new_readonly(step[10].key(), false),
            AccountMeta::new_readonly(step[12].key(), false),
            AccountMeta::new_readonly(step[14].key(), false),
        ],
        data,
    })
}

fn account_info_for_key<'info>(
    accounts: &VaultLiquidUnstakeAccounts<'info>,
    key: &Pubkey,
) -> Result<AccountInfo<'info>> {
    for account in [
        accounts.payer,
        accounts.user_input,
        accounts.user_output,
        accounts.input_mint,
        accounts.output_mint,
    ] {
        if account.key == key {
            return Ok(account.clone());
        }
    }
    accounts
        .step
        .iter()
        .find(|account| account.key == key)
        .cloned()
        .ok_or_else(|| ArbitrageError::InvalidAccount.into())
}

#[allow(clippy::too_many_arguments)]
pub fn validate_vault_liquid_unstake_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == VAULT_LIQUID_UNSTAKE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        VAULT_LIQUID_UNSTAKE_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[10].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[11].key(),
        anchor_spl::associated_token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[12].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[13].key(),
        anchor_lang::solana_program::sysvar::clock::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[14].key(),
        JITO_SENTINEL,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        input_token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        output_token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    let lst_mint = match direction {
        0 => {
            require_keys_eq!(
                input_mint.key(),
                WSOL_MINT,
                ArbitrageError::InvalidTokenMint
            );
            output_mint
        }
        1 => {
            require_keys_eq!(
                output_mint.key(),
                WSOL_MINT,
                ArbitrageError::InvalidTokenMint
            );
            input_mint
        }
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    for account in [&step[1], &step[3], &step[4], &step[7]] {
        require_keys_eq!(
            *account.owner,
            VAULT_LIQUID_UNSTAKE_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
    }
    require!(
        step[1].data_len() == POOL_ACCOUNT_LEN
            && step[3].data_len() == LST_INFO_ACCOUNT_LEN
            && step[7].data_len() == INVENTORY_SUMMARY_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[4].owner,
        VAULT_LIQUID_UNSTAKE_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require!(step[4].data_is_empty(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[5].owner,
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidAccount
    );
    require!(step[5].data_is_empty(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[8].owner,
        step[9].key(),
        ArbitrageError::InvalidAccount
    );
    require!(step[8].data_len() > 0, ArbitrageError::InvalidAccount);

    let pool_data = step[1].try_borrow_data()?;
    require!(
        pool_data.get(..8) == Some(POOL_DISCRIMINATOR.as_slice()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&pool_data, 40)?,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&pool_data, 104)?,
        step[5].key(),
        ArbitrageError::InvalidAccount
    );
    drop(pool_data);
    let lst_info_data = step[3].try_borrow_data()?;
    require!(
        lst_info_data.get(..8) == Some(LST_INFO_DISCRIMINATOR.as_slice()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&lst_info_data, 8)?,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&lst_info_data, 40)?,
        lst_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&lst_info_data, 72)?,
        step[8].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&lst_info_data, 104)?,
        step[9].key(),
        ArbitrageError::InvalidProgramId
    );
    drop(lst_info_data);
    let expected_lst_info = Pubkey::find_program_address(
        &[b"lst_info", step[1].key().as_ref(), lst_mint.key().as_ref()],
        &VAULT_LIQUID_UNSTAKE_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(
        step[3].key(),
        expected_lst_info,
        ArbitrageError::InvalidAccount
    );
    let expected_inventory = Pubkey::find_program_address(
        &[b"inventory_summary", step[1].key().as_ref()],
        &VAULT_LIQUID_UNSTAKE_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(
        step[7].key(),
        expected_inventory,
        ArbitrageError::InvalidAccount
    );
    let inventory_data = step[7].try_borrow_data()?;
    require!(
        inventory_data.get(..8) == Some(INVENTORY_SUMMARY_DISCRIMINATOR.as_slice()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&inventory_data, 8)?,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    drop(inventory_data);
    let expected_lst_vault = associated_token_address(&step[1].key(), &lst_mint.key());
    let expected_wsol = associated_token_address(&step[1].key(), &WSOL_MINT);
    require_keys_eq!(
        step[2].key(),
        expected_lst_vault,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[6].key(), expected_wsol, ArbitrageError::InvalidAccount);
    validate_token_account_for_mint_and_authority(&step[2], lst_mint, &step[10], &step[1])?;
    require!(
        read_token_amount(&step[2])? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
}

fn associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            anchor_spl::token::ID.as_ref(),
            mint.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0
}

fn buy_data(output_amount: u64, max_input: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(&BUY_LST_DISCRIMINATOR);
    data.extend_from_slice(&output_amount.to_le_bytes());
    data.push(1);
    data.extend_from_slice(&max_input.to_le_bytes());
    data
}

fn sell_data(input_amount: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(&SELL_LST_DISCRIMINATOR);
    data.extend_from_slice(&input_amount.to_le_bytes());
    data.push(0);
    data
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_match_confirmed_anchor_wires() {
        let buy = buy_data(77_532, 100_000);
        assert_eq!(buy.len(), 25);
        assert_eq!(&buy[..8], &BUY_LST_DISCRIMINATOR);
        assert_eq!(&buy[8..16], &77_532_u64.to_le_bytes());
        assert_eq!(buy[16], 1);
        assert_eq!(&buy[17..], &100_000_u64.to_le_bytes());

        let sell = sell_data(27_783_940);
        assert_eq!(sell.len(), 17);
        assert_eq!(&sell[..8], &SELL_LST_DISCRIMINATOR);
        assert_eq!(&sell[8..16], &27_783_940_u64.to_le_bytes());
        assert_eq!(sell[16], 0);
    }
}
