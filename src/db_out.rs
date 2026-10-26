//! `db_out`: Jupiter Lend events -> DatabaseChanges for substreams-sink-sql (PostgreSQL).
//!
//! * Event tables use `create_row` keyed by `<tx_id>:<log_index>` (unique per event).
//! * Latest-state tables (`tokens`, `vaults`, `dexes`, `vault_positions`) and daily aggregates
//!   use `upsert_row` (merged in-block, so the same key twice in one block is safe).
//! * Raw amounts are NUMERIC strings (never empty). Human amounts are derived in SQL views via
//!   `tokens.decimals` (see schema.sql) so an unknown decimals value can never break inserts.
use substreams::errors::Error;
use substreams_database_change::pb::sf::substreams::sink::database::v1::DatabaseChanges;
use substreams_database_change::tables::{Row, Tables};

use crate::pb::jupiter_lend::v1::*;
use crate::util::utc_day;

fn ctx_of(c: &buffa::MessageField<Ctx, buffa::Inline<Ctx>>) -> Ctx {
    c.as_option().cloned().unwrap_or_default()
}

fn id(c: &Ctx) -> String {
    format!("{}:{}", c.tx_id, c.log_index)
}

fn base(row: &mut Row, c: &Ctx) {
    row.set("tx_id", &c.tx_id)
        .set("log_index", c.log_index)
        .set("slot", c.slot)
        .set("block_time", c.block_time)
        .set("instruction", &c.instruction)
        .set("instruction_index", c.instruction_index)
        .set("caller_program", &c.caller_program)
        .set("fee_payer", &c.fee_payer);
}

/// Normalize an integer string for a NUMERIC column (never empty).
fn num(v: &str) -> String {
    if v.is_empty() {
        "0".into()
    } else {
        v.to_string()
    }
}

fn i(v: &str) -> i128 {
    v.parse::<i128>().unwrap_or(0)
}

