use anchor_lang::prelude::*;
use std::str::FromStr;

#[derive(Clone)]
pub struct ProgramIds {
    // Core DEX Programs
    pub raydium_cpmm: Pubkey,
    pub raydium_clmm: Pubkey,
    pub pumpfun: Pubkey,
    pub pumpswap: Pubkey,
    
    // System Programs
    pub token_program: Pubkey,
    pub token_2022_program: Pubkey,
    pub associated_token_program: Pubkey,
    pub memo_program: Pubkey,
    pub system_program: Pubkey,
}

impl Default for ProgramIds {
    fn default() -> Self {
        // 允许通过环境变量覆盖（构建期通过 RUSTFLAGS/anchor/env 注入）
        fn env_pk(name: &str, default: &str) -> Pubkey {
            if let Ok(v) = std::env::var(name) { Pubkey::from_str(&v).unwrap_or(Pubkey::from_str(default).unwrap()) }
            else { Pubkey::from_str(default).unwrap() }
        }
        // mainnet 默认，允许 ENV 覆盖
        let is_devnet = cfg!(feature = "devnet");
        let (cpmm_def, clmm_def, pumpfun_def, pumpswap_def) = if is_devnet {
            (
                "CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW",
                "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
                "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
            )
        } else {
            (
                "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
                "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
                "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
            )
        };
        Self {
            // Core DEX Programs（可被 ENV 覆盖）
            raydium_cpmm: env_pk("PROGRAM_ID_RAYDIUM_CPMM", cpmm_def),
            raydium_clmm: env_pk("PROGRAM_ID_RAYDIUM_CLMM", clmm_def),
            pumpfun: env_pk("PROGRAM_ID_PUMPFUN", pumpfun_def),
            pumpswap: env_pk("PROGRAM_ID_PUMPSWAP", pumpswap_def),
            // System Programs
            token_program: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
            token_2022_program: Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap(),
            associated_token_program: Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap(),
            memo_program: Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap(),
            system_program: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
        }
    }
}

impl ProgramIds {
    pub fn new(
        raydium_cpmm: Pubkey,
        raydium_clmm: Pubkey,
        pumpfun: Pubkey,
        pumpswap: Pubkey,
    ) -> Self {
        Self {
            raydium_cpmm,
            raydium_clmm,
            pumpfun,
            pumpswap,
            token_program: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
            token_2022_program: Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap(),
            associated_token_program: Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap(),
            memo_program: Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap(),
            system_program: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
        }
    }
}

/// 固定账户地址常量 - 仅包含4个核心DEX的地址
pub mod fixed_addresses {
    // ==============================================
    // Raydium CPMM 固定地址（按网络切换，ENV 可覆盖）
    // ==============================================
    #[cfg(feature = "devnet")]
    pub const RAYDIUM_CPMM_AUTHORITY: &str = "7rQ1QFNosMkUCuh7Z7fPbTHvh73b68sQYdirycEzJVuw";
    #[cfg(not(feature = "devnet"))]
    pub const RAYDIUM_CPMM_AUTHORITY: &str = "GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL";
    
    // ==============================================
    // Raydium CLMM 固定地址 (从instruction文件中提取)
    // ==============================================
    // 无额外固定地址，主要通过PDA推导
    
    // ==============================================
    // PumpFun Bonding Curve 固定地址
    // ==============================================
    pub const PUMPFUN_GLOBAL_ACCOUNT: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
    pub const PUMPFUN_FEE_RECIPIENT: &str = "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV";
    pub const PUMPFUN_EVENT_AUTHORITY: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
    
    // ==============================================
    // PumpSwap AMM 固定地址（若 devnet 不同，请用 ENV 覆盖）
    // ==============================================
    pub const PUMPSWAP_GLOBAL_CONFIG: &str = "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw";
    pub const PUMPSWAP_FEE_RECIPIENT: &str = "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV";
    pub const PUMPSWAP_FEE_RECIPIENT_ATA: &str = "94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb";
    pub const PUMPSWAP_EVENT_AUTHORITY: &str = "GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR";
    pub const PUMPSWAP_AMM_PROGRAM: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"; // AMM程序地址
    
