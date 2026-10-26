//! Decode Jupiter Lend Anchor events (from `Program data:` logs) into typed protobuf messages.
use std::collections::HashMap;

use crate::idl::{self, *};
use crate::pb::jupiter_lend::v1::*;
use crate::tx::{attribute, Ix, TxInput};
use crate::util::{ui, Reader};

pub fn is_lend_program(p: &str) -> bool {
    p == LIQUIDITY_PROGRAM
        || p == LENDING_PROGRAM
        || p == VAULTS_PROGRAM
        || p == REWARDS_PROGRAM
        || p == FLASHLOAN_PROGRAM
        || p == DEX_PROGRAM
}

fn instructions_for(p: &str) -> &'static [([u8; 8], &'static str, &'static [&'static str])] {
    match p {
        LIQUIDITY_PROGRAM => idl::liquidity::INSTRUCTIONS,
        LENDING_PROGRAM => idl::lending::INSTRUCTIONS,
        VAULTS_PROGRAM => idl::vaults::INSTRUCTIONS,
        REWARDS_PROGRAM => idl::lending_reward_rate_model::INSTRUCTIONS,
        FLASHLOAN_PROGRAM => idl::flashloan::INSTRUCTIONS,
        DEX_PROGRAM => idl::dex::INSTRUCTIONS,
        _ => &[],
    }
}

/// Instruction name + named accounts of the emitting instruction (from the IDL).
struct IxCtx<'a> {
    name: &'static str,
    names: &'static [&'static str],
    ix: &'a Ix,
}

impl<'a> IxCtx<'a> {
    fn of(ix: &'a Ix) -> Self {
        for (disc, name, names) in instructions_for(&ix.program_id) {
            if ix.data.len() >= 8 && ix.data[..8] == disc[..] {
                return IxCtx { name, names, ix };
            }
        }
        IxCtx { name: "", names: &[], ix }
    }
    fn acc(&self, name: &str) -> String {
        self.names
            .iter()
            .position(|n| *n == name)
            .and_then(|i| self.ix.accounts.get(i).cloned())
            .unwrap_or_default()
    }
    fn args(&self) -> Reader<'a> {
        Reader::new(if self.ix.data.len() >= 8 { &self.ix.data[8..] } else { &[] })
    }
}

fn protocol_type(caller: Option<&Ix>) -> &'static str {
    match caller.map(|c| c.program_id.as_str()) {
        Some(LENDING_PROGRAM) => "earn",
        Some(VAULTS_PROGRAM) => "vault",
        Some(DEX_PROGRAM) => "dex",
        Some(FLASHLOAN_PROGRAM) => "flashloan",
        _ => "other",
    }
}

fn vault_type(ix_name: &str) -> &'static str {
    if ix_name.ends_with("perfect_dex") {
        "perfect_dex"
    } else if ix_name.ends_with("_dex") {
        "dex"
    } else {
        "T1"
    }
}

fn s<T: ToString>(v: T) -> String {
    v.to_string()
}

