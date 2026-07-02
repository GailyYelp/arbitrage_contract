# Technical Overview

## Stack

- Language: Rust 2021
- Framework: Anchor `0.31.1`
- Runtime: Solana BPF program
- SPL helpers: `anchor-spl 0.31.1`
- Program type: `cdylib` and Rust library

The release profile enables overflow checks:

```toml
[profile.release]
overflow-checks = true
```

## Program Identity

Program ids are configured for three clusters in `Anchor.toml`:

- localnet: `7R2DEVjE6DQPsQYnLSaFwPQjp1RN5vR8BAfxNhociMxV`
- devnet: `9Vj7SrxY3mQdw48xZ29r53ZA2rjNiniTA7KNsjH8ubkK`
- mainnet: `BnvW7qAWpur6tcVHsmkgRjiH1orAc7ERL6aVymBPpNEB`

`src/lib.rs` currently declares the mainnet id:

```rust
declare_id!("BnvW7qAWpur6tcVHsmkgRjiH1orAc7ERL6aVymBPpNEB");
```

Feature flags:

- `devnet`: intended for devnet-specific constants or validation behavior.
- `flex`: reserved for more flexible validation behavior.
- `idl-build`: enables Anchor IDL build support.

Contract ABI limits are anchored by the repository-level `contract_abi.json`.
`build.rs` generates a read-only ABI constants module at compile time, and
`src/state.rs` re-exports `CONTRACT_PROTOCOL_VERSION`,
`CONTRACT_PROTOCOL_COUNT`, `CLIENT_MAX_SUPPORTED_PATH_LENGTH`, and
`REMAINING_ACCOUNTS_FIXED_PREFIX_LEN` from that generated module. The on-chain
`Protocol` enum remains explicit and auditable; `Protocol::contract_id()` uses
an explicit match for ABI ids instead of relying on enum discriminant casts.
Tests compare the enum with the generated manifest entries and the Borsh field
layout.

## Module Layout

```text
src/
  lib.rs
  errors.rs
  state.rs
  instructions/
    execute_arbitrage.rs
    mod.rs
    types.rs
  protocal/
    raydium_cpmm.rs
    raydium_clmm.rs
    raydium_pool_v4.rs
    raydium_launchpad.rs
    pumpfun_swap.rs
    pumpfun_amm.rs
```

## Public API

The program exposes one Anchor instruction:

```rust
pub fn execute_arbitrage<'info>(
    ctx: Context<'_, '_, 'info, 'info, ExecuteArbitrage>,
    params: SwapArbParams,
) -> Result<()>
```

`ExecuteArbitrage` is empty, so all accounts are dynamic `remaining_accounts`.

## Parameters

`SwapArbParams`:

```rust
pub struct SwapArbParams {
    pub mints_count: u8,
    pub input_amount: u64,
    pub min_profit_lamports: u64,
    pub steps: Vec<SwapStepMeta>,
}
```

`SwapStepMeta`:

```rust
pub struct SwapStepMeta {
    pub protocol: Protocol,
    pub accounts_len: u8,
    pub direction: u8,
    pub fee_rate: u16,
    pub min_output_amount: u64,
}
```

Validation performed by `execute_arbitrage`:

- `mints_count > 1`
- `input_amount > 0`
- `steps.len() == mints_count`
- every step has `direction` 0 or 1
- every step has `fee_rate < 10_000`
- every step enforces `min_output_amount` after CPI when non-zero
- `remaining_accounts.len() >= 2 * mints_count + 5`
- flattened step account count equals the sum of `steps[*].accounts_len`
- fixed program accounts match System, Associated Token, SPL Token, and Token-2022 program ids
- route mints are owned by SPL Token or Token-2022
- user token accounts match their route mint and token program owner

## Account Layout

The dynamic account list is parsed as:

```text
[0] payer
[1] system_program
[2] associated_token_program
[3] token_program
[4] token_2022_program
[5 .. 5 + M] mints
[5 + M .. 5 + 2M] user token accounts
[5 + 2M ..] step account groups
```

For each step:

- `in_mint = mints[i]`
- `out_mint = mints[(i + 1) % M]`
- `user_in = user_accounts[i]`
- `user_out = user_accounts[(i + 1) % M]`
- `step_slice = remaining_accounts[cursor .. cursor + accounts_len]`

Token program selection is inferred from the mint owner:

