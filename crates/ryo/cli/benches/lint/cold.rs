//! The cold cost of ruff, in this process: every configured rule together on ten workers, over
//! the pinned ruff with no store of ryo's own.

use std::sync::Arc;
use std::time::Instant;

use crate::bmf::{COLD_ROUNDS, Cost};

/// The workers a run over ruff is taken on.
const WORKERS: usize = 10;

/// What a lint over ruff with these rules cost: wall and process CPU (user and kernel), in milliseconds.
fn cost_of(rules: Vec<ryo_core::inbound::use_cases::lint::types::Rule>) -> (u128, u128) {
    // The target is read once before the timer, so that every round finds it in the page cache (`phases::pre_read`).
    crate::phases::pre_read(&crate::base::snapshot::ruff());
    let (wall, cpu) = (Instant::now(), cpu_time::ProcessTime::now());
    let counted = crate::counting::lint_with_workers(
        &crate::base::snapshot::ruff(),
        rules,
        Arc::new(ryo_core::outbound::lint_store::factory::discard()),
        WORKERS,
    )
    .expect("the rules run");
    let cost = (wall.elapsed().as_millis(), cpu.elapsed().as_millis());
    assert!(counted.reads > 0, "a cold run reads ruff's files");
    cost
}

/// Every live rule together, cold: the wall time and the CPU of [`COLD_ROUNDS`] rounds.
pub(crate) fn every_rule() -> Vec<Cost> {
    let every: Vec<_> = crate::counting::runnable()
        .into_iter()
        .map(|(_, rule)| rule)
        .collect();
    eprintln!(
        "{} rules on {WORKERS} workers, {COLD_ROUNDS} rounds",
        every.len()
    );
    let (walls, cpus): (Vec<u128>, Vec<u128>) = std::iter::repeat_with(|| cost_of(every.clone()))
        .take(COLD_ROUNDS)
        .unzip();
    vec![
        Cost {
            target: "ruff",
            metric: "cold-milliseconds",
            rounds: walls,
        },
        Cost {
            target: "ruff",
            metric: "cold-cpu-milliseconds",
            rounds: cpus,
        },
    ]
}
