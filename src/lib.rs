use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token_2022::Token2022;

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

pub mod errors;
pub mod instructions;
pub mod protocal;
pub mod state;

pub use state::*;

declare_id!("4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC");

#[derive(Accounts)]
pub struct ExecuteArbitrage<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    // 顺序按规范：system -> associated_token -> token -> token_2022
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub token_2022_program: Program<'info, Token2022>,
}

#[program]
pub mod arbitrage_contract {
    use super::*;

    pub fn execute_arbitrage<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage<'info>>,
        params: SwapArbParams,
    ) -> Result<()> {
        instructions::execute_arbitrage::execute_arbitrage(ctx, params)
    }
}
