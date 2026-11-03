//! The pure projected tuple kernel retains collision and invalid-cell controls.
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn};
use ryo_values::domain::plan_parts::types::ColumnIndex;
#[test]
fn distinct_representatives_handle_collisions_and_invalid_cells() {
    let columns = [
        FactColumn(vec![7, 7, 7, 7, 7, 8]),
        FactColumn(vec![1, 2, 0, 7, 0, 1]),
        FactColumn(vec![10, 10, 20, 11, 20, 10]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    let constant_hash = |_: ColumnsView<'_>, _: &[ColumnIndex], _: u32| Some(0);
    assert_eq!(
        crate::domain::stored_selection::service::representatives(
            source,
            &[ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)],
            &[0, 1, 2, 3, 4],
            constant_hash
        ),
        Some(vec![0, 2, 3]),
    );
    assert_eq!(
        crate::domain::stored_selection::service::representatives(
            source,
            &[ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)],
            &[0, 99],
            constant_hash
        ),
        None,
    );
    assert_eq!(
        crate::domain::stored_selection::service::representatives(
            source,
            &[ColumnIndex(99)],
            &[0],
            constant_hash
        ),
        None,
    );
}
