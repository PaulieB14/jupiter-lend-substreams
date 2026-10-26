//! `map_events`: Firehose Solana block -> typed Jupiter Lend events.
use std::collections::{BTreeMap, HashMap};

use substreams::errors::Error;
use substreams_solana::pb::sf::solana::r#type::v1::{Block, ConfirmedTransaction};

use crate::decoder::decode_tx;
use crate::idl::*;
use crate::pb::jupiter_lend::v1::{Events, Token};
use crate::tx::{Ix, TxInput};

fn program_keys() -> [[u8; 32]; 6] {
    let k = |s: &str| -> [u8; 32] {
        let v = bs58::decode(s).into_vec().unwrap();
        v.try_into().unwrap()
    };
    [
        k(LIQUIDITY_PROGRAM),
        k(LENDING_PROGRAM),
        k(VAULTS_PROGRAM),
        k(REWARDS_PROGRAM),
        k(FLASHLOAN_PROGRAM),
        k(DEX_PROGRAM),
    ]
}

fn b58(b: &[u8]) -> String {
    bs58::encode(b).into_string()
}

/// Build the decoder view of a transaction. A cheap byte scan over the account table rejects
/// unrelated txs. Top-level program ids are static keys, but CPI'd program ids (e.g. a Jupiter
/// swap route into the Lend DEX, or Earn `redeem` inside an aggregator route) may be loaded from
/// address lookup tables, so loaded addresses are scanned too.
pub fn to_tx_input(
    trx: &ConfirmedTransaction,
    keys: &[[u8; 32]; 6],
    slot: u64,
    block_time: i64,
    tx_index: u32,
) -> Option<TxInput> {
    let t = trx.transaction.as_option()?;
    let msg = t.message.as_option()?;
    let meta = trx.meta.as_option()?;
    let hit = |k: &Vec<u8>| keys.iter().any(|p| k.as_slice() == p);
    if !(msg.account_keys.iter().any(hit)
        || meta.loaded_writable_addresses.iter().any(hit)
        || meta.loaded_readonly_addresses.iter().any(hit))
    {
        return None;
    }
    let walk: Vec<Ix> = trx
        .walk_instructions()
        .map(|ix| Ix {
            program_id: ix.program_id().to_string(),
            data: ix.data().clone(),
            accounts: ix.accounts().iter().map(|a| a.to_string()).collect(),
            stack_height: ix.stack_height(),
        })
        .collect();
    let mut mints = HashMap::new();
    for tb in meta.pre_token_balances.iter().chain(meta.post_token_balances.iter()) {
        if let Some(u) = tb.ui_token_amount.as_option() {
            mints.entry(tb.mint.clone()).or_insert((u.decimals, tb.program_id.clone()));
        }
    }
    Some(TxInput {
        slot,
        block_time,
        tx_id: trx.id(),
        tx_index,
        fee_payer: msg.account_keys.first().map(|k| b58(k)).unwrap_or_default(),
        walk,
        logs: meta.log_messages.clone(),
        mints,
    })
}

#[substreams::handlers::map]
pub fn map_events(block: Block) -> Result<Events, Error> {
    let keys = program_keys();
    let slot = block.slot;
    let block_time = block.block_time.as_option().map(|t| t.timestamp).unwrap_or(0);
    let mut out = Events::default();
    let mut tokens: BTreeMap<String, (u32, String)> = BTreeMap::new();
    // `transactions()` yields successful transactions only (failed txs revert their events)
    for (i, trx) in block.transactions().enumerate() {
        let Some(tx) = to_tx_input(trx, &keys, slot, block_time, i as u32) else { continue };
        out.lend_tx_count += 1;
        let before = count(&out);
        if !decode_tx(&tx, &mut out) {
            out.truncated_log_tx_count += 1;
        }
        if count(&out) > before {
            for (m, v) in tx.mints {
                tokens.entry(m).or_insert(v);
            }
        }
    }
    out.tokens = tokens
        .into_iter()
        .map(|(mint, (decimals, token_program))| Token { mint, decimals, token_program, ..Default::default() })
        .collect();
    Ok(out)
}

pub fn count(e: &Events) -> usize {
    e.liquidity_operates.len()
        + e.liquidity_exchange_prices.len()
        + e.liquidity_collect_revenues.len()
        + e.liquidity_claims.len()
        + e.liquidity_rate_data_v1.len()
        + e.liquidity_rate_data_v2.len()
        + e.liquidity_token_configs.len()
        + e.liquidity_user_supply_configs.len()
        + e.liquidity_user_borrow_configs.len()
        + e.earn_deposits.len()
        + e.earn_withdraws.len()
        + e.earn_update_rates.len()
        + e.earn_rebalances.len()
        + e.vault_operates.len()
        + e.vault_user_positions.len()
        + e.vault_liquidates.len()
        + e.vault_absorbs.len()
        + e.vault_exchange_prices.len()
        + e.vault_rebalances.len()
        + e.vault_close_positions.len()
        + e.vault_core_settings.len()
        + e.vault_param_updates.len()
        + e.vault_creations.len()
        + e.vault_liquidation_rounding_diffs.len()
        + e.rewards_starts.len()
        + e.rewards_transitions.len()
        + e.rewards_stops.len()
        + e.dex_swaps.len()
        + e.dex_liquidity.len()
        + e.dex_arbitrages.len()
        + e.dex_creations.len()
        + e.dex_fee_updates.len()
        + e.flashloan_fees.len()
}
