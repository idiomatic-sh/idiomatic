//! The selected item reads the positions and columns a selection keeps of a stored relation and refuses every other representation.
use super::super::{
    interface::RepresentationsItem,
    types::{CellRead, PhysicalShape, RepresentationInput},
};
use super::item::SelectedItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::fixtures::{column, number, selection};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
fn reads_the_positions_and_columns_of_a_selection() -> Result<(), OperatingError> {
    let columns = [column(&[2, 3])];
    let source = ColumnsView {
        kinds: &[CellKind::Number],
        columns: &columns,
    };
    let kept = selection(vec![1, 0], vec![ColumnIndex(0)], vec![CellKind::Number]);
    let raw = RepresentationInput::Selected {
        source,
        positions: &kept.rows,
        columns: &kept.columns,
        kinds: &kept.kinds,
    };
    let contract = SelectedItem.prepare(raw)?;
    assert_eq!(contract.rows, RowCount(2));
    assert_eq!(SelectedItem.inspect(&contract)?, PhysicalShape::Complete);
    assert_eq!(
        SelectedItem.cell(raw, RowIndex(0), ColumnIndex(0))?,
        CellRead::Value(number(3))
    );
    assert_eq!(
        SelectedItem.cell(raw, RowIndex(2), ColumnIndex(0))?,
        CellRead::MissingPhysical
    );
    Ok(())
}
fn refuses_a_computed_table() -> Result<(), OperatingError> {
    let table = Table::default();
    SelectedItem
        .bulk(RepresentationInput::Computed(&table))
        .map(|_| ())
}

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = fn() -> Result<(), OperatingError>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_positions_and_columns_of_a_selection"),
        word: Word::Succeeds,
        input: || reads_the_positions_and_columns_of_a_selection,
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