/// Decode one transaction. Returns `false` when logs were incomplete (truncated / desynced).
pub fn decode_tx(tx: &TxInput, out: &mut Events) -> bool {
    let att = attribute(tx, &is_lend_program);
    if att.events.is_empty() {
        return !att.incomplete;
    }
    let dec = |mint: &str| -> i32 { tx.mints.get(mint).map(|(d, _)| *d as i32).unwrap_or(-1) };

    // Pre-pass: vault id per emitting vault instruction (events in the same ix share it).
    let mut vault_ids: HashMap<usize, u32> = HashMap::new();
    let mut liq_ticks: HashMap<usize, (i32, i32)> = HashMap::new();
    for ev in &att.events {
        let ix = &tx.walk[ev.walk_index];
        if ix.program_id != VAULTS_PROGRAM || ev.data.len() < 8 {
            continue;
        }
        let d = &ev.data[8..];
        let disc = &ev.data[..8];
        let id = if disc == idl::vaults::EV_LOG_USER_POSITION {
            let mut r = Reader::new(d);
            r.pubkey();
            r.u32();
            r.u16().map(|v| v as u32)
        } else if disc == idl::vaults::EV_LOG_LIQUIDATE_INFO
            || disc == idl::vaults::EV_LOG_LIQUIDATION_ROUNDING_DIFF
        {
            let mut r = Reader::new(d);
            let v = r.u16().map(|v| v as u32);
            if disc == idl::vaults::EV_LOG_LIQUIDATE_INFO {
                if let (Some(a), Some(b)) = (r.i32(), r.i32()) {
                    liq_ticks.entry(ev.walk_index).or_insert((a, b));
                }
            }
            v
        } else if disc == idl::vaults::EV_LOG_CLOSE_POSITION {
            let mut r = Reader::new(d);
            r.pubkey();
            r.u32();
            r.u16().map(|v| v as u32)
        } else {
            None
        };
        if let Some(id) = id {
            vault_ids.entry(ev.walk_index).or_insert(id);
        }
        // instructions carrying vault_id as first arg
        let ic = IxCtx::of(ix);
        if matches!(
            ic.name,
            "update_exchange_prices" | "init_vault_config" | "init_vault_state" | "init_position"
        ) {
            if let Some(v) = ic.args().u16() {
                vault_ids.entry(ev.walk_index).or_insert(v as u32);
            }
        }
    }

    for (log_index, ev) in att.events.iter().enumerate() {
        if ev.data.len() < 8 {
            continue;
        }
        let ix = &tx.walk[ev.walk_index];
        let caller = ev.caller.map(|c| &tx.walk[c]);
        let ic = IxCtx::of(ix);
        let ctx = Ctx {
            slot: tx.slot,
            block_time: tx.block_time,
            tx_id: tx.tx_id.clone(),
            tx_index: tx.tx_index,
            instruction_index: ev.walk_index as u32,
            log_index: log_index as u32,
            instruction: ic.name.to_string(),
            caller_program: caller.map(|c| c.program_id.clone()).unwrap_or_default(),
            fee_payer: tx.fee_payer.clone(),
            stack_height: ix.stack_height,
            ..Default::default()
        };
        let disc: [u8; 8] = ev.data[..8].try_into().unwrap();
        let mut r = Reader::new(&ev.data[8..]);
        let vault_id = vault_ids.get(&ev.walk_index).copied().unwrap_or_else(|| {
            // older vault program versions emit no vault id: resolve from the vault_config PDA
            if ix.program_id == VAULTS_PROGRAM {
                crate::vaults_registry::by_config(&ic.acc("vault_config")).map(|v| v.0).unwrap_or(0)
            } else {
                0
            }
        });
        let ok = match ix.program_id.as_str() {
            LIQUIDITY_PROGRAM => decode_liquidity(disc, &mut r, ctx, &ic, caller, &dec, out),
            LENDING_PROGRAM => decode_lending(disc, &mut r, ctx, &ic, &dec, out),
            VAULTS_PROGRAM => {
                let ticks = liq_ticks.get(&ev.walk_index).copied();
                decode_vaults(disc, &mut r, ctx, &ic, vault_id, ticks, &dec, out)
            }
            REWARDS_PROGRAM => decode_rewards(disc, &mut r, ctx, out),
            DEX_PROGRAM => decode_dex(disc, &mut r, ctx, &ic, out),
            FLASHLOAN_PROGRAM => {
                if disc == idl::flashloan::EV_SET_FLASHLOAN_FEE {
                    r.u16().map(|f| {
                        out.flashloan_fees
                            .push(FlashloanFee { ctx: ctx.into(), flashloan_fee: f as u32, ..Default::default() })
                    })
                } else {
                    Some(())
                }
            }
            _ => Some(()),
        };
        let _ = ok; // None == payload shorter than the IDL layout: event skipped
    }
    !att.incomplete
}

