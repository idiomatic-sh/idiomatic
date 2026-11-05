//! The `table_reach` service's cases, on values alone.

use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, Value as Cell};

use super::borrowed::reach_breadth;
use super::service::reach_all;
use super::types::{Edges, Walk};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operator::contract::views::{BorrowedRows, IndexedRows};
use crate::domain::positions::service_test::prepared_index;
use crate::domain::table_model::types::Rows;
use ryo_values::domain::rows::errors::TableModelError;
use ryo_values::domain::rows::service::push_row;

use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, TableModelError>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// Why a scenario is refused.
type Failure = TableModelError;

/// Three edges: 1 leads to 2 and to 4, and 2 leads to 3.
fn edges_table() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for [from, to] in [[1, 2], [2, 3], [1, 4]] {
        push_row(
            &mut table,
            &[Cell::Number(Integer(from)), Cell::Number(Integer(to))],
        );
    }
    table
}

/// The entities a walk from 1 reaches, nearest first, or in ascending order.
fn reached_from_one(breadth: bool) -> Result<Vec<Cell>, TableModelError> {
    let table = edges_table();
    let (from, mut walk) = ([ColumnIndex(0)], Walk::default());
    let order = prepared_index(&Rows::Computed(&table), &from)?;
    let edges = Edges {
        table: &table,
        order: &order,
        from: &from,
        to: ColumnIndex(1),
    };
    if breadth {
        let indexed = IndexedRows {
            #[cfg(feature = "diagnostic-row-progress")]
            progress: |_, _| {},
            rows: BorrowedRows::Computed(&table),
            columns: &from,
            index: &order,
        };
        reach_breadth(
            &indexed,
            ColumnIndex(1),
            Cell::Number(Integer(1)),
            &mut walk,
        );
    } else {
        reach_all(&edges, Cell::Number(Integer(1)), &mut walk);
    }
    Ok(walk.reached)
}

fn reaches_nearest_first() -> Result<Value, TableModelError> {
    let reached = reached_from_one(true)?;
    Ok(usize::from(
        reached
            == [
                Cell::Number(Integer(2)),
                Cell::Number(Integer(4)),
                Cell::Number(Integer(3)),
            ],
    ))
}

fn reaches_each_once_in_order() -> Result<Value, TableModelError> {
    let reached = reached_from_one(false)?;
    Ok(usize::from(
        reached
            == [
                Cell::Number(Integer(2)),
                Cell::Number(Integer(3)),
                Cell::Number(Integer(4)),
            ],
    ))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_breadth_walk_reaches_the_nearest_first"),
        word: Word::Succeeds,
        input: || reaches_nearest_first,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_full_walk_reaches_each_entity_once_in_ascending_order"),
        word: Word::Succeeds,
        input: || reaches_each_once_in_order,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::rows::types::Table;
