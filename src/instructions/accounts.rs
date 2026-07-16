use anchor_lang::prelude::*;
use anchor_spl::{associated_token, token, token_2022};
use core::ops::Range;

use crate::errors::ArbitrageError;
use crate::protocal::{
    aldrin_v2::ALDRIN_V2_MIN_ACCOUNTS,
    aquifer::AQUIFER_STEP_ACCOUNTS,
    binaryfi::BINARYFI_MIN_ACCOUNTS,
    bisonfi::BISONFI_STEP_ACCOUNTS,
    bonk_swap::BONK_SWAP_MIN_ACCOUNTS,
    boop_fun::BOOP_FUN_STEP_ACCOUNTS,
    carrot::CARROT_STEP_ACCOUNTS,
    crema_clmm::{
        CREMA_CLMM_FIXED_STEP_ACCOUNTS, CREMA_CLMM_MAX_STEP_ACCOUNTS, CREMA_CLMM_MIN_STEP_ACCOUNTS,
    },
    cropper::CROPPER_STEP_ACCOUNTS,
    deriverse::DERIVERSE_STEP_ACCOUNTS,
    fluxbeam::FLUXBEAM_MIN_ACCOUNTS,
    gamma_swap::GAMMA_SWAP_MIN_ACCOUNTS,
    goonfi::GOONFI_MIN_ACCOUNTS,
    goonfi_v2::GOONFI_V2_MIN_ACCOUNTS,
    guacswap::GUACSWAP_MIN_ACCOUNTS,
    hadron::{HADRON_BASE_STEP_ACCOUNTS, HADRON_SPREAD_STEP_ACCOUNTS},
    heaven::HEAVEN_STEP_ACCOUNTS,
    helium_treasury_management::HELIUM_TREASURY_MANAGEMENT_STEP_ACCOUNTS,
    huma::HUMA_STEP_ACCOUNTS,
    humidifi::HUMIDIFI_MIN_ACCOUNTS,
    hylo_earn_pool::HYLO_EARN_POOL_STEP_ACCOUNTS,
    hylo_exchange::HYLO_EXCHANGE_STEP_ACCOUNTS,
    invariant::INVARIANT_MIN_ACCOUNTS,
    jupiter_lend_earn::JUPITER_LEND_EARN_STEP_ACCOUNTS,
    kipseli::KIPSELI_STEP_ACCOUNTS,
    lemmingsfi::LEMMINGSFI_STEP_ACCOUNTS,
    lifinity_amm_v1::LIFINITY_AMM_V1_MIN_ACCOUNTS,
    lifinity_amm_v2::LIFINITY_AMM_V2_MIN_ACCOUNTS,
    m_swap::M_SWAP_STEP_ACCOUNTS,
    manifest::MANIFEST_MIN_ACCOUNTS,
    marcopolo::MARCOPOLO_STEP_ACCOUNTS,
    marinade_finance::MARINADE_FINANCE_STEP_ACCOUNTS,
    mercurial_stable_swap::MERCURIAL_STABLE_SWAP_MIN_ACCOUNTS,
    metadao_futarchy::METADAO_FUTARCHY_STEP_ACCOUNTS,
    meteora_damm_v1::METEORA_DAMM_V1_MIN_ACCOUNTS,
    meteora_damm_v2::METEORA_DAMM_V2_MIN_ACCOUNTS,
    meteora_dbc::METEORA_DBC_MIN_ACCOUNTS,
    meteora_dlmm::{METEORA_DLMM_FIXED_STEP_ACCOUNTS, METEORA_DLMM_MIN_ACCOUNTS},
    metric::METRIC_STEP_ACCOUNTS,
    moonit::MOONIT_STEP_ACCOUNTS,
    obric_v2::OBRIC_V2_MIN_ACCOUNTS,
    omnipair::OMNIPAIR_STEP_ACCOUNTS,
    one_dex::ONE_DEX_STEP_ACCOUNTS,
    openbook_v2::OPENBOOK_V2_MIN_ACCOUNTS,
    orca_token_swap::ORCA_TOKEN_SWAP_MIN_ACCOUNTS,
    orca_whirlpool::ORCA_WHIRLPOOL_MIN_ACCOUNTS,
    phoenix::PHOENIX_MIN_ACCOUNTS,
    pumpfun_amm::PUMPFUN_AMM_MIN_ACCOUNTS,
    raydium_clmm::RAYDIUM_CLMM_MIN_ACCOUNTS,
    raydium_cpmm::RAYDIUM_CPMM_MIN_ACCOUNTS,
    raydium_launchpad::RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
    raydium_pool_v4::RAYDIUM_POOL_V4_MIN_ACCOUNTS,
    raydium_stable_swap::RAYDIUM_STABLE_SWAP_MIN_ACCOUNTS,
    riptide::RIPTIDE_STEP_ACCOUNTS,
    saber_add_decimals::SABER_ADD_DECIMALS_STEP_ACCOUNTS,
    sanctum_infinity::SANCTUM_INFINITY_STEP_ACCOUNTS,
    sanctum_router::SANCTUM_ROUTER_STEP_ACCOUNTS,
    saros_dlmm::SAROS_DLMM_STEP_ACCOUNTS,
    scale_amm::{SCALE_AMM_BASE_STEP_ACCOUNTS, SCALE_AMM_MAX_STEP_ACCOUNTS},
    scale_vmm::{SCALE_VMM_BASE_STEP_ACCOUNTS, SCALE_VMM_MAX_STEP_ACCOUNTS},
    scorch::SCORCH_STEP_ACCOUNTS,
    serum_v3::SERUM_V3_STEP_ACCOUNTS,
    solayer_endoavs::SOLAYER_ENDOAVS_STEP_ACCOUNTS,
    solfi_v1::SOLFI_V1_MIN_ACCOUNTS,
    solfi_v2::SOLFI_V2_MIN_ACCOUNTS,
    stabble_swap::STABBLE_SWAP_MIN_ACCOUNTS,
    taurusfi::TAURUSFI_STEP_ACCOUNTS,
    tessera::TESSERA_MIN_ACCOUNTS,
    trends::TRENDS_STEP_ACCOUNTS,
    vault_liquid_unstake::VAULT_LIQUID_UNSTAKE_STEP_ACCOUNTS,
    vertigo::VERTIGO_STEP_ACCOUNTS,
    voltr::VOLTR_STEP_ACCOUNTS,
    wavebreak::WAVEBREAK_STEP_ACCOUNTS,
    whalestreet::WHALESTREET_MIN_ACCOUNTS,
    woofi_swap::WOOFI_SWAP_MIN_ACCOUNTS,
    xorca::XORCA_STEP_ACCOUNTS,
};
use crate::state::{Protocol, SwapArbParams, REMAINING_ACCOUNTS_FIXED_PREFIX_LEN};

pub const FIXED_REMAINING_ACCOUNTS: usize = REMAINING_ACCOUNTS_FIXED_PREFIX_LEN;

pub struct RouteAccounts<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
    pub associated_token_program: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub token_2022_program: &'a AccountInfo<'info>,
    pub mints: &'a [AccountInfo<'info>],
    pub user_accounts: &'a [AccountInfo<'info>],
    pub step_ranges: Vec<Range<usize>>,
}

pub fn parse_route_accounts<'a, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    params: &SwapArbParams,
) -> Result<RouteAccounts<'a, 'info>> {
    let mints_count = mints_count_to_usize(params.mints_count);
    let step_ranges =
        calculate_step_ranges(remaining_accounts.len(), params.mints_count, &params.steps)?;
    require!(params.input_amount > 0, ArbitrageError::InvalidAmount);
    validate_fixed_programs(remaining_accounts)?;
    validate_route_account_flags(remaining_accounts, mints_count)?;
    let mints_start = FIXED_REMAINING_ACCOUNTS;
    let user_accounts_start = mints_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;
    let user_accounts_end = user_accounts_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;

    Ok(RouteAccounts {
        payer: &remaining_accounts[0],
        system_program: &remaining_accounts[1],
        associated_token_program: &remaining_accounts[2],
        token_program: &remaining_accounts[3],
        token_2022_program: &remaining_accounts[4],
        mints: &remaining_accounts[mints_start..user_accounts_start],
        user_accounts: &remaining_accounts[user_accounts_start..user_accounts_end],
        step_ranges,
    })
}

fn validate_route_account_flags<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    mints_count: usize,
) -> Result<()> {
    let payer = &remaining_accounts[0];
    require!(payer.is_signer, ArbitrageError::InvalidAccount);
    require!(payer.is_writable, ArbitrageError::InvalidAccount);

    for account in &remaining_accounts[1..FIXED_REMAINING_ACCOUNTS] {
        require!(account.executable, ArbitrageError::InvalidAccount);
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.is_writable, ArbitrageError::InvalidAccount);
    }

    let mints_start = FIXED_REMAINING_ACCOUNTS;
    let user_accounts_start = mints_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;
    let user_accounts_end = user_accounts_start
        .checked_add(mints_count)
        .ok_or(ArbitrageError::MathOverflow)?;

    for mint in &remaining_accounts[mints_start..user_accounts_start] {
        require!(!mint.is_signer, ArbitrageError::InvalidAccount);
        require!(!mint.executable, ArbitrageError::InvalidAccount);
    }

    for user_account in &remaining_accounts[user_accounts_start..user_accounts_end] {
        require!(!user_account.is_signer, ArbitrageError::InvalidAccount);
        require!(user_account.is_writable, ArbitrageError::InvalidAccount);
    }

    Ok(())
}

fn validate_fixed_programs<'info>(remaining_accounts: &[AccountInfo<'info>]) -> Result<()> {
    require_keys_eq!(
        remaining_accounts[1].key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        remaining_accounts[2].key(),
        associated_token::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        remaining_accounts[3].key(),
        token::ID,
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        remaining_accounts[4].key(),
        token_2022::ID,
        ArbitrageError::InvalidAccount
    );
    Ok(())
}

