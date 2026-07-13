use anchor_lang::prelude::*;

mod abi_generated {
    include!(concat!(env!("OUT_DIR"), "/contract_abi_generated.rs"));
}

pub use abi_generated::{
    CLIENT_MAX_SUPPORTED_PATH_LENGTH, CONTRACT_PROTOCOL_COUNT, CONTRACT_PROTOCOL_VERSION,
    REMAINING_ACCOUNTS_FIXED_PREFIX_LEN,
};
pub const FEE_RATE_BPS_DENOMINATOR: u16 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
pub enum Protocol {
    RaydiumCPMM = 0,
    RaydiumCLMM = 1,
    RaydiumPoolV4 = 2,
    RaydiumLaunchPad = 3,
    PumpFunSwap = 4,
    PumpFunAMM = 5,
    OrcaWhirlpool = 6,
    MeteoraDammV2 = 7,
    MeteoraDammV1 = 8,
    ByrealCLMM = 9,
    PancakeSwap = 10,
    StabbleCLMM = 11,
    MeteoraDbc = 12,
    MeteoraDlmm = 13,
    StabbleStableSwap = 14,
    StabbleWeightedSwap = 15,
    GammaSwap = 16,
    RaydiumStableSwap = 17,
    WoofiSwap = 18,
    OrcaTokenSwapV2 = 19,
    Manifest = 20,
    Phoenix = 21,
    LifinityAmmV2 = 22,
    LifinityAmmV1 = 23,
    OrcaTokenSwapV1 = 24,
    SarosSwap = 25,
    SplTokenSwap = 26,
    DooarSwap = 27,
    PenguinSwap = 28,
    SenchaSwap = 29,
    SaberStableSwap = 30,
    MercurialStableSwap = 31,
    Invariant = 32,
    BonkSwap = 33,
    OpenBookV2 = 34,
    AldrinV2 = 35,
    SolFiV2 = 36,
    HumidiFi = 37,
    ObricV2 = 38,
    Tessera = 39,
    GoonFi = 40,
    GoonFiV2 = 41,
    SolFiV1 = 42,
    FluxBeam = 43,
    CremaClmm = 44,
    SanctumRouter = 45,
    Moonit = 46,
    BoopFun = 47,
    Heaven = 48,
    SarosDlmm = 49,
    AldrinV1 = 50,
    PerenaNumeraire = 51,
    SerumV3 = 52,
    SanctumInfinity = 53,
    PerenaStar = 54,
    MetaDaoFutarchy = 55,
    Gavel = 56,
    Omnipair = 57,
    ScaleAmm = 58,
    ScaleVmm = 59,
    Virtuals = 60,
    Trends = 61,
    FusionAmm = 62,
    Deriverse = 63,
    Carrot = 64,
    HyloExchange = 65,
    MSwap = 66,
    Guacswap = 67,
    Cropper = 68,
    Dexlab = 69,
    LemmingsFi = 70,
    Hadron = 71,
    BisonFi = 72,
    Voltr = 73,
    OneDex = 74,
    Huma = 75,
    SolayerEndoAvs = 76,
    HyloEarnPool = 77,
    JupiterLendEarn = 78,
    HeliumTreasuryManagement = 79,
}

