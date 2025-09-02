# V4 单指令路由协议（分段账户 + 极简步骤元数据）

本文件描述链上 Program 的 V4 入参与账户约定，保持路由/CPI 实现不变，仅升级交互方式，并提供与直连顺序的对比与联调检查清单。

## Anchor 固定账户（按顺序）
- user: Signer（可写）
- token_program: Program<Token>
- associated_token_program: Program<AssociatedToken>
- system_program: Program<System>

说明：这些通用程序账户不需要放入 step_accounts（部分 DEX 仍会把 token_2022/memo 作为参数账户放在本步切片内）。

## 指令数据体（Borsh）
- V4ArbParams
  - mints_count: u8            // 路径涉及的 mint 数量（闭环：steps == mints_count）
  - input_amount: u64          // 第一步输入量
  - min_profit_lamports: u64   // 终局最小利润
  - steps: Vec<V4StepMeta>     // 长度 mints_count
- V4StepMeta
  - dex_type: u8               // 0=CPMM, 1=CLMM, 2=PumpFun, 3=PumpSwap
  - accounts_len: u8           // 本步账户组长度（用于从 remaining_accounts 切片）
  - flags: u8                  // 预留；当前仅 CLMM 使用 bit0 表示是否包含 tick_array_extension

调用 discriminator: "global:execute_arbitrage_v4"

## remaining_accounts 三段分布（按顺序）
- 段1：mints[M]
- 段2：user_token_accounts[M]（与 mints 一一对应，优先 ATA，不强制）
- 段3：step_accounts（逐步平铺；第 i 步长度 = steps[i].accounts_len）

闭环约定：第 i 步币对 = (mints[i], mints[(i+1)%M])；对应用户账户 = user_accounts[i] 与 user_accounts[(i+1)%M]。

## 各 DEX 的 step_accounts 账户顺序（本步切片内）
说明：以下仅列出本步切片内需显式传入的账户。通用程序账户（token/system/associated_token）来自 Anchor 固定账户。

- Raydium CPMM（base ≥ 6）
  0 program_id（必须 executable）
  1 amm_config
  2 pool_state
  3 token0_vault
  4 token1_vault
  5 observation_state
  [可选] authority（用于日志/兼容）
  [可选] token_2022_program（若输入/输出 mint 属于 2022，可一并提供）

- Raydium CLMM（base=11，后接动态项）
  0 clmm_program（必须 executable）
  1 amm_config
  2 pool_state
  3 input_vault
  4 output_vault
  5 observation_state
  6 token_program
  7 token_program_2022
  8 memo_program
  9 input_vault_mint
  10 output_vault_mint
  [可选] tick_array_extension（flags.bit0=1 时需存在）
  [可选] tick_arrays[0..N]（按路径方向选择，数量 N≥0）

- PumpFun（Bonding Curve，base ≥ 5）
  0 pumpfun_program（必须 executable）
  1 global（PDA: "global"）
  2 bonding_curve（pool_id）
  3 mint
  4 creator
  5 event_authority（PDA: "__event_authority"）
  6 associated_bonding_curve（owner=bonding_curve, mint）
  [可选] fee_recipient
  [可选] creator_vault（PDA）
  [可选] global/user volume accumulators（买入路径尽力提供）

- PumpSwap（AMM，base ≥ 10）
  0 amm_program（必须 executable）
  1 global_config（PDA: "global_config"）
  2 pool_state
  3 base_mint
  4 quote_mint
  5 coin_creator
  6 event_authority（PDA: "__event_authority"）
  7 creator_vault_authority（PDA: "creator_vault"+creator）
  8 creator_vault_ata（owner=creator_vault_authority, mint=quote）
  9 pool_base_ata（owner=pool_state, mint=base）
  10 pool_quote_ata（owner=pool_state, mint=quote）
  [可选] fee_recipient
  [可选] fee_recipient_ata（owner=fee_recipient, mint=quote）

## 链上校验与路由（execute_arbitrage_v4.rs）
- 基础：mints_count>1、steps.len==mints_count；step_accounts 总长度与 steps_meta 对齐
- 用户 SPL 账户校验：program ∈ {Token,Token-2022} 且 mint/owner 匹配
- 每步 program_id 必须 executable
- 逐步路由：余额差法获取真实 amount_out，最终校验 input_amount + min_profit_lamports

## 与直连顺序对比（参考 swaps.rs 内部构造）

- Raydium CPMM 直连 swap_base_input（参考方向）：
```
payer, authority, amm_config, pool_state, user_in_ata, user_out_ata,
input_vault, output_vault, input_token_program, output_token_program,
input_mint, output_mint, observation_state
```
- V4 本步切片：
```
program_id, amm_config, pool_state, token0_vault, token1_vault, observation_state,
[authority?], [token_2022_program?]   // 用户 in/out 账户在 user_accounts 段；
                                      // token program/memo 多在 CLMM 切片或 Anchor 固定账户
```
映射要点：V4 不携带 payer/user_atas（在前两段）；V4 的 input/output_vault 由路由依据 in_mint 与 vault_mint 匹配推断。

- Raydium CLMM 直连（参考 v2）：
```
payer, amm_config, pool_state, user_in_ata, user_out_ata,
input_vault, output_vault, observation,
 token_program, token_2022, memo,
 input_vault_mint, output_vault_mint,
 [tick_ext?], tick_arrays...
```
- V4 本步切片：
```
clmm_program, amm_config, pool_state, input_vault, output_vault, observation,
token_program, token_program_2022, memo_program, input_vault_mint, output_vault_mint,
[tick_ext?], tick_arrays...
```
映射要点：V4 将 clmm_program 显式放入切片首位；用户 in/out 账户与 mints 在前两段；tick 动态在切片尾部显式追加。

## QA/联调检查清单
- 客户端：
  - 输出 mints_count、steps_count、各 steps[i].accounts_len 与总和；
  - 打印 mints 与 user_accounts 对应关系（第 i 个 mint ↔ 第 i 个用户账户）。
- 合约：
  - 观察 program_id executable 校验与“三段长度一致性”校验日志；
  - CPMM：校验 input_mint 与 token0/1_vault 的 mint 匹配（由路由内部完成）。
- CLMM：
  - 打印 tick_array_extension（是否存在）与 tick_arrays 数量/地址，与直连解析结果对比；
  - 数据缺失（pool_info 或 bitmap 扩展）时应 error 并返回。
- Pump 系列：
  - fee_recipient/creator_vault/volume accumulators 为可选，尽力提供；
  - 直连与 V4 的差异主要在用户账户段与程序账户位置，CPI metas 最终需一致。
