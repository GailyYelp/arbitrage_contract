# Business Overview

## Product Purpose

This project is an on-chain arbitrage execution contract for Solana. Its business purpose is to let an off-chain searcher submit a precomputed multi-step swap route and have the contract execute all legs atomically in one transaction.

If the final amount does not exceed the original input amount by the configured minimum profit, the transaction fails.

## Target User

The direct user is not a retail trader. The target users are:

- arbitrage searchers;
- market-making or MEV infrastructure;
- trading bots that need atomic multi-DEX execution;
- operators maintaining route builders and transaction submitters.

## Supported Trading Venues

The current code supports adapters for:

- Raydium CPMM
- Raydium CLMM
- Raydium Pool V4
- Raydium LaunchPad
- PumpFun Swap
- PumpFun AMM

This gives the route builder coverage across major Solana liquidity surfaces used by meme-token and long-tail token trading.

## Business Flow

1. Off-chain infrastructure monitors pools, prices, fees, and liquidity.
2. The searcher identifies a closed-loop route such as `A -> B -> C -> A`.
3. The client builds route metadata and all protocol accounts.
4. The transaction calls `execute_arbitrage`.
5. The contract executes each swap in order.
6. The output of each step becomes the input of the next step.
7. The contract checks final profit.
8. If the profit threshold is met, the route settles; otherwise the whole transaction reverts.

## Profit Rule

The final route must satisfy:

```text
final_amount >= input_amount + min_profit_lamports
```

`min_profit_lamports` is a caller-provided safety threshold. It should cover:

- expected transaction fees;
- priority fees / tips;
- route simulation error;
- slippage and pool state movement;
- desired net profit.

## What The Contract Does

- Executes a caller-supplied closed-loop arbitrage route.
- Supports multiple DEX protocols in a single transaction.
- Measures real swap output by balance differences.
- Supports SPL Token and Token-2022 mint ownership routing.
- Enforces a final minimum-profit condition.

## What The Contract Does Not Do

- It does not search for arbitrage opportunities.
- It does not price pools on chain.
- It does not maintain persistent strategy state.
- It does not protect each individual leg with a meaningful min-out value in most adapters.
- It does not validate all external program ids or pool account relationships through Anchor constraints.

## Operational Dependencies

The off-chain system is critical. It must correctly provide:

- route mint order;
- user token account order;
- per-step protocol type;
- per-step account list and account count;
- direction flags;
- fee rates for PumpFun-style simulations;
- profit threshold;
- transaction priority settings.

Bad client-side construction can cause failed transactions or unsafe execution attempts.

## Main Business Risks

- Market movement between simulation and landing can erase profit.
- Missing per-leg slippage limits can allow a bad intermediate fill, with only final profit as protection.
- Incorrect direction flags can invert trade semantics.
- Incorrect protocol account order can fail CPI or route funds through unintended pools.
- Program id and pool validation are mostly delegated to the client.
- Some protocol adapters appear incomplete or need production verification.

## Recommended Product Roadmap

- Add client-side route builder documentation matching the exact `remaining_accounts` ABI.
- Add devnet/mainnet simulation scripts for every supported protocol.
- Add per-step `minimum_amount_out` support when feasible.
- Add optional program id allowlists for supported protocols.
- Add stronger account validation for mints, vaults, pool ownership, and token accounts.
- Add observability through structured `msg!` logs for route id, step index, protocol, input, output, and final profit.
- Add integration tests using recorded or forked accounts.

## Success Metrics

Useful business and operational metrics include:

- successful arbitrage transaction count;
- failed transaction count by error type;
- realized gross and net profit;
- average priority fee and compute cost;
- route latency from detection to landing;
- protocol mix by volume and profit;
- failure rate by protocol adapter.

