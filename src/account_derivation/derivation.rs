use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address_with_program_id;
use crate::state::{DexType, PathStep};
use super::types::{ProgramIds, FixedAddresses, get_fixed_addresses, pda_seeds};
use std::collections::HashMap;

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

    /// 获取token program (目前默认使用Token Program)
    pub fn get_token_program_for_mint(&mut self, mint: &Pubkey, program_ids: &ProgramIds) -> Pubkey {
        if let Some(cached) = self.token_programs.get(mint) {
            return *cached;
        }
        
        // TODO: 实际实现中需要检查mint账户的owner来判断是Token还是Token2022
        program_ids.token_program
    }

    // ================================================================
    // Raydium CPMM 账户推导 - 需要的账户:
    // 1. authority (固定地址)
    // 2. user_input_ata  
    // 3. user_output_ata
    // 4. input_vault (客户端提供)
    // 5. output_vault (客户端提供)
    // 6. pool_id (客户端提供)
    // 7. amm_config (客户端提供)
    // 8. observation_account (客户端提供)
    // ================================================================

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

    /// 组装Raydium CPMM完整账户
    pub fn assemble_raydium_cpmm_accounts(
        &mut self,
        user: &Pubkey,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        program_ids: &ProgramIds,
    ) -> Result<RaydiumCpmmAccounts> {
        let authority = self.derive_raydium_cpmm_authority()?;
        let user_input_ata = self.derive_user_ata(user, input_mint, program_ids)?;
        let user_output_ata = self.derive_user_ata(user, output_mint, program_ids)?;
        let input_token_program = self.get_token_program_for_mint(input_mint, program_ids);
        let output_token_program = self.get_token_program_for_mint(output_mint, program_ids);

        Ok(RaydiumCpmmAccounts {
            payer: *user,
            authority,
            user_input_ata,
            user_output_ata,
            input_token_program,
            output_token_program,
            input_mint: *input_mint,
            output_mint: *output_mint,
        })
    }

    // ================================================================
    // Raydium CLMM 账户推导 - 需要的账户:
    // 1. payer (用户)
    // 2. amm_config (客户端提供)
    // 3. pool_id (客户端提供)
    // 4. user_input_ata
    // 5. user_output_ata  
    // 6. input_vault (客户端提供)
    // 7. output_vault (客户端提供)
    // 8. observation_account (客户端提供)
    // 9. tickarray_bitmap_extension (客户端提供)
    // 10. tick_array_accounts (客户端提供，动态数量)
    // ================================================================

    /// 组装Raydium CLMM完整账户
    pub fn assemble_raydium_clmm_accounts(
        &mut self,
        user: &Pubkey,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        program_ids: &ProgramIds,
    ) -> Result<RaydiumClmmAccounts> {
        let user_input_ata = self.derive_user_ata(user, input_mint, program_ids)?;
        let user_output_ata = self.derive_user_ata(user, output_mint, program_ids)?;
        let input_token_program = self.get_token_program_for_mint(input_mint, program_ids);
        let output_token_program = self.get_token_program_for_mint(output_mint, program_ids);

        Ok(RaydiumClmmAccounts {
            payer: *user,
            user_input_ata,
            user_output_ata,
            input_token_program,
            output_token_program,
            input_mint: *input_mint,
            output_mint: *output_mint,
        })
    }

    // ================================================================
    // PumpFun Bonding Curve 账户推导 - 需要的账户:
    // 1. global (固定地址)
    // 2. fee_recipient (固定地址) 
    // 3. mint (参数)
    // 4. bonding_curve (PDA: [b"bonding-curve", mint])
    // 5. associated_bonding_curve (bonding_curve的ATA)
    // 6. user_token_ata (用户的代币ATA)
    // 7. user (签名者)
    // 8. system_program
    // 9. token_program
    // 10. creator_vault (PDA: [b"creator-vault", creator])
    // 11. event_authority (固定地址)
    // 12. program_id
    // 13. global_volume_accumulator (PDA: [b"global_volume_accumulator"]) - 仅买入时需要
    // 14. user_volume_accumulator (PDA: [b"user_volume_accumulator", user]) - 仅买入时需要
    // ================================================================

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

    /// 组装PumpFun完整账户
    pub fn assemble_pumpfun_accounts(
        &mut self,
        user: &Pubkey,
        mint: &Pubkey,
        creator: &Pubkey,
        is_buy: bool,
        program_ids: &ProgramIds,
    ) -> Result<PumpFunAccounts> {
        // 先推导所有需要的账户
        let bonding_curve = self.derive_pumpfun_bonding_curve(mint, program_ids)?;
        let associated_bonding_curve = self.derive_pumpfun_associated_bonding_curve(&bonding_curve, mint, program_ids)?;
        let user_token_ata = self.derive_user_ata(user, mint, program_ids)?;
        let creator_vault = self.derive_pumpfun_creator_vault(creator, program_ids)?;

        // 买入时需要volume accumulator账户
        let volume_accumulators = if is_buy {
            Some(PumpFunVolumeAccumulators {
                global: self.derive_pumpfun_global_volume_accumulator(program_ids)?,
                user: self.derive_pumpfun_user_volume_accumulator(user, program_ids)?,
            })
        } else {
            None
        };
        
        // 获取固定地址
        let fixed_addrs = self.fixed_addresses.as_ref()
            .ok_or_else(|| error!(crate::errors::ArbitrageError::AccountNotFound))?;

        Ok(PumpFunAccounts {
            global_account: fixed_addrs.pumpfun_global_account,
            fee_recipient: fixed_addrs.pumpfun_fee_recipient,
            mint: *mint,
            bonding_curve,
            associated_bonding_curve,
            user_token_ata,
            user: *user,
            creator_vault,
            event_authority: fixed_addrs.pumpfun_event_authority,
            token_program: self.get_token_program_for_mint(mint, program_ids),
            system_program: program_ids.system_program,
            volume_accumulators,
        })
    }

    // ================================================================
    // PumpSwap AMM 账户推导 - 需要的账户:
    // 1. pool_id (参数)
    // 2. user (签名者)
    // 3. global_config (PDA: [b"global_config"])
    // 4. base_mint (参数)
    // 5. quote_mint (参数)
    // 6. user_base_ata
    // 7. user_quote_ata
    // 8. pool_base_ata (pool的base代币账户)
    // 9. pool_quote_ata (pool的quote代币账户)
    // 10. fee_recipient (固定地址)
    // 11. fee_recipient_ata
    // 12. base_token_program
    // 13. quote_token_program
    // 14. system_program
    // 15. associated_token_program
    // 16. event_authority (固定地址)
    // 17. amm_program (固定地址)
    // 18. creator_vault_ata (创建者金库ATA)
    // 19. creator_vault_authority (PDA: [b"creator_vault", creator] 使用amm_program)
    // ================================================================

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

    /// 组装PumpSwap完整账户
    pub fn assemble_pumpswap_accounts(
        &mut self,
        user: &Pubkey,
        pool: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        creator: &Pubkey,
        program_ids: &ProgramIds,
    ) -> Result<PumpSwapAccounts> {
        // 先推导所有需要的账户
        let global_config = self.derive_pumpswap_global_config(program_ids)?;
        let user_base_ata = self.derive_user_ata(user, base_mint, program_ids)?;
        let user_quote_ata = self.derive_user_ata(user, quote_mint, program_ids)?;
        let pool_base_ata = self.derive_pool_token_ata(pool, base_mint, program_ids)?;
        let pool_quote_ata = self.derive_pool_token_ata(pool, quote_mint, program_ids)?;
        let creator_vault_authority = self.derive_pumpswap_creator_vault_authority(creator)?;
        let creator_vault_ata = self.derive_pumpswap_creator_vault_ata(creator, quote_mint, program_ids)?;
        let fee_recipient_ata = self.derive_pumpswap_fee_recipient_ata(quote_mint, program_ids)?;
        
        // 获取固定地址
        let fixed_addrs = self.fixed_addresses.as_ref()
            .ok_or_else(|| error!(crate::errors::ArbitrageError::AccountNotFound))?;

        Ok(PumpSwapAccounts {
            pool: *pool,
            user: *user,
            global_config,
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            user_base_ata,
            user_quote_ata,
            pool_base_ata,
            pool_quote_ata,
            fee_recipient: fixed_addrs.pumpswap_fee_recipient,
            fee_recipient_ata,
            creator_vault_authority,
            creator_vault_ata,
            event_authority: fixed_addrs.pumpswap_event_authority,
            amm_program: fixed_addrs.pumpswap_amm_program,
            base_token_program: self.get_token_program_for_mint(base_mint, program_ids),
            quote_token_program: self.get_token_program_for_mint(quote_mint, program_ids),
            system_program: program_ids.system_program,
            associated_token_program: program_ids.associated_token_program,
        })
    }

    // ================================================================
    // 高级批量处理函数
    // ================================================================

    /// 为整个套利路径推导所有账户
    pub fn derive_for_path(&mut self, path: &[PathStep], user: &Pubkey, program_ids: &ProgramIds) -> Result<()> {
        for step in path {
            // 推导用户的输入输出代币账户
            self.derive_user_ata(user, &step.input_mint, program_ids)?;
            self.derive_user_ata(user, &step.output_mint, program_ids)?;
            
            // 根据DEX类型推导特定账户
            match step.dex_type {
                DexType::RaydiumCpmm => {
                    self.derive_raydium_cpmm_authority()?;
                }
                DexType::RaydiumClmm => {
                    // CLMM主要依赖客户端提供的动态账户，这里只推导用户ATA
                }
                DexType::PumpFunBondingCurve => {
                    self.derive_pumpfun_bonding_curve(&step.output_mint, program_ids)?;
                    if let Some(bonding_curve) = &step.pool_id {
                        self.derive_pumpfun_associated_bonding_curve(bonding_curve, &step.output_mint, program_ids)?;
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

// ================================================================
// 账户结构体定义
// ================================================================

/// Raydium CPMM所需账户
#[derive(Debug, Clone)]
pub struct RaydiumCpmmAccounts {
    pub payer: Pubkey,
    pub authority: Pubkey,
    pub user_input_ata: Pubkey,
    pub user_output_ata: Pubkey,
    pub input_token_program: Pubkey,
    pub output_token_program: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    // 以下账户需要客户端提供:
    // pool_id, amm_config, observation_account, input_vault, output_vault
}

/// Raydium CLMM所需账户
#[derive(Debug, Clone)]
pub struct RaydiumClmmAccounts {
    pub payer: Pubkey,
    pub user_input_ata: Pubkey,
    pub user_output_ata: Pubkey,
    pub input_token_program: Pubkey,
    pub output_token_program: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    // 以下账户需要客户端提供:
    // amm_config, pool_id, input_vault, output_vault, observation_account, 
    // tickarray_bitmap_extension, tick_array_accounts
}

/// PumpFun Volume Accumulator账户 (仅买入时需要)
#[derive(Debug, Clone)]
pub struct PumpFunVolumeAccumulators {
    pub global: Pubkey,
    pub user: Pubkey,
}

/// PumpFun所需账户
#[derive(Debug, Clone)]
pub struct PumpFunAccounts {
    pub global_account: Pubkey,
    pub fee_recipient: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub associated_bonding_curve: Pubkey,
    pub user_token_ata: Pubkey,
    pub user: Pubkey,
    pub creator_vault: Pubkey,
    pub event_authority: Pubkey,
    pub token_program: Pubkey,
    pub system_program: Pubkey,
    pub volume_accumulators: Option<PumpFunVolumeAccumulators>, // 仅买入时需要
}

/// PumpSwap所需账户
#[derive(Debug, Clone)]
pub struct PumpSwapAccounts {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub global_config: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub user_base_ata: Pubkey,
    pub user_quote_ata: Pubkey,
    pub pool_base_ata: Pubkey,
    pub pool_quote_ata: Pubkey,
    pub fee_recipient: Pubkey,
    pub fee_recipient_ata: Pubkey,
    pub creator_vault_authority: Pubkey,
    pub creator_vault_ata: Pubkey,
    pub event_authority: Pubkey,
    pub amm_program: Pubkey,
    pub base_token_program: Pubkey,
    pub quote_token_program: Pubkey,
    pub system_program: Pubkey,
    pub associated_token_program: Pubkey,
}