use anchor_lang::prelude::*;

use crate::errors::ArbitrageError;
use crate::state::Protocol;

pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
pub const RAYDIUM_CPMM_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW");
pub const RAYDIUM_CLMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
pub const RAYDIUM_CLMM_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("devi51mZmdwUJGU9hjN27vEz64Gps7uUefqxg27EAtH");
pub const RAYDIUM_POOL_V4_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const RAYDIUM_POOL_V4_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("DRaya7Kj3aMWQSy19kSjvmuwq9docCHofyP9kanQGaav");
pub const RAYDIUM_LAUNCHPAD_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");
pub const PUMPFUN_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMPFUN_AMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const MEMO_PROGRAM_V2_ID: Pubkey =
    anchor_lang::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

pub const RAYDIUM_CPMM_AUTHORITY_ID: Pubkey =
    anchor_lang::pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");
pub const RAYDIUM_CPMM_AUTHORITY_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("7rQ1QFNosMkUCuh7Z7fPbTHvh73b68sQYdirycEzJVuw");
pub const RAYDIUM_LAUNCHPAD_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh");
pub const RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr");
pub const PUMPFUN_SWAP_GLOBAL_ACCOUNT: Pubkey =
    anchor_lang::pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUMPFUN_SWAP_FEE_RECIPIENT: Pubkey =
    anchor_lang::pubkey!("68yFSZxzLWJXkxxRGydZ63C6mHx1NLEDWmwN9Lb5yySg");
pub const PUMPFUN_SWAP_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const PUMPFUN_AMM_GLOBAL_CONFIG_ACCOUNT: Pubkey =
    anchor_lang::pubkey!("ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw");
pub const PUMPFUN_AMM_FEE_RECIPIENT: Pubkey =
    anchor_lang::pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");
pub const PUMPFUN_AMM_FEE_RECIPIENT_DEVNET: Pubkey =
    anchor_lang::pubkey!("3PAxmkxnM2vHno9amWQCsaaFjYnPGcD87HZGx1ChVjPj");
pub const PUMPFUN_AMM_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR");
pub const PUMPFUN_SWAP_FEE_CONFIG: Pubkey =
    anchor_lang::pubkey!("8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt");
pub const PUMPFUN_AMM_FEE_CONFIG: Pubkey =
    anchor_lang::pubkey!("5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx");
pub const PUMPFUN_SWAP_FEE_CONFIG_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedAccountExpectation {
    pub index: usize,
    pub key: Pubkey,
}

pub fn validate_step_program_account(protocol: Protocol, program: &AccountInfo) -> Result<()> {
    require!(program.executable, ArbitrageError::InvalidProgramId);
    validate_protocol_program_id(protocol, program.key)
}

pub fn validate_step_fixed_accounts<'info>(
    protocol: Protocol,
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    validate_protocol_fixed_account_keys(protocol, |index| {
        step_accounts.get(index).map(AccountInfo::key)
    })
}

pub fn validate_protocol_program_id(protocol: Protocol, program_id: &Pubkey) -> Result<()> {
    #[cfg(feature = "flex")]
    {
        let _ = protocol;
        require!(
            program_id != &Pubkey::default(),
            ArbitrageError::InvalidProgramId
        );
        Ok(())
    }

    #[cfg(not(feature = "flex"))]
    {
        require!(
            program_id == &expected_protocol_program_id(protocol),
            ArbitrageError::InvalidProgramId
        );
        Ok(())
    }
}

#[cfg(feature = "flex")]
pub fn validate_protocol_fixed_account_keys(
    protocol: Protocol,
    mut key_at: impl FnMut(usize) -> Option<Pubkey>,
) -> Result<()> {
    let _ = protocol;
    let _ = &mut key_at;
    Ok(())
}

#[cfg(not(feature = "flex"))]
pub fn validate_protocol_fixed_account_keys(
    protocol: Protocol,
    mut key_at: impl FnMut(usize) -> Option<Pubkey>,
) -> Result<()> {
    for expected in expected_fixed_accounts(protocol) {
        let actual_key = key_at(expected.index).ok_or(ArbitrageError::InvalidAccountCount)?;
        require_keys_eq!(actual_key, expected.key, ArbitrageError::InvalidAccount);
    }
    Ok(())
}

