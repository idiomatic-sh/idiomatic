//! The `dfs` service's cases, on values alone.

use super::service::build;
use super::types::DfsColumns;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fault::types::DerivedFault;
use crate::domain::row_cells::types::{CanonicalTexts, ValueTexts};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    EveryEdgeHasARun,
    DuplicateOrderIsRefused,
    AbsentOrderColumn,
}
type Input = Scenario;
type Value = usize;
type Failure = DerivedFault;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_edge_has_the_run_of_edges_leaving_its_vertex"),
        word: Word::Succeeds,
        input: || Scenario::EveryEdgeHasARun,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("two_edges_with_one_key_and_one_order_are_refused"),
        word: Word::Refuses,
        input: || Scenario::DuplicateOrderIsRefused,
        expect: Expect::Error(DerivedFault::DuplicateDfsOrder),
    },
    Case {
        name: CaseName("an_order_column_the_edges_lack_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AbsentOrderColumn,
        expect: Expect::Error(DerivedFault::NoSuchColumn {
            column: ColumnIndex(8),
        }),
    },
];

struct NoTexts;
impl ValueTexts for NoTexts {
    fn text(&self, _value: Cell) -> Option<CellText<'_>> {
        None
    }
}
impl CanonicalTexts for NoTexts {
    fn intern(&mut self, _text: CellText<'_>) -> Result<Cell, DerivedFault> {
        Err(DerivedFault::WrongOperation)
    }
}

fn numbers(width: usize, cells: &[i64]) -> Table {
    Table {
        kinds: vec![CellKind::Number; width],
        cells: cells
            .iter()
            .map(|cell| Cell::Number(Integer(*cell)))
            .collect(),
        count: RowCount(cells.len() / width),
    }
}

/// Calls the part on one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let edges = match input {
        Scenario::EveryEdgeHasARun | Scenario::AbsentOrderColumn => {
            numbers(4, &[1, 2, 5, 0, 1, 3, 6, 1])
        }
        Scenario::DuplicateOrderIsRefused => numbers(4, &[1, 2, 5, 0, 1, 3, 5, 1]),
    };
    let order = if matches!(input, Scenario::AbsentOrderColumn) {
        ColumnIndex(8)
    } else {
        ColumnIndex(2)
    };
    let columns = DfsColumns {
        root: ColumnIndex(0),
        root_keys: &[],
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        order,
        edge: ColumnIndex(3),
        edge_keys: &[],
    };
    let prepared = build(
        columns,
        &numbers(1, &[1]),
        &edges,
        &mut NoTexts,
        &mut Vec::new(),
    )?;
    Ok(prepared.adjacency.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
