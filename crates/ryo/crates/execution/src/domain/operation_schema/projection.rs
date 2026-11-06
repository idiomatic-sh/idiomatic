//! Column projection metadata shared by preparation and empty continuation publication.
use crate::domain::operating::{errors::OperatingError, types::InputPlace};
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, JoinColumn, Side},
    plans::types::Operation,
    rows::{errors::RowsError, service::column_in},
};

/// Project columns without consulting a row; a repeated column remains repeated.
pub(crate) fn projected(
    kinds: &[CellKind],
    columns: &[ColumnIndex],
) -> Result<Vec<CellKind>, OperatingError> {
    columns
        .iter()
        .map(|column| {
            column_in(kinds, *column)
                .copied()
                .ok_or_else(|| RowsError::NoSuchColumn { index: column.0 }.into())
        })
        .collect()
}

/// Carry the schema of an empty first instruction through the compiler's closed
/// row-continuation family without evaluating predicates or acquiring bindings.
pub(crate) fn continuation_kinds(
    operation: &Operation,
    input: &[CellKind],
) -> Result<Vec<CellKind>, OperatingError> {
    match operation {
        Operation::Select { kept, .. } if kept.is_empty() => Ok(input.to_vec()),
        Operation::Project {
            columns,
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
            ..
        } => projected(input, columns),
        _ => Err(OperatingError::WrongOperation),
    }
}

/// Read one input's metadata in the same input order as execution.
pub(crate) fn schema_input<'a>(
    inputs: &[InputShape<'a>],
    place: InputPlace,
) -> Result<InputShape<'a>, OperatingError> {
    inputs
        .get(place.0)
        .copied()
        .ok_or(OperatingError::MissingInput {
            position: place.0 + 1,
        })
}

/// Output kinds of a two-input projection, optionally through a native tree view.
pub(crate) fn joined(
    left: &[CellKind],
    right: &[CellKind],
    view: Option<&[ColumnIndex]>,
    output: &[JoinColumn],
) -> Result<Vec<CellKind>, OperatingError> {
    output
        .iter()
        .map(|column| {
            let (kinds, source) = match column.side {
                Side::Left => (left, column.column),
                Side::Right => (
                    right,
                    view.map_or(column.column, |view| {
                        crate::domain::tree_join::service::source_column(view, column.column)
                    }),
                ),
            };
            column_in(kinds, source).copied().ok_or_else(|| {
                ryo_values::domain::rows::errors::RowsError::NoSuchColumn {
                    index: column.column.0,
                }
                .into()
            })
        })
        .collect()
}
