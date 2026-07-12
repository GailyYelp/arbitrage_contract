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

pub const TRENDS_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CURVEmPpijXDTNdqrA9PGP1io2rkgiVXH26xdXVGLLfz");
pub const TRENDS_STEP_ACCOUNTS: usize = 7;

const POOL_AUTHORITY: Pubkey = anchor_lang::pubkey!("C6B7knNSF8X8YfW54D8jQvM3VUCifsphHEQZSCyAERoE");
const QUOTE_MINT: Pubkey = anchor_lang::pubkey!("So11111111111111111111111111111111111111112");
const TOKEN_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const POOL_ACCOUNT_LEN: usize = 312;
const CONFIG_ACCOUNT_LEN: usize = 168;
const POOL_DISCRIMINATOR: [u8; 8] = [241, 154, 109, 4, 17, 177, 109, 188];
const CONFIG_DISCRIMINATOR: [u8; 8] = [155, 12, 170, 224, 30, 250, 204, 130];
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const TOTAL_FEE_BPS: u16 = 200;
const MIGRATION_QUOTE_THRESHOLD: u64 = 85_000_000_000;

pub struct TrendsAccounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub config: &'a AccountInfo<'info>,
    pub pool: &'a AccountInfo<'info>,
    pub pool_authority: &'a AccountInfo<'info>,
    pub input_token_account: &'a AccountInfo<'info>,
    pub output_token_account: &'a AccountInfo<'info>,
    pub base_mint: &'a AccountInfo<'info>,
    pub quote_mint: &'a AccountInfo<'info>,
    pub base_vault: &'a AccountInfo<'info>,
    pub quote_vault: &'a AccountInfo<'info>,
    pub trader: &'a AccountInfo<'info>,
    pub base_token_program: &'a AccountInfo<'info>,
    pub quote_token_program: &'a AccountInfo<'info>,
    pub event_authority: &'a AccountInfo<'info>,
}

