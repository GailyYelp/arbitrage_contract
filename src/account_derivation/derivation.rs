use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use crate::state::{DexType, PathStep};
use super::types::{ProgramIds, FixedAddresses, get_fixed_addresses, pda_seeds};
use std::collections::HashMap;

/// 账户推导引擎（V2 协议）
///
/// 目标：在“最小必需客户端账户（indices + 全局表）”基础上，链上统一推导“可确定”的账户，
/// 包括用户 ATAs、Token/Token-2022 程序选择、部分固定地址与 PDA，降低客户端负担并提升一致性。
///
/// 流程要点：
/// 1) initialize() 装载固定地址与系统程序；
/// 2) derive_for_path():
///    - 基于 remaining_accounts 自动识别每个 mint 的 token program（Token/Token-2022）；
///    - 为路径涉及的所有 mint 推导用户 ATAs 并缓存；
///    - 按 DEX 类型推导必要 PDA/固定账户（如 CPMM authority、Pump 系列 PDA 等）。
/// 3) 执行阶段：从缓存读取用户 ATAs/固定地址，配合 AccountResolver 解析出的 DEX 最小集 + 动态补充账户组装 CPI。
/// 注意：本模块不负责将账户加入 remaining_accounts，也不做强制校验，仅做推导与缓存（用于定位/日志）。
///
/// 每个 DEX 的“链上推导 vs 客户端传入”：
/// - Raydium CPMM
///   链上推导：authority（固定地址）、用户 ATAs、每个 mint 的 token program 选择（用于派生 ATA）。
///   客户端传入（indices）：amm_config、pool_state、token0_vault、token1_vault、input_mint、output_mint、observation_state；
///   说明：CPI metas 中 token_program 重复位来自外部传入（合约入口的 token_program）。
///
/// - Raydium CLMM
///   链上推导：用户 ATAs、每个 mint 的 token program 选择（用于派生 ATA）。
///   客户端传入（indices 基础 11 项）：clmm_program、amm_config、pool_state、input_vault、output_vault、
///     observation_state、token_program、token_program_2022、memo_program、input_vault_mint、output_vault_mint；
///   客户端追加（不计入 indices）：tick_array_extension、tick arrays（动态）；
///   合约在 CPI 前按 owner==clmm_program 动态注入上述追加账户到 metas（顺序沿用全局表）。
///
/// - PumpFun（Bonding Curve）
///   链上推导：bonding_curve PDA（mint）、associated_bonding_curve（bonding_curve+mint）、
///     （可选）creator_vault、（可选）volume accumulators（global/user，买入时）。
///   客户端传入（indices）：bonding_curve(pool_id)、mint、creator；
///   客户端追加（全局表）：program、global、fee_recipient、event_authority、rent、associated_bonding_curve 等。
///
/// - PumpSwap AMM
///   链上推导：global_config PDA、pool/user 双边 ATAs、fee_recipient_ata、creator_vault_authority PDA 及其 ATA、
///     每个 mint 的 token program 选择（用于派生 ATA）。
///   客户端传入（indices）：pool_state、base_mint、quote_mint、coin_creator；
///   客户端追加（全局表）：program、global_config、fee_recipient、fee_recipient_ata、event_authority、amm_program、
///     creator_vault_ata 等。
///
/// 缓存策略（单次指令内存级）：
/// - user_token_accounts: mint -> user_ata；
/// - token_programs: mint -> token_program_id（mint.owner 自动识别后缓存）；
/// - 各 DEX 推导缓存（如 Pump 系列 PDA）与 fixed_addresses；
/// - 执行时从缓存取 Pubkey，再在 remaining_accounts 中查找 AccountInfo 参与 CPI。

/// 完整的账户推导引擎
pub struct DerivedAccounts {
    // 基础缓存
    pub user_token_accounts: HashMap<Pubkey, Pubkey>,  // mint -> user_ata
    pub token_programs: HashMap<Pubkey, Pubkey>,       // mint -> token_program_id
    
    // DEX特定账户缓存
    pub raydium_accounts: HashMap<String, Pubkey>,
    pub pumpfun_accounts: HashMap<String, Pubkey>, 
    pub pumpswap_accounts: HashMap<String, Pubkey>,
    
