#![allow(deprecated)]

use anchor_lang::prelude::*;

/// devnet 部署命令
///
/// 0) 前置：三处 Program ID 一致
///    - src/lib.rs: declare_id!("<ProgramID>")
///    - Anchor.toml: [programs.devnet].arbitrage_contract = "<ProgramID>"
///    - 客户端常量：ARBITRAGE_CONTRACT_ID = "<ProgramID>"
///
/// 1) 显式切换到 devnet 与钱包。不要把个人钱包绝对路径提交到仓库。
/// ```text
///    solana config set --url https://api.devnet.solana.com
///    solana config set -k <DEVNET_WALLET_KEYPAIR>
/// ```
///
/// 2) 构建（带 devnet 特性）
/// ```text
///    anchor build -- --features devnet
/// ```
///
/// 3) 部署
/// ```text
///    anchor deploy --program-name arbitrage_contract \
///                  --program-keypair <DEVNET_PROGRAM_KEYPAIR> \
///                  --provider.cluster devnet
/// ```
///
/// 4) 验证（可选）
/// ```text
///    solana program show <DEVNET_PROGRAM_ID>
/// ```
///
/// mainnet 部署命令
///
/// 0) 前置：三处 Program ID 一致
///    - src/lib.rs: declare_id!("<MAINNET_PROGRAM_ID>")
///    - Anchor.toml: [programs.mainnet].arbitrage_contract = "<MAINNET_PROGRAM_ID>"
///    - 客户端常量：ARBITRAGE_CONTRACT_ID = "<MAINNET_PROGRAM_ID>"
///
/// 1) 设置主网与钱包。主网部署必须通过显式环境变量或命令行参数传入。
/// ```text
///    solana config set --url https://api.mainnet-beta.solana.com
///    solana config set -k <MAINNET_WALLET_KEYPAIR>
/// ```
///
/// 2) 构建（不要加 devnet 特性）
/// ```text
///    anchor build
/// ```
///
/// 3) 部署（Anchor 0.31 需指定 program-name）
/// ```text
///    scripts/preflight_deploy.sh --cluster mainnet \
///                                --provider-cluster mainnet \
///                                --program-keypair <MAINNET_PROGRAM_KEYPAIR> \
///                                --wallet <MAINNET_WALLET_KEYPAIR>
///
///    ANCHOR_PROVIDER_URL=https://api.mainnet-beta.solana.com \
///    ANCHOR_WALLET=<MAINNET_WALLET_KEYPAIR> \
///    anchor deploy --program-name arbitrage_contract \
///                  --program-keypair <MAINNET_PROGRAM_KEYPAIR>
/// ```
pub mod errors;
pub mod instructions;
pub mod protocal;
pub mod state;

pub use state::*;

// declare_id!("BnvW7qAWpur6tcVHsmkgRjiH1orAc7ERL6aVymBPpNEB");
declare_id!("9Vj7SrxY3mQdw48xZ29r53ZA2rjNiniTA7KNsjH8ubkK");

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
    const PRODUCTION_SOURCES: &[(&str, &str)] = &[
        ("src/errors.rs", include_str!("errors.rs")),
        (
            "src/instructions/accounts.rs",
            include_str!("instructions/accounts.rs"),
        ),
        (
            "src/instructions/execute_arbitrage.rs",
            include_str!("instructions/execute_arbitrage.rs"),
        ),
        (
            "src/instructions/mod.rs",
            include_str!("instructions/mod.rs"),
        ),
        (
            "src/instructions/program_ids.rs",
            include_str!("instructions/program_ids.rs"),
        ),
        (
            "src/instructions/types.rs",
            include_str!("instructions/types.rs"),
        ),
        ("src/lib.rs", include_str!("lib.rs")),
        ("src/protocal/mod.rs", include_str!("protocal/mod.rs")),
        (
            "src/protocal/gamma_swap.rs",
            include_str!("protocal/gamma_swap.rs"),
        ),
        (
            "src/protocal/orca_whirlpool.rs",
            include_str!("protocal/orca_whirlpool.rs"),
        ),
        (
            "src/protocal/meteora_dlmm.rs",
            include_str!("protocal/meteora_dlmm.rs"),
        ),
        (
            "src/protocal/pumpfun_amm.rs",
            include_str!("protocal/pumpfun_amm.rs"),
        ),
        (
            "src/protocal/pumpfun_swap.rs",
            include_str!("protocal/pumpfun_swap.rs"),
        ),
        (
            "src/protocal/raydium_clmm.rs",
            include_str!("protocal/raydium_clmm.rs"),
        ),
        (
            "src/protocal/raydium_cpmm.rs",
            include_str!("protocal/raydium_cpmm.rs"),
        ),
        (
            "src/protocal/raydium_launchpad.rs",
            include_str!("protocal/raydium_launchpad.rs"),
        ),
        (
            "src/protocal/raydium_pool_v4.rs",
            include_str!("protocal/raydium_pool_v4.rs"),
        ),
        (
            "src/protocal/stabble_swap.rs",
            include_str!("protocal/stabble_swap.rs"),
        ),
        ("src/state.rs", include_str!("state.rs")),
    ];

    const FORBIDDEN_PRODUCTION_PATTERNS: &[&str] =
        &[".unwrap(", ".expect(", "panic!(", ".unwrap_or"];

    #[test]
    fn production_sources_do_not_use_panicking_or_fallback_helpers() {
        let violations = PRODUCTION_SOURCES
            .iter()
            .flat_map(|(path, source)| {
                let production = production_source_text(source);
                FORBIDDEN_PRODUCTION_PATTERNS
                    .iter()
                    .flat_map(move |pattern| pattern_violations(path, production, pattern))
            })
            .collect::<Vec<_>>();

        assert!(
            violations.is_empty(),
            "production source must use explicit Result handling instead of panic/unwrap/fallback helpers: {:?}",
            violations
        );
    }

    fn production_source_text(source: &str) -> &str {
        match source.split_once("\n#[cfg(test)]") {
            Some((production, _)) => production,
            None => source,
        }
    }

    fn pattern_violations(path: &str, source: &str, pattern: &str) -> Vec<String> {
        source
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains(pattern))
            .map(|(idx, _)| format!("{path}:{}:{pattern}", idx + 1))
            .collect()
    }
}
