use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    errors::ArbitrageError,
    instructions::{
        program_ids::M_SWAP_PROGRAM_ID,
        types::{read_token_amount, token_balance_delta, SwapResult},
    },
};

pub const M_SWAP_STEP_ACCOUNTS: usize = 14;

const SWAP_GLOBAL: Pubkey = anchor_lang::pubkey!("6U4ZZZkftbuHxjRDHUfh83M9zG66aAAXDV3xTRX7yePr");
const MZERO_PROGRAM: Pubkey = anchor_lang::pubkey!("wMXX1K1nca5W4pZr1piETe78gcAVVrEFi9f4g46uXko");
const MZERO_GLOBAL: Pubkey = anchor_lang::pubkey!("GQBavw2gpCdbZkkSWk9PkzNTDdBwCHUGNpeuuQ7mV9GA");
const MZERO_MINT: Pubkey = anchor_lang::pubkey!("mzeroXDoBpRVhnEXBra27qzAMdxgpWVY3DzQW7xMVJp");
const MZERO_VAULT_AUTH: Pubkey =
    anchor_lang::pubkey!("8vtsGdu4ErjK2skhV7FfPQwXdae6myWjgWJ8gRMnXi2K");
const MZERO_MINT_AUTH: Pubkey =
    anchor_lang::pubkey!("Anfx7wng5TEe5UrkFKTirtADBawmtRs9KoD15BUbEmvT");
const MZERO_M_VAULT: Pubkey = anchor_lang::pubkey!("7upNeuSPSpinN7zzEsrxMe6p3N6tMub67dkkm5LFBTvp");
const DAWN_PROGRAM: Pubkey = anchor_lang::pubkey!("mextzNVPUbLbvyBwqBqnC5J1SSwjDLjjR4yppf6EBzc");
const DAWN_GLOBAL: Pubkey = anchor_lang::pubkey!("G7dEeYfaTpJy39FtYkFjQVkCXiFEcvfAtr2zCAJppbpL");
const DAWN_MINT: Pubkey = anchor_lang::pubkey!("dawn7ZUF7h7anFuEsDdAU1Y3HYwikwqNMAENZsQJdNL");
const DAWN_VAULT_AUTH: Pubkey =
    anchor_lang::pubkey!("Hkyu8KCkXg16w74BL2D5JpsL5VZhKkXGpPj8UD6wyf9x");
const DAWN_MINT_AUTH: Pubkey = anchor_lang::pubkey!("Am4facCvkQkHjwSArPX8Jqxs1ss14XMoC8JkTV3BDG95");
const DAWN_M_VAULT: Pubkey = anchor_lang::pubkey!("3rtUvF3XSMPTLw1pPLgeanfCu4T9YML8Vm1nMRRHagWd");
const M_MINT: Pubkey = anchor_lang::pubkey!("mzerojk9tg56ebsrEAhfkyc9VgKjTW2zDqp6C5mhjzH");
const M_EARN_GLOBAL: Pubkey = anchor_lang::pubkey!("CQNpruTHcw9QLfCG3gPaLQsFSqNz5XdtJzRDNWoSv3bZ");
const SWAP_M_ACCOUNT: Pubkey = anchor_lang::pubkey!("7dM9YCAbN9XGixnaP7wnyQDVZH6BVy6HPFeZr1SWVNka");
const TOKEN_2022_PROGRAM: Pubkey =
    anchor_lang::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const SYSTEM_PROGRAM: Pubkey = anchor_lang::pubkey!("11111111111111111111111111111111");
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_GLOBAL_DISCRIMINATOR: [u8; 8] = [15, 184, 147, 129, 183, 219, 223, 163];
const EXT_GLOBAL_V2_DISCRIMINATOR: [u8; 8] = [116, 209, 219, 83, 70, 143, 55, 127];

