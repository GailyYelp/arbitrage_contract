use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::HADRON_PROGRAM_ID,
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const HADRON_BASE_STEP_ACCOUNTS: usize = 12;
pub const HADRON_SPREAD_STEP_ACCOUNTS: usize = 14;
const CONFIG_LEN: usize = 724;
const ORACLE_LEN: usize = 128;
const CURVE_META_LEN: usize = 56;
const CURVE_UPDATES_LEN: usize = 998;
const FEE_CONFIG_LEN: usize = 72;
const SPREAD_CONFIG_HEADER_LEN: usize = 72;
const CONFIG_SEED: &[u8] = b"hadron-config";
const ORACLE_SEED: &[u8] = b"hadron-midprice";
const CURVE_META_SEED: &[u8] = b"hadron-curve-meta";
const CURVE_UPDATES_SEED: &[u8] = b"hadron-curve-updates";
const FEE_CONFIG_SEED: &[u8] = b"fee_config";
const SPREAD_CONFIG_SEED: &[u8] = b"spread_config";

pub struct HadronAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub config: &'info AccountInfo<'info>,
    pub oracle: &'info AccountInfo<'info>,
    pub curve_meta: &'info AccountInfo<'info>,
    pub curve_prefabs: &'info AccountInfo<'info>,
    pub vault_x: &'info AccountInfo<'info>,
    pub vault_y: &'info AccountInfo<'info>,
    pub fee_config: &'info AccountInfo<'info>,
    pub fee_recipient_x: &'info AccountInfo<'info>,
    pub fee_recipient_y: &'info AccountInfo<'info>,
    pub curve_updates: &'info AccountInfo<'info>,
    pub clock: &'info AccountInfo<'info>,
    pub spread_config: Option<&'info AccountInfo<'info>>,
    pub instructions_sysvar: Option<&'info AccountInfo<'info>>,
    pub user: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub mint_x: &'info AccountInfo<'info>,
    pub mint_y: &'info AccountInfo<'info>,
    pub token_program_x: &'info AccountInfo<'info>,
    pub token_program_y: &'info AccountInfo<'info>,
}

pub fn hadron_swap<'info>(
    accounts: HadronAccounts<'info>,
    direction: u8,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let is_x = direction == 0;
    let (vault_source, vault_destination, fee_recipient) = if is_x {
        (accounts.vault_x, accounts.vault_y, accounts.fee_recipient_x)
    } else {
        (accounts.vault_y, accounts.vault_x, accounts.fee_recipient_y)
    };
    let pre_out = read_token_amount(accounts.user_output)?;
    let data = hadron_swap_data(direction, amount_in, min_amount_out)?;
    let mut metas = vec![
        AccountMeta::new_readonly(accounts.token_program_x.key(), false),
        AccountMeta::new_readonly(accounts.token_program_y.key(), false),
        AccountMeta::new(accounts.config.key(), false),
        AccountMeta::new_readonly(accounts.oracle.key(), false),
        AccountMeta::new_readonly(accounts.curve_meta.key(), false),
        AccountMeta::new(accounts.curve_prefabs.key(), false),
        AccountMeta::new_readonly(accounts.config.key(), false),
        AccountMeta::new_readonly(accounts.user.key(), true),
        AccountMeta::new(accounts.user_input.key(), false),
        AccountMeta::new(vault_source.key(), false),
        AccountMeta::new(vault_destination.key(), false),
        AccountMeta::new(accounts.user_output.key(), false),
        AccountMeta::new_readonly(accounts.fee_config.key(), false),
        AccountMeta::new(fee_recipient.key(), false),
        AccountMeta::new_readonly(accounts.clock.key(), false),
        AccountMeta::new(accounts.curve_updates.key(), false),
    ];
    let mut infos = vec![
        accounts.token_program_x.clone(),
        accounts.token_program_y.clone(),
        accounts.config.clone(),
        accounts.oracle.clone(),
        accounts.curve_meta.clone(),
        accounts.curve_prefabs.clone(),
        accounts.config.clone(),
        accounts.user.clone(),
        accounts.user_input.clone(),
        vault_source.clone(),
        vault_destination.clone(),
        accounts.user_output.clone(),
        accounts.fee_config.clone(),
        fee_recipient.clone(),
        accounts.clock.clone(),
        accounts.curve_updates.clone(),
    ];
    if let (Some(spread), Some(instructions)) =
        (accounts.spread_config, accounts.instructions_sysvar)
    {
        metas.push(AccountMeta::new_readonly(spread.key(), false));
        metas.push(AccountMeta::new_readonly(instructions.key(), false));
        infos.push(spread.clone());
        infos.push(instructions.clone());
    }
    infos.push(accounts.program.clone());
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