#[substreams::handlers::map]
pub fn db_out(ev: Events) -> Result<DatabaseChanges, Error> {
    let mut t = Tables::new();

    for tk in &ev.tokens {
        t.upsert_row("tokens", tk.mint.as_str())
            .set("decimals", tk.decimals)
            .set("token_program", &tk.token_program);
    }

    // ------------------------------------------------------------ liquidity layer
    for e in &ev.liquidity_operates {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_operations", id(&c));
        base(r, &c);
        r.set("protocol", &e.protocol)
            .set("protocol_type", &e.protocol_type)
            .set("mint", &e.mint)
            .set("supply_amount", num(&e.supply_amount_raw))
            .set("borrow_amount", num(&e.borrow_amount_raw))
            .set("withdraw_to", &e.withdraw_to)
            .set("borrow_to", &e.borrow_to)
            .set("supply_exchange_price", num(&e.supply_exchange_price))
            .set("borrow_exchange_price", num(&e.borrow_exchange_price));

        let (s, b) = (i(&e.supply_amount_raw), i(&e.borrow_amount_raw));
        let day = utc_day(c.block_time);
        let d = t.upsert_row(
            "liquidity_daily",
            [("day", day.clone()), ("mint", e.mint.clone()), ("protocol_type", e.protocol_type.clone())],
        );
        d.add("operations", 1i64)
            .add("supplied", s.max(0))
            .add("withdrawn", (-s).max(0))
            .add("borrowed", b.max(0))
            .add("repaid", (-b).max(0));
        let x = t.upsert_row("token_daily", [("day", day), ("mint", e.mint.clone())]);
        x.add("operations", 1i64)
            .add("net_supply_change", s)
            .add("net_borrow_change", b)
            .set_if_null("first_supply_exchange_price", num(&e.supply_exchange_price))
            .set_if_null("first_borrow_exchange_price", num(&e.borrow_exchange_price))
            .set("last_supply_exchange_price", num(&e.supply_exchange_price))
            .set("last_borrow_exchange_price", num(&e.borrow_exchange_price))
            .set("last_slot", c.slot);
    }
    for e in &ev.liquidity_exchange_prices {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_exchange_prices", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("supply_exchange_price", num(&e.supply_exchange_price))
            .set("borrow_exchange_price", num(&e.borrow_exchange_price))
            .set("borrow_rate_bps", e.borrow_rate_bps)
            .set("utilization_bps", e.utilization_bps);
        t.upsert_row("token_daily", [("day", utc_day(c.block_time)), ("mint", e.mint.clone())])
            .set_if_null("first_supply_exchange_price", num(&e.supply_exchange_price))
            .set_if_null("first_borrow_exchange_price", num(&e.borrow_exchange_price))
            .set("last_supply_exchange_price", num(&e.supply_exchange_price))
            .set("last_borrow_exchange_price", num(&e.borrow_exchange_price))
            .set("last_borrow_rate_bps", e.borrow_rate_bps)
            .set("last_utilization_bps", e.utilization_bps)
            .set("last_slot", c.slot);
    }
    for e in &ev.liquidity_collect_revenues {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_revenue_collections", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("revenue_amount", num(&e.revenue_amount_raw))
            .set("revenue_collector", &e.revenue_collector);
        t.upsert_row("token_daily", [("day", utc_day(c.block_time)), ("mint", e.mint.clone())])
            .add("revenue_collected", e.revenue_amount_raw.parse::<u128>().unwrap_or(0));
    }
    for e in &ev.liquidity_claims {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_claims", id(&c));
        base(r, &c);
        r.set("user_address", &e.user)
            .set("mint", &e.mint)
            .set("recipient", &e.recipient)
            .set("amount", num(&e.amount_raw));
    }
    for e in &ev.liquidity_rate_data_v1 {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_rate_configs", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("version", 1i32)
            .set("kink1_bps", num(&e.kink_bps))
            .set("kink2_bps", "0")
            .set("rate_at_zero_bps", num(&e.rate_at_utilization_zero_bps))
            .set("rate_at_kink1_bps", num(&e.rate_at_utilization_kink_bps))
            .set("rate_at_kink2_bps", "0")
            .set("rate_at_max_bps", num(&e.rate_at_utilization_max_bps));
    }
    for e in &ev.liquidity_rate_data_v2 {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_rate_configs", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("version", 2i32)
            .set("kink1_bps", num(&e.kink1_bps))
            .set("kink2_bps", num(&e.kink2_bps))
            .set("rate_at_zero_bps", num(&e.rate_at_utilization_zero_bps))
            .set("rate_at_kink1_bps", num(&e.rate_at_utilization_kink1_bps))
            .set("rate_at_kink2_bps", num(&e.rate_at_utilization_kink2_bps))
            .set("rate_at_max_bps", num(&e.rate_at_utilization_max_bps));
    }
    for e in &ev.liquidity_token_configs {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_token_configs", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("fee_bps", num(&e.fee_bps))
            .set("max_utilization_bps", num(&e.max_utilization_bps));
    }
    for e in &ev.liquidity_user_supply_configs {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_protocol_configs", id(&c));
        base(r, &c);
        r.set("side", "supply")
            .set("protocol", &e.protocol)
            .set("mint", &e.mint)
            .set("mode", e.mode)
            .set("expand_percent", num(&e.expand_percent))
            .set("expand_duration", num(&e.expand_duration))
            .set("base_limit", num(&e.base_withdrawal_limit))
            .set("max_limit", "0");
    }
    for e in &ev.liquidity_user_borrow_configs {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("liquidity_protocol_configs", id(&c));
        base(r, &c);
        r.set("side", "borrow")
            .set("protocol", &e.protocol)
            .set("mint", &e.mint)
            .set("mode", e.mode)
            .set("expand_percent", num(&e.expand_percent))
            .set("expand_duration", num(&e.expand_duration))
            .set("base_limit", num(&e.base_debt_ceiling))
            .set("max_limit", num(&e.max_debt_ceiling));
    }

    // ------------------------------------------------------------ Earn
    for e in &ev.earn_deposits {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("earn_deposits", id(&c));
        base(r, &c);
        r.set("sender", &e.sender)
            .set("receiver", &e.receiver)
            .set("mint", &e.mint)
            .set("f_token_mint", &e.f_token_mint)
            .set("lending", &e.lending)
            .set("assets", num(&e.assets_raw))
            .set("shares", num(&e.shares_minted_raw));
        t.upsert_row("earn_daily", [("day", utc_day(c.block_time)), ("mint", e.mint.clone())])
            .add("deposits", 1i64)
            .add("deposited", i(&e.assets_raw));
    }
    for e in &ev.earn_withdraws {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("earn_withdrawals", id(&c));
        base(r, &c);
        r.set("sender", &e.sender)
            .set("receiver", &e.receiver)
            .set("owner", &e.owner)
            .set("mint", &e.mint)
            .set("f_token_mint", &e.f_token_mint)
            .set("lending", &e.lending)
            .set("assets", num(&e.assets_raw))
            .set("shares", num(&e.shares_burned_raw));
        t.upsert_row("earn_daily", [("day", utc_day(c.block_time)), ("mint", e.mint.clone())])
            .add("withdrawals", 1i64)
            .add("withdrawn", i(&e.assets_raw));
    }
    for e in &ev.earn_update_rates {
        let c = ctx_of(&e.ctx);
        // high-frequency: keep the daily first/last and the latest per fToken instead of every row
        t.upsert_row("earn_daily", [("day", utc_day(c.block_time)), ("mint", e.mint.clone())])
            .set_if_null("first_token_exchange_price", num(&e.token_exchange_price))
            .set("last_token_exchange_price", num(&e.token_exchange_price))
            .set("last_liquidity_exchange_price", num(&e.liquidity_exchange_price));
        t.upsert_row("earn_markets", e.f_token_mint.as_str())
            .set("mint", &e.mint)
            .set("lending", &e.lending)
            .set("token_exchange_price", num(&e.token_exchange_price))
            .set("liquidity_exchange_price", num(&e.liquidity_exchange_price))
            .set("last_slot", c.slot)
            .set("last_block_time", c.block_time);
    }
    for e in &ev.earn_rebalances {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("earn_rebalances", id(&c));
        base(r, &c);
        r.set("mint", &e.mint).set("f_token_mint", &e.f_token_mint).set("assets", num(&e.assets_raw));
    }

    // ------------------------------------------------------------ Borrow vaults
    for e in &ev.vault_creations {
        let c = ctx_of(&e.ctx);
        t.upsert_row("vaults", e.vault_config.as_str())
            .set("vault_id", e.vault_id)
            .set("supply_mint", &e.supply_token)
            .set("borrow_mint", &e.borrow_token)
            .set("oracle", &e.oracle)
            .set("supply_dex", &e.supply_dex)
            .set("borrow_dex", &e.borrow_dex)
            .set("vault_type", e.vault_type)
            .set("collateral_factor", e.collateral_factor)
            .set("liquidation_threshold", e.liquidation_threshold)
            .set("liquidation_max_limit", e.liquidation_max_limit)
            .set("liquidation_penalty", e.liquidation_penalty)
            .set("borrow_fee", e.borrow_fee)
            .set("created_slot", c.slot)
            .set("created_tx", &c.tx_id);
    }
    for e in &ev.vault_operates {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_operations", id(&c));
        base(r, &c);
        r.set("vault_id", e.vault_id)
            .set("vault_config", &e.vault_config)
            .set("supply_mint", &e.supply_mint)
            .set("borrow_mint", &e.borrow_mint)
            .set("nft_id", e.nft_id)
            .set("signer", &e.signer)
            .set("recipient", &e.to)
            .set("col_change", num(&e.col_change_raw))
            .set("debt_change", num(&e.debt_change_raw))
            .set("vault_type", &e.vault_type);
        // vaults seen in operations (fills mints for vaults created before the start block)
        let v = t
            .upsert_row("vaults", e.vault_config.as_str())
            .set("supply_mint", &e.supply_mint)
            .set("borrow_mint", &e.borrow_mint);
        if e.vault_id != 0 {
            v.set("vault_id", e.vault_id);
        }
        let (cc, dc) = (i(&e.col_change_raw), i(&e.debt_change_raw));
        t.upsert_row("vault_daily", [("day", utc_day(c.block_time)), ("vault_config", e.vault_config.clone())])
            .set("vault_id", e.vault_id)
            .add("operations", 1i64)
            .add("col_deposited", cc.max(0))
            .add("col_withdrawn", (-cc).max(0))
            .add("debt_borrowed", dc.max(0))
            .add("debt_repaid", (-dc).max(0));
    }
    for e in &ev.vault_user_positions {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_position_updates", id(&c));
        base(r, &c);
        r.set("vault_id", e.vault_id)
            .set("nft_id", e.nft_id)
            .set("owner", &e.user)
            .set("position_mint", &e.position_mint)
            .set("tick", e.tick)
            .set("col", num(&e.col_raw))
            .set("debt", num(&e.borrow_raw));
        t.upsert_row("vault_positions", [("vault_id", e.vault_id.to_string()), ("nft_id", e.nft_id.to_string())])
            .set("owner", &e.user)
            .set("position_mint", &e.position_mint)
            .set("tick", e.tick)
            .set("col", num(&e.col_raw))
            .set("debt", num(&e.borrow_raw))
            .set("last_slot", c.slot)
            .set("last_block_time", c.block_time)
            .set("last_tx", &c.tx_id);
    }
    for e in &ev.vault_liquidates {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_liquidations", id(&c));
        base(r, &c);
        r.set("vault_id", e.vault_id)
            .set("vault_config", &e.vault_config)
            .set("supply_mint", &e.supply_mint)
            .set("borrow_mint", &e.borrow_mint)
            .set("liquidator", &e.signer)
            .set("recipient", &e.to)
            .set("col_amount", num(&e.col_amount_raw))
            .set("debt_amount", num(&e.debt_amount_raw))
            .set("start_tick", e.start_tick)
            .set("end_tick", e.end_tick)
            .set("vault_type", &e.vault_type);
        t.upsert_row("vault_daily", [("day", utc_day(c.block_time)), ("vault_config", e.vault_config.clone())])
            .set("vault_id", e.vault_id)
            .add("liquidations", 1i64)
            .add("liquidated_col", i(&e.col_amount_raw))
            .add("liquidated_debt", i(&e.debt_amount_raw));
    }
    for e in &ev.vault_absorbs {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_absorbs", id(&c));
        base(r, &c);
        r.set("vault_id", e.vault_id)
            .set("vault_config", &e.vault_config)
            .set("col_amount", num(&e.col_amount_raw))
            .set("debt_amount", num(&e.debt_amount_raw));
    }
    for e in &ev.vault_exchange_prices {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_exchange_prices", id(&c));
        base(r, &c);
        r.set("vault_id", e.vault_id)
            .set("vault_config", &e.vault_config)
            .set("vault_supply_exchange_price", num(&e.vault_supply_exchange_price))
            .set("vault_borrow_exchange_price", num(&e.vault_borrow_exchange_price))
            .set("liquidity_supply_exchange_price", num(&e.liquidity_supply_exchange_price))
            .set("liquidity_borrow_exchange_price", num(&e.liquidity_borrow_exchange_price));
    }
    for e in &ev.vault_rebalances {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_rebalances", id(&c));
        base(r, &c);
        r.set("vault_config", &e.vault_config)
            .set("supply_amount", num(&e.supply_amount_raw))
            .set("borrow_amount", num(&e.borrow_amount_raw));
    }
    for e in &ev.vault_close_positions {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_position_closes", id(&c));
        base(r, &c);
        r.set("vault_id", e.vault_id)
            .set("nft_id", e.position_id)
            .set("signer", &e.signer)
            .set("position_mint", &e.position_mint);
    }
    for e in &ev.vault_core_settings {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_config_updates", id(&c));
        base(r, &c);
        let v = format!(
            "supply_rate_magnifier={},borrow_rate_magnifier={},collateral_factor={},liquidation_threshold={},liquidation_max_limit={},withdraw_gap={},liquidation_penalty={},borrow_fee={}",
            e.supply_rate_magnifier, e.borrow_rate_magnifier, e.collateral_factor, e.liquidation_threshold,
            e.liquidation_max_limit, e.withdraw_gap, e.liquidation_penalty, e.borrow_fee
        );
        r.set("vault_config", &e.vault_config).set("param", "core_settings").set("value", v);
    }
    for e in &ev.vault_param_updates {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_config_updates", id(&c));
        base(r, &c);
        r.set("vault_config", &e.vault_config).set("param", &e.param).set("value", &e.value);
    }
    for e in &ev.vault_liquidation_rounding_diffs {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("vault_liquidation_rounding_diffs", id(&c));
        base(r, &c);
        r.set("vault_id", e.vault_id)
            .set("actual_debt_amount", num(&e.actual_debt_amount_raw))
            .set("debt_amount", num(&e.debt_amount_raw))
            .set("diff", num(&e.diff_raw));
    }

    // ------------------------------------------------------------ rewards
    for e in &ev.rewards_starts {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("earn_reward_events", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("kind", if e.queued { "queue" } else { "start" })
            .set("reward_amount", num(&e.reward_amount_raw))
            .set("duration", e.duration)
            .set("start_time", e.start_time)
            .set("end_time", 0u64);
    }
    for e in &ev.rewards_transitions {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("earn_reward_events", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("kind", "transition")
            .set("reward_amount", "0")
            .set("duration", 0u64)
            .set("start_time", e.start_time)
            .set("end_time", e.end_time);
    }
    for e in &ev.rewards_stops {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("earn_reward_events", id(&c));
        base(r, &c);
        r.set("mint", &e.mint)
            .set("kind", if e.cancelled_queued { "cancel_queued" } else { "stop" })
            .set("reward_amount", "0")
            .set("duration", 0u64)
            .set("start_time", 0u64)
            .set("end_time", 0u64);
    }

    // ------------------------------------------------------------ DEX
    for e in &ev.dex_creations {
        let c = ctx_of(&e.ctx);
        t.upsert_row("dexes", e.dex_id.to_string())
            .set("token0", &e.token0)
            .set("token1", &e.token1)
            .set("smart_col", e.smart_col)
            .set("smart_debt", e.smart_debt)
            .set("fee", e.fee)
            .set("revenue_cut", e.revenue_cut)
            .set("created_slot", c.slot);
    }
    for e in &ev.dex_fee_updates {
        t.upsert_row("dexes", e.dex_id.to_string()).set("fee", e.fee).set("revenue_cut", e.revenue_cut);
    }
    for e in &ev.dex_swaps {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("dex_swaps", id(&c));
        base(r, &c);
        r.set("dex_id", e.dex_id)
            .set("dex", &e.dex)
            .set("token0", &e.token0)
            .set("token1", &e.token1)
            .set("swap0_to1", e.swap0_to1)
            .set("maker", &e.maker)
            .set("recipient", &e.to)
            .set("amount_in", num(&e.amount_in_raw))
            .set("amount_out", num(&e.amount_out_raw))
            .set("fee_amount_in", num(&e.fee_amount_in_raw))
            .set("reserves_token0", num(&e.reserves_token0))
            .set("reserves_token1", num(&e.reserves_token1))
            .set("pool_price", num(&e.pool_price));
        if !e.dex.is_empty() {
            t.upsert_row("dexes", e.dex_id.to_string())
                .set("dex", &e.dex)
                .set("token0", &e.token0)
                .set("token1", &e.token1);
        }
        let (ain, fee) = (i(&e.amount_in_raw), i(&e.fee_amount_in_raw));
        let d = t.upsert_row("dex_daily", [("day", utc_day(c.block_time)), ("dex_id", e.dex_id.to_string())]);
        d.add("swaps", 1i64);
        if e.swap0_to1 {
            d.add("volume_in_token0", ain).add("fees_token0", fee);
        } else {
            d.add("volume_in_token1", ain).add("fees_token1", fee);
        }
    }
    for e in &ev.dex_liquidity {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("dex_liquidity", id(&c));
        base(r, &c);
        r.set("dex_id", e.dex_id)
            .set("action", &e.action)
            .set("shares", num(&e.shares_raw))
            .set("token0_amount", num(&e.token0_amount_raw))
            .set("token1_amount", num(&e.token1_amount_raw))
            .set("user_address", &e.user)
            .set("protocol", &e.protocol);
    }
    for e in &ev.dex_arbitrages {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("dex_arbitrages", id(&c));
        base(r, &c);
        r.set("dex_id", e.dex_id).set("routing", num(&e.routing)).set("amount_out", num(&e.amount_out_raw));
    }
    for e in &ev.flashloan_fees {
        let c = ctx_of(&e.ctx);
        let r = t.create_row("flashloan_fee_updates", id(&c));
        base(r, &c);
        r.set("flashloan_fee", e.flashloan_fee);
    }

    Ok(t.to_database_changes())
}