fn decode_liquidity(
    disc: [u8; 8],
    r: &mut Reader,
    ctx: Ctx,
    ic: &IxCtx,
    caller: Option<&Ix>,
    dec: &dyn Fn(&str) -> i32,
    out: &mut Events,
) -> Option<()> {
    use idl::liquidity::*;
    match disc {
        EV_LOG_OPERATE => {
            let user = r.pubkey()?;
            let mint = r.pubkey()?;
            let supply = r.i128()?;
            let borrow = r.i128()?;
            let withdraw_to = r.pubkey()?;
            let borrow_to = r.pubkey()?;
            let sep = r.u64()?;
            let bep = r.u64()?;
            let d = dec(&mint);
            out.liquidity_operates.push(LiquidityOperate {
                ctx: ctx.into(),
                protocol: user,
                protocol_type: protocol_type(caller).into(),
                supply_amount_raw: s(supply),
                borrow_amount_raw: s(borrow),
                supply_amount_ui: ui(supply, d),
                borrow_amount_ui: ui(borrow, d),
                mint,
                withdraw_to,
                borrow_to,
                supply_exchange_price: s(sep),
                borrow_exchange_price: s(bep),
                decimals: d,
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_EXCHANGE_PRICES => {
            let mint = r.pubkey()?;
            let sep = r.u128()?;
            let bep = r.u128()?;
            let br = r.u16()?;
            let ut = r.u16()?;
            out.liquidity_exchange_prices.push(LiquidityExchangePrices {
                ctx: ctx.into(),
                mint,
                supply_exchange_price: s(sep),
                borrow_exchange_price: s(bep),
                borrow_rate_bps: br as u32,
                utilization_bps: ut as u32,
                ..Default::default()
            });
        }
        EV_LOG_COLLECT_REVENUE => {
            let mint = r.pubkey()?;
            let amt = r.u128()?;
            let d = dec(&mint);
            out.liquidity_collect_revenues.push(LiquidityCollectRevenue {
                ctx: ctx.into(),
                revenue_amount_raw: s(amt),
                revenue_amount_ui: if amt <= i128::MAX as u128 { ui(amt as i128, d) } else { String::new() },
                revenue_collector: ic.acc("revenue_collector"),
                mint,
                decimals: d,
                ..Default::default()
            });
        }
        EV_LOG_CLAIM => {
            let user = r.pubkey()?;
            let mint = r.pubkey()?;
            let recipient = r.pubkey()?;
            let amt = r.u64()?;
            let d = dec(&mint);
            out.liquidity_claims.push(LiquidityClaim {
                ctx: ctx.into(),
                user,
                recipient,
                amount_raw: s(amt),
                amount_ui: ui(amt as i128, d),
                mint,
                decimals: d,
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_RATE_DATA_V1 => {
            let mint = r.pubkey()?;
            let (a, b, c, e) = (r.u128()?, r.u128()?, r.u128()?, r.u128()?);
            out.liquidity_rate_data_v1.push(LiquidityRateDataV1 {
                ctx: ctx.into(),
                mint,
                kink_bps: s(a),
                rate_at_utilization_zero_bps: s(b),
                rate_at_utilization_kink_bps: s(c),
                rate_at_utilization_max_bps: s(e),
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_RATE_DATA_V2 => {
            let mint = r.pubkey()?;
            let v: Vec<u128> = (0..6).map(|_| r.u128()).collect::<Option<_>>()?;
            out.liquidity_rate_data_v2.push(LiquidityRateDataV2 {
                ctx: ctx.into(),
                mint,
                kink1_bps: s(v[0]),
                kink2_bps: s(v[1]),
                rate_at_utilization_zero_bps: s(v[2]),
                rate_at_utilization_kink1_bps: s(v[3]),
                rate_at_utilization_kink2_bps: s(v[4]),
                rate_at_utilization_max_bps: s(v[5]),
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_TOKEN_CONFIGS => {
            let mint = r.pubkey()?;
            let fee = r.u128()?;
            let mu = r.u128()?;
            out.liquidity_token_configs.push(LiquidityTokenConfig {
                ctx: ctx.into(),
                mint,
                fee_bps: s(fee),
                max_utilization_bps: s(mu),
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_USER_SUPPLY_CONFIGS => {
            let protocol = r.pubkey()?;
            let mint = r.pubkey()?;
            let mode = r.u8()?;
            let (a, b, c) = (r.u128()?, r.u128()?, r.u128()?);
            out.liquidity_user_supply_configs.push(LiquidityUserSupplyConfig {
                ctx: ctx.into(),
                protocol,
                mint,
                mode: mode as u32,
                expand_percent: s(a),
                expand_duration: s(b),
                base_withdrawal_limit: s(c),
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_USER_BORROW_CONFIGS => {
            let protocol = r.pubkey()?;
            let mint = r.pubkey()?;
            let mode = r.u8()?;
            let (a, b, c, e) = (r.u128()?, r.u128()?, r.u128()?, r.u128()?);
            out.liquidity_user_borrow_configs.push(LiquidityUserBorrowConfig {
                ctx: ctx.into(),
                protocol,
                mint,
                mode: mode as u32,
                expand_percent: s(a),
                expand_duration: s(b),
                base_debt_ceiling: s(c),
                max_debt_ceiling: s(e),
                ..Default::default()
            });
        }
        _ => {}
    }
    Some(())
}

fn decode_lending(
    disc: [u8; 8],
    r: &mut Reader,
    ctx: Ctx,
    ic: &IxCtx,
    dec: &dyn Fn(&str) -> i32,
    out: &mut Events,
) -> Option<()> {
    use idl::lending::*;
    let mint = ic.acc("mint");
    let f_token_mint = ic.acc("f_token_mint");
    let lending = ic.acc("lending");
    let d = dec(&mint);
    match disc {
        EV_LOG_DEPOSIT => {
            let sender = r.pubkey()?;
            let receiver = r.pubkey()?;
            let assets = r.u64()?;
            let shares = r.u64()?;
            out.earn_deposits.push(EarnDeposit {
                ctx: ctx.into(),
                sender,
                receiver,
                assets_raw: s(assets),
                shares_minted_raw: s(shares),
                assets_ui: ui(assets as i128, d),
                mint,
                f_token_mint,
                lending,
                decimals: d,
                ..Default::default()
            });
        }
        EV_LOG_WITHDRAW => {
            let sender = r.pubkey()?;
            let receiver = r.pubkey()?;
            let owner = r.pubkey()?;
            let assets = r.u64()?;
            let shares = r.u64()?;
            out.earn_withdraws.push(EarnWithdraw {
                ctx: ctx.into(),
                sender,
                receiver,
                owner,
                assets_raw: s(assets),
                shares_burned_raw: s(shares),
                assets_ui: ui(assets as i128, d),
                mint,
                f_token_mint,
                lending,
                decimals: d,
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_RATES => {
            let tep = r.u64()?;
            let lep = r.u64()?;
            out.earn_update_rates.push(EarnUpdateRates {
                ctx: ctx.into(),
                mint,
                f_token_mint,
                lending,
                token_exchange_price: s(tep),
                liquidity_exchange_price: s(lep),
                ..Default::default()
            });
        }
        EV_LOG_REBALANCE => {
            let assets = r.u64()?;
            out.earn_rebalances.push(EarnRebalance {
                ctx: ctx.into(),
                mint,
                f_token_mint,
                lending,
                assets_raw: s(assets),
                ..Default::default()
            });
        }
        _ => {}
    }
    Some(())
}

fn decode_vaults(
    disc: [u8; 8],
    r: &mut Reader,
    ctx: Ctx,
    ic: &IxCtx,
    vault_id: u32,
    ticks: Option<(i32, i32)>,
    dec: &dyn Fn(&str) -> i32,
    out: &mut Events,
) -> Option<()> {
    use idl::vaults::*;
    let vault_config = ic.acc("vault_config");
    let supply_mint = ic.acc("supply_token");
    let borrow_mint = ic.acc("borrow_token");
    let vtype = vault_type(ic.name);
    let param = |out: &mut Events, ctx: Ctx, p: &str, v: String| {
        out.vault_param_updates.push(VaultParamUpdate {
            ctx: ctx.into(),
            vault_config: ic.acc("vault_config"),
            param: p.into(),
            value: v,
            ..Default::default()
        })
    };
    match disc {
        EV_LOG_OPERATE => {
            let signer = r.pubkey()?;
            let nft_id = r.u32()?;
            let col = r.i128()?;
            let debt = r.i128()?;
            let to = r.pubkey()?;
            out.vault_operates.push(VaultOperate {
                ctx: ctx.into(),
                vault_id,
                vault_config,
                supply_mint,
                borrow_mint,
                nft_id,
                signer,
                to,
                col_change_raw: s(col),
                debt_change_raw: s(debt),
                vault_type: vtype.into(),
                ..Default::default()
            });
        }
        EV_LOG_USER_POSITION => {
            let user = r.pubkey()?;
            let nft_id = r.u32()?;
            let vid = r.u16()?;
            let position_mint = r.pubkey()?;
            let tick = r.i32()?;
            let col = r.u64()?;
            let borrow = r.u64()?;
            out.vault_user_positions.push(VaultUserPosition {
                ctx: ctx.into(),
                user,
                nft_id,
                vault_id: vid as u32,
                position_mint,
                tick,
                col_raw: s(col),
                borrow_raw: s(borrow),
                vault_config,
                ..Default::default()
            });
        }
        EV_LOG_LIQUIDATE => {
            let signer = r.pubkey()?;
            let col = r.u64()?;
            let debt = r.u64()?;
            let to = r.pubkey()?;
            let (cu, du) = if vtype == "T1" {
                (ui(col as i128, dec(&supply_mint)), ui(debt as i128, dec(&borrow_mint)))
            } else {
                (String::new(), String::new())
            };
            out.vault_liquidates.push(VaultLiquidate {
                ctx: ctx.into(),
                vault_id,
                vault_config,
                supply_mint,
                borrow_mint,
                signer,
                to,
                col_amount_raw: s(col),
                debt_amount_raw: s(debt),
                start_tick: ticks.map(|t| t.0).unwrap_or(0),
                end_tick: ticks.map(|t| t.1).unwrap_or(0),
                col_amount_ui: cu,
                debt_amount_ui: du,
                vault_type: vtype.into(),
                ..Default::default()
            });
        }
        EV_LOG_ABSORB => {
            let col = r.u64()?;
            let debt = r.u64()?;
            out.vault_absorbs.push(VaultAbsorb {
                ctx: ctx.into(),
                vault_id,
                vault_config,
                col_amount_raw: s(col),
                debt_amount_raw: s(debt),
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_EXCHANGE_PRICES => {
            let (a, b, c, d) = (r.u64()?, r.u64()?, r.u64()?, r.u64()?);
            out.vault_exchange_prices.push(VaultExchangePrices {
                ctx: ctx.into(),
                vault_id,
                vault_config,
                vault_supply_exchange_price: s(a),
                vault_borrow_exchange_price: s(b),
                liquidity_supply_exchange_price: s(c),
                liquidity_borrow_exchange_price: s(d),
                ..Default::default()
            });
        }
        EV_LOG_REBALANCE => {
            let a = r.i128()?;
            let b = r.i128()?;
            out.vault_rebalances.push(VaultRebalance {
                ctx: ctx.into(),
                vault_config,
                supply_amount_raw: s(a),
                borrow_amount_raw: s(b),
                ..Default::default()
            });
        }
        EV_LOG_CLOSE_POSITION => {
            let signer = r.pubkey()?;
            let position_id = r.u32()?;
            let vid = r.u16()?;
            let position_mint = r.pubkey()?;
            out.vault_close_positions.push(VaultClosePosition {
                ctx: ctx.into(),
                signer,
                position_id,
                vault_id: vid as u32,
                position_mint,
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_CORE_SETTINGS => {
            let srm = r.i16()?;
            let brm = r.i16()?;
            let cf = r.u16()?;
            let lt = r.u16()?;
            let lml = r.u16()?;
            let wg = r.u16()?;
            let lp = r.u16()?;
            let bf = r.u8()?;
            out.vault_core_settings.push(VaultCoreSettings {
                ctx: ctx.into(),
                vault_config,
                supply_rate_magnifier: srm as i32,
                borrow_rate_magnifier: brm as i32,
                collateral_factor: cf as u32,
                liquidation_threshold: lt as u32,
                liquidation_max_limit: lml as u32,
                withdraw_gap: wg as u32,
                liquidation_penalty: lp as u32,
                borrow_fee: bf as u32,
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_BORROW_FEE => param(out, ctx, "borrow_fee", s(r.u8()?)),
        EV_LOG_UPDATE_BORROW_RATE_MAGNIFIER => param(out, ctx, "borrow_rate_magnifier", s(r.i16()?)),
        EV_LOG_UPDATE_SUPPLY_RATE_MAGNIFIER => param(out, ctx, "supply_rate_magnifier", s(r.i16()?)),
        EV_LOG_UPDATE_COLLATERAL_FACTOR => param(out, ctx, "collateral_factor", s(r.u16()?)),
        EV_LOG_UPDATE_LIQUIDATION_THRESHOLD => param(out, ctx, "liquidation_threshold", s(r.u16()?)),
        EV_LOG_UPDATE_LIQUIDATION_MAX_LIMIT => param(out, ctx, "liquidation_max_limit", s(r.u16()?)),
        EV_LOG_UPDATE_LIQUIDATION_PENALTY => param(out, ctx, "liquidation_penalty", s(r.u16()?)),
        EV_LOG_UPDATE_WITHDRAW_GAP => param(out, ctx, "withdraw_gap", s(r.u16()?)),
        EV_LOG_UPDATE_ORACLE => param(out, ctx, "oracle", r.pubkey()?),
        EV_LOG_UPDATE_REBALANCER => param(out, ctx, "rebalancer", r.pubkey()?),
        EV_LOG_UPDATE_LOOKUP_TABLE => param(out, ctx, "lookup_table", r.pubkey()?),
        EV_LOG_INIT_VAULT_CONFIG => {
            let cfg = r.pubkey()?;
            let mut a = ic.args();
            let vid = a.u16()?;
            let _srm = a.i16()?;
            let _brm = a.i16()?;
            let cf = a.u16()?;
            let lt = a.u16()?;
            let lml = a.u16()?;
            let _wg = a.u16()?;
            let lp = a.u16()?;
            let bf = a.u8()?;
            let vt = a.u8()?;
            out.vault_creations.push(VaultCreated {
                ctx: ctx.into(),
                vault_id: vid as u32,
                vault_config: cfg,
                supply_token: ic.acc("supply_token"),
                borrow_token: ic.acc("borrow_token"),
                oracle: ic.acc("oracle"),
                supply_dex: ic.acc("supply_dex"),
                borrow_dex: ic.acc("borrow_dex"),
                vault_type: vt as u32,
                collateral_factor: cf as u32,
                liquidation_threshold: lt as u32,
                liquidation_max_limit: lml as u32,
                liquidation_penalty: lp as u32,
                borrow_fee: bf as u32,
                ..Default::default()
            });
        }
        EV_LOG_LIQUIDATION_ROUNDING_DIFF => {
            let vid = r.u16()?;
            let (a, b, c) = (r.u64()?, r.u64()?, r.u64()?);
            out.vault_liquidation_rounding_diffs.push(VaultLiquidationRoundingDiff {
                ctx: ctx.into(),
                vault_id: vid as u32,
                actual_debt_amount_raw: s(a),
                debt_amount_raw: s(b),
                diff_raw: s(c),
                ..Default::default()
            });
        }
        _ => {}
    }
    Some(())
}

fn decode_rewards(disc: [u8; 8], r: &mut Reader, ctx: Ctx, out: &mut Events) -> Option<()> {
    use idl::lending_reward_rate_model::*;
    match disc {
        EV_LOG_START_REWARDS => {
            let amt = r.u64()?;
            let dur = r.u64()?;
            let st = r.u64()?;
            let mint = r.pubkey()?;
            out.rewards_starts.push(RewardsStart {
                ctx: ctx.into(),
                mint,
                reward_amount_raw: s(amt),
                duration: dur,
                start_time: st,
                queued: false,
                ..Default::default()
            });
        }
        EV_LOG_QUEUE_NEXT_REWARDS => {
            let amt = r.u64()?;
            let dur = r.u64()?;
            let mint = r.pubkey()?;
            out.rewards_starts.push(RewardsStart {
                ctx: ctx.into(),
                mint,
                reward_amount_raw: s(amt),
                duration: dur,
                start_time: 0,
                queued: true,
                ..Default::default()
            });
        }
        EV_LOG_TRANSITIONED_TO_NEXT_REWARDS => {
            let st = r.u64()?;
            let et = r.u64()?;
            let mint = r.pubkey()?;
            out.rewards_transitions.push(RewardsTransition {
                ctx: ctx.into(),
                mint,
                start_time: st,
                end_time: et,
                ..Default::default()
            });
        }
        EV_LOG_STOP_REWARDS | EV_LOG_CANCEL_QUEUED_REWARDS => {
            let mint = r.pubkey()?;
            out.rewards_stops.push(RewardsStop {
                ctx: ctx.into(),
                mint,
                cancelled_queued: disc == EV_LOG_CANCEL_QUEUED_REWARDS,
                ..Default::default()
            });
        }
        _ => {}
    }
    Some(())
}

fn decode_dex(disc: [u8; 8], r: &mut Reader, ctx: Ctx, ic: &IxCtx, out: &mut Events) -> Option<()> {
    use idl::dex::*;
    // (action, layout) where layout true == (shares, token0, token1), false == (amount0, amount1, shares)
    let liq: Option<(&str, bool)> = match disc {
        EV_LOG_DEPOSIT_COL_LIQUIDITY => Some(("deposit_col", false)),
        EV_LOG_WITHDRAW_COL_LIQUIDITY => Some(("withdraw_col", false)),
        EV_LOG_BORROW_DEBT_LIQUIDITY => Some(("borrow_debt", false)),
        EV_LOG_PAYBACK_DEBT_LIQUIDITY => Some(("payback_debt", false)),
        EV_LOG_DEPOSIT_PERFECT_COL_LIQUIDITY => Some(("deposit_perfect_col", true)),
        EV_LOG_WITHDRAW_PERFECT_COL_LIQUIDITY => Some(("withdraw_perfect_col", true)),
        EV_LOG_BORROW_PERFECT_DEBT_LIQUIDITY => Some(("borrow_perfect_debt", true)),
        EV_LOG_PAYBACK_PERFECT_DEBT_LIQUIDITY => Some(("payback_perfect_debt", true)),
        EV_LOG_WITHDRAW_COL_IN_ONE_TOKEN => Some(("withdraw_col_in_one_token", true)),
        EV_LOG_PAYBACK_DEBT_IN_ONE_TOKEN => Some(("payback_debt_in_one_token", true)),
        _ => None,
    };
    if let Some((action, perfect)) = liq {
        let dex_id = r.u16()?;
        let (shares, a0, a1) = if perfect {
            (r.u64()?, r.u64()?, r.u64()?)
        } else {
            let a0 = r.u64()?;
            let a1 = r.u64()?;
            (r.u64()?, a0, a1)
        };
        let user = r.pubkey()?;
        let protocol = r.pubkey()?;
        out.dex_liquidity.push(DexLiquidity {
            ctx: ctx.into(),
            dex_id: dex_id as u32,
            action: action.into(),
            shares_raw: s(shares),
            token0_amount_raw: s(a0),
            token1_amount_raw: s(a1),
            user,
            protocol,
            ..Default::default()
        });
        return Some(());
    }
    match disc {
        EV_LOG_SWAP_INFO => {
            let dex_id = r.u16()?;
            let z = r.bool()?;
            let maker = r.pubkey()?;
            let to = r.pubkey()?;
            let ain = r.u64()?;
            let aout = r.u64()?;
            let fee = r.u64()?;
            let r0 = r.u128()?;
            let r1 = r.u128()?;
            let px = r.u128()?;
            out.dex_swaps.push(DexSwap {
                ctx: ctx.into(),
                dex_id: dex_id as u32,
                swap0_to1: z,
                maker,
                to,
                amount_in_raw: s(ain),
                amount_out_raw: s(aout),
                fee_amount_in_raw: s(fee),
                reserves_token0: s(r0),
                reserves_token1: s(r1),
                pool_price: s(px),
                dex: ic.acc("dex"),
                token0: ic.acc("token_0"),
                token1: ic.acc("token_1"),
                ..Default::default()
            });
        }
        EV_LOG_ARBITRAGE => {
            let dex_id = r.u16()?;
            let routing = r.i128()?;
            let aout = r.u64()?;
            out.dex_arbitrages.push(DexArbitrage {
                ctx: ctx.into(),
                dex_id: dex_id as u32,
                routing: s(routing),
                amount_out_raw: s(aout),
                ..Default::default()
            });
        }
        EV_LOG_INIT_DEX => {
            let dex_id = r.u16()?;
            let t0 = r.pubkey()?;
            let t1 = r.pubkey()?;
            let sc = r.bool()?;
            let sd = r.bool()?;
            let fee = r.u32()?;
            let rc = r.u32()?;
            let cp = r.u64()?;
            out.dex_creations.push(DexCreated {
                ctx: ctx.into(),
                dex_id: dex_id as u32,
                token0: t0,
                token1: t1,
                smart_col: sc,
                smart_debt: sd,
                fee,
                revenue_cut: rc,
                center_price: s(cp),
                ..Default::default()
            });
        }
        EV_LOG_UPDATE_FEE_AND_REVENUE_CUT => {
            let dex_id = r.u16()?;
            let fee = r.u32()?;
            let rc = r.u32()?;
            out.dex_fee_updates.push(DexFeeUpdate {
                ctx: ctx.into(),
                dex_id: dex_id as u32,
                fee,
                revenue_cut: rc,
                ..Default::default()
            });
        }
        _ => {}
    }
    Some(())
}
