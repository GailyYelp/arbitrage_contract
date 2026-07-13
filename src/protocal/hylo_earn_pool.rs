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
        program_ids::{HYLO_EARN_POOL_PROGRAM_ID, HYLO_EXCHANGE_PROGRAM_ID},
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const HYLO_EARN_POOL_STEP_ACCOUNTS: usize = 12;
const HYUSD_MINT: Pubkey = pubkey!("5YMkXAYccHSGnHn9nob9xEvv6Pvka9DZWH7nTbotTu9E");
const SHYUSD_MINT: Pubkey = pubkey!("HnnGv3HrSqjRpgdFmx7vQGjntNEoex1SU4e9Lxcxuihz");
const XSOL_MINT: Pubkey = pubkey!("4sWNB8zGWHkh6UnmwiEtzNxL4XrN7uK9tosbESbJFfVs");
const POOL_CONFIG_DISCRIMINATOR: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];
const HYLO_DISCRIMINATOR: [u8; 8] = [114, 161, 169, 210, 204, 175, 149, 174];
const USER_DEPOSIT_DISCRIMINATOR: [u8; 8] = [186, 198, 140, 233, 129, 39, 98, 153];
const USER_WITHDRAW_DISCRIMINATOR: [u8; 8] = [53, 254, 26, 242, 119, 237, 73, 33];

const POOL_AUTH_BUMP_OFFSET: usize = 40;
const LP_AUTH_BUMP_OFFSET: usize = 41;
const LP_MINT_BUMP_OFFSET: usize = 42;
const WITHDRAWAL_FEE_BITS_OFFSET: usize = 43;
const WITHDRAWAL_FEE_EXP_OFFSET: usize = 51;
const PAUSED_OFFSET: usize = 52;
const HYLO_STABLE_MINT_OFFSET: usize = 104;
const HYLO_LEVER_MINT_OFFSET: usize = 136;

pub struct HyloEarnPoolAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub pool_config: &'info AccountInfo<'info>,
    pub hylo: &'info AccountInfo<'info>,
    pub stablecoin_mint: &'info AccountInfo<'info>,
    pub pool_auth: &'info AccountInfo<'info>,
    pub stablecoin_pool: &'info AccountInfo<'info>,
    pub lp_token_auth: &'info AccountInfo<'info>,
    pub lp_token_mint: &'info AccountInfo<'info>,
    pub fee_auth: &'info AccountInfo<'info>,
    pub fee_vault: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub event_authority: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_stablecoin: &'info AccountInfo<'info>,
    pub user_lp_token: &'info AccountInfo<'info>,
}

