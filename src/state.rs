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

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct PathAccountMapping {
    pub dex_type: DexType,
    pub contract_type: ContractType,
    pub start_index: u8,           // 对齐字段名
    pub account_count: u8,
    pub derived_count: u8,         // 新增字段，表示合约内推导的账户数量
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct ArbitrageParams {
    pub input_amount: u64,
    pub min_profit_lamports: u64,
    pub max_slippage_bps: u16,
    pub path_steps: Vec<PathStep>,
    pub account_mappings: Vec<PathAccountMapping>,
}