    // 系统程序和固定地址
    pub system_programs: HashMap<String, Pubkey>,
    pub fixed_addresses: Option<FixedAddresses>,
}

impl DerivedAccounts {
    pub fn new() -> Self {
        Self {
            user_token_accounts: HashMap::new(),
            token_programs: HashMap::new(),
            raydium_accounts: HashMap::new(),
            pumpfun_accounts: HashMap::new(),
            pumpswap_accounts: HashMap::new(),
            system_programs: HashMap::new(),
            fixed_addresses: None,
        }
    }

    /// 初始化系统
    pub fn initialize(&mut self, program_ids: &ProgramIds) -> Result<()> {
        self.fixed_addresses = Some(get_fixed_addresses()?);
        
        self.system_programs.insert("token_program".to_string(), program_ids.token_program);
        self.system_programs.insert("token_2022_program".to_string(), program_ids.token_2022_program);
        self.system_programs.insert("associated_token_program".to_string(), program_ids.associated_token_program);
        self.system_programs.insert("system_program".to_string(), program_ids.system_program);
        
        Ok(())
    }

    // ================================================================
    // 基础工具函数
    // ================================================================

    /// 推导用户ATA账户
    pub fn derive_user_ata(&mut self, user: &Pubkey, mint: &Pubkey, program_ids: &ProgramIds) -> Result<Pubkey> {
        if let Some(existing) = self.user_token_accounts.get(mint) {
            return Ok(*existing);
        }

        let token_program = self.get_token_program_for_mint(mint, program_ids);
        let ata = get_associated_token_address_with_program_id(user, mint, &token_program);

        self.user_token_accounts.insert(*mint, ata);
        self.token_programs.insert(*mint, token_program);
        Ok(ata)
    }

    /// 获取 token program（带缓存；未命中时默认使用 Token Program）
    pub fn get_token_program_for_mint(&mut self, mint: &Pubkey, program_ids: &ProgramIds) -> Pubkey {
        if let Some(cached) = self.token_programs.get(mint) {
            return *cached;
        }
        
        program_ids.token_program
    }

    /// 从 remaining_accounts 检测并缓存某 mint 的 token program（Token 或 Token-2022）
    pub fn detect_and_cache_token_program_for_mint(
        &mut self,
        mint: &Pubkey,
        program_ids: &ProgramIds,
        remaining_accounts: &[AccountInfo],
    ) {
        // 若已缓存则跳过
        if self.token_programs.get(mint).is_some() { return; }
        if let Some(ai) = remaining_accounts.iter().find(|ai| ai.key() == *mint) {
            let owner = ai.owner;
            let detected = if owner == &program_ids.token_program {
                program_ids.token_program
            } else if owner == &program_ids.token_2022_program {
                program_ids.token_2022_program
            } else {
                // 未识别，退回默认 Token Program
                program_ids.token_program
            };
            self.token_programs.insert(*mint, detected);
            msg!("[TokenDetect] mint={} program_id={} (cached)", mint, detected);
        }
    }


    /// 推导Raydium CPMM authority (固定地址)
    pub fn derive_raydium_cpmm_authority(&mut self) -> Result<Pubkey> {
        let key = "cpmm_authority".to_string();
        if let Some(existing) = self.raydium_accounts.get(&key) {
            return Ok(*existing);
        }

        let fixed_addrs = self.fixed_addresses.as_ref()
            .ok_or_else(|| error!(crate::errors::ArbitrageError::AccountNotFound))?;
        
        let authority = fixed_addrs.raydium_cpmm_authority;
        self.raydium_accounts.insert(key, authority);
        Ok(authority)
    }

