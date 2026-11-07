//! The `adjacency` service's cases, on values alone.

use super::service::build_borrowed;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use crate::domain::vertices::types::EdgeDirection;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    ChainHasThreeVertices,
    ChainHasAForest,
    LoopHasNoForest,
    AbsentEndpoint,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_chain_of_two_edges_has_three_vertices"),
        word: Word::Succeeds,
        input: || Scenario::ChainHasThreeVertices,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_chain_has_a_validated_forest"),
        word: Word::Succeeds,
        input: || Scenario::ChainHasAForest,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_loop_has_no_forest"),
        word: Word::Succeeds,
        input: || Scenario::LoopHasNoForest,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("an_endpoint_column_the_rows_lack_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AbsentEndpoint,
        expect: Expect::Error(DerivedFault::NoSuchColumn { column: ColumnIndex(7) }),
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
    let ends = [ColumnIndex(0), ColumnIndex(1)];
    let build = |rows: &Table, ends: [ColumnIndex; 2]| build_borrowed(rows, ends, &[]);
    match input {
        Scenario::ChainHasThreeVertices => Ok(build(&edges(&[[1, 2], [2, 3]]), ends)?
            .map_or(0, |graph| graph.vertices().len())),
        Scenario::ChainHasAForest => Ok(build(&edges(&[[1, 2], [2, 3]]), ends)?
            .map_or(0, |graph| usize::from(graph.forest(EdgeDirection::Forward).is_some()))),
        Scenario::LoopHasNoForest => Ok(build(&edges(&[[1, 2], [2, 1]]), ends)?
            .map_or(1, |graph| usize::from(graph.forest(EdgeDirection::Forward).is_some()))),
        Scenario::AbsentEndpoint => build(&edges(&[[1, 2]]), [ColumnIndex(0), ColumnIndex(7)])
            .map(|_| 0),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