pub fn trends_swap<'info>(
    accounts: TrendsAccounts<'_, 'info>,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let metas = vec![
        AccountMeta::new_readonly(accounts.config.key(), false),
        AccountMeta::new(accounts.pool.key(), false),
        AccountMeta::new_readonly(accounts.pool_authority.key(), false),
        AccountMeta::new(accounts.input_token_account.key(), false),
        AccountMeta::new(accounts.output_token_account.key(), false),
        AccountMeta::new_readonly(accounts.base_mint.key(), false),
        AccountMeta::new_readonly(accounts.quote_mint.key(), false),
        AccountMeta::new(accounts.base_vault.key(), false),
        AccountMeta::new(accounts.quote_vault.key(), false),
        AccountMeta::new_readonly(accounts.trader.key(), true),
        AccountMeta::new_readonly(accounts.base_token_program.key(), false),
        AccountMeta::new_readonly(accounts.quote_token_program.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
        AccountMeta::new_readonly(accounts.event_authority.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
    ];
    let infos = vec![
        accounts.config.clone(),
        accounts.pool.clone(),
        accounts.pool_authority.clone(),
        accounts.input_token_account.clone(),
        accounts.output_token_account.clone(),
        accounts.base_mint.clone(),
        accounts.quote_mint.clone(),
        accounts.base_vault.clone(),
        accounts.quote_vault.clone(),
        accounts.trader.clone(),
        accounts.base_token_program.clone(),
        accounts.quote_token_program.clone(),
        accounts.program.clone(),
        accounts.event_authority.clone(),
        accounts.program.clone(),
    ];
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: swap_data(amount_in, min_amount_out).to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_trends_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    in_token_program: &AccountInfo<'info>,
    out_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == TRENDS_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(
        direction <= 1 && fee_rate == TOTAL_FEE_BPS,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[0].key(),
        TRENDS_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[1].key(),
        POOL_AUTHORITY,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[2].key(), config_pda(), ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[2].owner,
        TRENDS_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[3].owner,
        TRENDS_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        event_authority(),
        ArbitrageError::InvalidAccount
    );

    let config_data = step[2].try_borrow_data()?;
    require!(
        config_data.len() == CONFIG_ACCOUNT_LEN
            && config_data.starts_with(&CONFIG_DISCRIMINATOR)
            && read_pubkey(&config_data, 8)? != Pubkey::default(),
        ArbitrageError::InvalidAccount
    );
    let pool_data = step[3].try_borrow_data()?;
    require!(
        pool_data.len() == POOL_ACCOUNT_LEN && pool_data.starts_with(&POOL_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let base_mint_key = read_pubkey(&pool_data, 40)?;
    let base_vault_key = read_pubkey(&pool_data, 72)?;
    let quote_vault_key = read_pubkey(&pool_data, 104)?;
    let base_reserve = read_u64(&pool_data, 136)?;
    let quote_reserve = read_u64(&pool_data, 144)?;
    let virtual_base_reserve = read_u64(&pool_data, 152)?;
    let virtual_quote_reserve = read_u64(&pool_data, 160)?;
    let creator_fee = read_u64(&pool_data, 168)?;
    let protocol_fee = read_u64(&pool_data, 176)?;
    let is_migrated = *pool_data.get(184).ok_or(ArbitrageError::InvalidAccount)?;
    require!(
        base_mint_key != Pubkey::default()
            && base_mint_key != QUOTE_MINT
            && base_reserve > 0
            && quote_reserve < MIGRATION_QUOTE_THRESHOLD
            && virtual_base_reserve >= base_reserve
            && virtual_quote_reserve >= quote_reserve
            && is_migrated == 0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].key(),
        pool_pda(base_mint_key),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        base_vault_key,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[5].key(),
        quote_vault_key,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        base_vault_key,
        vault_pda(base_mint_key, step[3].key()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        quote_vault_key,
        vault_pda(QUOTE_MINT, step[3].key()),
        ArbitrageError::InvalidAccount
    );

    let (base_mint, quote_mint, base_program, quote_program) = if direction == 0 {
        (in_mint, out_mint, in_token_program, out_token_program)
    } else {
        (out_mint, in_mint, out_token_program, in_token_program)
    };
    require_keys_eq!(
        base_mint.key(),
        base_mint_key,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(quote_mint.key(), QUOTE_MINT, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        base_program.key(),
        TOKEN_2022_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        quote_program.key(),
        TOKEN_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(&step[4], base_mint, base_program, &step[1])?;
    validate_token_account_for_mint_and_authority(&step[5], quote_mint, quote_program, &step[1])?;
    require!(
        read_token_amount(&step[4])? == base_reserve,
        ArbitrageError::InvalidAccount
    );
    let expected_quote_vault = quote_reserve
        .checked_add(creator_fee)
        .and_then(|amount| amount.checked_add(protocol_fee))
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        read_token_amount(&step[5])? == expected_quote_vault,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn config_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"config"], &TRENDS_PROGRAM_ID).0
}

fn pool_pda(base_mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"pool", base_mint.as_ref(), QUOTE_MINT.as_ref()],
        &TRENDS_PROGRAM_ID,
    )
    .0
}

fn vault_pda(mint: Pubkey, pool: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"token_vault", mint.as_ref(), pool.as_ref()],
        &TRENDS_PROGRAM_ID,
    )
    .0
}

fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &TRENDS_PROGRAM_ID).0
}