pub fn validate_step_account_flags<'info>(
    protocol: Protocol,
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    match protocol {
        Protocol::AldrinV1 => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::aldrin_v1::ALDRIN_V1_MIN_ACCOUNTS,
            &[0],
            &[3, 4, 5, 6],
        ),
        Protocol::AldrinV2 => validate_fixed_len_step_account_flags(
            step_accounts,
            ALDRIN_V2_MIN_ACCOUNTS,
            &[0],
            &[3, 4, 5, 6],
        ),
        Protocol::RaydiumCPMM => validate_fixed_len_step_account_flags(
            step_accounts,
            RAYDIUM_CPMM_MIN_ACCOUNTS,
            &[0],
            &[3, 4, 5, 6],
        ),
        Protocol::RaydiumCLMM => validate_variable_len_step_account_flags(
            step_accounts,
            RAYDIUM_CLMM_MIN_ACCOUNTS,
            7,
            &[0, 6],
            &[2, 3, 4, 5],
        ),
        Protocol::ByrealCLMM => validate_variable_len_step_account_flags(
            step_accounts,
            RAYDIUM_CLMM_MIN_ACCOUNTS,
            7,
            &[0, 6],
            &[2, 3, 4, 5],
        ),
        Protocol::PancakeSwap => validate_variable_len_step_account_flags(
            step_accounts,
            RAYDIUM_CLMM_MIN_ACCOUNTS,
            7,
            &[0, 6],
            &[2, 3, 4, 5],
        ),
        Protocol::StabbleCLMM => validate_variable_len_step_account_flags(
            step_accounts,
            RAYDIUM_CLMM_MIN_ACCOUNTS,
            7,
            &[0, 6],
            &[2, 3, 4, 5],
        ),
        Protocol::StabbleStableSwap | Protocol::StabbleWeightedSwap => {
            validate_fixed_len_step_account_flags(
                step_accounts,
                STABBLE_SWAP_MIN_ACCOUNTS,
                &[0, 10, 11],
                &[3, 4, 5, 6],
            )
        }
        Protocol::GammaSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            GAMMA_SWAP_MIN_ACCOUNTS,
            &[0, 8, 9],
            &[3, 6, 7, 10],
        ),
        Protocol::RaydiumPoolV4 => validate_fixed_len_step_account_flags(
            step_accounts,
            RAYDIUM_POOL_V4_MIN_ACCOUNTS,
            &[0, 7],
            &[1, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13],
        ),
        Protocol::RaydiumStableSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            RAYDIUM_STABLE_SWAP_MIN_ACCOUNTS,
            &[0, 7],
            &[1, 3, 4, 5, 8, 9, 10, 11, 12, 13],
        ),
        Protocol::SolFiV2 => validate_fixed_len_step_account_flags(
            step_accounts,
            SOLFI_V2_MIN_ACCOUNTS,
            &[0, 8, 9],
            &[1, 4, 5],
        ),
        Protocol::SolFiV1 => validate_fixed_len_step_account_flags(
            step_accounts,
            SOLFI_V1_MIN_ACCOUNTS,
            &[0, 4],
            &[1, 2, 3],
        ),
        Protocol::FluxBeam | Protocol::Dexlab => validate_fixed_len_step_account_flags(
            step_accounts,
            FLUXBEAM_MIN_ACCOUNTS,
            &[0, 9, 10, 11],
            &[3, 4, 5, 6],
        ),
        Protocol::HumidiFi => validate_fixed_len_step_account_flags(
            step_accounts,
            HUMIDIFI_MIN_ACCOUNTS,
            &[0, 6, 7],
            &[1, 2, 3],
        ),
        Protocol::ObricV2 => validate_fixed_len_step_account_flags(
            step_accounts,
            OBRIC_V2_MIN_ACCOUNTS,
            &[0, 9],
            &[1, 4, 5, 6],
        ),
        Protocol::Tessera => validate_fixed_len_step_account_flags(
            step_accounts,
            TESSERA_MIN_ACCOUNTS,
            &[0, 7, 8],
            &[2, 3, 4],
        ),
        Protocol::GoonFi => validate_fixed_len_step_account_flags(
            step_accounts,
            GOONFI_MIN_ACCOUNTS,
            &[0, 6],
            &[1, 2, 3],
        ),
        Protocol::GoonFiV2 => validate_fixed_len_step_account_flags(
            step_accounts,
            GOONFI_V2_MIN_ACCOUNTS,
            &[0, 7, 8],
            &[1, 2, 3],
        ),
        Protocol::WhaleStreet => validate_fixed_len_step_account_flags(
            step_accounts,
            WHALESTREET_MIN_ACCOUNTS,
            &[0, 4],
            &[1, 2, 3],
        ),
        Protocol::BinaryFi => validate_fixed_len_step_account_flags(
            step_accounts,
            BINARYFI_MIN_ACCOUNTS,
            &[0, 6, 7],
            &[2, 4, 5],
        ),
        Protocol::XOrca => validate_fixed_len_step_account_flags(
            step_accounts,
            XORCA_STEP_ACCOUNTS,
            &[0, 4],
            &[2, 3],
        ),
        Protocol::Kipseli => validate_fixed_len_step_account_flags(
            step_accounts,
            KIPSELI_STEP_ACCOUNTS,
            &[0],
            &[1, 2, 3],
        ),
        Protocol::Riptide => validate_fixed_len_step_account_flags(
            step_accounts,
            RIPTIDE_STEP_ACCOUNTS,
            &[0, 4],
            &[1, 2, 3],
        ),
        Protocol::Metric => validate_fixed_len_step_account_flags(
            step_accounts,
            METRIC_STEP_ACCOUNTS,
            &[0, 4, 5],
            &[1, 2, 3],
        ),
        Protocol::TaurusFi => validate_fixed_len_step_account_flags(
            step_accounts,
            TAURUSFI_STEP_ACCOUNTS,
            &[0],
            &[2, 3, 4],
        ),
        Protocol::Scorch => validate_fixed_len_step_account_flags(
            step_accounts,
            SCORCH_STEP_ACCOUNTS,
            &[0, 1, 9],
            &[2, 3, 4, 7, 8],
        ),
        Protocol::Aquifer => validate_fixed_len_step_account_flags(
            step_accounts,
            AQUIFER_STEP_ACCOUNTS,
            &[0],
            &[1, 2, 5, 6, 7, 8],
        ),
        Protocol::VaultLiquidUnstake => validate_fixed_len_step_account_flags(
            step_accounts,
            VAULT_LIQUID_UNSTAKE_STEP_ACCOUNTS,
            &[0, 10, 11, 12],
            &[1, 2, 3, 4, 5, 6, 7],
        ),
        Protocol::Wavebreak => validate_fixed_len_step_account_flags(
            step_accounts,
            WAVEBREAK_STEP_ACCOUNTS,
            &[0, 3, 4, 5, 6],
            &[1, 2, 7],
        ),
        Protocol::Vertigo => validate_fixed_len_step_account_flags(
            step_accounts,
            VERTIGO_STEP_ACCOUNTS,
            &[0, 5, 6],
            &[1, 3, 4],
        ),
        Protocol::MarcoPolo => validate_fixed_len_step_account_flags(
            step_accounts,
            MARCOPOLO_STEP_ACCOUNTS,
            &[0, 11, 12, 13],
            &[2, 5, 6, 7, 8, 9],
        ),
        Protocol::MarinadeFinance => validate_fixed_len_step_account_flags(
            step_accounts,
            MARINADE_FINANCE_STEP_ACCOUNTS,
            &[0, 5, 8, 9, 10],
            &[1, 2, 3, 4, 6, 7],
        ),
        Protocol::RaydiumLaunchPad => validate_fixed_len_step_account_flags(
            step_accounts,
            RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
            &[0, 8],
            &[4, 5, 6, 9, 10],
        ),
        Protocol::PumpFunSwap => validate_pumpfun_swap_step_account_flags(step_accounts),
        Protocol::PumpFunAMM => validate_pumpfun_amm_step_account_flags(step_accounts),
        Protocol::OrcaWhirlpool => validate_orca_whirlpool_step_account_flags(step_accounts),
        Protocol::Cropper => validate_fixed_len_step_account_flags(
            step_accounts,
            CROPPER_STEP_ACCOUNTS,
            &[0],
            &[1, 2, 3, 4, 5, 6],
        ),
        Protocol::OrcaTokenSwapV2
        | Protocol::OrcaTokenSwapV1
        | Protocol::SarosSwap
        | Protocol::SplTokenSwap
        | Protocol::DooarSwap
        | Protocol::PenguinSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            ORCA_TOKEN_SWAP_MIN_ACCOUNTS,
            &[0],
            &[3, 4, 5, 6],
        ),
        Protocol::SenchaSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::sencha_swap::SENCHA_SWAP_MIN_ACCOUNTS,
            &[0],
            &[1, 2, 3, 4, 5],
        ),
        Protocol::SaberStableSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::saber_stable_swap::SABER_STABLE_SWAP_MIN_ACCOUNTS,
            &[0],
            &[3, 4, 5, 6],
        ),
        Protocol::MercurialStableSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            MERCURIAL_STABLE_SWAP_MIN_ACCOUNTS,
            &[0],
            &[3, 4],
        ),
        Protocol::Invariant => validate_fixed_len_step_account_flags(
            step_accounts,
            INVARIANT_MIN_ACCOUNTS,
            &[0],
            &[2, 3, 4, 5],
        ),
        Protocol::CremaClmm => validate_crema_clmm_step_account_flags(step_accounts),
        Protocol::SanctumRouter => validate_sanctum_router_step_account_flags(step_accounts),
        Protocol::SanctumInfinity => validate_sanctum_infinity_step_account_flags(step_accounts),
        Protocol::Moonit => validate_fixed_len_step_account_flags(
            step_accounts,
            MOONIT_STEP_ACCOUNTS,
            &[0, 6, 7, 8],
            &[1, 2, 3, 4],
        ),
        Protocol::BoopFun => validate_fixed_len_step_account_flags(
            step_accounts,
            BOOP_FUN_STEP_ACCOUNTS,
            &[0, 7, 8, 9],
            &[1, 2, 3, 4],
        ),
        Protocol::Heaven => validate_fixed_len_step_account_flags(
            step_accounts,
            HEAVEN_STEP_ACCOUNTS,
            &[0, 1, 2, 3, 4, 12],
            &[5, 8, 9, 10],
        ),
        Protocol::SarosDlmm => validate_fixed_len_step_account_flags(
            step_accounts,
            SAROS_DLMM_STEP_ACCOUNTS,
            &[0, 2, 3, 8, 9, 10, 12, 13],
            &[1, 4, 5, 6, 7, 11],
        ),
        Protocol::PerenaNumeraire => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::perena_numeraire::PERENA_NUMERAIRE_STEP_ACCOUNTS,
            &[0, 7, 8],
            &[1, 2, 3, 4, 5],
        ),
        Protocol::PerenaStar => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::perena_star::PERENA_STAR_STEP_ACCOUNTS,
            &[0, 7],
            &[1, 2, 4, 5, 6, 8, 9, 10, 12],
        ),
        Protocol::MetaDaoFutarchy => validate_fixed_len_step_account_flags(
            step_accounts,
            METADAO_FUTARCHY_STEP_ACCOUNTS,
            &[0],
            &[1, 2, 3],
        ),
        Protocol::Gavel => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::gavel::GAVEL_STEP_ACCOUNTS,
            &[0],
            &[2, 3, 4],
        ),
        Protocol::Omnipair => validate_fixed_len_step_account_flags(
            step_accounts,
            OMNIPAIR_STEP_ACCOUNTS,
            &[0, 3, 6],
            &[1, 2, 4, 5],
        ),
        Protocol::ScaleAmm => validate_scale_amm_step_account_flags(step_accounts),
        Protocol::ScaleVmm => validate_scale_vmm_step_account_flags(step_accounts),
        Protocol::Virtuals => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::virtuals::VIRTUALS_STEP_ACCOUNTS,
            &[0, 2],
            &[1, 3, 4, 5, 6],
        ),
        Protocol::Trends => validate_fixed_len_step_account_flags(
            step_accounts,
            TRENDS_STEP_ACCOUNTS,
            &[0],
            &[3, 4, 5],
        ),
        Protocol::FusionAmm => validate_fixed_len_step_account_flags(
            step_accounts,
            crate::protocal::fusionamm::FUSIONAMM_STEP_ACCOUNTS,
            &[0, 1, 2, 3],
            &[4, 5, 6, 9, 10, 11, 12, 13],
        ),
        Protocol::Deriverse => validate_fixed_len_step_account_flags(
            step_accounts,
            DERIVERSE_STEP_ACCOUNTS,
            &[0, 11, 12],
            &[3, 4, 5, 6, 7, 8, 9, 10],
        ),
        Protocol::Carrot => validate_carrot_step_account_flags(step_accounts),
        Protocol::HyloExchange => validate_fixed_len_step_account_flags(
            step_accounts,
            HYLO_EXCHANGE_STEP_ACCOUNTS,
            &[0],
            &[1, 5, 6],
        ),
        Protocol::MSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            M_SWAP_STEP_ACCOUNTS,
            &[0, 12, 13],
            &[2, 3, 5, 10, 11],
        ),
        Protocol::SerumV3 => validate_fixed_len_step_account_flags(
            step_accounts,
            SERUM_V3_STEP_ACCOUNTS,
            &[0, 9],
            &[1, 2, 3, 4, 5, 6, 7],
        ),
        Protocol::BonkSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            BONK_SWAP_MIN_ACCOUNTS,
            &[0],
            &[2, 3, 4, 5, 6, 7],
        ),
        Protocol::Guacswap => validate_fixed_len_step_account_flags(
            step_accounts,
            GUACSWAP_MIN_ACCOUNTS,
            &[0],
            &[2, 3, 4, 5, 6, 7],
        ),
        Protocol::LemmingsFi => validate_fixed_len_step_account_flags(
            step_accounts,
            LEMMINGSFI_STEP_ACCOUNTS,
            &[0, 7],
            &[2, 3, 4],
        ),
        Protocol::Hadron => validate_hadron_step_account_flags(step_accounts),
        Protocol::BisonFi => validate_fixed_len_step_account_flags(
            step_accounts,
            BISONFI_STEP_ACCOUNTS,
            &[0, 6, 7],
            &[1, 2, 3],
        ),
        Protocol::Voltr => validate_fixed_len_step_account_flags(
            step_accounts,
            VOLTR_STEP_ACCOUNTS,
            &[0, 8, 9, 10],
            &[2, 4, 5, 6],
        ),
        Protocol::OneDex => validate_fixed_len_step_account_flags(
            step_accounts,
            ONE_DEX_STEP_ACCOUNTS,
            &[0, 7],
            &[2, 4, 5, 6],
        ),
        Protocol::Huma => validate_fixed_len_step_account_flags(
            step_accounts,
            HUMA_STEP_ACCOUNTS,
            &[0, 13, 14],
            &[3, 5, 7, 8, 10, 11, 12],
        ),
        Protocol::SolayerEndoAvs => validate_fixed_len_step_account_flags(
            step_accounts,
            SOLAYER_ENDOAVS_STEP_ACCOUNTS,
            &[0, 5],
            &[2, 3],
        ),
        Protocol::HyloEarnPool => validate_fixed_len_step_account_flags(
            step_accounts,
            HYLO_EARN_POOL_STEP_ACCOUNTS,
            &[0, 10],
            &[1, 5, 7, 9],
        ),
        Protocol::JupiterLendEarn => validate_fixed_len_step_account_flags(
            step_accounts,
            JUPITER_LEND_EARN_STEP_ACCOUNTS,
            &[0, 11, 13, 14, 15],
            &[2, 4, 5, 6, 8, 9, 10],
        ),
        Protocol::HeliumTreasuryManagement => validate_fixed_len_step_account_flags(
            step_accounts,
            HELIUM_TREASURY_MANAGEMENT_STEP_ACCOUNTS,
            &[0, 6, 7],
            &[3, 4, 5],
        ),
        Protocol::SaberAddDecimals => validate_fixed_len_step_account_flags(
            step_accounts,
            SABER_ADD_DECIMALS_STEP_ACCOUNTS,
            &[0, 5],
            &[2, 3],
        ),
        Protocol::Manifest => validate_fixed_len_step_account_flags(
            step_accounts,
            MANIFEST_MIN_ACCOUNTS,
            &[0, 6, 7],
            &[1, 4, 5],
        ),
        Protocol::OpenBookV2 => validate_fixed_len_step_account_flags(
            step_accounts,
            OPENBOOK_V2_MIN_ACCOUNTS,
            &[0, 8, 9],
            &[1, 3, 4, 5, 6, 7],
        ),
        Protocol::Phoenix => validate_fixed_len_step_account_flags(
            step_accounts,
            PHOENIX_MIN_ACCOUNTS,
            &[0, 7],
            &[2, 5, 6],
        ),
        Protocol::LifinityAmmV2 => validate_fixed_len_step_account_flags(
            step_accounts,
            LIFINITY_AMM_V2_MIN_ACCOUNTS,
            &[0, 9],
            &[2, 5, 6, 7, 8],
        ),
        Protocol::LifinityAmmV1 => validate_fixed_len_step_account_flags(
            step_accounts,
            LIFINITY_AMM_V1_MIN_ACCOUNTS,
            &[0, 9],
            &[5, 6, 7, 8, 12],
        ),
        Protocol::MeteoraDammV2 => validate_fixed_len_step_account_flags(
            step_accounts,
            METEORA_DAMM_V2_MIN_ACCOUNTS,
            &[0],
            &[2, 3, 4],
        ),
        Protocol::MeteoraDammV1 => validate_fixed_len_step_account_flags(
            step_accounts,
            METEORA_DAMM_V1_MIN_ACCOUNTS,
            &[0, 12],
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ),
        Protocol::MeteoraDbc => validate_fixed_len_step_account_flags(
            step_accounts,
            METEORA_DBC_MIN_ACCOUNTS,
            &[0],
            &[3, 4, 5],
        ),
        Protocol::MeteoraDlmm => validate_meteora_dlmm_step_account_flags(step_accounts),
        Protocol::WoofiSwap => validate_fixed_len_step_account_flags(
            step_accounts,
            WOOFI_SWAP_MIN_ACCOUNTS,
            &[0, 2],
            &[3, 4, 5, 6, 7, 8, 9, 10, 11, 13],
        ),
    }
}

