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
        program_ids::{
            jupiter_lend_earn_program_id, jupiter_lend_liquidity_program_id,
            jupiter_lend_rewards_program_id,
        },
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const JUPITER_LEND_EARN_STEP_ACCOUNTS: usize = 16;
const LENDING_ADMIN_DISCRIMINATOR: [u8; 8] = [42, 8, 33, 220, 163, 40, 210, 5];
const LENDING_DISCRIMINATOR: [u8; 8] = [135, 199, 82, 16, 249, 131, 182, 241];
const RESERVE_DISCRIMINATOR: [u8; 8] = [21, 18, 59, 135, 120, 20, 31, 12];
const SUPPLY_POSITION_DISCRIMINATOR: [u8; 8] = [202, 219, 136, 118, 61, 177, 21, 146];
const REWARDS_DISCRIMINATOR: [u8; 8] = [166, 72, 71, 131, 172, 74, 166, 181];
const DEPOSIT_DISCRIMINATOR: [u8; 8] = [116, 144, 16, 97, 118, 109, 40, 119];
const REDEEM_DISCRIMINATOR: [u8; 8] = [235, 189, 237, 56, 166, 180, 184, 149];

pub struct JupiterLendEarnAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub lending_admin: &'info AccountInfo<'info>,
    pub lending: &'info AccountInfo<'info>,
    pub asset_mint: &'info AccountInfo<'info>,
    pub f_token_mint: &'info AccountInfo<'info>,
    pub reserve: &'info AccountInfo<'info>,
    pub supply_position: &'info AccountInfo<'info>,
    pub rate_model: &'info AccountInfo<'info>,
    pub vault: &'info AccountInfo<'info>,
    pub claim_account: &'info AccountInfo<'info>,
    pub liquidity: &'info AccountInfo<'info>,
    pub liquidity_program: &'info AccountInfo<'info>,
    pub rewards_rate_model: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub associated_token_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_asset: &'info AccountInfo<'info>,
    pub user_f_token: &'info AccountInfo<'info>,
}