#[cfg(not(feature = "flex"))]
pub fn expected_protocol_program_id(protocol: Protocol) -> Pubkey {
    match protocol {
        Protocol::RaydiumCPMM => raydium_cpmm_program_id(),
        Protocol::RaydiumCLMM => raydium_clmm_program_id(),
        Protocol::RaydiumPoolV4 => raydium_pool_v4_program_id(),
        Protocol::RaydiumLaunchPad => RAYDIUM_LAUNCHPAD_PROGRAM_ID,
        Protocol::PumpFunSwap => PUMPFUN_SWAP_PROGRAM_ID,
        Protocol::PumpFunAMM => PUMPFUN_AMM_PROGRAM_ID,
    }
}

#[cfg(not(feature = "flex"))]
pub fn expected_fixed_accounts(protocol: Protocol) -> &'static [FixedAccountExpectation] {
    match protocol {
        Protocol::RaydiumCPMM => raydium_cpmm_fixed_accounts(),
        Protocol::RaydiumCLMM => RAYDIUM_CLMM_FIXED_ACCOUNTS,
        Protocol::RaydiumPoolV4 => NO_FIXED_ACCOUNTS,
        Protocol::RaydiumLaunchPad => RAYDIUM_LAUNCHPAD_FIXED_ACCOUNTS,
        Protocol::PumpFunSwap => PUMPFUN_SWAP_FIXED_ACCOUNTS,
        Protocol::PumpFunAMM => pumpfun_amm_fixed_accounts(),
    }
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_cpmm_program_id() -> Pubkey {
    RAYDIUM_CPMM_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_cpmm_program_id() -> Pubkey {
    RAYDIUM_CPMM_PROGRAM_ID
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_clmm_program_id() -> Pubkey {
    RAYDIUM_CLMM_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_clmm_program_id() -> Pubkey {
    RAYDIUM_CLMM_PROGRAM_ID
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_pool_v4_program_id() -> Pubkey {
    RAYDIUM_POOL_V4_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_pool_v4_program_id() -> Pubkey {
    RAYDIUM_POOL_V4_PROGRAM_ID
}

#[cfg(not(feature = "flex"))]
const NO_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[];

#[cfg(all(feature = "devnet", not(feature = "flex")))]
const RAYDIUM_CPMM_FIXED_ACCOUNTS_DEVNET: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 1,
    key: RAYDIUM_CPMM_AUTHORITY_ID_DEVNET,
}];

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
const RAYDIUM_CPMM_FIXED_ACCOUNTS_MAINNET: &[FixedAccountExpectation] =
    &[FixedAccountExpectation {
        index: 1,
        key: RAYDIUM_CPMM_AUTHORITY_ID,
    }];

#[cfg(not(feature = "flex"))]
const RAYDIUM_CLMM_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 6,
    key: MEMO_PROGRAM_V2_ID,
}];

#[cfg(not(feature = "flex"))]
const RAYDIUM_LAUNCHPAD_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: RAYDIUM_LAUNCHPAD_AUTHORITY,
    },
    FixedAccountExpectation {
        index: 7,
        key: RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY,
    },
];

#[cfg(not(feature = "flex"))]
const PUMPFUN_SWAP_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: PUMPFUN_SWAP_GLOBAL_ACCOUNT,
    },
    FixedAccountExpectation {
        index: 2,
        key: PUMPFUN_SWAP_FEE_RECIPIENT,
    },
    FixedAccountExpectation {
        index: 6,
        key: PUMPFUN_SWAP_EVENT_AUTHORITY,
    },
];

