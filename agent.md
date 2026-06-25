# Agent Guide

## Project Role

This repository contains an Anchor-based Solana program for executing multi-step arbitrage routes across supported DEX protocols. The on-chain program exposes one main instruction, `execute_arbitrage`, and expects the client/searcher to provide the route, account layout, protocol metadata, and profit threshold.

The contract does not discover opportunities by itself. Off-chain systems are responsible for:

- finding profitable closed-loop routes;
- building the ordered mint list and user token accounts;
- collecting each protocol-specific account group;
- setting `input_amount`, `min_profit_lamports`, `direction`, and `fee_rate`;
- submitting the transaction with all required `remaining_accounts`.

## Important Files

- `src/lib.rs`: Anchor program declaration, program id, and public instruction entry.
- `src/instructions/execute_arbitrage.rs`: main route validation, account slicing, protocol dispatch, and final profit check.
- `src/state.rs`: serializable instruction parameters, route step metadata, and protocol enum.
- `src/instructions/types.rs`: shared swap result type and SPL Token/Token-2022 balance reader.
- `src/errors.rs`: custom program errors.
- `src/protocal/*.rs`: protocol-specific CPI instruction builders.
- `idls/*.json`: external protocol IDLs used as references.
- `Anchor.toml`: localnet/devnet/mainnet program ids and provider config.

Note: the directory is named `protocal`, not `protocol`. Keep the existing spelling unless doing a deliberate migration across all imports.

## Current Instruction Contract

`execute_arbitrage(ctx, params)` receives no fixed Anchor accounts in `ExecuteArbitrage`; all operational accounts are passed through `ctx.remaining_accounts`.

`remaining_accounts` must be ordered as:

1. `payer`
2. `system_program`
3. `associated_token_program`
4. `token_program`
5. `token_2022_program`
6. `mints[M]`
7. `user_token_accounts[M]`
8. flattened per-step protocol accounts

The fixed program accounts at indexes 1 through 4 are validated against the canonical System, Associated Token, SPL Token, and Token-2022 program ids. `params.input_amount` must be greater than zero.

Each route step must use `direction` 0 or 1 and `fee_rate <= 10_000`. The route mints must be owned by either SPL Token or Token-2022, and user token accounts are checked against the corresponding mint and token program before CPI dispatch.

`M = params.mints_count`. The number of steps must equal `M`, forming a closed loop:

- step `i` input mint is `mints[i]`;
- step `i` output mint is `mints[(i + 1) % M]`;
- user input/output token accounts use the same indexes.

Each `SwapStepMeta.accounts_len` tells the program how many flattened accounts belong to that step.

## Supported Protocols

`Protocol` in `src/state.rs` currently supports:

- `RaydiumCPMM`
- `RaydiumCLMM`
- `RaydiumPoolV4`
- `RaydiumLaunchPad`
- `PumpFunSwap`
- `PumpFunAMM`

Each protocol module manually builds a Solana `Instruction` with hard-coded discriminators/selectors and invokes the external program through CPI.

## Execution Model

The route executes sequentially:

1. Start with `params.input_amount`.
2. Dispatch each step to the matching protocol adapter.
3. Measure the real output with the balance-difference method.
4. Use the measured output as the next step's input.
5. After the final step, require:

```text
final_amount >= input_amount + min_profit_lamports
```

Most step-level CPI calls currently pass `minimum_amount_out = 0`. The effective protection is the final route-level profit check, not per-leg slippage protection.

## Development Rules For Agents

- Preserve the `remaining_accounts` ABI unless the client-side route builder is updated at the same time.
- Keep protocol account ordering documented when changing a CPI adapter.
- Treat `direction` semantics carefully. They are not fully uniform across all adapters.
- Do not assume every token is SPL Token classic. The main dispatcher now explicitly accepts only SPL Token and Token-2022 mints.
- Avoid broad refactors in protocol adapters unless backed by integration tests or mainnet/devnet transaction simulation.
- Prefer adding focused tests around parameter serialization, account slicing, direction mapping, and balance-difference behavior.
- Be careful with `Anchor.toml` provider wallet paths and `declare_id!`; program ids differ by cluster.

## Known Risk Areas

- No static account constraints are enforced by Anchor because accounts are supplied dynamically.
- DEX CPI program ids are taken from each step's `remaining_accounts`, so client-side validation and transaction construction remain critical.
- Per-leg minimum output is generally zero; profitable execution depends on final balance/profit validation.
- Raydium Pool V4 now expects the full AMM/OpenBook account group; older 5-account step slices are no longer accepted.
- Arithmetic helper functions in PumpFun simulation use plain `u64` multiplication in places where overflow should be considered.
- There is no repository-level test suite present besides a keypair utility test in `src/lib.rs`.
