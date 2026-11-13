//! The support of the `join` cases.
use std::borrow::Cow;

use super::super::super::{service::schema_of, types::InputShape};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as Source, JoinColumn, Side},
    plans::types::Operation,
};

/// The shape of the left input of the join: one number column.
pub(crate) const NUMBER: InputShape<'static> = InputShape {
    kinds: &[CellKind::Number],
    tree: None,
};

/// The kinds of the columns the join of `entry` and `sequence_element` on their first columns, answering the left's first
/// column, answers over these inputs.
pub(crate) fn subject(
    inputs: &[InputShape<'_>],
) -> Result<Cow<'static, [CellKind]>, OperatingError> {
    let operation = Operation::Join {
        left: Source::Relation(Relation::Entry),
        right: Source::Relation(Relation::SequenceElement),
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
    schema_of(&operation, inputs, &[]).map(Cow::Owned)
}
