### devnet 部署/升级速记（必要信息）

#### 基本信息
- Program: `arbitrage_contract`
- Program Id: `4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC`
- 升级权限钱包: `/Users/zhengwei/Desktop/wallet-keypair.json`

#### 配置检查（三处一致）
- Anchor.toml
```toml
[programs.devnet]
arbitrage_contract = "4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC"

[provider]
cluster = "devnet"
wallet = "/Users/zhengwei/Desktop/wallet-keypair.json"
```

- src/lib.rs
```rust
declare_id!("4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC");
```

#### 首次部署
```bash
cd /Users/zhengwei/CursorProjects/arbitrage_contract
anchor build
anchor deploy

# 若链上没有 IDL（首次），初始化一次；否则跳过
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
anchor idl init -f target/idl/arbitrage_contract.json 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC

# 验证
solana program show 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com anchor idl fetch 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC | head
```

#### 升级（代码或 IDL 变化）
```bash
anchor build
anchor deploy   # 或：anchor upgrade <PROGRAM_ID>

# 若 IDL 有变化（常用）
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
anchor idl upgrade -f target/idl/arbitrage_contract.json 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC
```

#### 可选：显式切换 devnet/钱包
```bash
solana config set --url https://api.devnet.solana.com
solana config set -k /Users/zhengwei/Desktop/wallet-keypair.json
```

#### 参考
- Solana 官方部署文档: https://solana.com/zh/docs/intro/quick-start/deploying-programs