pub fn jupiter_lend_earn_swap<'info>(
    accounts: JupiterLendEarnAccounts<'info>,
    direction: u8,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<SwapResult> {
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let output = if direction == 0 {
        accounts.user_f_token
    } else {
        accounts.user_asset
    };
    let pre_out = read_token_amount(output)?;
    let data = jupiter_lend_earn_data(direction, amount_in, min_amount_out);
    let (metas, infos) = if direction == 0 {
        (
            vec![
                AccountMeta::new(accounts.user.key(), true),
                AccountMeta::new(accounts.user_asset.key(), false),
                AccountMeta::new(accounts.user_f_token.key(), false),
                AccountMeta::new_readonly(accounts.asset_mint.key(), false),
                AccountMeta::new_readonly(accounts.lending_admin.key(), false),
                AccountMeta::new(accounts.lending.key(), false),
                AccountMeta::new(accounts.f_token_mint.key(), false),
                AccountMeta::new(accounts.reserve.key(), false),
                AccountMeta::new(accounts.supply_position.key(), false),
                AccountMeta::new_readonly(accounts.rate_model.key(), false),
                AccountMeta::new(accounts.vault.key(), false),
                AccountMeta::new(accounts.liquidity.key(), false),
                AccountMeta::new_readonly(accounts.liquidity_program.key(), false),
                AccountMeta::new_readonly(accounts.rewards_rate_model.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
                AccountMeta::new_readonly(accounts.system_program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.user_asset.clone(),
                accounts.user_f_token.clone(),
                accounts.asset_mint.clone(),
                accounts.lending_admin.clone(),
                accounts.lending.clone(),
                accounts.f_token_mint.clone(),
                accounts.reserve.clone(),
                accounts.supply_position.clone(),
                accounts.rate_model.clone(),
                accounts.vault.clone(),
                accounts.liquidity.clone(),
                accounts.liquidity_program.clone(),
                accounts.rewards_rate_model.clone(),
                accounts.token_program.clone(),
                accounts.associated_token_program.clone(),
                accounts.system_program.clone(),
                accounts.program.clone(),
            ],
        )
    } else {
        (
            vec![
                AccountMeta::new(accounts.user.key(), true),
                AccountMeta::new(accounts.user_f_token.key(), false),
                AccountMeta::new(accounts.user_asset.key(), false),
                AccountMeta::new_readonly(accounts.lending_admin.key(), false),
                AccountMeta::new(accounts.lending.key(), false),
                AccountMeta::new_readonly(accounts.asset_mint.key(), false),
                AccountMeta::new(accounts.f_token_mint.key(), false),
                AccountMeta::new(accounts.reserve.key(), false),
                AccountMeta::new(accounts.supply_position.key(), false),
                AccountMeta::new_readonly(accounts.rate_model.key(), false),
                AccountMeta::new(accounts.vault.key(), false),
                AccountMeta::new(accounts.claim_account.key(), false),
                AccountMeta::new(accounts.liquidity.key(), false),
                AccountMeta::new_readonly(accounts.liquidity_program.key(), false),
                AccountMeta::new_readonly(accounts.rewards_rate_model.key(), false),
                AccountMeta::new_readonly(accounts.token_program.key(), false),
                AccountMeta::new_readonly(accounts.associated_token_program.key(), false),
                AccountMeta::new_readonly(accounts.system_program.key(), false),
            ],
            vec![
                accounts.user.clone(),
                accounts.user_f_token.clone(),
                accounts.user_asset.clone(),
                accounts.lending_admin.clone(),
                accounts.lending.clone(),
                accounts.asset_mint.clone(),
                accounts.f_token_mint.clone(),
                accounts.reserve.clone(),
                accounts.supply_position.clone(),
                accounts.rate_model.clone(),
                accounts.vault.clone(),
                accounts.claim_account.clone(),
                accounts.liquidity.clone(),
                accounts.liquidity_program.clone(),
                accounts.rewards_rate_model.clone(),
                accounts.token_program.clone(),
                accounts.associated_token_program.clone(),
                accounts.system_program.clone(),
                accounts.program.clone(),
            ],
        )
    };
    invoke(
        &Instruction {
            program_id: accounts.program.key(),
            accounts: metas,
            data,
        },
        &infos,
    )?;
    Ok(SwapResult {
        amount_out: token_balance_delta(output, pre_out)?,
        fee_amount: 0,
    })
}

fn jupiter_lend_earn_data(direction: u8, amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = if direction == 0 {
        DEPOSIT_DISCRIMINATOR.to_vec()
    } else {
        REDEEM_DISCRIMINATOR.to_vec()
    };
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

pub fn validate_jupiter_lend_earn_semantic_accounts(
    step: &[AccountInfo<'_>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'_>,
    output_mint: &AccountInfo<'_>,
) -> Result<()> {
    require!(
        step.len() == JUPITER_LEND_EARN_STEP_ACCOUNTS && direction <= 1,
        ArbitrageError::InvalidAccountCount
    );
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        jupiter_lend_earn_program_id(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[11].key(),
        jupiter_lend_liquidity_program_id(),
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[13].key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[14].key(),
        anchor_spl::associated_token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[15].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        *step[1].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[2].owner,
        step[0].key(),
        ArbitrageError::InvalidAccount
    );
    for account in [&step[5], &step[6], &step[7], &step[10]] {
        require_keys_eq!(
            *account.owner,
            step[11].key(),
            ArbitrageError::InvalidAccount
        );
    }
    require_keys_eq!(
        *step[12].owner,
        jupiter_lend_rewards_program_id(),
        ArbitrageError::InvalidAccount
    );
    let (expected_input, expected_output) = if direction == 0 {
        (step[3].key(), step[4].key())
    } else {
        (step[4].key(), step[3].key())
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

    let (expected_admin, _) = Pubkey::find_program_address(&[b"lending_admin"], &step[0].key());
    let (expected_f_token, _) =
        Pubkey::find_program_address(&[b"f_token_mint", step[3].key().as_ref()], &step[0].key());
    let (expected_lending, _) = Pubkey::find_program_address(
        &[b"lending", step[3].key().as_ref(), step[4].key().as_ref()],
        &step[0].key(),
    );
    let (expected_reserve, _) =
        Pubkey::find_program_address(&[b"reserve", step[3].key().as_ref()], &step[11].key());
    let (expected_supply, _) = Pubkey::find_program_address(
        &[
            b"user_supply_position",
            step[3].key().as_ref(),
            step[2].key().as_ref(),
        ],
        &step[11].key(),
    );
    let (expected_rate, _) =
        Pubkey::find_program_address(&[b"rate_model", step[3].key().as_ref()], &step[11].key());
    let (expected_liquidity, _) = Pubkey::find_program_address(&[b"liquidity"], &step[11].key());
    let (expected_claim, _) = Pubkey::find_program_address(
        &[
            b"user_claim",
            step[1].key().as_ref(),
            step[3].key().as_ref(),
        ],
        &step[11].key(),
    );
    let (expected_rewards, _) = Pubkey::find_program_address(
        &[b"lending_rewards_rate_model", step[3].key().as_ref()],
        &jupiter_lend_rewards_program_id(),
    );
    let (expected_vault, _) = Pubkey::find_program_address(
        &[
            step[10].key().as_ref(),
            step[13].key().as_ref(),
            step[3].key().as_ref(),
        ],
        &step[14].key(),
    );
    for (actual, expected) in [
        (step[1].key(), expected_admin),
        (step[2].key(), expected_lending),
        (step[4].key(), expected_f_token),
        (step[5].key(), expected_reserve),
        (step[6].key(), expected_supply),
        (step[7].key(), expected_rate),
        (step[8].key(), expected_vault),
        (step[9].key(), expected_claim),
        (step[10].key(), expected_liquidity),
        (step[12].key(), expected_rewards),
    ] {
        require_keys_eq!(actual, expected, ArbitrageError::InvalidAccount);
    }

    let admin = step[1].try_borrow_data()?;
    require!(
        admin.len() >= 72 && admin.starts_with(&LENDING_ADMIN_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&admin, 40)?,
        step[11].key(),
        ArbitrageError::InvalidAccount
    );
    let lending = step[2].try_borrow_data()?;
    require!(
        lending.len() == 196 && lending.starts_with(&LENDING_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&lending, 8)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&lending, 40)?,
        step[4].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&lending, 75)?,
        step[12].key(),
        ArbitrageError::InvalidAccount
    );
    require!(read_u64(&lending, 107)? > 0, ArbitrageError::InvalidAccount);
    require!(read_u64(&lending, 115)? > 0, ArbitrageError::InvalidAccount);
    require_keys_eq!(
        read_pubkey(&lending, 131)?,
        step[5].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&lending, 163)?,
        step[6].key(),
        ArbitrageError::InvalidAccount
    );

    let reserve = step[5].try_borrow_data()?;
    require!(
        reserve.len() == 192 && reserve.starts_with(&RESERVE_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&reserve, 8)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&reserve, 40)?,
        step[8].key(),
        ArbitrageError::InvalidAccount
    );
    let lending_liquidity_price = read_u64(&lending, 107)?;
    let lending_update_timestamp = read_u64(&lending, 123)?;
    require!(
        read_u64(&reserve, 86)? >= lending_liquidity_price
            && read_u64(&reserve, 78)? >= lending_update_timestamp,
        ArbitrageError::InvalidAccount
    );
    let supply = step[6].try_borrow_data()?;
    require!(
        supply.len() == 124 && supply.starts_with(&SUPPLY_POSITION_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&supply, 8)?,
        step[2].key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&supply, 40)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    let rewards = step[12].try_borrow_data()?;
    require!(
        rewards.len() == 89 && rewards.starts_with(&REWARDS_DISCRIMINATOR),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&rewards, 8)?,
        step[3].key(),
        ArbitrageError::InvalidAccount
    );
    validate_classic_mint(&step[3], None, lending[74])?;
    validate_classic_mint(&step[4], Some(step[1].key()), lending[74])?;
    validate_token_account_for_mint_and_authority(&step[8], &step[3], &step[13], &step[10])
}

