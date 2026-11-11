//! The stored item reads the columns of a relation where the store keeps them and refuses every other representation.
use super::super::{interface::RepresentationsItem, types::CellRead};
use super::item::StoredItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service_test::column;
use crate::domain::table_model::types::PhysicalShape;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Value as Cell,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The rows the stored item is offered, and the representation they arrive in.
#[derive(Debug)]
enum Offered {
    /// Number columns where the store keeps them.
    Stored(Vec<FactColumn>),
    /// An empty table an earlier step computed.
    Computed,
}

/// What the item answers about a relation it admits: its rows, its shape, the cell at row 1 and
/// the cell one row past the end.
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
        Offered::Stored(columns) => read(RepresentationInput::Stored(ColumnsView {
            kinds: &[CellKind::Number],
            columns: &columns,
        })),
        Offered::Computed => read(RepresentationInput::Computed(&Table::default())),
    }
}

/// Admits the representation in bulk, then reads its contract, its shape and two cells.
fn read(offered: RepresentationInput<'_>) -> Result<Read, OperatingError> {
    let raw = StoredItem.bulk(offered)?;
    let contract = StoredItem.prepare(raw)?;
    Ok(Read {
        rows: contract.rows,
        shape: StoredItem.inspect(&contract)?,
        second: StoredItem.cell(raw, RowIndex(1), ColumnIndex(0))?,
        past_the_end: StoredItem.cell(raw, RowIndex(2), ColumnIndex(0))?,
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_columns_of_a_stored_relation"),
        word: Word::Succeeds,
        input: || Offered::Stored(vec![column(&[7, 8])]),
        expect: Expect::Value(Read {
            rows: RowCount(2),
            shape: PhysicalShape::Complete,
            second: CellRead::Value(Cell::Number(Integer(8))),
            past_the_end: CellRead::MissingPhysical,
        }),
    },
    Case {
        name: CaseName("refuses_a_computed_table"),
        word: Word::Refuses,
        input: || Offered::Computed,
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
