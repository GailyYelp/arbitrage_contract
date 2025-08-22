# 套利合约地址总结

## 核心程序 ID

### DEX 程序
- **Raydium CPMM**: `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C`
- **Raydium CLMM**: `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`
- **PumpFun**: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`
- **PumpSwap**: `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`

### 系统程序
- **Token Program**: `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
- **Token 2022**: `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`
- **Associated Token**: `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL`
- **Memo Program**: `MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr`
- **System Program**: `11111111111111111111111111111111`

## 固定账户地址

### Raydium CPMM
- **Authority**: `GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL`

### PumpFun Bonding Curve
- **Global Account**: `4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf`
- **Fee Recipient**: `62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV`
- **Event Authority**: `Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1`

### PumpSwap AMM
- **Global Config**: `ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw`
- **Fee Recipient**: `62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV`
- **Fee Recipient ATA**: `94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb`
- **Event Authority**: `GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR`
- **AMM Program**: `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`

### 代币相关
- **Wrapped SOL**: `So11111111111111111111111111111111111111112`

## 指令选择器 (Discriminators)

### Raydium
- **CPMM Swap Base In**: `[143, 190, 90, 218, 196, 30, 51, 222]`
- **CLMM Swap V2**: `[43, 4, 237, 11, 26, 201, 30, 98]`

### PumpFun
- **Buy**: `[102, 6, 61, 18, 1, 218, 235, 234]`
- **Sell**: `[51, 230, 133, 164, 1, 127, 131, 173]`

### PumpSwap
- **Buy**: `[102, 6, 61, 18, 1, 218, 235, 234]`
- **Sell**: `[51, 230, 133, 164, 1, 127, 131, 173]`

## PDA 种子 (Seeds)

### PumpFun
- **Global**: `b"global"`
- **Bonding Curve**: `b"bonding-curve"`
- **Creator Vault**: `b"creator-vault"`
- **Mint Authority**: `b"mint-authority"`
- **Event Authority**: `b"__event_authority"`
- **Global Volume Accumulator**: `b"global_volume_accumulator"`
- **User Volume Accumulator**: `b"user_volume_accumulator"`

### PumpSwap
- **Global Config**: `b"global_config"`
- **Pool**: `b"pool"`
- **LP Mint**: `b"pool_lp_mint"`
- **Creator Vault**: `b"creator_vault"`
- **Event Authority**: `b"__event_authority"`
- **Global Volume Accumulator**: `b"global_volume_accumulator"`
- **User Volume Accumulator**: `b"user_volume_accumulator"`

## PDA 推导函数

合约提供了以下 PDA 推导辅助函数：

1. `derive_pumpfun_bonding_curve(mint, program_id)`
2. `derive_pumpfun_creator_vault(creator, program_id)`
3. `derive_pumpfun_global_volume_accumulator(program_id)`
4. `derive_pumpfun_user_volume_accumulator(user, program_id)`
5. `derive_pumpswap_global_config(program_id)`
6. `derive_pumpswap_creator_vault(creator, amm_program)`

## 使用方式

```rust
use crate::account_derivation::types::{
    ProgramIds, FixedAddresses, get_fixed_addresses,
    instruction_discriminators, pda_seeds, pda_utils
};

// 获取程序 ID
let program_ids = ProgramIds::default();

// 获取固定地址
let fixed_addresses = get_fixed_addresses()?;

// 使用指令选择器
let discriminator = instruction_discriminators::PUMPFUN_BUY;

// PDA 推导
let bonding_curve = pda_utils::derive_pumpfun_bonding_curve(
    &mint, 
    &program_ids.pumpfun
)?;
```

所有地址都从 `money_donkey` 项目中的实际配置文件和指令文件中提取，确保与客户端完全一致。