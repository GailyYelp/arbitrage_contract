use anchor_lang::prelude::*;

use crate::errors::ArbitrageError;
use crate::instructions::accounts::{parse_route_accounts, validate_step_account_flags};
use crate::instructions::program_ids::{
    validate_step_fixed_accounts, validate_step_program_account,
};
use crate::instructions::types::{
    token_program_for_mint, validate_aldrin_v1_semantic_accounts,
    validate_aldrin_v2_semantic_accounts, validate_binaryfi_semantic_accounts,
    validate_bonk_swap_semantic_accounts, validate_boop_fun_semantic_accounts,
    validate_crema_clmm_semantic_accounts, validate_fluxbeam_semantic_accounts,
    validate_goonfi_semantic_accounts, validate_goonfi_v2_semantic_accounts,
    validate_heaven_semantic_accounts, validate_humidifi_semantic_accounts,
    validate_invariant_semantic_accounts, validate_lifinity_amm_v1_semantic_accounts,
    validate_lifinity_amm_v2_semantic_accounts, validate_manifest_semantic_accounts,
    validate_mercurial_stable_swap_semantic_accounts, validate_meteora_damm_v1_semantic_accounts,
    validate_meteora_damm_v2_semantic_accounts, validate_meteora_dbc_semantic_accounts,
    validate_meteora_dlmm_semantic_accounts, validate_moonit_semantic_accounts,
    validate_obric_v2_semantic_accounts, validate_openbook_v2_semantic_accounts,
    validate_orca_token_swap_semantic_accounts, validate_orca_whirlpool_semantic_accounts,
    validate_phoenix_semantic_accounts, validate_pumpfun_amm_semantic_accounts,
    validate_pumpfun_swap_semantic_accounts, validate_raydium_clmm_semantic_accounts,
    validate_raydium_cpmm_semantic_accounts, validate_raydium_launchpad_semantic_accounts,
    validate_raydium_pool_v4_authority, validate_raydium_pool_v4_semantic_accounts,
    validate_raydium_stable_swap_authority, validate_raydium_stable_swap_semantic_accounts,
    validate_saber_stable_swap_semantic_accounts, validate_sanctum_router_semantic_accounts,
    validate_saros_dlmm_semantic_accounts, validate_sencha_swap_semantic_accounts,
    validate_solfi_v1_semantic_accounts, validate_solfi_v2_semantic_accounts,
    validate_tessera_semantic_accounts, validate_token_account_for_mint,
    validate_token_account_for_mint_and_authority, validate_whalestreet_semantic_accounts,
    SwapResult,
};
use crate::protocal::{
    aldrin_v1::{aldrin_v1_swap, AldrinV1Accounts, ALDRIN_V1_MIN_ACCOUNTS},
    aldrin_v2::{aldrin_v2_swap, AldrinV2Accounts, ALDRIN_V2_MIN_ACCOUNTS},
    aquifer::{
        aquifer_swap, validate_aquifer_semantic_accounts, AquiferAccounts, AQUIFER_STEP_ACCOUNTS,
    },
    binaryfi::{binaryfi_swap, BinaryFiAccounts, BINARYFI_MIN_ACCOUNTS},
    bisonfi::{
        bisonfi_swap, validate_bisonfi_semantic_accounts, BisonFiAccounts, BISONFI_STEP_ACCOUNTS,
    },
    bonk_swap::{bonk_swap, BonkSwapAccounts, BONK_SWAP_MIN_ACCOUNTS},
    boop_fun::{boop_fun_swap, BoopFunAccounts, BOOP_FUN_STEP_ACCOUNTS},
    carrot::{
        carrot_swap, validate_carrot_semantic_accounts, CarrotAccounts, CARROT_STEP_ACCOUNTS,
    },
    crema_clmm::{
        crema_clmm_swap, CremaClmmAccounts, CREMA_CLMM_MAX_STEP_ACCOUNTS,
        CREMA_CLMM_MIN_STEP_ACCOUNTS,
    },
    cropper::{
        cropper_swap, validate_cropper_semantic_accounts, CropperAccounts, CROPPER_STEP_ACCOUNTS,
    },
    deriverse::{
        deriverse_swap, validate_deriverse_semantic_accounts, DeriverseAccounts,
        DERIVERSE_STEP_ACCOUNTS,
    },
    fluxbeam::{fluxbeam_swap, FluxBeamAccounts, FLUXBEAM_MIN_ACCOUNTS},
    fusionamm::{
        fusionamm_swap, validate_fusionamm_semantic_accounts, FusionAmmAccounts,
        FUSIONAMM_STEP_ACCOUNTS,
    },
    gamma_swap::{gamma_swap_base_input, GammaSwapAccounts, GAMMA_SWAP_MIN_ACCOUNTS},
    gavel::{gavel_swap, validate_gavel_semantic_accounts, GavelAccounts, GAVEL_STEP_ACCOUNTS},
    goonfi::{goonfi_swap, GoonfiAccounts, GOONFI_MIN_ACCOUNTS},
    goonfi_v2::{goonfi_v2_swap, GoonfiV2Accounts, GOONFI_V2_MIN_ACCOUNTS},
    guacswap::{
        guacswap, validate_guacswap_semantic_accounts, GuacswapAccounts, GUACSWAP_MIN_ACCOUNTS,
    },
    hadron::{
        hadron_swap, validate_hadron_semantic_accounts, HadronAccounts, HADRON_BASE_STEP_ACCOUNTS,
        HADRON_SPREAD_STEP_ACCOUNTS,
    },
    heaven::{heaven_swap, HeavenAccounts, HEAVEN_STEP_ACCOUNTS},
    helium_treasury_management::{
        helium_treasury_management_swap, validate_helium_treasury_management_semantic_accounts,
        HeliumTreasuryManagementAccounts, HELIUM_TREASURY_MANAGEMENT_STEP_ACCOUNTS,
    },
    huma::{huma_swap, validate_huma_semantic_accounts, HumaAccounts, HUMA_STEP_ACCOUNTS},
    humidifi::{humidifi_swap_v3, HumidifiAccounts, HUMIDIFI_MIN_ACCOUNTS},
    hylo_earn_pool::{
        hylo_earn_pool_swap, validate_hylo_earn_pool_semantic_accounts, HyloEarnPoolAccounts,
        HYLO_EARN_POOL_STEP_ACCOUNTS,
    },
    hylo_exchange::{
        hylo_exchange_swap, validate_hylo_exchange_semantic_accounts, HyloExchangeAccounts,
        HyloExchangeValidationAccounts, HYLO_EXCHANGE_STEP_ACCOUNTS,
    },
    invariant::{invariant_swap, InvariantAccounts, INVARIANT_MIN_ACCOUNTS},
    jupiter_lend_earn::{
        jupiter_lend_earn_swap, validate_jupiter_lend_earn_semantic_accounts,
        JupiterLendEarnAccounts, JUPITER_LEND_EARN_STEP_ACCOUNTS,
    },
    kipseli::{
        kipseli_swap, validate_kipseli_semantic_accounts, KipseliAccounts, KIPSELI_STEP_ACCOUNTS,
    },
    lemmingsfi::{
        lemmingsfi_swap, validate_lemmingsfi_semantic_accounts, LemmingsFiAccounts,
        LEMMINGSFI_STEP_ACCOUNTS,
    },
    lifinity_amm_v1::{
        lifinity_amm_v1_swap, LifinityAmmV1SwapAccounts, LIFINITY_AMM_V1_MIN_ACCOUNTS,
    },
    lifinity_amm_v2::{
        lifinity_amm_v2_swap, LifinityAmmV2SwapAccounts, LIFINITY_AMM_V2_MIN_ACCOUNTS,
    },
    m_swap::{m_swap, validate_m_swap_semantic_accounts, MSwapAccounts, M_SWAP_STEP_ACCOUNTS},
    manifest::{manifest_swap, ManifestSwapAccounts, MANIFEST_MIN_ACCOUNTS},
    marcopolo::{
        marcopolo_swap, validate_marcopolo_semantic_accounts, MarcoPoloAccounts,
        MARCOPOLO_STEP_ACCOUNTS,
    },
    marinade_finance::{
        marinade_finance_swap, validate_marinade_finance_semantic_accounts,
        MarinadeFinanceAccounts, MARINADE_FINANCE_STEP_ACCOUNTS,
    },
    mercurial_stable_swap::{
        mercurial_stable_swap, MercurialStableSwapAccounts, MERCURIAL_STABLE_SWAP_MIN_ACCOUNTS,
    },
    metadao_futarchy::{
        metadao_futarchy_swap, validate_metadao_futarchy_semantic_accounts,
        MetaDaoFutarchyAccounts, METADAO_FUTARCHY_STEP_ACCOUNTS,
    },
    meteora_damm_v1::{meteora_damm_v1_swap, MeteoraDammV1Accounts, METEORA_DAMM_V1_MIN_ACCOUNTS},
    meteora_damm_v2::{meteora_damm_v2_swap, MeteoraDammV2Accounts, METEORA_DAMM_V2_MIN_ACCOUNTS},
    meteora_dbc::{meteora_dbc_swap, MeteoraDbcAccounts, METEORA_DBC_MIN_ACCOUNTS},
    meteora_dlmm::{meteora_dlmm_swap, MeteoraDlmmAccounts, METEORA_DLMM_MIN_ACCOUNTS},
    metric::{
        metric_swap, validate_metric_semantic_accounts, MetricAccounts, METRIC_STEP_ACCOUNTS,
    },
    moonit::{moonit_swap, MoonitAccounts, MOONIT_STEP_ACCOUNTS},
    obric_v2::{obric_v2_swap, ObricV2Accounts, OBRIC_V2_MIN_ACCOUNTS},
    omnipair::{
        omnipair_swap, validate_omnipair_semantic_accounts, OmnipairAccounts,
        OMNIPAIR_STEP_ACCOUNTS,
    },
    one_dex::{
        one_dex_swap, validate_one_dex_semantic_accounts, OneDexAccounts, ONE_DEX_STEP_ACCOUNTS,
    },
    openbook_v2::{openbook_v2_swap, OpenBookV2SwapAccounts, OPENBOOK_V2_MIN_ACCOUNTS},
    orca_token_swap::{orca_token_swap, OrcaTokenSwapAccounts, ORCA_TOKEN_SWAP_MIN_ACCOUNTS},
    orca_whirlpool::{orca_whirlpool_swap, OrcaWhirlpoolAccounts, ORCA_WHIRLPOOL_MIN_ACCOUNTS},
    perena_numeraire::{
        perena_numeraire_swap, validate_perena_numeraire_semantic_accounts,
        PerenaNumeraireAccounts, PERENA_NUMERAIRE_STEP_ACCOUNTS,
    },
    perena_star::{
        perena_star_swap, validate_perena_star_semantic_accounts, PerenaStarAccounts,
        PERENA_STAR_STEP_ACCOUNTS,
    },
    phoenix::{phoenix_swap, PhoenixSwapAccounts, PHOENIX_MIN_ACCOUNTS},
    pumpfun_amm::{pumpfun_amm_swap, PumpFunAmmAccounts, PUMPFUN_AMM_MIN_ACCOUNTS},
    pumpfun_swap::{pumpfun_swap_swap, PumpFunSwapAccounts, PUMPFUN_SWAP_MIN_ACCOUNTS},
    raydium_clmm::{raydium_clmm_swap, RaydiumClmmAccounts, RAYDIUM_CLMM_MIN_ACCOUNTS},
    raydium_cpmm::{raydium_cpmm_swap, RaydiumCpmmAccounts, RAYDIUM_CPMM_MIN_ACCOUNTS},
    raydium_launchpad::{
        raydium_launchpad_swap, RaydiumLaunchpadAccounts, RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
    },
    raydium_pool_v4::{raydium_pool_v4_swap, RaydiumPoolV4Accounts, RAYDIUM_POOL_V4_MIN_ACCOUNTS},
    raydium_stable_swap::{
        raydium_stable_swap, RaydiumStableSwapAccounts, RAYDIUM_STABLE_SWAP_MIN_ACCOUNTS,
    },
    riptide::{
        riptide_swap, validate_riptide_semantic_accounts, RiptideAccounts, RIPTIDE_STEP_ACCOUNTS,
    },
    saber_add_decimals::{
        saber_add_decimals_swap, validate_saber_add_decimals_semantic_accounts,
        SaberAddDecimalsAccounts, SABER_ADD_DECIMALS_STEP_ACCOUNTS,
    },
    saber_stable_swap::{
        saber_stable_swap, SaberStableSwapAccounts, SABER_STABLE_SWAP_MIN_ACCOUNTS,
    },
    sanctum_infinity::{
        sanctum_infinity_swap, SanctumInfinityAccounts, SANCTUM_INFINITY_STEP_ACCOUNTS,
    },
    sanctum_router::{sanctum_router_swap, SanctumRouterAccounts, SANCTUM_ROUTER_STEP_ACCOUNTS},
    saros_dlmm::{saros_dlmm_swap, SarosDlmmAccounts, SAROS_DLMM_STEP_ACCOUNTS},
    scale_amm::{
        scale_amm_swap, token_programs_for_direction, validate_scale_amm_semantic_accounts,
        ScaleAmmAccounts,
    },
    scale_vmm::{scale_vmm_swap, validate_scale_vmm_semantic_accounts, ScaleVmmAccounts},
    scorch::{
        scorch_swap, validate_scorch_semantic_accounts, ScorchAccounts, SCORCH_STEP_ACCOUNTS,
    },
    sencha_swap::{sencha_swap, SenchaSwapAccounts, SENCHA_SWAP_MIN_ACCOUNTS},
    serum_v3::{
        serum_v3_swap, validate_serum_v3_semantic_accounts, SerumV3Accounts, SERUM_V3_STEP_ACCOUNTS,
    },
    solayer_endoavs::{
        solayer_endoavs_swap, validate_solayer_endoavs_semantic_accounts, SolayerEndoAvsAccounts,
        SOLAYER_ENDOAVS_STEP_ACCOUNTS,
    },
    solfi_v1::{solfi_v1_swap, SolfiV1Accounts, SOLFI_V1_MIN_ACCOUNTS},
    solfi_v2::{solfi_v2_swap, SolfiV2Accounts, SOLFI_V2_MIN_ACCOUNTS},
    stabble_swap::{stabble_swap_v2, StabbleSwapAccounts, STABBLE_SWAP_MIN_ACCOUNTS},
    taurusfi::{
        taurusfi_swap, validate_taurusfi_semantic_accounts, TaurusFiAccounts,
        TAURUSFI_STEP_ACCOUNTS,
    },
    tessera::{tessera_swap, TesseraAccounts, TESSERA_MIN_ACCOUNTS},
    trends::{trends_swap, validate_trends_semantic_accounts, TrendsAccounts},
    vault_liquid_unstake::{
        validate_vault_liquid_unstake_semantic_accounts, vault_liquid_unstake_swap,
        VaultLiquidUnstakeAccounts, VAULT_LIQUID_UNSTAKE_STEP_ACCOUNTS,
    },
    vertigo::{
        validate_vertigo_semantic_accounts, vertigo_swap, VertigoAccounts, VERTIGO_STEP_ACCOUNTS,
    },
    virtuals::{validate_virtuals_semantic_accounts, virtuals_swap, VirtualsAccounts},
    voltr::{validate_voltr_semantic_accounts, voltr_swap, VoltrAccounts, VOLTR_STEP_ACCOUNTS},
    wavebreak::{
        validate_wavebreak_semantic_accounts, wavebreak_swap, WavebreakAccounts,
        WAVEBREAK_STEP_ACCOUNTS,
    },
    whalestreet::{whalestreet_swap, WhaleStreetAccounts, WHALESTREET_MIN_ACCOUNTS},
    woofi_swap::{woofi_swap, WoofiSwapAccounts, WOOFI_SWAP_MIN_ACCOUNTS},
    xorca::{
        validate_xorca_amount, validate_xorca_semantic_accounts, xorca_swap, XOrcaAccounts,
        XORCA_STEP_ACCOUNTS,
    },
};
use crate::state::Protocol;
use crate::state::SwapArbParams;
use crate::ExecuteArbitrage;

