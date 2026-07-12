use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::types::{read_token_amount, token_balance_delta, SwapResult},
};

pub const SCALE_VMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SCALEWoRSpVZpMRqHEcDfNvBh3nUSe34jDr9r689gLa");
pub const SCALE_VMM_BASE_STEP_ACCOUNTS: usize = 11;
pub const SCALE_VMM_MAX_STEP_ACCOUNTS: usize = 16;

const SCALE_AMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SCALEwAvEK5gtkdHiFzXfPgtk2YwJxPDzaV3aDmR7tA");
const PAIR_ACCOUNT_LEN: usize = 327;
const CONFIG_ACCOUNT_LEN: usize = 115;
const AMM_CONFIG_ACCOUNT_LEN: usize = 107;
const MAX_BENEFICIARIES: usize = 5;
const PAIR_DISCRIMINATOR: [u8; 8] = [229, 212, 222, 222, 191, 128, 176, 235];
const CONFIG_DISCRIMINATOR: [u8; 8] = [160, 78, 128, 0, 248, 83, 230, 160];
const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

pub struct ScaleVmmAccounts<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub pair: &'a AccountInfo<'info>,
    pub mint_a: &'a AccountInfo<'info>,
    pub mint_b: &'a AccountInfo<'info>,
    pub user_ta_a: &'a AccountInfo<'info>,
    pub user_ta_b: &'a AccountInfo<'info>,
    pub vault_a: &'a AccountInfo<'info>,
    pub vault_b: &'a AccountInfo<'info>,
    pub platform_fee_ta_a: &'a AccountInfo<'info>,
    pub token_program_a: &'a AccountInfo<'info>,
    pub token_program_b: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub config: &'a AccountInfo<'info>,
    pub amm_program: &'a AccountInfo<'info>,
    pub amm_pool: &'a AccountInfo<'info>,
    pub amm_vault_a: &'a AccountInfo<'info>,
    pub amm_vault_b: &'a AccountInfo<'info>,
    pub amm_config: &'a AccountInfo<'info>,
    pub user: &'a AccountInfo<'info>,
    pub user_token_out: &'a AccountInfo<'info>,
    pub beneficiaries: &'a [AccountInfo<'info>],
}

pub fn scale_vmm_swap<'info>(
    accounts: ScaleVmmAccounts<'_, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    let pre_out = read_token_amount(accounts.user_token_out)?;
    let mut metas = vec![
        AccountMeta::new(accounts.pair.key(), false),
        AccountMeta::new(accounts.user.key(), true),
        AccountMeta::new_readonly(accounts.mint_a.key(), false),
        AccountMeta::new_readonly(accounts.mint_b.key(), false),
        AccountMeta::new(accounts.user_ta_a.key(), false),
        AccountMeta::new(accounts.user_ta_b.key(), false),
        AccountMeta::new(accounts.vault_a.key(), false),
        AccountMeta::new(accounts.vault_b.key(), false),
        AccountMeta::new(accounts.platform_fee_ta_a.key(), false),
        AccountMeta::new_readonly(accounts.token_program_a.key(), false),
        AccountMeta::new_readonly(accounts.token_program_b.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
        AccountMeta::new_readonly(accounts.config.key(), false),
        AccountMeta::new_readonly(accounts.amm_program.key(), false),
        AccountMeta::new(accounts.amm_pool.key(), false),
        AccountMeta::new(accounts.amm_vault_a.key(), false),
        AccountMeta::new(accounts.amm_vault_b.key(), false),
        AccountMeta::new_readonly(accounts.amm_config.key(), false),
    ];
    let mut infos = vec![
        accounts.pair.clone(),
        accounts.user.clone(),
        accounts.mint_a.clone(),
        accounts.mint_b.clone(),
        accounts.user_ta_a.clone(),
        accounts.user_ta_b.clone(),
        accounts.vault_a.clone(),
        accounts.vault_b.clone(),
        accounts.platform_fee_ta_a.clone(),
        accounts.token_program_a.clone(),
        accounts.token_program_b.clone(),
        accounts.system_program.clone(),
        accounts.config.clone(),
        accounts.amm_program.clone(),
        accounts.amm_pool.clone(),
        accounts.amm_vault_a.clone(),
        accounts.amm_vault_b.clone(),
        accounts.amm_config.clone(),
    ];
    for beneficiary in accounts.beneficiaries {
        metas.push(AccountMeta::new(beneficiary.key(), false));
        infos.push(beneficiary.clone());
    }
    infos.push(accounts.program.clone());
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: swap_data(amount_in, min_amount_out, direction).to_vec(),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_token_out, pre_out)?,
        fee_amount: 0,
    })
}

