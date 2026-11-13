//! The compact item reads admitted packed rows and refuses every other representation.
use super::super::{interface::RepresentationsItem, types::CellRead};
use super::item::CompactItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service_test::{compact, number};
use crate::domain::table_model::types::PhysicalShape;
use crate::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The table the compact item is offered, and the representation it arrives in.
#[derive(Debug)]
enum Offered {
    /// The table packed into compact rows.
    Packed(Table),
    /// The table as an earlier step computed it.
    Computed(Table),
}

/// What the item answers about a table it admits: its rows, its shape and the cell at row 1.
#[derive(Debug, PartialEq)]
struct Read {
    rows: RowCount,
    shape: PhysicalShape,
    second: CellRead,
}

type Input = Offered;
type Value = Read;
type Failure = OperatingError;

fn subject(offered: Input) -> Result<Value, Failure> {
    match offered {
        Offered::Packed(table) => read(RepresentationInput::Compact(&compact(&table))),
        Offered::Computed(table) => read(RepresentationInput::Computed(&table)),
    }
}

/// Admits the representation in bulk, then reads its contract, its shape and the cell at row 1.
fn read(offered: RepresentationInput<'_>) -> Result<Read, OperatingError> {
    let raw = CompactItem.bulk(offered)?;
    let contract = CompactItem.prepare(raw)?;
    Ok(Read {
        rows: contract.rows,
        shape: CompactItem.inspect(&contract)?,
        second: CompactItem.cell(raw, RowIndex(1), ColumnIndex(0))?,
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_rows_of_a_packed_table"),
        word: Word::Succeeds,
        input: || {
            Offered::Packed(Table {
                kinds: vec![CellKind::Number],
                cells: vec![number(8), number(7)],
                count: RowCount(2),
            })
        },
        expect: Expect::Value(Read {
            rows: RowCount(2),
            shape: PhysicalShape::Complete,
            second: CellRead::Value(Cell::Number(Integer(7))),
        }),
    },
    Case {
        name: CaseName("refuses_a_computed_table"),
        word: Word::Refuses,
        input: || Offered::Computed(Table::default()),
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
