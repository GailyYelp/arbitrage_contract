## 套利合约全流程说明（V2 indices 协议）

本文聚焦“客户端如何与合约交互”、以及“合约端内部架构/流程/模块职责”。不包含安全与版本治理内容。

### 总览
- 单指令架构：客户端构造一条 `execute_arbitrage` 指令，附带“全局去重账户表 + 每步 indices”。
- 账户长度治理：v0 交易 + Address Lookup Tables（ALT）承载“全局去重账户表”；每步仅传 `indices` 指向全局表，避免重复。
- 链上执行策略：逐步解析每步账户 → CPI 执行 DEX → 读取余额差得到真实 `amount_out` → 用作下一步输入 → 最终利润校验。

---

## 客户端应当传递什么

### 1) 指令方法与固定账户（Anchor）
- 方法: `global:execute_arbitrage`（8 字节 discriminator + Borsh 参数体）
- 固定账户（置于账户列表最前）：
  - `user`（Signer, W）
  - `token_program`（R）
  - `associated_token_program`（R）
  - `system_program`（R）

### 2) 全局去重账户表（remaining_accounts）
- 这是本协议的核心载体。把“本路径所有 CPI 可能用到的账户”都加入此表（去重后再加入）。
- 包含：
  - 各 DEX 的最小集（由 indices 指向）；
  - 外部 DEX 程序账户（program AccountInfo）；
  - 固定地址/PDA/动态账户（如 Raydium CLMM 的 tick arrays/extension）；
  - 用户与池的两侧 ATAs、fee_recipient_ata、creator_vault_* 等。
- 注：链上不会“补账户”。一切 CPI 会用到的账户都必须在这里。

### 3) 指令参数体（ArbitrageParams，Borsh）
- `input_amount: u64`：全路径初始输入数量
- `min_profit_lamports: u64`：最终利润阈值
- `max_slippage_bps: u16`：最大滑点（用于日志/治理，不参与硬校验由每步 min_out 覆盖）
- `path_steps: Vec<PathStep>`：每步交换描述
  - `pool_id: Option<Pubkey>`：目标池/bonding_curve
  - `dex_type: DexType`：RaydiumCpmm / RaydiumClmm / PumpFunBondingCurve / PumpSwap
  - `input_mint: Pubkey`、`output_mint: Pubkey`
  - `minimum_amount_out: u64`：本步最小可接受产出
- `account_mappings_v2: Vec<PathAccountMappingV2>`：每步 indices 映射
  - `dex_type: DexType`（与步一致）
  - `contract_type: ContractType`（与步一致）
  - `indices: Vec<u8>`（指向“全局去重账户表”的位置）

### 4) 每个 DEX 的 indices 期望数量（仅最小集）
- Raydium CPMM：7（`amm_config, pool_state, token0_vault, token1_vault, input_mint, output_mint, observation_state`）
- Raydium CLMM：11（`clmm_program, amm_config, pool_state, input_vault, output_vault, observation_state, token_program, token_program_2022, memo_program, input_vault_mint, output_vault_mint`）
  - 额外：`tick arrays/extension` 不计入 indices，必须追加在全局表，链上动态注入
- PumpFun（Bonding Curve）：3（`bonding_curve, mint, creator`）
- PumpSwap：4（`pool_state, base_mint, quote_mint, coin_creator`）

### 5) 客户端需要额外“追加到全局表”的账户（常用）
- CPMM：`raydium_cpmm_program`、`raydium_cpmm_authority`、用户两侧 ATAs
- CLMM：`tick_array_extension`、若干 `tick arrays`（owner=clmm_program 的数据账户）
- PumpFun：`program`、`global`、`fee_recipient`、`event_authority`、`associated_bonding_curve`、`creator_vault`、（买入）`global/user volume accumulators`
- PumpSwap：`program(amm_program)`、`global_config`、`fee_recipient`、`fee_recipient_ata`、`event_authority`、用户与池双方 ATAs、`creator_vault_authority/creator_vault_ata`

---

## 合约端如何接收并执行

### 入口（`instructions/execute_arbitrage.rs`）
1) 参数/环境校验：
   - 路径非空、长度上限、`input_amount > 0`、`steps == mappings_v2.len()`；
   - 固定程序一致性（associated_token/system）。
2) 账户推导缓存（`DerivedAccounts`）：
   - 识别每个 mint 的 token program（Token vs Token-2022），用于后续定位正确 ATA；
   - 为路径所有 mint 推导用户 ATAs 的 Pubkey 并缓存（仅算键，不加表）。
3) 逐步执行：
   - 构造 `AccountResolver`（握住 `remaining_accounts`）；
   - `validate_indices_for_dex`：校验每步 indices 数量/越界/重复，并打印角色与 W/S 提示；
   - 解析得到该 DEX 的最小账户集（`...Accounts<'info>`）；
   - 从缓存拿用户输入/输出 ATA 的 Pubkey，并在全局表中定位 `AccountInfo`；
   - 校验用户 ATA 的 program/mint/owner；
   - `DexRouter::execute_swap(...)` 执行实际 CPI（见下节）；
   - 读取本步真实 `amount_out` 并与 `minimum_amount_out` 对比；
   - 将 `amount_out` 作为下一步 `amount_in` 继续。
4) 终局：检查 `current_amount >= input_amount + min_profit_lamports`，成功返回。

### 账户解析（`account_resolver/`）
- `accounts.rs`：定义四类 DEX 的“最小账户集”（indices 所指向的 AccountInfo 组）。
- `resolver.rs`：
  - `resolve_*_by_indices(...)`：把 `indices` 转为类型化的 `...Accounts<'info>`；
  - `validate_indices_for_dex(...)`：数量/越界/重复检查，并打印“角色+W/S”提示；
  - 注：仅解析，不派生/不补账户。

