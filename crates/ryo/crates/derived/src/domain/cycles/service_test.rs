//! The `cycles` service's cases, on values alone.

use super::service::build;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    TwoNodeLoop,
    DuplicateEdgesAreKept,
    AbsentDestination,
}
type Input = Scenario;
type Value = Cow<'static, [usize]>;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_two_node_loop_leaves_each_node_for_the_other"),
        word: Word::Succeeds,
        input: || Scenario::TwoNodeLoop,
        expect: Expect::Value(Cow::Borrowed(&[1, 0])),
    },
    Case {
        name: CaseName("duplicate_edges_stay_duplicate"),
        word: Word::Succeeds,
        input: || Scenario::DuplicateEdgesAreKept,
        expect: Expect::Value(Cow::Borrowed(&[1, 1])),
    },
    Case {
        name: CaseName("a_destination_column_the_rows_lack_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AbsentDestination,
        expect: Expect::Error(DerivedFault::NoSuchColumn { column: ColumnIndex(3) }),
    },
];

fn edges(pairs: &[[i64; 2]]) -> Table {
    Table {
        kinds: vec![CellKind::Number, CellKind::Number],
        cells: pairs
            .iter()
            .flatten()
            .map(|cell| Cell::Number(Integer(*cell)))
            .collect(),
        count: RowCount(pairs.len()),
    }
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let (rows, destination) = match input {
        Scenario::TwoNodeLoop => (edges(&[[1, 2], [2, 1]]), ColumnIndex(1)),
        Scenario::DuplicateEdgesAreKept => (edges(&[[1, 2], [1, 2]]), ColumnIndex(1)),
        Scenario::AbsentDestination => (edges(&[[1, 2]]), ColumnIndex(3)),
    };
    let graph = build(&rows, ColumnIndex(0), destination, &mut Vec::new())?;
    Ok(Cow::Owned(graph.leaving.iter().map(|node| node.0).collect()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
