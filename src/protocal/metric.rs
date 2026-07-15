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
        program_ids::METRIC_PROGRAM_ID,
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const METRIC_STEP_ACCOUNTS: usize = 8;
pub const METRIC_SYSTEM_PROGRAM_ID: Pubkey = anchor_lang::system_program::ID;

const METRIC_MARKET_ACCOUNT_LEN: usize = 1_392;
const METRIC_MARKET_PREFIX: [u8; 2] = [0x01, 0xfe];
const METRIC_MINT_A_OFFSET: usize = 74;
const METRIC_VAULT_A_OFFSET: usize = 106;
const METRIC_MINT_B_OFFSET: usize = 138;
const METRIC_VAULT_B_OFFSET: usize = 170;
const METRIC_PRICE_PROVIDER_OFFSET: usize = 202;
const METRIC_FEED_OFFSET: usize = 234;
const METRIC_FEED_ACCOUNT_LEN: usize = 277;
const METRIC_FEED_PREFIX: [u8; 2] = [0x03, 0xff];
const METRIC_FEED_MINT_A_OFFSET: usize = 39;
const METRIC_FEED_MINT_B_OFFSET: usize = 71;
const METRIC_FEED_ORACLE_OFFSET: usize = 103;
const METRIC_FEED_ORACLE_OWNER_OFFSET: usize = 201;
const METRIC_ORACLE_SLOT_OFFSET: usize = 40;
const METRIC_SWAP_SELECTOR: u8 = 1;
const METRIC_EXACT_IN_MODE: u8 = 1;
const METRIC_A_TO_B: u8 = 1;
const METRIC_B_TO_A: u8 = 0;

pub struct MetricAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub payer: &'info AccountInfo<'info>,
    pub market: &'info AccountInfo<'info>,
    pub mint_a: &'info AccountInfo<'info>,
    pub mint_b: &'info AccountInfo<'info>,
    pub vault_a: &'info AccountInfo<'info>,
    pub vault_b: &'info AccountInfo<'info>,
    pub user_source: &'info AccountInfo<'info>,
    pub user_destination: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub price_provider: &'info AccountInfo<'info>,
    pub feed: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
    pub output_token_account: &'info AccountInfo<'info>,
}

pub fn metric_swap<'info>(
    accounts: MetricAccounts<'info>,
    amount_in: u64,
    direction: u8,
) -> Result<SwapResult> {
    let native_direction = match direction {
        0 => METRIC_A_TO_B,
        1 => METRIC_B_TO_A,
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    let pre_out = read_token_amount(accounts.output_token_account)?;
    let instruction = build_metric_swap_instruction(&accounts, amount_in, native_direction);
    let infos = vec![
        accounts.payer.clone(),
        accounts.market.clone(),
        accounts.mint_a.clone(),
        accounts.mint_b.clone(),
        accounts.vault_a.clone(),
        accounts.vault_b.clone(),
        accounts.user_source.clone(),
        accounts.user_destination.clone(),
        accounts.token_program.clone(),
        accounts.token_program.clone(),
        accounts.system_program.clone(),
        accounts.program.clone(),
        accounts.price_provider.clone(),
        accounts.feed.clone(),
        accounts.oracle.clone(),
        accounts.program.clone(),
    ];
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.output_token_account, pre_out)?,
        fee_amount: 0,
    })
}

