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
        program_ids::{MEMO_PROGRAM_V2_ID, SCORCH_ORACLE_PROGRAM_ID, SCORCH_PROGRAM_ID},
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const SCORCH_STEP_ACCOUNTS: usize = 11;
pub const SCORCH_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("EHcege7dok1iYs7SxL2XzDPvhg6XzMVcx2V5SkMUurJP");
pub const SCORCH_GLOBAL: Pubkey =
    anchor_lang::pubkey!("HLixVmXdBqzP1sXT9au4BHcvUjDgx5ev16cEJdd9tUSM");

const AUTHORITY_ACCOUNT_LEN: usize = 800;
const GLOBAL_ACCOUNT_LEN: usize = 4_048;
const POOL_ACCOUNT_LEN: usize = 658;
const ORACLE_ACCOUNT_LEN: usize = 592;
const MINT_A_OFFSET: usize = 16;
const VAULT_A_OFFSET: usize = 48;
const MINT_B_OFFSET: usize = 82;
const VAULT_B_OFFSET: usize = 114;
const ORACLE_MINT_OFFSET: usize = 16;
const ORACLE_VAULT_OFFSET: usize = 48;
const SWAP_SELECTOR: u8 = 2;
const PROTOCOL_PAYLOAD_LEN: usize = 18;
const SWAP_DATA_LEN: usize = 34;