impl Protocol {
    pub const ALL: [Protocol; CONTRACT_PROTOCOL_COUNT] = [
        Protocol::RaydiumCPMM,
        Protocol::RaydiumCLMM,
        Protocol::RaydiumPoolV4,
        Protocol::RaydiumLaunchPad,
        Protocol::PumpFunSwap,
        Protocol::PumpFunAMM,
        Protocol::OrcaWhirlpool,
        Protocol::MeteoraDammV2,
        Protocol::MeteoraDammV1,
        Protocol::ByrealCLMM,
        Protocol::PancakeSwap,
        Protocol::StabbleCLMM,
        Protocol::MeteoraDbc,
        Protocol::MeteoraDlmm,
        Protocol::StabbleStableSwap,
        Protocol::StabbleWeightedSwap,
        Protocol::GammaSwap,
        Protocol::RaydiumStableSwap,
        Protocol::WoofiSwap,
        Protocol::OrcaTokenSwapV2,
        Protocol::Manifest,
        Protocol::Phoenix,
        Protocol::LifinityAmmV2,
        Protocol::LifinityAmmV1,
        Protocol::OrcaTokenSwapV1,
        Protocol::SarosSwap,
        Protocol::SplTokenSwap,
        Protocol::DooarSwap,
        Protocol::PenguinSwap,
        Protocol::SenchaSwap,
        Protocol::SaberStableSwap,
        Protocol::MercurialStableSwap,
        Protocol::Invariant,
        Protocol::BonkSwap,
        Protocol::OpenBookV2,
        Protocol::AldrinV2,
        Protocol::SolFiV2,
        Protocol::HumidiFi,
        Protocol::ObricV2,
        Protocol::Tessera,
        Protocol::GoonFi,
        Protocol::GoonFiV2,
        Protocol::SolFiV1,
        Protocol::FluxBeam,
        Protocol::CremaClmm,
        Protocol::SanctumRouter,
        Protocol::Moonit,
        Protocol::BoopFun,
        Protocol::Heaven,
        Protocol::SarosDlmm,
        Protocol::AldrinV1,
        Protocol::PerenaNumeraire,
        Protocol::SerumV3,
        Protocol::SanctumInfinity,
        Protocol::PerenaStar,
        Protocol::MetaDaoFutarchy,
        Protocol::Gavel,
        Protocol::Omnipair,
        Protocol::ScaleAmm,
        Protocol::ScaleVmm,
        Protocol::Virtuals,
        Protocol::Trends,
        Protocol::FusionAmm,
        Protocol::Deriverse,
        Protocol::Carrot,
        Protocol::HyloExchange,
        Protocol::MSwap,
        Protocol::Guacswap,
        Protocol::Cropper,
        Protocol::Dexlab,
        Protocol::LemmingsFi,
        Protocol::Hadron,
        Protocol::BisonFi,
        Protocol::Voltr,
        Protocol::OneDex,
        Protocol::Huma,
        Protocol::SolayerEndoAvs,
        Protocol::HyloEarnPool,
        Protocol::JupiterLendEarn,
        Protocol::HeliumTreasuryManagement,
    ];

    pub const fn contract_id(self) -> u8 {
        match self {
            Protocol::RaydiumCPMM => 0,
            Protocol::RaydiumCLMM => 1,
            Protocol::RaydiumPoolV4 => 2,
            Protocol::RaydiumLaunchPad => 3,
            Protocol::PumpFunSwap => 4,
            Protocol::PumpFunAMM => 5,
            Protocol::OrcaWhirlpool => 6,
            Protocol::MeteoraDammV2 => 7,
            Protocol::MeteoraDammV1 => 8,
            Protocol::ByrealCLMM => 9,
            Protocol::PancakeSwap => 10,
            Protocol::StabbleCLMM => 11,
            Protocol::MeteoraDbc => 12,
            Protocol::MeteoraDlmm => 13,
            Protocol::StabbleStableSwap => 14,
            Protocol::StabbleWeightedSwap => 15,
            Protocol::GammaSwap => 16,
            Protocol::RaydiumStableSwap => 17,
            Protocol::WoofiSwap => 18,
            Protocol::OrcaTokenSwapV2 => 19,
            Protocol::Manifest => 20,
            Protocol::Phoenix => 21,
            Protocol::LifinityAmmV2 => 22,
            Protocol::LifinityAmmV1 => 23,
            Protocol::OrcaTokenSwapV1 => 24,
            Protocol::SarosSwap => 25,
            Protocol::SplTokenSwap => 26,
            Protocol::DooarSwap => 27,
            Protocol::PenguinSwap => 28,
            Protocol::SenchaSwap => 29,
            Protocol::SaberStableSwap => 30,
            Protocol::MercurialStableSwap => 31,
            Protocol::Invariant => 32,
            Protocol::BonkSwap => 33,
            Protocol::OpenBookV2 => 34,
            Protocol::AldrinV2 => 35,
            Protocol::SolFiV2 => 36,
            Protocol::HumidiFi => 37,
            Protocol::ObricV2 => 38,
            Protocol::Tessera => 39,
            Protocol::GoonFi => 40,
            Protocol::GoonFiV2 => 41,
            Protocol::SolFiV1 => 42,
            Protocol::FluxBeam => 43,
            Protocol::CremaClmm => 44,
            Protocol::SanctumRouter => 45,
            Protocol::Moonit => 46,
            Protocol::BoopFun => 47,
            Protocol::Heaven => 48,
            Protocol::SarosDlmm => 49,
            Protocol::AldrinV1 => 50,
            Protocol::PerenaNumeraire => 51,
            Protocol::SerumV3 => 52,
            Protocol::SanctumInfinity => 53,
            Protocol::PerenaStar => 54,
            Protocol::MetaDaoFutarchy => 55,
            Protocol::Gavel => 56,
            Protocol::Omnipair => 57,
            Protocol::ScaleAmm => 58,
            Protocol::ScaleVmm => 59,
            Protocol::Virtuals => 60,
            Protocol::Trends => 61,
            Protocol::FusionAmm => 62,
            Protocol::Deriverse => 63,
            Protocol::Carrot => 64,
            Protocol::HyloExchange => 65,
            Protocol::MSwap => 66,
            Protocol::Guacswap => 67,
            Protocol::Cropper => 68,
            Protocol::Dexlab => 69,
            Protocol::LemmingsFi => 70,
            Protocol::Hadron => 71,
            Protocol::BisonFi => 72,
            Protocol::Voltr => 73,
            Protocol::OneDex => 74,
            Protocol::Huma => 75,
            Protocol::SolayerEndoAvs => 76,
            Protocol::HyloEarnPool => 77,
            Protocol::JupiterLendEarn => 78,
            Protocol::HeliumTreasuryManagement => 79,
        }
    }