```rust
if mint.owner == token_program.key() {
    token_program
} else if mint.owner == token_2022_program.key() {
    token_2022_program
} else {
    InvalidTokenMint
}
```

## Protocol Account Slices

The dispatcher expects each step slice to start with the target protocol program account.

### Raydium CPMM

Minimum slice length: `7`

```text
[0] program
[1] authority
[2] amm_config
[3] pool_state
[4] input_vault
[5] output_vault
[6] observation_state
```

Selector: `[143, 190, 90, 218, 196, 30, 51, 222]`

### Raydium CLMM

Minimum slice length: `8`

```text
[0] program
[1] amm_config
[2] pool_state
[3] input_vault
[4] output_vault
[5] observation_state
[6] memo_program
[7..] tick array / extension accounts
```

Selector: `[43, 4, 237, 11, 26, 201, 30, 98]`

The adapter appends all extra accounts to the CPI metas, preserving writable flags.

### Raydium Pool V4

Minimum slice length: `15`

```text
[0] program
[1] pool_state
[2] amm_authority_info
[3] amm_open_orders
[4] amm_target_orders
[5] coin_vault
[6] pc_vault
[7] market_program
[8] market
[9] market_bids
[10] market_asks
[11] market_event_queue
[12] market_coin_vault
[13] market_pc_vault
[14] market_vault_signer
```

Selector: `[9]`

### Raydium LaunchPad

Minimum slice length: `8`

```text
[0] program
[1] authority
[2] global_config
[3] platform_config
[4] pool_state
[5] base_vault
[6] quote_vault
[7] event_authority
```

Direction mapping in dispatcher:

- `direction == 0`: buy branch maps output mint/user account as base and input as quote.
- otherwise: sell branch maps input as base and output as quote.

The adapter currently chooses selectors as:

- `direction == 0`: sell selector
- otherwise: buy selector

This mismatch should be verified against the client convention and Raydium LaunchPad IDL.

### PumpFun Swap

Minimum slice length: `7`

```text
[0] program
[1] global_account
[2] fee_recipient
[3] pool_id
[4] token_vault0
[5] creator_vault
[6] event_authority
[7..] volume accumulator / extra accounts
```

Direction mapping in adapter:

- `direction == 0`: sell token to SOL
- otherwise: buy token with SOL

For buy, the adapter simulates token output from pool reserves and `fee_rate`, then sends that as the requested token amount with `amount_in` as max SOL cost.

### PumpFun AMM

Minimum slice length: `10`

```text
[0] program
[1] pool_state
[2] global_config
[3] pool_base_token_account
[4] pool_quote_token_account
[5] fee_recipient
[6] fee_recipient_ata
[7] event_authority
[8] coin_creator_vault_ata
[9] coin_creator_vault_authority
[10..] volume accumulator / extra accounts
```

Direction mapping:

- `direction == 0`: sell base to quote
- `direction == 1`: buy base with quote

For buy, the adapter simulates base output from pool balances and `fee_rate`.

## Balance-Difference Output Accounting

Every adapter reads the output balance before CPI and after CPI:

```rust
let amount_out = post_out.saturating_sub(pre_out);
```

For SPL Token and Token-2022 accounts, `read_token_amount` reads bytes `64..72`, the canonical token account amount field.

For PumpFun Swap sell paths, SOL output is measured through payer lamports instead of a token account.

## Error Model

Custom errors are in `src/errors.rs`. The main path currently uses:

- `InvalidAccountCount`
- `InvalidAccountIndex`
- `InvalidAccount`
- `InsufficientProfit`

Many other error variants are defined for future validation or richer adapters.

## Build And Deploy Notes

Common commands documented in `src/lib.rs`:

```bash
anchor build
anchor build -- --features devnet
anchor deploy --program-name arbitrage_contract
```

Before deployment, keep these in sync:

- `declare_id!` in `src/lib.rs`
- `[programs.<cluster>].arbitrage_contract` in `Anchor.toml`
- client-side `ARBITRAGE_CONTRACT_ID`

## Testing Gaps

The repository does not currently include end-to-end Anchor tests. High-value tests to add:

- Borsh serialization compatibility for `SwapArbParams`.
- Account slicing boundaries for valid and invalid routes.
- Direction mapping tests for each protocol.
- Token vs Token-2022 program selection.
- PumpFun simulation overflow and zero-input cases.
- Devnet/mainnet transaction simulation for each protocol adapter.
