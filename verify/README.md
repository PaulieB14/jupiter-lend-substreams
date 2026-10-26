# verify/

Independent verification tooling (Python), not used by the package itself.

- `idldec.py`: an IDL-driven decoder for Solana RPC `getTransaction` JSON, written separately from the Rust decoder and used as the oracle.
- `compare.py out.jsonl [--sigs=sigs.json]`: compares `substreams run ... map_events -o jsonl` output field by field with `idldec`, and reports missing, extra and mismatched events.
- `rangesigs.py LO HI`: collects the successful Lend-program signatures in a slot range for the completeness check, then `txset.py` compares them.
- `fees_xcheck.py`: estimates daily borrow interest from exchange-price growth × TokenReserve totals and compares it with DefiLlama.
- `gen_fixtures.py`: writes the `tests/fixtures/*.tx.json` and `*.expected.json` pairs.

RPC defaults to `https://api.mainnet-beta.solana.com` and uses `maxSupportedTransactionVersion: 1`.
