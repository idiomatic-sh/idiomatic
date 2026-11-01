//! Logical projected columns are checked before a virtual RHS can be exposed.

use super::value::ReachJoin;
use crate::domain::table_model::types::{Rows, Table};
use ryo_language::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, ReachLookup};
use ryo_values::domain::plans::types::Operation;

#[test]
fn malformed_virtual_column_is_rejected_on_an_empty_population() {
    let table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Default::default()
    };
    let rows = Rows::Computed(&table);
    let operation = Operation::Join {
        left: Input::Relation(Relation::Function),
        right: Input::Relation(Relation::Function),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![],
        filter: vec![],
        view: vec![],
        reach: Some(ReachLookup {
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
            keys: vec![],
            columns: vec![ColumnIndex(2)],
        }),
    };
    assert!(
        ReachJoin::new(
            &operation,
            &rows,
            Default::default(),
            Vec::new(),
            Vec::new()
        )
        .is_err()
    );
}