pub fn hylo_earn_pool_swap<'info>(
    accounts: HyloEarnPoolAccounts<'info>,
    direction: u8,
    amount_in: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let output = if direction == 0 {
        accounts.user_lp_token
    } else {
        accounts.user_stablecoin
    };
    let pre_out = read_token_amount(output)?;
    let (metas, infos) = if direction == 0 {
        (
            vec![
                AccountMeta::new(accounts.user.key(), true),
                AccountMeta::new_readonly(accounts.pool_config.key(), false),
                AccountMeta::new_readonly(accounts.hylo.key(), false),
                AccountMeta::new_readonly(accounts.stablecoin_mint.key(), false),
                AccountMeta::new(accounts.user_stablecoin.key(), false),
                AccountMeta::new(accounts.user_lp_token.key(), false),
                AccountMeta::new_readonly(accounts.pool_auth.key(), false),
                AccountMeta::new(accounts.stablecoin_pool.key(), false),
                AccountMeta::new_readonly(accounts.lp_token_auth.key(), false),
                AccountMeta::new(accounts.lp_token_mint.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new_readonly(accounts.event_authority.key(), false),
                AccountMeta::new_readonly(accounts.program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.pool_config.clone(),
                accounts.hylo.clone(),
                accounts.stablecoin_mint.clone(),
                accounts.user_stablecoin.clone(),
                accounts.user_lp_token.clone(),
                accounts.pool_auth.clone(),
                accounts.stablecoin_pool.clone(),
                accounts.lp_token_auth.clone(),
                accounts.lp_token_mint.clone(),
                accounts.token_program.clone(),
                accounts.event_authority.clone(),
                accounts.program.clone(),
            ],
        )
    } else {
        (
            vec![
                AccountMeta::new(accounts.user.key(), true),
                AccountMeta::new(accounts.pool_config.key(), false),
                AccountMeta::new_readonly(accounts.hylo.key(), false),
                AccountMeta::new_readonly(accounts.stablecoin_mint.key(), false),
                AccountMeta::new(accounts.user_stablecoin.key(), false),
                AccountMeta::new_readonly(accounts.fee_auth.key(), false),
                AccountMeta::new(accounts.fee_vault.key(), false),
                AccountMeta::new(accounts.user_lp_token.key(), false),
                AccountMeta::new_readonly(accounts.pool_auth.key(), false),
                AccountMeta::new(accounts.stablecoin_pool.key(), false),
                AccountMeta::new(accounts.lp_token_mint.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new_readonly(accounts.event_authority.key(), false),
                AccountMeta::new_readonly(accounts.program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.pool_config.clone(),
                accounts.hylo.clone(),
                accounts.stablecoin_mint.clone(),
                accounts.user_stablecoin.clone(),
                accounts.fee_auth.clone(),
                accounts.fee_vault.clone(),
                accounts.user_lp_token.clone(),
                accounts.pool_auth.clone(),
                accounts.stablecoin_pool.clone(),
                accounts.lp_token_mint.clone(),
                accounts.token_program.clone(),
                accounts.event_authority.clone(),
                accounts.program.clone(),
            ],
        )
    };
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data: hylo_earn_pool_data(direction, amount_in),
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn hylo_earn_pool_data(direction: u8, amount_in: u64) -> Vec<u8> {
    let mut data = if direction == 0 {
        USER_DEPOSIT_DISCRIMINATOR.to_vec()
    } else {
        USER_WITHDRAW_DISCRIMINATOR.to_vec()
    };
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(0); // Option<SlippageConfig>::None; the arbitrage contract enforces min output.
    data
}

pub fn validate_hylo_earn_pool_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step.len() == HYLO_EARN_POOL_STEP_ACCOUNTS && direction <= 1,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        step[0].key(),
        HYLO_EARN_POOL_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[10].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        HYLO_EARN_POOL_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        HYLO_EXCHANGE_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );

    let config = step[1].try_borrow_data()?;
    require!(
        config.len() == 107 && config.starts_with(&POOL_CONFIG_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    let pool_auth_bump = read_byte(&config, POOL_AUTH_BUMP_OFFSET)?;
    let lp_auth_bump = read_byte(&config, LP_AUTH_BUMP_OFFSET)?;
    let lp_mint_bump = read_byte(&config, LP_MINT_BUMP_OFFSET)?;
    let withdrawal_fee = read_u64(&config, WITHDRAWAL_FEE_BITS_OFFSET)?;
    require!(
        read_byte(&config, WITHDRAWAL_FEE_EXP_OFFSET)? == (-4_i8) as u8,
        ArbitrageError::InvalidAccount
    );
    require!(
        read_byte(&config, PAUSED_OFFSET)? == 0 && withdrawal_fee <= 10_000,
        ArbitrageError::InvalidAccount
    );
    require!(
        fee_rate
            == if direction == 1 {
                u16::try_from(withdrawal_fee).map_err(|_| ArbitrageError::InvalidAccount)?
            } else {
                0
            },
        ArbitrageError::InvalidInstructionData
    );
    drop(config);

    let (pool_config, _) =
        Pubkey::find_program_address(&[b"pool_config"], &HYLO_EARN_POOL_PROGRAM_ID);
    let (pool_auth, expected_pool_auth_bump) =
        Pubkey::find_program_address(&[b"pool_auth"], &HYLO_EARN_POOL_PROGRAM_ID);
    let (lp_mint, expected_lp_mint_bump) =
        Pubkey::find_program_address(&[b"staked_hyUSD"], &HYLO_EARN_POOL_PROGRAM_ID);
    let (lp_auth, expected_lp_auth_bump) = Pubkey::find_program_address(
        &[b"mint_auth", lp_mint.as_ref()],
        &HYLO_EARN_POOL_PROGRAM_ID,
    );
    let (hylo, _) = Pubkey::find_program_address(&[b"hylo"], &HYLO_EXCHANGE_PROGRAM_ID);
    let (fee_auth, _) = Pubkey::find_program_address(
        &[b"fee_auth", HYUSD_MINT.as_ref()],
        &HYLO_EXCHANGE_PROGRAM_ID,
    );
    let (event_authority, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &HYLO_EARN_POOL_PROGRAM_ID);
    require_keys_eq!(step[1].key(), pool_config, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[2].key(), hylo, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[3].key(), HYUSD_MINT, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(step[4].key(), pool_auth, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[6].key(), lp_auth, ArbitrageError::InvalidAccount);
    require_keys_eq!(step[7].key(), lp_mint, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(step[7].key(), SHYUSD_MINT, ArbitrageError::InvalidTokenMint);
    require_keys_eq!(step[8].key(), fee_auth, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        step[11].key(),
        event_authority,
        ArbitrageError::InvalidAccount
    );
    require!(
        pool_auth_bump == expected_pool_auth_bump
            && lp_auth_bump == expected_lp_auth_bump
            && lp_mint_bump == expected_lp_mint_bump,
        ArbitrageError::InvalidAccount
    );

    let hylo_data = step[2].try_borrow_data()?;
    require!(
        hylo_data.starts_with(&HYLO_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&hylo_data, HYLO_STABLE_MINT_OFFSET)?,
        HYUSD_MINT,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        read_pubkey(&hylo_data, HYLO_LEVER_MINT_OFFSET)?,
        XSOL_MINT,
        ArbitrageError::InvalidTokenMint
    );
    drop(hylo_data);

    validate_classic_mint(&step[3], None)?;
    validate_classic_mint(&step[7], Some(lp_auth))?;
    validate_token_account_for_mint_and_authority(&step[5], &step[3], &step[10], &step[4])?;
    validate_token_account_for_mint_and_authority(&step[9], &step[3], &step[10], &step[8])?;
    require_keys_eq!(
        step[5].key(),
        associated_token_address(&pool_auth, &HYUSD_MINT),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        step[9].key(),
        associated_token_address(&fee_auth, &HYUSD_MINT),
        ArbitrageError::InvalidAccount
    );

    let (expected_input, expected_output) = if direction == 0 {
        (HYUSD_MINT, SHYUSD_MINT)
    } else {
        (SHYUSD_MINT, HYUSD_MINT)
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

fn associated_token_address(authority: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            authority.as_ref(),
            anchor_spl::token::ID.as_ref(),
            mint.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0
}

fn validate_classic_mint(mint: &AccountInfo<'_>, authority: Option<Pubkey>) -> Result<()> {
    require_keys_eq!(
        *mint.owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidAccount
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == 82 && read_byte(&data, 44)? == 6 && read_byte(&data, 45)? == 1,
        ArbitrageError::InvalidAccount
    );
    if let Some(authority) = authority {
        require!(read_u32(&data, 0)? == 1, ArbitrageError::InvalidAccount);
        require_keys_eq!(
            read_pubkey(&data, 4)?,
            authority,
            ArbitrageError::InvalidAccount
        );
    }
    Ok(())
}

fn read_byte(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or_else(|| ArbitrageError::InvalidAccount.into())
}
fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        data.get(offset..offset + 4)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}
fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}
fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let mut raw = [0_u8; 32];
    raw.copy_from_slice(bytes);
    Ok(Pubkey::new_from_array(raw))
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

    fn mint_data(authority: Option<Pubkey>, supply: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 82];
        if let Some(authority) = authority {
            data[..4].copy_from_slice(&1_u32.to_le_bytes());
            write_pubkey(&mut data, 4, authority);
        }
        data[36..44].copy_from_slice(&supply.to_le_bytes());
        data[44] = 6;
        data[45] = 1;
        data
    }

    fn token_data(mint: Pubkey, authority: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 165];
        write_pubkey(&mut data, 0, mint);
        write_pubkey(&mut data, 32, authority);
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        data[108] = 1;
        data
    }

    fn semantic_fixture() -> Vec<AccountInfo<'static>> {
        let (pool_config, _) =
            Pubkey::find_program_address(&[b"pool_config"], &HYLO_EARN_POOL_PROGRAM_ID);
        let (pool_auth, pool_auth_bump) =
            Pubkey::find_program_address(&[b"pool_auth"], &HYLO_EARN_POOL_PROGRAM_ID);
        let (lp_mint, lp_mint_bump) =
            Pubkey::find_program_address(&[b"staked_hyUSD"], &HYLO_EARN_POOL_PROGRAM_ID);
        let (lp_auth, lp_auth_bump) = Pubkey::find_program_address(
            &[b"mint_auth", lp_mint.as_ref()],
            &HYLO_EARN_POOL_PROGRAM_ID,
        );
        let (hylo, _) = Pubkey::find_program_address(&[b"hylo"], &HYLO_EXCHANGE_PROGRAM_ID);
        let (fee_auth, _) = Pubkey::find_program_address(
            &[b"fee_auth", HYUSD_MINT.as_ref()],
            &HYLO_EXCHANGE_PROGRAM_ID,
        );
        let (event_authority, _) =
            Pubkey::find_program_address(&[b"__event_authority"], &HYLO_EARN_POOL_PROGRAM_ID);
        let stablecoin_pool = associated_token_address(&pool_auth, &HYUSD_MINT);
        let fee_vault = associated_token_address(&fee_auth, &HYUSD_MINT);

        let mut config = vec![0_u8; 107];
        config[..8].copy_from_slice(&POOL_CONFIG_DISCRIMINATOR);
        config[POOL_AUTH_BUMP_OFFSET] = pool_auth_bump;
        config[LP_AUTH_BUMP_OFFSET] = lp_auth_bump;
        config[LP_MINT_BUMP_OFFSET] = lp_mint_bump;
        config[WITHDRAWAL_FEE_BITS_OFFSET..WITHDRAWAL_FEE_BITS_OFFSET + 8]
            .copy_from_slice(&10_u64.to_le_bytes());
        config[WITHDRAWAL_FEE_EXP_OFFSET] = (-4_i8) as u8;

        let mut hylo_data = vec![0_u8; 168];
        hylo_data[..8].copy_from_slice(&HYLO_DISCRIMINATOR);
        write_pubkey(&mut hylo_data, HYLO_STABLE_MINT_OFFSET, HYUSD_MINT);
        write_pubkey(&mut hylo_data, HYLO_LEVER_MINT_OFFSET, XSOL_MINT);

        vec![
            account(
                HYLO_EARN_POOL_PROGRAM_ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(pool_config, HYLO_EARN_POOL_PROGRAM_ID, config, true, false),
            account(hylo, HYLO_EXCHANGE_PROGRAM_ID, hylo_data, false, false),
            account(
                HYUSD_MINT,
                anchor_spl::token::ID,
                mint_data(None, 1_000_000_000_000),
                false,
                false,
            ),
            account(pool_auth, Pubkey::default(), vec![], false, false),
            account(
                stablecoin_pool,
                anchor_spl::token::ID,
                token_data(HYUSD_MINT, pool_auth, 30_595_814_572_329),
                true,
                false,
            ),
            account(lp_auth, Pubkey::default(), vec![], false, false),
            account(
                lp_mint,
                anchor_spl::token::ID,
                mint_data(Some(lp_auth), 26_331_501_021_629),
                true,
                false,
            ),
            account(fee_auth, Pubkey::default(), vec![], false, false),
            account(
                fee_vault,
                anchor_spl::token::ID,
                token_data(HYUSD_MINT, fee_auth, 123_000),
                true,
                false,
            ),
            account(
                anchor_spl::token::ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(event_authority, Pubkey::default(), vec![], false, false),
        ]
    }

    #[test]
    fn swap_data_matches_official_optional_slippage_wire() {
        for (direction, discriminator) in [
            (0, USER_DEPOSIT_DISCRIMINATOR),
            (1, USER_WITHDRAW_DISCRIMINATOR),
        ] {
            let data = hylo_earn_pool_data(direction, 1_000_000);
            assert_eq!(data.len(), 17);
            assert_eq!(&data[..8], &discriminator);
            assert_eq!(&data[8..16], &1_000_000_u64.to_le_bytes());
            assert_eq!(data[16], 0);
        }
    }

    #[test]
    fn semantic_validation_binds_pdas_mints_vaults_fees_and_directions() {
        for direction in [0, 1] {
            let step = semantic_fixture();
            let (input_mint, output_mint, fee_rate) = if direction == 0 {
                (&step[3], &step[7], 0)
            } else {
                (&step[7], &step[3], 10)
            };
            assert!(validate_hylo_earn_pool_semantic_accounts(
                &step,
                direction,
                fee_rate,
                input_mint,
                output_mint,
            )
            .is_ok());
        }

        let step = semantic_fixture();
        step[9].try_borrow_mut_data().unwrap()[0] ^= 1;
        assert!(
            validate_hylo_earn_pool_semantic_accounts(&step, 1, 10, &step[7], &step[3],).is_err()
        );
    }
}
