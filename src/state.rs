use anchor_lang::prelude::*;

pub const CONTRACT_PROTOCOL_VERSION: u16 = 3;
pub const CONTRACT_PROTOCOL_COUNT: usize = 6;
pub const CLIENT_MAX_SUPPORTED_PATH_LENGTH: usize = 5;
pub const REMAINING_ACCOUNTS_FIXED_PREFIX_LEN: usize = 5;
pub const MAX_FEE_RATE_BPS: u16 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
pub enum Protocol {
    RaydiumCPMM = 0,
    RaydiumCLMM = 1,
    RaydiumPoolV4 = 2,
    RaydiumLaunchPad = 3,
    PumpFunSwap = 4,
    PumpFunAMM = 5,
}

impl Protocol {
    pub const ALL: [Protocol; CONTRACT_PROTOCOL_COUNT] = [
        Protocol::RaydiumCPMM,
        Protocol::RaydiumCLMM,
        Protocol::RaydiumPoolV4,
        Protocol::RaydiumLaunchPad,
        Protocol::PumpFunSwap,
        Protocol::PumpFunAMM,
    ];

    pub const fn contract_id(self) -> u8 {
        self as u8
    }

    pub const fn manifest_name(self) -> &'static str {
        match self {
            Protocol::RaydiumCPMM => "RaydiumCPMM",
            Protocol::RaydiumCLMM => "RaydiumCLMM",
            Protocol::RaydiumPoolV4 => "RaydiumPoolV4",
            Protocol::RaydiumLaunchPad => "RaydiumLaunchpad",
            Protocol::PumpFunSwap => "PumpFunSwap",
            Protocol::PumpFunAMM => "PumpFunAMM",
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
    pub protocol: Protocol, // 0=CPMM,1=CLMM,2=PoolV4,3=LaunchPad,4=PumpFun,5=PumpFunAMM
    pub accounts_len: u8,   // 本步账户组长度（用于从 remaining_accounts 切片）
    pub direction: u8,      // 协议内方向标记；当前仅允许 0 或 1，具体语义由协议适配层映射
    pub fee_rate: u16,      // 手续费率
    pub min_output_amount: u64, // 本步最小输出，0 表示只依赖终局利润校验
}

impl SwapStepMeta {
    pub fn direction_value(&self) -> Result<DirectionValue> {
        DirectionValue::try_from(self.direction)
    }

    pub fn validate(&self) -> Result<()> {
        let _ = self.direction_value()?;
        require!(
            self.fee_rate <= MAX_FEE_RATE_BPS,
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
    fn step_validation_rejects_fee_rate_above_bps_denominator() {
        let step = SwapStepMeta {
            protocol: Protocol::PumpFunAMM,
            accounts_len: 10,
            direction: 1,
            fee_rate: MAX_FEE_RATE_BPS + 1,
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
        for protocol in Protocol::ALL {
            assert_protocol_id(protocols, protocol.manifest_name(), protocol.contract_id());
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
