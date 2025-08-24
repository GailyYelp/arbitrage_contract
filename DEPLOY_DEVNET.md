### arbitrage_contract 在 devnet 的部署与升级指南

本文档描述如何将本仓库的 `arbitrage_contract` 程序部署到 Solana devnet，并在后续执行升级与 IDL 同步。已结合当前工程状态与已创建的链上 Program。

---

#### 基本信息
- Program 名称: `arbitrage_contract`
- Program Id: `4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC`
- 升级权限钱包: `/Users/zhengwei/Desktop/wallet-keypair.json` (公钥: `8HSPyUXh5geS1CeYN9cNx5rTprKzYuBvtdUb8S3qxY51`)
- 工程目录: `/Users/zhengwei/CursorProjects/arbitrage_contract`
- 当前依赖矩阵: Anchor 0.31.1 + Solana 2.x（本机通过 anza 安装，已具备 `cargo-build-sbf`）

---

#### 环境准备
1) 安装/更新 Solana 工具链（已就绪可跳过）
```bash
curl -sSfL https://release.anza.xyz/v2.2.16/install | sh -s -
echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
solana --version
```

2) Anchor 版本确认
```bash
anchor --version   # 0.31.1
```

3) 切换到 devnet，并使用升级权限钱包
```bash
solana config set --url https://api.devnet.solana.com
solana config set -k /Users/zhengwei/Desktop/wallet-keypair.json
solana address     # 应输出 8HSPyU...
```

---

#### 配置检查（必须一致）
- `Anchor.toml`
```toml
[programs.devnet]
arbitrage_contract = "4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC"

[provider]
cluster = "devnet"
wallet = "/Users/zhengwei/Desktop/wallet-keypair.json"
```

- `src/lib.rs`
```rust
declare_id!("4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC");
```

---

#### 一、首次部署到 devnet
```bash
cd /Users/zhengwei/CursorProjects/arbitrage_contract

# 构建 SBF
anchor build

# 部署（会进行 createAccount、扩容与大量 write 分片写入）
anchor deploy

# 初始化上链 IDL（若链上无 IDL 才需要 init；已有则用 upgrade）
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
anchor idl init -f target/idl/arbitrage_contract.json 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC \
  --provider.wallet /Users/zhengwei/Desktop/wallet-keypair.json \
  --provider.cluster devnet

# 验证程序与 IDL
solana program show 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com anchor idl fetch 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC | head
```

---

#### 二、升级已部署的程序
1) 修改代码并保持 `declare_id!` 与 `Anchor.toml` 的 ProgramId 不变。
2) 重新构建并升级：
```bash
anchor build
anchor deploy   # 或 anchor upgrade <PROGRAM_ID>

# 同步 IDL（常用 upgrade；IDL 有变化时运行）
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
anchor idl upgrade -f target/idl/arbitrage_contract.json 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC \
  --provider.wallet /Users/zhengwei/Desktop/wallet-keypair.json \
  --provider.cluster devnet
```

---

#### 三、回收租金（下架程序，可选）
```bash
solana program close 4ZqQT3aUpSMiAjmyaYj6yHjfJQH6k7v3XBSpgAhWU8uC \
  --url https://api.devnet.solana.com \
  -k /Users/zhengwei/Desktop/wallet-keypair.json
```
说明：关闭后 Program/ProgramData 账户的租金会退回到签名者；历史“交易费”不退回。


---

#### 六、参考
- 部署指南（Solana 官方）: https://solana.com/zh/docs/intro/quick-start/deploying-programs