pub struct MSwapAccounts<'info> {
    pub program: &'info AccountInfo<'info>,
    pub swap_global: &'info AccountInfo<'info>,
    pub from_global: &'info AccountInfo<'info>,
    pub to_global: &'info AccountInfo<'info>,
    pub m_mint: &'info AccountInfo<'info>,
    pub swap_m_account: &'info AccountInfo<'info>,
    pub from_vault_auth: &'info AccountInfo<'info>,
    pub to_vault_auth: &'info AccountInfo<'info>,
    pub from_mint_auth: &'info AccountInfo<'info>,
    pub to_mint_auth: &'info AccountInfo<'info>,
    pub from_m_vault: &'info AccountInfo<'info>,
    pub to_m_vault: &'info AccountInfo<'info>,
    pub from_program: &'info AccountInfo<'info>,
    pub to_program: &'info AccountInfo<'info>,
    pub user: &'info AccountInfo<'info>,
    pub user_input: &'info AccountInfo<'info>,
    pub user_output: &'info AccountInfo<'info>,
    pub input_mint: &'info AccountInfo<'info>,
    pub output_mint: &'info AccountInfo<'info>,
    pub input_token_program: &'info AccountInfo<'info>,
    pub output_token_program: &'info AccountInfo<'info>,
    pub token_2022_program: &'info AccountInfo<'info>,
    pub system_program: &'info AccountInfo<'info>,
}