fn validate_fixed_len_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
    expected_len: usize,
    executable_readonly_indices: &[usize],
    writable_indices: &[usize],
) -> Result<()> {
    require!(
        step_accounts.len() == expected_len,
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        expected_len,
        executable_readonly_indices,
        writable_indices,
    )
}

fn validate_orca_whirlpool_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == ORCA_WHIRLPOOL_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let executable_indices = [0, 1, 2, 3];
    let writable_indices = [4, 5, 6, 9, 10, 11, 12];
    for (index, account) in step_accounts.iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        if writable_indices.contains(&index) {
            require!(account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        } else if index == 7 || index == 8 {
            // A prior route step may require the same mint writable. Solana unions
            // duplicate-account privileges across the whole transaction.
            require!(!account.executable, ArbitrageError::InvalidAccount);
        } else {
            require!(!account.is_writable, ArbitrageError::InvalidAccount);
            require!(
                account.executable == executable_indices.contains(&index),
                ArbitrageError::InvalidAccount
            );
        }
    }
    Ok(())
}

fn validate_hadron_step_account_flags<'info>(step_accounts: &[AccountInfo<'info>]) -> Result<()> {
    require!(
        matches!(
            step_accounts.len(),
            HADRON_BASE_STEP_ACCOUNTS | HADRON_SPREAD_STEP_ACCOUNTS
        ),
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        step_accounts.len(),
        &[0],
        &[1, 4, 5, 6, 8, 9, 10],
    )
}

