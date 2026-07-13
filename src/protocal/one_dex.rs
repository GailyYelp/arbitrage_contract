use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{
        read_token_amount, read_token_owner_from_data, token_balance_delta,
        validate_token_account_for_mint, validate_token_account_for_mint_and_authority, SwapResult,
    },
};

pub const ONE_DEX_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("DEXYosS6oEGvk8uCDayvwEZz4qEyDJRf9nFgYCaqPMTm");
pub const ONE_DEX_STEP_ACCOUNTS: usize = 8;
const METADATA_STATE: Pubkey = anchor_lang::pubkey!("5nmAbnjJfW1skrPvYjLTBNdhoKzJfznnbvDcM8G2U7Ki");
const FEE_AUTHORITY: Pubkey = anchor_lang::pubkey!("ATowQwFzdJBJ9VFSfoNKmuB8GiSeo8foM5vRriwmKmFB");
const POOL_LEN: usize = 432;
const METADATA_LEN: usize = 232;
const POOL_DISCRIMINATOR: [u8; 8] = [247, 237, 227, 245, 215, 195, 222, 70];
const METADATA_DISCRIMINATOR: [u8; 8] = [74, 68, 237, 189, 45, 190, 195, 21];
const SWAP_DISCRIMINATOR: [u8; 8] = [8, 151, 245, 76, 172, 203, 144, 39];
const AUTHORITY_OFFSET: usize = 8;
const TOKEN_0_MINT_OFFSET: usize = 89;
const TOKEN_0_VAULT_OFFSET: usize = 121;
const TOKEN_0_BALANCE_OFFSET: usize = 153;
const TOKEN_0_WEIGHT_OFFSET: usize = 161;
const TOKEN_1_MINT_OFFSET: usize = 169;
const TOKEN_1_VAULT_OFFSET: usize = 201;
const TOKEN_1_BALANCE_OFFSET: usize = 233;
const TOKEN_1_WEIGHT_OFFSET: usize = 241;
const TOTAL_WEIGHT_OFFSET: usize = 409;
const SWAP_FEE_OFFSET: usize = 417;
const FEE_TIERS_OFFSET: usize = 74;
const FEE_TIER_COUNT: usize = 8;
const PROTOCOL_SHARE_OFFSET: usize = 202;
const REFERRER_SHARE_OFFSET: usize = 210;
const FEE_SCALE: u64 = 1_000_000_000;
const TOTAL_WEIGHT: u64 = 100_000_000_000;
const EQUAL_WEIGHT: u64 = TOTAL_WEIGHT / 2;

pub struct OneDexAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub metadata: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub authority: &'info AccountInfo<'info>,
    pub vault_0: &'info AccountInfo<'info>,
    pub vault_1: &'info AccountInfo<'info>,
    pub fee_account: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
}

