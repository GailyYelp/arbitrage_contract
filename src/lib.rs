use anchor_lang::prelude::*;

/// Program ID（declare_id）生成与配置指引
///
/// 1) 生成 Program ID（dev/test/main 可分别生成）
/// - 生成密钥对（示例输出到 target/deploy/）：
///   ```bash
///   solana-keygen new -o target/deploy/arbitrage_contract-devnet-keypair.json -s --no-bip39-passphrase --force
///   solana-keygen new -o target/deploy/arbitrage_contract-testnet-keypair.json -s --no-bip39-passphrase --force
///   solana-keygen new -o target/deploy/arbitrage_contract-mainnet-keypair.json -s --no-bip39-passphrase --force
///   ```
/// - 查看 Program ID（公钥）：
///   ```bash
///   solana-keygen pubkey target/deploy/arbitrage_contract-devnet-keypair.json
///   solana-keygen pubkey target/deploy/arbitrage_contract-testnet-keypair.json
///   solana-keygen pubkey target/deploy/arbitrage_contract-mainnet-keypair.json
///   # 或使用 Anchor 汇总
///   anchor keys list
///   ```
///
/// 2) 配置位置（必须三处一致）
/// - 合约：`src/lib.rs` 的 `declare_id!("<ProgramID>")`
/// - Anchor：`Anchor.toml` 的对应网络段
///   ```toml
///   [programs.devnet]
///   arbitrage_contract = "<DevnetProgramID>"
///
///   [programs.testnet]
///   arbitrage_contract = "<TestnetProgramID>"
///
///   [programs.mainnet]
///   arbitrage_contract = "<MainnetProgramID>"
///
///   [provider]
///   cluster = "devnet"   # 或 "testnet"/"mainnet"
///   wallet  = "~/.config/solana/id.json"
///   ```
/// - 客户端：将 `ARBITRAGE_CONTRACT_ID`（或等价常量）设置为对应网络的 Program ID
///
/// 3) 部署流程（示例）
/// ```bash
/// solana config set --url devnet        # 或 mainnet-beta/testnet
/// anchor build
/// anchor deploy                         # 使用 Anchor.toml 的 [provider]
/// ```
///
/// 4) 策略建议
/// - 复用一套 Program ID 跨网络：省去改 `declare_id!`；各网络部署同一 ID 的程序
/// - 每网独立 Program ID：更隔离，但切换网络前需同步修改 `declare_id!`、`Anchor.toml` 与客户端常量，并用对应 keypair 部署
///
/// 5) 升级注意
/// - 升级（`anchor upgrade`）必须使用最初部署该 Program ID 的私钥；请妥善保管 keypair
/// - 若丢失私钥，将无法继续升级该 Program ID 下的程序

pub mod instructions;
pub mod state;
pub mod errors;
pub mod account_resolver;
pub mod account_derivation;
pub mod dex_router;

pub use instructions::*;
pub use state::*;
pub use errors::*;
pub use account_resolver::*;
pub use account_derivation::*;
pub use dex_router::*;

declare_id!("4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC");

#[program]
pub mod arbitrage_contract {
    use super::*;
    
    pub fn execute_arbitrage<'info>(
        ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage<'info>>,
        params: ArbitrageParams,
    ) -> Result<()> {
        instructions::execute_arbitrage(ctx, params)
    }
}