fn validate_carrot_step_account_flags<'info>(step_accounts: &[AccountInfo<'info>]) -> Result<()> {
    require!(
        step_accounts.len() == CARROT_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    for (index, account) in step_accounts.iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        match index {
            0 | 5 | 6 | 7 | 8 => {
                require!(account.executable, ArbitrageError::InvalidAccount);
                require!(!account.is_writable, ArbitrageError::InvalidAccount);
            }
            1 | 2 | 4 => {
                require!(!account.executable, ArbitrageError::InvalidAccount);
                require!(account.is_writable, ArbitrageError::InvalidAccount);
            }
            3 | 9 | 10 | 11 => {
                require!(!account.executable, ArbitrageError::InvalidAccount);
                require!(!account.is_writable, ArbitrageError::InvalidAccount);
            }
            12..=14 => {
                require!(!account.executable, ArbitrageError::InvalidAccount);
                require!(
                    account.is_writable == (account.key() == step_accounts[4].key()),
                    ArbitrageError::InvalidAccount
                );
            }
            _ => return Err(ArbitrageError::InvalidAccountCount.into()),
        }
    }
    Ok(())
}

fn validate_scale_amm_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        (SCALE_AMM_BASE_STEP_ACCOUNTS..=SCALE_AMM_MAX_STEP_ACCOUNTS).contains(&step_accounts.len()),
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        SCALE_AMM_BASE_STEP_ACCOUNTS,
        &[0, 6],
        &[1, 3, 4, 5],
    )?;
    for account in &step_accounts[SCALE_AMM_BASE_STEP_ACCOUNTS..] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(account.is_writable, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }
    Ok(())
}

fn validate_scale_vmm_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        (SCALE_VMM_BASE_STEP_ACCOUNTS..=SCALE_VMM_MAX_STEP_ACCOUNTS).contains(&step_accounts.len()),
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        SCALE_VMM_BASE_STEP_ACCOUNTS,
        &[0, 6],
        &[1, 2, 3, 4, 7, 8, 9],
    )?;
    for account in &step_accounts[SCALE_VMM_BASE_STEP_ACCOUNTS..] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(account.is_writable, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }
    Ok(())
}

fn validate_variable_len_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
    min_len: usize,
    fixed_prefix_len: usize,
    executable_readonly_indices: &[usize],
    writable_indices: &[usize],
) -> Result<()> {
    require!(
        fixed_prefix_len <= min_len,
        ArbitrageError::InvalidAccountCount
    );
    require!(
        step_accounts.len() >= min_len,
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        fixed_prefix_len,
        executable_readonly_indices,
        writable_indices,
    )?;

    for account in &step_accounts[fixed_prefix_len..] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }

    Ok(())
}

fn validate_crema_clmm_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        (CREMA_CLMM_MIN_STEP_ACCOUNTS..=CREMA_CLMM_MAX_STEP_ACCOUNTS)
            .contains(&step_accounts.len()),
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        CREMA_CLMM_FIXED_STEP_ACCOUNTS,
        &[0, 6],
        &[2, 3, 4, 5],
    )?;
    for account in &step_accounts[CREMA_CLMM_FIXED_STEP_ACCOUNTS..] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(account.is_writable, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }
    Ok(())
}

fn validate_sanctum_router_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == SANCTUM_ROUTER_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    for (index, account) in step_accounts.iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        match index {
            0 | 10 | 13 => {
                require!(account.executable, ArbitrageError::InvalidAccount);
                require!(!account.is_writable, ArbitrageError::InvalidAccount);
            }
            1 => {
                require!(account.executable, ArbitrageError::InvalidAccount);
                require!(account.is_writable, ArbitrageError::InvalidAccount);
            }
            2 | 4 | 5 | 6 | 7 | 8 | 9 | 14 => {
                require!(!account.executable, ArbitrageError::InvalidAccount);
                require!(account.is_writable, ArbitrageError::InvalidAccount);
            }
            3 | 11 | 12 => {
                require!(!account.executable, ArbitrageError::InvalidAccount);
                require!(!account.is_writable, ArbitrageError::InvalidAccount);
            }
            _ => return Err(ArbitrageError::InvalidAccountCount.into()),
        }
    }
    Ok(())
}

fn validate_sanctum_infinity_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == SANCTUM_INFINITY_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let wsol_calculator = anchor_lang::pubkey!("wsoGmxQLSvwWpuaidCApxN5kEowLe2HLQLJhCQnj4bE");
    let executable_indices: &[usize] = if step_accounts[5].key() == wsol_calculator {
        &[0, 5, 6, 9, 11]
    } else {
        &[0, 5, 8, 10, 11]
    };
    validate_step_account_flags_by_index(
        step_accounts,
        SANCTUM_INFINITY_STEP_ACCOUNTS,
        executable_indices,
        &[1, 2, 3, 4],
    )
}

fn validate_step_account_flags_by_index<'info>(
    step_accounts: &[AccountInfo<'info>],
    checked_len: usize,
    executable_readonly_indices: &[usize],
    writable_indices: &[usize],
) -> Result<()> {
    for (index, account) in step_accounts[..checked_len].iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        if writable_indices.contains(&index) {
            require!(account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        } else {
            require!(!account.is_writable, ArbitrageError::InvalidAccount);
            require!(
                account.executable == executable_readonly_indices.contains(&index),
                ArbitrageError::InvalidAccount
            );
        }
    }
    Ok(())
}

fn validate_pumpfun_swap_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == 17 || step_accounts.len() == 18,
        ArbitrageError::InvalidAccountCount
    );
    require!(step_accounts[0].executable, ArbitrageError::InvalidAccount);
    require!(
        !step_accounts[0].is_writable,
        ArbitrageError::InvalidAccount
    );
    let fee_program_index = step_accounts.len() - 1;
    require!(
        step_accounts[fee_program_index].executable,
        ArbitrageError::InvalidAccount
    );
    require!(
        !step_accounts[fee_program_index].is_writable,
        ArbitrageError::InvalidAccount
    );

    let writable_indices: &[usize] = if step_accounts.len() == 18 {
        &[2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 14]
    } else {
        &[2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13]
    };
    for (index, account) in step_accounts.iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        if index == 0 || index == fee_program_index {
            continue;
        }
        if writable_indices.contains(&index) {
            require!(account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        } else {
            require!(!account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        }
    }
    Ok(())
}

fn validate_pumpfun_amm_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() == 12 || step_accounts.len() == 14 || step_accounts.len() == 16,
        ArbitrageError::InvalidAccountCount
    );
    validate_step_account_flags_by_index(
        step_accounts,
        PUMPFUN_AMM_MIN_ACCOUNTS,
        &[0],
        &[1, 3, 4, 6, 8],
    )?;
    validate_pumpfun_dynamic_suffix_flags(step_accounts, PUMPFUN_AMM_MIN_ACCOUNTS)
}

fn validate_meteora_dlmm_step_account_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    require!(
        step_accounts.len() >= METEORA_DLMM_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );

    for (index, account) in step_accounts.iter().enumerate() {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        if index < METEORA_DLMM_FIXED_STEP_ACCOUNTS {
            match index {
                0..=3 => {
                    require!(!account.is_writable, ArbitrageError::InvalidAccount);
                    require!(account.executable, ArbitrageError::InvalidAccount);
                }
                4 | 5 | 6 | 9 => {
                    require!(account.is_writable, ArbitrageError::InvalidAccount);
                    require!(!account.executable, ArbitrageError::InvalidAccount);
                }
                7 | 8 | 11 => {
                    require!(!account.is_writable, ArbitrageError::InvalidAccount);
                    require!(!account.executable, ArbitrageError::InvalidAccount);
                }
                10 => {
                    if account.executable {
                        require!(!account.is_writable, ArbitrageError::InvalidAccount);
                    } else {
                        require!(account.is_writable, ArbitrageError::InvalidAccount);
                    }
                }
                _ => return Err(ArbitrageError::InvalidAccount.into()),
            }
        } else {
            require!(account.is_writable, ArbitrageError::InvalidAccount);
            require!(!account.executable, ArbitrageError::InvalidAccount);
        }
    }
    Ok(())
}

fn validate_pumpfun_dynamic_suffix_flags<'info>(
    step_accounts: &[AccountInfo<'info>],
    suffix_start: usize,
) -> Result<()> {
    let tail_len = step_accounts
        .len()
        .checked_sub(suffix_start)
        .ok_or(ArbitrageError::InvalidAccountCount)?;
    require!(
        tail_len == 2 || tail_len == 4 || tail_len == 6,
        ArbitrageError::InvalidAccountCount
    );
    let fee_config_index =
        pumpfun_dynamic_suffix_fee_config_index(step_accounts, suffix_start, tail_len)?;

    for account in &step_accounts[suffix_start..fee_config_index] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }
    for account in &step_accounts[fee_config_index + 2..] {
        require!(!account.is_signer, ArbitrageError::InvalidAccount);
        require!(!account.executable, ArbitrageError::InvalidAccount);
    }

    let fee_config = &step_accounts[fee_config_index];
    require!(!fee_config.is_signer, ArbitrageError::InvalidAccount);
    require!(!fee_config.is_writable, ArbitrageError::InvalidAccount);
    require!(!fee_config.executable, ArbitrageError::InvalidAccount);

    let fee_program = &step_accounts[fee_config_index + 1];
    require!(!fee_program.is_signer, ArbitrageError::InvalidAccount);
    require!(!fee_program.is_writable, ArbitrageError::InvalidAccount);
    require!(fee_program.executable, ArbitrageError::InvalidAccount);

    Ok(())
}

fn pumpfun_dynamic_suffix_fee_config_index<'info>(
    step_accounts: &[AccountInfo<'info>],
    suffix_start: usize,
    tail_len: usize,
) -> Result<usize> {
    match tail_len {
        2 => Ok(suffix_start),
        4 => {
            let buyback_only_fee_program_index = suffix_start
                .checked_add(1)
                .ok_or(ArbitrageError::InvalidAccountCount)?;
            if step_accounts[buyback_only_fee_program_index].executable {
                Ok(suffix_start)
            } else {
                suffix_start
                    .checked_add(2)
                    .ok_or(ArbitrageError::InvalidAccountCount.into())
            }
        }
        6 => suffix_start
            .checked_add(2)
            .ok_or(ArbitrageError::InvalidAccountCount.into()),
        _ => Err(ArbitrageError::InvalidAccountCount.into()),
    }
}

