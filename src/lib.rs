use anchor_lang::prelude::*;

/// devnet 部署命令
///
/// 0) 前置：三处 Program ID 一致
///    - src/lib.rs: declare_id!("<ProgramID>")
///    - Anchor.toml: [programs.devnet].arbitrage_contract = "<ProgramID>"
///    - 客户端常量：ARBITRAGE_CONTRACT_ID = "<ProgramID>"
///
/// 1) 显式切换到 devnet 与钱包
///    solana config set --url https://api.devnet.solana.com
///    solana config set -k /Users/zhengwei/Desktop/wallet-keypair.json
///
/// 2) 构建（带 devnet 特性）
///    anchor build -- --features devnet
///
/// 3) 部署
///    anchor deploy
///
/// 4) 验证（可选）
///    solana program show 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC
///
pub mod errors;
pub mod instructions;
pub mod protocal;
pub mod state;

pub use state::*;

declare_id!("4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC");

#[derive(Accounts)]
pub struct ExecuteArbitrage {}

#[program]
pub mod arbitrage_contract {
    use super::*;

    pub fn execute_arbitrage<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage>,
        params: SwapArbParams,
    ) -> Result<()> {
        instructions::execute_arbitrage::execute_arbitrage(ctx, params)
    }
}
