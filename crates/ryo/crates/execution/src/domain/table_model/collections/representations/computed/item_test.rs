//! The computed item reads the rows of an earlier step and refuses every other representation.
use super::super::{
    interface::RepresentationsItem,
    types::{CellRead, PhysicalShape, RepresentationInput},
};
use super::item::ComputedItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::fixtures::number;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
fn reads_the_rows_of_a_computed_table() -> Result<(), OperatingError> {
    let table = Table {
        kinds: vec![CellKind::Number],
        cells: vec![number(4), number(5)],
        count: RowCount(2),
    };
    let raw = RepresentationInput::Computed(&table);
    let contract = ComputedItem.prepare(raw)?;
    assert_eq!(contract.rows, RowCount(2));
    assert_eq!(ComputedItem.inspect(&contract)?, PhysicalShape::Complete);
    assert_eq!(
        ComputedItem.cell(raw, RowIndex(1), ColumnIndex(0))?,
        CellRead::Value(number(5))
    );
    assert_eq!(
        ComputedItem.cell(raw, RowIndex(2), ColumnIndex(0))?,
        CellRead::MissingPhysical
    );
    Ok(())
}
fn refuses_a_stored_relation() -> Result<(), OperatingError> {
    let stored = RepresentationInput::Stored(ColumnsView {
        kinds: &[],
        columns: &[],
    });
    ComputedItem.bulk(stored).map(|_| ())
}

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = fn() -> Result<(), OperatingError>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_rows_of_a_computed_table"),
        word: Word::Succeeds,
        input: || reads_the_rows_of_a_computed_table,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("refuses_a_stored_relation"),
        word: Word::Refuses,
        input: || refuses_a_stored_relation,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