    pub const fn manifest_name(self) -> &'static str {
        match self {
            Protocol::RaydiumCPMM => "RaydiumCPMM",
            Protocol::RaydiumCLMM => "RaydiumCLMM",
            Protocol::RaydiumPoolV4 => "RaydiumPoolV4",
            Protocol::RaydiumLaunchPad => "RaydiumLaunchpad",
            Protocol::PumpFunSwap => "PumpFunSwap",
            Protocol::PumpFunAMM => "PumpFunAMM",
            Protocol::OrcaWhirlpool => "OrcaWhirlpool",
            Protocol::MeteoraDammV2 => "MeteoraDammV2",
            Protocol::MeteoraDammV1 => "MeteoraDammV1",
            Protocol::ByrealCLMM => "ByrealCLMM",
            Protocol::PancakeSwap => "PancakeSwap",
            Protocol::StabbleCLMM => "StabbleCLMM",
            Protocol::MeteoraDbc => "MeteoraDbc",
            Protocol::MeteoraDlmm => "MeteoraDlmm",
            Protocol::StabbleStableSwap => "StabbleStableSwap",
            Protocol::StabbleWeightedSwap => "StabbleWeightedSwap",
            Protocol::GammaSwap => "GammaSwap",
            Protocol::RaydiumStableSwap => "RaydiumStableSwap",
            Protocol::WoofiSwap => "WoofiSwap",
            Protocol::OrcaTokenSwapV2 => "OrcaTokenSwapV2",
            Protocol::Manifest => "Manifest",
            Protocol::Phoenix => "Phoenix",
            Protocol::LifinityAmmV2 => "LifinityAmmV2",
            Protocol::LifinityAmmV1 => "LifinityAmmV1",
            Protocol::OrcaTokenSwapV1 => "OrcaTokenSwapV1",
            Protocol::SarosSwap => "SarosSwap",
            Protocol::SplTokenSwap => "SplTokenSwap",
            Protocol::DooarSwap => "DooarSwap",
            Protocol::PenguinSwap => "PenguinSwap",
            Protocol::SenchaSwap => "SenchaSwap",
            Protocol::SaberStableSwap => "SaberStableSwap",
            Protocol::MercurialStableSwap => "MercurialStableSwap",
            Protocol::Invariant => "Invariant",
            Protocol::BonkSwap => "BonkSwap",
            Protocol::OpenBookV2 => "OpenBookV2",
            Protocol::AldrinV2 => "AldrinV2",
            Protocol::SolFiV2 => "SolFiV2",
            Protocol::HumidiFi => "HumidiFi",
            Protocol::ObricV2 => "ObricV2",
            Protocol::Tessera => "Tessera",
            Protocol::GoonFi => "GoonFi",
            Protocol::GoonFiV2 => "GoonFiV2",
            Protocol::SolFiV1 => "SolFiV1",
            Protocol::FluxBeam => "FluxBeam",
            Protocol::CremaClmm => "CremaClmm",
            Protocol::SanctumRouter => "SanctumRouter",
            Protocol::Moonit => "Moonit",
            Protocol::BoopFun => "BoopFun",
            Protocol::Heaven => "Heaven",
            Protocol::SarosDlmm => "SarosDlmm",
            Protocol::AldrinV1 => "AldrinV1",
            Protocol::PerenaNumeraire => "PerenaNumeraire",
            Protocol::SerumV3 => "SerumV3",
            Protocol::SanctumInfinity => "SanctumInfinity",
            Protocol::PerenaStar => "PerenaStar",
            Protocol::MetaDaoFutarchy => "MetaDaoFutarchy",
            Protocol::Gavel => "Gavel",
            Protocol::Omnipair => "Omnipair",
            Protocol::ScaleAmm => "ScaleAmm",
            Protocol::ScaleVmm => "ScaleVmm",
            Protocol::Virtuals => "Virtuals",
            Protocol::Trends => "Trends",
            Protocol::FusionAmm => "FusionAmm",
            Protocol::Deriverse => "Deriverse",
            Protocol::Carrot => "Carrot",
            Protocol::HyloExchange => "HyloExchange",
            Protocol::MSwap => "MSwap",
            Protocol::Guacswap => "Guacswap",
            Protocol::Cropper => "Cropper",
            Protocol::Dexlab => "Dexlab",
            Protocol::LemmingsFi => "LemmingsFi",
            Protocol::Hadron => "Hadron",
            Protocol::BisonFi => "BisonFi",
            Protocol::Voltr => "Voltr",
            Protocol::OneDex => "OneDex",
            Protocol::Huma => "Huma",
            Protocol::SolayerEndoAvs => "SolayerEndoAvs",
            Protocol::HyloEarnPool => "HyloEarnPool",
            Protocol::JupiterLendEarn => "JupiterLendEarn",
            Protocol::HeliumTreasuryManagement => "HeliumTreasuryManagement",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionValue {
    Zero,
    One,
}

impl DirectionValue {
    pub fn as_u8(self) -> u8 {
        match self {
            DirectionValue::Zero => 0,
            DirectionValue::One => 1,
        }
    }
}

impl TryFrom<u8> for DirectionValue {
    type Error = anchor_lang::error::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(DirectionValue::Zero),
            1 => Ok(DirectionValue::One),
            _ => Err(crate::errors::ArbitrageError::InvalidInstructionData.into()),
        }
    }
}

