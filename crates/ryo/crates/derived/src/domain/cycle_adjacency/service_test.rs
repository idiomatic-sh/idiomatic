//! The `cycles` service's cases, on values alone. The adjacency is built in one read of each edge row, whatever the graph.

use super::service::build;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::{BulkRows, RowCells};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    TwoNodeLoop,
    DuplicateEdgesAreKept,
    AbsentDestination,
    /// A thousand edges from one node to itself.
    ThousandLoops,
}
type Input = Scenario;
/// The destination of each edge, grouped by source, and how many edge rows the build read.
type Value = (Cow<'static, [usize]>, usize);
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_two_node_loop_leaves_each_node_for_the_other"),
        word: Word::Succeeds,
        input: || Scenario::TwoNodeLoop,
        expect: Expect::Value((Cow::Borrowed(&[1, 0]), 2)),
    },
    Case {
        name: CaseName("duplicate_edges_stay_duplicate"),
        word: Word::Succeeds,
        input: || Scenario::DuplicateEdgesAreKept,
        expect: Expect::Value((Cow::Borrowed(&[1, 1]), 2)),
    },
    Case {
        name: CaseName("a_thousand_edges_are_each_read_once"),
        word: Word::Succeeds,
        input: || Scenario::ThousandLoops,
        expect: Expect::Value((Cow::Borrowed(&[0; 1000]), 1000)),
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

/// Edge rows that count how many times the build reads one.
struct Counted {
    rows: Table,
    reads: std::cell::Cell<usize>,
}

impl RowCells for Counted {
    fn bulk(&self) -> Option<BulkRows<'_>> {
        None
    }
    fn read_row<'a>(&'a self, row: RowIndex, buffer: &'a mut Vec<Cell>) -> Option<&'a [Cell]> {
        self.reads.set(self.reads.get() + 1);
        self.rows.read_row(row, buffer)
    }
    fn validate(&self) -> Result<(), DerivedFault> {
        self.rows.validate()
    }
    fn kinds(&self) -> &[CellKind] {
        self.rows.kinds()
    }
    fn count(&self) -> RowCount {
        self.rows.count()
    }
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Cell> {
        self.rows.cell(row, column)
    }
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let (rows, destination) = match input {
        Scenario::TwoNodeLoop => (edges(&[[1, 2], [2, 1]]), ColumnIndex(1)),
        Scenario::DuplicateEdgesAreKept => (edges(&[[1, 2], [1, 2]]), ColumnIndex(1)),
        Scenario::AbsentDestination => (edges(&[[1, 2]]), ColumnIndex(3)),
        Scenario::ThousandLoops => (edges(&[[7, 7]; 1000]), ColumnIndex(1)),
    };
    let counted = Counted { rows, reads: std::cell::Cell::new(0) };
    let graph = build(&counted, ColumnIndex(0), destination, &mut Vec::new())?;
    Ok((Cow::Owned(graph.leaving.0.clone()), counted.reads.get()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