pub fn calculate_step_ranges(
    remaining_accounts_len: usize,
    mints_count: u8,
    steps: &[crate::state::SwapStepMeta],
) -> Result<Vec<Range<usize>>> {
    require!(mints_count > 1, ArbitrageError::InvalidAccountCount);
    let m = mints_count_to_usize(mints_count);
    require!(steps.len() == m, ArbitrageError::InvalidAccountCount);

    for step in steps {
        step.validate()?;
    }

    let base_len = FIXED_REMAINING_ACCOUNTS
        .checked_add(m.checked_mul(2).ok_or(ArbitrageError::MathOverflow)?)
        .ok_or(ArbitrageError::MathOverflow)?;
    require!(
        remaining_accounts_len >= base_len,
        ArbitrageError::InvalidAccountCount
    );

    let declared_steps_len = steps.iter().try_fold(0usize, |acc, s| {
        acc.checked_add(step_accounts_len_to_usize(s.accounts_len))
            .ok_or(ArbitrageError::MathOverflow)
    })?;
    let actual_steps_len = remaining_accounts_len
        .checked_sub(base_len)
        .ok_or(ArbitrageError::InvalidAccountCount)?;
    require!(
        actual_steps_len == declared_steps_len,
        ArbitrageError::InvalidAccountCount
    );

    let mut cursor = base_len;
    let mut ranges = Vec::with_capacity(steps.len());
    for step in steps {
        let step_len = step_accounts_len_to_usize(step.accounts_len);
        let end = cursor
            .checked_add(step_len)
            .ok_or(ArbitrageError::MathOverflow)?;
        ranges.push(cursor..end);
        cursor = end;
    }

    Ok(ranges)
}

fn mints_count_to_usize(mints_count: u8) -> usize {
    usize::from(mints_count)
}

