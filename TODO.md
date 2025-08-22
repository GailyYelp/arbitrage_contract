# 🚀 Arbitrage Contract 开发任务清单

## 📋 项目概述
构建一个多DEX聚合套利合约，支持突破64账户限制，能够在单笔交易中执行多步骤跨DEX套利。

## 🎯 核心目标
- ✅ 突破Solana 64账户限制
- ✅ 支持多DEX套利路径
- ✅ 账户优化：客户端传递最少账户，合约推导其余账户
- ✅ 高效CPI调用多个DEX协议

---

## 📦 Phase 1: 核心模块开发

### 1️⃣ 账户解析器模块 (Account Resolver)
**目标：** 解析remaining_accounts，为每个DEX分配正确的账户

#### 任务清单：
- [ ] 创建 `src/account_resolver/mod.rs`
- [ ] 定义 `AccountResolver` 结构体
  ```rust
  pub struct AccountResolver<'info> {
      remaining_accounts: &'info [AccountInfo<'info>],
      mappings: &'info [PathAccountMapping],
  }
  ```
- [ ] 实现DEX专用解析方法
  - [ ] `resolve_raydium_cpmm_accounts()` - 解析Raydium CPMM账户（5个账户）
  - [ ] `resolve_raydium_clmm_accounts()` - 解析Raydium CLMM账户（8+N个账户）
  - [ ] `resolve_pumpfun_accounts()` - 解析PumpFun账户（3个账户）
  - [ ] `resolve_pumpswap_accounts()` - 解析PumpSwap账户（4个账户）
- [ ] 实现账户验证逻辑
  - [ ] 验证账户数量匹配
  - [ ] 验证账户索引边界
  - [ ] 验证账户类型（可写/只读）

#### 依赖：
- `PathAccountMapping` 结构（从客户端传入）
- DEX特定的账户结构定义

---

### 2️⃣ 账户推导器模块 (Account Derivation)
**目标：** 推导所有可计算的账户，减少客户端传递

#### 任务清单：
- [ ] 创建 `src/account_derivation/mod.rs`
- [ ] 定义 `DerivedAccounts` 结构体
  ```rust
  pub struct DerivedAccounts<'info> {
      pub user: &'info AccountInfo<'info>,
      pub user_token_accounts: HashMap<Pubkey, Pubkey>,
      pub program_authorities: HashMap<DexType, Pubkey>,
  }
  ```
- [ ] 实现推导方法
  - [ ] `derive_user_ata()` - 推导用户的关联代币账户
  - [ ] `derive_raydium_authority()` - 推导Raydium的authority账户
  - [ ] `derive_pumpfun_associated_accounts()` - 推导PumpFun的关联账户
  - [ ] `derive_pumpswap_pda_accounts()` - 推导PumpSwap的PDA账户
- [ ] 实现批量推导
  - [ ] `derive_for_path()` - 一次性推导整个路径需要的所有账户
  - [ ] 缓存已推导账户避免重复计算

#### 关键PDA推导规则：
```rust
// Raydium Authority
seeds = [b"vault_and_lp_mint_auth_seed"], program_id = raydium_program

// PumpFun Associated
seeds = [b"associated", bonding_curve.key()], program_id = pumpfun_program  

// User ATA
seeds = [user.key(), TOKEN_PROGRAM_ID, mint.key()], program_id = ATA_PROGRAM
```

---

### 3️⃣ DEX路由器模块 (DEX Router)
**目标：** 根据DEX类型路由到正确的CPI调用

#### 任务清单：
- [ ] 创建 `src/dex_router/mod.rs`
- [ ] 定义路由器接口
  ```rust
  pub trait DexSwap {
      fn execute_swap(
          accounts: Self::Accounts,
          derived: &DerivedAccounts,
          amount_in: u64,
          minimum_amount_out: u64,
      ) -> Result<u64>;
  }
  ```