fn validate_classic_mint(
    mint: &AccountInfo<'_>,
    authority: Option<Pubkey>,
    decimals: u8,
) -> Result<()> {
    require_keys_eq!(
        *mint.owner,
        anchor_spl::token::ID,
        ArbitrageError::InvalidTokenMint
    );
    let data = mint.try_borrow_data()?;
    require!(
        data.len() == 82 && data[45] == 1 && data[44] == decimals,
        ArbitrageError::InvalidTokenMint
    );
    require!(read_u64(&data, 36)? > 0, ArbitrageError::InvalidTokenMint);
    if let Some(authority) = authority {
        require!(read_u32(&data, 0)? == 1, ArbitrageError::InvalidTokenMint);
        require_keys_eq!(
            read_pubkey(&data, 4)?,
            authority,
            ArbitrageError::InvalidTokenMint
        );
    }
    Ok(())
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let bytes = data
        .get(offset..offset + 32)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(ArbitrageError::InvalidAccount)?;
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    ))
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
        let program = jupiter_lend_earn_program_id();
        let liquidity_program = jupiter_lend_liquidity_program_id();
        let rewards_program = jupiter_lend_rewards_program_id();
        let asset = Pubkey::new_from_array([3; 32]);
        let (admin, _) = Pubkey::find_program_address(&[b"lending_admin"], &program);
        let (f_token, _) =
            Pubkey::find_program_address(&[b"f_token_mint", asset.as_ref()], &program);
        let (lending, _) =
            Pubkey::find_program_address(&[b"lending", asset.as_ref(), f_token.as_ref()], &program);
        let (reserve, _) =
            Pubkey::find_program_address(&[b"reserve", asset.as_ref()], &liquidity_program);
        let (supply, _) = Pubkey::find_program_address(
            &[b"user_supply_position", asset.as_ref(), lending.as_ref()],
            &liquidity_program,
        );
        let (rate, _) =
            Pubkey::find_program_address(&[b"rate_model", asset.as_ref()], &liquidity_program);
        let (liquidity, _) = Pubkey::find_program_address(&[b"liquidity"], &liquidity_program);
        let (claim, _) = Pubkey::find_program_address(
            &[b"user_claim", admin.as_ref(), asset.as_ref()],
            &liquidity_program,
        );
        let (rewards, _) = Pubkey::find_program_address(
            &[b"lending_rewards_rate_model", asset.as_ref()],
            &rewards_program,
        );
        let (vault, _) = Pubkey::find_program_address(
            &[
                liquidity.as_ref(),
                anchor_spl::token::ID.as_ref(),
                asset.as_ref(),
            ],
            &anchor_spl::associated_token::ID,
        );

        let mut admin_data = vec![0_u8; 72];
        admin_data[..8].copy_from_slice(&LENDING_ADMIN_DISCRIMINATOR);
        write_pubkey(&mut admin_data, 40, liquidity_program);
        let mut lending_data = vec![0_u8; 196];
        lending_data[..8].copy_from_slice(&LENDING_DISCRIMINATOR);
        write_pubkey(&mut lending_data, 8, asset);
        write_pubkey(&mut lending_data, 40, f_token);
        lending_data[74] = 6;
        write_pubkey(&mut lending_data, 75, rewards);
        lending_data[107..115].copy_from_slice(&1_010_000_000_000_u64.to_le_bytes());
        lending_data[115..123].copy_from_slice(&1_009_000_000_000_u64.to_le_bytes());
        lending_data[123..131].copy_from_slice(&1_700_000_000_u64.to_le_bytes());
        write_pubkey(&mut lending_data, 131, reserve);
        write_pubkey(&mut lending_data, 163, supply);
        let mut reserve_data = vec![0_u8; 192];
        reserve_data[..8].copy_from_slice(&RESERVE_DISCRIMINATOR);
        write_pubkey(&mut reserve_data, 8, asset);
        write_pubkey(&mut reserve_data, 40, vault);
        reserve_data[78..86].copy_from_slice(&1_700_000_001_u64.to_le_bytes());
        reserve_data[86..94].copy_from_slice(&1_011_000_000_000_u64.to_le_bytes());
        let mut supply_data = vec![0_u8; 124];
        supply_data[..8].copy_from_slice(&SUPPLY_POSITION_DISCRIMINATOR);
        write_pubkey(&mut supply_data, 8, lending);
        write_pubkey(&mut supply_data, 40, asset);
        let mut rewards_data = vec![0_u8; 89];
        rewards_data[..8].copy_from_slice(&REWARDS_DISCRIMINATOR);
        write_pubkey(&mut rewards_data, 8, asset);

        vec![
            account(program, Pubkey::default(), vec![], false, true),
            account(admin, program, admin_data, false, false),
            account(lending, program, lending_data, true, false),
            account(
                asset,
                anchor_spl::token::ID,
                mint_data(None, 1_000_000_000),
                false,
                false,
            ),
            account(
                f_token,
                anchor_spl::token::ID,
                mint_data(Some(admin), 990_000_000),
                true,
                false,
            ),
            account(reserve, liquidity_program, reserve_data, true, false),
            account(supply, liquidity_program, supply_data, true, false),
            account(rate, liquidity_program, vec![1], false, false),
            account(
                vault,
                anchor_spl::token::ID,
                token_data(asset, liquidity, 1_000_000_000),
                true,
                false,
            ),
            account(claim, anchor_lang::system_program::ID, vec![], true, false),
            account(liquidity, liquidity_program, vec![1], true, false),
            account(liquidity_program, Pubkey::default(), vec![], false, true),
            account(rewards, rewards_program, rewards_data, false, false),
            account(
                anchor_spl::token::ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(
                anchor_spl::associated_token::ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
            account(
                anchor_lang::system_program::ID,
                Pubkey::default(),
                vec![],
                false,
                true,
            ),
        ]
    }

    #[test]
    fn encodes_native_min_output_for_both_directions() {
        for (direction, discriminator) in [(0, DEPOSIT_DISCRIMINATOR), (1, REDEEM_DISCRIMINATOR)] {
            let data = jupiter_lend_earn_data(direction, 1_000_000, 989_000);
            assert_eq!(data.len(), 24);
            assert_eq!(&data[..8], &discriminator);
            assert_eq!(&data[8..16], &1_000_000_u64.to_le_bytes());
            assert_eq!(&data[16..24], &989_000_u64.to_le_bytes());
        }
    }

    #[test]
    fn semantic_validation_binds_all_pdas_and_both_directions() {
        for direction in [0, 1] {
            let step = semantic_fixture();
            let (input, output) = if direction == 0 {
                (&step[3], &step[4])
            } else {
                (&step[4], &step[3])
            };
            assert!(validate_jupiter_lend_earn_semantic_accounts(
                &step, direction, 0, input, output,
            )
            .is_ok());
        }

        let mut step = semantic_fixture();
        step[9] = account(
            Pubkey::new_unique(),
            anchor_lang::system_program::ID,
            vec![],
            true,
            false,
        );
        assert!(
            validate_jupiter_lend_earn_semantic_accounts(&step, 1, 0, &step[4], &step[3]).is_err()
        );
    }
}
