//! A selection keeps the deterministic order of the audited inventory and never reaches outside its shard.
use super::constants;
use super::service::select;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::case;
use ryo_benchmark_contracts::domain::requests::types::{
    CaseId, CaseSelection, IdSelection, ShardPartition,
    case_shard::value::{CaseShard, ShardCount, ShardIndex},
};
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;

/// Which shard and which case IDs a row selects.
#[derive(Clone, Copy)]
struct Request {
    shard: Option<(usize, usize)>,
    ids: Option<&'static [&'static str]>,
}

/// What a selection admits.
#[derive(Debug, PartialEq, Eq)]
struct Chosen {
    global: usize,
    /// The first letter of each admitted case name in order, padded with a dash.
    names: [char; 3],
}

/// What a refused selection states.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    EmptyOrOutside,
    Unexpected,
}

type Input = Request;
type Value = Chosen;
type Failure = Refusal;

const SCRAMBLED: [&str; 3] = ["c", "a", "b"];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an unsharded inventory selects every case in rule and name order"),
        word: Word::Succeeds,
        input: || Request { shard: None, ids: None },
        expect: Expect::Value(Chosen {
            global: 3,
            names: ['a', 'b', 'c'],
        }),
    },
    Case {
        name: CaseName("the second of two shards selects the odd positions of the sorted inventory"),
        word: Word::Succeeds,
        input: || Request { shard: Some((1, 2)), ids: None },
        expect: Expect::Value(Chosen {
            global: 3,
            names: ['b', '-', '-'],
        }),
    },
    Case {
        name: CaseName("a subset inside the shard keeps exactly its cases"),
        word: Word::Succeeds,
        input: || Request { shard: None, ids: Some(&["a", "c"]) },
        expect: Expect::Value(Chosen {
            global: 3,
            names: ['a', 'c', '-'],
        }),
    },
    Case {
        name: CaseName("a subset naming a case outside its shard is refused"),
        word: Word::Refuses,
        input: || Request { shard: Some((1, 2)), ids: Some(&["a"]) },
        expect: Expect::Error(Refusal::EmptyOrOutside),
    },
    Case {
        name: CaseName("an empty subset is refused"),
        word: Word::Refuses,
        input: || Request { shard: None, ids: Some(&[]) },
        expect: Expect::Error(Refusal::EmptyOrOutside),
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

fn subject(request: Input) -> Result<Value, Failure> {
    let subset: std::collections::BTreeSet<CaseId> = request
        .ids
        .unwrap_or_default()
        .iter()
        .map(|name| CaseId(format!("{}/{name}", case().rule)))
        .collect();
    let shard = request.shard.map_or(ShardPartition::All, |(index, count)| {
        ShardPartition::Shard(
            CaseShard::new(ShardIndex(index), ShardCount(count)).expect("fixture shard"),
        )
    });
    let ids = if request.ids.is_some() {
        IdSelection::Subset(&subset)
    } else {
        IdSelection::WholeShard
    };
    let chosen = select(inventory(), CaseSelection { shard, ids }).map_err(|error| {
        if error.to_string() == constants::CASE_SELECTION_IS_EMPTY_OR_OUTSIDE {
            Refusal::EmptyOrOutside
        } else {
            Refusal::Unexpected
        }
    })?;
    let mut names = ['-'; 3];
    for (slot, selected) in names.iter_mut().zip(&chosen.selected_cases) {
        *slot = selected.name.to_string().chars().next().unwrap_or('-');
    }
    Ok(Chosen {
        global: chosen.global_count.0,
        names,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
