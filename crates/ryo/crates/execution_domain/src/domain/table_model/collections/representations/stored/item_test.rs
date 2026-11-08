//! The stored item reads the columns of a relation where the store keeps them and refuses every other representation.
use crate::domain::table_model::types::PhysicalShape;
use super::super::{
    interface::RepresentationsItem,
    types::CellRead,
};
use crate::domain::table_model::types::RepresentationInput;
use super::item::StoredItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service_test::{column, number};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
fn reads_the_columns_of_a_stored_relation() -> Result<(), OperatingError> {
    let columns = [column(&[7, 8])];
    let raw = RepresentationInput::Stored(ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    });
    let contract = StoredItem.prepare(raw)?;
    assert_eq!(contract.rows, RowCount(2));
    assert_eq!(StoredItem.inspect(&contract)?, PhysicalShape::Complete);
    assert_eq!(
        StoredItem.cell(raw, RowIndex(1), ColumnIndex(0))?,
        CellRead::Value(number(8))
    );
    assert_eq!(
        StoredItem.cell(raw, RowIndex(2), ColumnIndex(0))?,
        CellRead::MissingPhysical
    );
    Ok(())
}
fn refuses_a_computed_table() -> Result<(), OperatingError> {
    let table = Table::default();
    StoredItem
        .bulk(RepresentationInput::Computed(&table))
        .map(|_| ())
}

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = fn() -> Result<(), OperatingError>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_columns_of_a_stored_relation"),
        word: Word::Succeeds,
        input: || reads_the_columns_of_a_stored_relation,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("refuses_a_computed_table"),
        word: Word::Refuses,
        input: || refuses_a_computed_table,
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
