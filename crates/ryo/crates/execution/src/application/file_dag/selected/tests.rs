//! Malformed selected populations refuse at the production borrowed-row contract.
use crate::domain::{operating::errors::OperatingError, operator::contract::views::BorrowedRows};
use ryo_relations::domain::relations::types::{CellKind, FactColumn};
use ryo_values::domain::plan_parts::types::ColumnIndex;
fn source(values: &[&[i64]]) -> Vec<FactColumn> {
    values
        .iter()
        .map(|values| FactColumn(values.iter().map(|value| *value as u64).collect()))
        .collect()
}
fn borrowed<'a>(columns: &'a [FactColumn], positions: &'a [u32]) -> BorrowedRows<'a> {
    BorrowedRows::Selected {
        source: ryo_relations::domain::relations::types::ColumnsView {
            kinds: &[CellKind::Node, CellKind::Node],
            columns,
        },
        positions,
        columns: &[ColumnIndex(1), ColumnIndex(0)],
        kinds: &[CellKind::Node, CellKind::Node],
    }
}
#[test]
fn malformed_selected_populations_are_refused_before_indexing() {
    let complete = source(&[&[1, 2], &[7, 8]]);
    let mut ragged = complete.clone();
    ragged[1].0.pop();
    assert!(matches!(
        borrowed(&ragged, &[0]).validate(),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        borrowed(&complete, &[2]).validate(),
        Err(OperatingError::ShapeMismatch)
    ));
}
