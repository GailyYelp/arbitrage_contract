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
///    anchor deploy --program-name arbitrage_contract --program-keypair /Users/springbugs/.solana/accounts/program-keypair.json --provider.cluster devnet
///
/// 4) 验证（可选）
///    solana program show 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC
///
/// mainnet 部署命令
///
/// 0) 前置：三处 Program ID 一致
///    - src/lib.rs: declare_id!("<MAINNET_PROGRAM_ID>")
///    - Anchor.toml: [programs.mainnet].arbitrage_contract = "<MAINNET_PROGRAM_ID>"
///    - 客户端常量：ARBITRAGE_CONTRACT_ID = "<MAINNET_PROGRAM_ID>"
///
/// 1) 设置主网与钱包
///    solana config set --url https://api.mainnet-beta.solana.com
///    solana config set -k /Users/zhengwei/Desktop/wallet-keypair-mainnet.json
///    ###
///    solana config set -k /Users/springbugs/.solana/accounts/3n9fuagU7XgahE6skJNEt5njDbyamiG8oT5V9gMZ83vE.json
///
/// 2) 构建（不要加 devnet 特性）
///    anchor build
///
/// 3) 部署（Anchor 0.31 需指定 program-name）
///    ANCHOR_PROVIDER_URL=https://api.mainnet-beta.solana.com \
///    ANCHOR_WALLET=/Users/zhengwei/Desktop/wall-keypair-mainnet.json \
///    anchor deploy --program-name arbitrage_contract \
///                  --program-keypair target/deploy/arbitrage_contract-mainnet-keypair.json
///
// ANCHOR_PROVIDER_URL=https://api.mainnet-beta.solana.com \
// ANCHOR_WALLET=/Users/springbugs/.solana/accounts/3n9fuagU7XgahE6skJNEt5njDbyamiG8oT5V9gMZ83vE.json \
// anchor deploy --program-name arbitrage_contract \
//               --program-keypair /Users/springbugs/.solana/accounts/BnvW7qAWpur6tcVHsmkgRjiH1orAc7ERL6aVymBPpNEB.json \
//               --provider.wallet /Users/springbugs/.solana/accounts/3n9fuagU7XgahE6skJNEt5njDbyamiG8oT5V9gMZ83vE.json \
//               --provider.cluster mainnet
pub mod errors;
pub mod instructions;
pub mod protocal;
pub mod state;

pub use state::*;

declare_id!("BnvW7qAWpur6tcVHsmkgRjiH1orAc7ERL6aVymBPpNEB");
// declare_id!("9Vj7SrxY3mQdw48xZ29r53ZA2rjNiniTA7KNsjH8ubkK");

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

#[cfg(test)]
mod tests {
    use solana_keypair::Keypair;

    #[test]
    fn generate_secret_bin_from_base58() {
        const BASE58_KEYPAIR: &str = "";
        if BASE58_KEYPAIR.is_empty() {
            return;
        }

        let keypair = Keypair::from_base58_string(BASE58_KEYPAIR);

        let secret_bytes = keypair.to_bytes();

        println!("secret_bytes: {:?}", secret_bytes);
    }
}
