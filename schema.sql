-- Jupiter Lend Substreams — PostgreSQL schema (substreams-sink-sql, db_out module)
-- Raw token amounts are NUMERIC(78,0) base units. Divide by 10^tokens.decimals for human units (see views).
-- Exchange prices are 1e12 fixed point. Rates/utilization/fees: 1e4 == 100% unless noted.

CREATE TABLE IF NOT EXISTS liquidity_operations (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    protocol TEXT,
    protocol_type TEXT,
    mint TEXT,
    supply_amount NUMERIC(78,0),
    borrow_amount NUMERIC(78,0),
    withdraw_to TEXT,
    borrow_to TEXT,
    supply_exchange_price NUMERIC(78,0),
    borrow_exchange_price NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_slot ON liquidity_operations(slot);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_block_time ON liquidity_operations(block_time);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_tx ON liquidity_operations(tx_id);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_mint ON liquidity_operations(mint);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_protocol ON liquidity_operations(protocol);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_protocol_type ON liquidity_operations(protocol_type);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_withdraw_to ON liquidity_operations(withdraw_to);
CREATE INDEX IF NOT EXISTS idx_liquidity_operations_borrow_to ON liquidity_operations(borrow_to);

CREATE TABLE IF NOT EXISTS liquidity_exchange_prices (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    mint TEXT,
    supply_exchange_price NUMERIC(78,0),
    borrow_exchange_price NUMERIC(78,0),
    borrow_rate_bps INTEGER,
    utilization_bps INTEGER
);
CREATE INDEX IF NOT EXISTS idx_liquidity_exchange_prices_slot ON liquidity_exchange_prices(slot);
CREATE INDEX IF NOT EXISTS idx_liquidity_exchange_prices_block_time ON liquidity_exchange_prices(block_time);
CREATE INDEX IF NOT EXISTS idx_liquidity_exchange_prices_tx ON liquidity_exchange_prices(tx_id);
CREATE INDEX IF NOT EXISTS idx_liquidity_exchange_prices_mint ON liquidity_exchange_prices(mint);

CREATE TABLE IF NOT EXISTS liquidity_revenue_collections (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    mint TEXT,
    revenue_amount NUMERIC(78,0),
    revenue_collector TEXT
);
CREATE INDEX IF NOT EXISTS idx_liquidity_revenue_collections_slot ON liquidity_revenue_collections(slot);
CREATE INDEX IF NOT EXISTS idx_liquidity_revenue_collections_block_time ON liquidity_revenue_collections(block_time);
CREATE INDEX IF NOT EXISTS idx_liquidity_revenue_collections_tx ON liquidity_revenue_collections(tx_id);
CREATE INDEX IF NOT EXISTS idx_liquidity_revenue_collections_mint ON liquidity_revenue_collections(mint);

CREATE TABLE IF NOT EXISTS liquidity_claims (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    user_address TEXT,
    mint TEXT,
    recipient TEXT,
    amount NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_liquidity_claims_slot ON liquidity_claims(slot);
CREATE INDEX IF NOT EXISTS idx_liquidity_claims_block_time ON liquidity_claims(block_time);
CREATE INDEX IF NOT EXISTS idx_liquidity_claims_tx ON liquidity_claims(tx_id);

CREATE TABLE IF NOT EXISTS liquidity_rate_configs (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    mint TEXT,
    version INTEGER,
    kink1_bps NUMERIC(78,0),
    kink2_bps NUMERIC(78,0),
    rate_at_zero_bps NUMERIC(78,0),
    rate_at_kink1_bps NUMERIC(78,0),
    rate_at_kink2_bps NUMERIC(78,0),
    rate_at_max_bps NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_liquidity_rate_configs_slot ON liquidity_rate_configs(slot);
CREATE INDEX IF NOT EXISTS idx_liquidity_rate_configs_block_time ON liquidity_rate_configs(block_time);
CREATE INDEX IF NOT EXISTS idx_liquidity_rate_configs_tx ON liquidity_rate_configs(tx_id);

CREATE TABLE IF NOT EXISTS liquidity_token_configs (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    mint TEXT,
    fee_bps NUMERIC(78,0),
    max_utilization_bps NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_liquidity_token_configs_slot ON liquidity_token_configs(slot);
CREATE INDEX IF NOT EXISTS idx_liquidity_token_configs_block_time ON liquidity_token_configs(block_time);
CREATE INDEX IF NOT EXISTS idx_liquidity_token_configs_tx ON liquidity_token_configs(tx_id);

CREATE TABLE IF NOT EXISTS liquidity_protocol_configs (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    side TEXT,
    protocol TEXT,
    mint TEXT,
    mode INTEGER,
    expand_percent NUMERIC(78,0),
    expand_duration NUMERIC(78,0),
    base_limit NUMERIC(78,0),
    max_limit NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_liquidity_protocol_configs_slot ON liquidity_protocol_configs(slot);
CREATE INDEX IF NOT EXISTS idx_liquidity_protocol_configs_block_time ON liquidity_protocol_configs(block_time);
CREATE INDEX IF NOT EXISTS idx_liquidity_protocol_configs_tx ON liquidity_protocol_configs(tx_id);

CREATE TABLE IF NOT EXISTS earn_deposits (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    sender TEXT,
    receiver TEXT,
    mint TEXT,
    f_token_mint TEXT,
    lending TEXT,
    assets NUMERIC(78,0),
    shares NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_earn_deposits_slot ON earn_deposits(slot);
CREATE INDEX IF NOT EXISTS idx_earn_deposits_block_time ON earn_deposits(block_time);
CREATE INDEX IF NOT EXISTS idx_earn_deposits_tx ON earn_deposits(tx_id);
CREATE INDEX IF NOT EXISTS idx_earn_deposits_mint ON earn_deposits(mint);
CREATE INDEX IF NOT EXISTS idx_earn_deposits_receiver ON earn_deposits(receiver);
CREATE INDEX IF NOT EXISTS idx_earn_deposits_sender ON earn_deposits(sender);

CREATE TABLE IF NOT EXISTS earn_withdrawals (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    sender TEXT,
    receiver TEXT,
    owner TEXT,
    mint TEXT,
    f_token_mint TEXT,
    lending TEXT,
    assets NUMERIC(78,0),
    shares NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_earn_withdrawals_slot ON earn_withdrawals(slot);
CREATE INDEX IF NOT EXISTS idx_earn_withdrawals_block_time ON earn_withdrawals(block_time);
CREATE INDEX IF NOT EXISTS idx_earn_withdrawals_tx ON earn_withdrawals(tx_id);
CREATE INDEX IF NOT EXISTS idx_earn_withdrawals_mint ON earn_withdrawals(mint);
CREATE INDEX IF NOT EXISTS idx_earn_withdrawals_owner ON earn_withdrawals(owner);
CREATE INDEX IF NOT EXISTS idx_earn_withdrawals_receiver ON earn_withdrawals(receiver);

CREATE TABLE IF NOT EXISTS earn_rebalances (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    mint TEXT,
    f_token_mint TEXT,
    assets NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_earn_rebalances_slot ON earn_rebalances(slot);
CREATE INDEX IF NOT EXISTS idx_earn_rebalances_block_time ON earn_rebalances(block_time);
CREATE INDEX IF NOT EXISTS idx_earn_rebalances_tx ON earn_rebalances(tx_id);

CREATE TABLE IF NOT EXISTS earn_reward_events (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    mint TEXT,
    kind TEXT,
    reward_amount NUMERIC(78,0),
    duration BIGINT,
    start_time BIGINT,
    end_time BIGINT
);
CREATE INDEX IF NOT EXISTS idx_earn_reward_events_slot ON earn_reward_events(slot);
CREATE INDEX IF NOT EXISTS idx_earn_reward_events_block_time ON earn_reward_events(block_time);
CREATE INDEX IF NOT EXISTS idx_earn_reward_events_tx ON earn_reward_events(tx_id);

CREATE TABLE IF NOT EXISTS vault_operations (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_id INTEGER,
    vault_config TEXT,
    supply_mint TEXT,
    borrow_mint TEXT,
    nft_id BIGINT,
    signer TEXT,
    recipient TEXT,
    col_change NUMERIC(78,0),
    debt_change NUMERIC(78,0),
    vault_type TEXT
);
CREATE INDEX IF NOT EXISTS idx_vault_operations_slot ON vault_operations(slot);
CREATE INDEX IF NOT EXISTS idx_vault_operations_block_time ON vault_operations(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_operations_tx ON vault_operations(tx_id);
CREATE INDEX IF NOT EXISTS idx_vault_operations_vault_id ON vault_operations(vault_id);
CREATE INDEX IF NOT EXISTS idx_vault_operations_vault_config ON vault_operations(vault_config);
CREATE INDEX IF NOT EXISTS idx_vault_operations_nft_id ON vault_operations(nft_id);
CREATE INDEX IF NOT EXISTS idx_vault_operations_signer ON vault_operations(signer);

CREATE TABLE IF NOT EXISTS vault_position_updates (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_id INTEGER,
    nft_id BIGINT,
    owner TEXT,
    position_mint TEXT,
    tick INTEGER,
    col NUMERIC(78,0),
    debt NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_vault_position_updates_slot ON vault_position_updates(slot);
CREATE INDEX IF NOT EXISTS idx_vault_position_updates_block_time ON vault_position_updates(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_position_updates_tx ON vault_position_updates(tx_id);
CREATE INDEX IF NOT EXISTS idx_vault_position_updates_vault_id_nft_id ON vault_position_updates(vault_id, nft_id);
CREATE INDEX IF NOT EXISTS idx_vault_position_updates_owner ON vault_position_updates(owner);

CREATE TABLE IF NOT EXISTS vault_liquidations (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_id INTEGER,
    vault_config TEXT,
    supply_mint TEXT,
    borrow_mint TEXT,
    liquidator TEXT,
    recipient TEXT,
    col_amount NUMERIC(78,0),
    debt_amount NUMERIC(78,0),
    start_tick INTEGER,
    end_tick INTEGER,
    vault_type TEXT
);
CREATE INDEX IF NOT EXISTS idx_vault_liquidations_slot ON vault_liquidations(slot);
CREATE INDEX IF NOT EXISTS idx_vault_liquidations_block_time ON vault_liquidations(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_liquidations_tx ON vault_liquidations(tx_id);
CREATE INDEX IF NOT EXISTS idx_vault_liquidations_vault_id ON vault_liquidations(vault_id);
CREATE INDEX IF NOT EXISTS idx_vault_liquidations_vault_config ON vault_liquidations(vault_config);
CREATE INDEX IF NOT EXISTS idx_vault_liquidations_liquidator ON vault_liquidations(liquidator);

CREATE TABLE IF NOT EXISTS vault_absorbs (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_id INTEGER,
    vault_config TEXT,
    col_amount NUMERIC(78,0),
    debt_amount NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_vault_absorbs_slot ON vault_absorbs(slot);
CREATE INDEX IF NOT EXISTS idx_vault_absorbs_block_time ON vault_absorbs(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_absorbs_tx ON vault_absorbs(tx_id);

CREATE TABLE IF NOT EXISTS vault_exchange_prices (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_id INTEGER,
    vault_config TEXT,
    vault_supply_exchange_price NUMERIC(78,0),
    vault_borrow_exchange_price NUMERIC(78,0),
    liquidity_supply_exchange_price NUMERIC(78,0),
    liquidity_borrow_exchange_price NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_vault_exchange_prices_slot ON vault_exchange_prices(slot);
CREATE INDEX IF NOT EXISTS idx_vault_exchange_prices_block_time ON vault_exchange_prices(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_exchange_prices_tx ON vault_exchange_prices(tx_id);

CREATE TABLE IF NOT EXISTS vault_rebalances (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_config TEXT,
    supply_amount NUMERIC(78,0),
    borrow_amount NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_vault_rebalances_slot ON vault_rebalances(slot);
CREATE INDEX IF NOT EXISTS idx_vault_rebalances_block_time ON vault_rebalances(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_rebalances_tx ON vault_rebalances(tx_id);

CREATE TABLE IF NOT EXISTS vault_position_closes (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_id INTEGER,
    nft_id BIGINT,
    signer TEXT,
    position_mint TEXT
);
CREATE INDEX IF NOT EXISTS idx_vault_position_closes_slot ON vault_position_closes(slot);
CREATE INDEX IF NOT EXISTS idx_vault_position_closes_block_time ON vault_position_closes(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_position_closes_tx ON vault_position_closes(tx_id);

CREATE TABLE IF NOT EXISTS vault_config_updates (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_config TEXT,
    param TEXT,
    value TEXT
);
CREATE INDEX IF NOT EXISTS idx_vault_config_updates_slot ON vault_config_updates(slot);
CREATE INDEX IF NOT EXISTS idx_vault_config_updates_block_time ON vault_config_updates(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_config_updates_tx ON vault_config_updates(tx_id);

CREATE TABLE IF NOT EXISTS vault_liquidation_rounding_diffs (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    vault_id INTEGER,
    actual_debt_amount NUMERIC(78,0),
    debt_amount NUMERIC(78,0),
    diff NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_vault_liquidation_rounding_diffs_slot ON vault_liquidation_rounding_diffs(slot);
CREATE INDEX IF NOT EXISTS idx_vault_liquidation_rounding_diffs_block_time ON vault_liquidation_rounding_diffs(block_time);
CREATE INDEX IF NOT EXISTS idx_vault_liquidation_rounding_diffs_tx ON vault_liquidation_rounding_diffs(tx_id);

CREATE TABLE IF NOT EXISTS dex_swaps (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    dex_id INTEGER,
    dex TEXT,
    token0 TEXT,
    token1 TEXT,
    swap0_to1 BOOLEAN,
    maker TEXT,
    recipient TEXT,
    amount_in NUMERIC(78,0),
    amount_out NUMERIC(78,0),
    fee_amount_in NUMERIC(78,0),
    reserves_token0 NUMERIC(78,0),
    reserves_token1 NUMERIC(78,0),
    pool_price NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_dex_swaps_slot ON dex_swaps(slot);
CREATE INDEX IF NOT EXISTS idx_dex_swaps_block_time ON dex_swaps(block_time);
CREATE INDEX IF NOT EXISTS idx_dex_swaps_tx ON dex_swaps(tx_id);
CREATE INDEX IF NOT EXISTS idx_dex_swaps_dex_id ON dex_swaps(dex_id);
CREATE INDEX IF NOT EXISTS idx_dex_swaps_maker ON dex_swaps(maker);

CREATE TABLE IF NOT EXISTS dex_liquidity (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    dex_id INTEGER,
    action TEXT,
    shares NUMERIC(78,0),
    token0_amount NUMERIC(78,0),
    token1_amount NUMERIC(78,0),
    user_address TEXT,
    protocol TEXT
);
CREATE INDEX IF NOT EXISTS idx_dex_liquidity_slot ON dex_liquidity(slot);
CREATE INDEX IF NOT EXISTS idx_dex_liquidity_block_time ON dex_liquidity(block_time);
CREATE INDEX IF NOT EXISTS idx_dex_liquidity_tx ON dex_liquidity(tx_id);

CREATE TABLE IF NOT EXISTS dex_arbitrages (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    dex_id INTEGER,
    routing NUMERIC(78,0),
    amount_out NUMERIC(78,0)
);
CREATE INDEX IF NOT EXISTS idx_dex_arbitrages_slot ON dex_arbitrages(slot);
CREATE INDEX IF NOT EXISTS idx_dex_arbitrages_block_time ON dex_arbitrages(block_time);
CREATE INDEX IF NOT EXISTS idx_dex_arbitrages_tx ON dex_arbitrages(tx_id);

CREATE TABLE IF NOT EXISTS flashloan_fee_updates (
    id TEXT PRIMARY KEY,
    tx_id TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    slot BIGINT NOT NULL,
    block_time BIGINT NOT NULL,
    instruction TEXT,
    instruction_index INTEGER,
    caller_program TEXT,
    fee_payer TEXT,
    flashloan_fee INTEGER
);
CREATE INDEX IF NOT EXISTS idx_flashloan_fee_updates_slot ON flashloan_fee_updates(slot);
CREATE INDEX IF NOT EXISTS idx_flashloan_fee_updates_block_time ON flashloan_fee_updates(block_time);
CREATE INDEX IF NOT EXISTS idx_flashloan_fee_updates_tx ON flashloan_fee_updates(tx_id);

-- ------------------------------------------------------------ latest state (upserts)
CREATE TABLE IF NOT EXISTS tokens (
    mint TEXT PRIMARY KEY,
    decimals INTEGER NOT NULL,
    token_program TEXT
);

CREATE TABLE IF NOT EXISTS vaults (
    vault_config TEXT PRIMARY KEY,
    vault_id INTEGER,
    supply_mint TEXT,
    borrow_mint TEXT,
    oracle TEXT,
    supply_dex TEXT,
    borrow_dex TEXT,
    vault_type INTEGER,
    collateral_factor INTEGER,
    liquidation_threshold INTEGER,
    liquidation_max_limit INTEGER,
    liquidation_penalty INTEGER,
    borrow_fee INTEGER,
    created_slot BIGINT,
    created_tx TEXT
);
CREATE INDEX IF NOT EXISTS idx_vaults_vault_id ON vaults(vault_id);

-- latest LogUserPosition per position NFT (vault program versions that emit it)
CREATE TABLE IF NOT EXISTS vault_positions (
    vault_id INTEGER NOT NULL,
    nft_id BIGINT NOT NULL,
    owner TEXT,
    position_mint TEXT,
    tick INTEGER,
    col NUMERIC(78,0),
    debt NUMERIC(78,0),
    last_slot BIGINT,
    last_block_time BIGINT,
    last_tx TEXT,
    PRIMARY KEY (vault_id, nft_id)
);
CREATE INDEX IF NOT EXISTS idx_vault_positions_owner ON vault_positions(owner);

CREATE TABLE IF NOT EXISTS earn_markets (
    f_token_mint TEXT PRIMARY KEY,
    mint TEXT,
    lending TEXT,
    token_exchange_price NUMERIC(78,0),
    liquidity_exchange_price NUMERIC(78,0),
    last_slot BIGINT,
    last_block_time BIGINT
);

CREATE TABLE IF NOT EXISTS dexes (
    dex_id INTEGER PRIMARY KEY,
    dex TEXT,
    token0 TEXT,
    token1 TEXT,
    smart_col BOOLEAN,
    smart_debt BOOLEAN,
    fee BIGINT,
    revenue_cut BIGINT,
    created_slot BIGINT
);

-- ------------------------------------------------------------ daily aggregates (UTC day, delta upserts)
CREATE TABLE IF NOT EXISTS liquidity_daily (
    day TEXT NOT NULL,
    mint TEXT NOT NULL,
    protocol_type TEXT NOT NULL,
    operations BIGINT DEFAULT 0,
    supplied NUMERIC(78,0) DEFAULT 0,
    withdrawn NUMERIC(78,0) DEFAULT 0,
    borrowed NUMERIC(78,0) DEFAULT 0,
    repaid NUMERIC(78,0) DEFAULT 0,
    PRIMARY KEY (day, mint, protocol_type)
);

CREATE TABLE IF NOT EXISTS token_daily (
    day TEXT NOT NULL,
    mint TEXT NOT NULL,
    operations BIGINT DEFAULT 0,
    net_supply_change NUMERIC(78,0) DEFAULT 0,
    net_borrow_change NUMERIC(78,0) DEFAULT 0,
    revenue_collected NUMERIC(78,0) DEFAULT 0,
    first_supply_exchange_price NUMERIC(78,0),
    first_borrow_exchange_price NUMERIC(78,0),
    last_supply_exchange_price NUMERIC(78,0),
    last_borrow_exchange_price NUMERIC(78,0),
    last_borrow_rate_bps INTEGER,
    last_utilization_bps INTEGER,
    last_slot BIGINT,
    PRIMARY KEY (day, mint)
);

CREATE TABLE IF NOT EXISTS earn_daily (
    day TEXT NOT NULL,
    mint TEXT NOT NULL,
    deposits BIGINT DEFAULT 0,
    withdrawals BIGINT DEFAULT 0,
    deposited NUMERIC(78,0) DEFAULT 0,
    withdrawn NUMERIC(78,0) DEFAULT 0,
    first_token_exchange_price NUMERIC(78,0),
    last_token_exchange_price NUMERIC(78,0),
    last_liquidity_exchange_price NUMERIC(78,0),
    PRIMARY KEY (day, mint)
);

CREATE TABLE IF NOT EXISTS vault_daily (
    day TEXT NOT NULL,
    vault_config TEXT NOT NULL,
    vault_id INTEGER,
    operations BIGINT DEFAULT 0,
    col_deposited NUMERIC(78,0) DEFAULT 0,
    col_withdrawn NUMERIC(78,0) DEFAULT 0,
    debt_borrowed NUMERIC(78,0) DEFAULT 0,
    debt_repaid NUMERIC(78,0) DEFAULT 0,
    liquidations BIGINT DEFAULT 0,
    liquidated_col NUMERIC(78,0) DEFAULT 0,
    liquidated_debt NUMERIC(78,0) DEFAULT 0,
    PRIMARY KEY (day, vault_config)
);

CREATE TABLE IF NOT EXISTS dex_daily (
    day TEXT NOT NULL,
    dex_id TEXT NOT NULL,
    swaps BIGINT DEFAULT 0,
    volume_in_token0 NUMERIC(78,0) DEFAULT 0,
    volume_in_token1 NUMERIC(78,0) DEFAULT 0,
    fees_token0 NUMERIC(78,0) DEFAULT 0,
    fees_token1 NUMERIC(78,0) DEFAULT 0,
    PRIMARY KEY (day, dex_id)
);

-- ------------------------------------------------------------ convenience views (human units)
CREATE OR REPLACE VIEW v_liquidity_operations AS
SELECT o.*, t.decimals,
       o.supply_amount / power(10::numeric, t.decimals) AS supply_amount_ui,
       o.borrow_amount / power(10::numeric, t.decimals) AS borrow_amount_ui,
       to_timestamp(o.block_time) AS ts
FROM liquidity_operations o LEFT JOIN tokens t ON t.mint = o.mint;

CREATE OR REPLACE VIEW v_liquidity_daily AS
SELECT d.*, t.decimals,
       d.supplied / power(10::numeric, t.decimals) AS supplied_ui,
       d.withdrawn / power(10::numeric, t.decimals) AS withdrawn_ui,
       d.borrowed / power(10::numeric, t.decimals) AS borrowed_ui,
       d.repaid / power(10::numeric, t.decimals) AS repaid_ui
FROM liquidity_daily d LEFT JOIN tokens t ON t.mint = d.mint;

CREATE OR REPLACE VIEW v_vault_liquidations AS
SELECT l.*, to_timestamp(l.block_time) AS ts,
       l.col_amount / power(10::numeric, ts_.decimals) AS col_amount_ui,
       l.debt_amount / power(10::numeric, tb.decimals) AS debt_amount_ui
FROM vault_liquidations l
LEFT JOIN tokens ts_ ON ts_.mint = l.supply_mint AND l.vault_type = 'T1'
LEFT JOIN tokens tb ON tb.mint = l.borrow_mint AND l.vault_type = 'T1';