    // ==============================================
    // 代币相关
    // ==============================================
    pub const WRAPPED_SOL_MINT: &str = "So11111111111111111111111111111111111111112";
}

/// 指令选择器常量 - 用于构造CPI调用
pub mod instruction_discriminators {
    // Raydium CPMM
    pub const RAYDIUM_CPMM_SWAP_BASE_IN: &[u8; 8] = &[143, 190, 90, 218, 196, 30, 51, 222];
    
    // Raydium CLMM  
    pub const RAYDIUM_CLMM_SWAP_V2: &[u8; 8] = &[43, 4, 237, 11, 26, 201, 30, 98];
    
    // PumpFun
    pub const PUMPFUN_BUY: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
    pub const PUMPFUN_SELL: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
    
    // PumpSwap
    pub const PUMPSWAP_BUY: &[u8; 8] = &[102, 6, 61, 18, 1, 218, 235, 234];
    pub const PUMPSWAP_SELL: &[u8; 8] = &[51, 230, 133, 164, 1, 127, 131, 173];
}

/// PDA种子常量 - 用于账户推导
pub mod pda_seeds {
    // PumpFun PDA种子
    pub const PUMPFUN_GLOBAL: &[u8] = b"global";
    pub const PUMPFUN_BONDING_CURVE: &[u8] = b"bonding-curve";
    pub const PUMPFUN_CREATOR_VAULT: &[u8] = b"creator-vault";
    pub const PUMPFUN_MINT_AUTHORITY: &[u8] = b"mint-authority";
    pub const PUMPFUN_EVENT_AUTHORITY: &[u8] = b"__event_authority";
    pub const PUMPFUN_GLOBAL_VOLUME_ACCUMULATOR: &[u8] = b"global_volume_accumulator";
    pub const PUMPFUN_USER_VOLUME_ACCUMULATOR: &[u8] = b"user_volume_accumulator";
    
    // PumpSwap PDA种子
    pub const PUMPSWAP_GLOBAL_CONFIG: &[u8] = b"global_config";
    pub const PUMPSWAP_POOL: &[u8] = b"pool";
    pub const PUMPSWAP_LP_MINT: &[u8] = b"pool_lp_mint";
    pub const PUMPSWAP_CREATOR_VAULT: &[u8] = b"creator_vault";
    pub const PUMPSWAP_EVENT_AUTHORITY: &[u8] = b"__event_authority";
    pub const PUMPSWAP_GLOBAL_VOLUME_ACCUMULATOR: &[u8] = b"global_volume_accumulator";
    pub const PUMPSWAP_USER_VOLUME_ACCUMULATOR: &[u8] = b"user_volume_accumulator";
}