    /// 推导PumpFun bonding curve PDA
    pub fn derive_pumpfun_bonding_curve(&mut self, mint: &Pubkey, program_ids: &ProgramIds) -> Result<Pubkey> {
        let key = format!("bonding_curve_{}", mint);
        if let Some(existing) = self.pumpfun_accounts.get(&key) {
            return Ok(*existing);
        }

        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_BONDING_CURVE, mint.as_ref()],
            &program_ids.pumpfun,
        );

        self.pumpfun_accounts.insert(key, pda);
        Ok(pda)
    }

    /// 推导PumpFun associated bonding curve (bonding curve的代币账户)
    pub fn derive_pumpfun_associated_bonding_curve(
        &mut self,
        bonding_curve: &Pubkey,
        mint: &Pubkey,
        program_ids: &ProgramIds,
    ) -> Result<Pubkey> {
        let key = format!("associated_bonding_curve_{}_{}", bonding_curve, mint);
        if let Some(existing) = self.pumpfun_accounts.get(&key) {
            return Ok(*existing);
        }

        let token_program = self.get_token_program_for_mint(mint, program_ids);
        let ata = get_associated_token_address_with_program_id(bonding_curve, mint, &token_program);

        self.pumpfun_accounts.insert(key, ata);
        Ok(ata)
    }

    /// 推导PumpFun creator vault PDA
    pub fn derive_pumpfun_creator_vault(&mut self, creator: &Pubkey, program_ids: &ProgramIds) -> Result<Pubkey> {
        let key = format!("creator_vault_{}", creator);
        if let Some(existing) = self.pumpfun_accounts.get(&key) {
            return Ok(*existing);
        }

        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_CREATOR_VAULT, creator.as_ref()],
            &program_ids.pumpfun,
        );

        self.pumpfun_accounts.insert(key, pda);
        Ok(pda)
    }

    /// 推导PumpFun global volume accumulator PDA (仅买入时需要)
    pub fn derive_pumpfun_global_volume_accumulator(&mut self, program_ids: &ProgramIds) -> Result<Pubkey> {
        let key = "global_volume_accumulator".to_string();
        if let Some(existing) = self.pumpfun_accounts.get(&key) {
            return Ok(*existing);
        }

        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_GLOBAL_VOLUME_ACCUMULATOR],
            &program_ids.pumpfun,
        );

        self.pumpfun_accounts.insert(key, pda);
        Ok(pda)
    }

    /// 推导PumpFun user volume accumulator PDA (仅买入时需要)
    pub fn derive_pumpfun_user_volume_accumulator(&mut self, user: &Pubkey, program_ids: &ProgramIds) -> Result<Pubkey> {
        let key = format!("user_volume_accumulator_{}", user);
        if let Some(existing) = self.pumpfun_accounts.get(&key) {
            return Ok(*existing);
        }

        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPFUN_USER_VOLUME_ACCUMULATOR, user.as_ref()],
            &program_ids.pumpfun,
        );

        self.pumpfun_accounts.insert(key, pda);
        Ok(pda)
    }


    /// 推导PumpSwap global config PDA
    pub fn derive_pumpswap_global_config(&mut self, program_ids: &ProgramIds) -> Result<Pubkey> {
        let key = "global_config".to_string();
        if let Some(existing) = self.pumpswap_accounts.get(&key) {
            return Ok(*existing);
        }

        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPSWAP_GLOBAL_CONFIG],
            &program_ids.pumpswap,
        );

        self.pumpswap_accounts.insert(key, pda);
        Ok(pda)
    }

    /// 推导pool的代币账户ATA
    pub fn derive_pool_token_ata(&mut self, pool: &Pubkey, mint: &Pubkey, program_ids: &ProgramIds) -> Result<Pubkey> {
        let key = format!("pool_ata_{}_{}", pool, mint);
        if let Some(existing) = self.pumpswap_accounts.get(&key) {
            return Ok(*existing);
        }

        let token_program = self.get_token_program_for_mint(mint, program_ids);
        let ata = get_associated_token_address_with_program_id(pool, mint, &token_program);

        self.pumpswap_accounts.insert(key, ata);
        Ok(ata)
    }

    /// 推导PumpSwap creator vault authority PDA (使用AMM程序)
    pub fn derive_pumpswap_creator_vault_authority(&mut self, creator: &Pubkey) -> Result<Pubkey> {
        let key = format!("creator_vault_authority_{}", creator);
        if let Some(existing) = self.pumpswap_accounts.get(&key) {
            return Ok(*existing);
        }

        let fixed_addrs = self.fixed_addresses.as_ref()
            .ok_or_else(|| error!(crate::errors::ArbitrageError::AccountNotFound))?;

        let (pda, _) = Pubkey::find_program_address(
            &[pda_seeds::PUMPSWAP_CREATOR_VAULT, creator.as_ref()],
            &fixed_addrs.pumpswap_amm_program,
        );

        self.pumpswap_accounts.insert(key, pda);
        Ok(pda)
    }

    /// 推导creator vault ATA
    pub fn derive_pumpswap_creator_vault_ata(
        &mut self,
        creator: &Pubkey,
        quote_mint: &Pubkey,
        program_ids: &ProgramIds,
    ) -> Result<Pubkey> {
        let key = format!("creator_vault_ata_{}_{}", creator, quote_mint);
        if let Some(existing) = self.pumpswap_accounts.get(&key) {
            return Ok(*existing);
        }

        let creator_vault_authority = self.derive_pumpswap_creator_vault_authority(creator)?;
        let token_program = self.get_token_program_for_mint(quote_mint, program_ids);
        let ata = get_associated_token_address_with_program_id(&creator_vault_authority, quote_mint, &token_program);

        self.pumpswap_accounts.insert(key, ata);
        Ok(ata)
    }

    /// 推导fee recipient ATA
    pub fn derive_pumpswap_fee_recipient_ata(&mut self, mint: &Pubkey, program_ids: &ProgramIds) -> Result<Pubkey> {
        let key = format!("fee_recipient_ata_{}", mint);
        if let Some(existing) = self.pumpswap_accounts.get(&key) {
            return Ok(*existing);
        }

        let fee_recipient = self.fixed_addresses.as_ref()
            .ok_or_else(|| error!(crate::errors::ArbitrageError::AccountNotFound))?
            .pumpswap_fee_recipient;

        let token_program = self.get_token_program_for_mint(mint, program_ids);
        let ata = get_associated_token_address_with_program_id(
            &fee_recipient,
            mint,
            &token_program,
        );

        self.pumpswap_accounts.insert(key, ata);
        Ok(ata)
    }

    /// 为整个套利路径推导所有账户
    pub fn derive_for_path(&mut self, path: &[PathStep], user: &Pubkey, program_ids: &ProgramIds, remaining_accounts: &[AccountInfo]) -> Result<()> {
        for step in path {
            // 先尝试从 remaining_accounts 自动识别 token program（Token/Token-2022）
            self.detect_and_cache_token_program_for_mint(&step.input_mint, program_ids, remaining_accounts);
            self.detect_and_cache_token_program_for_mint(&step.output_mint, program_ids, remaining_accounts);

            // 再推导用户的输入输出代币账户（使用已缓存的正确 token program）
            self.derive_user_ata(user, &step.input_mint, program_ids)?;
            self.derive_user_ata(user, &step.output_mint, program_ids)?;
            
            // 根据DEX类型推导特定账户
            match step.dex_type {
                DexType::RaydiumCpmm => {
                    self.derive_raydium_cpmm_authority()?;
                }
                DexType::RaydiumClmm => {
                    // CLMM 主要依赖客户端提供的动态账户，这里仅完成用户 ATA 推导
                }
                DexType::PumpFunBondingCurve => {
                    // 方向感知：若 output_mint 是 WSOL，则 token_mint= input_mint；否则 token_mint= output_mint
                    let fixed_addrs = self.fixed_addresses.as_ref()
                        .ok_or_else(|| error!(crate::errors::ArbitrageError::AccountNotFound))?;
                    let token_mint = if step.output_mint == fixed_addrs.wrapped_sol_mint {
                        step.input_mint
                    } else {
                        step.output_mint
                    };

                    self.derive_pumpfun_bonding_curve(&token_mint, program_ids)?;
                    if let Some(bonding_curve) = &step.pool_id {
                        self.derive_pumpfun_associated_bonding_curve(bonding_curve, &token_mint, program_ids)?;
                    }
                }
                DexType::PumpSwap => {
                    self.derive_pumpswap_global_config(program_ids)?;
                    if let Some(pool_id) = &step.pool_id {
                        self.derive_pool_token_ata(pool_id, &step.input_mint, program_ids)?;
                        self.derive_pool_token_ata(pool_id, &step.output_mint, program_ids)?;
                    }
                }
            }
        }
        Ok(())
    }

    // ================================================================
    // 访问器方法
    // ================================================================
    
    /// 获取用户代币账户
    pub fn get_user_token_account(&self, mint: &Pubkey) -> Option<&Pubkey> {
        self.user_token_accounts.get(mint)
    }

    /// 获取固定地址
    pub fn get_fixed_addresses(&self) -> Option<&FixedAddresses> {
        self.fixed_addresses.as_ref()
    }
}