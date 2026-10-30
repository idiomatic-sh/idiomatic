//! The cold cost of ruff, in this process: every live rule together, and the prototype's eight rules, on ten workers, over
//! the pinned ruff with no store of ryo's own.

use std::sync::Arc;
use std::time::Instant;

use crate::bmf::{COLD_ROUNDS, Cost, ROUNDS};

/// The workers a run over ruff is taken on.
const WORKERS: usize = 10;

/// The eight rules the prototype ran, which the prototype's CPU is the cost of.
const SPIKE_RULES: [&str; 8] = [
    "allow-attributes-per-file",
    "deep-function-nesting",
    "deep-nesting",
    "duplicate-function-bodies",
    "empty-else",
    "file-length",
    "glob-imports",
    "match-on-bool",
];

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

/// The prototype's eight rules, cold: the process CPU (user and kernel) of [`ROUNDS`] rounds. The prototype over ruff on this
/// machine, `./spike-bin <ruff copy> 10 <out.json> 10 events`, reported 1390 ms of CPU (the walk, the visit and the
/// finish, as its own `TIMINGS` line reports them: `cpu_ms` walk=231.8 visit=1147.7 finish=11.7, 2026-10-17;
/// `/usr/bin/time` gave user 0.73 s and system 0.67 s; the report's "CPU about 0.7 s" is the user time alone).
pub(crate) fn spike() -> Vec<Cost> {
    let eight: Vec<_> = crate::counting::runnable()
        .into_iter()
        .filter(|(name, _)| SPIKE_RULES.contains(&name.as_str()))
        .map(|(_, rule)| rule)
        .collect();
    assert_eq!(
        eight.len(),
        SPIKE_RULES.len(),
        "all of the prototype's rules are live"
    );
    let cpus = std::iter::repeat_with(|| cost_of(eight.clone()).1)
        .take(ROUNDS)
        .collect();
    vec![Cost {
        target: "ruff",
        metric: "spike-cpu-milliseconds",
        rounds: cpus,
    }]
}