// ==============================================================================================
// 协议参数（分段账户 + 极简步骤元数据）
// 说明：
// - remaining_accounts 将按三段组织：mints[M]、user_token_accounts[M]、step_accounts（逐步平铺）；
// - 步骤携带切分所需元数据与每步 min_out，不重复携带 input/output mint；
// - 路由/CPI 逻辑保持不变（余额差法 + 每步最小输出 + 终局利润校验）。
// ==============================================================================================

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct SwapStepMeta {
    pub protocol: Protocol,
    pub accounts_len: u8, // 本步账户组长度（用于从 remaining_accounts 切片）
    pub direction: u8,    // 协议内方向标记；当前仅允许 0 或 1，具体语义由协议适配层映射
    pub fee_rate: u16,    // 手续费率
    pub min_output_amount: u64, // 本步最小输出，0 表示只依赖终局利润校验
}

impl SwapStepMeta {
    pub fn direction_value(&self) -> Result<DirectionValue> {
        DirectionValue::try_from(self.direction)
    }

    pub fn validate(&self) -> Result<()> {
        let _ = self.direction_value()?;
        require!(
            self.fee_rate < FEE_RATE_BPS_DENOMINATOR,
            crate::errors::ArbitrageError::FeeTooHigh
        );
        Ok(())
    }
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct SwapArbParams {
    pub mints_count: u8,   // 路径中涉及的 mint 数量（闭环：steps == mints_count）
    pub input_amount: u64, // 第一步输入量
    pub min_profit_lamports: u64, // 终局最小利润
    pub steps: Vec<SwapStepMeta>, // 长度应等于 mints_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::BTreeMap;

    #[test]
    fn direction_accepts_only_zero_or_one() {
        assert_eq!(DirectionValue::try_from(0).unwrap(), DirectionValue::Zero);
        assert_eq!(DirectionValue::try_from(1).unwrap(), DirectionValue::One);
        assert!(DirectionValue::try_from(2).is_err());
    }

    #[test]
    fn step_validation_rejects_fee_rate_at_or_above_bps_denominator() {
        let valid_step = SwapStepMeta {
            protocol: Protocol::PumpFunAMM,
            accounts_len: 10,
            direction: 1,
            fee_rate: FEE_RATE_BPS_DENOMINATOR - 1,
            min_output_amount: 0,
        };
        assert!(valid_step.validate().is_ok());

        let step = SwapStepMeta {
            protocol: Protocol::PumpFunAMM,
            accounts_len: 10,
            direction: 1,
            fee_rate: FEE_RATE_BPS_DENOMINATOR,
            min_output_amount: 0,
        };

        assert!(step.validate().is_err());
    }

    #[test]
    fn abi_manifest_matches_contract_protocol_ids_and_step_layout() {
        let manifest: Value =
            serde_json::from_str(include_str!("../../contract_abi.json")).expect("abi manifest");

        assert_eq!(
            manifest["contract_protocol_version"].as_u64(),
            Some(u64::from(CONTRACT_PROTOCOL_VERSION))
        );
        assert_eq!(
            manifest["contract_protocol_count"].as_u64(),
            Some(CONTRACT_PROTOCOL_COUNT as u64)
        );
        assert_eq!(CONTRACT_PROTOCOL_COUNT, Protocol::ALL.len());
        assert_eq!(
            manifest["limits"]["client_max_supported_path_length"].as_u64(),
            Some(CLIENT_MAX_SUPPORTED_PATH_LENGTH as u64)
        );
        assert_eq!(
            manifest["limits"]["remaining_accounts_fixed_prefix_len"].as_u64(),
            Some(REMAINING_ACCOUNTS_FIXED_PREFIX_LEN as u64)
        );

        let protocols = manifest["contract_protocols"]
            .as_array()
            .expect("contract protocols");
        let manifest_contract_protocol_count = protocols
            .iter()
            .filter(|protocol| protocol["contract_enum"].as_bool() == Some(true))
            .count();
        assert_eq!(manifest_contract_protocol_count, Protocol::ALL.len());
        let generated_contract_protocol_count = abi_generated::CONTRACT_PROTOCOLS_MANIFEST
            .iter()
            .filter(|protocol| protocol.contract_enum)
            .count();
        assert_eq!(generated_contract_protocol_count, Protocol::ALL.len());
        for protocol in Protocol::ALL {
            assert_protocol_id(protocols, protocol.manifest_name(), protocol.contract_id());
            assert_generated_protocol_id(protocol.manifest_name(), protocol.contract_id());
        }

        let step_fields = manifest["swap_step_meta"]["fields"]
            .as_array()
            .expect("swap step fields")
            .iter()
            .map(|field| {
                (
                    field["name"].as_str().expect("field name"),
                    field["type"].as_str().expect("field type"),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            step_fields,
            vec![
                ("protocol", "Protocol"),
                ("accounts_len", "u8"),
                ("direction", "u8"),
                ("fee_rate", "u16"),
                ("min_output_amount", "u64"),
            ]
        );
    }

    #[test]
    fn abi_constants_are_generated_from_manifest() {
        let production = include_str!("state.rs")
            .split_once("\n#[cfg(test)]")
            .map(|(production, _)| production)
            .unwrap_or_else(|| include_str!("state.rs"));

        assert!(
            production
                .contains("include!(concat!(env!(\"OUT_DIR\"), \"/contract_abi_generated.rs\"))"),
            "contract ABI constants must come from the build-script generated manifest module"
        );
        for forbidden in [
            "pub const CONTRACT_PROTOCOL_VERSION: u16 =",
            "pub const CONTRACT_PROTOCOL_COUNT: usize =",
            "pub const CLIENT_MAX_SUPPORTED_PATH_LENGTH: usize =",
            "pub const REMAINING_ACCOUNTS_FIXED_PREFIX_LEN: usize =",
            "self as u8",
        ] {
            assert!(
                !production.contains(forbidden),
                "contract ABI production code must not use forbidden pattern: {forbidden}"
            );
        }
    }

    #[test]
    fn swap_step_meta_serialization_matches_manifest_field_order() {
        let manifest: Value =
            serde_json::from_str(include_str!("../../contract_abi.json")).expect("abi manifest");
        let step_fields = manifest_fields(&manifest, "swap_step_meta");
        let offsets = field_offsets(&step_fields);
        let step = SwapStepMeta {
            protocol: Protocol::RaydiumLaunchPad,
            accounts_len: 8,
            direction: 1,
            fee_rate: 42,
            min_output_amount: 123_456,
        };

        let encoded = serialize_to_vec(&step);
        assert_eq!(encoded.len(), serialized_len(&step_fields));
        assert_eq!(
            encoded[offsets["protocol"]],
            Protocol::RaydiumLaunchPad.contract_id()
        );
        assert_eq!(encoded[offsets["accounts_len"]], step.accounts_len);
        assert_eq!(encoded[offsets["direction"]], step.direction);
        assert_eq!(
            &encoded[offsets["fee_rate"]..offsets["fee_rate"] + 2],
            &step.fee_rate.to_le_bytes()
        );
        assert_eq!(
            &encoded[offsets["min_output_amount"]..offsets["min_output_amount"] + 8],
            &step.min_output_amount.to_le_bytes()
        );
    }

    #[test]
    fn swap_arb_params_serialization_matches_manifest_field_order() {
        let manifest: Value =
            serde_json::from_str(include_str!("../../contract_abi.json")).expect("abi manifest");
        let param_fields = manifest_fields(&manifest, "swap_arb_params");
        let step = SwapStepMeta {
            protocol: Protocol::PumpFunSwap,
            accounts_len: 9,
            direction: 0,
            fee_rate: 25,
            min_output_amount: 99,
        };
        let step_encoded = serialize_to_vec(&step);
        let params = SwapArbParams {
            mints_count: 1,
            input_amount: 1_000,
            min_profit_lamports: 7,
            steps: vec![step],
        };
        let encoded = serialize_to_vec(&params);
        let mut cursor = 0usize;

        for (name, field_type) in param_fields {
            match (name.as_str(), field_type.as_str()) {
                ("mints_count", "u8") => {
                    assert_eq!(encoded[cursor], params.mints_count);
                    cursor += 1;
                }
                ("input_amount", "u64") => {
                    assert_eq!(
                        &encoded[cursor..cursor + 8],
                        &params.input_amount.to_le_bytes()
                    );
                    cursor += 8;
                }
                ("min_profit_lamports", "u64") => {
                    assert_eq!(
                        &encoded[cursor..cursor + 8],
                        &params.min_profit_lamports.to_le_bytes()
                    );
                    cursor += 8;
                }
                ("steps", "Vec<SwapStepMeta>") => {
                    assert_eq!(&encoded[cursor..cursor + 4], &1_u32.to_le_bytes());
                    cursor += 4;
                    assert_eq!(&encoded[cursor..cursor + step_encoded.len()], &step_encoded);
                    cursor += step_encoded.len();
                }
                other => panic!("unsupported SwapArbParams field in ABI manifest: {other:?}"),
            }
        }

        assert_eq!(cursor, encoded.len());
    }

    fn assert_protocol_id(protocols: &[Value], name: &str, expected_id: u8) {
        let protocol = protocols
            .iter()
            .find(|protocol| protocol["name"].as_str() == Some(name))
            .unwrap_or_else(|| panic!("missing protocol in ABI manifest: {name}"));

        assert_eq!(protocol["contract_enum"].as_bool(), Some(true));
        assert_eq!(protocol["id"].as_u64(), Some(u64::from(expected_id)));
    }

    fn assert_generated_protocol_id(name: &str, expected_id: u8) {
        let protocol = abi_generated::CONTRACT_PROTOCOLS_MANIFEST
            .iter()
            .find(|protocol| protocol.name == name)
            .unwrap_or_else(|| panic!("missing generated ABI manifest entry: {name}"));

        assert!(protocol.contract_enum);
        assert_eq!(protocol.id, Some(expected_id));
    }

    fn manifest_fields(manifest: &Value, section: &str) -> Vec<(String, String)> {
        manifest[section]["fields"]
            .as_array()
            .unwrap_or_else(|| panic!("missing {section} fields"))
            .iter()
            .map(|field| {
                (
                    field["name"].as_str().expect("field name").to_string(),
                    field["type"].as_str().expect("field type").to_string(),
                )
            })
            .collect()
    }

    fn serialized_len(fields: &[(String, String)]) -> usize {
        fields
            .iter()
            .map(|(_, field_type)| fixed_field_len(field_type))
            .sum()
    }

    fn field_offsets(fields: &[(String, String)]) -> BTreeMap<String, usize> {
        let mut offset = 0usize;
        let mut offsets = BTreeMap::new();
        for (name, field_type) in fields {
            offsets.insert(name.clone(), offset);
            offset += fixed_field_len(field_type);
        }
        offsets
    }

    fn fixed_field_len(field_type: &str) -> usize {
        match field_type {
            "Protocol" | "u8" => 1,
            "u16" => 2,
            "u64" => 8,
            other => panic!("unsupported fixed ABI field type: {other}"),
        }
    }

    fn serialize_to_vec<T: AnchorSerialize>(value: &T) -> Vec<u8> {
        let mut encoded = Vec::new();
        value.serialize(&mut encoded).expect("serialize ABI value");
        encoded
    }
}