fn swap_data(amount_in: u64, min_amount_out: u64) -> [u8; 24] {
    let mut data = [0u8; 24];
    data[..8].copy_from_slice(&SWAP_DISCRIMINATOR);
    data[8..16].copy_from_slice(&amount_in.to_le_bytes());
    data[16..24].copy_from_slice(&min_amount_out.to_le_bytes());
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

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account(
        key: Pubkey,
        owner: Pubkey,
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
            false,
            is_writable,
            lamports,
            data,
            owner,
            executable,
            0,
        )
    }

    fn token_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0u8; 165];
        data[0..32].copy_from_slice(mint.as_ref());
        data[32..64].copy_from_slice(owner.as_ref());
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data
    }

    #[test]
    fn swap_data_matches_confirmed_anchor_wire_format() {
        let data = swap_data(387_165_599, 0);
        assert_eq!(data.len(), 24);
        assert_eq!(&data[..8], &SWAP_DISCRIMINATOR);
        assert_eq!(
            u64::from_le_bytes(data[8..16].try_into().expect("amount")),
            387_165_599
        );
        assert_eq!(
            u64::from_le_bytes(data[16..24].try_into().expect("minimum")),
            0
        );
    }

    #[test]
    fn semantic_validation_binds_pool_vaults_fees_and_both_directions() {
        let base_mint = Pubkey::new_unique();
        let creator = Pubkey::new_unique();
        let pool = pool_pda(base_mint);
        let base_vault = vault_pda(base_mint, pool);
        let quote_vault = vault_pda(QUOTE_MINT, pool);
        let base_reserve = 824_324_170_166_389_u64;
        let quote_reserve = 5_873_323_234_u64;
        let creator_fee = 19_692_937_u64;
        let protocol_fee = 93_245_344_u64;

        let mut pool_data = vec![0u8; POOL_ACCOUNT_LEN];
        pool_data[..8].copy_from_slice(&POOL_DISCRIMINATOR);
        pool_data[8..40].copy_from_slice(creator.as_ref());
        pool_data[40..72].copy_from_slice(base_mint.as_ref());
        pool_data[72..104].copy_from_slice(base_vault.as_ref());
        pool_data[104..136].copy_from_slice(quote_vault.as_ref());
        pool_data[136..144].copy_from_slice(&base_reserve.to_le_bytes());
        pool_data[144..152].copy_from_slice(&quote_reserve.to_le_bytes());
        pool_data[152..160].copy_from_slice(&897_324_170_166_389_u64.to_le_bytes());
        pool_data[160..168].copy_from_slice(&35_873_323_234_u64.to_le_bytes());
        pool_data[168..176].copy_from_slice(&creator_fee.to_le_bytes());
        pool_data[176..184].copy_from_slice(&protocol_fee.to_le_bytes());

        let mut config_data = vec![0u8; CONFIG_ACCOUNT_LEN];
        config_data[..8].copy_from_slice(&CONFIG_DISCRIMINATOR);
        config_data[8..40].copy_from_slice(Pubkey::new_unique().as_ref());
        let step = vec![
            test_account(
                TRENDS_PROGRAM_ID,
                Pubkey::new_unique(),
                false,
                true,
                Vec::new(),
            ),
            test_account(
                POOL_AUTHORITY,
                anchor_lang::system_program::ID,
                false,
                false,
                Vec::new(),
            ),
            test_account(config_pda(), TRENDS_PROGRAM_ID, false, false, config_data),
            test_account(pool, TRENDS_PROGRAM_ID, true, false, pool_data),
            test_account(
                base_vault,
                TOKEN_2022_PROGRAM_ID,
                true,
                false,
                token_data(base_mint, POOL_AUTHORITY, base_reserve),
            ),
            test_account(
                quote_vault,
                TOKEN_PROGRAM_ID,
                true,
                false,
                token_data(
                    QUOTE_MINT,
                    POOL_AUTHORITY,
                    quote_reserve + creator_fee + protocol_fee,
                ),
            ),
            test_account(
                event_authority(),
                anchor_lang::system_program::ID,
                false,
                false,
                Vec::new(),
            ),
        ];
        let base_mint_account =
            test_account(base_mint, TOKEN_2022_PROGRAM_ID, false, false, vec![0; 82]);
        let quote_mint_account =
            test_account(QUOTE_MINT, TOKEN_PROGRAM_ID, false, false, vec![0; 82]);
        let token_program = test_account(
            TOKEN_PROGRAM_ID,
            Pubkey::new_unique(),
            false,
            true,
            Vec::new(),
        );
        let token_2022_program = test_account(
            TOKEN_2022_PROGRAM_ID,
            Pubkey::new_unique(),
            false,
            true,
            Vec::new(),
        );

        validate_trends_semantic_accounts(
            &step,
            0,
            TOTAL_FEE_BPS,
            &base_mint_account,
            &quote_mint_account,
            &token_2022_program,
            &token_program,
        )
        .expect("validate Trends sell");
        validate_trends_semantic_accounts(
            &step,
            1,
            TOTAL_FEE_BPS,
            &quote_mint_account,
            &base_mint_account,
            &token_program,
            &token_2022_program,
        )
        .expect("validate Trends buy");

        step[3].try_borrow_mut_data().expect("pool data")[184] = 1;
        assert!(validate_trends_semantic_accounts(
            &step,
            1,
            TOTAL_FEE_BPS,
            &quote_mint_account,
            &base_mint_account,
            &token_program,
            &token_2022_program,
        )
        .is_err());
    }
}
