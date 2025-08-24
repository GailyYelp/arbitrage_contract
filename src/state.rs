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