pub fn execute_arbitrage<'info>(
    ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage>,
    params: SwapArbParams,
) -> Result<()> {
    let route_accounts = parse_route_accounts(ctx.remaining_accounts, &params)?;
    let remaining_accounts = ctx.remaining_accounts;
    let payer = route_accounts.payer;
    let system_program = route_accounts.system_program;
    let associated_token_program = route_accounts.associated_token_program;
    let token_program = route_accounts.token_program;
    let token_2022_program = route_accounts.token_2022_program;
    let mints = route_accounts.mints;
    let user_accounts = route_accounts.user_accounts;
    let m = mints.len();

    let mut current_amount = params.input_amount;
    for (i, step) in params.steps.iter().enumerate() {
        let direction = step.direction_value()?.as_u8();
        // 闭环：第 i 步 (mints[i], mints[(i+1)%M])
        let in_idx = i;
        let out_idx = (i + 1) % m;
        let user_in_ai = user_accounts
            .get(in_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;
        let user_out_ai = user_accounts
            .get(out_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;
        let in_mint_ai = mints
            .get(in_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;
        let out_mint_ai = mints
            .get(out_idx)
            .ok_or(ArbitrageError::InvalidAccountIndex)?;

        let in_mint_program_ai =
            token_program_for_mint(in_mint_ai, token_program, token_2022_program)?;
        let out_mint_program_ai =
            token_program_for_mint(out_mint_ai, token_program, token_2022_program)?;
        validate_token_account_for_mint_and_authority(
            user_in_ai,
            in_mint_ai,
            in_mint_program_ai,
            payer,
        )?;
        validate_token_account_for_mint_and_authority(
            user_out_ai,
            out_mint_ai,
            out_mint_program_ai,
            payer,
        )?;

        // 切分本步账户组
        let step_slice = &remaining_accounts[route_accounts.step_ranges[i].clone()];
        let program_account = step_slice
            .first()
            .ok_or(ArbitrageError::InvalidAccountCount)?;
        validate_step_account_flags(step.protocol, step_slice)?;
        validate_step_program_account(step.protocol, program_account)?;
        validate_step_fixed_accounts(step.protocol, step_slice)?;

        let result = match step.protocol {
            Protocol::RaydiumCPMM => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= RAYDIUM_CPMM_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    in_mint_ai,
                    in_mint_program_ai,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    out_mint_ai,
                    out_mint_program_ai,
                    &step_slice[1],
                )?;
                validate_raydium_cpmm_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;
                let account_infos = RaydiumCpmmAccounts {
                    program: &step_slice[0],
                    payer,
                    authority: &step_slice[1],
                    amm_config: &step_slice[2],
                    pool_state: &step_slice[3],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    input_vault: &step_slice[4],
                    output_vault: &step_slice[5],
                    input_token_program: in_mint_program_ai,
                    output_token_program: out_mint_program_ai,
                    input_mint: in_mint_ai,
                    output_mint: out_mint_ai,
                    observation_state: &step_slice[6],
                };
                raydium_cpmm_swap(account_infos, current_amount, step.min_output_amount)
            }),
            Protocol::RaydiumCLMM
            | Protocol::ByrealCLMM
            | Protocol::PancakeSwap
            | Protocol::StabbleCLMM => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= RAYDIUM_CLMM_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_token_account_for_mint(&step_slice[3], in_mint_ai, in_mint_program_ai)?;
                validate_token_account_for_mint(&step_slice[4], out_mint_ai, out_mint_program_ai)?;
                validate_raydium_clmm_semantic_accounts(step_slice, in_mint_ai, out_mint_ai)?;
                let account_infos = RaydiumClmmAccounts {
                    program: &step_slice[0],
                    payer,
                    amm_config: &step_slice[1],
                    pool_state: &step_slice[2],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    input_vault: &step_slice[3],
                    output_vault: &step_slice[4],
                    observation_state: &step_slice[5],
                    token_program,
                    token_program_2022: token_2022_program,
                    memo_program: &step_slice[6],
                    input_mint: in_mint_ai,
                    output_mint: out_mint_ai,
                    remaining_accounts: step_slice[7..].to_vec(),
                };
                raydium_clmm_swap(account_infos, current_amount, step.min_output_amount)
            }),
            Protocol::StabbleStableSwap | Protocol::StabbleWeightedSwap => {
                execute_isolated_step(|| {
                    require!(
                        step_slice.len() >= STABBLE_SWAP_MIN_ACCOUNTS,
                        ArbitrageError::InvalidAccountCount
                    );
                    require_keys_eq!(
                        step_slice[1].key(),
                        in_mint_ai.key(),
                        ArbitrageError::InvalidTokenMint
                    );
                    require_keys_eq!(
                        step_slice[2].key(),
                        out_mint_ai.key(),
                        ArbitrageError::InvalidTokenMint
                    );
                    require_keys_eq!(
                        step_slice[10].key(),
                        in_mint_program_ai.key(),
                        ArbitrageError::InvalidAccount
                    );
                    require_keys_eq!(
                        step_slice[11].key(),
                        out_mint_program_ai.key(),
                        ArbitrageError::InvalidAccount
                    );
                    validate_token_account_for_mint(
                        &step_slice[3],
                        in_mint_ai,
                        in_mint_program_ai,
                    )?;
                    validate_token_account_for_mint(
                        &step_slice[4],
                        out_mint_ai,
                        out_mint_program_ai,
                    )?;
                    validate_token_account_for_mint(
                        &step_slice[5],
                        out_mint_ai,
                        out_mint_program_ai,
                    )?;
                    let account_infos = StabbleSwapAccounts {
                        program: &step_slice[0],
                        payer,
                        mint_in: in_mint_ai,
                        mint_out: out_mint_ai,
                        user_token_in: user_in_ai,
                        user_token_out: user_out_ai,
                        vault_token_in: &step_slice[3],
                        vault_token_out: &step_slice[4],
                        beneficiary_token_out: &step_slice[5],
                        pool: &step_slice[6],
                        withdraw_authority: &step_slice[7],
                        vault: &step_slice[8],
                        vault_authority: &step_slice[9],
                        token_program,
                        token_2022_program,
                    };
                    stabble_swap_v2(account_infos, current_amount, step.min_output_amount)
                })
            }
            Protocol::GammaSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= GAMMA_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    step_slice[4].key(),
                    in_mint_ai.key(),
                    ArbitrageError::InvalidTokenMint
                );
                require_keys_eq!(
                    step_slice[5].key(),
                    out_mint_ai.key(),
                    ArbitrageError::InvalidTokenMint
                );
                require_keys_eq!(
                    step_slice[8].key(),
                    in_mint_program_ai.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    step_slice[9].key(),
                    out_mint_program_ai.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_token_account_for_mint(&step_slice[6], in_mint_ai, in_mint_program_ai)?;
                validate_token_account_for_mint(&step_slice[7], out_mint_ai, out_mint_program_ai)?;
                let account_infos = GammaSwapAccounts {
                    program: &step_slice[0],
                    payer,
                    authority: &step_slice[1],
                    amm_config: &step_slice[2],
                    pool_state: &step_slice[3],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    input_vault: &step_slice[6],
                    output_vault: &step_slice[7],
                    input_token_program: in_mint_program_ai,
                    output_token_program: out_mint_program_ai,
                    input_mint: in_mint_ai,
                    output_mint: out_mint_ai,
                    observation_state: &step_slice[10],
                };
                gamma_swap_base_input(account_infos, current_amount, step.min_output_amount)
            }),
            Protocol::RaydiumPoolV4 => execute_raydium_pool_v4_step(
                step_slice,
                payer,
                token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::RaydiumStableSwap => execute_raydium_stable_swap_step(
                step_slice,
                payer,
                token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::SolFiV2 => execute_solfi_v2_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::SolFiV1 => execute_solfi_v1_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::FluxBeam => execute_fluxbeam_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
                None,
            ),
            Protocol::Dexlab => execute_fluxbeam_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
                Some(token_program),
            ),
            Protocol::HumidiFi => execute_humidifi_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
            ),
            Protocol::ObricV2 => execute_obric_v2_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::Tessera => execute_tessera_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::GoonFi => execute_goonfi_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::GoonFiV2 => execute_goonfi_v2_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::WhaleStreet => execute_whalestreet_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::BinaryFi => execute_binaryfi_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::XOrca => execute_isolated_step(|| {
                require!(
                    step_slice.len() == XORCA_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_xorca_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    step_slice[4].key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    step_slice[4].key(),
                    ArbitrageError::InvalidProgramId
                );
                validate_xorca_amount(
                    &step_slice[1],
                    &step_slice[2],
                    &step_slice[3],
                    current_amount,
                )?;
                xorca_swap(
                    XOrcaAccounts {
                        program: &step_slice[0],
                        owner: payer,
                        state: &step_slice[1],
                        vault: &step_slice[2],
                        user_orca: user_in_ai,
                        user_xorca: user_out_ai,
                        orca_mint: in_mint_ai,
                        xorca_mint: &step_slice[3],
                        token_program: &step_slice[4],
                    },
                    current_amount,
                )
            }),
            Protocol::Kipseli => execute_kipseli_step(
                step_slice,
                payer,
                system_program,
                token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                step.fee_rate,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::Riptide => execute_riptide_step(
                step_slice,
                payer,
                token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                step.fee_rate,
                current_amount,
            ),
            Protocol::Metric => execute_metric_step(
                step_slice,
                payer,
                token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                step.fee_rate,
                current_amount,
            ),
            Protocol::TaurusFi => execute_taurusfi_step(
                step_slice,
                payer,
                token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                step.fee_rate,
                current_amount,
            ),
            Protocol::Scorch => execute_scorch_step(
                step_slice,
                payer,
                token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                step.fee_rate,
                step.protocol_payload,
                current_amount,
                step.min_output_amount,
            ),
            Protocol::Aquifer => execute_aquifer_step(
                step_slice,
                payer,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                step.fee_rate,
                current_amount,
            ),
            Protocol::VaultLiquidUnstake => execute_isolated_step(|| {
                execute_vault_liquid_unstake_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    direction,
                    step.fee_rate,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::Wavebreak => execute_isolated_step(|| {
                execute_wavebreak_step(
                    step_slice,
                    payer,
                    system_program,
                    associated_token_program,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    direction,
                    step.fee_rate,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::Vertigo => execute_isolated_step(|| {
                execute_vertigo_step(
                    step_slice,
                    payer,
                    system_program,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    direction,
                    step.fee_rate,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::MarcoPolo => execute_isolated_step(|| {
                require!(
                    step_slice.len() == MARCOPOLO_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                let pool_price = validate_marcopolo_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (swapper_x_account, swapper_y_account) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                marcopolo_swap(
                    MarcoPoloAccounts {
                        program: &step_slice[0],
                        state: &step_slice[1],
                        pool: &step_slice[2],
                        token_x: &step_slice[3],
                        token_y: &step_slice[4],
                        pool_x_account: &step_slice[5],
                        pool_y_account: &step_slice[6],
                        swapper_x_account,
                        swapper_y_account,
                        swapper: payer,
                        referrer_x_account: &step_slice[7],
                        referrer_y_account: &step_slice[8],
                        referrer: &step_slice[9],
                        program_authority: &step_slice[10],
                        system_program,
                        token_program,
                        associated_token_program,
                        rent: &step_slice[14],
                    },
                    current_amount,
                    direction == 0,
                    pool_price,
                )
            }),
            Protocol::MarinadeFinance => execute_isolated_step(|| {
                execute_marinade_finance_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_program,
                    system_program,
                    direction,
                    current_amount,
                )
            }),
            Protocol::RaydiumLaunchPad => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= RAYDIUM_LAUNCHPAD_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                let (
                    base_mint,
                    quote_mint,
                    user_base_token,
                    user_quote_token,
                    base_token_program,
                    quote_token_program,
                ) = if direction == 0 {
                    // sell/base -> quote
                    (
                        in_mint_ai,
                        out_mint_ai,
                        user_in_ai,
                        user_out_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                } else {
                    // buy/quote -> base
                    (
                        out_mint_ai,
                        in_mint_ai,
                        user_out_ai,
                        user_in_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    base_mint,
                    base_token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    quote_mint,
                    quote_token_program,
                    &step_slice[1],
                )?;
                validate_raydium_launchpad_semantic_accounts(step_slice, base_mint, quote_mint)?;
                let account_infos = RaydiumLaunchpadAccounts {
                    payer,
                    authority: &step_slice[1],
                    global_config: &step_slice[2],
                    platform_config: &step_slice[3],
                    pool_state: &step_slice[4],
                    user_base_token,
                    user_quote_token,
                    base_vault: &step_slice[5],
                    quote_vault: &step_slice[6],
                    base_mint,
                    quote_mint,
                    base_token_program,
                    quote_token_program,
                    event_authority: &step_slice[7],
                    program: &step_slice[0],
                    system_program: &step_slice[8],
                    platform_fee_vault: &step_slice[9],
                    creator_fee_vault: &step_slice[10],
                };
                raydium_launchpad_swap(
                    account_infos,
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::PumpFunSwap => execute_pumpfun_swap_step(
                step_slice,
                payer,
                system_program,
                associated_token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.fee_rate,
            ),
            Protocol::PumpFunAMM => execute_pumpfun_amm_step(
                step_slice,
                payer,
                system_program,
                associated_token_program,
                user_in_ai,
                user_out_ai,
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
                direction,
                current_amount,
                step.fee_rate,
            ),
            Protocol::OrcaWhirlpool => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= ORCA_WHIRLPOOL_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    in_mint_ai,
                    in_mint_program_ai,
                    &step_slice[4],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    out_mint_ai,
                    out_mint_program_ai,
                    &step_slice[4],
                )?;
                validate_orca_whirlpool_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;

                let (token_owner_account_a, token_vault_a, token_owner_account_b, token_vault_b) =
                    if direction == 0 {
                        (user_in_ai, &step_slice[5], user_out_ai, &step_slice[6])
                    } else {
                        (user_out_ai, &step_slice[6], user_in_ai, &step_slice[5])
                    };
                let account_infos = OrcaWhirlpoolAccounts {
                    program: &step_slice[0],
                    payer,
                    token_program_a: &step_slice[1],
                    token_program_b: &step_slice[2],
                    memo_program: &step_slice[3],
                    whirlpool: &step_slice[4],
                    token_mint_a: &step_slice[7],
                    token_mint_b: &step_slice[8],
                    token_owner_account_a,
                    token_vault_a,
                    token_owner_account_b,
                    token_vault_b,
                    tick_array_0: &step_slice[9],
                    tick_array_1: &step_slice[10],
                    tick_array_2: &step_slice[11],
                    oracle: &step_slice[12],
                };
                orca_whirlpool_swap(
                    account_infos,
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                )
            }),
            Protocol::Cropper => execute_isolated_step(|| {
                require!(
                    step_slice.len() == CROPPER_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_cropper_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (owner_a, owner_b) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                cropper_swap(
                    CropperAccounts {
                        program: &step_slice[0],
                        payer,
                        token_program,
                        whirlpool: &step_slice[1],
                        token_owner_account_a: owner_a,
                        token_vault_a: &step_slice[2],
                        token_owner_account_b: owner_b,
                        token_vault_b: &step_slice[3],
                        tick_array_0: &step_slice[4],
                        tick_array_1: &step_slice[5],
                        tick_array_2: &step_slice[6],
                        oracle: &step_slice[7],
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                )
            }),
            Protocol::OrcaTokenSwapV2
            | Protocol::OrcaTokenSwapV1
            | Protocol::SarosSwap
            | Protocol::SplTokenSwap
            | Protocol::DooarSwap
            | Protocol::PenguinSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() == ORCA_TOKEN_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_orca_token_swap_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[3],
                    in_mint_ai,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    out_mint_ai,
                    token_program,
                    &step_slice[2],
                )?;
                require_keys_eq!(
                    step_slice[5].owner.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require!(
                    step_slice[5].data_len() >= 82,
                    ArbitrageError::InvalidAccount
                );
                validate_token_account_for_mint(&step_slice[6], &step_slice[5], token_program)?;

                orca_token_swap(
                    OrcaTokenSwapAccounts {
                        program: &step_slice[0],
                        pool_state: &step_slice[1],
                        authority: &step_slice[2],
                        payer,
                        input_token_account: user_in_ai,
                        input_vault: &step_slice[3],
                        output_vault: &step_slice[4],
                        output_token_account: user_out_ai,
                        pool_mint: &step_slice[5],
                        pool_fee_account: &step_slice[6],
                        token_program,
                    },
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::SenchaSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() == SENCHA_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_sencha_swap_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (input_reserve, output_reserve, input_fees, output_fees) = if direction == 0 {
                    (
                        &step_slice[2],
                        &step_slice[3],
                        &step_slice[4],
                        &step_slice[5],
                    )
                } else {
                    (
                        &step_slice[3],
                        &step_slice[2],
                        &step_slice[5],
                        &step_slice[4],
                    )
                };
                validate_token_account_for_mint_and_authority(
                    input_reserve,
                    in_mint_ai,
                    token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    output_reserve,
                    out_mint_ai,
                    token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    input_fees,
                    in_mint_ai,
                    token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    output_fees,
                    out_mint_ai,
                    token_program,
                    &step_slice[1],
                )?;
                sencha_swap(
                    SenchaSwapAccounts {
                        program: &step_slice[0],
                        token_program,
                        pool: &step_slice[1],
                        payer,
                        input_token_account: user_in_ai,
                        input_reserve,
                        input_fees,
                        output_token_account: user_out_ai,
                        output_reserve,
                        output_fees,
                    },
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::SaberStableSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() == SABER_STABLE_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_saber_stable_swap_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (input_reserve, output_reserve, output_admin_fees) = if direction == 0 {
                    (&step_slice[3], &step_slice[4], &step_slice[6])
                } else {
                    (&step_slice[4], &step_slice[3], &step_slice[5])
                };
                validate_token_account_for_mint_and_authority(
                    input_reserve,
                    in_mint_ai,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    output_reserve,
                    out_mint_ai,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint(output_admin_fees, out_mint_ai, token_program)?;
                saber_stable_swap(
                    SaberStableSwapAccounts {
                        program: &step_slice[0],
                        pool: &step_slice[1],
                        authority: &step_slice[2],
                        payer,
                        input_token_account: user_in_ai,
                        input_reserve,
                        output_reserve,
                        output_token_account: user_out_ai,
                        output_admin_fees,
                        token_program,
                    },
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::MercurialStableSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() == MERCURIAL_STABLE_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_mercurial_stable_swap_semantic_accounts(step_slice, direction)?;
                let (input_reserve, output_reserve) = if direction == 0 {
                    (&step_slice[3], &step_slice[4])
                } else {
                    (&step_slice[4], &step_slice[3])
                };
                validate_token_account_for_mint_and_authority(
                    input_reserve,
                    in_mint_ai,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    output_reserve,
                    out_mint_ai,
                    token_program,
                    &step_slice[2],
                )?;
                mercurial_stable_swap(
                    MercurialStableSwapAccounts {
                        program: &step_slice[0],
                        pool: &step_slice[1],
                        token_program,
                        authority: &step_slice[2],
                        payer,
                        token_a_reserve: &step_slice[3],
                        token_b_reserve: &step_slice[4],
                        input_token_account: user_in_ai,
                        output_token_account: user_out_ai,
                    },
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::Invariant => execute_isolated_step(|| {
                require!(
                    step_slice.len() == INVARIANT_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_invariant_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (account_x, account_y, mint_x, mint_y) = if direction == 0 {
                    (user_in_ai, user_out_ai, in_mint_ai, out_mint_ai)
                } else {
                    (user_out_ai, user_in_ai, out_mint_ai, in_mint_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    mint_x,
                    token_program,
                    &step_slice[6],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    mint_y,
                    token_program,
                    &step_slice[6],
                )?;
                invariant_swap(
                    InvariantAccounts {
                        program: &step_slice[0],
                        state: &step_slice[1],
                        pool: &step_slice[2],
                        tickmap: &step_slice[3],
                        account_x,
                        account_y,
                        reserve_x: &step_slice[4],
                        reserve_y: &step_slice[5],
                        payer,
                        authority: &step_slice[6],
                        token_program,
                    },
                    current_amount,
                    direction == 0,
                )
            }),
            Protocol::CremaClmm => execute_isolated_step(|| {
                execute_crema_clmm_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_program,
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::SanctumRouter => execute_isolated_step(|| {
                execute_sanctum_router_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_program,
                    direction,
                    current_amount,
                )
            }),
            Protocol::Moonit => execute_isolated_step(|| {
                execute_moonit_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_program,
                    associated_token_program,
                    system_program,
                    direction,
                    current_amount,
                )
            }),
            Protocol::BoopFun => execute_isolated_step(|| {
                execute_boop_fun_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_program,
                    associated_token_program,
                    system_program,
                    direction,
                    current_amount,
                )
            }),
            Protocol::Heaven => execute_isolated_step(|| {
                execute_heaven_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_program,
                    token_2022_program,
                    associated_token_program,
                    system_program,
                    direction,
                    current_amount,
                )
            }),
            Protocol::SarosDlmm => execute_isolated_step(|| {
                execute_saros_dlmm_step(
                    step_slice,
                    payer,
                    user_in_ai,
                    user_out_ai,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_program,
                    token_2022_program,
                    associated_token_program,
                    direction,
                    current_amount,
                )
            }),
            Protocol::PerenaNumeraire => execute_isolated_step(|| {
                require!(
                    step_slice.len() == PERENA_NUMERAIRE_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_perena_numeraire_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                    token_2022_program,
                )?;
                let (in_mint, out_mint, in_vault, out_vault, in_index, out_index) =
                    if direction == 0 {
                        (
                            &step_slice[2],
                            &step_slice[3],
                            &step_slice[4],
                            &step_slice[5],
                            0,
                            1,
                        )
                    } else {
                        (
                            &step_slice[3],
                            &step_slice[2],
                            &step_slice[5],
                            &step_slice[4],
                            1,
                            0,
                        )
                    };
                perena_numeraire_swap(
                    PerenaNumeraireAccounts {
                        program: &step_slice[0],
                        pool: &step_slice[1],
                        in_mint,
                        out_mint,
                        in_trader: user_in_ai,
                        out_trader: user_out_ai,
                        in_vault,
                        out_vault,
                        config: &step_slice[6],
                        payer,
                        token_program,
                        token_2022_program,
                    },
                    current_amount,
                    step.min_output_amount,
                    in_index,
                    out_index,
                )
            }),
            Protocol::PerenaStar => execute_isolated_step(|| {
                require!(
                    step_slice.len() == PERENA_STAR_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_perena_star_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (yielding_mint, bank_mint, yielding_user_ta, bank_mint_user_ta) =
                    if direction == 0 {
                        (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                    } else {
                        (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                    };
                perena_star_swap(
                    PerenaStarAccounts {
                        program: &step_slice[0],
                        payer,
                        bank_state: &step_slice[1],
                        vault_state: &step_slice[2],
                        oracle_state: &step_slice[3],
                        yielding_mint,
                        bank_mint,
                        yielding_user_ta,
                        bank_mint_user_ta,
                        yielding_vault_ata: &step_slice[4],
                        team_state: &step_slice[5],
                        fee_team_ata: &step_slice[6],
                        system_program,
                        token_program,
                        associated_token_program,
                        marginfi_program: &step_slice[7],
                        marginfi_group: &step_slice[8],
                        marginfi_account: &step_slice[9],
                        marginfi_bank: &step_slice[10],
                        marginfi_vault_authority: &step_slice[11],
                        marginfi_liquidity_vault: &step_slice[12],
                        marginfi_oracle: &step_slice[13],
                    },
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::MetaDaoFutarchy => execute_isolated_step(|| {
                require!(
                    step_slice.len() == METADAO_FUTARCHY_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_metadao_futarchy_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (user_base, user_quote) = if direction == 0 {
                    (user_out_ai, user_in_ai)
                } else {
                    (user_in_ai, user_out_ai)
                };
                metadao_futarchy_swap(
                    MetaDaoFutarchyAccounts {
                        program: &step_slice[0],
                        dao: &step_slice[1],
                        user_base,
                        user_quote,
                        base_vault: &step_slice[2],
                        quote_vault: &step_slice[3],
                        payer,
                        token_program,
                        event_authority: &step_slice[4],
                    },
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::Gavel => execute_isolated_step(|| {
                require!(
                    step_slice.len() == GAVEL_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_gavel_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (user_base, user_quote) = if direction == 0 {
                    (user_out_ai, user_in_ai)
                } else {
                    (user_in_ai, user_out_ai)
                };
                gavel_swap(
                    GavelAccounts {
                        program: &step_slice[0],
                        log_authority: &step_slice[1],
                        pool: &step_slice[2],
                        payer,
                        user_base,
                        user_quote,
                        base_vault: &step_slice[3],
                        quote_vault: &step_slice[4],
                        token_program,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::Omnipair => execute_isolated_step(|| {
                require!(
                    step_slice.len() == OMNIPAIR_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_omnipair_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (token_in_vault, token_out_vault) = if direction == 0 {
                    (&step_slice[4], &step_slice[5])
                } else {
                    (&step_slice[5], &step_slice[4])
                };
                omnipair_swap(
                    OmnipairAccounts {
                        program: &step_slice[0],
                        pair: &step_slice[1],
                        rate_model: &step_slice[2],
                        futarchy_authority: &step_slice[3],
                        token_in_vault,
                        token_out_vault,
                        user_token_in_account: user_in_ai,
                        user_token_out_account: user_out_ai,
                        token_in_mint: in_mint_ai,
                        token_out_mint: out_mint_ai,
                        user: payer,
                        token_program,
                        token_2022_program,
                        event_authority: &step_slice[6],
                    },
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::ScaleAmm => execute_isolated_step(|| {
                validate_scale_amm_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                    token_2022_program,
                )?;
                let (mint_a, mint_b, user_ta_a, user_ta_b) = if direction == 0 {
                    (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                } else {
                    (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                };
                let (token_program_a, token_program_b) = token_programs_for_direction(
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                    token_2022_program,
                )?;
                scale_amm_swap(
                    ScaleAmmAccounts {
                        program: &step_slice[0],
                        pool: &step_slice[1],
                        owner: &step_slice[2],
                        mint_a,
                        mint_b,
                        user_ta_a,
                        user_ta_b,
                        vault_a: &step_slice[3],
                        vault_b: &step_slice[4],
                        platform_fee_ta_a: &step_slice[5],
                        token_program_a,
                        token_program_b,
                        system_program: &step_slice[6],
                        config: &step_slice[7],
                        user: payer,
                        user_token_out: user_out_ai,
                        beneficiaries: &step_slice[8..],
                    },
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::ScaleVmm => execute_isolated_step(|| {
                validate_scale_vmm_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (mint_a, mint_b, user_ta_a, user_ta_b) = if direction == 0 {
                    (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                } else {
                    (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                };
                let (token_program_a, token_program_b) = token_programs_for_direction(
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                    token_2022_program,
                )?;
                scale_vmm_swap(
                    ScaleVmmAccounts {
                        program: &step_slice[0],
                        pair: &step_slice[1],
                        mint_a,
                        mint_b,
                        user_ta_a,
                        user_ta_b,
                        vault_a: &step_slice[2],
                        vault_b: &step_slice[3],
                        platform_fee_ta_a: &step_slice[4],
                        token_program_a,
                        token_program_b,
                        system_program,
                        config: &step_slice[5],
                        amm_program: &step_slice[6],
                        amm_pool: &step_slice[7],
                        amm_vault_a: &step_slice[8],
                        amm_vault_b: &step_slice[9],
                        amm_config: &step_slice[10],
                        user: payer,
                        user_token_out: user_out_ai,
                        beneficiaries: &step_slice[11..],
                    },
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::Virtuals => execute_isolated_step(|| {
                validate_virtuals_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                    system_program,
                )?;
                let (user_virtuals_ata, user_token_ata) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                virtuals_swap(
                    VirtualsAccounts {
                        program: &step_slice[0],
                        pool: &step_slice[1],
                        token_mint: &step_slice[2],
                        user_virtuals_ata,
                        user_token_ata,
                        pool_token_ata: &step_slice[3],
                        platform_prototype: &step_slice[4],
                        platform_virtuals_ata: &step_slice[5],
                        pool_virtuals_ata: &step_slice[6],
                        token_program,
                        user: payer,
                        user_token_out: user_out_ai,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::Trends => execute_isolated_step(|| {
                validate_trends_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;
                let (base_mint, quote_mint, base_token_program, quote_token_program) =
                    if direction == 0 {
                        (
                            in_mint_ai,
                            out_mint_ai,
                            in_mint_program_ai,
                            out_mint_program_ai,
                        )
                    } else {
                        (
                            out_mint_ai,
                            in_mint_ai,
                            out_mint_program_ai,
                            in_mint_program_ai,
                        )
                    };
                trends_swap(
                    TrendsAccounts {
                        program: &step_slice[0],
                        config: &step_slice[2],
                        pool: &step_slice[3],
                        pool_authority: &step_slice[1],
                        input_token_account: user_in_ai,
                        output_token_account: user_out_ai,
                        base_mint,
                        quote_mint,
                        base_vault: &step_slice[4],
                        quote_vault: &step_slice[5],
                        trader: payer,
                        base_token_program,
                        quote_token_program,
                        event_authority: &step_slice[6],
                    },
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::FusionAmm => execute_isolated_step(|| {
                require!(
                    step_slice.len() == FUSIONAMM_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    in_mint_ai,
                    in_mint_program_ai,
                    &step_slice[4],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    out_mint_ai,
                    out_mint_program_ai,
                    &step_slice[4],
                )?;
                validate_fusionamm_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;
                let (owner_a, owner_b, vault_a, vault_b) = if direction == 0 {
                    (user_in_ai, user_out_ai, &step_slice[5], &step_slice[6])
                } else {
                    (user_out_ai, user_in_ai, &step_slice[6], &step_slice[5])
                };
                fusionamm_swap(
                    FusionAmmAccounts {
                        program: &step_slice[0],
                        token_program_a: &step_slice[1],
                        token_program_b: &step_slice[2],
                        memo_program: &step_slice[3],
                        payer,
                        pool: &step_slice[4],
                        mint_a: &step_slice[7],
                        mint_b: &step_slice[8],
                        owner_a,
                        owner_b,
                        vault_a,
                        vault_b,
                        tick_arrays: [
                            &step_slice[9],
                            &step_slice[10],
                            &step_slice[11],
                            &step_slice[12],
                            &step_slice[13],
                        ],
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                )
            }),
            Protocol::Deriverse => execute_isolated_step(|| {
                require!(
                    step_slice.len() == DERIVERSE_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                let instrument_id = validate_deriverse_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;
                let (asset_mint, currency_mint, owner_asset, owner_currency) = if direction == 0 {
                    (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                } else {
                    (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[3],
                    asset_mint,
                    &step_slice[11],
                    &step_slice[3],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    currency_mint,
                    &step_slice[12],
                    &step_slice[4],
                )?;
                deriverse_swap(
                    DeriverseAccounts {
                        program: &step_slice[0],
                        payer,
                        asset_mint,
                        currency_mint,
                        asset_vault: &step_slice[3],
                        currency_vault: &step_slice[4],
                        market: &step_slice[5],
                        side_tree: &step_slice[6],
                        side_orders: &step_slice[7],
                        lines: &step_slice[8],
                        maps: &step_slice[9],
                        client_infos: &step_slice[10],
                        owner_asset,
                        owner_currency,
                        asset_token_program: &step_slice[11],
                        currency_token_program: &step_slice[12],
                    },
                    instrument_id,
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::Carrot => execute_isolated_step(|| {
                require!(
                    step_slice.len() == CARROT_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_carrot_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;
                let (user_shares, user_asset, asset_mint) = if direction == 0 {
                    (user_out_ai, user_in_ai, in_mint_ai)
                } else {
                    (user_in_ai, user_out_ai, out_mint_ai)
                };
                carrot_swap(
                    CarrotAccounts {
                        program: &step_slice[0],
                        vault: &step_slice[1],
                        shares_mint: &step_slice[2],
                        user_shares,
                        asset_mint,
                        asset_vault: &step_slice[4],
                        user_asset,
                        user: payer,
                        system_program: &step_slice[5],
                        asset_token_program: &step_slice[6],
                        shares_token_program: &step_slice[7],
                        log_program: &step_slice[8],
                        oracles: [&step_slice[9], &step_slice[10], &step_slice[11]],
                        reserve_vaults: [&step_slice[12], &step_slice[13], &step_slice[14]],
                    },
                    current_amount,
                    direction,
                )
            }),
            Protocol::HyloExchange => execute_isolated_step(|| {
                require!(
                    step_slice.len() == HYLO_EXCHANGE_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_hylo_exchange_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    HyloExchangeValidationAccounts {
                        input_mint: in_mint_ai,
                        output_mint: out_mint_ai,
                        input_token_program: in_mint_program_ai,
                        output_token_program: out_mint_program_ai,
                        system_program,
                        associated_token_program,
                        token_program,
                    },
                )?;
                hylo_exchange_swap(
                    HyloExchangeAccounts {
                        program: &step_slice[0],
                        hylo: &step_slice[1],
                        fee_auth: &step_slice[2],
                        vault_auth: &step_slice[3],
                        stablecoin_auth: &step_slice[4],
                        fee_vault: &step_slice[5],
                        lst_vault: &step_slice[6],
                        lst_header: &step_slice[7],
                        sol_usd_oracle: &step_slice[8],
                        event_authority: &step_slice[9],
                        user: payer,
                        user_input: user_in_ai,
                        user_output: user_out_ai,
                        input_mint: in_mint_ai,
                        output_mint: out_mint_ai,
                        token_program,
                        associated_token_program,
                        system_program,
                    },
                    current_amount,
                    direction,
                )
            }),
            Protocol::MSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() == M_SWAP_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_m_swap_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                    token_2022_program,
                    system_program,
                )?;
                m_swap(
                    MSwapAccounts {
                        program: &step_slice[0],
                        swap_global: &step_slice[1],
                        from_global: &step_slice[2],
                        to_global: &step_slice[3],
                        m_mint: &step_slice[4],
                        swap_m_account: &step_slice[5],
                        from_vault_auth: &step_slice[6],
                        to_vault_auth: &step_slice[7],
                        from_mint_auth: &step_slice[8],
                        to_mint_auth: &step_slice[9],
                        from_m_vault: &step_slice[10],
                        to_m_vault: &step_slice[11],
                        from_program: &step_slice[12],
                        to_program: &step_slice[13],
                        user: payer,
                        user_input: user_in_ai,
                        user_output: user_out_ai,
                        input_mint: in_mint_ai,
                        output_mint: out_mint_ai,
                        input_token_program: in_mint_program_ai,
                        output_token_program: out_mint_program_ai,
                        token_2022_program,
                        system_program,
                    },
                    current_amount,
                )
            }),
            Protocol::SanctumInfinity => execute_isolated_step(|| {
                require!(
                    step_slice.len() == SANCTUM_INFINITY_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                sanctum_infinity_swap(
                    SanctumInfinityAccounts {
                        payer,
                        user_input: user_in_ai,
                        user_output: user_out_ai,
                        input_mint: in_mint_ai,
                        output_mint: out_mint_ai,
                        token_program,
                        step: step_slice,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction,
                )
            }),
            Protocol::SerumV3 => execute_isolated_step(|| {
                require!(
                    step_slice.len() == SERUM_V3_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                let (base_lot_size, quote_lot_size) = validate_serum_v3_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (trader_base, trader_quote) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                serum_v3_swap(
                    SerumV3Accounts {
                        program: &step_slice[0],
                        market: &step_slice[1],
                        request_queue: &step_slice[2],
                        event_queue: &step_slice[3],
                        bids: &step_slice[4],
                        asks: &step_slice[5],
                        trader_base,
                        trader_quote,
                        payer,
                        base_vault: &step_slice[6],
                        quote_vault: &step_slice[7],
                        vault_signer: &step_slice[8],
                        token_program,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                    base_lot_size,
                    quote_lot_size,
                )
            }),
            Protocol::AldrinV1 => execute_isolated_step(|| {
                require!(
                    step_slice.len() == ALDRIN_V1_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_aldrin_v1_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (base_mint, quote_mint, user_base, user_quote) = if direction == 0 {
                    (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                } else {
                    (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    base_mint,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    quote_mint,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint(&step_slice[6], &step_slice[3], token_program)?;
                aldrin_v1_swap(
                    AldrinV1Accounts {
                        program: &step_slice[0],
                        pool: &step_slice[1],
                        pool_signer: &step_slice[2],
                        pool_mint: &step_slice[3],
                        base_token_vault: &step_slice[4],
                        quote_token_vault: &step_slice[5],
                        fee_pool_token_account: &step_slice[6],
                        payer,
                        user_base_token_account: user_base,
                        user_quote_token_account: user_quote,
                        token_program,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                )
            }),
            Protocol::AldrinV2 => execute_isolated_step(|| {
                require!(
                    step_slice.len() == ALDRIN_V2_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_aldrin_v2_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (base_mint, quote_mint, user_base, user_quote) = if direction == 0 {
                    (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                } else {
                    (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    base_mint,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    quote_mint,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint(&step_slice[6], &step_slice[3], token_program)?;
                aldrin_v2_swap(
                    AldrinV2Accounts {
                        program: &step_slice[0],
                        pool: &step_slice[1],
                        pool_signer: &step_slice[2],
                        pool_mint: &step_slice[3],
                        base_token_vault: &step_slice[4],
                        quote_token_vault: &step_slice[5],
                        fee_pool_token_account: &step_slice[6],
                        payer,
                        user_base_token_account: user_base,
                        user_quote_token_account: user_quote,
                        curve: &step_slice[7],
                        token_program,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                )
            }),
            Protocol::BonkSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() == BONK_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                let pool_price = validate_bonk_swap_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (swapper_x_account, swapper_y_account, token_x, token_y) = if direction == 0 {
                    (user_in_ai, user_out_ai, in_mint_ai, out_mint_ai)
                } else {
                    (user_out_ai, user_in_ai, out_mint_ai, in_mint_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[3],
                    token_x,
                    token_program,
                    &step_slice[8],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    token_y,
                    token_program,
                    &step_slice[8],
                )?;
                bonk_swap(
                    BonkSwapAccounts {
                        program: &step_slice[0],
                        state: &step_slice[1],
                        pool: &step_slice[2],
                        token_x,
                        token_y,
                        pool_x_account: &step_slice[3],
                        pool_y_account: &step_slice[4],
                        swapper_x_account,
                        swapper_y_account,
                        swapper: payer,
                        referrer_x_account: &step_slice[5],
                        referrer_y_account: &step_slice[6],
                        referrer: &step_slice[7],
                        program_authority: &step_slice[8],
                        system_program,
                        token_program,
                        associated_token_program,
                        rent: &step_slice[9],
                    },
                    current_amount,
                    direction == 0,
                    pool_price,
                )
            }),
            Protocol::Guacswap => execute_isolated_step(|| {
                require!(
                    step_slice.len() == GUACSWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_guacswap_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (swapper_x_account, swapper_y_account, token_x, token_y) = if direction == 0 {
                    (user_in_ai, user_out_ai, in_mint_ai, out_mint_ai)
                } else {
                    (user_out_ai, user_in_ai, out_mint_ai, in_mint_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[3],
                    token_x,
                    token_program,
                    &step_slice[8],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    token_y,
                    token_program,
                    &step_slice[8],
                )?;
                guacswap(
                    GuacswapAccounts {
                        program: &step_slice[0],
                        state: &step_slice[1],
                        pool: &step_slice[2],
                        token_x,
                        token_y,
                        pool_x_account: &step_slice[3],
                        pool_y_account: &step_slice[4],
                        swapper_x_account,
                        swapper_y_account,
                        swapper: payer,
                        referrer_x_account: &step_slice[5],
                        referrer_y_account: &step_slice[6],
                        referrer: &step_slice[7],
                        program_authority: &step_slice[8],
                        system_program,
                        token_program,
                        associated_token_program,
                        rent: &step_slice[9],
                    },
                    current_amount,
                    direction == 0,
                )
            }),
            Protocol::LemmingsFi => execute_isolated_step(|| {
                require!(
                    step_slice.len() == LEMMINGSFI_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_lemmingsfi_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (user_base, user_quote) = if direction == 0 {
                    (user_out_ai, user_in_ai)
                } else {
                    (user_in_ai, user_out_ai)
                };
                lemmingsfi_swap(
                    LemmingsFiAccounts {
                        program: &step_slice[0],
                        global_config: &step_slice[1],
                        market: &step_slice[2],
                        vault_base: &step_slice[3],
                        vault_quote: &step_slice[4],
                        user_base,
                        user_quote,
                        user: payer,
                        token_program,
                    },
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::Hadron => execute_isolated_step(|| {
                require!(
                    matches!(
                        step_slice.len(),
                        HADRON_BASE_STEP_ACCOUNTS | HADRON_SPREAD_STEP_ACCOUNTS
                    ),
                    ArbitrageError::InvalidAccountCount
                );
                validate_hadron_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    in_mint_program_ai,
                    out_mint_program_ai,
                )?;
                let (mint_x, mint_y, token_program_x, token_program_y) = if direction == 0 {
                    (
                        in_mint_ai,
                        out_mint_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                } else {
                    (
                        out_mint_ai,
                        in_mint_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                };
                hadron_swap(
                    HadronAccounts {
                        program: &step_slice[0],
                        config: &step_slice[1],
                        oracle: &step_slice[2],
                        curve_meta: &step_slice[3],
                        curve_prefabs: &step_slice[4],
                        vault_x: &step_slice[5],
                        vault_y: &step_slice[6],
                        fee_config: &step_slice[7],
                        fee_recipient_x: &step_slice[8],
                        fee_recipient_y: &step_slice[9],
                        curve_updates: &step_slice[10],
                        clock: &step_slice[11],
                        spread_config: (step_slice.len() == HADRON_SPREAD_STEP_ACCOUNTS)
                            .then_some(&step_slice[12]),
                        instructions_sysvar: (step_slice.len() == HADRON_SPREAD_STEP_ACCOUNTS)
                            .then_some(&step_slice[13]),
                        user: payer,
                        user_input: user_in_ai,
                        user_output: user_out_ai,
                        mint_x,
                        mint_y,
                        token_program_x,
                        token_program_y,
                    },
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::BisonFi => execute_isolated_step(|| {
                require!(
                    step_slice.len() == BISONFI_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                validate_bisonfi_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (user_base, user_quote) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                bisonfi_swap(
                    BisonFiAccounts {
                        program: &step_slice[0],
                        market: &step_slice[1],
                        base_vault: &step_slice[2],
                        quote_vault: &step_slice[3],
                        user_base,
                        user_quote,
                        user: payer,
                        base_token_program: &step_slice[6],
                        quote_token_program: &step_slice[7],
                        instructions_sysvar: &step_slice[8],
                    },
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::Voltr => execute_isolated_step(|| {
                require!(
                    step_slice.len() == VOLTR_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_voltr_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (expected_input_program, expected_output_program) = if direction == 0 {
                    (&step_slice[8], &step_slice[9])
                } else {
                    (&step_slice[9], &step_slice[8])
                };
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    expected_input_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    expected_output_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                let (user_asset, user_lp) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                voltr_swap(
                    VoltrAccounts {
                        program: &step_slice[0],
                        protocol: &step_slice[1],
                        vault: &step_slice[2],
                        asset_mint: &step_slice[3],
                        lp_mint: &step_slice[4],
                        idle_vault: &step_slice[5],
                        idle_authority: &step_slice[6],
                        lp_mint_authority: &step_slice[7],
                        asset_token_program: &step_slice[8],
                        lp_token_program: &step_slice[9],
                        system_program: &step_slice[10],
                        user: payer,
                        user_asset,
                        user_lp,
                    },
                    direction,
                    current_amount,
                )
            }),
            Protocol::OneDex => execute_isolated_step(|| {
                require!(
                    step_slice.len() == ONE_DEX_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_one_dex_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    step_slice[7].key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    step_slice[7].key(),
                    ArbitrageError::InvalidProgramId
                );
                one_dex_swap(
                    OneDexAccounts {
                        program: &step_slice[0],
                        metadata: &step_slice[1],
                        pool: &step_slice[2],
                        authority: &step_slice[3],
                        vault_0: &step_slice[4],
                        vault_1: &step_slice[5],
                        fee_account: &step_slice[6],
                        token_program: &step_slice[7],
                        user: payer,
                        user_input: user_in_ai,
                        user_output: user_out_ai,
                    },
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::Huma => execute_isolated_step(|| {
                require!(
                    step_slice.len() == HUMA_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_huma_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                    payer,
                )?;
                let (expected_input_program, expected_output_program) = if direction == 0 {
                    (&step_slice[13], &step_slice[14])
                } else {
                    (&step_slice[14], &step_slice[13])
                };
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    expected_input_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    expected_output_program.key(),
                    ArbitrageError::InvalidProgramId
                );
                let (user_underlying, user_mode) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                huma_swap(
                    HumaAccounts {
                        program: &step_slice[0],
                        huma_config: &step_slice[1],
                        pool_config: &step_slice[2],
                        pool_state: &step_slice[3],
                        mode_config: &step_slice[4],
                        mode_mint: &step_slice[5],
                        deployment_config: &step_slice[6],
                        deployment_state: &step_slice[7],
                        lender_state: &step_slice[8],
                        underlying_mint: &step_slice[9],
                        pool_authority: &step_slice[10],
                        pool_underlying_token: &step_slice[11],
                        treasury_underlying_token: &step_slice[12],
                        underlying_token_program: &step_slice[13],
                        mode_token_program: &step_slice[14],
                        user: payer,
                        user_underlying,
                        user_mode,
                    },
                    direction,
                    current_amount,
                )
            }),
            Protocol::SolayerEndoAvs => execute_isolated_step(|| {
                require!(
                    step_slice.len() == SOLAYER_ENDOAVS_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_solayer_endoavs_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    step_slice[5].key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    step_slice[5].key(),
                    ArbitrageError::InvalidProgramId
                );
                let (user_delegated, user_avs) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                solayer_endoavs_swap(
                    SolayerEndoAvsAccounts {
                        program: &step_slice[0],
                        endoavs: &step_slice[1],
                        avs_mint: &step_slice[2],
                        delegated_vault: &step_slice[3],
                        delegated_mint: &step_slice[4],
                        token_program: &step_slice[5],
                        user: payer,
                        user_delegated,
                        user_avs,
                    },
                    direction,
                    current_amount,
                )
            }),
            Protocol::HyloEarnPool => execute_isolated_step(|| {
                require!(
                    step_slice.len() == HYLO_EARN_POOL_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_hylo_earn_pool_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    step_slice[10].key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    step_slice[10].key(),
                    ArbitrageError::InvalidProgramId
                );
                let (user_stablecoin, user_lp_token) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                hylo_earn_pool_swap(
                    HyloEarnPoolAccounts {
                        program: &step_slice[0],
                        pool_config: &step_slice[1],
                        hylo: &step_slice[2],
                        stablecoin_mint: &step_slice[3],
                        pool_auth: &step_slice[4],
                        stablecoin_pool: &step_slice[5],
                        lp_token_auth: &step_slice[6],
                        lp_token_mint: &step_slice[7],
                        fee_auth: &step_slice[8],
                        fee_vault: &step_slice[9],
                        token_program: &step_slice[10],
                        event_authority: &step_slice[11],
                        user: payer,
                        user_stablecoin,
                        user_lp_token,
                    },
                    direction,
                    current_amount,
                )
            }),
            Protocol::JupiterLendEarn => execute_isolated_step(|| {
                require!(
                    step_slice.len() == JUPITER_LEND_EARN_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_jupiter_lend_earn_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    step_slice[13].key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    step_slice[13].key(),
                    ArbitrageError::InvalidProgramId
                );
                let (user_asset, user_f_token) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                jupiter_lend_earn_swap(
                    JupiterLendEarnAccounts {
                        program: &step_slice[0],
                        lending_admin: &step_slice[1],
                        lending: &step_slice[2],
                        asset_mint: &step_slice[3],
                        f_token_mint: &step_slice[4],
                        reserve: &step_slice[5],
                        supply_position: &step_slice[6],
                        rate_model: &step_slice[7],
                        vault: &step_slice[8],
                        claim_account: &step_slice[9],
                        liquidity: &step_slice[10],
                        liquidity_program: &step_slice[11],
                        rewards_rate_model: &step_slice[12],
                        token_program: &step_slice[13],
                        associated_token_program: &step_slice[14],
                        system_program: &step_slice[15],
                        user: payer,
                        user_asset,
                        user_f_token,
                    },
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::HeliumTreasuryManagement => execute_isolated_step(|| {
                require!(
                    step_slice.len() == HELIUM_TREASURY_MANAGEMENT_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_helium_treasury_management_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    step_slice[7].key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    step_slice[7].key(),
                    ArbitrageError::InvalidProgramId
                );
                helium_treasury_management_swap(
                    HeliumTreasuryManagementAccounts {
                        program: &step_slice[0],
                        treasury_management: &step_slice[1],
                        treasury_mint: &step_slice[2],
                        supply_mint: &step_slice[3],
                        treasury: &step_slice[4],
                        circuit_breaker: &step_slice[5],
                        circuit_breaker_program: &step_slice[6],
                        token_program: &step_slice[7],
                        owner: payer,
                        user_supply: user_in_ai,
                        user_treasury: user_out_ai,
                    },
                    direction,
                    current_amount,
                    step.min_output_amount,
                )
            }),
            Protocol::SaberAddDecimals => execute_isolated_step(|| {
                require!(
                    step_slice.len() == SABER_ADD_DECIMALS_STEP_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_saber_add_decimals_semantic_accounts(
                    step_slice,
                    direction,
                    step.fee_rate,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    step_slice[5].key(),
                    ArbitrageError::InvalidProgramId
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    step_slice[5].key(),
                    ArbitrageError::InvalidProgramId
                );
                let (user_underlying, user_wrapped) = if direction == 0 {
                    (user_in_ai, user_out_ai)
                } else {
                    (user_out_ai, user_in_ai)
                };
                saber_add_decimals_swap(
                    SaberAddDecimalsAccounts {
                        program: &step_slice[0],
                        wrapper: &step_slice[1],
                        wrapped_mint: &step_slice[2],
                        underlying_vault: &step_slice[3],
                        underlying_mint: &step_slice[4],
                        token_program: &step_slice[5],
                        owner: payer,
                        user_underlying,
                        user_wrapped,
                    },
                    direction,
                    current_amount,
                )
            }),
            Protocol::Manifest => execute_isolated_step(|| {
                require!(
                    step_slice.len() == MANIFEST_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_manifest_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                    token_2022_program,
                )?;
                let (
                    base_mint,
                    quote_mint,
                    trader_base,
                    trader_quote,
                    base_token_program,
                    quote_token_program,
                ) = if direction == 0 {
                    (
                        in_mint_ai,
                        out_mint_ai,
                        user_in_ai,
                        user_out_ai,
                        in_mint_program_ai,
                        out_mint_program_ai,
                    )
                } else {
                    (
                        out_mint_ai,
                        in_mint_ai,
                        user_out_ai,
                        user_in_ai,
                        out_mint_program_ai,
                        in_mint_program_ai,
                    )
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    base_mint,
                    base_token_program,
                    &step_slice[4],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    quote_mint,
                    quote_token_program,
                    &step_slice[5],
                )?;

                manifest_swap(
                    ManifestSwapAccounts {
                        program: &step_slice[0],
                        payer,
                        market: &step_slice[1],
                        system_program,
                        trader_base,
                        trader_quote,
                        base_vault: &step_slice[4],
                        quote_vault: &step_slice[5],
                        base_token_program,
                        quote_token_program,
                        base_mint,
                        quote_mint,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                )
            }),
            Protocol::OpenBookV2 => execute_isolated_step(|| {
                require!(
                    step_slice.len() == OPENBOOK_V2_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                let (base_lot_size, quote_lot_size, maker_rebate_rate, taker_fee) =
                    validate_openbook_v2_semantic_accounts(
                        step_slice,
                        direction,
                        in_mint_ai,
                        out_mint_ai,
                        token_program,
                        system_program,
                    )?;
                let (base_mint, quote_mint, user_base, user_quote) = if direction == 0 {
                    (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                } else {
                    (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    base_mint,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    quote_mint,
                    token_program,
                    &step_slice[2],
                )?;
                openbook_v2_swap(
                    OpenBookV2SwapAccounts {
                        program: &step_slice[0],
                        market: &step_slice[1],
                        market_authority: &step_slice[2],
                        bids: &step_slice[3],
                        asks: &step_slice[4],
                        base_vault: &step_slice[5],
                        quote_vault: &step_slice[6],
                        event_heap: &step_slice[7],
                        payer,
                        user_base,
                        user_quote,
                        token_program,
                        system_program,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                    base_lot_size,
                    quote_lot_size,
                    maker_rebate_rate,
                    taker_fee,
                )
            }),
            Protocol::Phoenix => execute_isolated_step(|| {
                require!(
                    step_slice.len() == PHOENIX_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                let (base_lot_size, quote_lot_size) = validate_phoenix_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                let (base_mint, quote_mint, trader_base, trader_quote) = if direction == 0 {
                    (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
                } else {
                    (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
                };
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    base_mint,
                    token_program,
                    &step_slice[5],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    quote_mint,
                    token_program,
                    &step_slice[6],
                )?;

                phoenix_swap(
                    PhoenixSwapAccounts {
                        program: &step_slice[0],
                        log_authority: &step_slice[1],
                        market: &step_slice[2],
                        payer,
                        trader_base,
                        trader_quote,
                        base_vault: &step_slice[5],
                        quote_vault: &step_slice[6],
                        token_program,
                    },
                    current_amount,
                    step.min_output_amount,
                    direction == 0,
                    base_lot_size,
                    quote_lot_size,
                )
            }),
            Protocol::LifinityAmmV2 => execute_isolated_step(|| {
                require!(
                    step_slice.len() == LIFINITY_AMM_V2_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_lifinity_amm_v2_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    &step_slice[3],
                    token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    &step_slice[4],
                    token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint(&step_slice[8], &step_slice[7], token_program)?;
                let (swap_source, swap_destination) = if direction == 0 {
                    (step_slice[5].clone(), step_slice[6].clone())
                } else {
                    (step_slice[6].clone(), step_slice[5].clone())
                };
                lifinity_amm_v2_swap(
                    current_amount,
                    step.min_output_amount,
                    LifinityAmmV2SwapAccounts {
                        program: step_slice[0].clone(),
                        authority: step_slice[1].clone(),
                        amm: step_slice[2].clone(),
                        user_transfer_authority: payer.clone(),
                        source_info: user_in_ai.clone(),
                        destination_info: user_out_ai.clone(),
                        swap_source,
                        swap_destination,
                        pool_mint: step_slice[7].clone(),
                        fee_account: step_slice[8].clone(),
                        token_program: token_program.clone(),
                        oracle_main_account: step_slice[10].clone(),
                        oracle_sub_account: step_slice[11].clone(),
                        oracle_pc_account: step_slice[12].clone(),
                    },
                )
            }),
            Protocol::LifinityAmmV1 => execute_isolated_step(|| {
                require!(
                    step_slice.len() == LIFINITY_AMM_V1_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_lifinity_amm_v1_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                    token_program,
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    &step_slice[3],
                    token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[6],
                    &step_slice[4],
                    token_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint(&step_slice[8], &step_slice[7], token_program)?;
                let (swap_source, swap_destination) = if direction == 0 {
                    (step_slice[5].clone(), step_slice[6].clone())
                } else {
                    (step_slice[6].clone(), step_slice[5].clone())
                };
                lifinity_amm_v1_swap(
                    current_amount,
                    step.min_output_amount,
                    LifinityAmmV1SwapAccounts {
                        program: step_slice[0].clone(),
                        authority: step_slice[1].clone(),
                        amm: step_slice[2].clone(),
                        user_transfer_authority: payer.clone(),
                        source_info: user_in_ai.clone(),
                        destination_info: user_out_ai.clone(),
                        swap_source,
                        swap_destination,
                        pool_mint: step_slice[7].clone(),
                        fee_account: step_slice[8].clone(),
                        token_program: token_program.clone(),
                        pyth_account: step_slice[10].clone(),
                        pyth_pc_account: step_slice[11].clone(),
                        config_account: step_slice[12].clone(),
                    },
                )
            }),
            Protocol::MeteoraDammV2 => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= METEORA_DAMM_V2_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                let (token_a_mint, token_b_mint, token_a_program, token_b_program) =
                    if direction == 0 {
                        (
                            in_mint_ai,
                            out_mint_ai,
                            in_mint_program_ai,
                            out_mint_program_ai,
                        )
                    } else {
                        (
                            out_mint_ai,
                            in_mint_ai,
                            out_mint_program_ai,
                            in_mint_program_ai,
                        )
                    };
                validate_token_account_for_mint_and_authority(
                    &step_slice[3],
                    token_a_mint,
                    token_a_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    token_b_mint,
                    token_b_program,
                    &step_slice[1],
                )?;
                validate_meteora_damm_v2_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;

                let account_infos = MeteoraDammV2Accounts {
                    program: &step_slice[0],
                    payer,
                    pool_authority: &step_slice[1],
                    pool: &step_slice[2],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    token_a_vault: &step_slice[3],
                    token_b_vault: &step_slice[4],
                    token_a_mint,
                    token_b_mint,
                    token_a_program,
                    token_b_program,
                    referral_token_account: &step_slice[0],
                    event_authority: &step_slice[7],
                };
                meteora_damm_v2_swap(account_infos, current_amount, step.min_output_amount)
            }),
            Protocol::MeteoraDbc => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= METEORA_DBC_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                let (base_mint, quote_mint, token_base_program, token_quote_program) =
                    if direction == 0 {
                        (
                            in_mint_ai,
                            out_mint_ai,
                            in_mint_program_ai,
                            out_mint_program_ai,
                        )
                    } else {
                        (
                            out_mint_ai,
                            in_mint_ai,
                            out_mint_program_ai,
                            in_mint_program_ai,
                        )
                    };
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    base_mint,
                    token_base_program,
                    &step_slice[1],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    quote_mint,
                    token_quote_program,
                    &step_slice[1],
                )?;
                validate_meteora_dbc_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;

                let account_infos = MeteoraDbcAccounts {
                    program: &step_slice[0],
                    payer,
                    pool_authority: &step_slice[1],
                    config: &step_slice[2],
                    pool: &step_slice[3],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    base_vault: &step_slice[4],
                    quote_vault: &step_slice[5],
                    base_mint,
                    quote_mint,
                    token_base_program,
                    token_quote_program,
                    referral_token_account: &step_slice[0],
                    event_authority: &step_slice[6],
                };
                meteora_dbc_swap(account_infos, current_amount, step.min_output_amount)
            }),
            Protocol::MeteoraDlmm => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= METEORA_DLMM_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                validate_token_account_for_mint(&step_slice[5], in_mint_ai, in_mint_program_ai)?;
                validate_token_account_for_mint(&step_slice[6], out_mint_ai, out_mint_program_ai)?;
                validate_meteora_dlmm_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;
                let (reserve_x, reserve_y) = if direction == 0 {
                    (&step_slice[5], &step_slice[6])
                } else {
                    (&step_slice[6], &step_slice[5])
                };
                let account_infos = MeteoraDlmmAccounts {
                    program: &step_slice[0],
                    payer,
                    token_x_program: &step_slice[1],
                    token_y_program: &step_slice[2],
                    memo_program: &step_slice[3],
                    lb_pair: &step_slice[4],
                    bin_array_bitmap_extension: &step_slice[10],
                    reserve_x,
                    reserve_y,
                    user_token_in: user_in_ai,
                    user_token_out: user_out_ai,
                    token_x_mint: &step_slice[7],
                    token_y_mint: &step_slice[8],
                    oracle: &step_slice[9],
                    event_authority: &step_slice[11],
                    bin_arrays: step_slice[12..].to_vec(),
                };
                meteora_dlmm_swap(account_infos, current_amount, step.min_output_amount)
            }),
            Protocol::MeteoraDammV1 => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= METEORA_DAMM_V1_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    in_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    out_mint_program_ai.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );

                let (token_a_mint, token_b_mint, protocol_token_fee) = match direction {
                    0 => (in_mint_ai, out_mint_ai, &step_slice[10]),
                    1 => (out_mint_ai, in_mint_ai, &step_slice[11]),
                    _ => return Err(ArbitrageError::InvalidInstructionData.into()),
                };
                require_keys_eq!(
                    step_slice[6].owner.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    step_slice[7].owner.key(),
                    token_program.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_token_account_for_mint_and_authority(
                    &step_slice[4],
                    token_a_mint,
                    token_program,
                    &step_slice[2],
                )?;
                validate_token_account_for_mint_and_authority(
                    &step_slice[5],
                    token_b_mint,
                    token_program,
                    &step_slice[3],
                )?;
                validate_token_account_for_mint(&step_slice[8], &step_slice[6], token_program)?;
                validate_token_account_for_mint(&step_slice[9], &step_slice[7], token_program)?;
                validate_token_account_for_mint(protocol_token_fee, in_mint_ai, token_program)?;
                validate_meteora_damm_v1_semantic_accounts(
                    step_slice,
                    direction,
                    in_mint_ai,
                    out_mint_ai,
                )?;

                let account_infos = MeteoraDammV1Accounts {
                    program: &step_slice[0],
                    payer,
                    pool: &step_slice[1],
                    input_token_account: user_in_ai,
                    output_token_account: user_out_ai,
                    a_vault: &step_slice[2],
                    b_vault: &step_slice[3],
                    a_token_vault: &step_slice[4],
                    b_token_vault: &step_slice[5],
                    a_vault_lp_mint: &step_slice[6],
                    b_vault_lp_mint: &step_slice[7],
                    a_vault_lp: &step_slice[8],
                    b_vault_lp: &step_slice[9],
                    protocol_token_fee,
                    vault_program: &step_slice[12],
                    token_program,
                };
                meteora_damm_v1_swap(account_infos, current_amount, step.min_output_amount)
            }),
            Protocol::WoofiSwap => execute_isolated_step(|| {
                require!(
                    step_slice.len() >= WOOFI_SWAP_MIN_ACCOUNTS,
                    ArbitrageError::InvalidAccountCount
                );
                require_keys_eq!(
                    step_slice[2].key(),
                    in_mint_program_ai.key(),
                    ArbitrageError::InvalidAccount
                );
                require_keys_eq!(
                    step_slice[2].key(),
                    out_mint_program_ai.key(),
                    ArbitrageError::InvalidAccount
                );
                validate_token_account_for_mint(&step_slice[5], in_mint_ai, in_mint_program_ai)?;
                validate_token_account_for_mint(&step_slice[9], out_mint_ai, out_mint_program_ai)?;

                let account_infos = WoofiSwapAccounts {
                    program: &step_slice[0],
                    wooconfig: &step_slice[1],
                    token_program: &step_slice[2],
                    payer,
                    wooracle_from: &step_slice[3],
                    woopool_from: &step_slice[4],
                    token_owner_account_from: user_in_ai,
                    token_vault_from: &step_slice[5],
                    price_update_from: &step_slice[6],
                    wooracle_to: &step_slice[7],
                    woopool_to: &step_slice[8],
                    token_owner_account_to: user_out_ai,
                    token_vault_to: &step_slice[9],
                    price_update_to: &step_slice[10],
                    woopool_quote: &step_slice[11],
                    quote_price_update: &step_slice[12],
                    quote_token_vault: &step_slice[13],
                    rebate_to: &step_slice[14],
                };
                woofi_swap(account_infos, current_amount, step.min_output_amount)
            }),
        }?;

        validate_step_min_output(result.amount_out, step.min_output_amount)?;

        // 更新运行金额（余额差法结果）
        current_amount = result.amount_out;
    }

    // 4) 终局利润阈值
    let minimum_final_amount =
        min_profit_threshold(params.input_amount, params.min_profit_lamports)?;
    require!(
        current_amount >= minimum_final_amount,
        ArbitrageError::InsufficientProfit
    );
    Ok(())
}

#[inline(never)]
fn execute_isolated_step<F>(execute: F) -> Result<SwapResult>
where
    F: FnOnce() -> Result<SwapResult>,
{
    execute()
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_pumpfun_swap_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    associated_token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    fee_rate: u16,
) -> Result<SwapResult> {
    require!(
        step_slice.len() >= PUMPFUN_SWAP_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let (
        base_mint,
        quote_mint,
        associated_base_user,
        associated_quote_user,
        base_token_program,
        quote_token_program,
        global_volume_accumulator,
        user_volume_accumulator_index,
    ) = if direction == 1 {
        (
            out_mint_ai,
            in_mint_ai,
            user_out_ai,
            user_in_ai,
            out_mint_program_ai,
            in_mint_program_ai,
            Some(&step_slice[12]),
            13,
        )
    } else {
        (
            in_mint_ai,
            out_mint_ai,
            user_in_ai,
            user_out_ai,
            in_mint_program_ai,
            out_mint_program_ai,
            None,
            12,
        )
    };
    validate_pumpfun_swap_semantic_accounts(
        &step_slice[0],
        payer,
        associated_token_program,
        base_mint,
        quote_mint,
        base_token_program,
        quote_token_program,
        step_slice,
        direction,
    )?;

    let account_infos = PumpFunSwapAccounts {
        program: &step_slice[0],
        global_account: &step_slice[1],
        base_mint,
        quote_mint,
        base_token_program,
        quote_token_program,
        associated_token_program,
        fee_recipient: &step_slice[2],
        associated_quote_fee_recipient: &step_slice[3],
        buyback_fee_recipient: &step_slice[4],
        associated_quote_buyback_fee_recipient: &step_slice[5],
        bonding_curve: &step_slice[6],
        associated_base_bonding_curve: &step_slice[7],
        associated_quote_bonding_curve: &step_slice[8],
        payer,
        associated_base_user,
        associated_quote_user,
        system_program,
        creator_vault: &step_slice[9],
        associated_creator_vault: &step_slice[10],
        sharing_config: &step_slice[11],
        global_volume_accumulator,
        user_volume_accumulator: &step_slice[user_volume_accumulator_index],
        associated_user_volume_accumulator: &step_slice[user_volume_accumulator_index + 1],
        event_authority: &step_slice[user_volume_accumulator_index + 2],
        fee_config: &step_slice[user_volume_accumulator_index + 3],
        fee_program: &step_slice[user_volume_accumulator_index + 4],
    };
    pumpfun_swap_swap(account_infos, direction, current_amount, fee_rate)
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_pumpfun_amm_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    associated_token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    fee_rate: u16,
) -> Result<SwapResult> {
    require!(
        step_slice.len() >= PUMPFUN_AMM_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let (
        base_mint,
        quote_mint,
        user_base_token_account,
        user_quote_token_account,
        base_token_program,
        quote_token_program,
    ) = if direction == 0 {
        (
            in_mint_ai,
            out_mint_ai,
            user_in_ai,
            user_out_ai,
            in_mint_program_ai,
            out_mint_program_ai,
        )
    } else {
        (
            out_mint_ai,
            in_mint_ai,
            user_out_ai,
            user_in_ai,
            out_mint_program_ai,
            in_mint_program_ai,
        )
    };

    validate_token_account_for_mint(&step_slice[3], base_mint, base_token_program)?;
    validate_token_account_for_mint(&step_slice[4], quote_mint, quote_token_program)?;
    validate_pumpfun_amm_semantic_accounts(
        &step_slice[0],
        payer,
        associated_token_program,
        quote_token_program,
        base_mint,
        quote_mint,
        step_slice,
    )?;
    let account_infos = PumpFunAmmAccounts {
        program: &step_slice[0],
        pool_state: &step_slice[1],
        payer,
        global_config: &step_slice[2],
        base_mint,
        quote_mint,
        user_base_token_account,
        user_quote_token_account,
        pool_base_token_account: &step_slice[3],
        pool_quote_token_account: &step_slice[4],
        fee_recipient: &step_slice[5],
        fee_recipient_ata: &step_slice[6],
        base_token_program,
        quote_token_program,
        system_program,
        associated_token_program,
        event_authority: &step_slice[7],
        coin_creator_vault_ata: &step_slice[8],
        coin_creator_vault_authority: &step_slice[9],
        remaining_accounts: step_slice[10..].to_vec(),
    };
    pumpfun_amm_swap(account_infos, direction, current_amount, fee_rate)
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_raydium_pool_v4_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() >= RAYDIUM_POOL_V4_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let (coin_vault_mint, pc_vault_mint, coin_vault_token_program, pc_vault_token_program) =
        if direction == 0 {
            (
                in_mint_ai,
                out_mint_ai,
                in_mint_program_ai,
                out_mint_program_ai,
            )
        } else {
            (
                out_mint_ai,
                in_mint_ai,
                out_mint_program_ai,
                in_mint_program_ai,
            )
        };
    validate_token_account_for_mint_and_authority(
        &step_slice[5],
        coin_vault_mint,
        coin_vault_token_program,
        &step_slice[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[6],
        pc_vault_mint,
        pc_vault_token_program,
        &step_slice[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[12],
        coin_vault_mint,
        coin_vault_token_program,
        &step_slice[14],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[13],
        pc_vault_mint,
        pc_vault_token_program,
        &step_slice[14],
    )?;
    validate_raydium_pool_v4_authority(&step_slice[0], &step_slice[1], &step_slice[2])?;
    validate_raydium_pool_v4_semantic_accounts(step_slice, direction, in_mint_ai, out_mint_ai)?;
    let account_infos = RaydiumPoolV4Accounts {
        program: &step_slice[0],
        token_program,
        pool_state: &step_slice[1],
        amm_authority_info: &step_slice[2],
        amm_open_orders: &step_slice[3],
        amm_target_orders: &step_slice[4],
        coin_vault: &step_slice[5],
        pc_vault: &step_slice[6],
        market_program: &step_slice[7],
        market: &step_slice[8],
        market_bids: &step_slice[9],
        market_asks: &step_slice[10],
        market_event_queue: &step_slice[11],
        market_coin_vault: &step_slice[12],
        market_pc_vault: &step_slice[13],
        market_vault_signer: &step_slice[14],
        input_token_account: user_in_ai,
        output_token_account: user_out_ai,
        payer,
    };
    raydium_pool_v4_swap(account_infos, current_amount, min_output_amount)
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_raydium_stable_swap_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() >= RAYDIUM_STABLE_SWAP_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        in_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        out_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );

    let (coin_vault_mint, pc_vault_mint) = if direction == 0 {
        (in_mint_ai, out_mint_ai)
    } else {
        (out_mint_ai, in_mint_ai)
    };
    validate_token_account_for_mint_and_authority(
        &step_slice[4],
        coin_vault_mint,
        token_program,
        &step_slice[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[5],
        pc_vault_mint,
        token_program,
        &step_slice[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[12],
        coin_vault_mint,
        token_program,
        &step_slice[14],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[13],
        pc_vault_mint,
        token_program,
        &step_slice[14],
    )?;
    validate_raydium_stable_swap_authority(&step_slice[0], &step_slice[1], &step_slice[2])?;
    validate_raydium_stable_swap_semantic_accounts(step_slice, direction, in_mint_ai, out_mint_ai)?;

    let account_infos = RaydiumStableSwapAccounts {
        program: &step_slice[0],
        token_program,
        pool_state: &step_slice[1],
        amm_authority: &step_slice[2],
        open_orders: &step_slice[3],
        coin_vault: &step_slice[4],
        pc_vault: &step_slice[5],
        model_data_account: &step_slice[6],
        market_program: &step_slice[7],
        market: &step_slice[8],
        market_bids: &step_slice[9],
        market_asks: &step_slice[10],
        market_event_queue: &step_slice[11],
        market_coin_vault: &step_slice[12],
        market_pc_vault: &step_slice[13],
        market_vault_signer: &step_slice[14],
        input_token_account: user_in_ai,
        output_token_account: user_out_ai,
        payer,
    };
    raydium_stable_swap(account_infos, current_amount, min_output_amount)
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_solfi_v2_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() >= SOLFI_V2_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_solfi_v2_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    validate_token_account_for_mint(&step_slice[4], &step_slice[6], &step_slice[8])?;
    validate_token_account_for_mint(&step_slice[5], &step_slice[7], &step_slice[9])?;
    let (user_base, user_quote) = match direction {
        0 => (user_in_ai, user_out_ai),
        1 => (user_out_ai, user_in_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    solfi_v2_swap(
        SolfiV2Accounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            oracle: &step_slice[2],
            global_config: &step_slice[3],
            base_vault: &step_slice[4],
            quote_vault: &step_slice[5],
            user_base,
            user_quote,
            base_mint: &step_slice[6],
            quote_mint: &step_slice[7],
            base_token_program: &step_slice[8],
            quote_token_program: &step_slice[9],
            instruction_sysvar: &step_slice[10],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_solfi_v1_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() >= SOLFI_V1_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_solfi_v1_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (base_mint, quote_mint, user_base, user_quote) = match direction {
        0 => (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai),
        1 => (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    validate_token_account_for_mint_and_authority(
        &step_slice[2],
        base_mint,
        &step_slice[4],
        &step_slice[1],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[3],
        quote_mint,
        &step_slice[4],
        &step_slice[1],
    )?;
    solfi_v1_swap(
        SolfiV1Accounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            base_vault: &step_slice[2],
            quote_vault: &step_slice[3],
            user_base,
            user_quote,
            token_program: &step_slice[4],
            instruction_sysvar: &step_slice[5],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_crema_clmm_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        (CREMA_CLMM_MIN_STEP_ACCOUNTS..=CREMA_CLMM_MAX_STEP_ACCOUNTS).contains(&step_slice.len()),
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        in_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        out_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    validate_crema_clmm_semantic_accounts(step_slice, direction, in_mint_ai, out_mint_ai)?;
    let (mint_a, mint_b, account_a, account_b) = if direction == 0 {
        (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai)
    } else {
        (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai)
    };
    validate_token_account_for_mint_and_authority(
        &step_slice[3],
        mint_a,
        token_program,
        &step_slice[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[4],
        mint_b,
        token_program,
        &step_slice[2],
    )?;
    crema_clmm_swap(
        CremaClmmAccounts {
            program: &step_slice[0],
            clmm_config: &step_slice[1],
            pool: &step_slice[2],
            token_a_mint: mint_a,
            token_b_mint: mint_b,
            account_a,
            account_b,
            token_a_vault: &step_slice[3],
            token_b_vault: &step_slice[4],
            tick_array_map: &step_slice[5],
            payer,
            token_program: &step_slice[6],
        },
        &step_slice[7..],
        current_amount,
        min_output_amount,
        direction == 0,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_sanctum_router_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == SANCTUM_ROUTER_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        in_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        out_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    validate_sanctum_router_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        token_program,
    )?;
    sanctum_router_swap(
        SanctumRouterAccounts {
            payer,
            user_input: user_in_ai,
            user_output: user_out_ai,
            input_mint: in_mint_ai,
            output_mint: out_mint_ai,
            token_program,
            step: step_slice,
        },
        current_amount,
        direction == 0,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_marinade_finance_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == MARINADE_FINANCE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        in_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        out_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    validate_marinade_finance_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        token_program,
        system_program,
    )?;
    marinade_finance_swap(
        MarinadeFinanceAccounts {
            payer,
            user_input: user_in_ai,
            user_output: user_out_ai,
            step: step_slice,
        },
        current_amount,
        direction == 0,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_moonit_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    associated_token_program: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == MOONIT_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        in_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        out_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    validate_moonit_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        token_program,
        associated_token_program,
        system_program,
    )?;
    moonit_swap(
        MoonitAccounts {
            payer,
            user_input: user_in_ai,
            user_output: user_out_ai,
            input_mint: in_mint_ai,
            output_mint: out_mint_ai,
            step: step_slice,
        },
        current_amount,
        direction == 0,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_boop_fun_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    associated_token_program: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == BOOP_FUN_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    require_keys_eq!(
        in_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    require_keys_eq!(
        out_mint_program_ai.key(),
        token_program.key(),
        ArbitrageError::InvalidAccount
    );
    validate_boop_fun_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        token_program,
        associated_token_program,
        system_program,
    )?;
    boop_fun_swap(
        BoopFunAccounts {
            payer,
            user_input: user_in_ai,
            user_output: user_out_ai,
            input_mint: in_mint_ai,
            output_mint: out_mint_ai,
            step: step_slice,
        },
        current_amount,
        direction == 0,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_heaven_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    token_2022_program: &'info AccountInfo<'info>,
    associated_token_program: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == HEAVEN_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_heaven_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
        token_program,
        token_2022_program,
        associated_token_program,
        system_program,
    )?;
    heaven_swap(
        HeavenAccounts {
            payer,
            user_input: user_in_ai,
            user_output: user_out_ai,
            step: step_slice,
        },
        current_amount,
        direction == 0,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_saros_dlmm_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    token_2022_program: &'info AccountInfo<'info>,
    associated_token_program: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == SAROS_DLMM_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_saros_dlmm_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
        token_program,
        token_2022_program,
        associated_token_program,
    )?;
    saros_dlmm_swap(
        SarosDlmmAccounts {
            payer,
            user_input: user_in_ai,
            user_output: user_out_ai,
            step: step_slice,
        },
        current_amount,
        direction == 0,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_fluxbeam_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
    required_classic_token_program: Option<&'info AccountInfo<'info>>,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == FLUXBEAM_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_fluxbeam_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    if let Some(classic_token_program) = required_classic_token_program {
        for token_program_index in [9_usize, 10, 11] {
            require_keys_eq!(
                step_slice[token_program_index].key(),
                classic_token_program.key(),
                ArbitrageError::InvalidAccount
            );
        }
    }
    validate_token_account_for_mint_and_authority(
        &step_slice[3],
        in_mint_ai,
        &step_slice[9],
        &step_slice[2],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[4],
        out_mint_ai,
        &step_slice[10],
        &step_slice[2],
    )?;
    fluxbeam_swap(
        FluxBeamAccounts {
            program: &step_slice[0],
            pool: &step_slice[1],
            authority: &step_slice[2],
            payer,
            input_token_account: user_in_ai,
            input_vault: &step_slice[3],
            output_vault: &step_slice[4],
            output_token_account: user_out_ai,
            pool_mint: &step_slice[5],
            pool_fee_account: &step_slice[6],
            input_mint: &step_slice[7],
            output_mint: &step_slice[8],
            input_token_program: &step_slice[9],
            output_token_program: &step_slice[10],
            pool_token_program: &step_slice[11],
        },
        current_amount,
        min_output_amount,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_humidifi_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == HUMIDIFI_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let swap_id = validate_humidifi_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[2],
        &step_slice[4],
        &step_slice[6],
        &step_slice[1],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[3],
        &step_slice[5],
        &step_slice[7],
        &step_slice[1],
    )?;
    let (user_base, user_quote) = match direction {
        0 => (user_in_ai, user_out_ai),
        1 => (user_out_ai, user_in_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    humidifi_swap_v3(
        HumidifiAccounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            base_vault: &step_slice[2],
            quote_vault: &step_slice[3],
            user_base,
            user_quote,
            clock: &step_slice[8],
            base_token_program: &step_slice[6],
            quote_token_program: &step_slice[7],
            instructions_sysvar: &step_slice[9],
            base_mint: &step_slice[4],
            quote_mint: &step_slice[5],
            extra_account: &step_slice[10],
            vote_account: &step_slice[11],
            output_token_account: user_out_ai,
        },
        swap_id,
        current_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_obric_v2_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == OBRIC_V2_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_obric_v2_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (mint_x, mint_y, user_x, user_y) = match direction {
        0 => (in_mint_ai, out_mint_ai, user_in_ai, user_out_ai),
        1 => (out_mint_ai, in_mint_ai, user_out_ai, user_in_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    validate_token_account_for_mint_and_authority(
        &step_slice[4],
        mint_x,
        &step_slice[9],
        &step_slice[1],
    )?;
    validate_token_account_for_mint_and_authority(
        &step_slice[5],
        mint_y,
        &step_slice[9],
        &step_slice[1],
    )?;
    obric_v2_swap(
        ObricV2Accounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            second_reference_oracle: &step_slice[2],
            third_reference_oracle: &step_slice[3],
            reserve_x: &step_slice[4],
            reserve_y: &step_slice[5],
            user_x,
            user_y,
            reference_oracle: &step_slice[6],
            x_price_feed: &step_slice[7],
            y_price_feed: &step_slice[8],
            token_program: &step_slice[9],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_tessera_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == TESSERA_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_tessera_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (user_base, user_quote) = match direction {
        0 => (user_in_ai, user_out_ai),
        1 => (user_out_ai, user_in_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    tessera_swap(
        TesseraAccounts {
            program: &step_slice[0],
            global_state: &step_slice[1],
            pool: &step_slice[2],
            payer,
            base_vault: &step_slice[3],
            quote_vault: &step_slice[4],
            user_base,
            user_quote,
            base_mint: &step_slice[5],
            quote_mint: &step_slice[6],
            base_token_program: &step_slice[7],
            quote_token_program: &step_slice[8],
            instructions_sysvar: &step_slice[9],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_goonfi_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == GOONFI_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    let blacklist_bump = validate_goonfi_semantic_accounts(
        step_slice,
        payer,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (user_base, user_quote) = match direction {
        0 => (user_in_ai, user_out_ai),
        1 => (user_out_ai, user_in_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    goonfi_swap(
        GoonfiAccounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            user_base,
            user_quote,
            base_vault: &step_slice[2],
            quote_vault: &step_slice[3],
            blacklist: &step_slice[4],
            instructions_sysvar: &step_slice[5],
            token_program: &step_slice[6],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
        blacklist_bump,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_goonfi_v2_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == GOONFI_V2_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_goonfi_v2_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (user_base, user_quote, base_mint, quote_mint) = match direction {
        0 => (user_in_ai, user_out_ai, in_mint_ai, out_mint_ai),
        1 => (user_out_ai, user_in_ai, out_mint_ai, in_mint_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    goonfi_v2_swap(
        GoonfiV2Accounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            user_base,
            user_quote,
            base_vault: &step_slice[2],
            quote_vault: &step_slice[3],
            base_mint,
            quote_mint,
            price_account: &step_slice[4],
            global_state: &step_slice[5],
            instructions_sysvar: &step_slice[6],
            base_token_program: &step_slice[7],
            quote_token_program: &step_slice[8],
            vote_account: &step_slice[9],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_whalestreet_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == WHALESTREET_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_whalestreet_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (user_base, user_quote) = match direction {
        0 => (user_in_ai, user_out_ai),
        1 => (user_out_ai, user_in_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    whalestreet_swap(
        WhaleStreetAccounts {
            program: &step_slice[0],
            pool: &step_slice[1],
            user_base,
            user_quote,
            base_vault: &step_slice[2],
            quote_vault: &step_slice[3],
            payer,
            token_program: &step_slice[4],
            instructions_sysvar: &step_slice[5],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_binaryfi_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == BINARYFI_MIN_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_binaryfi_semantic_accounts(
        step_slice,
        direction,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    binaryfi_swap(
        BinaryFiAccounts {
            program: &step_slice[0],
            payer,
            config: &step_slice[1],
            pool: &step_slice[2],
            authority: &step_slice[3],
            input_mint: in_mint_ai,
            output_mint: out_mint_ai,
            input_vault: &step_slice[4],
            output_vault: &step_slice[5],
            user_input: user_in_ai,
            user_output: user_out_ai,
            input_token_program: &step_slice[6],
            output_token_program: &step_slice[7],
            instructions_sysvar: &step_slice[8],
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_kipseli_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == KIPSELI_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_kipseli_semantic_accounts(
        step_slice,
        payer,
        direction,
        fee_rate,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    require_keys_eq!(
        system_program.key(),
        anchor_lang::system_program::ID,
        ArbitrageError::InvalidProgramId
    );
    let (user_base, user_quote, base_mint, quote_mint) = match direction {
        0 => (user_in_ai, user_out_ai, in_mint_ai, out_mint_ai),
        1 => (user_out_ai, user_in_ai, out_mint_ai, in_mint_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    kipseli_swap(
        KipseliAccounts {
            program: &step_slice[0],
            payer,
            pool: &step_slice[1],
            base_mint,
            quote_mint,
            user_base,
            user_quote,
            base_vault: &step_slice[2],
            quote_vault: &step_slice[3],
            token_program,
            system_program,
            ban_entry: &step_slice[4],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_riptide_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == RIPTIDE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_riptide_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    let (user_base, user_quote, base_mint, quote_mint) = match direction {
        0 => (user_in_ai, user_out_ai, in_mint_ai, out_mint_ai),
        1 => (user_out_ai, user_in_ai, out_mint_ai, in_mint_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    riptide_swap(
        RiptideAccounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            base_mint,
            quote_mint,
            user_base,
            user_quote,
            base_vault: &step_slice[2],
            quote_vault: &step_slice[3],
            token_program,
            memo_program: &step_slice[4],
            instructions_sysvar: &step_slice[5],
            output_token_account: user_out_ai,
        },
        current_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_metric_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == METRIC_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_metric_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    let (mint_a, mint_b) = match direction {
        0 => (in_mint_ai, out_mint_ai),
        1 => (out_mint_ai, in_mint_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    metric_swap(
        MetricAccounts {
            program: &step_slice[0],
            payer,
            market: &step_slice[1],
            mint_a,
            mint_b,
            vault_a: &step_slice[2],
            vault_b: &step_slice[3],
            user_source: user_in_ai,
            user_destination: user_out_ai,
            token_program,
            system_program: &step_slice[4],
            price_provider: &step_slice[5],
            feed: &step_slice[6],
            oracle: &step_slice[7],
            output_token_account: user_out_ai,
        },
        current_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_taurusfi_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == TAURUSFI_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_taurusfi_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    let (mint_a, mint_b) = match direction {
        0 => (in_mint_ai, out_mint_ai),
        1 => (out_mint_ai, in_mint_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    taurusfi_swap(
        TaurusFiAccounts {
            program: &step_slice[0],
            payer,
            pair: &step_slice[1],
            mint_a,
            mint_b,
            vault_a: &step_slice[2],
            vault_b: &step_slice[3],
            user_source: user_in_ai,
            user_destination: user_out_ai,
            authority: &step_slice[4],
            global: &step_slice[5],
            prices: &step_slice[6],
            token_program,
            instructions_sysvar: &step_slice[7],
            output_token_account: user_out_ai,
        },
        current_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_scorch_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    protocol_payload: [u8; 18],
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == SCORCH_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_scorch_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        &protocol_payload,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    require_keys_eq!(
        token_program.key(),
        anchor_spl::token::ID,
        ArbitrageError::InvalidProgramId
    );
    let (mint_a, mint_b) = match direction {
        0 => (in_mint_ai, out_mint_ai),
        1 => (out_mint_ai, in_mint_ai),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    scorch_swap(
        ScorchAccounts {
            program: &step_slice[0],
            oracle_program: &step_slice[1],
            payer,
            pool: &step_slice[2],
            mint_a,
            mint_b,
            vault_a: &step_slice[3],
            vault_b: &step_slice[4],
            user_source: user_in_ai,
            user_destination: user_out_ai,
            authority: &step_slice[5],
            global: &step_slice[6],
            oracle_a: &step_slice[7],
            oracle_b: &step_slice[8],
            token_program,
            memo_program: &step_slice[9],
            instructions_sysvar: &step_slice[10],
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
        protocol_payload,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_aquifer_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == AQUIFER_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_aquifer_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (mint_a, mint_b, token_program_a, token_program_b) = match direction {
        0 => (
            in_mint_ai,
            out_mint_ai,
            in_mint_program_ai,
            out_mint_program_ai,
        ),
        1 => (
            out_mint_ai,
            in_mint_ai,
            out_mint_program_ai,
            in_mint_program_ai,
        ),
        _ => return Err(ArbitrageError::InvalidInstructionData.into()),
    };
    aquifer_swap(
        AquiferAccounts {
            program: &step_slice[0],
            payer,
            dex: &step_slice[1],
            instance: &step_slice[2],
            mint_a,
            token_program_a,
            oracle_a: &step_slice[3],
            coin_a: &step_slice[5],
            vault_a: &step_slice[6],
            mint_b,
            token_program_b,
            oracle_b: &step_slice[4],
            coin_b: &step_slice[7],
            vault_b: &step_slice[8],
            user_source: user_in_ai,
            user_destination: user_out_ai,
            instructions_sysvar: &step_slice[9],
            output_token_account: user_out_ai,
        },
        current_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_vault_liquid_unstake_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == VAULT_LIQUID_UNSTAKE_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_vault_liquid_unstake_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    vault_liquid_unstake_swap(
        VaultLiquidUnstakeAccounts {
            payer,
            user_input: user_in_ai,
            user_output: user_out_ai,
            input_mint: in_mint_ai,
            output_mint: out_mint_ai,
            step: step_slice,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_wavebreak_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    associated_token_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == WAVEBREAK_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_wavebreak_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        current_amount,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (base_mint, user_base, quote_mint, user_quote, base_program, quote_program) =
        match direction {
            0 => (
                in_mint_ai,
                user_in_ai,
                out_mint_ai,
                user_out_ai,
                in_mint_program_ai,
                out_mint_program_ai,
            ),
            1 => (
                out_mint_ai,
                user_out_ai,
                in_mint_ai,
                user_in_ai,
                out_mint_program_ai,
                in_mint_program_ai,
            ),
            _ => return Err(ArbitrageError::InvalidInstructionData.into()),
        };
    wavebreak_swap(
        WavebreakAccounts {
            program: &step_slice[0],
            payer,
            curve: &step_slice[1],
            base_mint,
            user_base,
            quote_mint,
            quote_vault: &step_slice[2],
            user_quote,
            system_program,
            associated_token_program,
            base_token_program: base_program,
            quote_token_program: quote_program,
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn execute_vertigo_step<'info>(
    step_slice: &'info [AccountInfo<'info>],
    payer: &'info AccountInfo<'info>,
    system_program: &'info AccountInfo<'info>,
    user_in_ai: &'info AccountInfo<'info>,
    user_out_ai: &'info AccountInfo<'info>,
    in_mint_ai: &'info AccountInfo<'info>,
    out_mint_ai: &'info AccountInfo<'info>,
    in_mint_program_ai: &'info AccountInfo<'info>,
    out_mint_program_ai: &'info AccountInfo<'info>,
    direction: u8,
    fee_rate: u16,
    current_amount: u64,
    min_output_amount: u64,
) -> Result<SwapResult> {
    require!(
        step_slice.len() == VERTIGO_STEP_ACCOUNTS,
        ArbitrageError::InvalidAccountCount
    );
    validate_vertigo_semantic_accounts(
        step_slice,
        direction,
        fee_rate,
        in_mint_ai,
        out_mint_ai,
        in_mint_program_ai,
        out_mint_program_ai,
    )?;
    let (mint_a, mint_b, user_token_a, user_token_b, token_program_a, token_program_b) =
        match direction {
            0 => (
                in_mint_ai,
                out_mint_ai,
                user_in_ai,
                user_out_ai,
                in_mint_program_ai,
                out_mint_program_ai,
            ),
            1 => (
                out_mint_ai,
                in_mint_ai,
                user_out_ai,
                user_in_ai,
                out_mint_program_ai,
                in_mint_program_ai,
            ),
            _ => return Err(ArbitrageError::InvalidInstructionData.into()),
        };
    vertigo_swap(
        VertigoAccounts {
            program: &step_slice[0],
            payer,
            pool: &step_slice[1],
            owner: &step_slice[2],
            mint_a,
            mint_b,
            user_token_a,
            user_token_b,
            vault_a: &step_slice[3],
            vault_b: &step_slice[4],
            token_program_a,
            token_program_b,
            system_program,
            output_token_account: user_out_ai,
        },
        current_amount,
        min_output_amount,
        direction,
    )
}

fn min_profit_threshold(input_amount: u64, min_profit_lamports: u64) -> Result<u64> {
    input_amount
        .checked_add(min_profit_lamports)
        .ok_or(ArbitrageError::MathOverflow.into())
}

fn validate_step_min_output(amount_out: u64, min_output_amount: u64) -> Result<()> {
    require!(
        amount_out >= min_output_amount,
        ArbitrageError::InsufficientOutputAmount
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_min_output_accepts_equal_or_above_threshold() {
        assert!(validate_step_min_output(100, 100).is_ok());
        assert!(validate_step_min_output(101, 100).is_ok());
    }

    #[test]
    fn step_min_output_rejects_below_threshold() {
        let err = validate_step_min_output(99, 100).unwrap_err();
        assert_eq!(err, ArbitrageError::InsufficientOutputAmount.into());
    }

    #[test]
    fn min_profit_threshold_rejects_overflow() {
        assert_eq!(min_profit_threshold(100, 7).unwrap(), 107);

        let err = min_profit_threshold(u64::MAX, 1).unwrap_err();
        assert_eq!(err, ArbitrageError::MathOverflow.into());
    }
}