#[cfg(all(feature = "devnet", not(feature = "flex")))]
const PUMPFUN_AMM_FIXED_ACCOUNTS_DEVNET: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 2,
        key: PUMPFUN_AMM_GLOBAL_CONFIG_ACCOUNT,
    },
    FixedAccountExpectation {
        index: 5,
        key: PUMPFUN_AMM_FEE_RECIPIENT_DEVNET,
    },
    FixedAccountExpectation {
        index: 7,
        key: PUMPFUN_AMM_EVENT_AUTHORITY,
    },
];

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
const PUMPFUN_AMM_FIXED_ACCOUNTS_MAINNET: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 2,
        key: PUMPFUN_AMM_GLOBAL_CONFIG_ACCOUNT,
    },
    FixedAccountExpectation {
        index: 5,
        key: PUMPFUN_AMM_FEE_RECIPIENT,
    },
    FixedAccountExpectation {
        index: 7,
        key: PUMPFUN_AMM_EVENT_AUTHORITY,
    },
];

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_cpmm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    RAYDIUM_CPMM_FIXED_ACCOUNTS_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_cpmm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    RAYDIUM_CPMM_FIXED_ACCOUNTS_MAINNET
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn pumpfun_amm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    PUMPFUN_AMM_FIXED_ACCOUNTS_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn pumpfun_amm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    PUMPFUN_AMM_FIXED_ACCOUNTS_MAINNET
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUSTOM_PROGRAM_ID: Pubkey =
        anchor_lang::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");

    #[cfg(not(feature = "flex"))]
    #[test]
    fn expected_programs_match_selected_cluster_defaults() {
        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCPMM),
            RAYDIUM_CPMM_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCPMM),
            RAYDIUM_CPMM_PROGRAM_ID_DEVNET
        );

        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCLMM),
            RAYDIUM_CLMM_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCLMM),
            RAYDIUM_CLMM_PROGRAM_ID_DEVNET
        );

        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumPoolV4),
            RAYDIUM_POOL_V4_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumPoolV4),
            RAYDIUM_POOL_V4_PROGRAM_ID_DEVNET
        );

        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumLaunchPad),
            RAYDIUM_LAUNCHPAD_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::PumpFunSwap),
            PUMPFUN_SWAP_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::PumpFunAMM),
            PUMPFUN_AMM_PROGRAM_ID
        );
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn protocol_program_validation_rejects_wrong_program_id() {
        let err = validate_protocol_program_id(Protocol::RaydiumCPMM, &CUSTOM_PROGRAM_ID)
            .expect_err("wrong program id should fail");

        assert_eq!(err, ArbitrageError::InvalidProgramId.into());
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn protocol_program_validation_accepts_expected_program_id() {
        let expected_program_id = expected_protocol_program_id(Protocol::RaydiumCPMM);

        assert!(validate_protocol_program_id(Protocol::RaydiumCPMM, &expected_program_id).is_ok());
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn fixed_account_validation_accepts_expected_keys() {
        let mut keys = [Pubkey::default(); 8];
        keys[1] = RAYDIUM_LAUNCHPAD_AUTHORITY;
        keys[7] = RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY;

        assert!(
            validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
                keys.get(index).copied()
            })
            .is_ok()
        );
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn fixed_account_validation_rejects_wrong_key() {
        let mut keys = [Pubkey::default(); 8];
        keys[1] = RAYDIUM_LAUNCHPAD_AUTHORITY;
        keys[7] = CUSTOM_PROGRAM_ID;

        let err = validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
            keys.get(index).copied()
        })
        .expect_err("wrong fixed account should fail");

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn fixed_account_validation_rejects_missing_key() {
        let keys = [RAYDIUM_LAUNCHPAD_AUTHORITY];

        let err = validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
            keys.get(index).copied()
        })
        .expect_err("missing fixed account should fail");

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[cfg(feature = "flex")]
    #[test]
    fn flex_program_validation_accepts_custom_non_default_program_id() {
        assert!(validate_protocol_program_id(Protocol::RaydiumCPMM, &CUSTOM_PROGRAM_ID).is_ok());
    }

    #[cfg(feature = "flex")]
    #[test]
    fn flex_program_validation_rejects_default_program_id() {
        let err = validate_protocol_program_id(Protocol::RaydiumCPMM, &Pubkey::default())
            .expect_err("default program id should fail");

        assert_eq!(err, ArbitrageError::InvalidProgramId.into());
    }

    #[cfg(feature = "flex")]
    #[test]
    fn flex_fixed_account_validation_skips_canonical_key_checks() {
        let keys = [Pubkey::default()];

        assert!(
            validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
                keys.get(index).copied()
            })
            .is_ok()
        );
    }
}