### 账户推导与缓存（`account_derivation/derivation.rs`）
- 目标：仅做“期望值计算与缓存”（帮助定位和日志），不向全局表添加账户。
- 功能：
  - `detect_and_cache_token_program_for_mint`：mint.owner → Token or Token-2022；
  - `derive_user_ata`：基于签名者 + mint + 正确 token program 计算用户 ATA Pubkey；
  - 少量固定/PDA 期望值（如 CPMM authority、Pump 系列 PDA）用于在 `swaps.rs` 中定位对应 `AccountInfo`。

### 路由与交换（`dex_router/`）
- `router.rs`：根据 `DexType` 分发到具体的 DEX 交换实现；并提供每步 `minimum_amount_out` 校验工具。
- `swaps.rs`：四类 DEX 的 CPI 具体实现。统一流程：
  1) 读取执行前用户“输出 ATA”的余额（`pre_out`）。
  2) 构造外部指令 `Instruction{ program_id, accounts: Vec<AccountMeta>, data }`。
  3) 收集与 `accounts` 一一对应的 `account_infos: Vec<AccountInfo>`，并额外把“被调用程序的 AccountInfo”推入向量尾部。
  4) `invoke(&ix, &account_infos)`。
  5) 读取执行后余额（`post_out`），`amount_out = post_out - pre_out`。

#### Raydium CPMM（示例）
- metas 典型顺序：`payer, authority, amm_config, pool_state, user_in, user_out, token0_vault, token1_vault, token_program×2, input_mint, output_mint, observation_state`。
- program 账户：`raydium_cpmm_program` 需在全局表中存在，并校验 `executable` 与 program_id 一致。

#### Raydium CLMM
- indices 提供基础 11 个；`tick arrays/extension` 追加在全局表后，链上按 `owner == clmm_program` 动态注入到 metas/account_infos。
- program 账户：`clmm_program` 必须在基础 11 个中（indices[0]），并做一致性校验。

#### PumpFun（Bonding Curve）
- 链上根据用户输入/输出 ATA 的 mint 与 `wrapped_sol_mint` 自动判定 BUY/SELL，并使用对应 discriminator 与参数顺序：
  - BUY：`[BUY, token_amount=min_out, max_sol_cost=amount_in]`；
  - SELL：`[SELL, token_amount=amount_in, min_sol_output=min_out]`。
- metas 典型：`global, fee_recipient, mint, bonding_curve, associated_bonding_curve, user_ata, user, system, (BUY: token_program, creator_vault, event) / (SELL: creator_vault, token_program, event), [opt volume accumulators]`。
- program 账户：`pumpfun_program` 需在全局表中存在并通过一致性校验。

#### PumpSwap
- 通过 owner+mint 扫描定位 `user/pool` 两侧 ATAs、`fee_recipient_ata`、`creator_vault_ata`；`creator_vault_authority` 由 PDA 期望值推导后在全局表定位。
- program 账户：`amm_program` 需可执行且与预期一致。

---

## 模块职责一览

- `src/lib.rs`：程序入口模块与 `declare_id!`。
- `instructions/execute_arbitrage.rs`：主执行逻辑（参数校验 → 推导缓存 → 逐步解析与执行 → 金额校验）。
- `state.rs`：协议数据结构（`DexType/ContractType/PathStep/PathAccountMappingV2/ArbitrageParams`）。
- `account_resolver/accounts.rs`：四类 DEX 的最小账户集定义（`AccountInfo` 版）。
- `account_resolver/resolver.rs`：按 indices 解析、数量与角色提示校验。
- `account_derivation/derivation.rs`：用户 ATAs 与部分 PDA 的“期望值推导与缓存”。
- `dex_router/types.rs`：`SwapResult`、常量、工具（期望账户数量）。
- `dex_router/router.rs`：按 DEX 路由到交换实现，并做 `min_out` 校验。
- `dex_router/swaps.rs`：每个 DEX 的 CPI 构造与 `amount_out` 余额差计算。
- `errors.rs`：错误码枚举。

---

## 客户端适配器（要点回顾）

客户端（adapter）需完成：
- 收集全路径账户 → 去重 → 生成全局表；
- 为每步生成 `indices`（仅指向“最小集”）；
- 必要的“追加项”务必一并加入全局表（程序账户、固定地址/PDA、用户/池两侧 ATAs、CLMM tick arrays 等）；
- 追加用户 ATAs（用于余额差），可不计入 indices，但必须在全局表中；
- 生成 `ArbitrageParams` 并按 Anchor discriminator + Borsh 序列化指令 data；
- 最终拼出 `Instruction { program_id, accounts=[固定4+全局表], data }`，用 v0 交易 + ALT 发送。

---

## 新增 DEX 的接入步骤（简述）

1) 明确该 DEX 的“最小集账户”与“追加项”。
2) 客户端：
   - 在 adapter 中加入账户收集/去重/indices 生成；
   - 把程序账户与所有追加项放入全局表。
3) 合约端：
   - 在 `account_resolver/accounts.rs` 定义最小集结构；
   - 在 `resolver.rs` 增加 `resolve_xxx_by_indices` 与 `validate_indices_for_dex`；
   - 在 `dex_router/swaps.rs` 实现 CPI 构造（含 metas 与 account_infos），并读取余额差；
   - 在 `dex_router/types.rs`/`state.rs` 更新枚举与常量。