pub fn one_dex_swap<'info>(
    accounts: OneDexAccounts<'info>,
    direction: u8,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let (input_vault, output_vault) = if direction == 0 {
        (accounts.vault_0, accounts.vault_1)
    } else {
        (accounts.vault_1, accounts.vault_0)
    };
    let pre_out = read_token_amount(accounts.user_output)?;
    let mut data = SWAP_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    let metas = vec![
        AccountMeta::new_readonly(accounts.metadata.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.authority.key(), false),
        AccountMeta::new(input_vault.key(), false),
        AccountMeta::new(output_vault.key(), false),
        AccountMeta::new(accounts.user.key(), true),
        AccountMeta::new(accounts.user_input.key(), false),
        AccountMeta::new(accounts.user_output.key(), false),
        AccountMeta::new(accounts.fee_account.key(), false),
        AccountMeta::new(accounts.fee_account.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    let infos = vec![
        accounts.metadata.clone(),
        accounts.pool.clone(),
        accounts.authority.clone(),
        input_vault.clone(),
        output_vault.clone(),
        accounts.user.clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.fee_account.clone(),
        accounts.fee_account.clone(),
        accounts.token_program.clone(),
        accounts.program.clone(),
    ];
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

pub fn validate_one_dex_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == ONE_DEX_STEP_ACCOUNTS && direction <= 1 && fee_rate == 0,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step[0].key(),
        ONE_DEX_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[1].key(),
        METADATA_STATE,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        ONE_DEX_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        ONE_DEX_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[7].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );

    let metadata = step[1].try_borrow_data()?;
    require!(
        metadata.len() == METADATA_LEN && metadata.starts_with(&METADATA_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let protocol_share = read_u64(&metadata, PROTOCOL_SHARE_OFFSET)?;
    let referrer_share = read_u64(&metadata, REFERRER_SHARE_OFFSET)?;
    require!(
        protocol_share <= FEE_SCALE
            && referrer_share <= FEE_SCALE
            && protocol_share
                .checked_add(referrer_share)
                .is_some_and(|value| value <= FEE_SCALE),
        ArbitrageError::InvalidAccount
    );

    let pool = step[2].try_borrow_data()?;
    require!(
        pool.len() == POOL_LEN && pool.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let authority = read_pubkey(&pool, AUTHORITY_OFFSET)?;
    let mint_0 = read_pubkey(&pool, TOKEN_0_MINT_OFFSET)?;
    let mint_1 = read_pubkey(&pool, TOKEN_1_MINT_OFFSET)?;
    let vault_0 = read_pubkey(&pool, TOKEN_0_VAULT_OFFSET)?;
    let vault_1 = read_pubkey(&pool, TOKEN_1_VAULT_OFFSET)?;
    let balance_0 = read_u64(&pool, TOKEN_0_BALANCE_OFFSET)?;
    let balance_1 = read_u64(&pool, TOKEN_1_BALANCE_OFFSET)?;
    let swap_fee = read_u64(&pool, SWAP_FEE_OFFSET)?;
    let mut enabled_fee = false;
    for index in 0..FEE_TIER_COUNT {
        let tier = read_u64(&metadata, FEE_TIERS_OFFSET + index * 8)?;
        require!(tier > 0 && tier < FEE_SCALE, ArbitrageError::InvalidAccount);
        enabled_fee |= tier == swap_fee;
    }
    require!(
        authority != Pubkey::default()
            && mint_0 != mint_1
            && vault_0 != vault_1
            && balance_0 > 0
            && balance_1 > 0
            && read_u64(&pool, TOKEN_0_WEIGHT_OFFSET)? == EQUAL_WEIGHT
            && read_u64(&pool, TOKEN_1_WEIGHT_OFFSET)? == EQUAL_WEIGHT
            && read_u64(&pool, TOTAL_WEIGHT_OFFSET)? == TOTAL_WEIGHT
            && swap_fee > 0
            && swap_fee < FEE_SCALE
            && enabled_fee,
        ArbitrageError::InvalidAccount
    );
    drop(metadata);
    drop(pool);

    require_keys_eq!(authority, step[3].key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(vault_0, step[4].key(), ArbitrageError::InvalidAccount);
    require_keys_eq!(vault_1, step[5].key(), ArbitrageError::InvalidAccount);
    let (expected_input, expected_output) = if direction == 0 {
        (mint_0, mint_1)
    } else {
        (mint_1, mint_0)
    };
    require_keys_eq!(
        expected_input,
        input_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        expected_output,
        output_mint.key(),
        ArbitrageError::InvalidTokenMint
    );
    validate_token_account_for_mint_and_authority(
        &step[4],
        if direction == 0 {
            input_mint
        } else {
            output_mint
        },
        &step[7],
        &step[3],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[5],
        if direction == 0 {
            output_mint
        } else {
            input_mint
        },
        &step[7],
        &step[3],
    )?;
    require!(
        read_token_amount(&step[4])? >= balance_0 && read_token_amount(&step[5])? >= balance_1,
        ArbitrageError::InsufficientLiquidity
    );
    validate_token_account_for_mint(&step[6], input_mint, &step[7])?;
    let fee_data = step[6].try_borrow_data()?;
    require_keys_eq!(
        read_token_owner_from_data(&fee_data)?,
        FEE_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    drop(fee_data);
    require_keys_eq!(
        step[6].key(),
        associated_token_address(&FEE_AUTHORITY, &input_mint.key()),
        ArbitrageError::InvalidAccount
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

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(bytes);
    Ok(Pubkey::new_from_array(raw))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 8];
    raw.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(raw))
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

    fn write_pubkey(data: &mut [u8], offset: usize, key: Pubkey) {
        data[offset..offset + 32].copy_from_slice(key.as_ref());
    }

    fn token_data(mint: Pubkey, authority: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 165];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1;
        data
    }

    fn semantic_fixture(
        direction: u8,
    ) -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let pool = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let mint_0 = Pubkey::new_unique();
        let mint_1 = Pubkey::new_unique();
        let vault_0 = Pubkey::new_unique();
        let vault_1 = Pubkey::new_unique();
        let token_program = anchor_spl::token::ID;
        let balance_0 = 31_010_931_108_u64;
        let balance_1 = 404_006_649_815_u64;

        let mut metadata = vec![0_u8; METADATA_LEN];
        metadata[..8].copy_from_slice(&METADATA_DISCRIMINATOR);
        for (index, tier) in [
            100_000_u64,
            200_000,
            400_000,
            500_000,
            1_600_000,
            3_000_000,
            10_000_000,
            20_000_000,
        ]
        .into_iter()
        .enumerate()
        {
            let offset = FEE_TIERS_OFFSET + index * 8;
            metadata[offset..offset + 8].copy_from_slice(&tier.to_le_bytes());
        }
        metadata[PROTOCOL_SHARE_OFFSET..PROTOCOL_SHARE_OFFSET + 8]
            .copy_from_slice(&100_000_000_u64.to_le_bytes());
        metadata[REFERRER_SHARE_OFFSET..REFERRER_SHARE_OFFSET + 8]
            .copy_from_slice(&100_000_000_u64.to_le_bytes());

        let mut pool_data = vec![0_u8; POOL_LEN];
        pool_data[..8].copy_from_slice(&POOL_DISCRIMINATOR);
        write_pubkey(&mut pool_data, AUTHORITY_OFFSET, authority);
        write_pubkey(&mut pool_data, TOKEN_0_MINT_OFFSET, mint_0);
        write_pubkey(&mut pool_data, TOKEN_0_VAULT_OFFSET, vault_0);
        pool_data[TOKEN_0_BALANCE_OFFSET..TOKEN_0_BALANCE_OFFSET + 8]
            .copy_from_slice(&balance_0.to_le_bytes());
        pool_data[TOKEN_0_WEIGHT_OFFSET..TOKEN_0_WEIGHT_OFFSET + 8]
            .copy_from_slice(&EQUAL_WEIGHT.to_le_bytes());
        write_pubkey(&mut pool_data, TOKEN_1_MINT_OFFSET, mint_1);
        write_pubkey(&mut pool_data, TOKEN_1_VAULT_OFFSET, vault_1);
        pool_data[TOKEN_1_BALANCE_OFFSET..TOKEN_1_BALANCE_OFFSET + 8]
            .copy_from_slice(&balance_1.to_le_bytes());
        pool_data[TOKEN_1_WEIGHT_OFFSET..TOKEN_1_WEIGHT_OFFSET + 8]
            .copy_from_slice(&EQUAL_WEIGHT.to_le_bytes());
        pool_data[TOTAL_WEIGHT_OFFSET..TOTAL_WEIGHT_OFFSET + 8]
            .copy_from_slice(&TOTAL_WEIGHT.to_le_bytes());
        pool_data[SWAP_FEE_OFFSET..SWAP_FEE_OFFSET + 8].copy_from_slice(&100_000_u64.to_le_bytes());

        let (input_mint, output_mint) = if direction == 0 {
            (mint_0, mint_1)
        } else {
            (mint_1, mint_0)
        };
        let fee_account = associated_token_address(&FEE_AUTHORITY, &input_mint);
        let step = vec![
            account(ONE_DEX_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(METADATA_STATE, ONE_DEX_PROGRAM_ID, metadata, false, false),
            account(pool, ONE_DEX_PROGRAM_ID, pool_data, true, false),
            account(authority, Pubkey::default(), vec![], false, false),
            account(
                vault_0,
                token_program,
                token_data(mint_0, authority, balance_0),
                true,
                false,
            ),
            account(
                vault_1,
                token_program,
                token_data(mint_1, authority, balance_1),
                true,
                false,
            ),
            account(
                fee_account,
                token_program,
                token_data(input_mint, FEE_AUTHORITY, 0),
                true,
                false,
            ),
            account(token_program, Pubkey::default(), vec![], false, true),
        ];
        (
            step,
            account(input_mint, token_program, vec![], false, false),
            account(output_mint, token_program, vec![], false, false),
        )
    }

    #[test]
    fn native_wire_matches_confirmed_anchor_instruction() {
        let mut data = SWAP_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&62_109_041_u64.to_le_bytes());
        data.extend_from_slice(&0_u64.to_le_bytes());
        assert_eq!(data.len(), 24);
        assert_eq!(
            data,
            [
                8, 151, 245, 76, 172, 203, 144, 39, 113, 181, 179, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0,
            ]
        );
    }

    #[test]
    fn semantic_validation_binds_weights_vaults_fees_and_both_directions() {
        for direction in [0, 1] {
            let (step, input_mint, output_mint) = semantic_fixture(direction);
            assert!(validate_one_dex_semantic_accounts(
                &step,
                direction,
                0,
                &input_mint,
                &output_mint,
            )
            .is_ok());
        }

        let (step, input_mint, output_mint) = semantic_fixture(0);
        step[2].try_borrow_mut_data().unwrap()[TOKEN_0_WEIGHT_OFFSET] ^= 1;
        assert!(
            validate_one_dex_semantic_accounts(&step, 0, 0, &input_mint, &output_mint).is_err()
        );
    }
}
