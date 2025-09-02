use anchor_lang::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
pub enum DexType {
    RaydiumCpmm = 0,
    RaydiumClmm = 1,
    PumpFunBondingCurve = 2,  // 对齐money_donkey命名
    PumpSwap = 3,             // 对齐money_donkey命名
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, AnchorSerialize, AnchorDeserialize)]
pub enum ContractType {
    CPMM = 0,
    CLMM = 1,
    BondingCurve = 2,
    PumpSwap = 3,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct PathStep {
    pub pool_id: Option<Pubkey>,
    pub dex_type: DexType,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub minimum_amount_out: u64,
}


/// 协议与参数（V2 indices 协议）
/// 用途：
/// - `PathStep` 描述每步的池/DEX与最小产出；
/// - `PathAccountMappingV2.indices` 为每步指向全局 remaining_accounts 的索引；
/// - `ArbitrageParams` 汇总整条路径的输入、阈值与映射，供合约入口解析执行。
/// V2（indices 协议）：指向全局 remaining_accounts 的索引
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct PathAccountMappingV2 {
    pub dex_type: DexType,
    pub contract_type: ContractType,
    pub indices: Vec<u8>,
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct ArbitrageParams {
    pub input_amount: u64,
    pub min_profit_lamports: u64,
    pub max_slippage_bps: u16,
    pub path_steps: Vec<PathStep>,
    pub account_mappings_v2: Vec<PathAccountMappingV2>,
}

// ==============================================================================================
// V4 协议参数（分段账户 + 极简步骤元数据）
// 说明：
// - remaining_accounts 将按三段组织：mints[M]、user_token_accounts[M]、step_accounts（逐步平铺）；
// - 步骤仅携带切分所需的最小元数据（不包含每步 input/output mint 与 min_out）；
// - 路由/CPI 逻辑保持不变（余额差法+终局利润校验）。
// ==============================================================================================

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct V4StepMeta {
    pub dex_type: DexType, // 0=CPMM,1=CLMM,2=PumpFun,3=PumpSwap
    pub accounts_len: u8,  // 本步账户组长度（用于从 remaining_accounts 切片）
    pub flags: u8,         // 预留：仅 CLMM 使用 bit0 表示是否包含 tick_array_extension；其余 DEX 填 0
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct V4ArbParams {
    pub mints_count: u8,        // 路径中涉及的 mint 数量（闭环：steps == mints_count）
    pub input_amount: u64,      // 第一步输入量
    pub min_profit_lamports: u64, // 终局最小利润
    pub steps: Vec<V4StepMeta>, // 长度应等于 mints_count
}