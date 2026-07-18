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
        program_ids::{GFX_SSL_CONTROLLER, GFX_SSL_CONTROLLER_PROGRAM_ID, GFX_SSL_PROGRAM_ID},
        types::{
            read_token_amount, token_balance_delta, validate_token_account_for_mint_and_authority,
            SwapResult,
        },
    },
};

pub const GFX_SSL_FIXED_STEP_ACCOUNTS: usize = 11;
pub const GFX_SSL_MIN_STEP_ACCOUNTS: usize = 12;
pub const GFX_SSL_MAX_STEP_ACCOUNTS: usize = 31;

const PAIR_LEN: usize = 1_536;
const SSL_LEN: usize = 400;
const PAIR_DISCRIMINATOR: [u8; 8] = [85, 72, 49, 176, 182, 228, 141, 82];
const SSL_DISCRIMINATOR: [u8; 8] = [128, 155, 14, 83, 234, 219, 23, 190];
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const PAIR_ORACLES_OFFSET: usize = 112;
const PAIR_ORACLE_LEN: usize = 208;
const PAIR_ORACLE_COMPONENT_LEN: usize = 33;
const PAIR_ORACLE_COUNT_OFFSET: usize = 136;
const PAIR_N_ORACLE_OFFSET: usize = 1_152;
const PAIR_MAX_DELAY_OFFSET: usize = 1_168;
const PAIR_CONFIDENCE_OFFSET: usize = 1_176;
const PAIR_FEE_COLLECTOR_OFFSET: usize = 1_218;
const MAX_ORACLE_GROUPS: usize = 5;
const MAX_ORACLE_COMPONENTS: usize = 4;

pub struct GfxSslAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub user_input: &'a AccountInfo<'info>,
    pub user_output: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub step: &'a [AccountInfo<'info>],
}