pub struct ScorchAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub oracle_program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub mint_a: &'info AccountInfo<'info>,
    pub mint_b: &'info AccountInfo<'info>,
    pub vault_a: &'info AccountInfo<'info>,
    pub vault_b: &'info AccountInfo<'info>,
    pub user_source: &'info AccountInfo<'info>,
    pub user_destination: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub global: &'info AccountInfo<'info>,
    pub oracle_a: &'info AccountInfo<'info>,
    pub oracle_b: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub memo_program: &'info AccountInfo<'info>,
    pub instructions_sysvar: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn scorch_swap<'info>(
    accounts: ScorchAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
    protocol_payload: [u8; PROTOCOL_PAYLOAD_LEN],
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(
        protocol_payload[0] == SWAP_SELECTOR,
        ArbitrageError::InvalidInstructionData
    );
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let instruction = build_scorch_swap_instruction(
        &accounts,
        amount_in,
        min_amount_out,
        direction,
        protocol_payload,
    )?;
    let (input_mint, output_mint, input_vault, output_vault, input_oracle, output_oracle) =
        directional_accounts(&accounts, direction)?;
    let infos = vec![
        accounts.authority.clone(),
        accounts.payer.clone(),
        accounts.user_source.clone(),
        accounts.user_destination.clone(),
        input_vault.clone(),
        output_vault.clone(),
        input_mint.clone(),
        output_mint.clone(),
        accounts.token_program.clone(),
        accounts.token_program.clone(),
        accounts.memo_program.clone(),
        accounts.oracle_program.clone(),
        accounts.global.clone(),
        input_oracle.clone(),
        output_oracle.clone(),
        accounts.pool.clone(),
        accounts.instructions_sysvar.clone(),
        accounts.program.clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn build_scorch_swap_instruction(
    accounts: &ScorchAccounts<'_>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
    protocol_payload: [u8; PROTOCOL_PAYLOAD_LEN],
) -> Result<Instruction> {
    let (input_mint, output_mint, input_vault, output_vault, input_oracle, output_oracle) =
        directional_accounts(accounts, direction)?;
    let mut data = Vec::with_capacity(SWAP_DATA_LEN);
    data.extend_from_slice(&protocol_payload);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    require!(
        data.len() == SWAP_DATA_LEN,
        ArbitrageError::InvalidInstructionData
    );
    Ok(Instruction {
        program_id: accounts.program.key(),
        accounts: vec![
            AccountMeta::new_readonly(accounts.authority.key(), false),
            AccountMeta::new_readonly(accounts.payer.key(), true),
            AccountMeta::new(accounts.user_source.key(), false),
            AccountMeta::new(accounts.user_destination.key(), false),
            AccountMeta::new(input_vault.key(), false),
            AccountMeta::new(output_vault.key(), false),
            AccountMeta::new_readonly(input_mint.key(), false),
            AccountMeta::new_readonly(output_mint.key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(accounts.token_program.key(), false),
            AccountMeta::new_readonly(accounts.memo_program.key(), false),
            AccountMeta::new_readonly(accounts.oracle_program.key(), false),
            AccountMeta::new_readonly(accounts.global.key(), false),
            AccountMeta::new(input_oracle.key(), false),
            AccountMeta::new(output_oracle.key(), false),
            AccountMeta::new(accounts.pool.key(), false),
            AccountMeta::new_readonly(accounts.instructions_sysvar.key(), false),
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
    &'a AccountInfo<'info>,
);

fn directional_accounts<'a, 'info>(
    accounts: &'a ScorchAccounts<'info>,
    direction: u8,
) -> Result<DirectionalAccounts<'a, 'info>> {
    match direction {
        0 => Ok((
            accounts.mint_a,
            accounts.mint_b,
            accounts.vault_a,
            accounts.vault_b,
            accounts.oracle_a,
            accounts.oracle_b,
        )),
        1 => Ok((
            accounts.mint_b,
            accounts.mint_a,
            accounts.vault_b,
            accounts.vault_a,
            accounts.oracle_b,
            accounts.oracle_a,
        )),
        _ => Err(ArbitrageError::InvalidInstructionData.into()),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn validate_scorch_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    protocol_payload: &[u8; PROTOCOL_PAYLOAD_LEN],
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == SCORCH_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require!(
        protocol_payload[0] == SWAP_SELECTOR,
        ArbitrageError::InvalidInstructionData
    );
    require_keys_eq!(
        step[0].key(),
        SCORCH_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[1].key(),
        SCORCH_ORACLE_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[5].key(),
        SCORCH_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[6].key(), SCORCH_GLOBAL, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[9].key(),
        MEMO_PROGRAM_V2_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[10].key(),
        INSTRUCTIONS_SYSVAR_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[2].owner,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[5].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    for dependency in [&step[6], &step[7], &step[8]] {
        require_keys_eq!(
            *dependency.owner,
            step[1].key(),
            ArbitrageError::InvalidAccount
        );
    }
    require!(
        step[5].data_len() == AUTHORITY_ACCOUNT_LEN && step[6].data_len() == GLOBAL_ACCOUNT_LEN,
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

    let pool = step[2].try_borrow_data()?;
    require!(
        pool.len() == POOL_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    let mint_a = read_pubkey(&pool, MINT_A_OFFSET)?;
    let vault_a = read_pubkey(&pool, VAULT_A_OFFSET)?;
    let mint_b = read_pubkey(&pool, MINT_B_OFFSET)?;
    let vault_b = read_pubkey(&pool, VAULT_B_OFFSET)?;
    drop(pool);
    require!(
        ![mint_a, vault_a, mint_b, vault_b].contains(&Pubkey::default())
            && mint_a != mint_b
            && vault_a != vault_b,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[3].key(), vault_a, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[4].key(), vault_b, ArbitrageError::InvalidAccount);
    validate_oracle(&step[7], mint_a, vault_a)?;
    validate_oracle(&step[8], mint_b, vault_b)?;

    let (expected_input, expected_output, mint_a_account, mint_b_account, program_a, program_b) =
        match direction {
            0 => (
                mint_a,
                mint_b,
                input_mint,
                output_mint,
                input_token_program,
                output_token_program,
            ),
            1 => (
                mint_b,
                mint_a,
                output_mint,
                input_mint,
                output_token_program,
                input_token_program,
            ),
            _ => return Err(ArbitrageError::InvalidInstructionData.into()),
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
    validate_token_account_for_mint_and_authority(&step[3], mint_a_account, program_a, &step[5])?;
    validate_token_account_for_mint_and_authority(&step[4], mint_b_account, program_b, &step[5])?;
    require!(
        read_token_amount(&step[3])? > 0 && read_token_amount(&step[4])? > 0,
        ArbitrageError::InsufficientLiquidity
    );
    Ok(())
}

fn validate_oracle(account: &AccountInfo<'_>, mint: Pubkey, vault: Pubkey) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.len() == ORACLE_ACCOUNT_LEN,
        ArbitrageError::InvalidAccount
    );
    require!(
        read_pubkey(&data, ORACLE_MINT_OFFSET)? == mint
            && read_pubkey(&data, ORACLE_VAULT_OFFSET)? == vault,
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
    fn wire_matches_confirmed_mainnet_layout_for_both_directions() {
        let accounts: &'static [AccountInfo<'static>] = Box::leak(
            (1_u8..=18)
                .map(|seed| account(Pubkey::new_from_array([seed; 32])))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let scorch = ScorchAccounts {
            program: &accounts[0],
            oracle_program: &accounts[1],
            payer: &accounts[2],
            pool: &accounts[3],
            mint_a: &accounts[4],
            mint_b: &accounts[5],
            vault_a: &accounts[6],
            vault_b: &accounts[7],
            user_source: &accounts[8],
            user_destination: &accounts[9],
            authority: &accounts[10],
            global: &accounts[11],
            oracle_a: &accounts[12],
            oracle_b: &accounts[13],
            token_program: &accounts[14],
            memo_program: &accounts[15],
            instructions_sysvar: &accounts[16],
            output_token_account: &accounts[17],
        };
        let mut payload = [7_u8; PROTOCOL_PAYLOAD_LEN];
        payload[0] = SWAP_SELECTOR;
        let amount_in = 80_252_918_u64;
        let min_out = 221_000_000_u64;

        let forward =
            build_scorch_swap_instruction(&scorch, amount_in, min_out, 0, payload).unwrap();
        assert_eq!(forward.accounts.len(), 17);
        assert_eq!(forward.data.len(), SWAP_DATA_LEN);
        assert_eq!(&forward.data[..18], &payload);
        assert_eq!(&forward.data[18..26], &amount_in.to_le_bytes());
        assert_eq!(&forward.data[26..34], &min_out.to_le_bytes());
        assert_eq!(forward.accounts[4].pubkey, scorch.vault_a.key());
        assert_eq!(forward.accounts[5].pubkey, scorch.vault_b.key());
        assert_eq!(forward.accounts[13].pubkey, scorch.oracle_a.key());
        assert_eq!(forward.accounts[14].pubkey, scorch.oracle_b.key());
        assert_eq!(forward.accounts[15].pubkey, scorch.pool.key());

        let reverse =
            build_scorch_swap_instruction(&scorch, amount_in, min_out, 1, payload).unwrap();
        assert_eq!(reverse.accounts[4].pubkey, scorch.vault_b.key());
        assert_eq!(reverse.accounts[5].pubkey, scorch.vault_a.key());
        assert_eq!(reverse.accounts[6].pubkey, scorch.mint_b.key());
        assert_eq!(reverse.accounts[7].pubkey, scorch.mint_a.key());
        assert_eq!(reverse.accounts[13].pubkey, scorch.oracle_b.key());
        assert_eq!(reverse.accounts[14].pubkey, scorch.oracle_a.key());
    }
}
