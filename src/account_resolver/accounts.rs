use anchor_lang::prelude::*;

/// 账户结构定义（V2 协议 - 客户端最小集 + 合约推导项说明）
/// 用途：
/// - 表达各 DEX 在 V2 协议下客户端需要显式传入的最小账户集合；
/// - 注明哪些账户在合约内可通过 PDA/固定地址/ATA 推导，避免重复传参；
/// - 对齐客户端 indices 顺序常量，便于双端一致。
/// 说明：以下结构体仅包含“indices 最小集”。CPI 还需要的其它账户由客户端追加到全局表 remaining_accounts（不计入 indices），
/// 合约仅计算期望值/选择正确 token program 并在全局表中定位/校验/记录日志，不负责“补账户”。
/// Raydium CPMM账户 - 仅包含客户端传递的7个账户（indices）
/// 对应客户端 SmartAccountCollector.collect_raydium_cpmm() 的传递顺序
#[derive(Clone)]
pub struct RaydiumCpmmAccounts<'info> {
    pub amm_config: &'info AccountInfo<'info>,         // 1. AMM配置地址
    pub pool_state: &'info AccountInfo<'info>,         // 2. 池地址 (pool_id)  
    pub token0_vault: &'info AccountInfo<'info>,       // 3. token0金库
    pub token1_vault: &'info AccountInfo<'info>,       // 4. token1金库
    pub input_mint: &'info AccountInfo<'info>,         // 5. 输入代币mint
    pub output_mint: &'info AccountInfo<'info>,        // 6. 输出代币mint
    pub observation_state: &'info AccountInfo<'info>,  // 7. observation账户
    
    // 注意：以下账户不在 indices 最小集中：
    // - cpmm_program（客户端追加到全局表；CPI 需要的外部程序账户）
    // - authority（固定地址；链上计算期望值用于定位/校验）
    // - user_input_ata、user_output_ata（客户端追加到全局表；用于余额差与转账）
    // - input_token_program、output_token_program（链上依据 mint.owner 选择；AccountInfo 来源于入口 token_program 或全局表）
}

#[derive(Clone)]
pub struct RaydiumClmmAccounts<'info> {
    pub clmm_program: &'info AccountInfo<'info>,
    pub amm_config: &'info AccountInfo<'info>,
    pub pool_state: &'info AccountInfo<'info>,
    pub input_vault: &'info AccountInfo<'info>,
    pub output_vault: &'info AccountInfo<'info>,
    pub observation_state: &'info AccountInfo<'info>,
    pub token_program: &'info AccountInfo<'info>,
    pub token_program_2022: &'info AccountInfo<'info>,
    pub memo_program: &'info AccountInfo<'info>,
    pub input_vault_mint: &'info AccountInfo<'info>,
    pub output_vault_mint: &'info AccountInfo<'info>,
}

// 说明：CLMM 的 tick arrays 与 tick_array_extension 不在 indices 最小集中，需由客户端追加到全局表；
// 合约在执行前按 owner == clmm_program 从全局表动态注入到 CPI metas。

/// PumpFun账户 - 仅包含客户端传递的3个账户（indices）  
/// 对应客户端 SmartAccountCollector.collect_pumpfun() 的传递顺序
#[derive(Clone)]
pub struct PumpfunAccounts<'info> {
    pub bonding_curve: &'info AccountInfo<'info>,      // 1. bonding_curve地址 (pool_id)
    pub mint: &'info AccountInfo<'info>,               // 2. 代币mint
    pub creator: &'info AccountInfo<'info>,            // 3. 创建者地址
    // 可选扩展：indices 可追加 fee_recipient（若提供则优先使用）
    pub fee_recipient_opt: Option<&'info AccountInfo<'info>>, 
    
    // 注意：以下账户不在 indices 最小集中（需客户端追加到全局表，合约仅做期望值定位/校验/日志）：
    // - program、global_account、fee_recipient（可选由 indices 指定）、event_authority、rent
    // - associated_bonding_curve（可计算期望值，用于定位 AccountInfo）
    // - creator_vault_pda（可计算期望值，用于定位 AccountInfo）
    // - （买入）global/user volume accumulators（可选存在，尽力定位）
    // - user_input_ata、user_output_ata（用于余额差；由客户端追加）
    // - system_program、token_program（入口固定账户/全局表提供）
}

/// PumpSwap账户 - 仅包含客户端传递的4个账户（indices）
/// 对应客户端 SmartAccountCollector.collect_pumpswap() 的传递顺序  
#[derive(Clone)]
pub struct PumpswapAccounts<'info> {
    pub pool_state: &'info AccountInfo<'info>,         // 1. 池地址 (pool_id)
    pub base_mint: &'info AccountInfo<'info>,          // 2. 基础代币mint
    pub quote_mint: &'info AccountInfo<'info>,         // 3. 计价代币mint
    pub coin_creator: &'info AccountInfo<'info>,       // 4. 代币创建者
    // 可选扩展：indices 可追加 fee_recipient 与 fee_recipient_ata（若提供则优先使用）
    pub fee_recipient_opt: Option<&'info AccountInfo<'info>>,
    pub fee_recipient_ata_opt: Option<&'info AccountInfo<'info>>,
    
    // 注意：以下账户不在 indices 最小集中：
    // - global_config、event_authority、amm_program（客户端追加到全局表；其中 amm_program 需可执行校验）
    // - fee_recipient（可选由 indices 追加或客户端在全局表提供）、fee_recipient_ata（同上）
    // - user_base_ata、user_quote_ata、pool_base_ata、pool_quote_ata（客户端追加；合约可通过 owner+mint 扫描定位）
    // - coin_creator_vault_authority（可计算期望值；在全局表中定位）
    // - coin_creator_vault_ata（客户端追加；或通过 owner+mint 扫描定位）
    // - system_program、token_program、associated_token_program（入口固定账户/全局表提供）
    // - volume_accumulators（若协议使用则追加；合约尽力定位，不强依赖）
}