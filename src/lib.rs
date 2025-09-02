use anchor_lang::prelude::*;

/// 部署/升级必要命令
///
/// 1) 生成/查看 Program ID
///    solana-keygen new -o target/deploy/arbitrage_contract-devnet-keypair.json -s --no-bip39-passphrase --force
///    solana-keygen pubkey target/deploy/arbitrage_contract-devnet-keypair.json
///
/// 2) 配置（三处一致）
///    - src/lib.rs: declare_id!("<ProgramID>")
///    - Anchor.toml: [programs.<cluster>].arbitrage_contract = "<ProgramID>"
///    - 客户端常量：ARBITRAGE_CONTRACT_ID = "<ProgramID>"
///
/// 3) 构建/部署（示例：devnet）
///    solana config set --url devnet
///    anchor build -- --features devnet    // 切换 ProgramIds 到 devnet
///    anchor deploy
///
/// 4) 升级
///    anchor upgrade <ProgramID> target/deploy/arbitrage_contract.so

pub mod instructions;
pub mod state;
pub mod errors;
pub mod account_resolver;
pub mod account_derivation;
pub mod dex_router;
pub use instructions::*;
pub use state::*;

declare_id!("4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC");

#[program]
pub mod arbitrage_contract {
    use super::*;
    
    pub fn execute_arbitrage<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage<'info>>,
        params: ArbitrageParams,
    ) -> Result<()> {
        instructions::execute_arbitrage::execute_arbitrage(ctx, params)
    }

    pub fn execute_arbitrage_v4<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrageV4<'info>>,
        params: V4ArbParams,
    ) -> Result<()> {
        instructions::execute_arbitrage_v4::execute_arbitrage_v4(ctx, params)
    }
}