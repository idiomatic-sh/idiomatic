//! The computed item reads the rows of an earlier step and refuses every other representation.
use super::super::{interface::RepresentationsItem, types::CellRead};
use super::item::ComputedItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service_test::number;
use crate::domain::table_model::types::PhysicalShape;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The rows the computed item is offered, and the representation they arrive in.
#[derive(Debug)]
enum Offered {
    /// A table an earlier step computed.
    Computed(Table),
    /// A stored relation with no columns.
    Stored,
}

/// What the item answers about a table it admits: its rows, its shape, the cell at row 1 and the
/// cell one row past the end.
#[derive(Debug, PartialEq)]
struct Read {
    rows: RowCount,
    shape: PhysicalShape,
    second: CellRead,
    past_the_end: CellRead,
}

type Input = Offered;
type Value = Read;
type Failure = OperatingError;

fn subject(offered: Input) -> Result<Value, Failure> {
    match offered {
        Offered::Computed(table) => read(RepresentationInput::Computed(&table)),
        Offered::Stored => read(RepresentationInput::Stored(ColumnsView {
            kinds: &[],
            columns: &[],
        })),
    }
}

/// Admits the representation in bulk, then reads its contract, its shape and two cells.
fn read(offered: RepresentationInput<'_>) -> Result<Read, OperatingError> {
    let raw = ComputedItem.bulk(offered)?;
    let contract = ComputedItem.prepare(raw)?;
    Ok(Read {
        rows: contract.rows,
        shape: ComputedItem.inspect(&contract)?,
        second: ComputedItem.cell(raw, RowIndex(1), ColumnIndex(0))?,
        past_the_end: ComputedItem.cell(raw, RowIndex(2), ColumnIndex(0))?,
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_rows_of_a_computed_table"),
        word: Word::Succeeds,
        input: || {
            Offered::Computed(Table {
                kinds: vec![CellKind::Number],
                cells: vec![number(4), number(5)],
                count: RowCount(2),
            })
        },
        expect: Expect::Value(Read {
            rows: RowCount(2),
            shape: PhysicalShape::Complete,
            second: CellRead::Value(Cell::Number(Integer(5))),
            past_the_end: CellRead::MissingPhysical,
        }),
    },
    Case {
        name: CaseName("refuses_a_stored_relation"),
        word: Word::Refuses,
        input: || Offered::Stored,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
