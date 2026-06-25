use anchor_lang::prelude::*;

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
// - 步骤仅携带切分所需的最小元数据（不包含每步 input/output mint 与 min_out）；
// - 路由/CPI 逻辑保持不变（余额差法+终局利润校验）。
// ==============================================================================================

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct SwapStepMeta {
    pub protocol: Protocol, // 0=CPMM,1=CLMM,2=PoolV4,3=LaunchPad,4=PumpFun,5=PumpFunAMM
    pub accounts_len: u8,   // 本步账户组长度（用于从 remaining_accounts 切片）
    pub direction: u8,      // 协议内方向标记；当前仅允许 0 或 1，具体语义由协议适配层映射
    pub fee_rate: u16,      // 手续费率
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
        };

        assert!(step.validate().is_err());
    }
}
