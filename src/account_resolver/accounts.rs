use anchor_lang::prelude::*;

/// Raydium CPMM账户 - 仅包含客户端传递的7个账户
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
    
    // 注意: 以下账户由合约内推导，不在此结构体中：
    // - cp_swap_program (推导)
    // - authority (推导) 
    // - user_input_ata (推导)
    // - user_output_ata (推导)
    // - input_token_program (推导)
    // - output_token_program (推导)
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

/// PumpFun账户 - 仅包含客户端传递的3个账户  
/// 对应客户端 SmartAccountCollector.collect_pumpfun() 的传递顺序
#[derive(Clone)]
pub struct PumpfunAccounts<'info> {
    pub bonding_curve: &'info AccountInfo<'info>,      // 1. bonding_curve地址 (pool_id)
    pub mint: &'info AccountInfo<'info>,               // 2. 代币mint
    pub creator: &'info AccountInfo<'info>,            // 3. 创建者地址
    
    // 注意: 以下账户由合约内推导，不在此结构体中：
    // - user_input_ata (推导)
    // - user_output_ata (推导) 
    // - global_account (推导, 固定地址)
    // - fee_recipient (推导, 固定地址)
    // - associated_bonding_curve (推导)
    // - event_authority (推导)
    // - creator_vault_pda (推导)
    // - global_volume_accumulator (推导)
    // - user_volume_accumulator (推导)
    // - system_program (推导, 固定地址)
    // - token_program (推导, 固定地址)
}

/// PumpSwap账户 - 仅包含客户端传递的4个账户
/// 对应客户端 SmartAccountCollector.collect_pumpswap() 的传递顺序  
#[derive(Clone)]
pub struct PumpswapAccounts<'info> {
    pub pool_state: &'info AccountInfo<'info>,         // 1. 池地址 (pool_id)
    pub base_mint: &'info AccountInfo<'info>,          // 2. 基础代币mint
    pub quote_mint: &'info AccountInfo<'info>,         // 3. 计价代币mint
    pub coin_creator: &'info AccountInfo<'info>,       // 4. 代币创建者
    
    // 注意: 以下账户由合约内推导，不在此结构体中：
    // - user_base_ata (推导)
    // - user_quote_ata (推导)
    // - pool_base_token_account (推导)
    // - pool_quote_token_account (推导)
    // - global_config (推导)
    // - fee_recipient (推导, 固定地址)
    // - fee_recipient_ata (推导)
    // - event_authority (推导)
    // - amm_program (推导, 固定地址)
    // - coin_creator_vault_authority (推导)
    // - coin_creator_vault_ata (推导)
    // - system_program (推导)
    // - token_program (推导)
    // - associated_token_program (推导)
    // - volume_accumulators (条件推导)
}