pub fn validate_gfx_ssl_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        (GFX_SSL_MIN_STEP_ACCOUNTS..=GFX_SSL_MAX_STEP_ACCOUNTS).contains(&step.len()),
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    require!(fee_rate == 0, ArbitrageError::InvalidFeeAmount);
    require_keys_eq!(
        step[0].key(),
        GFX_SSL_PROGRAM_ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        step[1].key(),
        GFX_SSL_CONTROLLER,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        GFX_SSL_CONTROLLER_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require!(
        !step[1].try_borrow_data()?.is_empty(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );

    require_keys_eq!(
        *step[2].owner,
        GFX_SSL_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let pair = step[2].try_borrow_data()?;
    require!(
        pair.len() == PAIR_LEN && pair[..8] == PAIR_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    let controller = read_pubkey(&pair, 8)?;
    let mint0 = read_pubkey(&pair, 40)?;
    let mint1 = read_pubkey(&pair, 72)?;
    require_keys_eq!(controller, step[1].key(), ArbitrageError::InvalidAccount);
    require!(
        mint0 != Pubkey::default()
            && mint1 != Pubkey::default()
            && mint0.to_bytes() < mint1.to_bytes()
            && read_u64(&pair, PAIR_MAX_DELAY_OFFSET)? > 0
            && read_u64(&pair, PAIR_CONFIDENCE_OFFSET)? > 0,
        ArbitrageError::InvalidAccount
    );
    let expected_pair = Pubkey::find_program_address(
        &[
            b"GFX-SSL-Pair",
            controller.as_ref(),
            mint0.as_ref(),
            mint1.as_ref(),
        ],
        &GFX_SSL_PROGRAM_ID,
    );
    require_keys_eq!(
        step[2].key(),
        expected_pair.0,
        ArbitrageError::InvalidAccount
    );
    require!(pair[104] == expected_pair.1, ArbitrageError::InvalidAccount);
    let fee_collector = read_pubkey(&pair, PAIR_FEE_COLLECTOR_OFFSET)?;
    require!(
        fee_collector != Pubkey::default(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(step[9].key(), fee_collector, ArbitrageError::InvalidAccount);

    let (expected_input_mint, expected_output_mint) = if direction == 0 {
        (mint0, mint1)
    } else {
        (mint1, mint0)
    };
    require_keys_eq!(
        input_mint.key(),
        expected_input_mint,
        ArbitrageError::InvalidTokenMint
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output_mint,
        ArbitrageError::InvalidTokenMint
    );

    validate_ssl(&step[3], mint0)?;
    validate_ssl(&step[4], mint1)?;
    let expected_vaults = [
        associated_token_address(&step[3].key(), &mint0),
        associated_token_address(&step[3].key(), &mint1),
        associated_token_address(&step[4].key(), &mint1),
        associated_token_address(&step[4].key(), &mint0),
    ];
    let expected_mints = [mint0, mint1, mint1, mint0];
    let expected_authorities = [step[3].key(), step[3].key(), step[4].key(), step[4].key()];
    for index in 0..4 {
        require_keys_eq!(
            step[5 + index].key(),
            expected_vaults[index],
            ArbitrageError::InvalidAccount
        );
        let mint_ai = if expected_mints[index] == input_mint.key() {
            input_mint
        } else {
            output_mint
        };
        require_keys_eq!(
            mint_ai.key(),
            expected_mints[index],
            ArbitrageError::InvalidTokenMint
        );
        validate_token_account_for_mint_and_authority(
            &step[5 + index],
            mint_ai,
            token_program,
            if expected_authorities[index] == step[3].key() {
                &step[3]
            } else {
                &step[4]
            },
        )?;
    }
    let expected_fee_ata = associated_token_address(&fee_collector, &expected_input_mint);
    require_keys_eq!(
        step[10].key(),
        expected_fee_ata,
        ArbitrageError::InvalidAccount
    );
    validate_token_account_for_mint_and_authority(&step[10], input_mint, token_program, &step[9])?;
    let output_vaults = if direction == 0 {
        [&step[6], &step[7]]
    } else {
        [&step[5], &step[8]]
    };
    let output_liquidity = read_token_amount(output_vaults[0])?
        .checked_add(read_token_amount(output_vaults[1])?)
        .ok_or(ArbitrageError::InvalidAccount)?;
    require!(output_liquidity > 0, ArbitrageError::InsufficientLiquidity);
    validate_oracles(&pair, &step[GFX_SSL_FIXED_STEP_ACCOUNTS..])?;
    Ok(())
}

pub fn gfx_ssl_swap<'a, 'info>(
    accounts: GfxSslAccounts<'a, 'info>,
    amount_in: u64,
    min_amount_out: u64,
    direction: u8,
) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    require!(min_amount_out > 0, ArbitrageError::InvalidAmount);
    require!(direction <= 1, ArbitrageError::InvalidInstructionData);
    let step = accounts.step;
    let (ssl_in, ssl_out, liability_in, swapped_in, liability_out, swapped_out) = if direction == 0
    {
        (&step[3], &step[4], &step[5], &step[6], &step[7], &step[8])
    } else {
        (&step[4], &step[3], &step[7], &step[8], &step[5], &step[6])
    };
    let mut metas = vec![
        AccountMeta::new_readonly(step[1].key(), false),
        AccountMeta::new(step[2].key(), false),
        AccountMeta::new(ssl_in.key(), false),
        AccountMeta::new(ssl_out.key(), false),
        AccountMeta::new(liability_in.key(), false),
        AccountMeta::new(swapped_in.key(), false),
        AccountMeta::new(liability_out.key(), false),
        AccountMeta::new(swapped_out.key(), false),
        AccountMeta::new(accounts.user_input.key(), false),
        AccountMeta::new(accounts.user_output.key(), false),
        AccountMeta::new(step[10].key(), false),
        AccountMeta::new(accounts.payer.key(), true),
        AccountMeta::new_readonly(step[9].key(), false),
        AccountMeta::new_readonly(accounts.token_program.key(), false),
    ];
    metas.extend(
        step[GFX_SSL_FIXED_STEP_ACCOUNTS..]
            .iter()
            .map(|oracle| AccountMeta::new_readonly(oracle.key(), false)),
    );
    let instruction = Instruction {
        program_id: step[0].key(),
        accounts: metas,
        data: swap_data(amount_in, min_amount_out),
    };
    let pre_out = read_token_amount(accounts.user_output)?;
    let mut infos = vec![
        step[1].clone(),
        step[2].clone(),
        ssl_in.clone(),
        ssl_out.clone(),
        liability_in.clone(),
        swapped_in.clone(),
        liability_out.clone(),
        swapped_out.clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        step[10].clone(),
        accounts.payer.clone(),
        step[9].clone(),
        accounts.token_program.clone(),
    ];
    infos.extend_from_slice(&step[GFX_SSL_FIXED_STEP_ACCOUNTS..]);
    infos.push(step[0].clone());
    invoke(&instruction, &infos)?;
    Ok(SwapResult {
        amount_out: token_balance_delta(accounts.user_output, pre_out)?,
        fee_amount: 0,
    })
}

fn validate_ssl(account: &AccountInfo<'_>, mint: Pubkey) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        GFX_SSL_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    require!(
        data.len() == SSL_LEN && data[..8] == SSL_DISCRIMINATOR,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, 8)?,
        GFX_SSL_CONTROLLER,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        read_pubkey(&data, 40)?,
        mint,
        ArbitrageError::InvalidTokenMint
    );
    require!(data[75] == 0, ArbitrageError::InvalidAccount);
    let expected = Pubkey::find_program_address(
        &[b"GFX-SSL", GFX_SSL_CONTROLLER.as_ref(), mint.as_ref()],
        &GFX_SSL_PROGRAM_ID,
    );
    require_keys_eq!(account.key(), expected.0, ArbitrageError::InvalidAccount);
    require!(data[73] == expected.1, ArbitrageError::InvalidAccount);
    Ok(())
}

fn validate_oracles(pair: &[u8], oracles: &[AccountInfo<'_>]) -> Result<()> {
    let n_oracle = usize::try_from(read_u64(pair, PAIR_N_ORACLE_OFFSET)?)
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    require!(
        (1..=MAX_ORACLE_GROUPS).contains(&n_oracle),
        ArbitrageError::InvalidAccount
    );
    let mut cursor = 0usize;
    for group in 0..n_oracle {
        let group_offset = PAIR_ORACLES_OFFSET
            .checked_add(
                group
                    .checked_mul(PAIR_ORACLE_LEN)
                    .ok_or(ArbitrageError::InvalidAccount)?,
            )
            .ok_or(ArbitrageError::InvalidAccount)?;
        let count = usize::try_from(read_u64(
            pair,
            group_offset
                .checked_add(PAIR_ORACLE_COUNT_OFFSET)
                .ok_or(ArbitrageError::InvalidAccount)?,
        )?)
        .map_err(|_| ArbitrageError::InvalidAccount)?;
        require!(
            (1..=MAX_ORACLE_COMPONENTS).contains(&count),
            ArbitrageError::InvalidAccount
        );
        for component in 0..count {
            let offset = group_offset
                .checked_add(
                    component
                        .checked_mul(PAIR_ORACLE_COMPONENT_LEN)
                        .ok_or(ArbitrageError::InvalidAccount)?,
                )
                .ok_or(ArbitrageError::InvalidAccount)?;
            let oracle = oracles
                .get(cursor)
                .ok_or(ArbitrageError::InvalidAccountCount)?;
            require_keys_eq!(
                oracle.key(),
                read_pubkey(pair, offset)?,
                ArbitrageError::InvalidAccount
            );
            require!(
                matches!(pair.get(offset + 32), Some(0 | 1)),
                ArbitrageError::InvalidAccount
            );
            require!(
                !oracle.try_borrow_data()?.is_empty(),
                ArbitrageError::InvalidAccount
            );
            cursor = cursor
                .checked_add(1)
                .ok_or(ArbitrageError::InvalidAccountCount)?;
        }
    }
    require!(cursor == oracles.len(), ArbitrageError::InvalidAccountCount);
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

fn swap_data(amount_in: u64, min_amount_out: u64) -> Vec<u8> {
    let mut data = SWAP_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());
    data
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset
        .checked_add(32)
        .ok_or(ArbitrageError::InvalidAccount)?;
    let bytes: [u8; 32] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset
        .checked_add(8)
        .ok_or(ArbitrageError::InvalidAccount)?;
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

    #[test]
    fn swap_wire_layout_matches_official_anchor_idl() {
        let data = swap_data(11, 22);
        assert_eq!(data.len(), 24);
        assert_eq!(data[..8], SWAP_DISCRIMINATOR);
        assert_eq!(u64::from_le_bytes(data[8..16].try_into().unwrap()), 11);
        assert_eq!(u64::from_le_bytes(data[16..24].try_into().unwrap()), 22);
    }

    #[test]
    fn associated_token_address_is_bound_to_classic_token_program() {
        let authority = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        assert_eq!(
            associated_token_address(&authority, &mint),
            anchor_spl::associated_token::get_associated_token_address_with_program_id(
                &authority,
                &mint,
                &anchor_spl::token::ID,
            )
        );
    }
}