- [ ] 实现各DEX的CPI调用
  - [ ] `raydium_cpmm_swap()` - Raydium CPMM交换实现
  - [ ] `raydium_clmm_swap()` - Raydium CLMM交换实现  
  - [ ] `pumpfun_buy()` / `pumpfun_sell()` - PumpFun买卖实现
  - [ ] `pumpswap_swap()` - PumpSwap交换实现
- [ ] 实现路由分发
  - [ ] `execute_swap()` - 根据DexType分发到具体实现
  - [ ] 处理不同DEX的返回值格式
  - [ ] 统一错误处理

#### CPI调用模板：
```rust
// Raydium CPMM CPI
let cpi_accounts = raydium_cp_swap::cpi::accounts::Swap { ... };
let cpi_ctx = CpiContext::new(program, cpi_accounts);
raydium_cp_swap::cpi::swap_base_input(cpi_ctx, amount_in, minimum_amount_out)?;
```

---

## 📦 Phase 2: 主指令实现

### 4️⃣ 执行套利指令 (Execute Arbitrage)
**目标：** 串联所有模块，实现完整的套利流程

#### 任务清单：
- [ ] 更新 `src/instructions/execute_arbitrage.rs`
- [ ] 实现主要执行流程
  ```rust
  pub fn execute_arbitrage(
      ctx: Context<ExecuteArbitrage>,
      params: ArbitrageParams,
  ) -> Result<()>
  ```
- [ ] 执行步骤
  - [ ] 参数验证（路径、金额、滑点）
  - [ ] 推导所有需要的账户
  - [ ] 创建账户解析器
  - [ ] 循环执行每个交换步骤
  - [ ] 验证最终利润
  - [ ] 返回执行结果

#### 错误处理：
- [ ] 路径验证错误
- [ ] 账户不足错误
- [ ] CPI调用失败
- [ ] 利润不足错误
- [ ] 滑点超限错误

---

## 📦 Phase 3: 依赖和配置

### 5️⃣ Cargo依赖配置
**目标：** 添加所有DEX的CPI依赖

#### 任务清单：
- [ ] 更新 `Cargo.toml`
  ```toml
  [dependencies]
  anchor-lang = "0.31.1"
  anchor-spl = "0.31.1"
  
  # DEX CPI依赖
  raydium-cp-swap = { git = "...", features = ["cpi", "no-entrypoint"] }
  raydium-clmm = { git = "...", features = ["cpi", "no-entrypoint"] }
  # PumpFun和PumpSwap可能需要自定义接口
  ```
- [ ] 配置feature flags
- [ ] 解决版本冲突

### 6️⃣ 类型定义更新
**目标：** 确保类型与客户端一致

#### 任务清单：
- [ ] 更新 `src/state.rs`
  - [ ] 同步 `DexType` 枚举
  - [ ] 同步 `PathStep` 结构
  - [ ] 同步 `PathAccountMapping` 结构
  - [ ] 同步 `ArbitrageParams` 结构
- [ ] 确保序列化格式一致


---

## 📝 备注

### 关键设计决策
1. **不实现状态管理** - 先专注核心功能，统计功能可后续添加
2. **使用remaining_accounts** - 这是突破64账户限制的核心技术
3. **客户端/合约分工** - 客户端收集账户，合约推导和执行

### 参考资源
- Raydium CPI Example: `/Users/zhengwei/CursorProjects/raydium-cpi-example`
- 客户端代码: `/Users/zhengwei/CursorProjects/money_donkey/src/arbitrage_contract/client/`
- Anchor文档: https://www.anchor-lang.com/

### 风险和挑战
1. **DEX接口变化** - 需要持续跟踪各DEX的更新
2. **账户限制** - 即使优化后，极复杂路径可能仍超限
3. **CPI深度限制** - Solana对CPI调用深度有限制

---

## ✅ 完成标准
- [ ] 能够执行单DEX交换
- [ ] 能够执行2-3步跨DEX套利
- [ ] 账户传递数量减少50%以上


---

*最后更新: 2024-08-15*
*维护者: Money Donkey Team*