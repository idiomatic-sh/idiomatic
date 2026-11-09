//! The `equality` service's cases, on values alone.

use super::service::{build, capacities};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    ChainsEveryRow,
    EqualKeysShareAHash,
    RecycledBuffersKeepTheirCapacity,
    AbsentColumn,
}
type Input = Scenario;
type Value = u64;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_row_is_indexed_and_chained"),
        word: Word::Succeeds,
        input: || Scenario::ChainsEveryRow,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("equal_keys_share_one_hash"),
        word: Word::Succeeds,
        input: || Scenario::EqualKeysShareAHash,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_recycled_index_keeps_the_capacity_it_grew"),
        word: Word::Succeeds,
        input: || Scenario::RecycledBuffersKeepTheirCapacity,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_column_the_rows_lack_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AbsentColumn,
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(4),
        }),
    },
];

fn table(keys: &[i64]) -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: keys.iter().map(|key| Cell::Number(Integer(*key))).collect(),
        count: RowCount(keys.len()),
    }
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let first = [ColumnIndex(0)];
    Ok(match input {
        Scenario::ChainsEveryRow => build(&table(&[5, 6, 5]), &first, None)?.rows.len() as u64,
        Scenario::EqualKeysShareAHash => build(&table(&[5, 6, 5]), &first, None)?
            .hashes
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len() as u64,
        Scenario::RecycledBuffersKeepTheirCapacity => {
            let grown = build(&table(&[1, 2, 3, 4]), &first, None)?;
            let before = capacities(&grown).rows;
            let again = build(&table(&[1]), &first, Some(grown))?;
            u64::from(capacities(&again).rows >= before)
        }
        Scenario::AbsentColumn => build(&table(&[1]), &[ColumnIndex(4)], None)?.rows.len() as u64,
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
