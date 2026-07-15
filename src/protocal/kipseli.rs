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
        program_ids::KIPSELI_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const KIPSELI_STEP_ACCOUNTS: usize = 5;
const KIPSELI_POOL_ACCOUNT_LEN: usize = 496;
const KIPSELI_POOL_DISCRIMINATOR: [u8; 8] = [0xf1, 0x9a, 0x6d, 0x04, 0x11, 0xb1, 0x6d, 0xbc];
const KIPSELI_POOL_SEED_KEY_OFFSET: usize = 40;
const KIPSELI_BASE_MINT_OFFSET: usize = 72;
const KIPSELI_QUOTE_MINT_OFFSET: usize = 104;
const KIPSELI_SWAP_DISCRIMINATOR: [u8; 8] = [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62];
const KIPSELI_POOL_SEED: &[u8] = b"pool-config";
const KIPSELI_BAN_SEED: &[u8] = b"ban";

pub struct KipseliAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub pool: &'info AccountInfo<'info>,
    pub base_mint: &'info AccountInfo<'info>,
    pub quote_mint: &'info AccountInfo<'info>,
    pub user_base: &'info AccountInfo<'info>,
    pub user_quote: &'info AccountInfo<'info>,
    pub base_vault: &'info AccountInfo<'info>,
    pub quote_vault: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub ban_entry: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn kipseli_swap<'info>(
    accounts: KipseliAccounts<'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    let base_to_quote = match direction {
        0 => 1,
        1 => 0,
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new(accounts.user_base.key(), false),
        AccountMeta::new(accounts.user_quote.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.payer.key(), false),
        AccountMeta::new_readonly(accounts.ban_entry.key(), false),
        AccountMeta::new_readonly(accounts.ban_entry.key(), false),
    ];
    let infos = vec![
        accounts.payer.clone(),
        accounts.pool.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.user_base.clone(),
        accounts.user_quote.clone(),
        accounts.token_program.clone(),
        accounts.system_program.clone(),
        accounts.payer.clone(),
        accounts.ban_entry.clone(),
        accounts.ban_entry.clone(),
        accounts.program.clone(),
    ];
    let mut data = Vec::with_capacity(25);
    data.extend_from_slice(&KIPSELI_SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.push(base_to_quote);
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn validate_kipseli_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    payer: &AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == KIPSELI_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        KIPSELI_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
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

    let pool = step[1].try_borrow_data()?;
    require!(
        pool.len() == KIPSELI_POOL_ACCOUNT_LEN && pool.starts_with(&KIPSELI_POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let seed_key = read_pubkey(&pool, KIPSELI_POOL_SEED_KEY_OFFSET)?;
    let base_mint = read_pubkey(&pool, KIPSELI_BASE_MINT_OFFSET)?;
    let quote_mint = read_pubkey(&pool, KIPSELI_QUOTE_MINT_OFFSET)?;
    drop(pool);
    require!(
        seed_key != Pubkey::default() && base_mint != quote_mint,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[1].key(),
        Pubkey::find_program_address(&[KIPSELI_POOL_SEED, seed_key.as_ref()], step[0].key).0,
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output, base_mint_account, quote_mint_account) = match direction {
        0 => (base_mint, quote_mint, input_mint, output_mint),
        1 => (quote_mint, base_mint, output_mint, input_mint),
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

    let expected_base_vault = Pubkey::find_program_address(
        &[
            step[1].key().as_ref(),
            anchor_spl::token::ID.as_ref(),
            base_mint.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0;
    let expected_quote_vault = Pubkey::find_program_address(
        &[
            step[1].key().as_ref(),
            anchor_spl::token::ID.as_ref(),
            quote_mint.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0;
    require_keys_eq!(
        step[2].key(),
        expected_base_vault,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].key(),
        expected_quote_vault,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(
        &step[2],
        base_mint_account,
        input_token_program,
        &step[1],
    )?;
    validate_token_account_for_mint_and_authority(
        &step[3],
        quote_mint_account,
        output_token_program,
        &step[1],
    )?;
    require!(
        read_token_amount(&step[2])? > 0 && read_token_amount(&step[3])? > 0,
        ArbitrageError::InsufficientLiquidity
    );

    require_keys_eq!(
        step[4].key(),
        Pubkey::find_program_address(&[KIPSELI_BAN_SEED, payer.key().as_ref()], step[0].key).0,
        ArbitrageError::InvalidAccount
    );
    require!(
        *step[4].owner == anchor_lang::system_program::ID || *step[4].owner == step[0].key(),
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

    const TOKEN_ACCOUNT_LEN: usize = 165;

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
        let mut data = vec![0_u8; TOKEN_ACCOUNT_LEN];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1;
        data
    }

    fn semantic_fixture() -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let payer_key = Pubkey::new_unique();
        let seed_key = Pubkey::new_unique();
        let base_mint_key = Pubkey::new_unique();
        let quote_mint_key = Pubkey::new_unique();
        let (pool_key, _) = Pubkey::find_program_address(
            &[KIPSELI_POOL_SEED, seed_key.as_ref()],
            &KIPSELI_PROGRAM_ID,
        );
        let (base_vault_key, _) = Pubkey::find_program_address(
            &[
                pool_key.as_ref(),
                anchor_spl::token::ID.as_ref(),
                base_mint_key.as_ref(),
            ],
            &anchor_spl::associated_token::ID,
        );
        let (quote_vault_key, _) = Pubkey::find_program_address(
            &[
                pool_key.as_ref(),
                anchor_spl::token::ID.as_ref(),
                quote_mint_key.as_ref(),
            ],
            &anchor_spl::associated_token::ID,
        );
        let ban_key = Pubkey::find_program_address(
            &[KIPSELI_BAN_SEED, payer_key.as_ref()],
            &KIPSELI_PROGRAM_ID,
        )
        .0;
        let mut pool_data = vec![0_u8; KIPSELI_POOL_ACCOUNT_LEN];
        pool_data[..8].copy_from_slice(&KIPSELI_POOL_DISCRIMINATOR);
        write_pubkey(&mut pool_data, KIPSELI_POOL_SEED_KEY_OFFSET, seed_key);
        write_pubkey(&mut pool_data, KIPSELI_BASE_MINT_OFFSET, base_mint_key);
        write_pubkey(&mut pool_data, KIPSELI_QUOTE_MINT_OFFSET, quote_mint_key);

        let step = vec![
            account(KIPSELI_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(pool_key, KIPSELI_PROGRAM_ID, pool_data, true, false),
            account(
                base_vault_key,
                anchor_spl::token::ID,
                token_data(base_mint_key, pool_key, 1_000_000),
                true,
                false,
            ),
            account(
                quote_vault_key,
                anchor_spl::token::ID,
                token_data(quote_mint_key, pool_key, 2_000_000),
                true,
                false,
            ),
            account(
                ban_key,
                anchor_lang::system_program::ID,
                vec![],
                false,
                false,
            ),
        ];
        let payer = account(
            payer_key,
            anchor_lang::system_program::ID,
            vec![],
            true,
            false,
        );
        let base_mint = account(base_mint_key, anchor_spl::token::ID, vec![], false, false);
        let quote_mint = account(quote_mint_key, anchor_spl::token::ID, vec![], false, false);
        let token_program = account(
            anchor_spl::token::ID,
            Pubkey::default(),
            vec![],
            false,
            true,
        );
        (step, payer, base_mint, quote_mint, token_program)
    }

    #[test]
    fn swap_v2_wire_matches_confirmed_mainnet_layout() {
        let mut data = Vec::new();
        data.extend_from_slice(&KIPSELI_SWAP_DISCRIMINATOR);
        data.extend_from_slice(&3_990_580_000_u64.to_le_bytes());
        data.extend_from_slice(&0_u64.to_le_bytes());
        data.push(1);
        assert_eq!(data.len(), 25);
        assert_eq!(&data[..8], &KIPSELI_SWAP_DISCRIMINATOR);
        assert_eq!(&data[8..16], &3_990_580_000_u64.to_le_bytes());
    }

    #[test]
    fn semantic_validation_binds_pool_pda_vault_atas_mints_ban_and_direction() {
        let (step, payer, base_mint, quote_mint, token_program) = semantic_fixture();
        assert!(validate_kipseli_semantic_accounts(
            &step,
            &payer,
            0,
            0,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_ok());
        assert!(validate_kipseli_semantic_accounts(
            &step,
            &payer,
            1,
            0,
            &quote_mint,
            &base_mint,
            &token_program,
            &token_program,
        )
        .is_ok());
        assert!(validate_kipseli_semantic_accounts(
            &step,
            &payer,
            0,
            1,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_err());

        let (bad_pool, payer, base_mint, quote_mint, token_program) = semantic_fixture();
        bad_pool[1].try_borrow_mut_data().unwrap()[KIPSELI_POOL_SEED_KEY_OFFSET] ^= 1;
        assert!(validate_kipseli_semantic_accounts(
            &bad_pool,
            &payer,
            0,
            0,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_err());

        let (bad_vault, payer, base_mint, quote_mint, token_program) = semantic_fixture();
        bad_vault[2].try_borrow_mut_data().unwrap()[0] ^= 1;
        assert!(validate_kipseli_semantic_accounts(
            &bad_vault,
            &payer,
            0,
            0,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_err());

        let (mut bad_ban, payer, base_mint, quote_mint, token_program) = semantic_fixture();
        bad_ban[4] = account(
            Pubkey::new_unique(),
            anchor_lang::system_program::ID,
            vec![],
            false,
            false,
        );
        assert!(validate_kipseli_semantic_accounts(
            &bad_ban,
            &payer,
            0,
            0,
            &base_mint,
            &quote_mint,
            &token_program,
            &token_program,
        )
        .is_err());
    }
}
