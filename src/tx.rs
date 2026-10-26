//! Chain-agnostic transaction view used by the decoder (built from a Firehose block in
//! `map_events`, or from Solana RPC JSON in tests), plus log -> instruction attribution.
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Ix {
    pub program_id: String,
    pub data: Vec<u8>,
    pub accounts: Vec<String>,
    pub stack_height: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TxInput {
    pub slot: u64,
    pub block_time: i64,
    pub tx_id: String,
    pub tx_index: u32,
    pub fee_payer: String,
    /// Flattened instructions in execution order: top-level #0, its CPIs, top-level #1, ...
    pub walk: Vec<Ix>,
    pub logs: Vec<String>,
    /// mint -> (decimals, token program) from pre/post token balances
    pub mints: HashMap<String, (u32, String)>,
}

const PRECOMPILES: [&str; 3] = [
    "Ed25519SigVerify111111111111111111111111111",
    "KeccakSecp256k11111111111111111111111111111",
    "Secp256r1SigVerify1111111111111111111111111",
];

/// One `Program data:` payload attributed to the instruction that emitted it.
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub walk_index: usize,
    pub caller: Option<usize>,
    pub data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Attribution {
    pub events: Vec<RawEvent>,
    /// logs were truncated by the runtime or the invoke stack did not line up with the
    /// instruction walk: events after that point cannot be attributed safely and are dropped.
    pub incomplete: bool,
}

/// Walk the log lines with an invoke stack. The k-th `Program <id> invoke [n]` line is the
/// k-th instruction of the flattened walk (Solana records inner instructions in execution
/// order), so every `Program data:` line maps to exactly one emitting instruction.
pub fn attribute(tx: &TxInput, wanted: &dyn Fn(&str) -> bool) -> Attribution {
    let mut out = Attribution::default();
    let mut stack: Vec<usize> = Vec::new();
    let mut next = 0usize;
    for line in &tx.logs {
        if let Some(rest) = line.strip_prefix("Program ") {
            if rest.starts_with("data: ") {
                let Some(&top) = stack.last() else { continue };
                if !wanted(&tx.walk[top].program_id) {
                    continue;
                }
                use base64::Engine;
                if let Ok(data) =
                    base64::engine::general_purpose::STANDARD.decode(&rest["data: ".len()..])
                {
                    let caller = if stack.len() > 1 { Some(stack[stack.len() - 2]) } else { None };
                    out.events.push(RawEvent { walk_index: top, caller, data });
                }
                continue;
            }
            if rest.starts_with("log: ") || rest.starts_with("return: ") {
                continue;
            }
            // "Program <id> invoke [n]" | "Program <id> success" | "Program <id> failed: ..."
            let mut parts = rest.splitn(2, ' ');
            let pid = parts.next().unwrap_or("");
            let tail = parts.next().unwrap_or("");
            if tail.starts_with("invoke [") {
                // precompiles (ed25519 / secp256k1 / secp256r1) execute without invoke logs
                while next < tx.walk.len()
                    && tx.walk[next].program_id != pid
                    && PRECOMPILES.contains(&tx.walk[next].program_id.as_str())
                {
                    next += 1;
                }
                if next >= tx.walk.len() || tx.walk[next].program_id != pid {
                    out.incomplete = true;
                    return out;
                }
                stack.push(next);
                next += 1;
            } else if tail == "success" || tail.starts_with("failed") {
                stack.pop();
            }
        } else if line.starts_with("Log truncated") {
            out.incomplete = true;
            return out;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ix(p: &str) -> Ix {
        Ix { program_id: p.into(), ..Default::default() }
    }
    #[test]
    fn nested_attribution() {
        let tx = TxInput {
            walk: vec![ix("A"), ix("B"), ix("C"), ix("B")],
            logs: vec![
                "Program A invoke [1]".into(),
                "Program data: AQ==".into(),
                "Program B invoke [2]".into(),
                "Program C invoke [3]".into(),
                "Program data: Ag==".into(),
                "Program C success".into(),
                "Program data: Aw==".into(),
                "Program B success".into(),
                "Program B invoke [2]".into(),
                "Program log: Program data: fake".into(),
                "Program data: BA==".into(),
                "Program B success".into(),
                "Program A success".into(),
            ],
            ..Default::default()
        };
        let a = attribute(&tx, &|_| true);
        assert!(!a.incomplete);
        let got: Vec<(usize, Option<usize>, u8)> =
            a.events.iter().map(|e| (e.walk_index, e.caller, e.data[0])).collect();
        assert_eq!(got, vec![(0, None, 1), (2, Some(1), 2), (1, Some(0), 3), (3, Some(0), 4)]);
        let only_b = attribute(&tx, &|p| p == "B");
        assert_eq!(only_b.events.len(), 2);
    }
    #[test]
    fn truncated_and_mismatch() {
        let tx = TxInput {
            walk: vec![ix("A")],
            logs: vec!["Program A invoke [1]".into(), "Log truncated".into()],
            ..Default::default()
        };
        assert!(attribute(&tx, &|_| true).incomplete);
        let tx2 = TxInput {
            walk: vec![ix("A")],
            logs: vec!["Program Z invoke [1]".into()],
            ..Default::default()
        };
        assert!(attribute(&tx2, &|_| true).incomplete);
    }
}
