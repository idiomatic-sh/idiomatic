//! Missing RHS is a schema/input refusal, independent of any owner or source.
use super::{service::schema_of, types::InputShape};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, JoinColumn, Side},
    plans::types::Operation,
};
#[test]
fn join_without_its_second_input_reports_position_two() {
    let operation = Operation::Join {
        left: Input::Relation(Relation::Entry),
        right: Input::Relation(Relation::SequenceElement),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    assert!(matches!(
        schema_of(
            &operation,
            &[InputShape {
                kinds: &[CellKind::Number],
                tree: None
            }],
            &[]
        ),
        Err(OperatingError::MissingInput { position: 2 })
    ));
}