pub fn validate_scale_vmm_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    in_mint: &AccountInfo<'info>,
    out_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        (SCALE_VMM_BASE_STEP_ACCOUNTS..=SCALE_VMM_MAX_STEP_ACCOUNTS).contains(&step.len()),
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[0].key(),
        SCALE_VMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        SCALE_VMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
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

    let pair_data = step[1].try_borrow_data()?;
    require!(
        pair_data.len() == PAIR_ACCOUNT_LEN && pair_data.starts_with(&PAIR_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require!(
        *pair_data.get(8).ok_or(ArbitrageError::InvalidAccount)? == 1,
        ArbitrageError::InvalidAccount
    );
    require!(
        *pair_data.get(9).ok_or(ArbitrageError::InvalidAccount)? == 0,
        ArbitrageError::InvalidAccount
    );
    let mint_a = read_pubkey(&pair_data, 10)?;
    let mint_b = read_pubkey(&pair_data, 42)?;
    let reserve_a = read_u128(&pair_data, 74)?;
    let reserve_b = read_u128(&pair_data, 90)?;
    let shift = read_u128(&pair_data, 106)?;
    let curve = *pair_data.get(122).ok_or(ArbitrageError::InvalidAccount)?;
    let count = usize::from(*pair_data.get(123).ok_or(ArbitrageError::InvalidAccount)?);
    let graduated_pool = read_pubkey(&pair_data, 294)?;
    let bump = *pair_data.get(326).ok_or(ArbitrageError::InvalidAccount)?;
    require!(
        mint_a != Pubkey::default()
            && mint_b != Pubkey::default()
            && mint_a != mint_b
            && reserve_a <= u128::from(u64::MAX)
            && reserve_b > 0
            && reserve_b <= u128::from(u64::MAX)
            && reserve_a.checked_add(shift).is_some()
            && curve <= 1
            && graduated_pool == Pubkey::default(),
        ArbitrageError::InvalidAccount
    );
    require!(
        count <= MAX_BENEFICIARIES && step.len() == SCALE_VMM_BASE_STEP_ACCOUNTS + count,
        ArbitrageError::InvalidAccountCount
    );
    let expected_pair = Pubkey::find_program_address(
        &[b"state", mint_a.as_ref(), mint_b.as_ref()],
        &SCALE_VMM_PROGRAM_ID,
    );
    require_keys_eq!(
        step[1].key(),
        expected_pair.0,
        ArbitrageError::InvalidAccount
    );
    require!(bump == expected_pair.1, ArbitrageError::InvalidAccount);
    let (expected_in, expected_out) = if direction == 0 {
        (mint_a, mint_b)
    } else {
        (mint_b, mint_a)
    };
    require_keys_eq!(in_mint.key(), expected_in, ArbitrageError::InvalidAccount);
    require_keys_eq!(out_mint.key(), expected_out, ArbitrageError::InvalidAccount);

    let config_pda = Pubkey::find_program_address(&[b"config"], &SCALE_VMM_PROGRAM_ID);
    require_keys_eq!(step[5].key(), config_pda.0, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        *step[5].owner,
        SCALE_VMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let config_data = step[5].try_borrow_data()?;
    require!(
        config_data.len() == CONFIG_ACCOUNT_LEN && config_data.starts_with(&CONFIG_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let platform_wallet = read_pubkey(&config_data, 40)?;
    let base_token = read_pubkey(&config_data, 72)?;
    let platform_bps = read_u16(&config_data, 104)?;
    let threshold = read_u64(&config_data, 106)?;
    let config_bump = *config_data.get(114).ok_or(ArbitrageError::InvalidAccount)?;
    require!(
        platform_wallet != Pubkey::default()
            && base_token == mint_a
            && platform_bps < 10_000
            && threshold > 0
            && reserve_a < u128::from(threshold)
            && config_bump == config_pda.1,
        ArbitrageError::InvalidAccount
    );

    let expected_vault_a = Pubkey::find_program_address(
        &[step[1].key().as_ref(), mint_a.as_ref()],
        &SCALE_VMM_PROGRAM_ID,
    )
    .0;
    let expected_vault_b = Pubkey::find_program_address(
        &[step[1].key().as_ref(), mint_b.as_ref()],
        &SCALE_VMM_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(
        step[2].key(),
        expected_vault_a,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[3].key(),
        expected_vault_b,
        ArbitrageError::InvalidAccount
    );
    validate_token_account(
        &step[2],
        mint_a,
        step[1].key(),
        Some(reserve_a as u64),
        token_program,
    )?;
    validate_token_account(
        &step[3],
        mint_b,
        step[1].key(),
        Some(reserve_b as u64),
        token_program,
    )?;
    let platform_ata = associated_token_address(platform_wallet, mint_a, token_program.key());
    require_keys_eq!(step[4].key(), platform_ata, ArbitrageError::InvalidAccount);
    validate_token_account(&step[4], mint_a, platform_wallet, None, token_program)?;

    require_keys_eq!(
        step[6].key(),
        SCALE_AMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let amm_pool = Pubkey::find_program_address(
        &[
            b"pool",
            step[1].key().as_ref(),
            mint_a.as_ref(),
            mint_b.as_ref(),
        ],
        &SCALE_AMM_PROGRAM_ID,
    )
    .0;
    require_keys_eq!(step[7].key(), amm_pool, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[8].key(),
        Pubkey::find_program_address(&[amm_pool.as_ref(), mint_a.as_ref()], &SCALE_AMM_PROGRAM_ID)
            .0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[9].key(),
        Pubkey::find_program_address(&[amm_pool.as_ref(), mint_b.as_ref()], &SCALE_AMM_PROGRAM_ID)
            .0,
        ArbitrageError::InvalidAccount
    );
    let amm_config_pda = Pubkey::find_program_address(&[b"config"], &SCALE_AMM_PROGRAM_ID);
    require_keys_eq!(
        step[10].key(),
        amm_config_pda.0,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[10].owner,
        SCALE_AMM_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let amm_config = step[10].try_borrow_data()?;
    require!(
        amm_config.len() == AMM_CONFIG_ACCOUNT_LEN
            && amm_config.starts_with(&CONFIG_DISCRIMINATOR)
            && read_pubkey(&amm_config, 72)? == mint_a
            && *amm_config.get(106).ok_or(ArbitrageError::InvalidAccount)? == amm_config_pda.1,
        ArbitrageError::InvalidAccount
    );

    let mut total_bps = u32::from(platform_bps);
    let mut wallets = [Pubkey::default(); MAX_BENEFICIARIES];
    for index in 0..count {
        let offset = 124usize
            .checked_add(index.checked_mul(34).ok_or(ArbitrageError::MathOverflow)?)
            .ok_or(ArbitrageError::MathOverflow)?;
        let wallet = read_pubkey(&pair_data, offset)?;
        let share = read_u16(&pair_data, offset + 32)?;
        require!(
            wallet != Pubkey::default() && share < 10_000,
            ArbitrageError::InvalidAccount
        );
        for previous in &wallets[..index] {
            require!(*previous != wallet, ArbitrageError::InvalidAccount);
        }
        wallets[index] = wallet;
        total_bps = total_bps
            .checked_add(u32::from(share))
            .ok_or(ArbitrageError::MathOverflow)?;
        let ata = associated_token_address(wallet, mint_a, token_program.key());
        require_keys_eq!(
            step[SCALE_VMM_BASE_STEP_ACCOUNTS + index].key(),
            ata,
            ArbitrageError::InvalidAccount
        );
        validate_token_account(
            &step[SCALE_VMM_BASE_STEP_ACCOUNTS + index],
            mint_a,
            wallet,
            None,
            token_program,
        )?;
    }
    require!(
        total_bps < 10_000 && total_bps == u32::from(fee_rate),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn associated_token_address(wallet: Pubkey, mint: Pubkey, token_program: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &anchor_spl::associated_token::ID,
    )
    .0
}

fn validate_token_account<'info>(
    account: &AccountInfo<'info>,
    mint: Pubkey,
    authority: Pubkey,
    amount: Option<u64>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(data.len() >= 72, ArbitrageError::InvalidAccount);
    require!(
        read_pubkey(&data, 0)? == mint && read_pubkey(&data, 32)? == authority,
        ArbitrageError::InvalidAccount
    );
    if let Some(expected) = amount {
        require!(
            read_u64(&data, 64)? == expected,
            ArbitrageError::InvalidAccount
        );
    }
    Ok(())
}

fn swap_data(amount: u64, limit: u64, direction: u8) -> [u8; 24] {
    let mut data = [0u8; 24];
    data[..8].copy_from_slice(if direction == 0 {
        &BUY_DISCRIMINATOR
    } else {
        &SELL_DISCRIMINATOR
    });
    data[8..16].copy_from_slice(&amount.to_le_bytes());
    data[16..24].copy_from_slice(&limit.to_le_bytes());
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
fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u16::from_le_bytes(bytes))
}
fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}
fn read_u128(data: &[u8], offset: usize) -> Result<u128> {
    let bytes: [u8; 16] = data
        .get(offset..offset + 16)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u128::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_data_matches_onchain_anchor_idl() {
        let buy = swap_data(500_000_000, 1, 0);
        let sell = swap_data(95_649_928, 1, 1);
        assert_eq!(&buy[..8], &BUY_DISCRIMINATOR);
        assert_eq!(&sell[..8], &SELL_DISCRIMINATOR);
        assert_eq!(&buy[8..16], &500_000_000_u64.to_le_bytes());
    }
}
