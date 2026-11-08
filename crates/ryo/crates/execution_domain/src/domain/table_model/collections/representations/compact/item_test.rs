//! The compact item reads admitted packed rows and refuses every other representation.
use crate::domain::table_model::types::PhysicalShape;
use super::super::{
    interface::RepresentationsItem,
    types::CellRead,
};
use crate::domain::table_model::types::RepresentationInput;
use super::item::CompactItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service_test::{compact, number};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
fn reads_the_rows_of_a_packed_table() -> Result<(), OperatingError> {
    let table = Table {
        kinds: vec![CellKind::Number],
        cells: vec![number(8), number(7)],
        count: RowCount(2),
    };
    let packed = compact(&table);
    let raw = RepresentationInput::Compact(&packed);
    let contract = CompactItem.prepare(raw)?;
    assert_eq!(contract.rows, RowCount(2));
    assert_eq!(CompactItem.inspect(&contract)?, PhysicalShape::Complete);
    assert_eq!(
        CompactItem.cell(raw, RowIndex(1), ColumnIndex(0))?,
        CellRead::Value(number(7))
    );
    Ok(())
}
fn refuses_a_computed_table() -> Result<(), OperatingError> {
    let table = Table::default();
    CompactItem
        .bulk(RepresentationInput::Computed(&table))
        .map(|_| ())
}

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = fn() -> Result<(), OperatingError>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_rows_of_a_packed_table"),
        word: Word::Succeeds,
        input: || reads_the_rows_of_a_packed_table,
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
