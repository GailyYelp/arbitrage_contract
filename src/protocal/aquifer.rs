use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program::invoke,
        sysvar::instructions::ID as INSTRUCTIONS_SYSVAR_ID,
    },
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::{AQUIFER_ORACLE_PROGRAM_ID, AQUIFER_PROGRAM_ID},
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const AQUIFER_STEP_ACCOUNTS: usize = 10;

const DEX_ACCOUNT_LEN: usize = 8_552;
const INSTANCE_ACCOUNT_LEN: usize = 8_492;
const COIN_ACCOUNT_LEN: usize = 1_056;
const ORACLE_ACCOUNT_LEN: usize = 128;
const INSTANCE_DEX_OFFSET: usize = 0;
const ORACLE_SLOT_OFFSET: usize = 8;
const ORACLE_MINT_OFFSET: usize = 24;
const ORACLE_DEX_OFFSET: usize = 56;
const COIN_ORACLE_OFFSET: usize = 920;
const COIN_MINT_OFFSET: usize = 952;
const COIN_VAULT_OFFSET: usize = 984;
const COIN_INSTANCE_OFFSET: usize = 1_016;
const SWAP_SELECTOR: u8 = 1;
const SWAP_DATA_LEN: usize = 9;

pub struct AquiferAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub dex: &'info AccountInfo<'info>,
    pub instance: &'info AccountInfo<'info>,
    pub mint_a: &'info AccountInfo<'info>,
    pub token_program_a: &'info AccountInfo<'info>,
    pub oracle_a: &'info AccountInfo<'info>,
    pub coin_a: &'info AccountInfo<'info>,
    pub vault_a: &'info AccountInfo<'info>,
    pub mint_b: &'info AccountInfo<'info>,
    pub token_program_b: &'info AccountInfo<'info>,
    pub oracle_b: &'info AccountInfo<'info>,
    pub coin_b: &'info AccountInfo<'info>,
    pub vault_b: &'info AccountInfo<'info>,
    pub user_source: &'info AccountInfo<'info>,
    pub user_destination: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn aquifer_swap<'info>(
    accounts: AquiferAccounts<'info>,
    amount_in: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let instruction = build_aquifer_swap_instruction(&accounts, amount_in, direction)?;
    let (input_mint, input_program, input_oracle, input_coin, input_vault) =
        input_accounts(&accounts, direction)?;
    let (output_mint, output_program, output_oracle, output_coin, output_vault) =
        output_accounts(&accounts, direction)?;
    let infos = vec![
        accounts.instructions_sysvar.clone(),
        accounts.payer.clone(),
        output_program.clone(),
        accounts.user_destination.clone(),
        output_mint.clone(),
        input_program.clone(),
        accounts.user_source.clone(),
        input_mint.clone(),
        accounts.dex.clone(),
        accounts.instance.clone(),
        output_oracle.clone(),
        input_oracle.clone(),
        output_coin.clone(),
        output_vault.clone(),
        input_coin.clone(),
        input_vault.clone(),
        accounts.program.clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn build_aquifer_swap_instruction(
    accounts: &AquiferAccounts<'_>,
    amount_in: u64,
    direction: u8,
) -> Result<Instruction> {
    let (input_mint, input_program, input_oracle, input_coin, input_vault) =
        input_accounts(accounts, direction)?;
    let (output_mint, output_program, output_oracle, output_coin, output_vault) =
        output_accounts(accounts, direction)?;
    let mut data = Vec::with_capacity(SWAP_DATA_LEN);
    data.push(SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    Ok(Instruction {
        program_id: accounts.program.key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
            AccountMeta::new(accounts.payer.key(), true),
            AccountMeta::new_readonly(output_program.key(), false),
            AccountMeta::new(accounts.user_destination.key(), false),
            AccountMeta::new_readonly(output_mint.key(), false),
            AccountMeta::new_readonly(input_program.key(), false),
            AccountMeta::new(accounts.user_source.key(), false),
            AccountMeta::new_readonly(input_mint.key(), false),
            AccountMeta::new(accounts.dex.key(), false),
            AccountMeta::new(accounts.instance.key(), false),
            AccountMeta::new_readonly(output_oracle.key(), false),
            AccountMeta::new_readonly(input_oracle.key(), false),
            AccountMeta::new(output_coin.key(), false),
            AccountMeta::new(output_vault.key(), false),
            AccountMeta::new(input_coin.key(), false),
            AccountMeta::new(input_vault.key(), false),
        ],
        data,
    })
}

type DirectionalAccounts<'a, 'info> = (
    &'a AccountInfo<'info>,
    &'a AccountInfo<'info>,
    &'a AccountInfo<'info>,
    &'a AccountInfo<'info>,
    &'a AccountInfo<'info>,
);

fn input_accounts<'a, 'info>(
    accounts: &'a AquiferAccounts<'info>,
    direction: u8,
) -> Result<DirectionalAccounts<'a, 'info>> {
    match direction {
        0 => Ok((
            accounts.mint_a,
            accounts.token_program_a,
            accounts.oracle_a,
            accounts.coin_a,
            accounts.vault_a,
        )),
        1 => Ok((
            accounts.mint_b,
            accounts.token_program_b,
            accounts.oracle_b,
            accounts.coin_b,
            accounts.vault_b,
        )),
        _ => Err(ArbitrageError::InvalidInstructionData.into()),
    }
}

fn output_accounts<'a, 'info>(
    accounts: &'a AquiferAccounts<'info>,
    direction: u8,
) -> Result<DirectionalAccounts<'a, 'info>> {
    match direction {
        0 => Ok((
            accounts.mint_b,
            accounts.token_program_b,
            accounts.oracle_b,
            accounts.coin_b,
            accounts.vault_b,
        )),
        1 => Ok((
            accounts.mint_a,
            accounts.token_program_a,
            accounts.oracle_a,
            accounts.coin_a,
            accounts.vault_a,
        )),
        _ => Err(ArbitrageError::InvalidInstructionData.into()),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn validate_aquifer_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == AQUIFER_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        AQUIFER_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[9].key(),
        INSTRUCTIONS_SYSVAR_ID,
        ArbitrageError::InvalidProgramId
    );
    for account in [&step[1], &step[2], &step[5], &step[7]] {
        require_keys_eq!(
            *account.owner,
            AQUIFER_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
    }
    for account in [&step[3], &step[4]] {
        require_keys_eq!(
            *account.owner,
            AQUIFER_ORACLE_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
    }
    require!(
        step[1].data_len() == DEX_ACCOUNT_LEN
            && step[2].data_len() == INSTANCE_ACCOUNT_LEN
            && step[3].data_len() == ORACLE_ACCOUNT_LEN
            && step[4].data_len() == ORACLE_ACCOUNT_LEN
            && step[5].data_len() == COIN_ACCOUNT_LEN
            && step[7].data_len() == COIN_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    require!(
        matches!(
            input_token_program.key(),
            anchor_spl::token::ID | anchor_spl::token_2022::ID
        ) && matches!(
            output_token_program.key(),
            anchor_spl::token::ID | anchor_spl::token_2022::ID
        ),
        ArbitrageError::InvalidProgramId
    );

    let dex = step[1].try_borrow_data()?;
    require!(read_u64(&dex, 0)? == 1, ArbitrageError::InvalidAccount);
    drop(dex);
    let instance = step[2].try_borrow_data()?;
    require_keys_eq!(
        read_pubkey(&instance, INSTANCE_DEX_OFFSET)?,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    drop(instance);

    let (mint_a, mint_b, program_a, program_b) = match direction {
        0 => (
            input_mint,
            output_mint,
            input_token_program,
            output_token_program,
        ),
        1 => (
            output_mint,
            input_mint,
            output_token_program,
            input_token_program,
        ),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    validate_coin(&step[5], &step[3], mint_a, &step[6], &step[2], &step[1])?;
    validate_coin(&step[7], &step[4], mint_b, &step[8], &step[2], &step[1])?;
    validate_token_account_for_mint_and_authority(&step[6], mint_a, program_a, &step[5])?;
    validate_token_account_for_mint_and_authority(&step[8], mint_b, program_b, &step[7])?;
    require!(
        read_token_amount(&step[6])? > 0 && read_token_amount(&step[8])? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
}

fn validate_coin(
    coin: &AccountInfo<'_>,
    oracle: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    vault: &AccountInfo<'_>,
    instance: &AccountInfo<'_>,
    dex: &AccountInfo<'_>,
) -> Result<()> {
    let coin_data = coin.try_borrow_data()?;
    require_keys_eq!(
        read_pubkey(&coin_data, COIN_ORACLE_OFFSET)?,
        oracle.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&coin_data, COIN_MINT_OFFSET)?,
        mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&coin_data, COIN_VAULT_OFFSET)?,
        vault.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&coin_data, COIN_INSTANCE_OFFSET)?,
        instance.key(),
        ArbitrageError::InvalidAccount
    );
    drop(coin_data);
    let oracle_data = oracle.try_borrow_data()?;
    require!(
        read_u64(&oracle_data, ORACLE_SLOT_OFFSET)? > 0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&oracle_data, ORACLE_MINT_OFFSET)?,
        mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&oracle_data, ORACLE_DEX_OFFSET)?,
        dex.key(),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 32] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 8] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(key: Pubkey) -> AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(Pubkey::default()));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        AccountInfo::new(key, false, false, lamports, data, owner, false, 0)
    }

    #[test]
    fn wire_matches_observed_sixteen_account_layout() {
        let accounts: &'static [AccountInfo<'static>] = Box::leak(
            (1_u8..=18)
                .map(|seed| account(Pubkey::new_from_array([seed; 32])))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let aquifer = AquiferAccounts {
            program: &accounts[0],
            payer: &accounts[1],
            dex: &accounts[2],
            instance: &accounts[3],
            mint_a: &accounts[4],
            token_program_a: &accounts[5],
            oracle_a: &accounts[6],
            coin_a: &accounts[7],
            vault_a: &accounts[8],
            mint_b: &accounts[9],
            token_program_b: &accounts[10],
            oracle_b: &accounts[11],
            coin_b: &accounts[12],
            vault_b: &accounts[13],
            user_source: &accounts[14],
            user_destination: &accounts[15],
            instructions_sysvar: &accounts[16],
            output_token_account: &accounts[17],
        };
        let amount = 771_434_613_u64;
        let forward = build_aquifer_swap_instruction(&aquifer, amount, 0).unwrap();
        assert_eq!(forward.accounts.len(), 16);
        assert_eq!(
            forward.data,
            [vec![1], amount.to_le_bytes().to_vec()].concat()
        );
        assert_eq!(forward.accounts[4].pubkey, aquifer.mint_b.key());
        assert_eq!(forward.accounts[7].pubkey, aquifer.mint_a.key());
        assert_eq!(forward.accounts[12].pubkey, aquifer.coin_b.key());
        assert_eq!(forward.accounts[14].pubkey, aquifer.coin_a.key());

        let reverse = build_aquifer_swap_instruction(&aquifer, amount, 1).unwrap();
        assert_eq!(reverse.accounts[4].pubkey, aquifer.mint_a.key());
        assert_eq!(reverse.accounts[7].pubkey, aquifer.mint_b.key());
        assert_eq!(reverse.accounts[12].pubkey, aquifer.coin_a.key());
        assert_eq!(reverse.accounts[14].pubkey, aquifer.coin_b.key());
    }
}
