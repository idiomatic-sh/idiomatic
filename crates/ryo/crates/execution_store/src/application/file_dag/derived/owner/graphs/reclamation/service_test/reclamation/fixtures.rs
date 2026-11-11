//! The support of the `reclamation` cases.
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn};

pub(super) fn rows<'a>(columns: &'a [FactColumn; 2]) -> Rows<'a> {
    Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number; 2],
        columns,
    })
}

pub(super) fn columns() -> [FactColumn; 2] {
    [FactColumn(vec![1, 2, 3, 4]), FactColumn(vec![4, 3, 2, 1])]
}