fn step_accounts_len_to_usize(accounts_len: u8) -> usize {
    usize::from(accounts_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Protocol, SwapStepMeta};

    fn test_account(
        key: Pubkey,
        owner: Pubkey,
        is_signer: bool,
        is_writable: bool,
        executable: bool,
    ) -> AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(owner));
        let lamports = Box::leak(Box::new(0_u64));
        let data = Box::leak(Vec::<u8>::new().into_boxed_slice());
        AccountInfo::new(
            key,
            is_signer,
            is_writable,
            lamports,
            data,
            owner,
            executable,
            0,
        )
    }

    fn step(accounts_len: u8) -> SwapStepMeta {
        SwapStepMeta {
            protocol: Protocol::RaydiumCPMM,
            accounts_len,
            direction: 0,
            fee_rate: 0,
            protocol_payload: [0; 18],
            min_output_amount: 0,
        }
    }

    fn params_for_two_mints() -> SwapArbParams {
        SwapArbParams {
            mints_count: 2,
            input_amount: 1,
            min_profit_lamports: 0,
            steps: vec![step(0), step(0)],
        }
    }

    fn valid_route_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            test_account(
                Pubkey::new_unique(),
                anchor_lang::system_program::ID,
                true,
                true,
                false,
            ),
            test_account(
                anchor_lang::system_program::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(
                associated_token::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(
                token::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(
                token_2022::ID,
                anchor_lang::system_program::ID,
                false,
                false,
                true,
            ),
            test_account(Pubkey::new_unique(), token::ID, false, false, false),
            test_account(Pubkey::new_unique(), token::ID, false, false, false),
            test_account(Pubkey::new_unique(), token::ID, false, true, false),
            test_account(Pubkey::new_unique(), token::ID, false, true, false),
        ]
    }

    #[test]
    fn validate_step_account_flags_accepts_scale_amm_dynamic_beneficiaries() {
        let owner = Pubkey::new_unique();
        let accounts = vec![
            test_account(Pubkey::new_unique(), owner, false, false, true),
            test_account(Pubkey::new_unique(), owner, false, true, false),
            test_account(Pubkey::new_unique(), owner, false, false, false),
            test_account(Pubkey::new_unique(), owner, false, true, false),
            test_account(Pubkey::new_unique(), owner, false, true, false),
            test_account(Pubkey::new_unique(), owner, false, true, false),
            test_account(Pubkey::new_unique(), owner, false, false, true),
            test_account(Pubkey::new_unique(), owner, false, false, false),
            test_account(Pubkey::new_unique(), owner, false, true, false),
        ];

        validate_step_account_flags(Protocol::ScaleAmm, &accounts)
            .expect("Scale AMM dynamic account flags");
    }

    fn step_account(is_writable: bool, executable: bool) -> AccountInfo<'static> {
        test_account(
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            false,
            is_writable,
            executable,
        )
    }

    fn readonly_step_account() -> AccountInfo<'static> {
        step_account(false, false)
    }

    fn writable_step_account() -> AccountInfo<'static> {
        step_account(true, false)
    }

    fn executable_step_account() -> AccountInfo<'static> {
        step_account(false, true)
    }

    fn cpmm_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn clmm_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            writable_step_account(),
        ]
    }

    fn perena_numeraire_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn perena_star_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn metadao_futarchy_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn stabble_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn gamma_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
            writable_step_account(),
        ]
    }

    fn raydium_launchpad_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn raydium_stable_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn solfi_v2_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
            executable_step_account(),
            readonly_step_account(),
        ]
    }

    fn solfi_v1_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
        ]
    }

    fn pumpfun_swap_buy_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
        ]
    }

    fn pumpfun_amm_step_accounts_with_volume() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
        ]
    }

    fn pumpfun_amm_step_accounts_with_buyback_only() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn orca_whirlpool_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn cropper_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn fusionamm_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn deriverse_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn carrot_step_accounts() -> Vec<AccountInfo<'static>> {
        let mut accounts = vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
        ];
        accounts[12].key = accounts[4].key;
        accounts
    }

    fn hylo_exchange_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
        ]
    }

    fn m_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn lemmingsfi_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
        ]
    }

    fn bisonfi_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
            executable_step_account(),
            readonly_step_account(),
        ]
    }

    fn voltr_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn one_dex_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
        ]
    }

    fn huma_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn solayer_endoavs_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
        ]
    }

    fn jupiter_lend_earn_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn helium_treasury_management_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn saber_add_decimals_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            executable_step_account(),
        ]
    }

    fn whalestreet_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
        ]
    }

    fn binaryfi_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
            readonly_step_account(),
        ]
    }

    fn xorca_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
        ]
    }

    fn kipseli_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn riptide_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
        ]
    }

    fn scorch_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
        ]
    }

    fn aquifer_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn wavebreak_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            writable_step_account(),
        ]
    }

    fn vertigo_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn orca_token_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn sencha_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn saber_stable_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn mercurial_stable_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn invariant_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn crema_clmm_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
        ]
    }

    fn sanctum_infinity_step_accounts(wsol_input: bool) -> Vec<AccountInfo<'static>> {
        let wsol_calculator = anchor_lang::pubkey!("wsoGmxQLSvwWpuaidCApxN5kEowLe2HLQLJhCQnj4bE");
        let calculator = |key| {
            test_account(
                key,
                anchor_lang::solana_program::bpf_loader_upgradeable::ID,
                false,
                false,
                true,
            )
        };
        if wsol_input {
            vec![
                executable_step_account(),
                writable_step_account(),
                writable_step_account(),
                writable_step_account(),
                writable_step_account(),
                calculator(wsol_calculator),
                executable_step_account(),
                readonly_step_account(),
                readonly_step_account(),
                executable_step_account(),
                readonly_step_account(),
                executable_step_account(),
                readonly_step_account(),
            ]
        } else {
            vec![
                executable_step_account(),
                writable_step_account(),
                writable_step_account(),
                writable_step_account(),
                writable_step_account(),
                executable_step_account(),
                readonly_step_account(),
                readonly_step_account(),
                executable_step_account(),
                readonly_step_account(),
                calculator(wsol_calculator),
                executable_step_account(),
                readonly_step_account(),
            ]
        }
    }

    fn manifest_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn openbook_v2_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ]
    }

    fn aldrin_v2_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn aldrin_v1_step_accounts() -> Vec<AccountInfo<'static>> {
        let mut accounts = aldrin_v2_step_accounts();
        accounts.pop();
        accounts
    }

    fn phoenix_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
        ]
    }

    fn lifinity_amm_v2_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
        ]
    }

    fn lifinity_amm_v1_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
        ]
    }

    fn meteora_damm_v2_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            readonly_step_account(),
        ]
    }

    fn meteora_dbc_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    fn meteora_dlmm_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            executable_step_account(),
            readonly_step_account(),
            writable_step_account(),
        ]
    }

    fn meteora_damm_v1_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            executable_step_account(),
        ]
    }

    fn woofi_swap_step_accounts() -> Vec<AccountInfo<'static>> {
        vec![
            executable_step_account(),
            readonly_step_account(),
            executable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            writable_step_account(),
            readonly_step_account(),
        ]
    }

    #[test]
    fn calculates_ranges_after_fixed_mints_and_user_accounts() {
        let steps = vec![step(7), step(8), step(10)];
        let ranges = calculate_step_ranges(5 + 2 * 3 + 25, 3, &steps).unwrap();

        assert_eq!(ranges, vec![11..18, 18..26, 26..36]);
    }

    #[test]
    fn rejects_missing_fixed_accounts() {
        let steps = vec![step(7), step(8)];
        let err = calculate_step_ranges(2 * 2 + 15, 2, &steps).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn rejects_mismatched_step_account_lengths() {
        let steps = vec![step(7), step(8)];
        let err = calculate_step_ranges(5 + 2 * 2 + 14, 2, &steps).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn rejects_invalid_direction_values() {
        let mut steps = vec![step(7), step(8)];
        steps[1].direction = 2;

        let err = calculate_step_ranges(5 + 2 * 2 + 15, 2, &steps).unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidInstructionData.into());
    }

    #[test]
    fn parse_route_accounts_accepts_expected_account_flags() {
        let accounts = valid_route_accounts();
        let route_accounts =
            parse_route_accounts(&accounts, &params_for_two_mints()).expect("route accounts");

        assert_eq!(route_accounts.mints.len(), 2);
        assert_eq!(route_accounts.user_accounts.len(), 2);
    }

    #[test]
    fn parse_route_accounts_rejects_non_signer_payer() {
        let mut accounts = valid_route_accounts();
        accounts[0].is_signer = false;

        let err = match parse_route_accounts(&accounts, &params_for_two_mints()) {
            Ok(_) => panic!("payer must sign"),
            Err(err) => err,
        };

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn parse_route_accounts_rejects_readonly_user_token_account() {
        let mut accounts = valid_route_accounts();
        accounts[7].is_writable = false;

        let err = match parse_route_accounts(&accounts, &params_for_two_mints()) {
            Ok(_) => panic!("user token account must be writable"),
            Err(err) => err,
        };

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn parse_route_accounts_accepts_writable_non_executable_mint() {
        let mut accounts = valid_route_accounts();
        accounts[5].is_writable = true;

        assert!(parse_route_accounts(&accounts, &params_for_two_mints()).is_ok());
    }

    #[test]
    fn parse_route_accounts_rejects_executable_mint() {
        let mut accounts = valid_route_accounts();
        accounts[5].executable = true;

        let err = match parse_route_accounts(&accounts, &params_for_two_mints()) {
            Ok(_) => panic!("mint must not be executable"),
            Err(err) => err,
        };

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_expected_cpmm_flags() {
        let accounts = cpmm_step_accounts();

        assert!(validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_rejects_writable_readonly_cpmm_account() {
        let mut accounts = cpmm_step_accounts();
        accounts[1].is_writable = true;

        let err = validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_cpmm_vault() {
        let mut accounts = cpmm_step_accounts();
        accounts[4].is_writable = false;

        let err = validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_extra_fixed_len_cpmm_account() {
        let mut accounts = cpmm_step_accounts();
        accounts.push(readonly_step_account());

        let err = validate_step_account_flags(Protocol::RaydiumCPMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_clmm_dynamic_account_flags() {
        let accounts = clmm_step_accounts();

        assert!(validate_step_account_flags(Protocol::RaydiumCLMM, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_stabble_swap_accounts() {
        let accounts = stabble_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::StabbleStableSwap, &accounts).is_ok());
        assert!(validate_step_account_flags(Protocol::StabbleWeightedSwap, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_gamma_swap_accounts() {
        let accounts = gamma_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::GammaSwap, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_raydium_launchpad_accounts() {
        let accounts = raydium_launchpad_step_accounts();

        assert!(validate_step_account_flags(Protocol::RaydiumLaunchPad, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_raydium_stable_swap_accounts() {
        let accounts = raydium_stable_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::RaydiumStableSwap, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_solfi_v2_accounts() {
        let accounts = solfi_v2_step_accounts();
        assert!(validate_step_account_flags(Protocol::SolFiV2, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_solfi_v1_accounts() {
        let accounts = solfi_v1_step_accounts();
        assert!(validate_step_account_flags(Protocol::SolFiV1, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_rejects_non_executable_launchpad_system_program() {
        let mut accounts = raydium_launchpad_step_accounts();
        accounts[8].executable = false;

        let err = validate_step_account_flags(Protocol::RaydiumLaunchPad, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_launchpad_fee_vault() {
        let mut accounts = raydium_launchpad_step_accounts();
        accounts[9].is_writable = false;

        let err = validate_step_account_flags(Protocol::RaydiumLaunchPad, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_signer_dynamic_account() {
        let mut accounts = clmm_step_accounts();
        accounts[7].is_signer = true;

        let err = validate_step_account_flags(Protocol::RaydiumCLMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_pumpfun_fee_program_suffix() {
        let swap_accounts = pumpfun_swap_buy_step_accounts();
        assert!(validate_step_account_flags(Protocol::PumpFunSwap, &swap_accounts).is_ok());

        let amm_accounts = pumpfun_amm_step_accounts_with_volume();
        assert!(validate_step_account_flags(Protocol::PumpFunAMM, &amm_accounts).is_ok());

        let buyback_only_amm_accounts = pumpfun_amm_step_accounts_with_buyback_only();
        assert!(
            validate_step_account_flags(Protocol::PumpFunAMM, &buyback_only_amm_accounts).is_ok()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_orca_whirlpool_accounts() {
        let accounts = orca_whirlpool_step_accounts();

        assert!(validate_step_account_flags(Protocol::OrcaWhirlpool, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_writable_whirlpool_mints_from_privilege_union() {
        let mut accounts = orca_whirlpool_step_accounts();
        accounts[7].is_writable = true;
        accounts[8].is_writable = true;

        assert!(validate_step_account_flags(Protocol::OrcaWhirlpool, &accounts).is_ok());

        accounts[3].is_writable = true;
        assert!(validate_step_account_flags(Protocol::OrcaWhirlpool, &accounts).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_cropper_accounts() {
        let accounts = cropper_step_accounts();
        assert!(validate_step_account_flags(Protocol::Cropper, &accounts).is_ok());

        let mut readonly_tick = cropper_step_accounts();
        readonly_tick[4].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Cropper, &readonly_tick).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_token_swap_variant_accounts() {
        let accounts = orca_token_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::OrcaTokenSwapV2, &accounts).is_ok());
        assert!(validate_step_account_flags(Protocol::OrcaTokenSwapV1, &accounts).is_ok());
        assert!(validate_step_account_flags(Protocol::SarosSwap, &accounts).is_ok());
        assert!(validate_step_account_flags(Protocol::SplTokenSwap, &accounts).is_ok());
        assert!(validate_step_account_flags(Protocol::DooarSwap, &accounts).is_ok());
        assert!(validate_step_account_flags(Protocol::PenguinSwap, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_extended_token_swap_accounts() {
        let accounts = vec![
            executable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            writable_step_account(),
            readonly_step_account(),
            readonly_step_account(),
            executable_step_account(),
            executable_step_account(),
            executable_step_account(),
        ];

        assert!(validate_step_account_flags(Protocol::FluxBeam, &accounts).is_ok());
        assert!(validate_step_account_flags(Protocol::Dexlab, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_manifest_accounts() {
        let accounts = manifest_step_accounts();

        assert!(validate_step_account_flags(Protocol::Manifest, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_checks_openbook_v2_accounts() {
        let accounts = openbook_v2_step_accounts();
        assert!(validate_step_account_flags(Protocol::OpenBookV2, &accounts).is_ok());

        let mut readonly_bids = openbook_v2_step_accounts();
        readonly_bids[3].is_writable = false;
        assert!(validate_step_account_flags(Protocol::OpenBookV2, &readonly_bids).is_err());

        let mut writable_authority = openbook_v2_step_accounts();
        writable_authority[2].is_writable = true;
        assert!(validate_step_account_flags(Protocol::OpenBookV2, &writable_authority).is_err());
    }

    #[test]
    fn validate_step_account_flags_checks_aldrin_v2_accounts() {
        let accounts = aldrin_v2_step_accounts();
        assert!(validate_step_account_flags(Protocol::AldrinV2, &accounts).is_ok());

        let mut writable_pool = aldrin_v2_step_accounts();
        writable_pool[1].is_writable = true;
        assert!(validate_step_account_flags(Protocol::AldrinV2, &writable_pool).is_err());

        let mut readonly_vault = aldrin_v2_step_accounts();
        readonly_vault[4].is_writable = false;
        assert!(validate_step_account_flags(Protocol::AldrinV2, &readonly_vault).is_err());
    }

    #[test]
    fn validate_step_account_flags_checks_aldrin_v1_accounts() {
        let accounts = aldrin_v1_step_accounts();
        assert!(validate_step_account_flags(Protocol::AldrinV1, &accounts).is_ok());

        let mut writable_pool = aldrin_v1_step_accounts();
        writable_pool[1].is_writable = true;
        assert!(validate_step_account_flags(Protocol::AldrinV1, &writable_pool).is_err());

        let mut readonly_vault = aldrin_v1_step_accounts();
        readonly_vault[4].is_writable = false;
        assert!(validate_step_account_flags(Protocol::AldrinV1, &readonly_vault).is_err());
    }

    #[test]
    fn validate_step_account_flags_checks_perena_numeraire_accounts() {
        let accounts = perena_numeraire_step_accounts();
        assert!(validate_step_account_flags(Protocol::PerenaNumeraire, &accounts).is_ok());

        let mut readonly_pool = perena_numeraire_step_accounts();
        readonly_pool[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::PerenaNumeraire, &readonly_pool).is_err());

        let mut writable_config = perena_numeraire_step_accounts();
        writable_config[6].is_writable = true;
        assert!(validate_step_account_flags(Protocol::PerenaNumeraire, &writable_config).is_err());
    }

    #[test]
    fn validate_step_account_flags_checks_perena_star_accounts() {
        let accounts = perena_star_step_accounts();
        assert!(validate_step_account_flags(Protocol::PerenaStar, &accounts).is_ok());

        let mut readonly_vault = perena_star_step_accounts();
        readonly_vault[2].is_writable = false;
        assert!(validate_step_account_flags(Protocol::PerenaStar, &readonly_vault).is_err());

        let mut executable_oracle = perena_star_step_accounts();
        executable_oracle[3].executable = true;
        assert!(validate_step_account_flags(Protocol::PerenaStar, &executable_oracle).is_err());
    }

    #[test]
    fn validate_step_account_flags_checks_metadao_futarchy_accounts() {
        let accounts = metadao_futarchy_step_accounts();
        assert!(validate_step_account_flags(Protocol::MetaDaoFutarchy, &accounts).is_ok());

        let mut readonly_vault = metadao_futarchy_step_accounts();
        readonly_vault[2].is_writable = false;
        assert!(validate_step_account_flags(Protocol::MetaDaoFutarchy, &readonly_vault).is_err());

        let mut executable_event_authority = metadao_futarchy_step_accounts();
        executable_event_authority[4].executable = true;
        assert!(validate_step_account_flags(
            Protocol::MetaDaoFutarchy,
            &executable_event_authority
        )
        .is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_sencha_swap_accounts() {
        let accounts = sencha_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::SenchaSwap, &accounts).is_ok());

        let mut readonly_pool = sencha_swap_step_accounts();
        readonly_pool[1].is_writable = false;
        let err = validate_step_account_flags(Protocol::SenchaSwap, &readonly_pool).unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_saber_stable_swap_accounts() {
        let accounts = saber_stable_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::SaberStableSwap, &accounts).is_ok());

        let mut writable_pool = saber_stable_swap_step_accounts();
        writable_pool[1].is_writable = true;
        let err =
            validate_step_account_flags(Protocol::SaberStableSwap, &writable_pool).unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_mercurial_stable_swap_accounts() {
        let accounts = mercurial_stable_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::MercurialStableSwap, &accounts).is_ok());

        let mut readonly_reserve = mercurial_stable_swap_step_accounts();
        readonly_reserve[3].is_writable = false;
        let err = validate_step_account_flags(Protocol::MercurialStableSwap, &readonly_reserve)
            .unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_invariant_accounts() {
        let accounts = invariant_step_accounts();
        assert!(validate_step_account_flags(Protocol::Invariant, &accounts).is_ok());

        let mut readonly_tickmap = invariant_step_accounts();
        readonly_tickmap[3].is_writable = false;
        let err = validate_step_account_flags(Protocol::Invariant, &readonly_tickmap).unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_crema_dynamic_tick_arrays() {
        let accounts = crema_clmm_step_accounts();
        assert!(validate_step_account_flags(Protocol::CremaClmm, &accounts).is_ok());

        let mut readonly_tick_array = crema_clmm_step_accounts();
        readonly_tick_array[7].is_writable = false;
        assert!(validate_step_account_flags(Protocol::CremaClmm, &readonly_tick_array).is_err());

        let mut too_many = crema_clmm_step_accounts();
        too_many.push(writable_step_account());
        assert!(validate_step_account_flags(Protocol::CremaClmm, &too_many).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_both_sanctum_infinity_directions() {
        assert!(validate_step_account_flags(
            Protocol::SanctumInfinity,
            &sanctum_infinity_step_accounts(true),
        )
        .is_ok());
        assert!(validate_step_account_flags(
            Protocol::SanctumInfinity,
            &sanctum_infinity_step_accounts(false),
        )
        .is_ok());

        let mut readonly_reserve = sanctum_infinity_step_accounts(true);
        readonly_reserve[3].is_writable = false;
        assert!(validate_step_account_flags(Protocol::SanctumInfinity, &readonly_reserve).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_phoenix_accounts() {
        assert!(validate_step_account_flags(Protocol::Phoenix, &phoenix_step_accounts()).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_lifinity_amm_v2_accounts() {
        assert!(validate_step_account_flags(
            Protocol::LifinityAmmV2,
            &lifinity_amm_v2_step_accounts(),
        )
        .is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_lifinity_amm_v1_accounts() {
        assert!(validate_step_account_flags(
            Protocol::LifinityAmmV1,
            &lifinity_amm_v1_step_accounts(),
        )
        .is_ok());

        let mut writable_amm = lifinity_amm_v1_step_accounts();
        writable_amm[2].is_writable = true;
        assert!(validate_step_account_flags(Protocol::LifinityAmmV1, &writable_amm).is_err());
    }

    #[test]
    fn validate_step_account_flags_rejects_writable_manifest_mint() {
        let mut accounts = manifest_step_accounts();
        accounts[2].is_writable = true;

        let err = validate_step_account_flags(Protocol::Manifest, &accounts).unwrap_err();
        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_orca_token_swap_pool_mint() {
        let mut accounts = orca_token_swap_step_accounts();
        accounts[5].is_writable = false;

        let err = validate_step_account_flags(Protocol::OrcaTokenSwapV2, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_accepts_meteora_damm_v2_accounts() {
        let accounts = meteora_damm_v2_step_accounts();

        assert!(validate_step_account_flags(Protocol::MeteoraDammV2, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_meteora_dbc_accounts() {
        let accounts = meteora_dbc_step_accounts();

        assert!(validate_step_account_flags(Protocol::MeteoraDbc, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_trends_accounts() {
        let accounts = meteora_dbc_step_accounts();

        assert!(validate_step_account_flags(Protocol::Trends, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_fusionamm_accounts() {
        let accounts = fusionamm_step_accounts();
        assert!(validate_step_account_flags(Protocol::FusionAmm, &accounts).is_ok());

        let mut readonly_tick = fusionamm_step_accounts();
        readonly_tick[13].is_writable = false;
        assert!(validate_step_account_flags(Protocol::FusionAmm, &readonly_tick).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_deriverse_accounts() {
        let accounts = deriverse_step_accounts();
        assert!(validate_step_account_flags(Protocol::Deriverse, &accounts).is_ok());

        let mut readonly_market = deriverse_step_accounts();
        readonly_market[5].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Deriverse, &readonly_market).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_carrot_accounts() {
        let accounts = carrot_step_accounts();
        assert!(validate_step_account_flags(Protocol::Carrot, &accounts).is_ok());

        let mut writable_nonselected_reserve = carrot_step_accounts();
        writable_nonselected_reserve[13].is_writable = true;
        assert!(
            validate_step_account_flags(Protocol::Carrot, &writable_nonselected_reserve).is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_hylo_exchange_accounts() {
        let accounts = hylo_exchange_step_accounts();
        assert!(validate_step_account_flags(Protocol::HyloExchange, &accounts).is_ok());

        let mut readonly_vault = hylo_exchange_step_accounts();
        readonly_vault[6].is_writable = false;
        assert!(validate_step_account_flags(Protocol::HyloExchange, &readonly_vault).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_m_swap_accounts() {
        let accounts = m_swap_step_accounts();
        assert!(validate_step_account_flags(Protocol::MSwap, &accounts).is_ok());

        let mut readonly_vault = m_swap_step_accounts();
        readonly_vault[10].is_writable = false;
        assert!(validate_step_account_flags(Protocol::MSwap, &readonly_vault).is_err());

        let mut non_executable_extension = m_swap_step_accounts();
        non_executable_extension[12].executable = false;
        assert!(validate_step_account_flags(Protocol::MSwap, &non_executable_extension).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_lemmingsfi_accounts() {
        let accounts = lemmingsfi_step_accounts();
        assert!(validate_step_account_flags(Protocol::LemmingsFi, &accounts).is_ok());

        let mut readonly_vault = lemmingsfi_step_accounts();
        readonly_vault[3].is_writable = false;
        assert!(validate_step_account_flags(Protocol::LemmingsFi, &readonly_vault).is_err());

        let mut non_executable_token_program = lemmingsfi_step_accounts();
        non_executable_token_program[7].executable = false;
        assert!(
            validate_step_account_flags(Protocol::LemmingsFi, &non_executable_token_program)
                .is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_bisonfi_accounts() {
        let accounts = bisonfi_step_accounts();
        assert!(validate_step_account_flags(Protocol::BisonFi, &accounts).is_ok());

        let mut readonly_market = bisonfi_step_accounts();
        readonly_market[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::BisonFi, &readonly_market).is_err());

        let mut non_executable_token_program = bisonfi_step_accounts();
        non_executable_token_program[6].executable = false;
        assert!(
            validate_step_account_flags(Protocol::BisonFi, &non_executable_token_program).is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_voltr_accounts() {
        let accounts = voltr_step_accounts();
        assert!(validate_step_account_flags(Protocol::Voltr, &accounts).is_ok());

        let mut readonly_idle_vault = voltr_step_accounts();
        readonly_idle_vault[5].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Voltr, &readonly_idle_vault).is_err());

        let mut non_executable_asset_program = voltr_step_accounts();
        non_executable_asset_program[8].executable = false;
        assert!(
            validate_step_account_flags(Protocol::Voltr, &non_executable_asset_program).is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_one_dex_accounts() {
        let accounts = one_dex_step_accounts();
        assert!(validate_step_account_flags(Protocol::OneDex, &accounts).is_ok());

        let mut readonly_fee = one_dex_step_accounts();
        readonly_fee[6].is_writable = false;
        assert!(validate_step_account_flags(Protocol::OneDex, &readonly_fee).is_err());

        let mut non_executable_token_program = one_dex_step_accounts();
        non_executable_token_program[7].executable = false;
        assert!(
            validate_step_account_flags(Protocol::OneDex, &non_executable_token_program).is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_huma_accounts() {
        let accounts = huma_step_accounts();
        assert!(validate_step_account_flags(Protocol::Huma, &accounts).is_ok());

        let mut readonly_pool_state = huma_step_accounts();
        readonly_pool_state[3].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Huma, &readonly_pool_state).is_err());

        let mut non_executable_token_program = huma_step_accounts();
        non_executable_token_program[13].executable = false;
        assert!(
            validate_step_account_flags(Protocol::Huma, &non_executable_token_program).is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_solayer_endoavs_accounts() {
        let accounts = solayer_endoavs_step_accounts();
        assert!(validate_step_account_flags(Protocol::SolayerEndoAvs, &accounts).is_ok());

        let mut readonly_avs_mint = solayer_endoavs_step_accounts();
        readonly_avs_mint[2].is_writable = false;
        assert!(validate_step_account_flags(Protocol::SolayerEndoAvs, &readonly_avs_mint).is_err());

        let mut non_executable_token_program = solayer_endoavs_step_accounts();
        non_executable_token_program[5].executable = false;
        assert!(validate_step_account_flags(
            Protocol::SolayerEndoAvs,
            &non_executable_token_program
        )
        .is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_jupiter_lend_earn_accounts() {
        let accounts = jupiter_lend_earn_step_accounts();
        assert!(validate_step_account_flags(Protocol::JupiterLendEarn, &accounts).is_ok());

        let mut readonly_claim = jupiter_lend_earn_step_accounts();
        readonly_claim[9].is_writable = false;
        assert!(validate_step_account_flags(Protocol::JupiterLendEarn, &readonly_claim).is_err());

        let mut non_executable_liquidity_program = jupiter_lend_earn_step_accounts();
        non_executable_liquidity_program[11].executable = false;
        assert!(validate_step_account_flags(
            Protocol::JupiterLendEarn,
            &non_executable_liquidity_program
        )
        .is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_helium_treasury_management_accounts() {
        let accounts = helium_treasury_management_step_accounts();
        assert!(validate_step_account_flags(Protocol::HeliumTreasuryManagement, &accounts).is_ok());

        let mut readonly_treasury = helium_treasury_management_step_accounts();
        readonly_treasury[4].is_writable = false;
        assert!(validate_step_account_flags(
            Protocol::HeliumTreasuryManagement,
            &readonly_treasury
        )
        .is_err());

        let mut non_executable_breaker_program = helium_treasury_management_step_accounts();
        non_executable_breaker_program[6].executable = false;
        assert!(validate_step_account_flags(
            Protocol::HeliumTreasuryManagement,
            &non_executable_breaker_program,
        )
        .is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_saber_add_decimals_accounts() {
        let accounts = saber_add_decimals_step_accounts();
        assert!(validate_step_account_flags(Protocol::SaberAddDecimals, &accounts).is_ok());

        let mut readonly_wrapped_mint = saber_add_decimals_step_accounts();
        readonly_wrapped_mint[2].is_writable = false;
        assert!(
            validate_step_account_flags(Protocol::SaberAddDecimals, &readonly_wrapped_mint)
                .is_err()
        );

        let mut non_executable_token_program = saber_add_decimals_step_accounts();
        non_executable_token_program[5].executable = false;
        assert!(validate_step_account_flags(
            Protocol::SaberAddDecimals,
            &non_executable_token_program,
        )
        .is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_whalestreet_accounts() {
        let accounts = whalestreet_step_accounts();
        assert!(validate_step_account_flags(Protocol::WhaleStreet, &accounts).is_ok());

        let mut readonly_pool = whalestreet_step_accounts();
        readonly_pool[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::WhaleStreet, &readonly_pool).is_err());

        let mut non_executable_token_program = whalestreet_step_accounts();
        non_executable_token_program[4].executable = false;
        assert!(
            validate_step_account_flags(Protocol::WhaleStreet, &non_executable_token_program,)
                .is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_binaryfi_accounts() {
        let accounts = binaryfi_step_accounts();
        assert!(validate_step_account_flags(Protocol::BinaryFi, &accounts).is_ok());

        let mut readonly_pool = binaryfi_step_accounts();
        readonly_pool[2].is_writable = false;
        assert!(validate_step_account_flags(Protocol::BinaryFi, &readonly_pool).is_err());

        let mut non_executable_output_program = binaryfi_step_accounts();
        non_executable_output_program[7].executable = false;
        assert!(
            validate_step_account_flags(Protocol::BinaryFi, &non_executable_output_program)
                .is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_xorca_accounts() {
        let accounts = xorca_step_accounts();
        assert!(validate_step_account_flags(Protocol::XOrca, &accounts).is_ok());

        let mut readonly_mint = xorca_step_accounts();
        readonly_mint[3].is_writable = false;
        assert!(validate_step_account_flags(Protocol::XOrca, &readonly_mint).is_err());

        let mut non_executable_token_program = xorca_step_accounts();
        non_executable_token_program[4].executable = false;
        assert!(
            validate_step_account_flags(Protocol::XOrca, &non_executable_token_program).is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_kipseli_non_executable_ban_pda() {
        let accounts = kipseli_step_accounts();
        assert!(validate_step_account_flags(Protocol::Kipseli, &accounts).is_ok());

        let mut readonly_pool = kipseli_step_accounts();
        readonly_pool[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Kipseli, &readonly_pool).is_err());

        let mut executable_ban = kipseli_step_accounts();
        executable_ban[4].executable = true;
        assert!(validate_step_account_flags(Protocol::Kipseli, &executable_ban).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_riptide_memo_and_instructions_sysvar() {
        let accounts = riptide_step_accounts();
        assert!(validate_step_account_flags(Protocol::Riptide, &accounts).is_ok());

        let mut readonly_market = riptide_step_accounts();
        readonly_market[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Riptide, &readonly_market).is_err());

        let mut non_executable_memo = riptide_step_accounts();
        non_executable_memo[4].executable = false;
        assert!(validate_step_account_flags(Protocol::Riptide, &non_executable_memo).is_err());

        let mut executable_sysvar = riptide_step_accounts();
        executable_sysvar[5].executable = true;
        assert!(validate_step_account_flags(Protocol::Riptide, &executable_sysvar).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_scorch_programs_oracles_and_sysvar() {
        let accounts = scorch_step_accounts();
        assert!(validate_step_account_flags(Protocol::Scorch, &accounts).is_ok());

        let mut readonly_pool = scorch_step_accounts();
        readonly_pool[2].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Scorch, &readonly_pool).is_err());

        let mut readonly_oracle = scorch_step_accounts();
        readonly_oracle[7].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Scorch, &readonly_oracle).is_err());

        let mut non_executable_memo = scorch_step_accounts();
        non_executable_memo[9].executable = false;
        assert!(validate_step_account_flags(Protocol::Scorch, &non_executable_memo).is_err());

        let mut executable_sysvar = scorch_step_accounts();
        executable_sysvar[10].executable = true;
        assert!(validate_step_account_flags(Protocol::Scorch, &executable_sysvar).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_aquifer_accounts() {
        let accounts = aquifer_step_accounts();
        assert!(validate_step_account_flags(Protocol::Aquifer, &accounts).is_ok());

        let mut readonly_dex = aquifer_step_accounts();
        readonly_dex[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Aquifer, &readonly_dex).is_err());

        let mut writable_oracle = aquifer_step_accounts();
        writable_oracle[3].is_writable = true;
        assert!(validate_step_account_flags(Protocol::Aquifer, &writable_oracle).is_err());

        let mut executable_sysvar = aquifer_step_accounts();
        executable_sysvar[9].executable = true;
        assert!(validate_step_account_flags(Protocol::Aquifer, &executable_sysvar).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_wavebreak_accounts() {
        let accounts = wavebreak_step_accounts();
        assert!(validate_step_account_flags(Protocol::Wavebreak, &accounts).is_ok());

        let mut readonly_curve = wavebreak_step_accounts();
        readonly_curve[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Wavebreak, &readonly_curve).is_err());

        let mut readonly_vault = wavebreak_step_accounts();
        readonly_vault[2].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Wavebreak, &readonly_vault).is_err());

        let mut non_executable_token_program = wavebreak_step_accounts();
        non_executable_token_program[5].executable = false;
        assert!(
            validate_step_account_flags(Protocol::Wavebreak, &non_executable_token_program)
                .is_err()
        );

        let mut readonly_base_mint = wavebreak_step_accounts();
        readonly_base_mint[7].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Wavebreak, &readonly_base_mint).is_err());
    }

    #[test]
    fn validate_step_account_flags_accepts_vertigo_accounts() {
        let accounts = vertigo_step_accounts();
        assert!(validate_step_account_flags(Protocol::Vertigo, &accounts).is_ok());

        let mut readonly_pool = vertigo_step_accounts();
        readonly_pool[1].is_writable = false;
        assert!(validate_step_account_flags(Protocol::Vertigo, &readonly_pool).is_err());

        let mut writable_owner = vertigo_step_accounts();
        writable_owner[2].is_writable = true;
        assert!(validate_step_account_flags(Protocol::Vertigo, &writable_owner).is_err());

        let mut non_executable_token_program = vertigo_step_accounts();
        non_executable_token_program[5].executable = false;
        assert!(
            validate_step_account_flags(Protocol::Vertigo, &non_executable_token_program).is_err()
        );
    }

    #[test]
    fn validate_step_account_flags_accepts_meteora_dlmm_accounts() {
        let accounts = meteora_dlmm_step_accounts();

        assert!(validate_step_account_flags(Protocol::MeteoraDlmm, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_meteora_damm_v1_accounts() {
        let accounts = meteora_damm_v1_step_accounts();

        assert!(validate_step_account_flags(Protocol::MeteoraDammV1, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_accepts_woofi_swap_accounts() {
        let accounts = woofi_swap_step_accounts();

        assert!(validate_step_account_flags(Protocol::WoofiSwap, &accounts).is_ok());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_meteora_damm_v1_vault() {
        let mut accounts = meteora_damm_v1_step_accounts();
        accounts[3].is_writable = false;

        let err = validate_step_account_flags(Protocol::MeteoraDammV1, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_orca_tick_array() {
        let mut accounts = orca_whirlpool_step_accounts();
        accounts[9].is_writable = false;

        let err = validate_step_account_flags(Protocol::OrcaWhirlpool, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_readonly_meteora_dlmm_bin_array() {
        let mut accounts = meteora_dlmm_step_accounts();
        accounts[12].is_writable = false;

        let err = validate_step_account_flags(Protocol::MeteoraDlmm, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_non_executable_pumpfun_fee_program() {
        let mut accounts = pumpfun_swap_buy_step_accounts();
        accounts[17].executable = false;

        let err = validate_step_account_flags(Protocol::PumpFunSwap, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[test]
    fn validate_step_account_flags_rejects_unexpected_pumpfun_tail_length() {
        let mut accounts = pumpfun_amm_step_accounts_with_volume();
        accounts.remove(11);

        let err = validate_step_account_flags(Protocol::PumpFunAMM, &accounts).unwrap_err();

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[test]
    fn route_account_lengths_use_lossless_index_helpers() {
        assert_eq!(mints_count_to_usize(7), 7);
        assert_eq!(step_accounts_len_to_usize(9), 9);

        for (name, source) in [
            ("accounts", include_str!("accounts.rs")),
            ("execute_arbitrage", include_str!("execute_arbitrage.rs")),
        ] {
            let production = production_source_text(source);
            for forbidden in [
                "params.mints_count as usize",
                "mints_count as usize",
                "accounts_len as usize",
                "2 * mints_count",
            ] {
                assert!(
                    !production.contains(forbidden),
                    "{name} route account boundary math must use checked helpers instead of {forbidden}"
                );
            }
        }
    }

    fn production_source_text(source: &str) -> &str {
        match source.split_once("\n#[cfg(test)]") {
            Some((production, _)) => production,
            None => source,
        }
    }
}
