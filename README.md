# Lending & borrowing on Solana: Jupiter Lend Substreams

Jupiter Lend lending & borrowing on Solana (Fluid architecture): Earn deposits and withdrawals, Borrow vault positions and liquidations, liquidity-layer supply/borrow flows, exchange prices, rates, revenue, rewards and the Lend DEX, delivered as Substreams modules and a PostgreSQL sink.

- Package: `jupiter-lend-substreams` (v0.1.0)
- Source: https://github.com/PaulieB14/jupiter-lend-substreams
- Network: Solana mainnet, from slot `355648000` (programs deployed 2025-07-25)
- Crates: substreams 0.8, substreams-solana 0.16, substreams-database-change 5.0, buffa 0.9.2
- Layouts: the official Anchor IDLs from [`jup-ag/jupiter-lend`](https://github.com/jup-ag/jupiter-lend) (`idl/`)

## What it indexes

| Program | ID | Events decoded |
|---|---|---|
| Liquidity (core) | `jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC` | `LogOperate` (every supply / withdraw / borrow / payback from every protocol), `LogUpdateExchangePrices` (+ borrow rate, utilization), `LogCollectRevenue`, `LogClaim`, rate curves V1/V2, token configs (reserve factor), protocol supply/borrow configs |
| Earn (lending, fTokens) | `jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9` | `LogDeposit`, `LogWithdraw`, `LogUpdateRates` (fToken exchange price), `LogRebalance` — mint / fToken / lending account taken from the emitting instruction |
| Borrow (vaults) | `jupr81YtYssSyPt8jbnGuiWon5f6x9TcDEFxYe3Bdzi` | `LogOperate` (col/debt change per position NFT), `LogUserPosition` (position after the op: tick, col, debt), `LogLiquidate` + `LogLiquidateInfo` (ticks), `LogAbsorb`, `LogUpdateExchangePrices`, `LogRebalance`, `LogClosePosition`, vault creation and every config update, liquidation rounding diffs |
| Earn rewards | `jup7TthsMgcR9Y3L277b8Eo9uboVSmu1utkuXHNUKar` | start / queue / transition / stop / cancel reward programs |
| Lend DEX (smart col / smart debt) | `jupZ4m2GqUCJ5iueMfzQf8khFfH31d4XAQt3RzCT9Vd` | `LogSwapInfo` (amounts, fee, reserves, price), col/debt liquidity deposit/withdraw/borrow/payback, arbitrage, DEX creation, fee updates |
| Flashloan | `jupgfSgfuAXv4B6R2Uxu85Z1qdzgju79s6MfZekN6XS` | fee updates (flashloan flows appear as liquidity `LogOperate` with `protocol_type = flashloan`) |

**How events are decoded.** Jupiter Lend emits plain Anchor `emit!` events (`Program data:` logs), not self-CPI events. Liquidity and vault `LogOperate` share a discriminator, so every event is attributed to the exact emitting instruction by walking the log invoke stack against the flattened instruction list (top-level + CPI, execution order). That gives each event the IDL instruction name, the instruction's named accounts (mint, vault config, fToken, DEX …) and the calling program. Transactions whose logs were truncated by the runtime are counted in `truncated_log_tx_count`. There were 4 of 1,824 in the Oct 10, 2025 window and 0 in recent windows. Events after a truncation point are lost.

**Flow attribution.** `protocol_type` on every liquidity flow comes from the calling program: `earn`, `vault`, `dex`, `flashloan`, or `other` (direct integrators).

**Historical vault ids.** Vault-program versions before late 2025 did not emit vault ids. `src/vaults_registry.rs` (generated from the 108 on-chain `VaultConfig` accounts by `scripts/gen_vaults.py`) maps `vault_config → vault_id` for history; newer vaults are resolved from their own events.

## Modules

| Module | Kind | Output |
|---|---|---|
| `map_events` | map (block filter on the 6 program ids) | `jupiter_lend.v1.Events`: one typed message per event layout, each with a `Ctx` (slot, time, tx, instruction index, log index, instruction name, caller program, fee payer) plus the mints/decimals seen |
| `db_out` | map | `DatabaseChanges` for `substreams-sink-sql` (PostgreSQL) |

Amounts are raw base units as decimal strings (i128/u128 safe) plus exact `*_ui` strings using the decimals from the transaction's token balances. Exchange prices are 1e12 fixed point. Rates and utilization are in bps (10000 = 100%). VaultConfig `collateral_factor`, `liquidation_threshold` and `liquidation_max_limit` are 1e3 (800 = 80%), and `withdraw_gap` and `liquidation_penalty` are 1e4. For smart (DEX) vaults, col/debt changes are in DEX shares.

## SQL tables

| Group | Tables |
|---|---|
| Liquidity layer | `liquidity_operations`, `liquidity_exchange_prices`, `liquidity_revenue_collections`, `liquidity_claims`, `liquidity_rate_configs`, `liquidity_token_configs`, `liquidity_protocol_configs` |
| Earn | `earn_deposits`, `earn_withdrawals`, `earn_rebalances`, `earn_reward_events`, `earn_markets` (latest fToken price) |
| Borrow | `vaults`, `vault_operations`, `vault_position_updates`, `vault_positions` (latest per NFT), `vault_liquidations`, `vault_absorbs`, `vault_exchange_prices`, `vault_rebalances`, `vault_position_closes`, `vault_config_updates`, `vault_liquidation_rounding_diffs` |
| DEX | `dexes`, `dex_swaps`, `dex_liquidity`, `dex_arbitrages`, `flashloan_fee_updates` |
| Daily (UTC) | `liquidity_daily` (mint × protocol_type flows), `token_daily` (net flows, first/last exchange prices, borrow rate, utilization, revenue collected), `earn_daily`, `vault_daily` (flows, liquidations), `dex_daily` (swaps, volume, fees) |
| Reference | `tokens` (mint → decimals) and views `v_liquidity_operations`, `v_liquidity_daily`, `v_vault_liquidations` in human units |

Event tables use the primary key `<tx_id>:<log_index>`. Latest-state and daily tables are delta upserts.

```bash
substreams-sink-sql setup "psql://user:pass@host:5432/db?sslmode=disable" jupiter-lend-substreams-v0.1.0.spkg
substreams-sink-sql run   "psql://user:pass@host:5432/db?sslmode=disable" jupiter-lend-substreams-v0.1.0.spkg
```

### Example queries

```sql
-- Daily realized borrow APR per token, from exchange-price growth
SELECT day, mint,
       (last_borrow_exchange_price / NULLIF(first_borrow_exchange_price,0) - 1) * 365 * 100 AS borrow_apr_pct_approx,
       last_utilization_bps / 100.0 AS utilization_pct
FROM token_daily ORDER BY day DESC, mint;

-- Net flows into the liquidity layer by product, last 7 days (human units)
SELECT day, protocol_type, mint, supplied_ui - withdrawn_ui AS net_supply, borrowed_ui - repaid_ui AS net_borrow
FROM v_liquidity_daily WHERE day >= to_char(now() - interval '7 days', 'YYYY-MM-DD') ORDER BY day, protocol_type;

-- Liquidations by vault
SELECT vault_id, count(*) AS liquidations, sum(col_amount_ui) AS col_seized, sum(debt_amount_ui) AS debt_repaid
FROM v_vault_liquidations GROUP BY 1 ORDER BY 2 DESC;

-- Top liquidators
SELECT liquidator, count(*) n, sum(debt_amount) AS debt_raw FROM vault_liquidations GROUP BY 1 ORDER BY 2 DESC LIMIT 20;

-- Largest open borrow positions (vault program versions that emit LogUserPosition)
SELECT p.vault_id, p.nft_id, p.owner, p.debt / power(10, t.decimals) AS debt, v.borrow_mint, p.tick
FROM vault_positions p JOIN vaults v ON v.vault_id = p.vault_id LEFT JOIN tokens t ON t.mint = v.borrow_mint
WHERE p.debt > 0 ORDER BY debt DESC LIMIT 20;

-- One wallet's Earn history
SELECT to_timestamp(block_time) ts, 'deposit' kind, mint, assets FROM earn_deposits WHERE receiver = :'wallet'
UNION ALL
SELECT to_timestamp(block_time), 'withdraw', mint, -assets FROM earn_withdrawals WHERE owner = :'wallet'
ORDER BY ts;

-- Revenue actually collected to the treasury
SELECT mint, sum(revenue_amount) FROM liquidity_revenue_collections GROUP BY 1;
```

**Fees and TVL.** Events carry flows and exchange prices, not the reserve totals stored in account state. Daily borrower interest is `total_borrow × Δborrow_exchange_price / exchange_price`, and `token_daily` keeps the first and last exchange price per day for this. Absolute TVL needs either a full sync from the start block (sum of flows) or the reserve totals from RPC or the Lend API.

## Verification (v0.1.0)

- **Field by field against Solana RPC.** Every event in 2,776 transactions from slots 461,600,000–461,620,000 (Oct 2026) was compared with an independent Python IDL decoder (`verify/`). That covered 7,536 events and 78,111 fields, with 0 differences, 0 missing and 0 extra. A high-volatility window was checked the same way: the Oct 10, 2025 crash, slots 372,515,000–372,527,000, 1,793 transactions. That covered 4,903 events and 42,667 fields, including 145 liquidations and 815 vault operations, with 0 differences. Two events came from one transaction the RPC no longer returned.
- **Rare admin events.** All 576 signatures of the AuthorizationList account were scanned. Up to 3 real examples of each admin event type were run through `map_events` and compared with the Python decoder: 28 transactions and 480 fields, with 0 differences, 0 missing and 0 extra. Types covered: liquidity rate data V1/V2, token configs, user supply/borrow configs, collect revenue, vault creation, vault param updates, vault exchange prices and DEX creation. Pause/lockdown, auth and most DEX config events are not decoded, by design.
- **Consistency across programs.** All 816 T1 vault `LogOperate` col/debt deltas in that window equal the liquidity-layer `LogOperate` flows of the same instruction, so the amounts are in native units and actual amounts are emitted, not `i128::MIN` sentinels.
- **Completeness.** The set of transactions with Lend events matches the program signature sets from `getSignaturesForAddress`.
- **Speed.** On Pinax, `db_out` in production mode ran 30,000 recent slots in 91 s, and the block filter cut that to 2,911 processed blocks.
- **Postgres** (`substreams-sink-sql`). Row counts equal the `map_events` output up to the last flushed block, and the daily aggregates equal the sums of the event rows.
- **Cross-checks.**
  - fToken exchange prices match the Jupiter Lend API `convertToAssets` to 4–5 significant figures.
  - Realized 24h fToken APYs are within 0.5 pp of the API `totalRate`.
  - Borrow interest on Oct 25, 2026 from exchange-price growth (USDC, SOL and USDT only) came to $100.3k, against DefiLlama `dailyFees` of $111.0k.

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
substreams pack
cargo test          # unit tests + real-transaction fixtures (tests/fixtures)
python3 scripts/gen_idl.py idl      # regenerate src/idl.rs from the IDLs
python3 scripts/gen_vaults.py       # refresh the vault registry from mainnet
```

## License

MIT