/// 获取所有固定地址
pub fn get_fixed_addresses() -> Result<FixedAddresses> {
    use crate::errors::ArbitrageError;
    
    macro_rules! parse_pubkey {
        ($addr:expr) => {
            Pubkey::from_str($addr).map_err(|_| ArbitrageError::InvalidPublicKey)?
        };
    }
    
    // 允许通过 ENV 覆盖固定地址（例如在 devnet 下）
    fn env_or(name: &str, default_addr: &str) -> Result<Pubkey> {
        if let Ok(v) = std::env::var(name) { return Ok(Pubkey::from_str(&v).map_err(|_| ArbitrageError::InvalidPublicKey)?); }
        Ok(Pubkey::from_str(default_addr).map_err(|_| ArbitrageError::InvalidPublicKey)?)
    }

    Ok(FixedAddresses {
        // Raydium CPMM
        raydium_cpmm_authority: env_or("ADDR_RAYDIUM_CPMM_AUTHORITY", fixed_addresses::RAYDIUM_CPMM_AUTHORITY)?,
        
        // PumpFun
        pumpfun_global_account: env_or("ADDR_PUMPFUN_GLOBAL", fixed_addresses::PUMPFUN_GLOBAL_ACCOUNT)?,
        pumpfun_fee_recipient: env_or("ADDR_PUMPFUN_FEE_RECIPIENT", fixed_addresses::PUMPFUN_FEE_RECIPIENT)?,
        pumpfun_event_authority: env_or("ADDR_PUMPFUN_EVENT_AUTHORITY", fixed_addresses::PUMPFUN_EVENT_AUTHORITY)?,
        
        // PumpSwap
        pumpswap_global_config: env_or("ADDR_PUMPSWAP_GLOBAL_CONFIG", fixed_addresses::PUMPSWAP_GLOBAL_CONFIG)?,
        pumpswap_fee_recipient: env_or("ADDR_PUMPSWAP_FEE_RECIPIENT", fixed_addresses::PUMPSWAP_FEE_RECIPIENT)?,
        pumpswap_fee_recipient_ata: env_or("ADDR_PUMPSWAP_FEE_RECIPIENT_ATA", fixed_addresses::PUMPSWAP_FEE_RECIPIENT_ATA)?,
        pumpswap_event_authority: env_or("ADDR_PUMPSWAP_EVENT_AUTHORITY", fixed_addresses::PUMPSWAP_EVENT_AUTHORITY)?,
        pumpswap_amm_program: env_or("ADDR_PUMPSWAP_AMM_PROGRAM", fixed_addresses::PUMPSWAP_AMM_PROGRAM)?,
        
        // 代币
        wrapped_sol_mint: parse_pubkey!(fixed_addresses::WRAPPED_SOL_MINT),
    })
}

#[derive(Clone)]
pub struct FixedAddresses {
    // Raydium CPMM 固定地址
    pub raydium_cpmm_authority: Pubkey,
    
    // PumpFun 固定地址
    pub pumpfun_global_account: Pubkey,
    pub pumpfun_fee_recipient: Pubkey,
    pub pumpfun_event_authority: Pubkey,
    
    // PumpSwap 固定地址
    pub pumpswap_global_config: Pubkey,
    pub pumpswap_fee_recipient: Pubkey,
    pub pumpswap_fee_recipient_ata: Pubkey,
    pub pumpswap_event_authority: Pubkey,
    pub pumpswap_amm_program: Pubkey,
    
    // 代币相关
    pub wrapped_sol_mint: Pubkey,
}

/// PDA推导辅助函数
pub mod pda_utils {
    use super::*;
    
    /// 推导PumpFun bonding curve PDA
    pub fn derive_pumpfun_bonding_curve(mint: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_BONDING_CURVE, mint.as_ref()],
            program_id,
        );
        Ok(pda)
    }
    
    /// 推导PumpFun creator vault PDA
    pub fn derive_pumpfun_creator_vault(creator: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_CREATOR_VAULT, creator.as_ref()],
            program_id,
        );
        Ok(pda)
    }
    
    /// 推导PumpFun global volume accumulator PDA
    pub fn derive_pumpfun_global_volume_accumulator(program_id: &Pubkey) -> Result<Pubkey> {
        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_GLOBAL_VOLUME_ACCUMULATOR],
            program_id,
        );
        Ok(pda)
    }
    
    /// 推导PumpFun user volume accumulator PDA
    pub fn derive_pumpfun_user_volume_accumulator(user: &Pubkey, program_id: &Pubkey) -> Result<Pubkey> {
        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_USER_VOLUME_ACCUMULATOR, user.as_ref()],
            program_id,
        );
        Ok(pda)
    }
    
    /// 推导PumpSwap global config PDA
    pub fn derive_pumpswap_global_config(program_id: &Pubkey) -> Result<Pubkey> {
        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPSWAP_GLOBAL_CONFIG],
            program_id,
        );
        Ok(pda)
    }
    
    /// 推导PumpSwap creator vault PDA
    pub fn derive_pumpswap_creator_vault(creator: &Pubkey, amm_program: &Pubkey) -> Result<Pubkey> {
        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPSWAP_CREATOR_VAULT, creator.as_ref()],
            amm_program,
        );
        Ok(pda)
    }
}