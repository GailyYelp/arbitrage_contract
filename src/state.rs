use anchor_lang::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
pub enum Protocol {
    RaydiumCPMM = 0,
    RaydiumCLMM = 1,
    RaydiumPoolV4 = 2,
    RaydiumLaunchPad = 3,
    PumpFunSwap = 4,
    PumpFunAMM = 5,
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
    pub direction: u8, // 0: buy, zero_to_one, base_to_quote   1: sell, one_to_zero, quote_to_base
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct SwapArbParams {
    pub mints_count: u8,   // 路径中涉及的 mint 数量（闭环：steps == mints_count）
    pub input_amount: u64, // 第一步输入量
    pub min_profit_lamports: u64, // 终局最小利润
    pub steps: Vec<SwapStepMeta>, // 长度应等于 mints_count
}
