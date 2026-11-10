//! A selection keeps the deterministic order of the audited inventory.
use super::service::select;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::case;
use ryo_benchmark_contracts::domain::requests::types::{
    CaseSelection, ShardPartition,
    case_shard::value::{CaseShard, ShardCount, ShardIndex},
};
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;

/// Which shard a row selects.
#[derive(Clone, Copy)]
struct Request {
    shard: Option<(usize, usize)>,
}

/// What a selection admits.
#[derive(Debug, PartialEq, Eq)]
struct Chosen {
    global: usize,
    /// The first letter of each admitted case name in order, padded with a dash.
    names: [char; 3],
}

type Input = Request;
type Value = Chosen;
type Failure = std::convert::Infallible;

const SCRAMBLED: [&str; 3] = ["c", "a", "b"];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an unsharded inventory selects every case in rule and name order"),
        word: Word::Succeeds,
        input: || Request {
            shard: None,
        },
        expect: Expect::Value(Chosen {
            global: 3,
            names: ['a', 'b', 'c'],
        }),
    },
    Case {
        name: CaseName(
            "the second of two shards selects the odd positions of the sorted inventory",
        ),
        word: Word::Succeeds,
        input: || Request {
            shard: Some((1, 2)),
        },
        expect: Expect::Value(Chosen {
            global: 3,
            names: ['b', '-', '-'],
        }),
    },
];

fn inventory() -> Vec<PreparedCase> {
    SCRAMBLED
        .iter()
        .map(|name| {
            let mut prepared = case();
            prepared.name = (*name).into();
            prepared
        })
        .collect()
}

fn subject(request: Input) -> Value {
    let shard = request.shard.map_or(ShardPartition::All, |(index, count)| {
        ShardPartition::Shard(
            CaseShard::new(ShardIndex(index), ShardCount(count)).expect("fixture shard"),
        )
    });
    let chosen = select(inventory(), CaseSelection { shard });
    let mut names = ['-'; 3];
    for (slot, selected) in names.iter_mut().zip(&chosen.selected_cases) {
        *slot = selected.name.to_string().chars().next().unwrap_or('-');
    }
    Chosen {
        global: chosen.global_count.0,
        names,
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |request| Ok::<_, Failure>(subject(request)));
}
