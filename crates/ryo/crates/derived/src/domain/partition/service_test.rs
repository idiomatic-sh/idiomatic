//! The `partition` service's cases, on values alone.

use super::service::ordered_positions;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    ClusteredKeepsOriginalOrder,
    ReturningRunsSortByHash,
    AbsentColumn,
}
type Input = Scenario;
type Value = Cow<'static, [usize]>;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_clustered_stream_keeps_its_original_order"),
        word: Word::Succeeds,
        input: || Scenario::ClusteredKeepsOriginalOrder,
        expect: Expect::Value(Cow::Borrowed(&[0, 1, 2])),
    },
    Case {
        name: CaseName("returning_key_runs_sort_by_hash_then_row"),
        word: Word::Succeeds,
        input: || Scenario::ReturningRunsSortByHash,
        // Keys 1, 2, 1: under `mix` (one `FxHasher` step) key 2 hashes to 0x5173_154e_2b89_abd7, below key 1's
        // 0xa8b9_8aa7_17c4_d5eb, so the run of key 2 (row 1) comes first, then key 1's rows in row order.
        expect: Expect::Value(Cow::Borrowed(&[1, 0, 2])),
    },
    Case {
        name: CaseName("a_key_column_the_rows_lack_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AbsentColumn,
        expect: Expect::Error(DerivedFault::NoSuchColumn { column: ColumnIndex(5) }),
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
    let (rows, column) = match input {
        Scenario::ClusteredKeepsOriginalOrder => (table(&[1, 1, 2]), ColumnIndex(0)),
        Scenario::ReturningRunsSortByHash => (table(&[1, 2, 1]), ColumnIndex(0)),
        Scenario::AbsentColumn => (table(&[1]), ColumnIndex(5)),
    };
    Ok(Cow::Owned(
        ordered_positions(&rows, &[column])?
            .iter()
            .map(|row| row.0)
            .collect(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