fn hadron_swap_data(direction: u8, amount_in: u64, min_amount_out: u64) -> Result<Vec<u8>> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let mut data = Vec::with_capacity(26);
    data.push(3);
    data.push(u8::from(direction == 0));
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data.extend_from_slice(&i64::MAX.to_le_bytes());
    Ok(data)
}

pub fn validate_hadron_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
) -> Result<()> {
    validate_hadron_semantic_accounts_at_slot(
        step,
        direction,
        fee_rate,
        input_mint,
        output_mint,
        input_token_program,
        output_token_program,
        Clock::get()?.slot,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_hadron_semantic_accounts_at_slot<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
    current_slot: u64,
) -> Result<()> {
    require!(
        matches!(
            step.len(),
            HADRON_BASE_STEP_ACCOUNTS | HADRON_SPREAD_STEP_ACCOUNTS
        ),
        ArbitrageError::InvalidAccountCount
    );
    require!(
        direction <= 1 && fee_rate == 0,
        ArbitrageError::InvalidInstructionData
    );
    require_keys_eq!(
        step[0].key(),
        HADRON_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        HADRON_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let config = step[1].try_borrow_data()?;
    require!(
        config.len() == CONFIG_LEN && config[0] == 1,
        ArbitrageError::InvalidAccount
    );
    let seed = read_u64(&config, 1)?.to_le_bytes();
    let mint_x = read_pubkey(&config, 41)?;
    let mint_y = read_pubkey(&config, 73)?;
    let bump = config[105];
    let spread_initialized = config[138];
    let delta_staleness = u64::from(config[139]);
    require!(spread_initialized <= 1, ArbitrageError::InvalidAccount);
    require!(
        (spread_initialized == 1) == (step.len() == HADRON_SPREAD_STEP_ACCOUNTS),
        ArbitrageError::InvalidAccountCount
    );
    let token_x = read_pubkey(&config, 184)?;
    let token_y = read_pubkey(&config, 216)?;
    require_keys_eq!(
        read_pubkey(&config, 106)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&config, 430)?,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require!(read_u64(&config, 422)? > 0, ArbitrageError::InvalidAccount);
    let expected_config = Pubkey::find_program_address(
        &[CONFIG_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
        &HADRON_PROGRAM_ID,
    );
    require_keys_eq!(
        expected_config.0,
        step[1].key(),
        ArbitrageError::InvalidAccount
    );
    require!(expected_config.1 == bump, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        Pubkey::find_program_address(
            &[ORACLE_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
            &HADRON_PROGRAM_ID
        )
        .0,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        Pubkey::find_program_address(
            &[CURVE_META_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
            &HADRON_PROGRAM_ID
        )
        .0,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        Pubkey::find_program_address(
            &[CURVE_UPDATES_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
            &HADRON_PROGRAM_ID
        )
        .0,
        step[10].key(),
        ArbitrageError::InvalidAccount
    );
    let has_pool_fee = config[141];
    require!(has_pool_fee <= 1, ArbitrageError::InvalidAccount);
    let expected_fee = if has_pool_fee == 1 {
        Pubkey::find_program_address(
            &[FEE_CONFIG_SEED, step[1].key().as_ref()],
            &HADRON_PROGRAM_ID,
        )
        .0
    } else {
        Pubkey::find_program_address(&[FEE_CONFIG_SEED], &HADRON_PROGRAM_ID).0
    };
    require_keys_eq!(expected_fee, step[7].key(), ArbitrageError::InvalidAccount);
    drop(config);

    for account in [&step[2], &step[3], &step[4], &step[7], &step[10]] {
        require_keys_eq!(
            *account.owner,
            HADRON_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
    }
    require!(
        step[2].data_len() == ORACLE_LEN
            && step[3].data_len() == CURVE_META_LEN
            && step[10].data_len() == CURVE_UPDATES_LEN,
        ArbitrageError::InvalidAccount
    );
    let oracle = step[2].try_borrow_data()?;
    let oracle_slot = read_u64(&oracle, 56)?;
    require!(
        read_u64(&oracle, 40)? > 0
            && read_u64(&oracle, 48)? > 0
            && current_slot >= oracle_slot
            && current_slot - oracle_slot <= delta_staleness,
        ArbitrageError::InvalidAccount
    );
    drop(oracle);
    let fee = step[7].try_borrow_data()?;
    require!(
        fee.len() >= FEE_CONFIG_LEN && fee[0] == 1,
        ArbitrageError::InvalidAccount
    );
    let fee_recipient = read_pubkey(&fee, 40)?;
    drop(fee);
    let (token_x_ai, token_y_ai) = if direction == 0 {
        (input_token_program, output_token_program)
    } else {
        (output_token_program, input_token_program)
    };
    validate_token_binding(&step[5], mint_x, token_x_ai, step[1].key())?;
    validate_token_binding(&step[6], mint_y, token_y_ai, step[1].key())?;
    validate_token_binding(&step[8], mint_x, token_x_ai, fee_recipient)?;
    validate_token_binding(&step[9], mint_y, token_y_ai, fee_recipient)?;
    require_keys_eq!(
        step[5].key(),
        associated_token_address(step[1].key(), mint_x, token_x_ai.key()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[6].key(),
        associated_token_address(step[1].key(), mint_y, token_y_ai.key()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[8].key(),
        associated_token_address(fee_recipient, mint_x, token_x_ai.key()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[9].key(),
        associated_token_address(fee_recipient, mint_y, token_y_ai.key()),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[11].key(),
        anchor_lang::solana_program::sysvar::clock::ID,
        ArbitrageError::InvalidAccount
    );
    if spread_initialized == 1 {
        require_keys_eq!(
            Pubkey::find_program_address(
                &[SPREAD_CONFIG_SEED, step[1].key().as_ref()],
                &HADRON_PROGRAM_ID
            )
            .0,
            step[12].key(),
            ArbitrageError::InvalidAccount
        );
        require_keys_eq!(
            *step[12].owner,
            HADRON_PROGRAM_ID,
            ArbitrageError::InvalidAccount
        );
        let spread = step[12].try_borrow_data()?;
        require!(
            spread.len() >= SPREAD_CONFIG_HEADER_LEN
                && spread[0] == 1
                && read_pubkey(&spread, 40)? == step[1].key(),
            ArbitrageError::InvalidAccount
        );
        drop(spread);
        require_keys_eq!(
            step[13].key(),
            anchor_lang::solana_program::sysvar::instructions::ID,
            ArbitrageError::InvalidAccount
        );
    }
    let (expected_input, expected_output) = if direction == 0 {
        (mint_x, mint_y)
    } else {
        (mint_y, mint_x)
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
    require_keys_eq!(
        input_token_program.key(),
        if direction == 0 { token_x } else { token_y },
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_token_program.key(),
        if direction == 0 { token_y } else { token_x },
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
        data.get(offset..offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn validate_token_binding<'info>(
    account: &AccountInfo<'info>,
    mint: Pubkey,
    token_program: &AccountInfo<'info>,
    authority: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() >= 109 && data[108] != 0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, 0)?,
        mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&data, 32)?,
        authority,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn associated_token_address(owner: Pubkey, mint: Pubkey, token_program: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &anchor_spl::associated_token::ID,
    )
    .0
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

    fn token_data(mint: Pubkey, authority: Pubkey) -> Vec<u8> {
        let mut data = vec![0_u8; 165];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[108] = 1;
        data
    }

    fn semantic_fixture() -> (
        Vec<AccountInfo<'static>>,
        AccountInfo<'static>,
        AccountInfo<'static>,
        AccountInfo<'static>,
    ) {
        let seed = 42_u64.to_le_bytes();
        let mint_x = Pubkey::new_unique();
        let mint_y = Pubkey::new_unique();
        let token_program = anchor_spl::token::ID;
        let (config, bump) = Pubkey::find_program_address(
            &[CONFIG_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
            &HADRON_PROGRAM_ID,
        );
        let oracle = Pubkey::find_program_address(
            &[ORACLE_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
            &HADRON_PROGRAM_ID,
        )
        .0;
        let curve_meta = Pubkey::find_program_address(
            &[CURVE_META_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
            &HADRON_PROGRAM_ID,
        )
        .0;
        let curve_updates = Pubkey::find_program_address(
            &[CURVE_UPDATES_SEED, &seed, mint_x.as_ref(), mint_y.as_ref()],
            &HADRON_PROGRAM_ID,
        )
        .0;
        let curve_prefabs = Pubkey::new_unique();
        let fee_config = Pubkey::find_program_address(&[FEE_CONFIG_SEED], &HADRON_PROGRAM_ID).0;
        let fee_recipient = Pubkey::new_unique();
        let vault_x = associated_token_address(config, mint_x, token_program);
        let vault_y = associated_token_address(config, mint_y, token_program);
        let fee_x = associated_token_address(fee_recipient, mint_x, token_program);
        let fee_y = associated_token_address(fee_recipient, mint_y, token_program);

        let mut config_data = vec![0_u8; CONFIG_LEN];
        config_data[0] = 1;
        config_data[1..9].copy_from_slice(&seed);
        write_pubkey(&mut config_data, 41, mint_x);
        write_pubkey(&mut config_data, 73, mint_y);
        config_data[105] = bump;
        write_pubkey(&mut config_data, 106, curve_meta);
        config_data[138] = 0;
        config_data[139] = 7;
        config_data[141] = 0;
        write_pubkey(&mut config_data, 184, token_program);
        write_pubkey(&mut config_data, 216, token_program);
        config_data[422..430].copy_from_slice(&(1_u64 << 32).to_le_bytes());
        write_pubkey(&mut config_data, 430, curve_prefabs);

        let mut oracle_data = vec![0_u8; ORACLE_LEN];
        oracle_data[40..48].copy_from_slice(&(1_u64 << 32).to_le_bytes());
        oracle_data[48..56].copy_from_slice(&(1_u64 << 32).to_le_bytes());
        oracle_data[56..64].copy_from_slice(&100_u64.to_le_bytes());
        let mut fee_data = vec![0_u8; FEE_CONFIG_LEN];
        fee_data[0] = 1;
        write_pubkey(&mut fee_data, 40, fee_recipient);

        let step = vec![
            account(HADRON_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(config, HADRON_PROGRAM_ID, config_data, true, false),
            account(oracle, HADRON_PROGRAM_ID, oracle_data, false, false),
            account(
                curve_meta,
                HADRON_PROGRAM_ID,
                vec![0; CURVE_META_LEN],
                false,
                false,
            ),
            account(curve_prefabs, HADRON_PROGRAM_ID, vec![], true, false),
            account(
                vault_x,
                token_program,
                token_data(mint_x, config),
                true,
                false,
            ),
            account(
                vault_y,
                token_program,
                token_data(mint_y, config),
                true,
                false,
            ),
            account(fee_config, HADRON_PROGRAM_ID, fee_data, false, false),
            account(
                fee_x,
                token_program,
                token_data(mint_x, fee_recipient),
                true,
                false,
            ),
            account(
                fee_y,
                token_program,
                token_data(mint_y, fee_recipient),
                true,
                false,
            ),
            account(
                curve_updates,
                HADRON_PROGRAM_ID,
                vec![0; CURVE_UPDATES_LEN],
                true,
                false,
            ),
            account(
                anchor_lang::solana_program::sysvar::clock::ID,
                anchor_lang::solana_program::sysvar::ID,
                vec![],
                false,
                false,
            ),
        ];
        (
            step,
            account(mint_x, token_program, vec![], false, false),
            account(mint_y, token_program, vec![], false, false),
            account(token_program, Pubkey::default(), vec![], false, true),
        )
    }

    #[test]
    fn swap_data_matches_litesvm_verified_layout() {
        let data = hadron_swap_data(0, 100_000_000, 7_700_000).expect("Hadron data");
        assert_eq!(data.len(), 26);
        assert_eq!(&data[..2], &[3, 1]);
        assert_eq!(&data[2..10], &100_000_000_u64.to_le_bytes());
        assert_eq!(&data[10..18], &7_700_000_u64.to_le_bytes());
        assert_eq!(&data[18..], &i64::MAX.to_le_bytes());
        assert_eq!(hadron_swap_data(1, 1, 0).unwrap()[1], 0);
        assert!(hadron_swap_data(2, 1, 0).is_err());
    }

    #[test]
    fn semantic_validation_binds_both_directions_and_rejects_staleness() {
        let (step, mint_x, mint_y, token_program) = semantic_fixture();
        validate_hadron_semantic_accounts_at_slot(
            &step,
            0,
            0,
            &mint_x,
            &mint_y,
            &token_program,
            &token_program,
            107,
        )
        .expect("forward semantics");
        validate_hadron_semantic_accounts_at_slot(
            &step,
            1,
            0,
            &mint_y,
            &mint_x,
            &token_program,
            &token_program,
            107,
        )
        .expect("reverse semantics");
        assert!(validate_hadron_semantic_accounts_at_slot(
            &step,
            0,
            0,
            &mint_x,
            &mint_y,
            &token_program,
            &token_program,
            108,
        )
        .is_err());
    }
}