pub fn m_swap<'info>(accounts: MSwapAccounts<'info>, amount_in: u64) -> Result<SwapResult> {
    require!(amount_in > 0, ArbitrageError::InvalidAmount);
    let pre_out = read_token_amount(accounts.user_output)?;
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(&SWAP_DISCRIMINATOR);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.push(0);
    let metas = vec![
        AccountMeta::new_readonly(accounts.user.key(), true),
        AccountMeta::new_readonly(accounts.program.key(), false),
        AccountMeta::new_readonly(accounts.program.key(), false),
        AccountMeta::new_readonly(accounts.swap_global.key(), false),
        AccountMeta::new(accounts.from_global.key(), false),
        AccountMeta::new(accounts.to_global.key(), false),
        AccountMeta::new(accounts.input_mint.key(), false),
        AccountMeta::new(accounts.output_mint.key(), false),
        AccountMeta::new_readonly(accounts.m_mint.key(), false),
        AccountMeta::new(accounts.user_input.key(), false),
        AccountMeta::new(accounts.user_output.key(), false),
        AccountMeta::new(accounts.swap_m_account.key(), false),
        AccountMeta::new_readonly(accounts.from_vault_auth.key(), false),
        AccountMeta::new_readonly(accounts.to_vault_auth.key(), false),
        AccountMeta::new_readonly(accounts.from_mint_auth.key(), false),
        AccountMeta::new_readonly(accounts.to_mint_auth.key(), false),
        AccountMeta::new(accounts.from_m_vault.key(), false),
        AccountMeta::new(accounts.to_m_vault.key(), false),
        AccountMeta::new_readonly(accounts.input_token_program.key(), false),
        AccountMeta::new_readonly(accounts.output_token_program.key(), false),
        AccountMeta::new_readonly(accounts.token_2022_program.key(), false),
        AccountMeta::new_readonly(accounts.from_program.key(), false),
        AccountMeta::new_readonly(accounts.to_program.key(), false),
        AccountMeta::new_readonly(accounts.system_program.key(), false),
    ];
    let infos = vec![
        accounts.user.clone(),
        accounts.program.clone(),
        accounts.program.clone(),
        accounts.swap_global.clone(),
        accounts.from_global.clone(),
        accounts.to_global.clone(),
        accounts.input_mint.clone(),
        accounts.output_mint.clone(),
        accounts.m_mint.clone(),
        accounts.user_input.clone(),
        accounts.user_output.clone(),
        accounts.swap_m_account.clone(),
        accounts.from_vault_auth.clone(),
        accounts.to_vault_auth.clone(),
        accounts.from_mint_auth.clone(),
        accounts.to_mint_auth.clone(),
        accounts.from_m_vault.clone(),
        accounts.to_m_vault.clone(),
        accounts.input_token_program.clone(),
        accounts.output_token_program.clone(),
        accounts.token_2022_program.clone(),
        accounts.from_program.clone(),
        accounts.to_program.clone(),
        accounts.system_program.clone(),
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

#[allow(clippy::too_many_arguments)]
pub fn validate_m_swap_semantic_accounts<'info>(
    step: &[AccountInfo<'info>],
    direction: u8,
    fee_rate: u16,
    input_mint: &AccountInfo<'info>,
    output_mint: &AccountInfo<'info>,
    input_token_program: &AccountInfo<'info>,
    output_token_program: &AccountInfo<'info>,
    token_2022_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> Result<()> {
    require!(
        step.len() == M_SWAP_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require!(direction <= 1, ArbitrageError::InvalidPath);
    require!(fee_rate == 0, ArbitrageError::InvalidInstructionData);
    let reverse = direction == 1;
    let ordered = |a, b| if reverse { (b, a) } else { (a, b) };
    let (from_global, to_global) = ordered(MZERO_GLOBAL, DAWN_GLOBAL);
    let (from_vault_auth, to_vault_auth) = ordered(MZERO_VAULT_AUTH, DAWN_VAULT_AUTH);
    let (from_mint_auth, to_mint_auth) = ordered(MZERO_MINT_AUTH, DAWN_MINT_AUTH);
    let (from_vault, to_vault) = ordered(MZERO_M_VAULT, DAWN_M_VAULT);
    let (from_program, to_program) = ordered(MZERO_PROGRAM, DAWN_PROGRAM);
    let expected = [
        M_SWAP_PROGRAM_ID,
        SWAP_GLOBAL,
        from_global,
        to_global,
        M_MINT,
        SWAP_M_ACCOUNT,
        from_vault_auth,
        to_vault_auth,
        from_mint_auth,
        to_mint_auth,
        from_vault,
        to_vault,
        from_program,
        to_program,
    ];
    for (account, key) in step.iter().zip(expected) {
        require_keys_eq!(account.key(), key, ArbitrageError::InvalidAccount);
    }
    let (expected_input, expected_output) = ordered(MZERO_MINT, DAWN_MINT);
    require_keys_eq!(
        input_mint.key(),
        expected_input,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        output_mint.key(),
        expected_output,
        ArbitrageError::InvalidAccount
    );
    for account in [input_mint, output_mint, &step[4]] {
        require_keys_eq!(
            *account.owner,
            TOKEN_2022_PROGRAM,
            ArbitrageError::InvalidAccount
        );
        let data = account.try_borrow_data()?;
        require!(
            data.len() >= 45 && data[44] == 6,
            ArbitrageError::InvalidAccount
        );
    }
    for program in [
        input_token_program,
        output_token_program,
        token_2022_program,
    ] {
        require_keys_eq!(
            program.key(),
            TOKEN_2022_PROGRAM,
            ArbitrageError::InvalidAccount
        );
    }
    require_keys_eq!(
        system_program.key(),
        SYSTEM_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        *step[1].owner,
        M_SWAP_PROGRAM_ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(*step[2].owner, from_program, ArbitrageError::InvalidAccount);
    require_keys_eq!(*step[3].owner, to_program, ArbitrageError::InvalidAccount);
    validate_swap_global(&step[1])?;
    validate_extension_global(&step[2], expected_input, !reverse)?;
    validate_extension_global(&step[3], expected_output, reverse)?;
    validate_token_account(&step[5], M_MINT, SWAP_GLOBAL, true)?;
    validate_token_account(&step[10], M_MINT, from_vault_auth, false)?;
    validate_token_account(&step[11], M_MINT, to_vault_auth, false)?;
    Ok(())
}

fn validate_swap_global(account: &AccountInfo) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.get(..8) == Some(SWAP_GLOBAL_DISCRIMINATOR.as_slice()),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn validate_extension_global(
    account: &AccountInfo,
    ext_mint: Pubkey,
    is_mzero: bool,
) -> Result<()> {
    let data = account.try_borrow_data()?;
    require!(
        data.get(..8) == Some(EXT_GLOBAL_V2_DISCRIMINATOR.as_slice())
            && data.get(40).copied() == Some(0)
            && pubkey_at(&data, 41)? == ext_mint
            && pubkey_at(&data, 73)? == M_MINT
            && pubkey_at(&data, 105)? == M_EARN_GLOBAL,
        ArbitrageError::InvalidAccount
    );
    if is_mzero {
        let last_m = u64_at(&data, 173)?;
        let last_ext = u64_at(&data, 181)?;
        require!(
            data.get(140).copied() == Some(2) && last_m > 0 && last_m == last_ext,
            ArbitrageError::InvalidAccount
        );
        require!(
            pubkey_vec_contains(&data, 197, SWAP_GLOBAL)?,
            ArbitrageError::InvalidAccount
        );
    } else {
        require!(
            data.get(140).copied() == Some(0) && pubkey_vec_contains(&data, 141, SWAP_GLOBAL)?,
            ArbitrageError::InvalidAccount
        );
    }
    Ok(())
}

fn validate_token_account(
    account: &AccountInfo,
    mint: Pubkey,
    authority: Pubkey,
    require_empty: bool,
) -> Result<()> {
    require_keys_eq!(
        *account.owner,
        TOKEN_2022_PROGRAM,
        ArbitrageError::InvalidAccount
    );
    let data = account.try_borrow_data()?;
    let amount = u64_at(&data, 64)?;
    require!(
        pubkey_at(&data, 0)? == mint
            && pubkey_at(&data, 32)? == authority
            && (!require_empty || amount == 0),
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

fn pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey> {
    let end = offset.checked_add(32).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 32] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(Pubkey::new_from_array(bytes))
}

fn u64_at(data: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(ArbitrageError::MathOverflow)?;
    let bytes: [u8; 8] = data
        .get(offset..end)
        .ok_or(ArbitrageError::InvalidAccount)?
        .try_into()
        .map_err(|_| ArbitrageError::InvalidAccount)?;
    Ok(u64::from_le_bytes(bytes))
}

fn pubkey_vec_contains(data: &[u8], offset: usize, needle: Pubkey) -> Result<bool> {
    let len_end = offset.checked_add(4).ok_or(ArbitrageError::MathOverflow)?;
    let count = u32::from_le_bytes(
        data.get(offset..len_end)
            .ok_or(ArbitrageError::InvalidAccount)?
            .try_into()
            .map_err(|_| ArbitrageError::InvalidAccount)?,
    );
    require!(count <= 64, ArbitrageError::InvalidAccount);
    for index in 0..count {
        let key_offset = len_end
            .checked_add(
                (index as usize)
                    .checked_mul(32)
                    .ok_or(ArbitrageError::MathOverflow)?,
            )
            .ok_or(ArbitrageError::MathOverflow)?;
        if pubkey_at(data, key_offset)? == needle {
            return Ok(true);
        }
    }
    Ok(false)
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

    fn mint(key: Pubkey) -> AccountInfo<'static> {
        let mut data = vec![0_u8; 45];
        data[44] = 6;
        account(key, TOKEN_2022_PROGRAM, data, true, false)
    }

    fn token_account(key: Pubkey, authority: Pubkey, amount: u64) -> AccountInfo<'static> {
        let mut data = vec![0_u8; 72];
        data[..32].copy_from_slice(M_MINT.as_ref());
        data[32..64].copy_from_slice(authority.as_ref());
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        account(key, TOKEN_2022_PROGRAM, data, true, false)
    }

    fn extension_global(ext_mint: Pubkey, mzero: bool) -> Vec<u8> {
        let mut data = vec![0_u8; if mzero { 393 } else { 273 }];
        data[..8].copy_from_slice(&EXT_GLOBAL_V2_DISCRIMINATOR);
        data[40] = 0;
        data[41..73].copy_from_slice(ext_mint.as_ref());
        data[73..105].copy_from_slice(M_MINT.as_ref());
        data[105..137].copy_from_slice(M_EARN_GLOBAL.as_ref());
        if mzero {
            data[140] = 2;
            data[173..181].copy_from_slice(&1_088_214_891_562_u64.to_le_bytes());
            data[181..189].copy_from_slice(&1_088_214_891_562_u64.to_le_bytes());
            data[197..201].copy_from_slice(&1_u32.to_le_bytes());
            data[201..233].copy_from_slice(SWAP_GLOBAL.as_ref());
        } else {
            data[140] = 0;
            data[141..145].copy_from_slice(&1_u32.to_le_bytes());
            data[145..177].copy_from_slice(SWAP_GLOBAL.as_ref());
        }
        data
    }

    fn step(direction: u8) -> Vec<AccountInfo<'static>> {
        let reverse = direction == 1;
        let ordered = |a, b| if reverse { (b, a) } else { (a, b) };
        let (from_global, to_global) = ordered(MZERO_GLOBAL, DAWN_GLOBAL);
        let (from_program, to_program) = ordered(MZERO_PROGRAM, DAWN_PROGRAM);
        let (from_vault_auth, to_vault_auth) = ordered(MZERO_VAULT_AUTH, DAWN_VAULT_AUTH);
        let (from_mint_auth, to_mint_auth) = ordered(MZERO_MINT_AUTH, DAWN_MINT_AUTH);
        let (from_vault, to_vault) = ordered(MZERO_M_VAULT, DAWN_M_VAULT);
        vec![
            account(M_SWAP_PROGRAM_ID, Pubkey::default(), vec![], false, true),
            account(
                SWAP_GLOBAL,
                M_SWAP_PROGRAM_ID,
                SWAP_GLOBAL_DISCRIMINATOR.to_vec(),
                false,
                false,
            ),
            account(
                from_global,
                from_program,
                extension_global(if reverse { DAWN_MINT } else { MZERO_MINT }, !reverse),
                true,
                false,
            ),
            account(
                to_global,
                to_program,
                extension_global(if reverse { MZERO_MINT } else { DAWN_MINT }, reverse),
                true,
                false,
            ),
            mint(M_MINT),
            token_account(SWAP_M_ACCOUNT, SWAP_GLOBAL, 0),
            account(from_vault_auth, Pubkey::default(), vec![], false, false),
            account(to_vault_auth, Pubkey::default(), vec![], false, false),
            account(from_mint_auth, Pubkey::default(), vec![], false, false),
            account(to_mint_auth, Pubkey::default(), vec![], false, false),
            token_account(from_vault, from_vault_auth, 1_000_000),
            token_account(to_vault, to_vault_auth, 1_000_000),
            account(from_program, Pubkey::default(), vec![], false, true),
            account(to_program, Pubkey::default(), vec![], false, true),
        ]
    }

    #[test]
    fn deployed_swap_payload_matches_confirmed_wire_data() {
        let amount = 94_672_609_u64;
        let mut data = Vec::new();
        data.extend_from_slice(&SWAP_DISCRIMINATOR);
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(0);
        assert_eq!(data.len(), 17);
        assert_eq!(
            data,
            [
                0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8, 0xe1, 0x96, 0xa4, 0x05, 0x00, 0x00,
                0x00, 0x00, 0x00,
            ]
        );
    }

    #[test]
    fn semantic_validation_accepts_both_canonical_directions_and_rejects_index_drift() {
        let token_2022 = account(TOKEN_2022_PROGRAM, Pubkey::default(), vec![], false, true);
        let system = account(SYSTEM_PROGRAM, Pubkey::default(), vec![], false, true);
        for direction in [0, 1] {
            let accounts = step(direction);
            let (input, output) = if direction == 0 {
                (mint(MZERO_MINT), mint(DAWN_MINT))
            } else {
                (mint(DAWN_MINT), mint(MZERO_MINT))
            };
            assert!(validate_m_swap_semantic_accounts(
                &accounts,
                direction,
                0,
                &input,
                &output,
                &token_2022,
                &token_2022,
                &token_2022,
                &system,
            )
            .is_ok());
        }

        let mut drifted = step(0);
        let mut data = extension_global(MZERO_MINT, true);
        data[181..189].copy_from_slice(&1_088_214_891_561_u64.to_le_bytes());
        drifted[2] = account(MZERO_GLOBAL, MZERO_PROGRAM, data, true, false);
        assert!(validate_m_swap_semantic_accounts(
            &drifted,
            0,
            0,
            &mint(MZERO_MINT),
            &mint(DAWN_MINT),
            &token_2022,
            &token_2022,
            &token_2022,
            &system,
        )
        .is_err());
    }
}