fn build_metric_swap_instruction(
    accounts: &MetricAccounts<'_>,
    amount_in: u64,
    native_direction: u8,
) -> Instruction {
    let metas = vec![
        AccountMeta::new_readonly(accounts.payer.key(), true),
        AccountMeta::new(accounts.market.key(), false),
        AccountMeta::new_readonly(accounts.mint_a.key(), false),
        AccountMeta::new_readonly(accounts.mint_b.key(), false),
        AccountMeta::new(accounts.vault_a.key(), false),
        AccountMeta::new(accounts.vault_b.key(), false),
        AccountMeta::new(accounts.user_source.key(), false),
        AccountMeta::new(accounts.user_destination.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
        AccountMeta::new_readonly(accounts.price_provider.key(), false),
        AccountMeta::new_readonly(accounts.feed.key(), false),
        AccountMeta::new_readonly(accounts.oracle.key(), false),
    ];
    let mut data = Vec::with_capacity(27);
    data.push(METRIC_SWAP_SELECTOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(METRIC_EXACT_IN_MODE);
    data.push(native_direction);
    data.extend_from_slice(&0_u64.to_le_bytes());
    data.extend_from_slice(&0_u64.to_le_bytes());
    Instruction {
        program_id: accounts.program.key(),
        accounts: metas,
        data,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn validate_metric_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == METRIC_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        METRIC_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[4].key(),
        METRIC_SYSTEM_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
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

    let market = step[1].try_borrow_data()?;
    require!(
        market.len() == METRIC_MARKET_ACCOUNT_LEN && market.starts_with(&METRIC_MARKET_PREFIX),
        ArbitrageError::InvalidAccount
    );
    let mint_a = read_pubkey(&market, METRIC_MINT_A_OFFSET)?;
    let vault_a = read_pubkey(&market, METRIC_VAULT_A_OFFSET)?;
    let mint_b = read_pubkey(&market, METRIC_MINT_B_OFFSET)?;
    let vault_b = read_pubkey(&market, METRIC_VAULT_B_OFFSET)?;
    let price_provider = read_pubkey(&market, METRIC_PRICE_PROVIDER_OFFSET)?;
    let feed = read_pubkey(&market, METRIC_FEED_OFFSET)?;
    drop(market);
    require!(
        ![mint_a, vault_a, mint_b, vault_b, price_provider, feed].contains(&Pubkey::default())
            && mint_a != mint_b
            && vault_a != vault_b
            && price_provider != feed,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[2].key(), vault_a, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[3].key(), vault_b, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[5].key(),
        price_provider,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(step[6].key(), feed, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[6].owner,
        price_provider,
        ArbitrageError::InvalidAccount
    );

    let feed_data = step[6].try_borrow_data()?;
    require!(
        feed_data.len() == METRIC_FEED_ACCOUNT_LEN && feed_data.starts_with(&METRIC_FEED_PREFIX),
        ArbitrageError::InvalidAccount
    );
    let feed_mint_a = read_pubkey(&feed_data, METRIC_FEED_MINT_A_OFFSET)?;
    let feed_mint_b = read_pubkey(&feed_data, METRIC_FEED_MINT_B_OFFSET)?;
    let oracle = read_pubkey(&feed_data, METRIC_FEED_ORACLE_OFFSET)?;
    let oracle_owner = read_pubkey(&feed_data, METRIC_FEED_ORACLE_OWNER_OFFSET)?;
    drop(feed_data);
    require!(
        feed_mint_a == mint_a
            && feed_mint_b == mint_b
            && oracle != Pubkey::default()
            && oracle_owner != Pubkey::default(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[7].key(), oracle, ArbitrageError::InvalidAccount);
    require_keys_eq!(*step[7].owner, oracle_owner, ArbitrageError::InvalidAccount);
    let oracle_data = step[7].try_borrow_data()?;
    require!(
        read_u64(&oracle_data, METRIC_ORACLE_SLOT_OFFSET)? > 0,
        ArbitrageError::InvalidAccount
    );
    drop(oracle_data);

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
    validate_token_account_for_mint_and_authority(&step[2], mint_a_account, program_a, &step[1])?;
    validate_token_account_for_mint_and_authority(&step[3], mint_b_account, program_b, &step[1])?;
    require!(
        read_token_amount(&step[2])? > 0 && read_token_amount(&step[3])? > 0,
        ArbitrageError::InsufficientLiquidity
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

    #[test]
    fn semantic_validation_binds_market_feed_oracle_and_vaults() {
        let market_key = Pubkey::new_unique();
        let mint_a_key = Pubkey::new_unique();
        let mint_b_key = Pubkey::new_unique();
        let vault_a_key = Pubkey::new_unique();
        let vault_b_key = Pubkey::new_unique();
        let provider_key = Pubkey::new_unique();
        let feed_key = Pubkey::new_unique();
        let oracle_key = Pubkey::new_unique();
        let oracle_owner = Pubkey::new_unique();
        let mut market_data = vec![0_u8; METRIC_MARKET_ACCOUNT_LEN];
        market_data[..2].copy_from_slice(&METRIC_MARKET_PREFIX);
        for (offset, key) in [
            (METRIC_MINT_A_OFFSET, mint_a_key),
            (METRIC_VAULT_A_OFFSET, vault_a_key),
            (METRIC_MINT_B_OFFSET, mint_b_key),
            (METRIC_VAULT_B_OFFSET, vault_b_key),
            (METRIC_PRICE_PROVIDER_OFFSET, provider_key),
            (METRIC_FEED_OFFSET, feed_key),
        ] {
            write_pubkey(&mut market_data, offset, key);
        }
        let mut feed_data = vec![0_u8; METRIC_FEED_ACCOUNT_LEN];
        feed_data[..2].copy_from_slice(&METRIC_FEED_PREFIX);
        for (offset, key) in [
            (METRIC_FEED_MINT_A_OFFSET, mint_a_key),
            (METRIC_FEED_MINT_B_OFFSET, mint_b_key),
            (METRIC_FEED_ORACLE_OFFSET, oracle_key),
            (METRIC_FEED_ORACLE_OWNER_OFFSET, oracle_owner),
        ] {
            write_pubkey(&mut feed_data, offset, key);
        }
        let mut oracle_data = vec![0_u8; METRIC_ORACLE_SLOT_OFFSET + 8];
        oracle_data[METRIC_ORACLE_SLOT_OFFSET..METRIC_ORACLE_SLOT_OFFSET + 8]
            .copy_from_slice(&42_u64.to_le_bytes());
        let step = vec![
            account(METRIC_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(market_key, METRIC_PROGRAM_ID, market_data, true, false),
            account(
                vault_a_key,
                anchor_spl::token::ID,
                token_data(mint_a_key, market_key, 1_000_000),
                true,
                false,
            ),
            account(
                vault_b_key,
                anchor_spl::token::ID,
                token_data(mint_b_key, market_key, 2_000_000),
                true,
                false,
            ),
            account(
                METRIC_SYSTEM_PROGRAM_ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(provider_key, Pubkey::default(), vec![], false, true),
            account(feed_key, provider_key, feed_data, false, false),
            account(oracle_key, oracle_owner, oracle_data, false, false),
        ];
        let mint_a = account(mint_a_key, anchor_spl::token::ID, vec![0; 82], false, false);
        let mint_b = account(mint_b_key, anchor_spl::token::ID, vec![0; 82], false, false);
        let token_program = account(
            anchor_spl::token::ID,
            Pubkey::default(),
            vec![],
            false,
            true,
        );
        assert!(validate_metric_semantic_accounts(
            &step,
            0,
            0,
            &mint_a,
            &mint_b,
            &token_program,
            &token_program,
        )
        .is_ok());

        let mut wrong_oracle = step.clone();
        wrong_oracle[7] = account(
            Pubkey::new_unique(),
            oracle_owner,
            vec![0; METRIC_ORACLE_SLOT_OFFSET + 8],
            false,
            false,
        );
        assert!(validate_metric_semantic_accounts(
            &wrong_oracle,
            0,
            0,
            &mint_a,
            &mint_b,
            &token_program,
            &token_program,
        )
        .is_err());
    }

    #[test]
    fn instruction_layout_is_exact_input_and_uses_canonical_direction() {
        let ai = |key| {
            Box::leak(Box::new(account(
                key,
                Pubkey::default(),
                vec![],
                true,
                false,
            ))) as &'static AccountInfo<'static>
        };
        let program = Box::leak(Box::new(account(
            METRIC_PROGRAM_ID,
            Pubkey::default(),
            vec![],
            false,
            true,
        )));
        let payer = ai(Pubkey::new_unique());
        let market = ai(Pubkey::new_unique());
        let mint_a = ai(Pubkey::new_unique());
        let mint_b = ai(Pubkey::new_unique());
        let vault_a = ai(Pubkey::new_unique());
        let vault_b = ai(Pubkey::new_unique());
        let user_source = ai(Pubkey::new_unique());
        let user_destination = ai(Pubkey::new_unique());
        let token = ai(anchor_spl::token::ID);
        let system = ai(METRIC_SYSTEM_PROGRAM_ID);
        let provider = ai(Pubkey::new_unique());
        let feed = ai(Pubkey::new_unique());
        let oracle = ai(Pubkey::new_unique());
        let accounts = MetricAccounts {
            program,
            payer,
            market,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            user_source,
            user_destination,
            token_program: token,
            system_program: system,
            price_provider: provider,
            feed,
            oracle,
            output_token_account: user_destination,
        };
        let ix = build_metric_swap_instruction(&accounts, 123_456_789, METRIC_A_TO_B);
        assert_eq!(ix.accounts.len(), 15);
        assert_eq!(ix.data.len(), 27);
        assert_eq!(ix.data[0], METRIC_SWAP_SELECTOR);
        assert_eq!(&ix.data[1..9], &123_456_789_u64.to_le_bytes());
        assert_eq!(ix.data[9], METRIC_EXACT_IN_MODE);
        assert_eq!(ix.data[10], METRIC_A_TO_B);
        assert_eq!(&ix.data[11..], &[0; 16]);
        assert_eq!(ix.accounts[11].pubkey, METRIC_PROGRAM_ID);
    }
}
